#[macro_use]
mod utils;

use rand::{rngs::SmallRng, SeedableRng};
use tempfile::NamedTempFile;

use crate::utils::write_random_content;

#[test]
/// Makes sure that the files ouch produces are what they claim to be, checking their
/// types through MIME sniffing.
fn sanity_check_through_mime() {
    let temp_dir = tempfile::tempdir().expect("to build a temporary directory");
    let temp_dir_path = temp_dir.path();

    let test_file = &mut NamedTempFile::new_in(temp_dir_path).expect("to be able to build a temporary file");
    write_random_content(test_file, &mut SmallRng::from_entropy());

    let formats = [
        "tar", "zip", "tar.gz", "tgz", "tbz", "tbz2", "txz", "tlz", "tlzma", "tzst", "tar.bz", "tar.bz2", "tar.lzma",
        "tar.xz", "tar.zst",
    ];

    let expected_mimes = [
        "application/x-tar",
        "application/zip",
        "application/gzip",
        "application/gzip",
        "application/x-bzip2",
        "application/x-bzip2",
        "application/x-xz",
        "application/x-xz",
        "application/x-xz",
        "application/zstd",
        "application/x-bzip2",
        "application/x-bzip2",
        "application/x-xz",
        "application/x-xz",
        "application/zstd",
    ];

    assert_eq!(formats.len(), expected_mimes.len());

    for (format, expected_mime) in formats.iter().zip(expected_mimes.iter()) {
        let path_to_compress = test_file.path();

        let compressed_file_path = &format!("{}.{}", path_to_compress.display(), format);
        ouch!("c", path_to_compress, compressed_file_path);

        let sniffed =
            infer::get_from_path(compressed_file_path).expect("the file to be read").expect("the MIME to be found");

        assert_eq!(&sniffed.mime_type(), expected_mime);
    }
}
