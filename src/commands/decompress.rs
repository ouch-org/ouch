use std::{
    collections::HashSet,
    ffi::OsString,
    io::{self, BufReader, Read},
    ops::ControlFlow,
    path::{Path, PathBuf},
};

use fs_err::{self as fs, PathExt};

use crate::{
    BUFFER_CAPACITY, INITIAL_CURRENT_DIR, QuestionAction, QuestionPolicy, Result,
    commands::{warn_user_about_loading_sevenz_in_memory, warn_user_about_loading_zip_in_memory},
    extension::{
        CompressionFormat::{self, *},
        Extension, split_first_compression_format,
    },
    info, info_accessible,
    non_archive::lz4::MultiFrameLz4Decoder,
    utils::{
        BytesFmt, LZMA_MEMLIMIT_BYTES, LimitedReader, PathFmt, copy_limited_decompression, file_size,
        io::{ReadSeek, lock_and_flush_output_stdio},
        is_path_stdin, resolve_path_conflict, user_wants_to_continue,
    },
};

pub struct DecompressOptions<'a> {
    /// Example: "archive.tar.gz"
    pub input_file_path: &'a Path,
    /// Example: [Gz, Tar] (notice it's ordered in decompression order)
    pub formats: Vec<Extension>,
    pub output_dir: &'a Path,
    /// Whether the user passed `--dir` explicitly. When false (and `here` is false),
    /// archives are extracted into a basename-derived subdirectory of the CWD.
    pub output_dir_was_explicit: bool,
    /// `--here`: extract directly into the current directory like `tar -xf`.
    /// Only meaningful when `output_dir_was_explicit` is false.
    pub here: bool,
    pub question_policy: QuestionPolicy,
    pub password: Option<&'a [u8]>,
    pub remove: bool,
    /// Resolved target prepared upfront (before sandbox is applied).
    pub prepared: PreparedTarget,
}

/// Per-input decision made before the sandbox is applied.
/// file_name is the single output file for non-archives and None for archives.
pub enum PreparedTarget {
    Target { dir: PathBuf, file_name: Option<OsString> },
    Cancelled,
}

/// Resolve conflicts and create the output directory before the sandbox starts.
/// claimed_targets holds directories already taken by earlier inputs of this run.
pub fn prepare_decompress_target(
    formats: &[Extension],
    output_dir: &Path,
    output_file_path: &Path,
    output_dir_was_explicit: bool,
    here: bool,
    question_policy: QuestionPolicy,
    claimed_targets: &mut HashSet<PathBuf>,
) -> Result<PreparedTarget> {
    let (first_extension, _) = split_first_compression_format(formats);
    let is_archive = matches!(first_extension, Tar | Zip | Rar | SevenZip);

    // For a single file in an explicit directory, resolve the conflict on the file, not the directory
    if !is_archive && (output_dir_was_explicit || here) {
        let name = output_file_path.file_name().unwrap_or_default().to_os_string();
        let file_target = output_dir.join(&name);
        let claimed = claimed_targets.contains(&file_target);
        let resolved = if !claimed && !file_target.fs_err_try_exists()? {
            file_target
        } else if let Some(p) = resolve_path_conflict(&file_target, question_policy, QuestionAction::Decompression)? {
            p
        } else {
            return Ok(PreparedTarget::Cancelled);
        };
        claimed_targets.insert(resolved.clone());
        let file_name = resolved.file_name().unwrap_or_default().to_os_string();
        let dir = resolved
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| output_dir.to_path_buf());
        return Ok(PreparedTarget::Target {
            dir,
            file_name: Some(file_name),
        });
    }

    // --dir and --here unpack into output_dir as is
    // default mode makes a new directory named after the archive so report.txt.gz becomes report/report.txt
    let target = if output_dir_was_explicit || here {
        output_dir.to_path_buf()
    } else if is_archive {
        output_file_path.to_path_buf()
    } else {
        output_file_path.with_extension("")
    };
    let is_cwd = target == *INITIAL_CURRENT_DIR;
    // merging several inputs into the CWD is intentional so the claimed check skips it
    let claimed_by_earlier_input = !is_cwd && claimed_targets.contains(&target);
    let is_valid = !claimed_by_earlier_input
        && (is_cwd || !target.fs_err_try_exists()? || (target.is_dir() && target.read_dir()?.next().is_none()));

    let resolved = if is_valid {
        target
    } else if let Some(p) = resolve_path_conflict(&target, question_policy, QuestionAction::Decompression)? {
        p
    } else {
        return Ok(PreparedTarget::Cancelled);
    };

    if !resolved.fs_err_try_exists()? {
        fs::create_dir(&resolved)?;
    }
    claimed_targets.insert(resolved.clone());

    let file_name = (!is_archive).then(|| output_file_path.file_name().unwrap_or_default().to_os_string());
    Ok(PreparedTarget::Target {
        dir: resolved,
        file_name,
    })
}

