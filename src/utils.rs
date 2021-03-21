use std::{fs, path::{Component, Path, PathBuf}};

use colored::Colorize;
use crate::{error::OuchResult, file::File};

pub (crate) fn create_path_if_non_existent(path: &Path) -> OuchResult<()> {
    if !path.exists() {
        println!(
            "{}: attempting to create folder {:?}.",
            "info".yellow(),
            &path
        );
        std::fs::create_dir_all(path)?;
        println!(
            "{}: directory {:#?} created.",
            "info".yellow(),
            fs::canonicalize(&path)?
        );
    }
    Ok(())
}

pub (crate) fn get_destination_path(dest: &Option<File>) -> &Path {
    match dest {
        Some(output) => {
            // Must be None according to the way command-line arg. parsing in Ouch works
            assert_eq!(output.extension, None);

            Path::new(&output.path)
        }
        None => Path::new("."),
    }
}