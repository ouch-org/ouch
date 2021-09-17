use std::{
    cmp, env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use crate::{dialogs::Confirmation, info, oof};

pub fn create_dir_if_non_existent(path: &Path) -> crate::Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)?;
        info!("directory {} created.", to_utf(path));
    }
    Ok(())
}

pub fn cd_into_same_dir_as(filename: &Path) -> crate::Result<PathBuf> {
    let previous_location = env::current_dir()?;

    let parent = filename.parent().ok_or(crate::Error::CompressingRootFolder)?;

    // TODO: fix this error variant, as it is not the only possible error that can
    // come out of this operation
    env::set_current_dir(parent).ok().ok_or(crate::Error::CompressingRootFolder)?;

    Ok(previous_location)
}

pub fn user_wants_to_overwrite(path: &Path, flags: &oof::Flags) -> crate::Result<bool> {
    match (flags.is_present("yes"), flags.is_present("no")) {
        (true, true) => {
            unreachable!("This should've been cutted out in the ~/src/cli.rs filter flags function.")
        }
        (true, _) => return Ok(true),
        (_, true) => return Ok(false),
        _ => {}
    }

    let file_path_str = to_utf(path);

    const OVERWRITE_CONFIRMATION_QUESTION: Confirmation =
        Confirmation::new("Do you want to overwrite 'FILE'?", Some("FILE"));

    OVERWRITE_CONFIRMATION_QUESTION.ask(Some(&file_path_str))
}

pub fn to_utf(os_str: impl AsRef<OsStr>) -> String {
    let text = format!("{:?}", os_str.as_ref());
    text.trim_matches('"').to_string()
}

pub struct Bytes {
    bytes: f64,
}

/// Module with a list of bright colors.
#[allow(dead_code)]
#[cfg(target_family = "unix")]
pub mod colors {
    pub const fn reset() -> &'static str {
        "\u{1b}[39m"
    }
    pub const fn black() -> &'static str {
        "\u{1b}[38;5;8m"
    }
    pub const fn blue() -> &'static str {
        "\u{1b}[38;5;12m"
    }
    pub const fn cyan() -> &'static str {
        "\u{1b}[38;5;14m"
    }
    pub const fn green() -> &'static str {
        "\u{1b}[38;5;10m"
    }
    pub const fn magenta() -> &'static str {
        "\u{1b}[38;5;13m"
    }
    pub const fn red() -> &'static str {
        "\u{1b}[38;5;9m"
    }
    pub const fn white() -> &'static str {
        "\u{1b}[38;5;15m"
    }
    pub const fn yellow() -> &'static str {
        "\u{1b}[38;5;11m"
    }
}
// Windows does not support ANSI escape codes
#[allow(dead_code, non_upper_case_globals)]
#[cfg(not(target_family = "unix"))]
pub mod colors {
    pub fn empty() -> &'static str {
        ""
    }
    pub const reset: fn() -> &'static str = empty;
    pub const black: fn() -> &'static str = empty;
    pub const blue: fn() -> &'static str = empty;
    pub const cyan: fn() -> &'static str = empty;
    pub const green: fn() -> &'static str = empty;
    pub const magenta: fn() -> &'static str = empty;
    pub const red: fn() -> &'static str = empty;
    pub const white: fn() -> &'static str = empty;
    pub const yellow: fn() -> &'static str = empty;
}

impl Bytes {
    const UNIT_PREFIXES: [&'static str; 6] = ["", "k", "M", "G", "T", "P"];

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
