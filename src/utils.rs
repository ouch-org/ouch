use std::{
    cmp, env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

use colored::Colorize;

use crate::{dialogs::Confirmation, extension::CompressionFormat, file::File};

#[macro_export]
#[cfg(debug_assertions)]
macro_rules! debug {
    ($x:expr) => {
        dbg!($x)
    };
}

#[macro_export]
#[cfg(not(debug_assertions))]
macro_rules! debug {
    ($x:expr) => {
        std::convert::identity($x)
    };
}

pub fn ensure_exists<'a, P>(path: P) -> crate::Result<()>
where
    P: AsRef<Path> + 'a,
{
    let exists = path.as_ref().exists();
    if !exists {
        return Err(crate::Error::FileNotFound(PathBuf::from(path.as_ref())));
    }
    Ok(())
}

pub fn check_for_multiple_files(
    files: &[PathBuf],
    format: &CompressionFormat,
) -> crate::Result<()> {
    if files.len() != 1 {
        eprintln!(
            "{}: cannot compress multiple files directly to {:#?}.\n\
               Try using an intermediate archival method such as Tar.\n\
               Example: filename.tar{}",
            "[ERROR]".red(),
            format,
            format
        );
        return Err(crate::Error::InvalidInput);
    }

    Ok(())
}

pub fn create_path_if_non_existent(path: &Path) -> crate::Result<()> {
    if !path.exists() {
        println!(
            "{}: attempting to create folder {:?}.",
            "[INFO]".yellow(),
            &path
        );
        std::fs::create_dir_all(path)?;
        println!(
            "{}: directory {:#?} created.",
            "[INFO]".yellow(),
            fs::canonicalize(&path)?
        );
    }
    Ok(())
}

pub fn get_destination_path<'a>(dest: &'a Option<File>) -> &'a Path {
    match dest {
        Some(output_file) => {
            // Must be None according to the way command-line arg. parsing in Ouch works
            assert_eq!(output_file.extension, None);
            Path::new(&output_file.path)
        }
        None => Path::new("."),
    }
}

pub fn change_dir_and_return_parent(filename: &Path) -> crate::Result<PathBuf> {
    let previous_location = env::current_dir()?;

    let parent = if let Some(parent) = filename.parent() {
        parent
    } else {
        return Err(crate::Error::CompressingRootFolder);
    };

    env::set_current_dir(parent)
        .ok()
        .ok_or(crate::Error::CompressingRootFolder)?;

    Ok(previous_location)
}

pub fn permission_for_overwriting(
    path: &Path,
    flags: &oof::Flags,
    confirm: &Confirmation,
) -> crate::Result<bool> {
    match (flags.is_present("yes"), flags.is_present("false")) {
        (true, true) => {
            unreachable!("This shoul've been cutted out in the ~/src/cli.rs filter flags function.")
        }
        (true, _) => return Ok(true),
        (_, true) => return Ok(false),
        _ => {}
    }

    let file_path_str = to_utf(path);
    confirm.ask(Some(&file_path_str))
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
    use termion::color::*;

    pub fn reset() -> &'static str {
        Reset.fg_str()
    }
    pub fn black() -> &'static str {
        LightBlack.fg_str()
    }
    pub fn blue() -> &'static str {
        LightBlue.fg_str()
    }
    pub fn cyan() -> &'static str {
        LightCyan.fg_str()
    }
    pub fn green() -> &'static str {
        LightGreen.fg_str()
    }
    pub fn magenta() -> &'static str {
        LightMagenta.fg_str()
    }
    pub fn red() -> &'static str {
        LightRed.fg_str()
    }
    pub fn white() -> &'static str {
        LightWhite.fg_str()
    }
    pub fn yellow() -> &'static str {
        LightYellow.fg_str()
    }
}
// Termion does not support Windows
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
        Self {
            bytes: bytes as f64,
        }
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
