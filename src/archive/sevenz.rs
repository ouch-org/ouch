//! SevenZip archive format compress function
use std::{
    env,
    fs::File,
    io::{Read, Seek, Write},
    path::{Path, PathBuf},
};

use same_file::Handle;

use crate::{
    info,
    utils::{self, cd_into_same_dir_as, Bytes, EscapedPathDisplay, FileVisibilityPolicy},
    warning,
};

pub fn compress_sevenz<W>(
    files: &[PathBuf],
    output_path: &Path,
    writer: W,
    file_visibility_policy: FileVisibilityPolicy,
    quiet: bool,
) -> crate::Result<W>
where
    W: Write + Seek,
{
    let mut writer = sevenz_rust::SevenZWriter::new(writer).map_err(crate::Error::SevenzipError)?;
    let output_handle = Handle::from_path(output_path);
    for filename in files {
        let previous_location = cd_into_same_dir_as(filename)?;

        // Safe unwrap, input shall be treated before
        let filename = filename.file_name().unwrap();

        for entry in file_visibility_policy.build_walker(filename) {
            let entry = entry?;
            let path = entry.path();

            // If the output_path is the same as the input file, warn the user and skip the input (in order to avoid compression recursion)
            if let Ok(ref handle) = output_handle {
                if matches!(Handle::from_path(path), Ok(x) if &x == handle) {
                    warning!(
                        "The output file and the input file are the same: `{}`, skipping...",
                        output_path.display()
                    );
                    continue;
                }
            }

            // This is printed for every file in `input_filenames` and has
            // little importance for most users, but would generate lots of
            // spoken text for users using screen readers, braille displays
            // and so on
            if !quiet {
                info!(inaccessible, "Compressing '{}'.", EscapedPathDisplay::new(path));
            }

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

            if metadata.is_dir() {
                writer
                    .push_archive_entry::<std::fs::File>(
                        sevenz_rust::SevenZArchiveEntry::from_path(path, path.to_str().unwrap().to_owned()),
                        None,
                    )
                    .map_err(crate::Error::SevenzipError)?;
            } else {
                let reader = File::open(path)?;
                writer
                    .push_archive_entry::<std::fs::File>(
                        sevenz_rust::SevenZArchiveEntry::from_path(path, path.to_str().unwrap().to_owned()),
                        Some(reader),
                    )
                    .map_err(crate::Error::SevenzipError)?;
            }
        }

        env::set_current_dir(previous_location)?;
    }

    let bytes = writer.finish()?;
    Ok(bytes)
}

pub fn decompress_sevenz<R>(reader: R, output_path: &Path, quiet: bool) -> crate::Result<usize>
where
    R: Read + Seek,
{
    let mut count: usize = 0;
    sevenz_rust::decompress_with_extract_fn(reader, output_path, |entry, reader, dest| {
        count += 1;
        // Manually handle writing all files from 7z archive, due to library exluding empty files
        use std::io::BufWriter;

        use filetime_creation as ft;

        let file_path = output_path.join(entry.name());

        if entry.is_directory() {
            // This is printed for every file in the archive and has little
            // importance for most users, but would generate lots of
            // spoken text for users using screen readers, braille displays
            // and so on
            if !quiet {
                info!(
                    inaccessible,
                    "File {} extracted to \"{}\"",
                    entry.name(),
                    file_path.display()
                );
            }
            let dir = dest;
            if !dir.exists() {
                std::fs::create_dir_all(dir)?;
            }
        } else {
            // same reason is in _is_dir: long, often not needed text
            if !quiet {
                info!(
                    inaccessible,
                    "{:?} extracted. ({})",
                    file_path.display(),
                    Bytes::new(entry.size()),
                );
            }
            let path = dest;
            path.parent().and_then(|p| {
                if !p.exists() {
                    std::fs::create_dir_all(p).ok()
                } else {
                    None
                }
            });
            let file = File::create(path)?;
            let mut writer = BufWriter::new(file);
            std::io::copy(reader, &mut writer)?;
            ft::set_file_handle_times(
                writer.get_ref(),
                Some(ft::FileTime::from_system_time(entry.access_date().into())),
                Some(ft::FileTime::from_system_time(entry.last_modified_date().into())),
                Some(ft::FileTime::from_system_time(entry.creation_date().into())),
            )
            .unwrap_or_default();
        }
        Ok(true)
    })
    .map_err(crate::Error::SevenzipError)?;
    Ok(count)
}
