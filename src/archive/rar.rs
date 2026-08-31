//! Contains RAR-specific building and unpacking functions

use std::path::{Path, PathBuf};

use fs_err as fs;
use unrar::{
    Archive, ExtractEvent,
    error::{Code, UnrarError, When},
};

use crate::{
    QuestionPolicy,
    error::{Error, FinalError, Result},
    info,
    list::{FileInArchive, ListFileType},
    utils::{BytesFmt, PathFmt, resolve_extraction_conflict, validate_entry_path},
    warning,
};

/// Unpacks the archive into `output_folder` and asks before replacing files.
pub fn unpack_archive(
    archive_path: &Path,
    output_folder: &Path,
    password: Option<&[u8]>,
    question_policy: QuestionPolicy,
) -> Result<u64> {
    // Rar reference records need a full extraction pass to resolve.
    fs::create_dir_all(output_folder)?;
    let staging = tempfile::Builder::new()
        .prefix(".ouch-rar-")
        .tempdir_in(output_folder)?;
    extract_all(archive_path, staging.path(), password)?;
    move_into_place(staging.path(), staging.path(), output_folder, question_policy)
}

/// Move each staged entry into `output_folder` at the same relative path.
fn move_into_place(root: &Path, dir: &Path, output_folder: &Path, question_policy: QuestionPolicy) -> Result<u64> {
    let mut files_unpacked = 0;
    for entry in fs::read_dir(dir)? {
        let source = entry?.path();
        let dest = output_folder.join(source.strip_prefix(root).expect("child of staging root"));

        if fs::symlink_metadata(&source)?.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|err| Error::Custom {
                reason: FinalError::with_title(format!("failed to create {}", PathFmt(&dest))).detail(err.to_string()),
            })?;
            files_unpacked += move_into_place(root, &source, output_folder, question_policy)?;
        } else if let Some(target) = resolve_extraction_conflict(&dest, question_policy)? {
            let size = fs::symlink_metadata(&source)?.len();
            std::fs::rename(&source, &target).map_err(|err| Error::Custom {
                reason: FinalError::with_title(format!("failed to extract {}", PathFmt(&target)))
                    .detail(err.to_string()),
            })?;
            info!("extracted ({}) {}", BytesFmt(size), PathFmt(&target));
            files_unpacked += 1;
        }
    }
    Ok(files_unpacked)
}

/// Extract the whole archive into a staging folder in one pass.
fn extract_all(archive_path: &Path, output_folder: &Path, password: Option<&[u8]>) -> Result<()> {
    let archive = match password {
        Some(password) => Archive::with_password(archive_path, password),
        None => Archive::new(archive_path),
    };

    let archive = archive.open_for_processing()?;

    let mut first_err: Option<(PathBuf, i32)> = None;
    let mut unsafe_path: Option<(PathBuf, String)> = None;

    let cb_result = archive.extract_all_with_callback(output_folder, |event| match event {
        ExtractEvent::Start { filename, .. } => {
            if let Err(e) = validate_entry_path(&filename) {
                warning!("refusing unsafe rar entry {}: {}", PathFmt(&filename), e);
                unsafe_path = Some((filename, e.to_string()));
                false
            } else {
                true
            }
        }
        ExtractEvent::Ok { .. } => true,
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

    if let Some((path, reason)) = unsafe_path {
        return Err(Error::Custom {
            reason: FinalError::with_title(format!("refusing to extract unsafe rar entry {}", PathFmt(&path)))
                .detail(reason),
        });
    }

    if let Some((path, code)) = first_err {
        let inner = UnrarError::from(Code::from(code), When::Process).to_string();
        return Err(Error::Custom {
            reason: FinalError::with_title(format!("failed to extract {}", PathFmt(&path))).detail(inner),
        });
    }
    let _status = cb_result?;
    Ok(())
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
        let size = Some(item.unpacked_size);
        let path = item.filename;

        Ok(FileInArchive {
            path,
            file_type: if is_dir {
                ListFileType::Directory
            } else {
                ListFileType::File
            },
            size,
        })
    }))
}

pub fn no_compression() -> Error {
    Error::UnsupportedFormat {
        reason: "Creating RAR archives is not allowed due to licensing restrictions.".into(),
    }
}
