use std::{
    fs,
    io::{self, Read, Seek},
    path::{Path, PathBuf},
};

use zip::{self, read::ZipFile, ZipArchive};

use crate::{
    oof,
    utils::{self, colors},
};

#[cfg(unix)]
fn __unix_set_permissions(file_path: &Path, file: &ZipFile) {
    use std::os::unix::fs::PermissionsExt;

    if let Some(mode) = file.unix_mode() {
        fs::set_permissions(&file_path, fs::Permissions::from_mode(mode)).unwrap();
    }
}

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

pub fn unpack_archive<R>(
    mut archive: ZipArchive<R>,
    into: &Path,
    flags: &oof::Flags,
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
        if file_path.exists() && !utils::permission_for_overwriting(&file_path, flags)? {
            // The user does not want to overwrite the file
            continue;
        }

        check_for_comments(&file);

        match (&*file.name()).ends_with('/') {
            _is_dir @ true => {
                println!("File {} extracted to \"{}\"", idx, file_path.display());
                fs::create_dir_all(&file_path)?;
            },
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
            },
        }

        #[cfg(unix)]
        __unix_set_permissions(&file_path, &file);

        let file_path = fs::canonicalize(file_path.clone())?;
        unpacked_files.push(file_path);
    }

    Ok(unpacked_files)
}
