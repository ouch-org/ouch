use std::{
    env, fs,
    path::{Path, PathBuf},
};

use ouch::{cli::Command, commands::run, oof};

#[test]
fn test_compress_decompress_with_empty_dir() {
    // System temporary directory depends on the platform, for linux it's /tmp
    let system_tmp = env::temp_dir();

    // Create a temporary testing folder that will be deleted on scope drop
    let testing_dir =
        tempfile::Builder::new().prefix("ouch-testing").tempdir_in(system_tmp).expect("Could not create testing_dir");

    let testing_dir_path = testing_dir.path();

    let empty_dir_path: PathBuf = create_empty_dir(&testing_dir_path, "dummy_empty_dir_name");

    let mut file_paths: Vec<PathBuf> = vec![empty_dir_path];

    let format = "zip";

    let compressed_archive_path: PathBuf = compress_files(&testing_dir_path, &file_paths, &format);

    let mut extracted_paths = extract_files(&compressed_archive_path);

    // // DEBUG UTIL:
    // // Uncomment line below to freeze the code and see compressed and extracted files in
    // // the temporary directory before their auto-destruction.
    // std::thread::sleep(std::time::Duration::from_secs(10));

    // no need to sort a unitary value vector but i will keep this
    //  for retrocompatibility, for now.
    file_paths.sort();
    extracted_paths.sort();

    assert_correct_paths(&file_paths, &extracted_paths, format);
}

fn create_empty_dir(at: &Path, filename: &str) -> PathBuf {
    let dirname = Path::new(filename);
    let full_path = at.join(dirname);

    fs::create_dir(&full_path).expect("Failed to create an empty directory");

    full_path
}

fn compress_files(at: &Path, paths_to_compress: &[PathBuf], format: &str) -> PathBuf {
    let archive_path = String::from("archive.") + format;
    let archive_path = at.join(archive_path);

    let command = Command::Compress { files: paths_to_compress.to_vec(), output_path: archive_path.to_path_buf() };
    run(command, &oof::Flags::default()).expect("Failed to compress test dummy files");

    archive_path
}

fn extract_files(archive_path: &Path) -> Vec<PathBuf> {
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

    let command = Command::Decompress {
        files: vec![archive_path.to_owned()],
        output_folder: Some(extraction_output_folder.clone()),
    };
    run(command, &oof::Flags::default()).expect("Failed to extract");

    fs::read_dir(extraction_output_folder).unwrap().map(Result::unwrap).map(|entry| entry.path()).collect()
}

fn assert_correct_paths(original: &[PathBuf], extracted: &[PathBuf], format: &str) {
    assert_eq!(
        original.len(),
        extracted.len(),
        "Number of compressed files does not match number of decompressed when testing archive format '{:?}'.",
        format
    );
    for (original, extracted) in original.iter().zip(extracted) {
        assert_eq!(original.file_name(), extracted.file_name(), "");
    }
}
