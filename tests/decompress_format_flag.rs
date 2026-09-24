mod utils;

use fs_err as fs;

// a misplaced archive in --format must be rejected not panic
#[test]
fn format_flag_rejects_misplaced_archive() {
    let (_tempdir, dir) = crate::utils::testdir().unwrap();

    fs::write(dir.join("f.txt"), "hello").unwrap();
    crate::utils::cargo_bin()
        .current_dir(dir)
        .args(["compress", "f.txt", "f.txt.gz"])
        .assert()
        .success();
    // remove the source so decompression reaches the format handling not a conflict prompt
    fs::remove_file(dir.join("f.txt")).unwrap();

    crate::utils::cargo_bin()
        .current_dir(dir)
        .args(["decompress", "f.txt.gz", "--format", "gz.tar"])
        .assert()
        .failure()
        .code(1);
}