enum DecompressionSummary {
    Archive { files_unpacked: u64, output_path: PathBuf },
    NonArchive { output_path: PathBuf },
}

/// Decompress (or unpack) a compressed (or packed) file.
pub fn decompress_file(options: DecompressOptions) -> Result<()> {
    assert!(options.output_dir.fs_err_try_exists()?);

    let input_is_stdin = is_path_stdin(options.input_file_path);
    let (first_extension, extensions) = split_first_compression_format(&options.formats);

    // Grab previous decoder and wrap it inside of a new one
    let chain_reader_decoder = |format: &CompressionFormat, decoder: Box<dyn Read>| -> Result<Box<dyn Read>> {
        let decoder: Box<dyn Read> = match format {
            Gzip => Box::new(flate2::read::MultiGzDecoder::new(decoder)),
            Bzip => Box::new(bzip2::read::MultiBzDecoder::new(decoder)),
            Bzip3 => {
                #[cfg(not(feature = "bzip3"))]
                return Err(crate::Error::bzip3_no_support());
                #[cfg(feature = "bzip3")]
                Box::new(bzip3::read::Bz3Decoder::new(decoder)?)
            }
            Lz4 => Box::new(MultiFrameLz4Decoder::new(decoder)),
            Lzma => Box::new(lzma_rust2::LzmaReader::new_mem_limit(
                decoder,
                LZMA_MEMLIMIT_BYTES,
                None,
            )?),
            Xz => Box::new(lzma_rust2::XzReader::new(decoder, true)),
            Lzip => Box::new(lzma_rust2::LzipReader::new(decoder)),
            Snappy => Box::new(snap::read::FrameDecoder::new(decoder)),
            Zstd => Box::new(zstd::stream::Decoder::new(decoder)?),
            Brotli => Box::new(brotli::Decompressor::new(decoder, BUFFER_CAPACITY)),
            Tar | Zip | Rar | SevenZip => unreachable!(),
        };
        Ok(decoder)
    };

    let create_decoder_up_to_first_extension = || -> Result<Box<dyn Read>> {
        let mut reader: Box<dyn Read> = if input_is_stdin {
            Box::new(io::stdin())
        } else {
            Box::new(BufReader::with_capacity(
                BUFFER_CAPACITY,
                fs::File::open(options.input_file_path)?,
            ))
        };

        for format in extensions.iter().rev() {
            reader = chain_reader_decoder(format, reader)?;
        }

        Ok(reader)
    };

    // Return early on a cancelled target
    let PreparedTarget::Target { dir, file_name } = options.prepared else {
        return Ok(());
    };

    let control_flow = match first_extension {
        Gzip | Bzip | Bzip3 | Lz4 | Lzma | Xz | Lzip | Snappy | Zstd | Brotli => {
            let reader = create_decoder_up_to_first_extension()?;
            let reader = chain_reader_decoder(&first_extension, reader)?;
            // Bomb cap: abort if decompressed output exceeds OUCH_MAX_DECOMPRESSED_BYTES
            let mut reader = LimitedReader::new(reader);

            // Non-archive targets always have a file name
            let Some(file_name) = file_name else {
                unreachable!("a non-archive target always has a file name");
            };

            let final_output_path = dir.join(file_name);
            let mut writer = fs::File::create(&final_output_path)?;
            io::copy(&mut reader, &mut writer)?;
            ControlFlow::Continue(DecompressionSummary::NonArchive {
                output_path: final_output_path,
            })
        }
        Tar => unpack_archive(
            |output_dir| {
                let reader = LimitedReader::new(create_decoder_up_to_first_extension()?);
                crate::archive::tar::unpack_archive(reader, output_dir)
            },
            dir,
        )?,
        Zip | SevenZip => {
            let unpack_fn = match first_extension {
                Zip => crate::archive::zip::unpack_archive,
                SevenZip => crate::archive::sevenz::unpack_archive,
                _ => unreachable!(),
            };

            let should_load_everything_into_memory = input_is_stdin || !extensions.is_empty();

            // due to `io::Seek` being required by `Zip` and `SevenZip`, we might have to
            // copy all contents into a Vec to pass an `io::Cursor` (impls Seek)
            let reader: Box<dyn ReadSeek> = if should_load_everything_into_memory {
                let memory_warning_fn = match first_extension {
                    Zip => warn_user_about_loading_zip_in_memory,
                    SevenZip => warn_user_about_loading_sevenz_in_memory,
                    _ => unreachable!(),
                };

                // Make thread own locks to keep output messages adjacent
                let locks = lock_and_flush_output_stdio();
                memory_warning_fn();
                if !user_wants_to_continue(
                    options.input_file_path,
                    options.question_policy,
                    QuestionAction::Decompression,
                )? {
                    return Ok(());
                }
                drop(locks);

                let mut vec = vec![];
                // Bomb cap: abort if the in-memory decompressed image exceeds the limit
                copy_limited_decompression(create_decoder_up_to_first_extension()?, &mut vec)?;
                Box::new(io::Cursor::new(vec))
            } else {
                Box::new(BufReader::with_capacity(
                    BUFFER_CAPACITY,
                    fs::File::open(options.input_file_path)?,
                ))
            };

            unpack_archive(|output_dir| unpack_fn(reader, output_dir, options.password), dir)?
        }
        #[cfg(feature = "unrar")]
        Rar => {
            let unpack_fn: Box<dyn FnOnce(&Path) -> Result<u64>> = if options.formats.len() > 1 || input_is_stdin {
                // unrar's C API needs a path; spill the decoded stream into the wrapper dir.
                let mut temp_file = tempfile::Builder::new().prefix(".ouch-rar-").tempfile_in(&dir)?;
                copy_limited_decompression(create_decoder_up_to_first_extension()?, &mut temp_file)?;
                Box::new(move |output_dir| {
                    crate::archive::rar::unpack_archive(temp_file.path(), output_dir, options.password)
                })
            } else {
                Box::new(|output_dir| {
                    crate::archive::rar::unpack_archive(options.input_file_path, output_dir, options.password)
                })
            };

            unpack_archive(unpack_fn, dir)?
        }
        #[cfg(not(feature = "unrar"))]
        Rar => {
            return Err(crate::Error::rar_no_support());
        }
    };

    let ControlFlow::Continue(decompression_summary) = control_flow else {
        return Ok(());
    };

    match decompression_summary {
        DecompressionSummary::Archive {
            files_unpacked,
            output_path,
        } => {
            // In default mode (no --dir, no --here), if the wrapper subdir we created
            // ended up containing exactly one entry whose name matches the wrapper itself
            // (e.g. `archive.zip` contained a single `archive/` root), flatten that
            // duplicate so the user sees `./archive/...` not `./archive/archive/...`.
            // Leave the nested layout if the rename fails.
            if !options.output_dir_was_explicit && !options.here {
                let _ = deduplicate_basename_wrapper(&output_path);
            }
            info_accessible!("Successfully decompressed archive to {}", PathFmt(&output_path));
            info_accessible!("Files unpacked: {files_unpacked}");
        }
        DecompressionSummary::NonArchive { output_path } => {
            if input_is_stdin {
                info_accessible!("STDIN decompressed to {}", PathFmt(&output_path));
            } else {
                info_accessible!(
                    "File {} decompressed to {}",
                    PathFmt(options.input_file_path),
                    PathFmt(&output_path),
                );
                info_accessible!("Input file size: {}", BytesFmt(file_size(options.input_file_path)?));
            }
            info_accessible!("Output file size: {}", BytesFmt(file_size(&output_path)?));
        }
    }

    if !input_is_stdin && options.remove {
        fs::remove_file(options.input_file_path)?;
        info!("Removed input file {}", PathFmt(options.input_file_path));
    }

    Ok(())
}

