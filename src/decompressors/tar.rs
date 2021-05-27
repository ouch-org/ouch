use std::{
    fs,
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use tar::{self, Archive};
use utils::colors;

use super::decompressor::{DecompressionResult, Decompressor};
use crate::{dialogs::Confirmation, file::File, oof, utils};

#[derive(Debug)]
pub struct TarDecompressor;

impl TarDecompressor {
    fn unpack_files(from: File, into: &Path, flags: &oof::Flags) -> crate::Result<Vec<PathBuf>> {
        println!(
            "{}[INFO]{} attempting to decompress {:?}",
            colors::blue(),
            colors::reset(),
            &from.path
        );
        let mut files_unpacked = vec![];
        let confirm = Confirmation::new("Do you want to overwrite 'FILE'?", Some("FILE"));

        let mut archive: Archive<Box<dyn Read>> = match from.contents_in_memory {
            Some(bytes) => tar::Archive::new(Box::new(Cursor::new(bytes))),
            None => {
                let file = fs::File::open(&from.path)?;
                tar::Archive::new(Box::new(file))
            },
        };

        for file in archive.entries()? {
            let mut file = file?;

            let file_path = PathBuf::from(into).join(file.path()?);
            if file_path.exists()
                && !utils::permission_for_overwriting(&file_path, flags, &confirm)?
            {
                // The user does not want to overwrite the file
                continue;
            }

            file.unpack_in(into)?;

            println!(
                "{}[INFO]{} {:?} extracted. ({})",
                colors::yellow(),
                colors::reset(),
                into.join(file.path()?),
                utils::Bytes::new(file.size())
            );

            let file_path = fs::canonicalize(file_path)?;
            files_unpacked.push(file_path);
        }

        Ok(files_unpacked)
    }
}

impl Decompressor for TarDecompressor {
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
