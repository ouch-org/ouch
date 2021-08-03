use std::{
    io::Read,
    path::{Path, PathBuf},
};

use tar;
use utils::colors;

use crate::{oof, utils};
pub fn unpack_archive(
    reader: Box<dyn Read>,
    output_folder: &Path,
    flags: &oof::Flags,
) -> crate::Result<Vec<PathBuf>> {
    // TODO: move this printing to the caller.
    // println!(
    //     "{}[INFO]{} attempting to decompress {:?}",
    //     colors::blue(),
    //     colors::reset(),
    //     &input_path
    // );

    let mut archive = tar::Archive::new(reader);

    let mut files_unpacked = vec![];
    for file in archive.entries()? {
        let mut file = file?;

        let file_path = output_folder.join(file.path()?);
        if file_path.exists() && !utils::permission_for_overwriting(&file_path, flags)? {
            // The user does not want to overwrite the file
            continue;
        }

        file.unpack_in(output_folder)?;

        println!(
            "{}[INFO]{} {:?} extracted. ({})",
            colors::yellow(),
            colors::reset(),
            output_folder.join(file.path()?),
            utils::Bytes::new(file.size())
        );

        files_unpacked.push(file_path);
    }

    Ok(files_unpacked)
}
