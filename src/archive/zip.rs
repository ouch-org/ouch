//! Contains Zip-specific building and unpacking functions

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    env,
    io::{self, prelude::*},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use filetime_creation::{set_file_mtime, FileTime};
use fs_err as fs;
use same_file::Handle;
use time::OffsetDateTime;
use zip::{self, read::ZipFile, DateTime, ZipArchive};

use crate::{
    error::FinalError,
    list::FileInArchive,
    utils::{
        cd_into_same_dir_as, get_invalid_utf8_paths,
        logger::{info, info_accessible, warning},
        pretty_format_list_of_paths, strip_cur_dir, Bytes, EscapedPathDisplay, FileVisibilityPolicy,
    },
};

/// Unpacks the archive given by `archive` into the folder given by `output_folder`.
/// Assumes that output_folder is empty
pub fn unpack_archive<R>(
    mut archive: ZipArchive<R>,
    output_folder: &Path,
    password: Option<&[u8]>,
    quiet: bool,
) -> crate::Result<usize>
where
    R: Read + Seek,
{
    let mut unpacked_files = 0;

    for idx in 0..archive.len() {
        let mut file = match password {
            Some(password) => archive.by_index_decrypt(idx, password)?,
            None => archive.by_index(idx)?,
        };
        let file_path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let file_path = output_folder.join(file_path);

        display_zip_comment_if_exists(&file);

        match file.name().ends_with('/') {
            _is_dir @ true => {
                // This is printed for every file in the archive and has little
                // importance for most users, but would generate lots of
                // spoken text for users using screen readers, braille displays
                // and so on
                if !quiet {
                    info(format!("File {} extracted to \"{}\"", idx, file_path.display()));
                }
                fs::create_dir_all(&file_path)?;

                #[cfg(unix)]
                unix_set_permissions(&file_path, &file)?;
            }
            _is_file @ false => {
                if let Some(path) = file_path.parent() {
                    if !path.exists() {
                        fs::create_dir_all(path)?;
                    }
                }
                let file_path = strip_cur_dir(file_path.as_path());

                let mode = file.unix_mode();
                let is_symlink = mode.is_some_and(|mode| mode & 0o170000 == 0o120000);

                if is_symlink {
                    let mut target = String::new();
                    file.read_to_string(&mut target)?;

                    if !quiet {
                        info(format!("linking {} -> {}", file_path.display(), target));
                    }

                    #[cfg(unix)]
                    std::os::unix::fs::symlink(&target, file_path)?;
                    #[cfg(windows)]
                    std::os::windows::fs::symlink_file(&target, file_path)?;
                } else {
                    let mut output_file = fs::File::create(file_path)?;
                    io::copy(&mut file, &mut output_file)?;
                    set_last_modified_time(&file, file_path)?;
                    #[cfg(unix)]
                    unix_set_permissions(&file_path, &file)?;
                }

                // same reason is in _is_dir: long, often not needed text
                if !quiet {
                    info(format!(
                        "extracted ({}) {:?}",
                        Bytes::new(file.size()),
                        file_path.display(),
                    ));
                }
            }
        }

        unpacked_files += 1;
    }

    Ok(unpacked_files)
}

/// List contents of `archive`, returning a vector of archive entries
pub fn list_archive<R>(
    mut archive: ZipArchive<R>,
    password: Option<&[u8]>,
) -> impl Iterator<Item = crate::Result<FileInArchive>>
where
    R: Read + Seek + Send + 'static,
{
    struct Files(mpsc::Receiver<crate::Result<FileInArchive>>);
    impl Iterator for Files {
        type Item = crate::Result<FileInArchive>;

        fn next(&mut self) -> Option<Self::Item> {
            self.0.recv().ok()
        }
    }

    let password = password.map(|p| p.to_owned());

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for idx in 0..archive.len() {
            let file_in_archive = (|| {
                let zip_result = match password.clone() {
                    Some(password) => archive.by_index_decrypt(idx, &password),
                    None => archive.by_index(idx),
                };

                let file = match zip_result {
                    Ok(f) => f,
                    Err(e) => return Err(e.into()),
                };

                let path = file.enclosed_name().unwrap_or_else(|| file.mangled_name()).to_owned();
                let is_dir = file.is_dir();

                Ok(FileInArchive { path, is_dir })
            })();
            tx.send(file_in_archive).unwrap();
        }
    });

    Files(rx)
}

