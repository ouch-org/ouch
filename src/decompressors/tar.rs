use std::{fs, io::{Cursor, Read}, path::{Path, PathBuf}};

use colored::Colorize;
use tar::{self, Archive};

use crate::{error::OuchResult, utils};
use crate::file::File;

use super::decompressor::{DecompressionResult, Decompressor};

pub struct TarDecompressor {}

impl TarDecompressor {

    fn unpack_files(from: File, into: &Path) -> OuchResult<Vec<PathBuf>> {

        println!("{}: attempting to decompress {:?}", "ouch".bright_blue(), &from.path);
        let mut files_unpacked = vec![];

        let mut archive: Archive<Box<dyn Read>> = match from.contents {
            Some(bytes) => {
                tar::Archive::new(Box::new(Cursor::new(bytes)))
            }
            None => {
                let file = fs::File::open(&from.path)?;
                tar::Archive::new(Box::new(file))
            }
        };

        for file in archive.entries()? {
            let mut file = file?;

            // TODO: check if file/folder already exists and ask user's permission for overwriting
            file.unpack_in(into)?;

            println!(
                "{}: {:?} extracted. ({} bytes)",
                "info".yellow(),
                into.join(file.path()?),
                file.size()
            );

            let file_path = fs::canonicalize(into.join(file.path()?))?;
            files_unpacked.push(file_path);
        }

        Ok(files_unpacked)
    }
}

impl Decompressor for TarDecompressor {
    fn decompress(&self, from: File, into: &Option<File>) -> OuchResult<DecompressionResult> {
        let destination_path = utils::get_destination_path(into);

        utils::create_path_if_non_existent(destination_path)?;

        let files_unpacked = Self::unpack_files(from, destination_path)?;

        Ok(DecompressionResult::FilesUnpacked(files_unpacked))
    }
}