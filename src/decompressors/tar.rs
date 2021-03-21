use std::{
    fs,
    path::{Path, PathBuf},
};

use colored::Colorize;
use tar;

use crate::error::OuchResult;
use crate::file::File;

use super::decompressor::Decompressor;

pub struct TarDecompressor {}

impl TarDecompressor {

    fn create_path_if_non_existent(path: &Path) -> OuchResult<()> {
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

    fn unpack_files(from: &Path, into: &Path) -> OuchResult<Vec<PathBuf>> {

        let mut files_unpacked = vec![];

        let file = fs::File::open(from)?;
        let mut archive = tar::Archive::new(file);

        for file in archive.entries()? {
            let mut file = file?;

            // TODO: check if file/folder already exists and ask user's permission for overwriting
            file.unpack_in(into)?;
            
            let file_path = fs::canonicalize(into.join(file.path()?))?;
            files_unpacked.push(file_path);
        }

        Ok(files_unpacked)
    }
}

impl Decompressor for TarDecompressor {
    fn decompress(&self, from: &File, into: &Option<File>) -> OuchResult<Vec<PathBuf>> {
        let destination_path = match into {
            Some(output) => {
                // Must be None according to the way command-line arg. parsing in Ouch works
                assert_eq!(output.extension, None);

                Path::new(&output.path)
            }
            None => Path::new("."),
        };

        Self::create_path_if_non_existent(destination_path)?;

        let files_unpacked = Self::unpack_files(&from.path, destination_path)?;

        Ok(files_unpacked)
    }
}