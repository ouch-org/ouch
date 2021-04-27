use std::{
    fs,
    io::{Cursor, Read},
};

use tar::{self, Archive};

use super::lister::{Lister, Listing};
use crate::file::File;
use super::FileMetadata;

#[derive(Debug)]
pub struct TarLister;

impl TarLister {
    fn unpack_files(from: File) -> crate::Result<Listing> {
        
        let mut files = vec![];

        let mut archive: Archive<Box<dyn Read>> = match from.contents_in_memory {
            Some(bytes) => tar::Archive::new(Box::new(Cursor::new(bytes))),
            None => {
                let file = fs::File::open(&from.path)?;
                tar::Archive::new(Box::new(file))
            }
        };

        for file in archive.entries()? {
            let file = file?;

            let file_path = fs::canonicalize(file.path()?)?;

            let metadata = FileMetadata {
                path: file_path,
                bytes: file.size(),
            };

            files.push(metadata)
        }

        Ok(files)
    }
}

impl Lister for TarLister {
    fn list(&self, from: File) -> crate::Result<Listing> {
        let files_unpacked = Self::unpack_files(from)?;

        Ok(files_unpacked)
    }
}
