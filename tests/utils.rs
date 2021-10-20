//! Files in common between one or more integration tests

#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
};

use ouch::{commands::run, Opts, Subcommand};

pub fn create_empty_dir(at: &Path, filename: &str) -> PathBuf {
    let dirname = Path::new(filename);
    let full_path = at.join(dirname);

    fs::create_dir(&full_path).expect("Failed to create an empty directory");

    full_path
}

pub fn compress_files(at: &Path, paths_to_compress: &[PathBuf], format: &str) -> PathBuf {
    let archive_path = String::from("archive.") + format;
    let archive_path = at.join(archive_path);

    let command = Opts {
        yes: false,
        no: false,
        cmd: Subcommand::Compress { files: paths_to_compress.to_vec(), output: archive_path.clone() },
    };
    run(command, None).expect("Failed to compress test dummy files");

    archive_path
}

pub fn extract_files(archive_path: &Path) -> Vec<PathBuf> {
    // We will extract in the same folder as the archive
    // If the archive is at:
    //   /tmp/ouch-testing-tar.Rbq4DusBrtF8/archive.tar
    // Then the extraction_output_folder will be:
    //   /tmp/ouch-testing-tar.Rbq4DusBrtF8/extraction_results/
    let mut extraction_output_folder = archive_path.to_path_buf();
    // Remove the name of the extracted archive
    assert!(extraction_output_folder.pop());
    // Add the suffix "results"
    extraction_output_folder.push("extraction_results");

    let command = Opts {
        yes: false,
        no: false,
        cmd: Subcommand::Decompress {
            files: vec![archive_path.to_owned()],
            output: Some(extraction_output_folder.clone()),
        },
    };
    run(command, None).expect("Failed to extract");

    fs::read_dir(extraction_output_folder).unwrap().map(Result::unwrap).map(|entry| entry.path()).collect()
}

pub fn assert_correct_paths(original: &[PathBuf], extracted: &[PathBuf], format: &str) {
    assert_eq!(
        original.len(),
        extracted.len(),
        "Number of compressed files does not match number of decompressed when testing archive format '{:?}'.",
        format
    );
    for (original, extracted) in original.iter().zip(extracted) {
        assert_eq!(original.file_name(), extracted.file_name());
    }
}