/// Compresses the archives given by `input_filenames` into the file given previously to `writer`.
pub fn build_archive_from_paths<W>(
    input_filenames: &[PathBuf],
    output_path: &Path,
    writer: W,
    file_visibility_policy: FileVisibilityPolicy,
    quiet: bool,
    follow_symlinks: bool,
) -> crate::Result<W>
where
    W: Write + Seek,
{
    let mut writer = zip::ZipWriter::new(writer);
    // always use ZIP64 to allow compression of files larger than 4GB
    // the format is widely supported and the extra 20B is negligible in most cases
    let options = zip::write::SimpleFileOptions::default().large_file(true);
    let output_handle = Handle::from_path(output_path);

    #[cfg(not(unix))]
    let executable = options.unix_permissions(0o755);

    // Vec of any filename that failed the UTF-8 check
    let invalid_unicode_filenames = get_invalid_utf8_paths(input_filenames);

    if !invalid_unicode_filenames.is_empty() {
        let error = FinalError::with_title("Cannot build zip archive")
            .detail("Zip archives require files to have valid UTF-8 paths")
            .detail(format!(
                "Files with invalid paths: {}",
                pretty_format_list_of_paths(&invalid_unicode_filenames)
            ));

        return Err(error.into());
    }

    for filename in input_filenames {
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
                    warning(format!(
                        "Cannot compress `{}` into itself, skipping",
                        output_path.display()
                    ));
                }
            }

            // This is printed for every file in `input_filenames` and has
            // little importance for most users, but would generate lots of
            // spoken text for users using screen readers, braille displays
            // and so on
            if !quiet {
                info(format!("Compressing '{}'", EscapedPathDisplay::new(path)));
            }

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

            #[cfg(unix)]
            let mode = metadata.permissions().mode();

            let entry_name = path.to_str().ok_or_else(|| {
                FinalError::with_title("Zip requires that all directories names are valid UTF-8")
                    .detail(format!("File at '{path:?}' has a non-UTF-8 name"))
            })?;

            if metadata.is_dir() {
                writer.add_directory(entry_name, options)?;
            } else if path.is_symlink() && !follow_symlinks {
                let target_path = path.read_link()?;
                let target_name = target_path.to_str().ok_or_else(|| {
                    FinalError::with_title("Zip requires that all directories names are valid UTF-8")
                        .detail(format!("File at '{target_path:?}' has a non-UTF-8 name"))
                })?;

                // This approach writes the symlink target path as the content of the symlink entry.
                // We detect symlinks during extraction by checking for the Unix symlink mode (0o120000) in the entry's permissions.
                #[cfg(unix)]
                let symlink_options = options.unix_permissions(0o120000 | (mode & 0o777));
                #[cfg(windows)]
                let symlink_options = options.unix_permissions(0o120777);

                writer.add_symlink(entry_name, target_name, symlink_options)?;
            } else {
                #[cfg(not(unix))]
                let options = if is_executable::is_executable(path) {
                    executable
                } else {
                    options
                };

                let mut file = fs::File::open(path)?;

                #[cfg(unix)]
                let options = options.unix_permissions(mode);
                // Updated last modified time
                let last_modified_time = options.last_modified_time(get_last_modified_time(&file));

                writer.start_file(entry_name, last_modified_time)?;
                io::copy(&mut file, &mut writer)?;
            }
        }

        env::set_current_dir(previous_location)?;
    }

    let bytes = writer.finish()?;
    Ok(bytes)
}

fn display_zip_comment_if_exists<R: Read>(file: &ZipFile<'_, R>) {
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
        info_accessible(format!("Found comment in {}: {}", file.name(), comment));
    }
}

fn get_last_modified_time(file: &fs::File) -> DateTime {
    file.metadata()
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|time| DateTime::try_from(OffsetDateTime::from(time)).ok())
        .unwrap_or_default()
}

fn set_last_modified_time<R: Read>(zip_file: &ZipFile<'_, R>, path: &Path) -> crate::Result<()> {
    // Extract modification time from zip file and convert to FileTime
    let file_time = zip_file
        .last_modified()
        .and_then(|datetime| OffsetDateTime::try_from(datetime).ok())
        .map(|time| {
            // Zip does not support nanoseconds, so we can assume zero here
            FileTime::from_unix_time(time.unix_timestamp(), 0)
        });

    // Set the modification time if available
    if let Some(modification_time) = file_time {
        set_file_mtime(path, modification_time)?;
    }

    Ok(())
}

#[cfg(unix)]
fn unix_set_permissions<R: Read>(file_path: &Path, file: &ZipFile<'_, R>) -> crate::Result<()> {
    use std::fs::Permissions;

    if let Some(mode) = file.unix_mode() {
        fs::set_permissions(file_path, Permissions::from_mode(mode))?;
    }

    Ok(())
}
