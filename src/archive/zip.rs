use std::{
    env, fs,
    io::{self, prelude::*},
    path::{Path, PathBuf},
};

use walkdir::WalkDir;
use zip::{self, read::ZipFile, ZipArchive};

use crate::{
    info, oof,
    utils::{self, Bytes},
};

pub fn unpack_archive<R>(mut archive: ZipArchive<R>, into: &Path, flags: &oof::Flags) -> crate::Result<Vec<PathBuf>>
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
        if file_path.exists() && !utils::user_wants_to_overwrite(&file_path, flags)? {
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

        let file_path = fs::canonicalize(file_path.clone())?;
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
    let invalid_unicode_filenames: Vec<PathBuf> = input_filenames
        .iter()
        .map(|path| (path, path.to_str()))
        .filter(|(_, x)| x.is_none())
        .map(|(a, _)| a.to_path_buf())
        .collect();

    if !invalid_unicode_filenames.is_empty() {
        // TODO: make this an error variant
        panic!("invalid unicode filenames found, cannot be supported by Zip:\n {:#?}", invalid_unicode_filenames);
    }

    for filename in input_filenames {
        let previous_location = utils::cd_into_same_dir_as(filename)?;

        // Safe unwrap, input shall be treated before
        let filename = filename.file_name().unwrap();

        for entry in WalkDir::new(filename) {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            writer.start_file(path.to_str().unwrap().to_owned(), options)?;
            
            // TODO: better error messages
            let file_bytes = fs::read(entry.path())?;
            writer.write_all(&*file_bytes)?;
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
