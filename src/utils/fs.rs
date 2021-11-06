//! Random filesystem-related stuff used on ouch.

use std::{
    borrow::Cow,
    env,
    ffi::OsStr,
    path::{Component, Path, PathBuf},
};

use fs_err as fs;

use crate::{extension::Extension, info};

/// Checks given path points to an empty directory.
pub fn dir_is_empty(dir_path: &Path) -> bool {
    let is_empty = |mut rd: std::fs::ReadDir| rd.next().is_none();

    dir_path.read_dir().map(is_empty).unwrap_or_default()
}

/// Creates the dir if non existent.
pub fn create_dir_if_non_existent(path: &Path) -> crate::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        info!("directory {} created.", to_utf(path));
    }
    Ok(())
}

/// Removes the current dir from the beginning of a path
/// normally used for presentation sake.
/// If this function fails, it will return source path as a PathBuf.
pub fn strip_cur_dir(source_path: &Path) -> &Path {
    source_path.strip_prefix(Component::CurDir).unwrap_or(source_path)
}

/// Returns current directory, but before change the process' directory to the
/// one that contains the file pointed to by `filename`.
pub fn cd_into_same_dir_as(filename: &Path) -> crate::Result<PathBuf> {
    let previous_location = env::current_dir()?;

    let parent = filename.parent().ok_or(crate::Error::CompressingRootFolder)?;
    env::set_current_dir(parent)?;

    Ok(previous_location)
}

/// Converts an OsStr to utf8 with custom formatting.
///
/// This is different from [`Path::display`].
///
/// See      for a comparison.
pub fn to_utf(os_str: impl AsRef<OsStr>) -> String {
    let text = format!("{:?}", os_str.as_ref());
    text.trim_matches('"').to_string()
}

/// Converts a slice of AsRef<OsStr> to comma separated String
///
/// Panics if the slice is empty.
pub fn concatenate_list_of_os_str(os_strs: &[impl AsRef<OsStr>]) -> String {
    let mut iter = os_strs.iter().map(AsRef::as_ref);

    let mut string = to_utf(iter.next().unwrap()); // May panic

    for os_str in iter {
        string += ", ";
        string += &to_utf(os_str);
    }
    string
}

/// Display the directory name, but change to "current directory" when necessary.
pub fn nice_directory_display(os_str: impl AsRef<OsStr>) -> Cow<'static, str> {
    if os_str.as_ref() == "." {
        Cow::Borrowed("current directory")
    } else {
        let text = to_utf(os_str);
        Cow::Owned(format!("'{}'", text))
    }
}

/// Try to detect the file extension by looking for known magic strings
/// Source: https://en.wikipedia.org/wiki/List_of_file_signatures
pub fn try_infer(path: &Path) -> Option<Extension> {
    fn is_zip(buf: &[u8]) -> bool {
        buf.len() > 3
            && buf[0] == 0x50
            && buf[1] == 0x4B
            && (buf[2] == 0x3 || buf[2] == 0x5 || buf[2] == 0x7)
            && (buf[3] == 0x4 || buf[3] == 0x6 || buf[3] == 0x8)
    }
    fn is_tar(buf: &[u8]) -> bool {
        buf.len() > 261
            && buf[257] == 0x75
            && buf[258] == 0x73
            && buf[259] == 0x74
            && buf[260] == 0x61
            && buf[261] == 0x72
    }
    fn is_gz(buf: &[u8]) -> bool {
        buf.len() > 2 && buf[0] == 0x1F && buf[1] == 0x8B && buf[2] == 0x8
    }
    fn is_bz2(buf: &[u8]) -> bool {
        buf.len() > 2 && buf[0] == 0x42 && buf[1] == 0x5A && buf[2] == 0x68
    }
    fn is_xz(buf: &[u8]) -> bool {
        buf.len() > 5
            && buf[0] == 0xFD
            && buf[1] == 0x37
            && buf[2] == 0x7A
            && buf[3] == 0x58
            && buf[4] == 0x5A
            && buf[5] == 0x00
    }
    fn is_lz(buf: &[u8]) -> bool {
        buf.len() > 3 && buf[0] == 0x4C && buf[1] == 0x5A && buf[2] == 0x49 && buf[3] == 0x50
    }
    fn is_lz4(buf: &[u8]) -> bool {
        buf.len() > 3 && buf[0] == 0x04 && buf[1] == 0x22 && buf[2] == 0x4D && buf[3] == 0x18
    }
    fn is_zst(buf: &[u8]) -> bool {
        buf.len() > 3 && buf[0] == 0x28 && buf[1] == 0xB5 && buf[2] == 0x2F && buf[3] == 0xFD
    }

    let buf = {
        let mut b = [0; 270];
        // Reading errors will just make the inferring fail so its safe to ignore
        let _ = std::fs::File::open(&path).map(|mut f| std::io::Read::read(&mut f, &mut b));
        b
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
    use once_cell::sync::Lazy;

    static DISABLE_COLORED_TEXT: Lazy<bool> = Lazy::new(|| {
        std::env::var_os("NO_COLOR").is_some() || atty::isnt(atty::Stream::Stdout) || atty::isnt(atty::Stream::Stderr)
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
