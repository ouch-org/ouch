use std::{borrow::Cow, cmp, ffi::OsStr, path::Path};

use crate::CURRENT_DIRECTORY;

/// Converts an OsStr to utf8 with custom formatting.
///
/// This is different from [`Path::display`], see
/// <https://gist.github.com/marcospb19/ebce5572be26397cf08bbd0fd3b65ac1> for a comparison.
pub fn path_to_str(path: &Path) -> Cow<str> {
    os_str_to_str(path.as_ref())
}

pub fn os_str_to_str(os_str: &OsStr) -> Cow<str> {
    let format = || {
        let text = format!("{os_str:?}");
        Cow::Owned(text.trim_matches('"').to_string())
    };

    os_str.to_str().map_or_else(format, Cow::Borrowed)
}

/// Removes the current dir from the beginning of a path as it's redundant information,
/// useful for presentation sake.
pub fn strip_cur_dir(source_path: &Path) -> &Path {
    let current_dir = &*CURRENT_DIRECTORY;

    source_path.strip_prefix(current_dir).unwrap_or(source_path)
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
pub fn nice_directory_display(path: &Path) -> Cow<str> {
    if path == Path::new(".") {
        Cow::Borrowed("current directory")
    } else {
        path_to_str(path)
    }
}

/// Struct useful to printing bytes as kB, MB, GB, etc.
pub struct Bytes(f64);

impl Bytes {
    const UNIT_PREFIXES: [&'static str; 6] = ["", "ki", "Mi", "Gi", "Ti", "Pi"];

    /// Create a new Bytes.
    pub fn new(bytes: u64) -> Self {
        Self(bytes as f64)
    }
}

impl std::fmt::Display for Bytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let &Self(num) = self;

        debug_assert!(num >= 0.0);
        if num < 1_f64 {
            return write!(f, "{} B", num);
        }

        let delimiter = 1000_f64;
        let exponent = cmp::min((num.ln() / 6.90775).floor() as i32, 4);

        write!(
            f,
            "{:.2} {}B",
            num / delimiter.powi(exponent),
            Bytes::UNIT_PREFIXES[exponent as usize]
        )
    }
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
        assert_eq!("12.00 MiB", format_bytes(mb * 12));
        assert_eq!("123.00 MiB", format_bytes(mb * 123));
        assert_eq!("5.50 MiB", format_bytes(mb * 5 + kb * 500));
        assert_eq!("7.54 GiB", format_bytes(gb * 7 + 540 * mb));
        assert_eq!("1.20 TiB", format_bytes(gb * 1200));

        // bytes
        assert_eq!("234.00 B", format_bytes(234));
        assert_eq!("999.00 B", format_bytes(999));
        // kilobytes
        assert_eq!("2.23 kiB", format_bytes(2234));
        assert_eq!("62.50 kiB", format_bytes(62500));
        assert_eq!("329.99 kiB", format_bytes(329990));
        // megabytes
        assert_eq!("2.75 MiB", format_bytes(2750000));
        assert_eq!("55.00 MiB", format_bytes(55000000));
        assert_eq!("987.65 MiB", format_bytes(987654321));
        // gigabytes
        assert_eq!("5.28 GiB", format_bytes(5280000000));
        assert_eq!("95.20 GiB", format_bytes(95200000000));
        assert_eq!("302.00 GiB", format_bytes(302000000000));
        assert_eq!("302.99 GiB", format_bytes(302990000000));
        // Weird aproximation cases:
        assert_eq!("999.90 GiB", format_bytes(999900000000));
        assert_eq!("1.00 TiB", format_bytes(999990000000));
    }
}
