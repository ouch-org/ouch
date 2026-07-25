//! Receive command from the cli and call the respective function for that command.

mod compress;
mod decompress;
mod list;

use std::path::PathBuf;

use bstr::ByteSlice;
use decompress::{DecompressOptions, PreparedTarget, prepare_decompress_target};
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use utils::colors;

use crate::{
    CliArgs, INITIAL_CURRENT_DIR, QuestionPolicy, Result,
    check::{self, CheckFileSignatureControlFlow},
    cli::Subcommand,
    commands::{compress::compress_files, decompress::decompress_file, list::list_archive_contents},
    error::{Error, FinalError},
    extension::{self, parse_format_flag},
    info, info_accessible,
    list::ListOptions,
    sandbox::{self, SandboxPolicy},
    utils::{
        self, BytesFmt, FileVisibilityPolicy, NoQuotePathFmt, PathFmt, QuestionAction, canonicalize, colors::*,
        file_size, is_path_stdin,
    },
    warning,
};

/// Warn the user that (de)compressing this .zip archive might freeze their system.
fn warn_user_about_loading_zip_in_memory() {
    const ZIP_IN_MEMORY_LIMITATION_WARNING: &str = "\n  \
        The format '.zip' is limited by design and cannot be (de)compressed with encoding streams.\n  \
        When chaining '.zip' with other formats, all (de)compression needs to be done in-memory\n  \
        Careful, you might run out of RAM if the archive is too large!";

    eprintln!("{}[WARNING]{}: {ZIP_IN_MEMORY_LIMITATION_WARNING}", *ORANGE, *RESET);
}

/// Warn the user that (de)compressing this .7z archive might freeze their system.
fn warn_user_about_loading_sevenz_in_memory() {
    const SEVENZ_IN_MEMORY_LIMITATION_WARNING: &str = "\n  \
        The format '.7z' is limited by design and cannot be (de)compressed with encoding streams.\n  \
        When chaining '.7z' with other formats, all (de)compression needs to be done in-memory\n  \
        Careful, you might run out of RAM if the archive is too large!";

    eprintln!("{}[WARNING]{}: {SEVENZ_IN_MEMORY_LIMITATION_WARNING}", *ORANGE, *RESET);
}

