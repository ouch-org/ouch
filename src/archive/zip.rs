//! Contains Zip-specific building and unpacking functions

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    env,
    fs::File,
    io::{self, prelude::*},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use filetime::{set_file_mtime, FileTime};
use fs_err as fs;
use humansize::{format_size, DECIMAL};
use zip::{self, read::ZipFile, DateTime, ZipArchive};

use crate::{
    error::FinalError,
    info,
    list::FileInArchive,
    progress::OutputLine,
    utils::{
        self, cd_into_same_dir_as, get_invalid_utf8_paths, pretty_format_list_of_paths, strip_cur_dir, to_utf,
        FileVisibilityPolicy,
    },
};

/// Unpacks the archive given by `archive` into the folder given by `output_folder`.
/// Assumes that output_folder is empty
pub fn unpack_archive<R, D>(
    mut archive: ZipArchive<R>,
    output_folder: &Path,
    mut log_out: D,
) -> crate::Result<Vec<PathBuf>>
where
    R: Read + Seek,
    D: OutputLine,
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

        match file.name().ends_with('/') {
            _is_dir @ true => {
                // This is printed for every file in the archive and has little
                // importance for most users, but would generate lots of
                // spoken text for users using screen readers, braille displays
                // and so on
                info!(@log_out, inaccessible, "File {} extracted to \"{}\"", idx, file_path.display());
                fs::create_dir_all(&file_path)?;
            }
            _is_file @ false => {
                if let Some(path) = file_path.parent() {
                    if !path.exists() {
                        fs::create_dir_all(path)?;
                    }
                }
                let file_path = strip_cur_dir(file_path.as_path());

                // same reason is in _is_dir: long, often not needed text
                info!(
                    @log_out,
                    inaccessible,
                    "{:?} extracted. ({})",
                    file_path.display(),
                    format_size(file.size(), DECIMAL),
                );

                let mut output_file = fs::File::create(file_path)?;
                io::copy(&mut file, &mut output_file)?;

                set_last_modified_time(&file, file_path)?;
            }
        }

        #[cfg(unix)]
        unix_set_permissions(&file_path, &file)?;

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
    mut log_out: D,
) -> crate::Result<W>
where
    W: Write + Seek,
    D: OutputLine,
{
    let mut writer = zip::ZipWriter::new(writer);
    let options = zip::write::FileOptions::default();

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

        // Safe unwrap, input shall be treated before
        let filename = filename.file_name().unwrap();

        for entry in file_visibility_policy.build_walker(filename) {
            let entry = entry?;
            let path = entry.path();

            // This is printed for every file in `input_filenames` and has
            // little importance for most users, but would generate lots of
            // spoken text for users using screen readers, braille displays
            // and so on
            info!(@log_out, inaccessible, "Compressing '{}'.", to_utf(path));

            let metadata = match path.metadata() {
                Ok(metadata) => metadata,
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::NotFound && utils::is_symlink(path) {
                        // This path is for a broken symlink
                        // We just ignore it
                        continue;
                    }
                    return Err(e.into());
                }
            };

            #[cfg(unix)]
            let options = options.unix_permissions(metadata.permissions().mode());

            if metadata.is_dir() {
                writer.add_directory(path.to_str().unwrap().to_owned(), options)?;
            } else {
                #[cfg(not(unix))]
                let options = if is_executable::is_executable(path) {
                    executable
                } else {
                    options
                };

                let mut file = File::open(entry.path())?;
                writer.start_file(
                    path.to_str().unwrap(),
                    options.last_modified_time(get_last_modified_time(&file)),
                )?;
                io::copy(&mut file, &mut writer)?;
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

fn get_last_modified_time(file: &File) -> DateTime {
    file.metadata()
        .and_then(|metadata| metadata.modified())
        .map_err(|_| ())
        .and_then(|time| DateTime::from_time(time.into()))
        .unwrap_or_default()
}

fn set_last_modified_time(zip_file: &ZipFile, path: &Path) -> crate::Result<()> {
    let modification_time_in_seconds = zip_file
        .last_modified()
        .to_time()
        .expect("Zip archive contains a file with broken 'last modified time'")
        .unix_timestamp();

    // Zip does not support nanoseconds, so we can assume zero here
    let modification_time = FileTime::from_unix_time(modification_time_in_seconds, 0);

    set_file_mtime(path, modification_time)?;

    Ok(())
}

#[cfg(unix)]
fn unix_set_permissions(file_path: &Path, file: &ZipFile) -> crate::Result<()> {
    use std::fs::Permissions;

    if let Some(mode) = file.unix_mode() {
        fs::set_permissions(file_path, Permissions::from_mode(mode))?;
    }

    Ok(())
}
