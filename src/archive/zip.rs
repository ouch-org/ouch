//! Contains Zip-specific building and unpacking functions

use std::{
    env,
    io::{self, prelude::*},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use fs_err as fs;
use zip::{self, read::ZipFile, ZipArchive};

use crate::{
    error::FinalError,
    info,
    list::FileInArchive,
    utils::{
        self, cd_into_same_dir_as, concatenate_os_str_list, get_invalid_utf8_paths, strip_cur_dir, to_utf, Bytes,
        FileVisibilityPolicy,
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

    let mut unpacked_files = Vec::with_capacity(archive.len());

    for idx in 0..archive.len() {
        let mut file = archive.by_index(idx)?;
        let file_path = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let file_path = output_folder.join(file_path);

        display_zip_comment_if_exists(&file);

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

                #[cfg(unix)]
                set_last_modified_time(&output_file, &file)?;
            }
        }

        #[cfg(unix)]
        __unix_set_permissions(&file_path, &file)?;

        unpacked_files.push(file_path);
    }

    Ok(unpacked_files)
}

/// List contents of `archive`, returning a vector of archive entries
pub fn list_archive<R>(mut archive: ZipArchive<R>) -> impl Iterator<Item = crate::Result<FileInArchive>>
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

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for idx in 0..archive.len() {
            let maybe_file_in_archive = (|| {
                let file = match archive.by_index(idx) {
                    Ok(f) => f,
                    Err(e) => return Some(Err(e.into())),
                };

                let path = match file.enclosed_name() {
                    Some(path) => path.to_owned(),
                    None => return None,
                };
                let is_dir = file.is_dir();

                Some(Ok(FileInArchive { path, is_dir }))
            })();
            if let Some(file_in_archive) = maybe_file_in_archive {
                tx.send(file_in_archive).unwrap();
            }
        }
    });

    Files(rx)
}

/// Compresses the archives given by `input_filenames` into the file given previously to `writer`.
pub fn build_archive_from_paths<W, D>(
    input_filenames: &[PathBuf],
    writer: W,
    file_visibility_policy: FileVisibilityPolicy,
    mut display_handle: D,
) -> crate::Result<W>
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
            .detail(format!(
                "Files with invalid paths: {}",
                concatenate_os_str_list(&invalid_unicode_filenames)
            ));

        return Err(error.into());
    }

    for filename in input_filenames {
        let previous_location = cd_into_same_dir_as(filename)?;

        // Safe unwrap, input shall be treated before
        let filename = filename.file_name().unwrap();

        for entry in file_visibility_policy.build_walker(filename) {
            let entry = entry?;
            let path = entry.path();

            // This is printed for every file in `input_filenames` and has
            // little importance for most users, but would generate lots of
            // spoken text for users using screen readers, braille displays
            // and so on
            info!(@display_handle, inaccessible, "Compressing '{}'.", to_utf(path));

            if path.is_dir() {
                writer.add_directory(path.to_str().unwrap().to_owned(), options)?;
            } else {
                writer.start_file(path.to_str().unwrap().to_owned(), options)?;
                let file_bytes = match fs::read(entry.path()) {
                    Ok(b) => b,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::NotFound && utils::is_symlink(path) {
                            // This path is for a broken symlink
                            // We just ignore it
                            continue;
                        }
                        return Err(e.into());
                    }
                };
                writer.write_all(&*file_bytes)?;
            }
        }

        env::set_current_dir(previous_location)?;
    }

    let bytes = writer.finish()?;
    Ok(bytes)
}

fn display_zip_comment_if_exists(file: &ZipFile) {
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
/// Attempts to convert a [`zip::DateTime`] to a [`libc::timespec`].
fn convert_zip_date_time(date_time: zip::DateTime) -> Option<libc::timespec> {
    use time::{Date, Month, PrimitiveDateTime, Time};

    // Safety: time::Month is repr(u8) and goes from 1 to 12
    let month: Month = unsafe { std::mem::transmute(date_time.month()) };

    let date = Date::from_calendar_date(date_time.year() as _, month, date_time.day()).ok()?;

    let time = Time::from_hms(date_time.hour(), date_time.minute(), date_time.second()).ok()?;

    let date_time = PrimitiveDateTime::new(date, time);
    let timestamp = date_time.assume_utc().unix_timestamp();

    Some(libc::timespec {
        tv_sec: timestamp,
        tv_nsec: 0,
    })
}

#[cfg(unix)]
fn set_last_modified_time(file: &fs::File, zip_file: &ZipFile) -> crate::Result<()> {
    use std::os::unix::prelude::AsRawFd;

    use libc::UTIME_NOW;

    let now = libc::timespec {
        tv_sec: 0,
        tv_nsec: UTIME_NOW,
    };

    let last_modified = zip_file.last_modified();
    let last_modified = convert_zip_date_time(last_modified).unwrap_or(now);

    // The first value is the last accessed time, which we'll set as being right now.
    // The second value is the last modified time, which we'll copy over from the zip archive
    let times = [now, last_modified];

    let output_fd = file.as_raw_fd();

    // TODO: check for -1
    unsafe { libc::futimens(output_fd, &times as *const _) };

    Ok(())
}

#[cfg(unix)]
fn __unix_set_permissions(file_path: &Path, file: &ZipFile) -> crate::Result<()> {
    use std::{fs::Permissions, os::unix::fs::PermissionsExt};

    if let Some(mode) = file.unix_mode() {
        fs::set_permissions(file_path, Permissions::from_mode(mode))?;
    }

    Ok(())
}
