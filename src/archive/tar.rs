//! Contains Tar-specific building and unpacking functions

use std::{
    env,
    io::prelude::*,
    path::{Path, PathBuf},
};

use fs_err as fs;
use tar;

use crate::{
    error::FinalError,
    info,
    list::FileInArchive,
    utils::{self, walk_dir, Bytes},
    QuestionPolicy,
};

/// Unpacks the archive given by `archive` into the folder given by `into`.
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

        if file_path.is_dir() {
            // ToDo: Maybe we should emphasise that `file_path` is a directory and everything inside it will be gone?
            fs::remove_dir_all(&file_path)?;
        } else if file_path.is_file() {
            fs::remove_file(&file_path)?;
        }

        file.unpack_in(output_folder)?;

        info!("{:?} extracted. ({})", output_folder.join(file.path()?), Bytes::new(file.size()));

        files_unpacked.push(file_path);
    }

    Ok(files_unpacked)
}

/// List contents of `archive`, returning a vector of archive entries
pub fn list_archive(reader: Box<dyn Read>) -> crate::Result<Vec<FileInArchive>> {
    let mut archive = tar::Archive::new(reader);

    let mut files = vec![];
    for file in archive.entries()? {
        let file = file?;

        let path = file.path()?.into_owned();
        let is_dir = file.header().entry_type().is_dir();

        files.push(FileInArchive { path, is_dir });
    }

    Ok(files)
}

/// Compresses the archives given by `input_filenames` into the file given previously to `writer`.
pub fn build_archive_from_paths<W>(input_filenames: &[PathBuf], writer: W, read_hidden_files: bool) -> crate::Result<W>
where
    W: Write,
{
    let mut builder = tar::Builder::new(writer);

    for filename in input_filenames {
        let previous_location = utils::cd_into_same_dir_as(filename)?;

        // Safe unwrap, input shall be treated before
        let filename = filename.file_name().unwrap();

        for entry in walk_dir(filename, read_hidden_files) {
            let entry = entry?;
            let path = entry.path();

            info!("Compressing '{}'.", utils::to_utf(path));

            if path.is_dir() {
                builder.append_dir(path, path)?;
            } else {
                let mut file = fs::File::open(path)?;
                builder.append_file(path, file.file_mut()).map_err(|err| {
                    FinalError::with_title("Could not create archive")
                        .detail("Unexpected error while trying to read file")
                        .detail(format!("Error: {}.", err))
                })?;
            }
        }
        env::set_current_dir(previous_location)?;
    }

    Ok(builder.into_inner()?)
}