/// This function checks what command needs to be run and performs A LOT of ahead-of-time checks
/// to assume everything is OK.
///
/// There are a lot of custom errors to give enough error description and explanation.
pub fn run(args: CliArgs, question_policy: QuestionPolicy, file_visibility_policy: FileVisibilityPolicy) -> Result<()> {
    // No global pool here because Landlock only confines threads created after it is applied
    // Decompression builds its pool after the sandbox so the workers are confined

    match args.cmd {
        Subcommand::Compress {
            files,
            output: output_path,
            level,
            fast,
            slow,
            follow_symlinks,
        } => {
            // After cleaning, if there are no input files left, exit
            if files.is_empty() {
                return Err(FinalError::with_title("No files to compress").into());
            }

            // gitignore and follow_symlinks both read paths outside the declared input set so the
            // sandbox cannot confine them; run unsandboxed and say why
            let sandbox_disabled = sandbox::disabled_by_request(args.no_sandbox) || args.gitignore || follow_symlinks;
            if cfg!(target_os = "linux") && !sandbox::disabled_by_request(args.no_sandbox) {
                if args.gitignore {
                    info!("Sandbox: disabled because --gitignore reads git configuration outside the input files");
                }
                if follow_symlinks {
                    info!("Sandbox: disabled because --follow-symlinks may read files outside the input set");
                }
            }

            // Formats from path extension, like "file.tar.gz.xz" -> vec![Tar, Gzip, Lzma]
            let (formats_from_flag, formats) = match args.format {
                Some(formats) => {
                    let parsed_formats = parse_format_flag(&formats)?;
                    (Some(formats), parsed_formats)
                }
                None => (None, extension::extensions_from_path(&output_path)?),
            };

            check::check_invalid_compression_with_non_archive_format(
                &formats,
                &output_path,
                &files,
                formats_from_flag.as_deref(),
            )?;
            check::check_archive_formats_position(&formats, &output_path)?;

            let (output_file, output_path) = match utils::create_file_or_prompt_on_conflict(
                &output_path,
                question_policy,
                QuestionAction::Compression,
            )? {
                Some(writer) => writer,
                None => return Ok(()),
            };

            // Read on inputs. The output FD is held already so no write-path grant is needed, and
            // the sandbox is deliberately not widened to let compression delete files in the
            // output directory.
            let sandbox_active = {
                let mut policy = SandboxPolicy::new();
                for f in &files {
                    policy.allow_read(sandbox::canonicalize_for_sandbox(f));
                }
                policy.set_disabled(sandbox_disabled).apply()
            };

            let level = if fast {
                Some(1) // Lowest level of compression
            } else if slow {
                Some(i16::MAX) // Highest level of compression
            } else {
                level
            };

            let compress_result = compress_files(
                files,
                formats,
                output_file,
                &output_path,
                follow_symlinks,
                question_policy,
                file_visibility_policy,
                level,
            );

            if let Ok(true) = compress_result {
                info_accessible!("Output file size: {}", BytesFmt(file_size(&output_path)?));
                info_accessible!("Successfully compressed to {}", PathFmt(&output_path));
            } else if let Ok(false) = compress_result {
                // user cancelled; remove the partial output where the sandbox permits it
                if !sandbox_active {
                    let _ = utils::remove_file_or_dir(&output_path);
                }
            } else if compress_result.is_err() {
                let deleted = !sandbox_active && utils::remove_file_or_dir(&output_path).is_ok();
                if !deleted {
                    if sandbox_active {
                        // Not a cleanup failure: the sandbox intentionally does not grant removal
                        // in the output directory, so the partial file is left in place.
                        warning!(
                            "Compression did not finish; the partial file at {} was left because the sandbox does not permit removing it. Delete it manually.",
                            PathFmt(&output_path)
                        );
                    } else {
                        eprintln!("{red}FATAL ERROR:\n", red = *colors::RED);
                        eprintln!(
                            "  Compression did not finish; the partial file at {} may be corrupted.",
                            PathFmt(&output_path)
                        );
                        eprintln!("  Please delete it manually.");
                        eprintln!("  Compression failed for reasons below.");
                    }
                }
            }

            compress_result.map(|_| ())
        }
        Subcommand::Decompress {
            files,
            output_dir,
            here,
            remove,
        } => {
            let mut files_output_paths: Vec<_> = vec![];
            let mut files_extensions: Vec<Vec<_>> = vec![];

            if let Some(format) = args.format {
                let format = parse_format_flag(&format)?;
                for path in files.iter() {
                    let file_name = path.file_name().ok_or_else(|| Error::Custom {
                        reason: FinalError::with_title(format!("{} does not have a file name", PathFmt(path))),
                    })?;
                    files_output_paths.push(file_name.into());
                    files_extensions.push(format.clone());
                }
            } else {
                for path in files.iter() {
                    let (output_path, mut extensions) = extension::separate_known_extensions_from_name(path)?;
                    let mut output_path = output_path.to_owned();

                    match check::check_file_signature(path, &extensions, question_policy)? {
                        CheckFileSignatureControlFlow::HaltProgram => return Ok(()),
                        CheckFileSignatureControlFlow::Continue => {}
                        CheckFileSignatureControlFlow::ChangeToDetectedExtension {
                            new_extension,
                            new_path_filename,
                        } => {
                            extensions = vec![new_extension];
                            output_path = output_path.with_file_name(new_path_filename);
                        }
                    }

                    files_output_paths.push(output_path);
                    files_extensions.push(extensions);
                }
            }

            check::check_missing_formats_when_decompressing(&files, &files_extensions)?;

            // The directory that will contain the output files
            // We default to the current directory if the user didn't specify an output directory with --dir
            let output_dir_was_explicit = output_dir.is_some();
            let output_dir = if let Some(dir) = output_dir {
                utils::create_dir_if_non_existent(&dir)?;
                // If not canonicalized, strip_prefix won't work and logs will break
                // Led to bugs when output_dir was a symlink
                canonicalize(&dir)?
            } else {
                INITIAL_CURRENT_DIR.clone()
            };

            // Per-input output paths for single-file outputs and archive wrapper dirs.
            let output_file_paths: Vec<PathBuf> = files_output_paths
                .iter()
                .map(|file_name| {
                    if is_path_stdin(file_name) {
                        output_dir.join("ouch-output")
                    } else {
                        output_dir.join(file_name)
                    }
                })
                .collect();

            // Resolve conflicts and create output dirs before the sandbox applies.
            // claimed targets make duplicate basenames conflict instead of silently merging
            let mut claimed_targets = std::collections::HashSet::new();
            let mut prepared: Vec<PreparedTarget> = Vec::with_capacity(files.len());
            for (formats, output_file_path) in files_extensions.iter().zip(&output_file_paths) {
                prepared.push(prepare_decompress_target(
                    formats,
                    &output_dir,
                    output_file_path,
                    output_dir_was_explicit,
                    here,
                    question_policy,
                    &mut claimed_targets,
                )?);
            }

            // Read on inputs and read-write on each output directory.
            {
                let mut policy = SandboxPolicy::new();
                for f in &files {
                    if !is_path_stdin(f) {
                        policy.allow_read(sandbox::canonicalize_for_sandbox(f));
                    }
                }

                // Collect the directories the warnings refer to while building the policy.
                // The warnings are printed only when the sandbox is actually enforced.
                let mut home_targets: Vec<PathBuf> = Vec::new();
                for p in &prepared {
                    if let PreparedTarget::Target { dir, .. } = p {
                        let canon = sandbox::canonicalize_for_sandbox(dir);
                        // Only --dir or --here can point the grant at $HOME
                        // default mode always makes a fresh subdirectory
                        if (output_dir_was_explicit || here) && sandbox::is_home_or_ancestor(&canon) {
                            home_targets.push(canon.clone());
                        }
                        policy.allow_read_write(canon);
                    }
                }

                let mut remove_parents: Vec<PathBuf> = Vec::new();
                if remove {
                    // Collect unique parents so each directory grants and warns once.
                    for f in &files {
                        if is_path_stdin(f) {
                            continue;
                        }
                        // Grant in the directory that actually holds the input. For a symlinked
                        // input that is the symlink's own directory, not the target's, because
                        // --remove unlinks the symlink itself.
                        let input_dir = match f.parent() {
                            Some(p) if !p.as_os_str().is_empty() => sandbox::canonicalize_for_sandbox(p),
                            _ => sandbox::canonicalize_for_sandbox(std::path::Path::new(".")),
                        };
                        if !remove_parents.contains(&input_dir) {
                            remove_parents.push(input_dir);
                        }
                    }
                    // The decompressor can delete the input archive but not write nearby.
                    for parent in &remove_parents {
                        policy.allow_remove_in(parent.clone());
                    }
                }

                // Apply once and warn only about what a real sandbox cannot confine.
                let enforced = policy.set_disabled(args.no_sandbox).apply();
                if enforced {
                    for target in home_targets {
                        warning!(
                            "Sandbox: extraction target {} is $HOME or an ancestor of it; the sandbox cannot meaningfully confine writes",
                            PathFmt(&target)
                        );
                    }
                    for parent in remove_parents {
                        warning!(
                            "Sandbox: the extractor can delete any file in {}, not just the archive",
                            PathFmt(&parent)
                        );
                    }
                }
            }

            // Build the pool after the sandbox so its worker threads are confined by it
            // --threads 0 means auto and lets rayon pick one thread per core
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(args.threads.filter(|&t| t != 0).unwrap_or(0))
                .build()
                .map_err(|e| FinalError::with_title("Failed to initialize thread pool").detail(e.to_string()))?;

            pool.install(move || {
                files.par_iter().zip(files_extensions).zip(prepared).try_for_each(
                    |((input_path, formats), prepared)| {
                        decompress_file(DecompressOptions {
                            input_file_path: input_path,
                            formats,
                            output_dir: &output_dir,
                            output_dir_was_explicit,
                            here,
                            question_policy,
                            password: args.password.as_deref().map(|str| {
                                <[u8] as ByteSlice>::from_os_str(str).expect("convert password to bytes failed")
                            }),
                            remove,
                            prepared,
                        })
                        .map_err(|err| match err {
                            Error::IoError { reason } => Error::Custom {
                                reason: FinalError::with_title(format!(
                                    "Failed to decompress {}",
                                    NoQuotePathFmt(input_path)
                                ))
                                .detail(reason),
                            },
                            other => other,
                        })
                    },
                )
            })
        }
        Subcommand::List { archives: files, tree } => {
            let mut formats = vec![];

            if let Some(format) = args.format {
                let format = parse_format_flag(&format)?;
                for _ in 0..files.len() {
                    formats.push(format.clone());
                }
            } else {
                for path in files.iter() {
                    let mut extensions = extension::extensions_from_path(path)?;

                    match check::check_file_signature(path, &extensions, question_policy)? {
                        CheckFileSignatureControlFlow::HaltProgram => return Ok(()),
                        CheckFileSignatureControlFlow::Continue => {}
                        CheckFileSignatureControlFlow::ChangeToDetectedExtension { new_extension, .. } => {
                            extensions = vec![new_extension]
                        }
                    }

                    formats.push(extensions);
                }
            }

            // Ensure we were not told to list the content of a non-archive compressed file
            check::check_for_non_archive_formats(&files, &formats)?;

            // Flatten the per-file formats up front so the next step can inspect them.
            let flat_formats: Vec<Vec<extension::CompressionFormat>> = formats
                .iter()
                .map(|f| extension::flatten_compression_formats(f))
                .collect();
            // Open RAR spill tempfiles up front so unrar can read from them under the sandbox.
            // The archive format sits at the front so match there not at the end.
            let needs_spill = |formats: &[extension::CompressionFormat]| {
                formats.len() > 1 && matches!(formats.first(), Some(extension::CompressionFormat::Rar))
            };

            // all spills share one private temp dir so the sandbox can allow their cleanup
            let rar_spill_dir = if flat_formats.iter().any(|fs| needs_spill(fs)) {
                Some(tempfile::tempdir()?)
            } else {
                None
            };
            let mut rar_spill_tempfiles: Vec<Option<tempfile::NamedTempFile>> = Vec::with_capacity(flat_formats.len());
            for fs in &flat_formats {
                let spill = if needs_spill(fs) {
                    let dir = rar_spill_dir
                        .as_ref()
                        .expect("spill dir is created when any input needs a spill");
                    Some(tempfile::Builder::new().prefix(".ouch-rar-").tempfile_in(dir.path())?)
                } else {
                    None
                };
                rar_spill_tempfiles.push(spill);
            }

            {
                let mut policy = SandboxPolicy::new();
                for f in &files {
                    policy.allow_read(sandbox::canonicalize_for_sandbox(f));
                }
                if let Some(spill_dir) = &rar_spill_dir {
                    let canon = sandbox::canonicalize_for_sandbox(spill_dir.path());
                    // unrar reopens the spill files by path
                    policy.allow_read(canon.clone());
                    // each NamedTempFile unlinks its spill file on drop
                    policy.allow_remove_in(canon.clone());
                    // Grant directory removal on the spill directory's parent
                    if let Some(parent) = canon.parent() {
                        policy.allow_remove_dir_in(parent.to_path_buf());
                    }
                }
                policy.set_disabled(args.no_sandbox).apply();
            }

            let list_options = ListOptions {
                tree,
                quiet: args.quiet,
            };

            for (i, ((archive_path, formats), spill)) in files
                .iter()
                .zip(flat_formats)
                .zip(rar_spill_tempfiles.iter_mut())
                .enumerate()
            {
                if i > 0 && !args.quiet {
                    println!();
                }
                list_archive_contents(
                    archive_path,
                    formats,
                    list_options,
                    question_policy,
                    args.password
                        .as_deref()
                        .map(|str| <[u8] as ByteSlice>::from_os_str(str).expect("convert password to bytes failed")),
                    spill.take(),
                )?;
            }

            Ok(())
        }
    }
}
