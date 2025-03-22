//! Filesystem utility functions.

use std::{
    env,
    io::Read,
    path::{Path, PathBuf},
};

use fs_err as fs;

use super::{question::FileConflitOperation, user_wants_to_overwrite};
use crate::{
    extension::Extension,
    utils::{logger::info_accessible, EscapedPathDisplay},
    QuestionPolicy,
};

pub fn is_path_stdin(path: &Path) -> bool {
    path.as_os_str() == "-"
}


pub fn resolve_path(path: &Path, question_policy: QuestionPolicy) -> crate::Result<Option<PathBuf>> {
    if path.exists() {
        match user_wants_to_overwrite(path, question_policy)? {
            FileConflitOperation::Cancel => return Ok(None),
            FileConflitOperation::Overwrite => {
                remove_file_or_dir(path)?;
                Ok(Some(path.to_path_buf()))
            },
            FileConflitOperation::Rename => {
                let renamed_path = rename_for_available_filename(path);
                Ok(Some(renamed_path))
            },
        }
    } else {
        Ok(Some(path.to_path_buf()))
    }
}

pub fn remove_file_or_dir(path: &Path) -> crate::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

pub fn rename_for_available_filename(path: &Path) -> PathBuf {
    let mut renamed_path = rename_or_increment_filename(path);
    while renamed_path.exists() {
        renamed_path = rename_or_increment_filename(&renamed_path);
    }
    renamed_path
}

pub fn rename_or_increment_filename(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    let new_filename = match filename.rsplit_once('_') {
        Some((base, number_str)) if number_str.chars().all(char::is_numeric) => {
            let number = number_str.parse::<u32>().unwrap_or(0);
            format!("{}_{}", base, number + 1)
        },
        _ => format!("{}_1", filename),
    };
    
    let mut new_path = parent.join(new_filename);
    if !extension.is_empty() {
        new_path.set_extension(extension);
    }

    new_path
}

/// Creates a directory at the path, if there is nothing there.
pub fn create_dir_if_non_existent(path: &Path) -> crate::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        // creating a directory is an important change to the file system we
        // should always inform the user about
        info_accessible(format!("Directory {} created", EscapedPathDisplay::new(path)));
    }
    Ok(())
}

/// Returns current directory, but before change the process' directory to the
/// one that contains the file pointed to by `filename`.
pub fn cd_into_same_dir_as(filename: &Path) -> crate::Result<PathBuf> {
    let previous_location = env::current_dir()?;

    let parent = filename.parent().ok_or(crate::Error::CompressingRootFolder)?;
    env::set_current_dir(parent)?;

    Ok(previous_location)
}

/// Try to detect the file extension by looking for known magic strings
/// Source: <https://en.wikipedia.org/wiki/List_of_file_signatures>
pub fn try_infer_extension(path: &Path) -> Option<Extension> {
    fn is_zip(buf: &[u8]) -> bool {
        buf.len() >= 3
            && buf[..=1] == [0x50, 0x4B]
            && (buf[2..=3] == [0x3, 0x4] || buf[2..=3] == [0x5, 0x6] || buf[2..=3] == [0x7, 0x8])
    }
    fn is_tar(buf: &[u8]) -> bool {
        buf.len() > 261 && buf[257..=261] == [0x75, 0x73, 0x74, 0x61, 0x72]
    }
    fn is_gz(buf: &[u8]) -> bool {
        buf.starts_with(&[0x1F, 0x8B, 0x8])
    }
    fn is_bz2(buf: &[u8]) -> bool {
        buf.starts_with(&[0x42, 0x5A, 0x68])
    }
    fn is_bz3(buf: &[u8]) -> bool {
        buf.starts_with(bzip3::MAGIC_NUMBER)
    }
    fn is_xz(buf: &[u8]) -> bool {
        buf.starts_with(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00])
    }
    fn is_lz4(buf: &[u8]) -> bool {
        buf.starts_with(&[0x04, 0x22, 0x4D, 0x18])
    }
    fn is_sz(buf: &[u8]) -> bool {
        buf.starts_with(&[0xFF, 0x06, 0x00, 0x00, 0x73, 0x4E, 0x61, 0x50, 0x70, 0x59])
    }
    fn is_zst(buf: &[u8]) -> bool {
        buf.starts_with(&[0x28, 0xB5, 0x2F, 0xFD])
    }
    fn is_rar(buf: &[u8]) -> bool {
        // ref https://www.rarlab.com/technote.htm#rarsign
        // RAR 5.0 8 bytes length signature: 0x52 0x61 0x72 0x21 0x1A 0x07 0x01 0x00
        // RAR 4.x 7 bytes length signature: 0x52 0x61 0x72 0x21 0x1A 0x07 0x00
        buf.len() >= 7
            && buf.starts_with(&[0x52, 0x61, 0x72, 0x21, 0x1A, 0x07])
            && (buf[6] == 0x00 || (buf.len() >= 8 && buf[6..=7] == [0x01, 0x00]))
    }
    fn is_sevenz(buf: &[u8]) -> bool {
        buf.starts_with(&[0x37, 0x7A, 0xBC, 0xAF, 0x27, 0x1C])
    }

    let buf = {
        let mut buf = [0; 270];

        // Error cause will be ignored, so use std::fs instead of fs_err
        let result = std::fs::File::open(path).map(|mut file| file.read(&mut buf));

        // In case of file open or read failure, could not infer a extension
        if result.is_err() {
            return None;
        }
        buf
    };

    use crate::extension::CompressionFormat::*;
    if is_zip(&buf) {
        Some(Extension::new(&[Zip], "zip"))
    } else if is_tar(&buf) {
        Some(Extension::new(&[Tar], "tar"))
    } else if is_gz(&buf) {
        Some(Extension::new(&[Gzip], "gz"))
    } else if is_bz2(&buf) {
        Some(Extension::new(&[Bzip], "bz2"))
    } else if is_bz3(&buf) {
        Some(Extension::new(&[Bzip3], "bz3"))
    } else if is_xz(&buf) {
        Some(Extension::new(&[Lzma], "xz"))
    } else if is_lz4(&buf) {
        Some(Extension::new(&[Lz4], "lz4"))
    } else if is_sz(&buf) {
        Some(Extension::new(&[Snappy], "sz"))
    } else if is_zst(&buf) {
        Some(Extension::new(&[Zstd], "zst"))
    } else if is_rar(&buf) {
        Some(Extension::new(&[Rar], "rar"))
    } else if is_sevenz(&buf) {
        Some(Extension::new(&[SevenZip], "7z"))
    } else {
        None
    }
}
