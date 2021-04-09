use std::{
    fs,
    io::{Cursor, Read},
    path::{PathBuf},
};


use tar::{self, Archive};

use super::lister::{ListingResult, Lister};
use crate::file::File;

#[derive(Debug)]
pub struct TarLister;

impl TarLister {
    fn unpack_files(from: File) -> crate::Result<Vec<PathBuf>> {
        
        let mut files_unpacked = vec![];

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
            files_unpacked.push(file_path);
        }

        Ok(files_unpacked)
    }
}

impl Lister for TarLister {
    fn list(
        &self,
        from: File,
    ) -> crate::Result<ListingResult> {
        
        let files_unpacked = Self::unpack_files(from)?;

        Ok(files_unpacked)
    }
}
