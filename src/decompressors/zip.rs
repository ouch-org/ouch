use std::{
    fs,
    io::{self, Cursor, Read, Seek},
    path::{Path, PathBuf},
};

use utils::colors;
use zip::{self, read::ZipFile, ZipArchive};

use super::decompressor::{DecompressionResult, Decompressor};
use crate::{dialogs::Confirmation, file::File, utils};

#[cfg(unix)]
fn __unix_set_permissions(file_path: &Path, file: &ZipFile) {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = file.unix_mode() {
        fs::set_permissions(&file_path, fs::Permissions::from_mode(mode)).unwrap();
    }
}

pub struct ZipDecompressor;

impl ZipDecompressor {
    fn check_for_comments(file: &ZipFile) {
        let comment = file.comment();
        if !comment.is_empty() {
            println!(
                "{}[INFO]{} Comment in {}: {}",
                colors::yellow(),
                colors::reset(),
                file.name(),
                comment
            );
        }
    }

    pub fn zip_decompress<R>(
        archive: &mut ZipArchive<R>,
        into: &Path,
        flags: &oof::Flags,
    ) -> crate::Result<Vec<PathBuf>>
    where
        R: Read + Seek,
    {
        let confirm = Confirmation::new("Do you want to overwrite 'FILE'?", Some("FILE"));
        let mut unpacked_files = vec![];
        for idx in 0..archive.len() {
            let mut file = archive.by_index(idx)?;
            let file_path = match file.enclosed_name() {
                Some(path) => path.to_owned(),
                None => continue,
            };

            let file_path = into.join(file_path);
            if file_path.exists()
                && !utils::permission_for_overwriting(&file_path, flags, &confirm)?
            {
                // The user does not want to overwrite the file
                continue;
            }

            Self::check_for_comments(&file);

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

                    let mut output_file = fs::File::create(&file_path)?;
                    io::copy(&mut file, &mut output_file)?;
                }
            }

            #[cfg(unix)]
            __unix_set_permissions(&file_path, &file);

            let file_path = fs::canonicalize(file_path.clone())?;
            unpacked_files.push(file_path);
        }

        Ok(unpacked_files)
    }

    fn unpack_files(from: File, into: &Path, flags: &oof::Flags) -> crate::Result<Vec<PathBuf>> {
        println!("{}[INFO]{} decompressing {:?}", colors::blue(), colors::reset(), &from.path);

        match from.contents_in_memory {
            Some(bytes) => {
                // Decompressing a .zip archive loaded up in memory
                let mut archive = zip::ZipArchive::new(Cursor::new(bytes))?;
                Ok(Self::zip_decompress(&mut archive, into, flags)?)
            }
            None => {
                // Decompressing a .zip archive from the file system
                let file = fs::File::open(&from.path)?;
                let mut archive = zip::ZipArchive::new(file)?;

                Ok(Self::zip_decompress(&mut archive, into, flags)?)
            }
        }
    }
}

impl Decompressor for ZipDecompressor {
    fn decompress(
        &self,
        from: File,
        into: &Option<File>,
        flags: &oof::Flags,
    ) -> crate::Result<DecompressionResult> {
        let destination_path = utils::get_destination_path(into);

        utils::create_path_if_non_existent(destination_path)?;

        let files_unpacked = Self::unpack_files(from, destination_path, flags)?;

        Ok(DecompressionResult::FilesUnpacked(files_unpacked))
    }
}
