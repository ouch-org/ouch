//! Contains Zip-specific building and unpacking functions

use std::{
    env, fs,
    io::{self, prelude::*},
    path::{Path, PathBuf},
};

use walkdir::WalkDir;
use zip::{self, read::ZipFile, ZipArchive};

use crate::{
    info,
    utils::{self, dir_is_empty, Bytes},
};

use self::utf8::get_invalid_utf8_paths;

/// Unpacks the archive given by `archive` into the folder given by `into`.
pub fn unpack_archive<R>(
    mut archive: ZipArchive<R>,
    into: &Path,
    skip_questions_positively: Option<bool>,
) -> crate::Result<Vec<PathBuf>>
where
    R: Read + Seek,
{
    let mut unpacked_files = vec![];
    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let file_path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let file_path = into.join(file_path);
        if file_path.exists() && !utils::user_wants_to_overwrite(&file_path, skip_questions_positively)? {
            continue;
        }

        check_for_comments(&file);

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

                info!("{:?} extracted. ({})", file_path.display(), Bytes::new(file.size()));

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

pub fn build_archive_from_paths<W>(input_filenames: &[PathBuf], writer: W) -> crate::Result<W>
where
    W: Write + Seek,
{
    let mut writer = zip::ZipWriter::new(writer);
    let options = zip::write::FileOptions::default();

    // Vec of any filename that failed the UTF-8 check
    let invalid_unicode_filenames = get_invalid_utf8_paths(input_filenames);

    if let Some(filenames) = invalid_unicode_filenames {
        // TODO: make this an error variant
        panic!("invalid unicode filenames found, cannot be supported by Zip:\n {:#?}", filenames);
    }

    for filename in input_filenames {
        let previous_location = utils::cd_into_same_dir_as(filename)?;

        // Safe unwrap, input shall be treated before
        let filename = filename.file_name().unwrap();

        for entry in WalkDir::new(filename) {
            let entry = entry?;
            let path = entry.path();

            info!("Compressing '{}'.", utils::to_utf(path));

            if path.is_dir() {
                if dir_is_empty(path) {
                    writer.add_directory(path.to_str().unwrap().to_owned(), options)?;
                }
                // If a dir has files, the files are responsible for creating them.
            } else {
                writer.start_file(path.to_str().unwrap().to_owned(), options)?;
                // TODO: better error messages
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
        info!("Found comment in {}: {}", file.name(), comment);
    }
}

#[cfg(unix)]
fn __unix_set_permissions(file_path: &Path, file: &ZipFile) -> crate::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = file.unix_mode() {
        fs::set_permissions(file_path, fs::Permissions::from_mode(mode))?;
    }

    Ok(())
}

mod utf8 {

    use std::path::{Path, PathBuf};

    // Sad double reference in order to make `filter` happy in `get_invalid_utf8_paths`
    #[cfg(unix)]
    fn is_invalid_utf8(path: &&Path) -> bool {
        use std::os::unix::prelude::OsStrExt;
        use std::str;

        // str::from_utf8 does not make any allocations
        let bytes = path.as_os_str().as_bytes();
        let is_invalid = str::from_utf8(bytes).is_err();

        is_invalid
    }

    #[cfg(not(unix))]
    fn is_invalid_utf8(path: &&Path) -> bool {
        path.to_str().is_none()
    }

    pub fn get_invalid_utf8_paths(paths: &[PathBuf]) -> Option<Vec<PathBuf>> {
        let mut invalid_paths = paths.iter().map(PathBuf::as_path).filter(is_invalid_utf8).peekable();

        let a_path_is_invalid = invalid_paths.peek().is_some();

        let clone_paths = || invalid_paths.map(ToOwned::to_owned).collect();

        a_path_is_invalid.then(clone_paths)
    }
}
