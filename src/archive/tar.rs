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
    info,
    list::FileInArchive,
    utils::{self, Bytes},
};

/// Unpacks the archive given by `archive` into the folder given by `into`.
/// Assumes that output_folder is empty
pub fn unpack_archive(
    reader: Box<dyn Read>,
    output_folder: &Path,
    mut display_handle: impl Write,
) -> crate::Result<Vec<PathBuf>> {
    assert!(output_folder.read_dir().expect("dir exists").count() == 0);
    let mut archive = tar::Archive::new(reader);

    let mut files_unpacked = vec![];
    for file in archive.entries()? {
        let mut file = file?;

        let file_path = output_folder.join(file.path()?);
        file.unpack_in(output_folder)?;

        // This is printed for every file in the archive and has little
        // importance for most users, but would generate lots of
        // spoken text for users using screen readers, braille displays
        // and so on
        info!(@display_handle, inaccessible, "{:?} extracted. ({})", output_folder.join(file.path()?), Bytes::new(file.size()));

        files_unpacked.push(file_path);
    }

    Ok(files_unpacked)
}

/// List contents of `archive`, returning a vector of archive entries
pub fn list_archive(
    archive: tar::Archive<impl Read + 'static>,
) -> crate::Result<impl Iterator<Item = crate::Result<FileInArchive>>> {
    // NOTE: tar::Archive::entries takes a &mut self
    // This makes returning an iterator impossible
    // Current workaround is just to leak the archive
    // This can be replaced when upstream add `into_entries` function that consumes the archive
    let archive = Box::leak(Box::new(archive));

    Ok(archive.entries()?.map(|file| {
        let file = file?;

        let path = file.path()?.into_owned();
        let is_dir = file.header().entry_type().is_dir();

        Ok(FileInArchive { path, is_dir })
    }))
}

/// Compresses the archives given by `input_filenames` into the file given previously to `writer`.
pub fn build_archive_from_paths<W, D>(input_filenames: &[PathBuf], writer: W, mut display_handle: D) -> crate::Result<W>
where
    W: Write,
    D: Write,
{
    let mut builder = tar::Builder::new(writer);

    for filename in input_filenames {
        let previous_location = utils::cd_into_same_dir_as(filename)?;

        // Safe unwrap, input shall be treated before
        let filename = filename.file_name().unwrap();

        for entry in WalkDir::new(&filename) {
            let entry = entry?;
            let path = entry.path();

            // This is printed for every file in `input_filenames` and has
            // little importance for most users, but would generate lots of
            // spoken text for users using screen readers, braille displays
            // and so on
            info!(@display_handle, inaccessible, "Compressing '{}'.", utils::to_utf(path));

            if path.is_dir() {
                builder.append_dir(path, path)?;
            } else {
                let mut file = match fs::File::open(path) {
                    Ok(f) => f,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound && utils::is_symlink(path) {
                            // This path is for a broken symlink
                            // We just ignore it
                            continue;
                        }
                        return Err(e.into());
                    }
                };
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
