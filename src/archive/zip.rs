//! Contains Zip-specific building and unpacking functions

use std::{
    env,
    io::{self, prelude::*},
    path::{Path, PathBuf},
};

use fs_err as fs;
use walkdir::WalkDir;
use zip::{self, read::ZipFile, ZipArchive};

use crate::{
    error::FinalError,
    info,
    list::FileInArchive,
    utils::{
        cd_into_same_dir_as, concatenate_os_str_list, dir_is_empty, get_invalid_utf8_paths, strip_cur_dir, to_utf,
        Bytes,
    },
};

/// Unpacks the archive given by `archive` into the folder given by `output_folder`.
/// Assumes that output_folder is empty
pub fn unpack_archive<R, D>(
    mut archive: ZipArchive<R>,
    output_folder: &Path,
    mut display_handle: D,
) -> crate::Result<Vec<PathBuf>>
where
    R: Read + Seek,
    D: Write,
{
    assert!(output_folder.read_dir().expect("dir exists").count() == 0);
    let mut unpacked_files = vec![];
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let file_path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let file_path = output_folder.join(file_path);

        check_for_comments(&file);

        match (&*file.name()).ends_with('/') {
            _is_dir @ true => {
                // This is printed for every file in the archive and has little
                // importance for most users, but would generate lots of
                // spoken text for users using screen readers, braille displays
                // and so on
                info!(@display_handle, inaccessible, "File {} extracted to \"{}\"", idx, file_path.display());
                fs::create_dir_all(&file_path)?;
            }
            _is_file @ false => {
                if let Some(path) = file_path.parent() {
                    if !path.exists() {
                        fs::create_dir_all(&path)?;
                    }
                }
                let file_path = strip_cur_dir(file_path.as_path());

                // same reason is in _is_dir: long, often not needed text
                info!(@display_handle, inaccessible, "{:?} extracted. ({})", file_path.display(), Bytes::new(file.size()));

                let mut output_file = fs::File::create(&file_path)?;
                io::copy(&mut file, &mut output_file)?;
            }
        }

        #[cfg(unix)]
        __unix_set_permissions(&file_path, &file)?;

        let file_path = fs::canonicalize(&file_path)?;
        unpacked_files.push(file_path);
    }

    Ok(unpacked_files)
}

/// List contents of `archive`, returning a vector of archive entries
pub fn list_archive<R>(mut archive: ZipArchive<R>) -> crate::Result<Vec<FileInArchive>>
where
    R: Read + Seek,
{
    let mut files = vec![];
    for idx in 0..archive.len() {
        let file = archive.by_index(idx)?;

        let path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };
        let is_dir = file.is_dir();

        files.push(FileInArchive { path, is_dir });
    }
    Ok(files)
}

/// Compresses the archives given by `input_filenames` into the file given previously to `writer`.
pub fn build_archive_from_paths<W, D>(input_filenames: &[PathBuf], writer: W, mut display_handle: D) -> crate::Result<W>
where
    W: Write + Seek,
    D: Write,
{
    let mut writer = zip::ZipWriter::new(writer);
    let options = zip::write::FileOptions::default();

    // Vec of any filename that failed the UTF-8 check
    let invalid_unicode_filenames = get_invalid_utf8_paths(input_filenames);

    if !invalid_unicode_filenames.is_empty() {
        let error = FinalError::with_title("Cannot build zip archive")
            .detail("Zip archives require files to have valid UTF-8 paths")
            .detail(format!("Files with invalid paths: {}", concatenate_os_str_list(&invalid_unicode_filenames)));

        return Err(error.into());
    }

    for filename in input_filenames {
        let previous_location = cd_into_same_dir_as(filename)?;

        // Safe unwrap, input shall be treated before
        let filename = filename.file_name().unwrap();

        for entry in WalkDir::new(filename) {
            let entry = entry?;
            let path = entry.path();

            // This is printed for every file in `input_filenames` and has
            // little importance for most users, but would generate lots of
            // spoken text for users using screen readers, braille displays
            // and so on
            info!(@display_handle, inaccessible, "Compressing '{}'.", to_utf(path));

            if path.is_dir() {
                if dir_is_empty(path) {
                    writer.add_directory(path.to_str().unwrap().to_owned(), options)?;
                }
                // If a dir has files, the files are responsible for creating them.
            } else {
                writer.start_file(path.to_str().unwrap().to_owned(), options)?;
                let file_bytes = fs::read(entry.path())?;
                writer.write_all(&*file_bytes)?;
            }
        }

        env::set_current_dir(previous_location)?;
    }

    let bytes = writer.finish()?;
    Ok(bytes)
}

fn check_for_comments(file: &ZipFile) {
    let comment = file.comment();
    if !comment.is_empty() {
        // Zip file comments seem to be pretty rare, but if they are used,
        // they may contain important information, so better show them
        //
        // "The .ZIP file format allows for a comment containing up to 65,535 (216−1) bytes
        // of data to occur at the end of the file after the central directory."
        //
        // If there happen to be cases of very long and unnecessary comments in
        // the future, maybe asking the user if he wants to display the comment
        // (informing him of its size) would be sensible for both normal and
        // accessibility mode..
        info!(accessible, "Found comment in {}: {}", file.name(), comment);
    }
}

#[cfg(unix)]
fn __unix_set_permissions(file_path: &Path, file: &ZipFile) -> crate::Result<()> {
    use std::{fs::Permissions, os::unix::fs::PermissionsExt};

    if let Some(mode) = file.unix_mode() {
        fs::set_permissions(file_path, Permissions::from_mode(mode))?;
    }

    Ok(())
}
