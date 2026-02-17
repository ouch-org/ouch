//! Filesystem utility functions.

use std::{
    borrow::Cow,
    env,
    io::Read,
    path::{Path, PathBuf},
};

use fs_err::{self as fs, PathExt};

use super::{question::FileConflitOperation, user_wants_to_overwrite};
use crate::{
    extension::CompressionFormat,
    info_accessible,
    utils::{strip_path_ascii_prefix, PathFmt, QuestionAction},
    QuestionPolicy, Result,
};

pub fn is_path_stdin(path: &Path) -> bool {
    path.as_os_str() == "-"
}

/// Check if &Path exists, if it does then ask the user if they want to overwrite or rename it.
/// If the user want to overwrite then the file or directory will be removed and returned the same input path
/// If the user want to rename then nothing will be removed and a new path will be returned with a new name
///
/// * `Ok(None)` means the user wants to cancel the operation
/// * `Ok(Some(path))` returns a valid PathBuf without any another file or directory with the same name
/// * `Err(_)` is an error
pub fn resolve_path_conflict(
    path: &Path,
    question_policy: QuestionPolicy,
    question_action: QuestionAction,
) -> crate::Result<Option<PathBuf>> {
    if path.fs_err_try_exists()? {
        match user_wants_to_overwrite(path, question_policy, question_action)? {
            FileConflitOperation::Cancel => Ok(None),
            FileConflitOperation::Overwrite => {
                remove_file_or_dir(path)?;
                Ok(Some(path.to_path_buf()))
            }
            FileConflitOperation::Rename => Ok(Some(find_available_filename_by_renaming(path)?)),
            FileConflitOperation::Merge => Ok(Some(path.to_path_buf())),
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

pub fn file_size(path: &Path) -> crate::Result<u64> {
    Ok(fs::metadata(path)?.len())
}

/// Say you want to write to `archive.tar.gz` but that already exists.
///
/// So the user chooses to `rename` to avoid the conflict (keep both files).
///
/// In this scenario, this function will return `archive_1.tar.gz`, subsequent
/// calls will keep incrementing the number:
///
/// - archive_1.tar.gz
/// - archive_2.tar.gz
/// - archive_3.tar.gz
pub fn find_available_filename_by_renaming(path: &Path) -> crate::Result<PathBuf> {
    fn create_path_with_given_index(path: &Path, i: usize) -> PathBuf {
        let parent = path.parent().unwrap_or_else(|| Path::new(""));
        let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");

        let new_filename = match file_name.split_once('.') {
            Some((stem, extension)) if !stem.is_empty() => format!("{stem}_{i}.{extension}"),
            _ => format!("{file_name}_{i}"),
        };

        parent.join(new_filename)
    }

    for i in 1.. {
        let renamed_path = create_path_with_given_index(path, i);
        if !renamed_path.fs_err_try_exists()? {
            return Ok(renamed_path);
        }
    }
    unreachable!()
}

/// Creates a directory at the path, if there is nothing there.
pub fn create_dir_if_non_existent(path: &Path) -> crate::Result<()> {
    if !path.fs_err_try_exists()? {
        fs::create_dir_all(path)?;
        info_accessible!("Directory {:?} created", PathFmt(path));
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
pub fn try_infer_format(path: &Path) -> Option<CompressionFormat> {
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
        buf.starts_with(b"BZ3v1")
    }
    fn is_lzma(buf: &[u8]) -> bool {
        buf.len() >= 14 && buf[0] == 0x5d && (buf[12] == 0x00 || buf[12] == 0xff) && buf[13] == 0x00
    }
    fn is_xz(buf: &[u8]) -> bool {
        buf.starts_with(&[0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00])
    }
    fn is_lzip(buf: &[u8]) -> bool {
        buf.starts_with(&[0x4C, 0x5A, 0x49, 0x50])
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

    if is_zip(&buf) {
        Some(CompressionFormat::Zip)
    } else if is_tar(&buf) {
        Some(CompressionFormat::Tar)
    } else if is_gz(&buf) {
        Some(CompressionFormat::Gzip)
    } else if is_bz2(&buf) {
        Some(CompressionFormat::Bzip)
    } else if is_bz3(&buf) {
        Some(CompressionFormat::Bzip3)
    } else if is_lzma(&buf) {
        Some(CompressionFormat::Lzma)
    } else if is_xz(&buf) {
        Some(CompressionFormat::Xz)
    } else if is_lzip(&buf) {
        Some(CompressionFormat::Lzip)
    } else if is_lz4(&buf) {
        Some(CompressionFormat::Lz4)
    } else if is_sz(&buf) {
        Some(CompressionFormat::Snappy)
    } else if is_zst(&buf) {
        Some(CompressionFormat::Zstd)
    } else if is_rar(&buf) {
        Some(CompressionFormat::Rar)
    } else if is_sevenz(&buf) {
        Some(CompressionFormat::SevenZip)
    } else {
        None
    }
}

#[inline]
pub fn create_symlink(target: &Path, full_path: &Path) -> crate::Result<()> {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, full_path)?;

    // FIXME: how to detect whether the destination is a folder or a regular file?
    // regular file should use fs::symlink_file
    // folder should use fs::symlink_dir
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, full_path)?;

    Ok(())
}

#[cfg(unix)]
#[inline]
pub fn set_permission_mode(path: &Path, mode: u32) -> crate::Result<()> {
    use std::{fs::Permissions, os::unix::fs::PermissionsExt};
    fs::set_permissions(path, Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(windows)]
#[inline]
pub fn set_permission_mode(_path: &Path, _mode: u32) -> crate::Result<()> {
    Ok(())
}

/// Canonicalize a path.
///
/// On Windows, it strips the `\\?\` extended path prefix that fs::canonicalize
/// adds that would break `strip_prefix` calls involving this path.
pub fn canonicalize(path: impl AsRef<Path>) -> Result<PathBuf> {
    let canonicalized = fs::canonicalize(path.as_ref())?;

    Ok(if cfg!(windows) {
        strip_path_ascii_prefix(Cow::Owned(canonicalized), r"\\?\").into_owned()
    } else {
        canonicalized
    })
}
