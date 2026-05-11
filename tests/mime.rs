#[macro_use]
mod utils;

use rand::{SeedableRng, rngs::SmallRng};
use tempfile::NamedTempFile;

use crate::utils::write_random_content;

#[test]
/// Makes sure that the files ouch produces are what they claim to be, checking their
/// types through MIME sniffing.
fn sanity_check_through_mime() {
    let temp_dir = tempfile::tempdir().expect("to build a temporary directory");
    let temp_dir_path = temp_dir.path();

    let test_file = &mut NamedTempFile::new_in(temp_dir_path).expect("to be able to build a temporary file");
    write_random_content(test_file, &mut SmallRng::from_os_rng());

    let formats = [
        ("7z", "application/x-7z-compressed"),
        ("cb7", "application/x-7z-compressed"),
        ("tar", "application/x-tar"),
        ("cbt", "application/x-tar"),
        ("zip", "application/zip"),
        ("cbz", "application/zip"),
        ("epub", "application/zip"),
        ("tar.gz", "application/gzip"),
        ("tgz", "application/gzip"),
        ("tbz", "application/x-bzip2"),
        ("tbz2", "application/x-bzip2"),
        ("txz", "application/x-xz"),
        ("tzst", "application/zstd"),
        ("tar.bz", "application/x-bzip2"),
        ("tar.bz2", "application/x-bzip2"),
        ("tar.xz", "application/x-xz"),
        ("tar.zst", "application/zstd"),
    ];

    for (format, expected_mime) in formats {
        let path_to_compress = test_file.path();

        let compressed_file_path = &format!("{}.{}", path_to_compress.display(), format);
        ouch!("c", path_to_compress, compressed_file_path);

        let sniffed = infer::get_from_path(compressed_file_path)
            .expect("the file to be read")
            .expect("the MIME to be found");

        assert_eq!(sniffed.mime_type(), expected_mime);
    }
}
