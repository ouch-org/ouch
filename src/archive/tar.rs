//! Contains Tar-specific building and unpacking functions

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::HashMap,
    io::{self, BufReader, prelude::*},
    ops::Not,
    path::{Path, PathBuf},
};

use fs_err as fs;
use same_file::Handle;

use crate::{
    QuestionPolicy, Result,
    error::FinalError,
    info,
    list::{FileInArchive, ListFileType},
    utils::{
        self, BytesFmt, FileType, FileVisibilityPolicy, PathFmt, canonicalize, create_symlink, is_same_file_as_output,
        read_file_type, resolve_extraction_conflict, sanitize_archive_mode, set_permission_mode,
        validate_dest_inside_root, validate_entry_path, validate_symlink_target,
    },
    warning,
};

/// Magic bytes of common compression formats, checked against the start of a tar stream to
/// give a clearer error when the data is still compressed (one extra layer of compression
/// that the filename did not account for, e.g. an actual `.tar.gz.gz` misnamed `.tgz`).
const LEFTOVER_COMPRESSION_MAGIC: &[(&[u8], &str)] = &[
    (&[0x1f, 0x8b], "gzip"),
    (&[0x42, 0x5a, 0x68], "bzip2"),
    (&[0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00], "xz"),
    (&[0x28, 0xb5, 0x2f, 0xfd], "zstd"),
    (&[0x04, 0x22, 0x4d, 0x18], "lz4"),
];

/// If `peeked` starts with the magic bytes of a known compression format, return that
/// format's name so the caller can explain why tar parsing is failing.
fn detect_leftover_compression(peeked: &[u8]) -> Option<&'static str> {
    LEFTOVER_COMPRESSION_MAGIC
        .iter()
        .find(|(magic, _)| peeked.starts_with(magic))
        .map(|(_, name)| *name)
}

/// Unpacks the archive given by `archive` into the folder given by `into`.
/// Assumes that output_folder is empty
pub fn unpack_archive(reader: impl Read, output_folder: &Path, question_policy: QuestionPolicy) -> Result<u64> {
    let mut reader = BufReader::new(reader);
    if let Some(format) = detect_leftover_compression(reader.fill_buf()?) {
        return Err(FinalError::with_title("Could not extract archive")
            .detail(format!(
                "The decompressed data is still {format}-compressed instead of being a tar archive"
            ))
            .detail("This usually means the file was compressed one more time than its extension suggests")
            .hint("Rename the file to include the missing compression extension, or decompress the extra layer manually before extracting it")
            .into());
    }

    let mut archive = tar::Archive::new(reader);

    let mut files_unpacked = 0;
    let mut read_only_dirs_and_modes = Vec::new();

    for entry in archive.entries()? {
        let mut entry = entry?;

        // Set when the user renamed a file so the log can show the real path.
        let mut written = None;

        match entry.header().entry_type() {
            tar::EntryType::Symlink => {
                let raw_path = entry.path()?.into_owned();
                let safe_relpath = validate_entry_path(&raw_path)?;
                let full_path = output_folder.join(&safe_relpath);
                let target = entry
                    .link_name()?
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing symlink target"))?;

                validate_symlink_target(&safe_relpath, &target)?;
                validate_dest_inside_root(output_folder, &full_path)?;
                create_symlink(&target, &full_path)?;
            }
            tar::EntryType::Link => {
                let raw_link = entry.path()?.into_owned();
                let safe_link_path = validate_entry_path(&raw_link)?;
                let raw_target = entry
                    .link_name()?
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing hardlink target"))?
                    .into_owned();
                let safe_target = validate_entry_path(&raw_target)?;

                let full_link_path = output_folder.join(&safe_link_path);
                let full_target_path = output_folder.join(&safe_target);

                validate_dest_inside_root(output_folder, &full_link_path)?;
                validate_dest_inside_root(output_folder, &full_target_path)?;
                fs::hard_link(&full_target_path, &full_link_path)?;
            }
            tar::EntryType::Regular | tar::EntryType::GNUSparse => {
                let raw_path = entry.path()?.into_owned();
                let safe_relpath = validate_entry_path(&raw_path)?;
                let full_path = output_folder.join(&safe_relpath);

                let Some(dest) = resolve_extraction_conflict(&full_path, question_policy)? else {
                    continue;
                };

                if dest == full_path {
                    entry.unpack_in(output_folder)?;
                } else {
                    entry.unpack(&dest)?;
                }
                written = Some(dest);
            }
            tar::EntryType::Directory => {
                let original_mode = entry.header().mode()?;
                let is_writeable = (original_mode & 0o200) != 0;

                // this is no-op when dir already exists, errs if a file with another type is found there
                entry.unpack_in(output_folder)?;

                if cfg!(unix) && is_writeable.not() {
                    // We unpacked a read-only directory, make it writeable so that we can
                    // create the files inside of it, by the end, restore the original mode
                    let original_path = entry.path()?.to_path_buf();
                    let safe_relpath = validate_entry_path(&original_path)?;
                    let unpacked = output_folder.join(&safe_relpath);
                    set_permission_mode(&unpacked, sanitize_archive_mode(original_mode) | 0o200)?;

                    // Store the absolute path because the restore loop runs without changing directory.
                    read_only_dirs_and_modes.push((unpacked, sanitize_archive_mode(original_mode)));
                }
            }
            _ => continue,
        }

        let unpacked_path = match written {
            Some(path) => path,
            None => output_folder.join(entry.path()?),
        };

        if entry.header().entry_type().is_dir() {
            info!("Directory {} created", PathFmt(&unpacked_path));
        } else {
            info!("extracted ({}) {}", BytesFmt(entry.size()), PathFmt(&unpacked_path));
        }
        files_unpacked += 1;
    }

    // Restore original mode for read-only dirs we made writeable
    if cfg!(unix) {
        for (path, mode) in &read_only_dirs_and_modes {
            set_permission_mode(path, *mode)?;
        }
    }

    Ok(files_unpacked)
}

