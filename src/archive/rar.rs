//! Contains RAR-specific building and unpacking functions

use std::path::{Path, PathBuf};

use unrar::{
    Archive, ExtractEvent,
    error::{Code, UnrarError, When},
};

use crate::{
    error::{Error, FinalError, Result},
    info,
    list::{FileInArchive, ListFileType},
    utils::{BytesFmt, PathFmt},
};

/// Unpacks the archive given by `archive_path` into the folder given by `output_folder`.
/// Assumes that output_folder is empty
pub fn unpack_archive(archive_path: &Path, output_folder: &Path, password: Option<&[u8]>) -> Result<u64> {
    let archive = match password {
        Some(password) => Archive::with_password(archive_path, password),
        None => Archive::new(archive_path),
    };

    let archive = archive.open_for_processing()?;

    let mut files_unpacked: u64 = 0;
    let mut first_err: Option<(PathBuf, i32)> = None;

    let cb_result = archive.extract_all_with_callback(output_folder, |event| match event {
        ExtractEvent::Ok { filename, size } => {
            info!("extracted ({}) {}", BytesFmt(size), PathFmt(&filename));
            files_unpacked += 1;
            true
        }
        ExtractEvent::Err { filename, error_code } => {
            first_err = Some((filename, error_code));
            // Returning false cancels the rest of the extraction so any
            // additional per-file errors don't get silently swallowed.
            false
        }
        ExtractEvent::LargeDictWarning {
            dict_size_kb,
            max_dict_size_kb,
        } => {
            info!(
                "archive requires {} KiB dictionary; this build supports up to {} KiB",
                dict_size_kb, max_dict_size_kb,
            );
            // Reject the oversized dictionary so the DLL fails the
            // operation with Code::LargeDict instead of silently
            // proceeding with a result it cannot actually produce.
            false
        }
        _ => true,
    });

    if let Some((path, code)) = first_err {
        let inner = UnrarError::from(Code::from(code), When::Process).to_string();
        return Err(Error::Custom {
            reason: FinalError::with_title(format!("failed to extract {}", PathFmt(&path))).detail(inner),
        });
    }
    let _status = cb_result?;
    Ok(files_unpacked)
}

/// List contents of `archive_path`, returning a vector of archive entries
pub fn list_archive(
    archive_path: &Path,
    password: Option<&[u8]>,
) -> Result<impl Iterator<Item = Result<FileInArchive>> + use<>> {
    let archive = match password {
        Some(password) => Archive::with_password(archive_path, password),
        None => Archive::new(archive_path),
    };

    Ok(archive.open_for_listing()?.map(|item| {
        let item = item?;
        let is_dir = item.is_directory();
        let path = item.filename;

        Ok(FileInArchive {
            path,
            file_type: if is_dir {
                ListFileType::Directory
            } else {
                ListFileType::File
            },
        })
    }))
}

pub fn no_compression() -> Error {
    Error::UnsupportedFormat {
        reason: "Creating RAR archives is not allowed due to licensing restrictions.".into(),
    }
}