/// Run the format-specific unpack closure against an already-prepared target dir.
fn unpack_archive(
    unpack_fn: impl FnOnce(&Path) -> Result<u64>,
    output_dir: PathBuf,
) -> Result<ControlFlow<(), DecompressionSummary>> {
    let files_unpacked = unpack_fn(&output_dir)?;
    Ok(ControlFlow::Continue(DecompressionSummary::Archive {
        files_unpacked,
        output_path: output_dir,
    }))
}

/// Expects `wrapper` to be a just-decompressed archive output directory, if
/// `wrapper` contains exactly one entry with the same name (e.g. extracting
/// `archive.zip` produced `archive/archive/...`), then flatten it to `archive/...`.
fn deduplicate_basename_wrapper(wrapper: &Path) -> Result<()> {
    let Some(wrapper_name) = wrapper.file_name() else {
        return Ok(());
    };

    let only_file_in_dir = {
        // Read at most two entries. A single-entry directory has exactly one.
        let mut entries = fs::read_dir(wrapper)?;
        let Some(first_file) = entries.next().transpose()? else {
            return Ok(());
        };
        // More than one entry so do not deduplicate.
        if entries.next().transpose()?.is_some() {
            return Ok(());
        }
        first_file
    };

    // Name does not match so nothing to deduplicate.
    if only_file_in_dir.file_name() != wrapper_name {
        return Ok(());
    }

    // Only collapse when the inner duplicate is itself a directory.
    if !only_file_in_dir.file_type()?.is_dir() {
        return Ok(());
    }

    // Promote inner entries one level up, all writes stay inside `wrapper`.
    let inner_dir = only_file_in_dir.path();

    let mut staged: Option<tempfile::TempPath> = None;
    for entry in fs::read_dir(&inner_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == wrapper_name {
            // Stage under a random name to avoid colliding with the still-existing inner dir.
            let tmp = tempfile::Builder::new()
                .prefix(".ouch-")
                .tempfile_in(wrapper)?
                .into_temp_path();
            fs::remove_file(&tmp)?;
            fs::rename(entry.path(), &tmp)?;
            staged = Some(tmp);
        } else {
            fs::rename(entry.path(), wrapper.join(name))?;
        }
    }

    fs::remove_dir(&inner_dir)?;
    if let Some(tmp) = staged {
        fs::rename(&tmp, wrapper.join(wrapper_name))?;
        let _ = tmp.keep();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs as std_fs;

    use tempfile::tempdir;

    use super::*;

    /// Collect sorted relative paths of every entry under root.
    fn list_tree(root: &Path) -> Vec<String> {
        fn walk(p: &Path, base: &Path, out: &mut Vec<String>) {
            if let Ok(entries) = std_fs::read_dir(p) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let rel = path.strip_prefix(base).unwrap().to_string_lossy().replace('\\', "/");
                    if path.is_dir() {
                        out.push(format!("{rel}/"));
                        walk(&path, base, out);
                    } else {
                        out.push(rel);
                    }
                }
            }
        }
        let mut out = Vec::new();
        walk(root, root, &mut out);
        out.sort();
        out
    }

    /// The main case: wrapper contains exactly one entry whose name equals the wrapper's
    /// name. The inner entry should be promoted up one level. The flatten must not
    /// create or touch anything in the wrapper's parent directory.
    #[test]
    fn deduplicate_flattens_when_inner_dir_matches_wrapper_name() {
        let dir = tempdir().unwrap();
        let wrapper = dir.path().join("archive");
        let inner = wrapper.join("archive");
        std_fs::create_dir_all(&inner).unwrap();
        std_fs::write(inner.join("a.txt"), "a").unwrap();
        std_fs::write(inner.join("b.txt"), "b").unwrap();

        deduplicate_basename_wrapper(&wrapper).unwrap();
        assert_eq!(list_tree(&wrapper), vec!["a.txt", "b.txt"]);
        // Parent must be untouched: only `wrapper` itself should be visible there.
        let parent_entries: Vec<_> = std_fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(parent_entries, vec![std::ffi::OsString::from("archive")]);
    }

    /// A single inner entry with a different name stays nested.
    #[test]
    fn deduplicate_keeps_wrapper_when_inner_name_differs() {
        let dir = tempdir().unwrap();
        let wrapper = dir.path().join("archive");
        let inner = wrapper.join("mytool");
        std_fs::create_dir_all(&inner).unwrap();
        std_fs::write(inner.join("file.txt"), "x").unwrap();

        deduplicate_basename_wrapper(&wrapper).unwrap();
        assert_eq!(list_tree(&wrapper), vec!["mytool/", "mytool/file.txt"]);
    }

    /// More than one entry means no flatten.
    #[test]
    fn deduplicate_keeps_wrapper_when_multiple_entries() {
        let dir = tempdir().unwrap();
        let wrapper = dir.path().join("archive");
        std_fs::create_dir_all(&wrapper).unwrap();
        std_fs::write(wrapper.join("a.txt"), "a").unwrap();
        std_fs::write(wrapper.join("b.txt"), "b").unwrap();

        deduplicate_basename_wrapper(&wrapper).unwrap();
        assert_eq!(list_tree(&wrapper), vec!["a.txt", "b.txt"]);
    }

    /// An empty wrapper is left alone.
    #[test]
    fn deduplicate_is_noop_on_empty_wrapper() {
        let dir = tempdir().unwrap();
        let wrapper = dir.path().join("archive");
        std_fs::create_dir(&wrapper).unwrap();

        deduplicate_basename_wrapper(&wrapper).unwrap();
        assert!(wrapper.is_dir());
        assert_eq!(list_tree(&wrapper), Vec::<String>::new());
    }

    /// Wrapper contains a single entry, but it's a file (not a directory).
    /// No flatten should happen — the wrapper survives and the file stays inside.
    #[test]
    fn deduplicate_keeps_wrapper_when_inner_is_file() {
        let dir = tempdir().unwrap();
        let wrapper = dir.path().join("archive");
        std_fs::create_dir(&wrapper).unwrap();
        std_fs::write(wrapper.join("archive"), "data").unwrap();

        deduplicate_basename_wrapper(&wrapper).unwrap();
        assert!(wrapper.is_dir(), "wrapper must remain a directory");
        assert_eq!(list_tree(&wrapper), vec!["archive".to_string()]);
        assert_eq!(std_fs::read(wrapper.join("archive")).unwrap(), b"data");
    }

    /// Inner dir's children include one that shares the wrapper's name. The
    /// in-place flatten must stage it under a random name to avoid the collision
    /// and end with `wrapper/archive` containing the collision's contents.
    #[test]
    fn deduplicate_handles_inner_entry_named_like_wrapper() {
        let dir = tempdir().unwrap();
        let wrapper = dir.path().join("archive");
        let inner = wrapper.join("archive");
        let collision = inner.join("archive");
        std_fs::create_dir_all(&collision).unwrap();
        std_fs::write(collision.join("inside"), "x").unwrap();
        std_fs::write(inner.join("other.txt"), "o").unwrap();

        deduplicate_basename_wrapper(&wrapper).unwrap();
        assert_eq!(list_tree(&wrapper), vec!["archive/", "archive/inside", "other.txt"]);
        assert_eq!(std_fs::read(wrapper.join("archive").join("inside")).unwrap(), b"x");
        assert_eq!(std_fs::read(wrapper.join("other.txt")).unwrap(), b"o");
    }

    /// Only the outer wrapper collapses and nested same-name folders stay.
    #[test]
    fn deduplicate_only_flattens_outer_wrapper_not_inner_duplicates() {
        let dir = tempdir().unwrap();
        let wrapper = dir.path().join("testing");
        let outer_inner = wrapper.join("testing");
        let nested = outer_inner.join("testing");
        std_fs::create_dir_all(&nested).unwrap();
        std_fs::write(nested.join("file"), "deep").unwrap();

        deduplicate_basename_wrapper(&wrapper).unwrap();
        // After one flatten testing/testing/file remains since only the outer wrapper collapses.
        assert_eq!(list_tree(&wrapper), vec!["testing/", "testing/file"]);
    }
}
