use std::{env, fs, path::PathBuf};

use colored::Colorize;
use tar::Builder;
use walkdir::WalkDir;

use super::compressor::Entry;
use crate::{compressors::Compressor, file::File};

pub struct TarCompressor {}

impl TarCompressor {
    // TODO: implement this
    fn make_archive_from_memory(_input: File) -> crate::Result<Vec<u8>> {
        println!(
            "{}: .tar.tar and .zip.tar is currently unimplemented.",
            "error".red()
        );
        Err(crate::Error::InvalidZipArchive(""))
    }

    fn make_archive_from_files(input_filenames: Vec<PathBuf>) -> crate::Result<Vec<u8>> {
        let change_dir_and_return_parent = |filename: &PathBuf| -> crate::Result<PathBuf> {
            let previous_location = env::current_dir()?;
            let parent = filename.parent().unwrap();
            env::set_current_dir(parent)?;
            Ok(previous_location)
        };

        let buf = Vec::new();
        let mut b = Builder::new(buf);

        for filename in input_filenames {
            let previous_location = change_dir_and_return_parent(&filename)?;
            // Safe unwrap since this filename came from `fs::canonicalize`.
            let filename = filename.file_name().unwrap();
            for entry in WalkDir::new(&filename) {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }
                b.append_file(path, &mut fs::File::open(path)?)?;
            }
            env::set_current_dir(previous_location)?;
        }

        Ok(b.into_inner()?)
    }
}

impl Compressor for TarCompressor {
    fn compress(&self, from: Entry) -> crate::Result<Vec<u8>> {
        match from {
            Entry::Files(filenames) => Self::make_archive_from_files(filenames),
            Entry::InMemory(file) => Self::make_archive_from_memory(file),
        }
    }
}
