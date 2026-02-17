//! SevenZip archive format compress function

use std::{
    env,
    io::{self, Read, Seek, Write},
    path::{Path, PathBuf},
};

use bstr::ByteSlice;
use fs_err as fs;
use fs_err::PathExt;
use same_file::Handle;
use sevenz_rust2::ArchiveEntry;

use crate::{
    error::{Error, FinalError, Result},
    info,
    list::FileInArchive,
    utils::{cd_into_same_dir_as, BytesFmt, FileVisibilityPolicy, PathFmt},
    warning,
};

pub fn compress_sevenz<W>(
    files: &[PathBuf],
    output_path: &Path,
    writer: W,
    file_visibility_policy: FileVisibilityPolicy,
) -> crate::Result<W>
where
    W: Write + Seek,
{
    let mut writer = sevenz_rust2::ArchiveWriter::new(writer)?;
    let output_handle = Handle::from_path(output_path);

    for filename in files {
        let previous_location = cd_into_same_dir_as(filename)?;

        // Unwrap safety:
        //   paths should be canonicalized by now, and the root directory rejected.
        let filename = filename.file_name().unwrap();

        for entry in file_visibility_policy.build_walker(filename) {
            let entry = entry?;
            let path = entry.path();

            // If the output_path is the same as the input file, warn the user and skip the input (in order to avoid compression recursion)
            if let Ok(handle) = &output_handle {
                if matches!(Handle::from_path(path), Ok(x) if &x == handle) {
                    warning!("Cannot compress {:?} into itself, skipping", PathFmt(output_path));

                    continue;
                }
            }

            info!("Compressing {:?}", PathFmt(path));

            let metadata = match path.metadata() {
                Ok(metadata) => metadata,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound && path.is_symlink() {
                        // This path is for a broken symlink, ignore it
                        continue;
                    }
                    return Err(e.into());
                }
            };

            let entry_name = path.to_str().ok_or_else(|| {
                FinalError::with_title("7z requires that all entry names are valid UTF-8")
                    .detail(format!("File {:?} has a non-UTF-8 name", PathFmt(path)))
            })?;

            let entry = sevenz_rust2::ArchiveEntry::from_path(path, entry_name.to_owned());
            let entry_data = if metadata.is_dir() {
                None
            } else {
                Some(fs::File::open(path)?)
            };

            writer.push_archive_entry::<fs::File>(entry, entry_data)?;
        }

        env::set_current_dir(previous_location)?;
    }

    let bytes = writer.finish()?;
    Ok(bytes)
}

pub fn decompress_sevenz<R>(reader: R, output_path: &Path, password: Option<&[u8]>) -> crate::Result<u64>
where
    R: Read + Seek,
{
    let mut files_unpacked = 0;

    let entry_extract_fn = |entry: &ArchiveEntry, reader: &mut dyn Read, path: &PathBuf| {
        files_unpacked += 1;
        // Manually handle writing all files from 7z archive, due to library exluding empty files
        use std::io::BufWriter;

        use filetime_creation as ft;

        let file_path = output_path.join(entry.name());

        if entry.is_directory() {
            info!("File {} extracted to {:?}", entry.name(), PathFmt(&file_path));
            if !path.fs_err_try_exists()? {
                fs::create_dir_all(path)?;
            }
        } else {
            info!("extracted ({}) {:?}", BytesFmt(entry.size()), PathFmt(&file_path));

            if let Some(parent) = path.parent() {
                if !parent.fs_err_try_exists()? {
                    fs::create_dir_all(parent)?;
                }
            }

            let file = fs::File::create(path)?;
            let mut writer = BufWriter::new(file);
            io::copy(reader, &mut writer)?;

            ft::set_file_handle_times(
                writer.get_ref().file(),
                Some(ft::FileTime::from_system_time(entry.access_date().into())),
                Some(ft::FileTime::from_system_time(entry.last_modified_date().into())),
                Some(ft::FileTime::from_system_time(entry.creation_date().into())),
            )
            .unwrap_or_default();
        }

        Ok(true)
    };

    match password {
        Some(password) => sevenz_rust2::decompress_with_extract_fn_and_password(
            reader,
            output_path,
            sevenz_rust2::Password::from(password.to_str().map_err(|err| Error::InvalidPassword {
                reason: err.to_string(),
            })?),
            entry_extract_fn,
        )?,
        None => sevenz_rust2::decompress_with_extract_fn(reader, output_path, entry_extract_fn)?,
    }

    Ok(files_unpacked)
}

/// List contents of `archive_path`, returning a vector of archive entries
pub fn list_archive<R>(reader: R, password: Option<&[u8]>) -> Result<impl Iterator<Item = crate::Result<FileInArchive>>>
where
    R: Read + Seek,
{
    let mut files = Vec::new();

    let entry_extract_fn = |entry: &ArchiveEntry, _: &mut dyn Read, _: &PathBuf| {
        files.push(Ok(FileInArchive {
            path: entry.name().into(),
            is_dir: entry.is_directory(),
        }));
        Ok(true)
    };

    match password {
        Some(password) => {
            let password = match password.to_str() {
                Ok(p) => p,
                Err(err) => {
                    return Err(Error::InvalidPassword {
                        reason: err.to_string(),
                    })
                }
            };
            sevenz_rust2::decompress_with_extract_fn_and_password(
                reader,
                ".",
                sevenz_rust2::Password::from(password),
                entry_extract_fn,
            )?;
        }
        None => sevenz_rust2::decompress_with_extract_fn(reader, ".", entry_extract_fn)?,
    }

    Ok(files.into_iter())
}