/// List contents of `archive`, returning a vector of archive entries
pub fn list_archive(mut archive: tar::Archive<impl Read>) -> Result<impl Iterator<Item = Result<FileInArchive>>> {
    let entries = archive.entries()?.map(|file| {
        let file = file?;
        let path = file.path()?.into_owned();
        let size = file.header().size().ok();
        let file_type = get_file_type(file.header(), &file)?;
        Ok(FileInArchive { path, file_type, size })
    });

    Ok(entries.collect::<Vec<_>>().into_iter())
}

fn get_file_type(header: &tar::Header, file: &tar::Entry<impl Read>) -> Result<ListFileType> {
    Ok(match header.entry_type() {
        tar::EntryType::Directory => ListFileType::Directory,
        tar::EntryType::Symlink => file
            .link_name()?
            .map(|t| ListFileType::Symlink { target: t.into_owned() })
            .unwrap_or(ListFileType::File),
        tar::EntryType::Link => file
            .link_name()?
            .map(|t| ListFileType::Hardlink { target: t.into_owned() })
            .unwrap_or(ListFileType::File),
        _ => ListFileType::File,
    })
}

/// Compresses the archives given by `input_filenames` into the file given previously to `writer`.
pub fn build_archive<W>(
    explicit_paths: &[PathBuf],
    output_path: &Path,
    writer: W,
    file_visibility_policy: FileVisibilityPolicy,
    follow_symlinks: bool,
) -> Result<W>
where
    W: Write,
{
    let mut builder = tar::Builder::new(writer);
    let output_handle = Handle::from_path(output_path);
    let mut seen_inode: HashMap<(u64, u64), PathBuf> = HashMap::new();

    for explicit_path in explicit_paths {
        let previous_location = utils::cd_into_same_dir_as(explicit_path)?;
        let _cwd_guard = utils::CwdGuard::new(previous_location);

        // Unwrap expectation:
        //   paths should be canonicalized by now, and the root directory rejected.
        let filename = explicit_path.file_name().unwrap();

        let iter = file_visibility_policy.workaround_build_walker_or_broken_link_path(explicit_path, filename);

        for entry in iter {
            let path = entry?;

            // Avoid compressing the output file into itself
            if let Ok(handle) = output_handle.as_ref()
                && is_same_file_as_output(&path, handle)
            {
                warning!("Cannot compress {} into itself, skipping", PathFmt(output_path));
                continue;
            }

            info!("Compressing {}", PathFmt(&path));

            let (metadata, file_type) = {
                if follow_symlinks {
                    (path.metadata()?, read_file_type(canonicalize(&path)?)?)
                } else {
                    (path.symlink_metadata()?, read_file_type(&path)?)
                }
            };

            // Treat unix hardlinks (ignore directory, since user-created directory hard links are
            // not a thing)
            //
            // TODO: to better support Windows hard links,
            // we should wait for this issue to be resolved:
            // https://github.com/rust-lang/rust/issues/63010
            #[cfg(unix)]
            if metadata.nlink() > 1 && !file_type.is_directory() {
                let inode_identifier = (metadata.dev(), metadata.ino());

                match seen_inode.get(&inode_identifier) {
                    Some(target_path) => {
                        let mut header = tar::Header::new_gnu();
                        header.set_entry_type(tar::EntryType::Link);
                        header.set_size(0);

                        builder.append_link(&mut header, &path, target_path).map_err(|err| {
                            FinalError::with_title("Could not create archive")
                                .detail(format!("Error appending hard link {}: {err}", PathFmt(&path)))
                        })?;
                        continue; // skip handling this file
                    }
                    None => {
                        // First time we see this file, let it be processed normally by the
                        // code below, but save it to this hashmap
                        seen_inode.insert(inode_identifier, path.to_path_buf());
                    }
                }
            }

            match file_type {
                FileType::Regular => {
                    let mut file = fs::File::open(&path)?;
                    builder.append_file(&path, file.file_mut()).map_err(|err| {
                        FinalError::with_title("Could not create archive")
                            .detail("Unexpected error while trying to read file")
                            .detail(format!("Error: {err}"))
                    })?;
                }
                FileType::Directory => {
                    builder.append_dir(&path, &path)?;
                }
                FileType::Symlink => {
                    let target_path = path.read_link()?;

                    let mut header = tar::Header::new_gnu();
                    header.set_entry_type(tar::EntryType::Symlink);
                    header.set_size(0);

                    builder.append_link(&mut header, &path, &target_path).map_err(|err| {
                        FinalError::with_title("Could not create archive")
                            .detail("Unexpected error while trying to read link")
                            .detail(format!("Error: {err}"))
                    })?;
                }
            }
        }
    }

    Ok(builder.into_inner()?)
}
