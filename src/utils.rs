//! Random stuff used on ouch.

use std::{
    cmp, env,
    ffi::OsStr,
    io,
    path::Component,
    path::{Path, PathBuf},
};

use fs_err as fs;

use crate::{dialogs::Confirmation, info, Error};

/// Create the file if it doesn't exist and if it does then ask to overwrite it.
/// If the user doesn't want to overwrite then we return [`Ok(None)`]
pub fn create_or_ask_overwrite(path: &Path, question_policy: QuestionPolicy) -> Result<Option<fs::File>, Error> {
    match fs::OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(w) => Ok(Some(w)),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            if user_wants_to_overwrite(path, question_policy)? {
                if path.is_dir() {
                    // We can't just use `fs::File::create(&path)` because it would return io::ErrorKind::IsADirectory
                    // ToDo: Maybe we should emphasise that `path` is a directory and everything inside it will be gone?
                    fs::remove_dir_all(path)?;
                }
                Ok(Some(fs::File::create(path)?))
            } else {
                Ok(None)
            }
        }
        Err(e) => Err(Error::from(e)),
    }
}

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
pub fn strip_cur_dir(source_path: &Path) -> PathBuf {
    source_path
        .strip_prefix(Component::CurDir)
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|_| source_path.to_path_buf())
}

/// Returns current directory, but before change the process' directory to the
/// one that contains the file pointed to by `filename`.
pub fn cd_into_same_dir_as(filename: &Path) -> crate::Result<PathBuf> {
    let previous_location = env::current_dir()?;

    let parent = filename.parent().ok_or(crate::Error::CompressingRootFolder)?;
    env::set_current_dir(parent)?;

    Ok(previous_location)
}

/// Check if QuestionPolicy flags were set, otherwise, ask user if they want to overwrite.
pub fn user_wants_to_overwrite(path: &Path, question_policy: QuestionPolicy) -> crate::Result<bool> {
    match question_policy {
        QuestionPolicy::AlwaysYes => Ok(true),
        QuestionPolicy::AlwaysNo => Ok(false),
        QuestionPolicy::Ask => {
            let path = to_utf(strip_cur_dir(path));
            let path = Some(path.as_str());
            let placeholder = Some("FILE");
            Confirmation::new("Do you want to overwrite 'FILE'?", placeholder).ask(path)
        }
    }
}

/// Converts an OsStr to utf8 with custom formatting.
///
/// This is different from [`Path::display`].
///
/// See https://gist.github.com/marcospb19/ebce5572be26397cf08bbd0fd3b65ac1 for a comparison.
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
pub fn nice_directory_display(os_str: impl AsRef<OsStr>) -> String {
    let text = to_utf(os_str);
    if text == "." {
        "current directory".to_string()
    } else {
        format!("'{}'", text)
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
    color!(STYLE_BOLD = "\u{1b}[1m");
    color!(STYLE_RESET = "\u{1b}[0m");
    color!(ALL_RESET = "\u{1b}[0;39m");
}

/// Struct useful to printing bytes as kB, MB, GB, etc.
pub struct Bytes {
    bytes: f64,
}

impl Bytes {
    const UNIT_PREFIXES: [&'static str; 6] = ["", "k", "M", "G", "T", "P"];

    /// Create a new Bytes.
    pub fn new(bytes: u64) -> Self {
        Self { bytes: bytes as f64 }
    }
}

impl std::fmt::Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num = self.bytes;
        debug_assert!(num >= 0.0);
        if num < 1_f64 {
            return write!(f, "{} B", num);
        }
        let delimiter = 1000_f64;
        let exponent = cmp::min((num.ln() / 6.90775).floor() as i32, 4);

        write!(f, "{:.2} ", num / delimiter.powi(exponent))?;
        write!(f, "{}B", Bytes::UNIT_PREFIXES[exponent as usize])
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
/// Determines if overwrite questions should be skipped or asked to the user
pub enum QuestionPolicy {
    /// Ask the user every time
    Ask,
    /// Set by `--yes`, will say 'Y' to all overwrite questions
    AlwaysYes,
    /// Set by `--no`, will say 'N' to all overwrite questions
    AlwaysNo,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pretty_bytes_formatting() {
        fn format_bytes(bytes: u64) -> String {
            format!("{}", Bytes::new(bytes))
        }
        let b = 1;
        let kb = b * 1000;
        let mb = kb * 1000;
        let gb = mb * 1000;

        assert_eq!("0 B", format_bytes(0)); // This is weird
        assert_eq!("1.00 B", format_bytes(b));
        assert_eq!("999.00 B", format_bytes(b * 999));
        assert_eq!("12.00 MB", format_bytes(mb * 12));
        assert_eq!("123.00 MB", format_bytes(mb * 123));
        assert_eq!("5.50 MB", format_bytes(mb * 5 + kb * 500));
        assert_eq!("7.54 GB", format_bytes(gb * 7 + 540 * mb));
        assert_eq!("1.20 TB", format_bytes(gb * 1200));

        // bytes
        assert_eq!("234.00 B", format_bytes(234));
        assert_eq!("999.00 B", format_bytes(999));
        // kilobytes
        assert_eq!("2.23 kB", format_bytes(2234));
        assert_eq!("62.50 kB", format_bytes(62500));
        assert_eq!("329.99 kB", format_bytes(329990));
        // megabytes
        assert_eq!("2.75 MB", format_bytes(2750000));
        assert_eq!("55.00 MB", format_bytes(55000000));
        assert_eq!("987.65 MB", format_bytes(987654321));
        // gigabytes
        assert_eq!("5.28 GB", format_bytes(5280000000));
        assert_eq!("95.20 GB", format_bytes(95200000000));
        assert_eq!("302.00 GB", format_bytes(302000000000));
    }
}
