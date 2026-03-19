//! Contains Tar-specific building and unpacking functions

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::HashMap,
    io::{self, Read, Write},
    ops::Not,
    path::{Path, PathBuf},
};

use fs_err as fs;
use same_file::Handle;

use crate::{
    Error, Result,
    archive::{ConflictResolution, ConflictResolver},
    error::FinalError,
    info,
    list::FileInArchive,
    utils::{
        self, BytesFmt, FileType, FileVisibilityPolicy, PathFmt, UnpackEntryType, canonicalize, create_symlink,
        current_dir, is_same_file_as_output, read_file_type, set_current_dir, set_permission_mode,
    },
    warning,
};

/// Unpacks the archive given by `archive` into the folder given by `into`.
/// Assumes that output_folder is empty
pub fn unpack_archive(reader: impl Read, output_folder: &Path) -> Result<u64> {
    let resolver = ConflictResolver::new(output_folder);
    let mut archive = tar::Archive::new(reader);

    let mut files_unpacked = 0;
    let mut read_only_dirs_and_modes = Vec::new();

    for entry in archive.entries()? {
        let mut entry = entry?;

        let relative_path = entry.path()?.into_owned();
        let entry_type: UnpackEntryType = entry.header().entry_type().try_into()?;

        let solution = resolver.resolve_archive_path_and_check_conflicts(&relative_path, entry_type)?;
        let resolved_path = match &solution {
            ConflictResolution::Proceed(path) => path,
            ConflictResolution::SkipEntry => continue,
        };

        match entry_type {
            // Entry::unpack allow us to point the destination where the file should go, but
            // for symlink and hardlinks, it doesn't find the correct target location
            UnpackEntryType::Symlink | UnpackEntryType::HardLink => {
                let is_symlink = matches!(entry_type, UnpackEntryType::Symlink);

                let target = entry
                    .link_name()?
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing hardlink target"))?;

                if is_symlink {
                    // Just a sanity check, discard the resolved path
                    let _ = resolver.resolve_archive_path(target.as_ref())?;
                    create_symlink(&target, resolved_path)?;
                } else {
                    let unpacked_target_path = resolver.resolve_archive_path(target.as_ref())?;
                    fs::hard_link(&unpacked_target_path, resolved_path)?;
                }
            }
            _ => {
                entry
                    .unpack(resolved_path)
                    .map_err(|err| FinalError::with_title("Failed to unpack .tar entry").detail(err.to_string()))?;
            }
        }

        // TODO: make a test for this, I suspect it's not working
        if cfg!(unix)
            && let tar::EntryType::Directory = entry.header().entry_type()
        {
            let original_mode = entry.header().mode()?;
            let is_writeable = (original_mode & 0o200) != 0;

            if is_writeable.not() {
                // We unpacked a read-only directory, make it writeable so that we can
                // create the files inside of it, by the end, restore the original mode
                let unpacked = output_folder.join(resolved_path);
                set_permission_mode(&unpacked, original_mode | 0o200)?;

                read_only_dirs_and_modes.push((resolved_path.to_owned(), original_mode));
            }
        }

        info!(
            "extracted ({}) {}",
            BytesFmt(entry.size()),
            PathFmt(&output_folder.join(relative_path)),
        );
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
        let is_dir = file.header().entry_type().is_dir();
        Ok(FileInArchive { path, is_dir })
    });

    Ok(entries.collect::<Vec<_>>().into_iter())
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

        // Unwrap expectation:
        //   paths should be canonicalized by now, and the root directory rejected.
        let filename = explicit_path.file_name().unwrap();

        let iter = file_visibility_policy.workaround_build_walker_or_broken_link_path(explicit_path, filename);

        for entry in iter {
            let path = entry.unwrap();

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
                    let target_path = {
                        let target_path = path.read_link()?;
                        if !target_path.is_absolute() {
                            target_path
                        } else {
                            warning!(
                                "Symlink at {} has an absolute target, trying to relativize to make it valid.",
                                PathFmt(&path),
                            );
                            target_path
                                .strip_prefix(&*current_dir())
                                .map_err(|_| {
                                    FinalError::with_title("Failed to add symlink to archive")
                                        .detail(format!("symlink at path {}", PathFmt(&path)))
                                        .detail(format!("with target {}", PathFmt(&target_path)))
                                        .detail("points to an absolute path, which is forbidden for security reasons")
                                        .detail("relativizing failed, the path is outside of current directory")
                                })?
                                .to_owned()
                        }
                    };

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
        set_current_dir(previous_location)?;
    }

    Ok(builder.into_inner()?)
}

impl TryFrom<tar::EntryType> for UnpackEntryType {
    type Error = Error;

    fn try_from(value: tar::EntryType) -> Result<Self> {
        let result = match value {
            tar::EntryType::Regular | tar::EntryType::Continuous => Ok(Self::Regular),
            tar::EntryType::Directory => Ok(Self::Directory),
            tar::EntryType::Symlink => Ok(Self::Symlink),
            tar::EntryType::Link => Ok(Self::HardLink),
            tar::EntryType::Char => Ok(Self::Char),
            tar::EntryType::Block => Ok(Self::Block),
            tar::EntryType::Fifo => Ok(Self::Fifo),
            _ => {
                debug_assert!(false);
                Err("unknown/unsupported")
            }
        };

        result.map_err(|name| FinalError::with_title(format!("unsupported file type in tar archive: {name}")).into())
    }
}
