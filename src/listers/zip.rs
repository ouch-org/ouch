use std::{
    fs,
    io::{Cursor, Read, Seek},
    path::PathBuf,
};

use utils::colors;
use zip::{self, ZipArchive};

use super::lister::{Lister, ListingResult};
use crate::{file::File, utils};
pub struct ZipLister;

impl ZipLister {
    pub fn zip_decompress<R>(archive: &mut ZipArchive<R>) -> crate::Result<Vec<PathBuf>>
    where
        R: Read + Seek,
    {
        let mut unpacked_files = vec![];
        for idx in 0..archive.len() {
            let file = archive.by_index(idx)?;
            let file_path = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            match (&*file.name()).ends_with('/') {
                _is_dir @ true => {
                    println!("File {} extracted to \"{}\"", idx, file_path.display());
                    fs::create_dir_all(&file_path)?;
                }
                _is_file @ false => {
                    if let Some(path) = file_path.parent() {
                        if !path.exists() {
                            fs::create_dir_all(&path)?;
                        }
                    }
                    println!(
                        "{}[INFO]{} \"{}\" extracted. ({})",
                        colors::yellow(),
                        colors::reset(),
                        file_path.display(),
                        utils::Bytes::new(file.size())
                    );
                }
            }
            unpacked_files.push(file_path);
        }

        Ok(unpacked_files)
    }

    fn unpack_files(from: File) -> crate::Result<Vec<PathBuf>> {
        println!(
            "{}[INFO]{} decompressing {:?}",
            colors::blue(),
            colors::reset(),
            &from.path
        );

        match from.contents_in_memory {
            Some(bytes) => {
                // Decompressing a .zip archive loaded up in memory
                let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
                Ok(Self::zip_decompress(&mut archive)?)
            }
            None => {
                // Decompressing a .zip archive from the file system
                let file = fs::File::open(&from.path)?;
                let mut archive = zip::ZipArchive::new(file)?;

                Ok(Self::zip_decompress(&mut archive)?)
            }
        }
    }
}

impl Lister for ZipLister {
    fn list(&self, from: File) -> crate::Result<ListingResult> {
        let files_unpacked = Self::unpack_files(from)?;

        Ok(files_unpacked)
    }
}
