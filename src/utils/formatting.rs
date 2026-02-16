use std::{
    borrow::Cow,
    cmp,
    ffi::OsStr,
    fmt::{self, Display},
    path::{Path, PathBuf},
};

use crate::INITIAL_CURRENT_DIR;

/// Converts invalid UTF-8 bytes to the Unicode replacement codepoint (�) in its Display implementation.
pub struct EscapedPathDisplay<'a> {
    path: &'a Path,
}

impl<'a> EscapedPathDisplay<'a> {
    pub fn new(path: &'a Path) -> Self {
        Self { path }
    }
}

#[cfg(unix)]
impl Display for EscapedPathDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::os::unix::prelude::OsStrExt;

        let bstr = bstr::BStr::new(self.path.as_os_str().as_bytes());

        write!(f, "{bstr}")
    }
}

#[cfg(windows)]
impl Display for EscapedPathDisplay<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::{char, fmt::Write, os::windows::prelude::OsStrExt};

        let utf16 = self.path.as_os_str().encode_wide();
        let chars = char::decode_utf16(utf16).map(|decoded| decoded.unwrap_or(char::REPLACEMENT_CHARACTER));

        for char in chars {
            f.write_char(char)?;
        }

        Ok(())
    }
}

/// Converts an OsStr to utf8 with custom formatting.
///
/// This is different from [`Path::display`].
///
/// See <https://gist.github.com/marcospb19/ebce5572be26397cf08bbd0fd3b65ac1> for a comparison.
pub fn path_to_str(path: &Path) -> Cow<'_, str> {
    os_str_to_str(path.as_ref())
}

pub fn os_str_to_str(os_str: &OsStr) -> Cow<'_, str> {
    let format = || {
        let text = format!("{os_str:?}");
        Cow::Owned(text.trim_matches('"').to_string())
    };

    os_str.to_str().map_or_else(format, Cow::Borrowed)
}

/// Removes the current dir from the beginning of a path as it's redundant information,
/// useful for presentation sake.
pub fn strip_cur_dir(source_path: &Path) -> &Path {
    source_path.strip_prefix(&*INITIAL_CURRENT_DIR).unwrap_or(source_path)
}

/// Converts a slice of `AsRef<OsStr>` to comma separated String
///
/// Panics if the slice is empty.
pub fn pretty_format_list_of_paths(paths: &[impl AsRef<Path>]) -> String {
    let mut iter = paths.iter().map(AsRef::as_ref);

    let first_path = iter.next().unwrap();
    let mut string = path_to_str(first_path).into_owned();

    for path in iter {
        string += ", ";
        string += &path_to_str(path);
    }
    string
}

/// Display the directory name, but use "current directory" when necessary.
pub fn nice_directory_display(path: &Path) -> Cow<'_, str> {
    if path == Path::new(".") {
        Cow::Borrowed("current directory")
    } else {
        path_to_str(path)
    }
}

/// Strips an ascii prefix from the path (similar to `<&str>::strip_prefix`).
///
/// # Panics:
///
/// - Panics if prefix is not valid ASCII (to ensure safety).
pub fn strip_path_ascii_prefix<'a>(path: Cow<'a, Path>, ascii_prefix: &str) -> Cow<'a, Path> {
    assert!(ascii_prefix.is_ascii());
    let prefix_slice = ascii_prefix.as_bytes();
    let path_slice = path.as_os_str().as_encoded_bytes();

    if let Some(stripped) = path_slice.strip_prefix(prefix_slice) {
        // Encoding Safety:
        //   this function returns a format that is guaranteed to be a superset
        //   of UTF-8, it might be WTF-8 encoding surrogates in UTF-8-like ways,
        //   it's impossible for us to break surrogate pairs or character
        //   boundaries if we slice an ASCII prefix, ASCII characters in WTF-8
        //   and UTF-8 look exactly just like in plain ASCII encoding
        let str = unsafe { OsStr::from_encoded_bytes_unchecked(stripped) };
        Cow::from(PathBuf::from(str))
    } else {
        path
    }
}

pub struct PathFmt<'a>(pub &'a Path);

impl<'a> fmt::Display for PathFmt<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let path = self.0;
        let path = strip_path_ascii_prefix(Cow::Borrowed(path), "./");
        let path = path.as_ref();

        let path = path.strip_prefix(&*INITIAL_CURRENT_DIR).unwrap_or(path);
        let path = if path.as_os_str().is_empty() {
            Path::new(".")
        } else {
            path
        };

        write!(f, "\"{}\"", path.display())
    }
}

/// Pretty `fmt::Display` impl for printing bytes as kB, MB, GB, etc.
pub struct BytesFmt(pub u64);

impl BytesFmt {
    const UNIT_PREFIXES: [&'static str; 6] = ["", "ki", "Mi", "Gi", "Ti", "Pi"];
}

impl fmt::Display for BytesFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let num = self.0 as f64;

        debug_assert!(num >= 0.0);
        if num < 1_f64 {
            return write!(f, "{num:>6.2}   B");
        }

        let delimiter = 1000_f64;
        let exponent = cmp::min((num.ln() / 6.90775).floor() as i32, 4);

        write!(
            f,
            "{:>6.2} {:>2}B",
            num / delimiter.powi(exponent),
            Self::UNIT_PREFIXES[exponent as usize],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pretty_bytes_formatting() {
        fn format_bytes(bytes: u64) -> String {
            format!("{}", BytesFmt(bytes))
        }
        let b = 1;
        let kb = b * 1000;
        let mb = kb * 1000;
        let gb = mb * 1000;

        assert_eq!("  0.00   B", format_bytes(0)); // This is weird
        assert_eq!("  1.00   B", format_bytes(b));
        assert_eq!("999.00   B", format_bytes(b * 999));
        assert_eq!(" 12.00 MiB", format_bytes(mb * 12));
        assert_eq!("123.00 MiB", format_bytes(mb * 123));
        assert_eq!("  5.50 MiB", format_bytes(mb * 5 + kb * 500));
        assert_eq!("  7.54 GiB", format_bytes(gb * 7 + 540 * mb));
        assert_eq!("  1.20 TiB", format_bytes(gb * 1200));

        // bytes
        assert_eq!("234.00   B", format_bytes(234));
        assert_eq!("999.00   B", format_bytes(999));
        // kilobytes
        assert_eq!("  2.23 kiB", format_bytes(2234));
        assert_eq!(" 62.50 kiB", format_bytes(62500));
        assert_eq!("329.99 kiB", format_bytes(329990));
        // megabytes
        assert_eq!("  2.75 MiB", format_bytes(2750000));
        assert_eq!(" 55.00 MiB", format_bytes(55000000));
        assert_eq!("987.65 MiB", format_bytes(987654321));
        // gigabytes
        assert_eq!("  5.28 GiB", format_bytes(5280000000));
        assert_eq!(" 95.20 GiB", format_bytes(95200000000));
        assert_eq!("302.00 GiB", format_bytes(302000000000));
        assert_eq!("302.99 GiB", format_bytes(302990000000));
        // Weird aproximation cases:
        assert_eq!("999.90 GiB", format_bytes(999900000000));
        assert_eq!("  1.00 TiB", format_bytes(999990000000));
    }
}
