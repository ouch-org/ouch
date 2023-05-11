//! SevenZip archive format compress function
use std::{
    env::current_dir,
    path::{Path, PathBuf},
};

pub fn compress_sevenz(files: Vec<PathBuf>, output_path: &Path) -> crate::Result<bool> {
    let mut writer = sevenz_rust::SevenZWriter::create(output_path).map_err(crate::Error::SevenzipError)?;

    for filep in files.iter() {
        writer
            .push_archive_entry::<std::fs::File>(
                sevenz_rust::SevenZWriter::<std::fs::File>::create_archive_entry(
                    filep,
                    filep
                        .strip_prefix(current_dir()?)
                        .expect("StripPrefix Failed")
                        .as_os_str()
                        .to_str()
                        .unwrap()
                        .to_string(),
                ),
                None,
            )
            .map_err(crate::Error::SevenzipError)?;
    }

    writer.finish()?;
    Ok(true)
}

pub fn decompress_sevenz(input_file_path: &Path, output_path: &Path) -> crate::Result<usize> {
    let mut count: usize = 0;
    sevenz_rust::decompress_file_with_extract_fn(input_file_path, output_path, |entry, reader, dest| {
        count += 1;
        sevenz_rust::default_entry_extract_fn(entry, reader, dest)
    })
    .map_err(crate::Error::SevenzipError)?;
    Ok(count)
}
