mod utils;

use std::{env, path::PathBuf};

use utils::*;

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
