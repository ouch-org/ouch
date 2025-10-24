//! Receive command from the cli and call the respective function for that command.

mod compress;
mod decompress;
mod list;

use std::{ops::ControlFlow, path::PathBuf};

use bstr::ByteSlice;
use decompress::DecompressOptions;
pub use decompress::Unpacked;
use rayon::prelude::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator};
use utils::colors;

use crate::{
    check,
    cli::Subcommand,
    commands::{compress::compress_files, decompress::decompress_file, list::list_archive_contents},
    error::{Error, FinalError},
    extension::{self, parse_format_flag},
    list::ListOptions,
    utils::{
        self, colors::*, is_path_stdin, logger::info_accessible, path_to_str, EscapedPathDisplay, FileVisibilityPolicy,
        QuestionAction,
    },
    CliArgs, QuestionPolicy,
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
pub fn run(
    args: CliArgs,
    question_policy: QuestionPolicy,
    file_visibility_policy: FileVisibilityPolicy,
) -> crate::Result<()> {
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap();
    }

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
                formats_from_flag.as_ref(),
            )?;
            check::check_archive_formats_position(&formats, &output_path)?;

            let output_file =
                match utils::ask_to_create_file(&output_path, question_policy, QuestionAction::Compression)? {
                    Some(writer) => writer,
                    None => return Ok(()),
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
                args.quiet,
                follow_symlinks,
                question_policy,
                file_visibility_policy,
                level,
            );

            if let Ok(true) = compress_result {
                // this is only printed once, so it doesn't result in much text. On the other hand,
                // having a final status message is important especially in an accessibility context
                // as screen readers may not read a commands exit code, making it hard to reason
                // about whether the command succeeded without such a message
                info_accessible(format!("Successfully compressed '{}'", path_to_str(&output_path)));
            } else {
                // If Ok(false) or Err() occurred, delete incomplete file at `output_path`
                //
                // if deleting fails, print an extra alert message pointing
                // out that we left a possibly CORRUPTED file at `output_path`
                if utils::remove_file_or_dir(&output_path).is_err() {
                    eprintln!("{red}FATAL ERROR:\n", red = *colors::RED);
                    eprintln!(
                        "  Ouch failed to delete the file '{}'.",
                        EscapedPathDisplay::new(&output_path)
                    );
                    eprintln!("  Please delete it manually.");
                    eprintln!("  This file is corrupted if compression didn't finished.");

                    if compress_result.is_err() {
                        eprintln!("  Compression failed for reasons below.");
                    }
                }
            }

            compress_result.map(|_| ())
        }
        Subcommand::Decompress {
            files,
            output_dir,
            remove,
            no_smart_unpack,
        } => {
            let mut output_paths = vec![];
            let mut formats = vec![];

            if let Some(format) = args.format {
                let format = parse_format_flag(&format)?;
                for path in files.iter() {
                    // TODO: use Error::Custom
                    let file_name = path.file_name().ok_or_else(|| Error::NotFound {
                        error_title: format!("{} does not have a file name", EscapedPathDisplay::new(path)),
                    })?;
                    output_paths.push(file_name.as_ref());
                    formats.push(format.clone());
                }
            } else {
                for path in files.iter() {
                    let (pathbase, mut file_formats) = extension::separate_known_extensions_from_name(path)?;

                    if let ControlFlow::Break(_) = check::check_mime_type(path, &mut file_formats, question_policy)? {
                        return Ok(());
                    }

                    output_paths.push(pathbase);
                    formats.push(file_formats);
                }
            }

            check::check_missing_formats_when_decompressing(&files, &formats)?;

            let is_output_dir_provided = output_dir.is_some();
            let is_smart_unpack = !is_output_dir_provided && !no_smart_unpack;

            // The directory that will contain the output files
            // We default to the current directory if the user didn't specify an output directory with --dir
            let output_dir = if let Some(dir) = output_dir {
                utils::create_dir_if_non_existent(&dir)?;
                dir
            } else {
                PathBuf::from(".")
            };

            files
                .par_iter()
                .zip(formats)
                .zip(output_paths)
                .try_for_each(|((input_path, formats), file_name)| {
                    // Path used by single file format archives
                    let output_file_path = if is_path_stdin(file_name) {
                        output_dir.join("stdin-output")
                    } else {
                        output_dir.join(file_name)
                    };
                    decompress_file(DecompressOptions {
                        input_file_path: input_path,
                        formats,
                        is_output_dir_provided,
                        output_dir: &output_dir,
                        output_file_path,
                        is_smart_unpack,
                        question_policy,
                        quiet: args.quiet,
                        password: args.password.as_deref().map(|str| {
                            <[u8] as ByteSlice>::from_os_str(str).expect("convert password to bytes failed")
                        }),
                        remove,
                    })
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
                    let mut file_formats = extension::extensions_from_path(path)?;

                    if let ControlFlow::Break(_) = check::check_mime_type(path, &mut file_formats, question_policy)? {
                        return Ok(());
                    }

                    formats.push(file_formats);
                }
            }

            // Ensure we were not told to list the content of a non-archive compressed file
            check::check_for_non_archive_formats(&files, &formats)?;

            let list_options = ListOptions { tree };

            for (i, (archive_path, formats)) in files.iter().zip(formats).enumerate() {
                if i > 0 {
                    println!();
                }
                let formats = extension::flatten_compression_formats(&formats);
                list_archive_contents(
                    archive_path,
                    formats,
                    list_options,
                    question_policy,
                    args.password
                        .as_deref()
                        .map(|str| <[u8] as ByteSlice>::from_os_str(str).expect("convert password to bytes failed")),
                )?;
            }

            Ok(())
        }
    }
}
