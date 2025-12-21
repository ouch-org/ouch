//! Contains Ar-specific building and unpacking functions

use std::{
    env,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver},
    thread,
};

use fs_err as fs;
use same_file::Handle;

use crate::{
    commands::Unpacked,
    error::FinalError,
    info,
    list::FileInArchive,
    utils::{self, Bytes, EscapedPathDisplay, FileVisibilityPolicy},
    warning,
};

/// Unpacks an ar archive into the given output folder
pub fn unpack_archive(reader: Box<dyn Read>, output_folder: &Path) -> crate::Result<Unpacked> {
    let mut archive = ar::Archive::new(reader);
    let mut files_unpacked = 0;

    while let Some(entry_result) = archive.next_entry() {
        let mut entry = entry_result.map_err(|e| {
            FinalError::with_title("Failed to read ar archive").detail(format!("Error reading ar entry: {e}"))
        })?;

        let identifier = String::from_utf8_lossy(entry.header().identifier())
            .trim_end_matches('/')
            .to_string();

        // Skip empty identifiers
        if identifier.is_empty() {
            continue;
        }

        let output_path = output_folder.join(&identifier);

        // Create parent directories if needed
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Extract the file
        let mut output_file = fs::File::create(&output_path)?;
        let size = std::io::copy(&mut entry, &mut output_file)?;

        info!(
            "extracted ({}) {:?}",
            Bytes::new(size),
            utils::strip_cur_dir(&output_path),
        );

        files_unpacked += 1;
    }

    Ok(Unpacked {
        files_unpacked,
        read_only_directories: Vec::new(),
    })
}

/// List contents of an ar archive
pub fn list_archive<R: Read + Send + 'static>(reader: R) -> impl Iterator<Item = crate::Result<FileInArchive>> {
    struct Files(Receiver<crate::Result<FileInArchive>>);
    impl Iterator for Files {
        type Item = crate::Result<FileInArchive>;

        fn next(&mut self) -> Option<Self::Item> {
            self.0.recv().ok()
        }
    }

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut archive = ar::Archive::new(reader);

        while let Some(entry_result) = archive.next_entry() {
            let entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    let _ = tx.send(Err(FinalError::with_title("Failed to read ar archive")
                        .detail(format!("Error reading ar entry: {e}"))
                        .into()));
                    break;
                }
            };

            let identifier = String::from_utf8_lossy(entry.header().identifier())
                .trim_end_matches('/')
                .to_string();

            // Skip empty identifiers
            if identifier.is_empty() {
                continue;
            }

            let file = FileInArchive {
                path: identifier.into(),
                is_dir: false,
            };

            if tx.send(Ok(file)).is_err() {
                break;
            }
        }
    });

    Files(rx)
}

/// Compresses the files given by `input_filenames` into an ar archive written to `writer`.
pub fn build_archive_from_paths<W>(
    input_filenames: &[PathBuf],
    output_path: &Path,
    writer: W,
    file_visibility_policy: FileVisibilityPolicy,
) -> crate::Result<W>
where
    W: Write,
{
    let mut builder = ar::Builder::new(writer);
    let output_handle = Handle::from_path(output_path);

    for filename in input_filenames {
        let previous_location = utils::cd_into_same_dir_as(filename)?;

        // Unwrap safety:
        //   paths should be canonicalized by now, and the root directory rejected.
        let filename = filename.file_name().unwrap();

        for entry in file_visibility_policy.build_walker(filename) {
            let entry = entry?;
            let path = entry.path();

            // If the output_path is the same as the input file, warn the user and skip
            if let Ok(handle) = &output_handle {
                if matches!(Handle::from_path(path), Ok(x) if &x == handle) {
                    warning!("Cannot compress `{}` into itself, skipping", output_path.display());
                    continue;
                }
            }

            // ar archives only support regular files, skip directories and symlinks
            if !path.is_file() || path.is_symlink() {
                continue;
            }

            info!("Compressing '{}'", EscapedPathDisplay::new(path));

            let file = match fs::File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound && path.is_symlink() {
                        // Broken symlink, skip
                        continue;
                    }
                    return Err(e.into());
                }
            };

            // Get file name for the archive entry
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "unknown".to_string());

            // ar crate requires std::fs::File, so we get the inner file
            let mut std_file = file.into_parts().0;

            builder
                .append_file(file_name.as_bytes(), &mut std_file)
                .map_err(|err| {
                    FinalError::with_title("Could not create ar archive")
                        .detail(format!("Error adding file '{}': {err}", path.display()))
                })?;
        }
        env::set_current_dir(previous_location)?;
    }

    Ok(builder.into_inner()?)
}
