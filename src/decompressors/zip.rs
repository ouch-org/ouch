use std::{
    fs,
    io::{self, Cursor, Read, Seek},
    path::{Path, PathBuf},
};

use colored::Colorize;
use zip::{self, read::ZipFile, ZipArchive};

use super::decompressor::{DecompressionResult, Decompressor};
use crate::{file::File, utils};

pub struct ZipDecompressor {}

impl ZipDecompressor {
    fn check_for_comments(file: &ZipFile) {
        let comment = file.comment();
        if !comment.is_empty() {
            println!(
                "{}: Comment in {}: {}",
                "info".yellow(),
                file.name(),
                comment
            );
        }
    }

    pub fn zip_decompress<T>(
        archive: &mut ZipArchive<T>,
        into: &Path,
    ) -> crate::Result<Vec<PathBuf>>
    where
        T: Read + Seek,
    {
        let mut unpacked_files = vec![];
        for idx in 0..archive.len() {
            let mut file = archive.by_index(idx)?;
            let file_path = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            let file_path = into.join(file_path);

            Self::check_for_comments(&file);

            let is_dir = (&*file.name()).ends_with('/');

            if is_dir {
                println!("File {} extracted to \"{}\"", idx, file_path.display());
                fs::create_dir_all(&file_path)?;
            } else {
                if let Some(p) = file_path.parent() {
                    if !p.exists() {
                        fs::create_dir_all(&p)?;
                    }
                }
                println!(
                    "{}: \"{}\" extracted. ({} bytes)",
                    "info".yellow(),
                    file_path.display(),
                    file.size()
                );
                let mut outfile = fs::File::create(&file_path)?;
                io::copy(&mut file, &mut outfile)?;
            }

            // TODO: check if permissions are correct when on Unix

            let file_path = fs::canonicalize(file_path.clone())?;
            unpacked_files.push(file_path);
        }

        Ok(unpacked_files)
    }

    fn unpack_files(from: File, into: &Path) -> crate::Result<Vec<PathBuf>> {
        println!(
            "{}: attempting to decompress {:?}",
            "ouch".bright_blue(),
            &from.path
        );

        match from.contents_in_memory {
            Some(bytes) => {
                let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
                Ok(Self::zip_decompress(&mut archive, into)?)
            }
            None => {
                let file = fs::File::open(&from.path)?;
                let mut archive = zip::ZipArchive::new(file)?;
                Ok(Self::zip_decompress(&mut archive, into)?)
            }
        }
    }
}

impl Decompressor for ZipDecompressor {
    fn decompress(&self, from: File, into: &Option<File>) -> crate::Result<DecompressionResult> {
        let destination_path = utils::get_destination_path(into);

        utils::create_path_if_non_existent(destination_path)?;

        let files_unpacked = Self::unpack_files(from, destination_path)?;

        Ok(DecompressionResult::FilesUnpacked(files_unpacked))
    }
}
