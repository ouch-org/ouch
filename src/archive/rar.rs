//! Contains RAR-specific building and unpacking functions

use std::path::Path;

use unrar::Archive;

use crate::{
    error::{Error, Result},
    info,
    list::FileInArchive,
    utils::Bytes,
};

/// Unpacks the archive given by `archive_path` into the folder given by `output_folder`.
/// Assumes that output_folder is empty
pub fn unpack_archive(archive_path: &Path, output_folder: &Path, password: Option<&[u8]>) -> crate::Result<u64> {
    let archive = match password {
        Some(password) => Archive::with_password(archive_path, password),
        None => Archive::new(archive_path),
    };

    let mut archive = archive.open_for_processing()?;
    let mut files_unpacked = 0;

    while let Some(header) = archive.read_header()? {
        let entry = header.entry();
        archive = if entry.is_file() {
            info!(
                "extracted ({}) {}",
                Bytes::new(entry.unpacked_size),
                entry.filename.display(),
            );
            files_unpacked += 1;
            header.extract_with_base(output_folder)?
        } else {
            header.skip()?
        };
    }

    Ok(files_unpacked)
}

/// List contents of `archive_path`, returning a vector of archive entries
pub fn list_archive(
    archive_path: &Path,
    password: Option<&[u8]>,
) -> Result<impl Iterator<Item = Result<FileInArchive>>> {
    let archive = match password {
        Some(password) => Archive::with_password(archive_path, password),
        None => Archive::new(archive_path),
    };

    Ok(archive.open_for_listing()?.map(|item| {
        let item = item?;
        let is_dir = item.is_directory();
        let path = item.filename;

        Ok(FileInArchive { path, is_dir })
    }))
}

pub fn no_compression() -> Error {
    Error::UnsupportedFormat {
        reason: "Creating RAR archives is not allowed due to licensing restrictions.".into(),
    }
}
