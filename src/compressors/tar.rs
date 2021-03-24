use std::{fs, path::PathBuf};

use colored::Colorize;
use tar::{Builder, EntryType, Header};
use walkdir::WalkDir;

use crate::{compressors::Compressor, error::{Error, OuchResult}, file::File};

use super::compressor::Entry;

pub struct TarCompressor {}

impl TarCompressor {

    // TODO: this function does not seem to be working correctly ;/
    fn make_archive_from_memory(input: File) -> OuchResult<Vec<u8>> {
        
        let contents = match input.contents_in_memory {
            Some(bytes) => bytes,
            None => {
                eprintln!("{}: reached TarCompressor::make_archive_from_memory without known content.", "internal error".red());
                return Err(Error::InvalidInput);
            }
        };

        println!("todo");

        Ok(vec![])
    }

    fn make_archive_from_files(input_filenames: Vec<PathBuf>) -> OuchResult<Vec<u8>> {
    
        let buf = Vec::new();
        let mut b = Builder::new(buf);
    
        for filename in input_filenames {
            // TODO: check if file exists

            for entry in WalkDir::new(&filename) {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }
                b.append_file(path, &mut fs::File::open(path)?)?;
            }
        }
        
        Ok(b.into_inner()?)
    }
}

impl Compressor for TarCompressor {
    fn compress(&self, from: Entry) -> OuchResult<Vec<u8>> {

        match from {
            Entry::Files(filenames) => {
                Ok(
                    Self::make_archive_from_files(filenames)?
                )
            },
            Entry::InMemory(file) => {
                Ok(
                    Self::make_archive_from_memory(file)?
                )
            }
        }        
    }
}