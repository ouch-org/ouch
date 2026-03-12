//! Contains Tar-specific building and unpacking functions

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::HashMap,
    env,
    io::{self, prelude::*},
    ops::Not,
    path::{Path, PathBuf},
};

use fs_err::{self as fs};
use same_file::Handle;

use crate::{
    Result,
    error::FinalError,
    info,
    list::FileInArchive,
    utils::{
        self, BytesFmt, FileType, FileVisibilityPolicy, PathFmt, canonicalize, create_symlink, is_same_file_as_output,
        read_file_type, set_permission_mode,
    },
    warning,
};

/// Unpacks the archive given by `archive` into the folder given by `into`.
/// Assumes that output_folder is empty
pub fn unpack_archive(reader: impl Read, output_folder: &Path) -> Result<u64> {
    let mut archive = tar::Archive::new(reader);

    let mut files_unpacked = 0;
    let mut read_only_dirs_and_modes = Vec::new();

    for file in archive.entries()? {
        let mut file = file?;

        match file.header().entry_type() {
            tar::EntryType::Symlink => {
                let relative_path = file.path()?;
                let full_path = output_folder.join(&relative_path);
                let target = file
                    .link_name()?
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing symlink target"))?;

                create_symlink(&target, &full_path)?;
            }
            tar::EntryType::Link => {
                let link_path = file.path()?;
                let target = file
                    .link_name()?
                    .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing hardlink target"))?;

                let full_link_path = output_folder.join(&link_path);
                let full_target_path = output_folder.join(&target);

                fs::hard_link(&full_target_path, &full_link_path)?;
            }
            tar::EntryType::Regular => {
                file.unpack_in(output_folder)?;
            }
            tar::EntryType::Directory => {
                let original_mode = file.header().mode()?;
                let is_writeable = (original_mode & 0o200) != 0;

                // this is no-op when dir already exists, errs if a file with another type is found there
                file.unpack_in(output_folder)?;

                if cfg!(unix) && is_writeable.not() {
                    // We unpacked a read-only directory, make it writeable so that we can
                    // create the files inside of it, by the end, restore the original mode
                    let original_path = file.path()?.to_path_buf();
                    let unpacked = output_folder.join(&original_path);
                    set_permission_mode(&unpacked, original_mode | 0o200)?;

                    read_only_dirs_and_modes.push((original_path, original_mode));
                }
            }
            _ => continue,
        }

        info!(
            "extracted ({}) {:?}",
            BytesFmt(file.size()),
            PathFmt(&output_folder.join(file.path()?)),
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
                warning!("Cannot compress {:?} into itself, skipping", PathFmt(output_path));
                continue;
            }

            info!("Compressing {:?}", PathFmt(&path));

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
                                .detail(format!("Error appending hard link {:?}: {err}", PathFmt(&path)))
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
        env::set_current_dir(previous_location)?;
    }

    Ok(builder.into_inner()?)
}
