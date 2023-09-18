//! Contains Tar-specific building and unpacking functions

use std::{
    env,
    io::prelude::*,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
};

use fs_err as fs;
use same_file::Handle;

use crate::{
    error::FinalError,
    info,
    list::FileInArchive,
    utils::{self, Bytes, FileVisibilityPolicy},
    warning,
};

/// Unpacks the archive given by `archive` into the folder given by `into`.
/// Assumes that output_folder is empty
pub fn unpack_archive(reader: Box<dyn Read>, output_folder: &Path, quiet: bool) -> crate::Result<usize> {
    assert!(output_folder.read_dir().expect("dir exists").count() == 0);
    let mut archive = tar::Archive::new(reader);

    let mut files_unpacked = 0;
    for file in archive.entries()? {
        let mut file = file?;

        file.unpack_in(output_folder)?;

        // This is printed for every file in the archive and has little
        // importance for most users, but would generate lots of
        // spoken text for users using screen readers, braille displays
        // and so on
        if !quiet {
            info!(
                inaccessible,
                "{:?} extracted. ({})",
                utils::strip_cur_dir(&output_folder.join(file.path()?)),
                Bytes::new(file.size()),
            );

            files_unpacked += 1;
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
    quiet: bool,
) -> crate::Result<W>
where
    W: Write,
{
    let mut builder = tar::Builder::new(writer);
    let output_handle = Handle::from_path(output_path);

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
                    warning!(
                        "The output file and the input file are the same: `{}`, skipping...",
                        output_path.display()
                    );
                    continue;
                }
            }

            // This is printed for every file in `input_filenames` and has
            // little importance for most users, but would generate lots of
            // spoken text for users using screen readers, braille displays
            // and so on
            if !quiet {
                info!(inaccessible, "Compressing '{}'.", path.display());
            }

            if path.is_dir() {
                builder.append_dir(path, path)?;
            } else {
                let mut file = match fs::File::open(path) {
                    Ok(f) => f,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound && utils::is_symlink(path) {
                            // This path is for a broken symlink
                            // We just ignore it
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
        }
        env::set_current_dir(previous_location)?;
    }

    Ok(builder.into_inner()?)
}
