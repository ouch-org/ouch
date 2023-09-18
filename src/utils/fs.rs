//! Filesystem utility functions.

use std::{
    env,
    io::Read,
    path::{Path, PathBuf},
};

use fs_err as fs;

use super::user_wants_to_overwrite;
use crate::{extension::Extension, info, QuestionPolicy};

/// Remove `path` asking the user to overwrite if necessary.
///
/// * `Ok(true)` means the path is clear,
/// * `Ok(false)` means the user doesn't want to overwrite
/// * `Err(_)` is an error
pub fn clear_path(path: &Path, question_policy: QuestionPolicy) -> crate::Result<bool> {
    if path.exists() && !user_wants_to_overwrite(path, question_policy)? {
        return Ok(false);
    }

    remove_file_or_dir(path)?;

    Ok(true)
}

pub fn remove_file_or_dir(path: &Path) -> crate::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.is_file() {
        fs::remove_file(path)?;
    }
    Ok(())
}

/// Creates a directory at the path, if there is nothing there.
pub fn create_dir_if_non_existent(path: &Path) -> crate::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        // creating a directory is an important change to the file system we
        // should always inform the user about
        info!(accessible, "directory {} created.", path.display());
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

/// Returns true if a path is a symlink.
/// This is the same as the nightly <https://doc.rust-lang.org/std/path/struct.Path.html#method.is_symlink>
/// Useful to detect broken symlinks when compressing. (So we can safely ignore them)
pub fn is_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}
