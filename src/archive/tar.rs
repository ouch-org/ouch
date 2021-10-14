//! Contains Tar-specific building and unpacking functions

use std::{
    env,
    io::prelude::*,
    path::{Path, PathBuf},
};

use fs_err as fs;
use tar;
use walkdir::WalkDir;

use crate::{
    error::FinalError,
    info, oof,
    utils::{self, to_utf, Bytes},
};

pub fn unpack_archive(reader: Box<dyn Read>, output_folder: &Path, flags: &oof::Flags) -> crate::Result<Vec<PathBuf>> {
    let mut archive = tar::Archive::new(reader);

    let mut files_unpacked = vec![];
    for file in archive.entries()? {
        let mut file = file?;

        let file_path = output_folder.join(file.path()?);
        if file_path.exists() && !utils::user_wants_to_overwrite(&file_path, flags)? {
            continue;
        }

        file.unpack_in(output_folder)?;

        info!("{:?} extracted. ({})", output_folder.join(file.path()?), Bytes::new(file.size()));

        files_unpacked.push(file_path);
    }

    Ok(files_unpacked)
}

pub fn build_archive_from_paths<W>(input_filenames: &[PathBuf], writer: W) -> crate::Result<W>
where
    W: Write,
{
    let mut builder = tar::Builder::new(writer);

    for filename in input_filenames {
        let previous_location = utils::cd_into_same_dir_as(filename)?;

        // Safe unwrap, input shall be treated before
        let filename = filename.file_name().unwrap();

        for entry in WalkDir::new(&filename) {
            let entry = entry?;
            let path = entry.path();

            info!("Compressing '{}'.", utils::to_utf(path));

            if path.is_dir() {
                builder.append_dir(path, path)?;
            } else {
                let mut file = fs::File::open(path)?;
                dbg!(&path);
                dbg!(&file);
                dbg!(&entry);
                dbg!(&previous_location);
                dbg!(&filename);

                // builder.append_file(path, file.file_mut())?;
                builder.append_file(path, file.file_mut()).map_err(|err| {
                    FinalError::with_title(format!("Could not create archive '{}'", to_utf(path.clone()))) // output_path == writer? da
                        .detail(format!("Unexpected error while trying to read file '{}'", to_utf(output_path)))
                        .detail(format!("Error: {}.", err))
                })?;
            }
        }
        env::set_current_dir(previous_location)?;
    }

    Ok(builder.into_inner()?)
}
