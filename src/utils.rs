use std::{env, fs, path::{Path, PathBuf}};

use colored::Colorize;

use crate::{extension::CompressionFormat, file::File};

pub(crate) fn ensure_exists<'a, P>(path: P) -> crate::Result<()>
where
    P: AsRef<Path> + 'a,
{
    let exists = path.as_ref().exists();
    if !exists {
        eprintln!(
            "{}: could not find file {:?}",
            "[ERROR]".red(),
            path.as_ref()
        );
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

pub(crate) fn get_destination_path(dest: &Option<File>) -> &Path {
    match dest {
        Some(output) => {
            // Must be None according to the way command-line arg. parsing in Ouch works
            assert_eq!(output.extension, None);

            Path::new(&output.path)
        }
        None => Path::new("."),
    }
}

pub (crate) fn change_dir_and_return_parent(filename: &PathBuf) -> crate::Result<PathBuf> {
    let previous_location = env::current_dir()?;
    let parent = filename.parent().unwrap();
    env::set_current_dir(parent)?;
    Ok(previous_location)
}