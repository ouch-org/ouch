//! Contains RAR-specific building and unpacking functions

use std::path::Path;

use unrar::Archive;

use crate::{error::Error, list::FileInArchive, utils::logger::info};

/// Unpacks the archive given by `archive_path` into the folder given by `output_folder`.
/// Assumes that output_folder is empty
pub fn unpack_archive(
    archive_path: &Path,
    output_folder: &Path,
    password: Option<impl AsRef<[u8]>>,
    quiet: bool,
) -> crate::Result<usize> {
    assert!(output_folder.read_dir().expect("dir exists").count() == 0);

    let password = password.as_ref().map(|p| p.as_ref());

    let archive = match password {
        Some(password) => Archive::with_password(archive_path, password),
        None => Archive::new(archive_path),
    };

    let mut archive = archive.open_for_processing()?;
    let mut unpacked = 0;

    while let Some(header) = archive.read_header()? {
        let entry = header.entry();
        archive = if entry.is_file() {
            if !quiet {
                info(format!(
                    "{} extracted. ({})",
                    entry.filename.display(),
                    entry.unpacked_size
                ));
            }
            unpacked += 1;
            header.extract_with_base(output_folder)?
        } else {
            header.skip()?
        };
    }

    Ok(unpacked)
}

/// List contents of `archive_path`, returning a vector of archive entries
pub fn list_archive(
    archive_path: &Path,
    password: Option<impl AsRef<[u8]>>,
) -> impl Iterator<Item = crate::Result<FileInArchive>> {
    let password = password.as_ref().map(|p| p.as_ref());
    let archive = match password {
        Some(password) => Archive::with_password(archive_path, password),
        None => Archive::new(archive_path),
    };
    archive.open_for_listing().expect("cannot open archive").map(|item| {
        let item = item?;
        let is_dir = item.is_directory();
        let path = item.filename;

        Ok(FileInArchive { path, is_dir })
    })
}

pub fn no_compression() -> Error {
    Error::UnsupportedFormat {
        reason: "Creating RAR archives is not allowed due to licensing restrictions.".into(),
    }
}
