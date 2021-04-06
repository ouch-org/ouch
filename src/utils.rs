use std::{
    env,
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

pub(crate) fn ensure_exists<'a, P>(path: P) -> crate::Result<()>
where
    P: AsRef<Path> + 'a,
{
    let exists = path.as_ref().exists();
    if !exists {
        return Err(crate::Error::FileNotFound(PathBuf::from(path.as_ref())));
    }
    Ok(())
}

pub(crate) fn check_for_multiple_files(
    files: &[PathBuf],
    format: &CompressionFormat,
) -> crate::Result<()> {
    if files.len() != 1 {
        eprintln!("{}: cannot compress multiple files directly to {:#?}.\n       Try using an intermediate archival method such as Tar.\n       Example: filename.tar{}", "[ERROR]".red(), format, format);
        return Err(crate::Error::InvalidInput);
    }

    Ok(())
}

pub(crate) fn create_path_if_non_existent(path: &Path) -> crate::Result<()> {
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

pub(crate) fn get_destination_path<'a>(dest: &'a Option<File>) -> &'a Path {
    match dest {
        Some(output_file) => {
            // Must be None according to the way command-line arg. parsing in Ouch works
            assert_eq!(output_file.extension, None);
            Path::new(&output_file.path)
        }
        None => Path::new("."),
    }
}

pub(crate) fn change_dir_and_return_parent(filename: &Path) -> crate::Result<PathBuf> {
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
