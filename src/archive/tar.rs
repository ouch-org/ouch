//! Contains Tar-specific building and unpacking functions

use std::{
    env, fs,
    io::prelude::*,
    path::{Path, PathBuf},
};

use tar;
use walkdir::WalkDir;

use crate::{
    info,
    utils::{self, Bytes},
    QuestionPolicy,
};

pub fn unpack_archive(
    reader: Box<dyn Read>,
    output_folder: &Path,
    question_policy: QuestionPolicy,
) -> crate::Result<Vec<PathBuf>> {
    let mut archive = tar::Archive::new(reader);

    let mut files_unpacked = vec![];
    for file in archive.entries()? {
        let mut file = file?;

        let file_path = output_folder.join(file.path()?);
        if file_path.exists() && !utils::user_wants_to_overwrite(&file_path, question_policy)? {
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
                builder.append_file(path, &mut file)?;
            }
        }
        env::set_current_dir(previous_location)?;
    }

    Ok(builder.into_inner()?)
}
