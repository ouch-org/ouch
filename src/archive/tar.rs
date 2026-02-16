//! Contains Tar-specific building and unpacking functions

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::HashMap,
    env,
    io::prelude::*,
    ops::Not,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
};

use fs_err::{self as fs};
use same_file::Handle;

use crate::{
    error::FinalError,
    info,
    list::FileInArchive,
    utils::{self, create_symlink, set_permission_mode, BytesFmt, EscapedPathDisplay, FileVisibilityPolicy},
    warning,
};

/// Unpacks the archive given by `archive` into the folder given by `into`.
/// Assumes that output_folder is empty
pub fn unpack_archive(reader: impl Read, output_folder: &Path) -> crate::Result<u64> {
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
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing symlink target"))?;

                create_symlink(&target, &full_path)?;
            }
            tar::EntryType::Link => {
                let link_path = file.path()?;
                let target = file
                    .link_name()?
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing hardlink target"))?;

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
            utils::strip_cur_dir(&output_folder.join(file.path()?)),
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
pub fn list_archive(
    mut archive: tar::Archive<impl Read + Send + 'static>,
) -> impl Iterator<Item = crate::Result<FileInArchive>> {
    struct Files(Receiver<crate::Result<FileInArchive>>);
    impl Iterator for Files {
        type Item = crate::Result<FileInArchive>;

        fn next(&mut self) -> Option<Self::Item> {
            self.0.recv().ok()
        }
    }

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for file in archive.entries().expect("entries is only used once") {
            let file_in_archive = (|| {
                let file = file?;
                let path = file.path()?.into_owned();
                let is_dir = file.header().entry_type().is_dir();
                Ok(FileInArchive { path, is_dir })
            })();
            tx.send(file_in_archive).unwrap();
        }
    });

    Files(rx)
}

/// Compresses the archives given by `input_filenames` into the file given previously to `writer`.
pub fn build_archive_from_paths<W>(
    input_filenames: &[PathBuf],
    output_path: &Path,
    writer: W,
    file_visibility_policy: FileVisibilityPolicy,
    follow_symlinks: bool,
) -> crate::Result<W>
where
    W: Write,
{
    let mut builder = tar::Builder::new(writer);
    let output_handle = Handle::from_path(output_path);
    let mut seen_inode: HashMap<(u64, u64), PathBuf> = HashMap::new();

    for filename in input_filenames {
        let previous_location = utils::cd_into_same_dir_as(filename)?;

        // Unwrap safety:
        //   paths should be canonicalized by now, and the root directory rejected.
        let filename = filename.file_name().unwrap();

        for entry in file_visibility_policy.build_walker(filename) {
            let entry = entry?;
            let path = entry.path();

            // If the output_path is the same as the input file, warn the user and skip the input (in order to avoid compression recursion)
            if let Ok(handle) = &output_handle {
                if matches!(Handle::from_path(path), Ok(x) if &x == handle) {
                    warning!("Cannot compress `{}` into itself, skipping", output_path.display());

                    continue;
                }
            }

            info!("Compressing '{}'", EscapedPathDisplay::new(path));

            let link_meta = path.symlink_metadata()?;

            if !follow_symlinks && link_meta.is_symlink() {
                let target_path = path.read_link()?;

                let mut header = tar::Header::new_gnu();
                header.set_entry_type(tar::EntryType::Symlink);
                header.set_size(0);

                builder.append_link(&mut header, path, &target_path).map_err(|err| {
                    FinalError::with_title("Could not create archive")
                        .detail("Unexpected error while trying to read link")
                        .detail(format!("Error: {err}."))
                })?;
                continue;
            }

            // TODO: to better support Windows hard links,
            // we should wait for this issue to be resolved:
            // https://github.com/rust-lang/rust/issues/63010
            #[cfg(unix)]
            if link_meta.nlink() > 1 && link_meta.is_file() {
                let key = (link_meta.dev(), link_meta.ino());

                match seen_inode.get(&key) {
                    Some(target_path) => {
                        let mut header = tar::Header::new_gnu();
                        header.set_entry_type(tar::EntryType::Link);
                        header.set_size(0);

                        builder.append_link(&mut header, path, target_path).map_err(|err| {
                            FinalError::with_title("Could not create archive").detail(format!(
                                "Error appending hard link '{}': {}",
                                path.display(),
                                err
                            ))
                        })?;
                    }
                    None => {
                        seen_inode.insert(key, path.to_path_buf());
                        let mut file = fs::File::open(path)?;
                        builder.append_file(path, file.file_mut())?
                    }
                }
                continue;
            }

            if path.is_dir() {
                builder.append_dir(path, path)?;
                continue;
            }

            let mut file = match fs::File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound && path.is_symlink() {
                        // This path is for a broken symlink, ignore it
                        continue;
                    }
                    return Err(e.into());
                }
            };
            builder.append_file(path, file.file_mut()).map_err(|err| {
                FinalError::with_title("Could not create archive")
                    .detail("Unexpected error while trying to read file")
                    .detail(format!("Error: {err}."))
            })?;
        }
        env::set_current_dir(previous_location)?;
    }

    Ok(builder.into_inner()?)
}
