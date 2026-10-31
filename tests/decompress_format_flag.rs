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

// with --format the output keeps the input name, so --dir . decompresses onto the input: must fail and leave it intact
#[test]
fn decompress_onto_input_file_is_rejected() {
    let (_tempdir, dir) = crate::utils::testdir().unwrap();

    fs::write(dir.join("f.txt"), "hello").unwrap();
    crate::utils::cargo_bin()
        .current_dir(dir)
        .args(["compress", "f.txt", "f.gz"])
        .assert()
        .success();
    fs::rename(dir.join("f.gz"), dir.join("data")).unwrap();
    let compressed = fs::read(dir.join("data")).unwrap();

    let output = crate::utils::cargo_bin()
        .current_dir(dir)
        .args([
            "decompress",
            "--yes",
            "--remove",
            "--dir",
            ".",
            "--format",
            "gz",
            "data",
        ])
        .assert()
        .failure()
        .code(1)
        .get_output()
        .clone();
    assert!(String::from_utf8_lossy(&output.stderr).contains("onto itself"));
    assert_eq!(fs::read(dir.join("data")).unwrap(), compressed);
}
