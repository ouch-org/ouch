//! Filesystem utility functions.

use std::{
    env,
    fs::ReadDir,
    io::Read,
    path::{Path, PathBuf},
};

use fs_err as fs;

use super::to_utf;
use crate::{extension::Extension, info};

/// Checks if given path points to an empty directory.
pub fn dir_is_empty(dir_path: &Path) -> bool {
    let is_empty = |mut rd: ReadDir| rd.next().is_none();

    dir_path.read_dir().map(is_empty).unwrap_or_default()
}

/// Creates a directory at the path, if there is nothing there.
pub fn create_dir_if_non_existent(path: &Path) -> crate::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        info!("directory {} created.", to_utf(path));
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
/// Source: https://en.wikipedia.org/wiki/List_of_file_signatures
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
        buf.len() > 2 && buf[..=2] == [0x1F, 0x8B, 0x8]
    }
    fn is_bz2(buf: &[u8]) -> bool {
        buf.len() > 2 && buf[..=2] == [0x42, 0x5A, 0x68]
    }
    fn is_xz(buf: &[u8]) -> bool {
        buf.len() > 5 && buf[..=5] == [0xFD, 0x37, 0x7A, 0x58, 0x5A, 0x00]
    }
    fn is_lz(buf: &[u8]) -> bool {
        buf.len() > 3 && buf[..=3] == [0x4C, 0x5A, 0x49, 0x50]
    }
    fn is_lz4(buf: &[u8]) -> bool {
        buf.len() > 3 && buf[..=3] == [0x04, 0x22, 0x4D, 0x18]
    }
    fn is_zst(buf: &[u8]) -> bool {
        buf.len() > 3 && buf[..=3] == [0x28, 0xB5, 0x2F, 0xFD]
    }

    let buf = {
        let mut buf = [0; 270];

        // Error cause will be ignored, so use std::fs instead of fs_err
        let result = std::fs::File::open(&path).map(|mut file| file.read(&mut buf));

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
    } else if is_lz(&buf) {
        Some(Extension::new(&[Lzma], "lz"))
    } else if is_lz4(&buf) {
        Some(Extension::new(&[Lz4], "lz4"))
    } else if is_zst(&buf) {
        Some(Extension::new(&[Zstd], "zst"))
    } else {
        None
    }
}

/// Module with a list of bright colors.
#[allow(dead_code)]
pub mod colors {
    use std::env;

    use once_cell::sync::Lazy;

    static DISABLE_COLORED_TEXT: Lazy<bool> = Lazy::new(|| {
        env::var_os("NO_COLOR").is_some() || atty::isnt(atty::Stream::Stdout) || atty::isnt(atty::Stream::Stderr)
    });

    macro_rules! color {
        ($name:ident = $value:literal) => {
            #[cfg(target_family = "unix")]
            /// Inserts color onto text based on configuration
            pub static $name: Lazy<&str> = Lazy::new(|| if *DISABLE_COLORED_TEXT { "" } else { $value });
            #[cfg(not(target_family = "unix"))]
            pub static $name: &&str = &"";
        };
    }

    color!(RESET = "\u{1b}[39m");
    color!(BLACK = "\u{1b}[38;5;8m");
    color!(BLUE = "\u{1b}[38;5;12m");
    color!(CYAN = "\u{1b}[38;5;14m");
    color!(GREEN = "\u{1b}[38;5;10m");
    color!(MAGENTA = "\u{1b}[38;5;13m");
    color!(RED = "\u{1b}[38;5;9m");
    color!(WHITE = "\u{1b}[38;5;15m");
    color!(YELLOW = "\u{1b}[38;5;11m");
    // Requires true color support
    color!(ORANGE = "\u{1b}[38;2;255;165;0m");
    color!(STYLE_BOLD = "\u{1b}[1m");
    color!(STYLE_RESET = "\u{1b}[0m");
    color!(ALL_RESET = "\u{1b}[0;39m");
}
