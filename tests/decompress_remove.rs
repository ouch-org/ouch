mod utils;

use fs_err as fs;

// --remove keeps the input when an entry is skipped and removes it only after a full unpack
#[test]
fn remove_keeps_input_when_extraction_is_incomplete() {
    for ext in ["tar", "zip", "7z"] {
        let (_tempdir, dir) = crate::utils::testdir().unwrap();
        let source = dir.join("src");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("data.txt"), "from archive").unwrap();

        let archive = dir.join(format!("skip.{ext}"));
        crate::utils::cargo_bin()
            .args(["compress", source.join("data.txt").to_str().unwrap()])
            .arg(&archive)
            .assert()
            .success();

        let out = dir.join("out");
        fs::create_dir(&out).unwrap();
        fs::write(out.join("data.txt"), "original").unwrap();

        crate::utils::cargo_bin()
            .arg("decompress")
            .arg(&archive)
            .arg("-d")
            .arg(&out)
            .arg("--remove")
            .write_stdin("m\ns\n")
            .assert()
            .success();

        assert!(archive.exists(), "archive kept when an entry was skipped");
        assert_eq!("original", fs::read_to_string(out.join("data.txt")).unwrap());

        let full = dir.join(format!("full.{ext}"));
        crate::utils::cargo_bin()
            .args(["compress", source.join("data.txt").to_str().unwrap()])
            .arg(&full)
            .assert()
            .success();

        let outf = dir.join("outf");
        crate::utils::cargo_bin()
            .arg("decompress")
            .arg(&full)
            .arg("-d")
            .arg(&outf)
            .arg("--remove")
            .arg("--yes")
            .assert()
            .success();

        assert!(!full.exists(), "archive removed after a full unpack");
    }
}

// --remove keeps the input when an entry is refused as unsafe not just when skipped for a conflict
#[test]
fn remove_keeps_input_when_an_entry_is_unsafe() {
    let (_tempdir, dir) = crate::utils::testdir().unwrap();

    // the first entry name escapes the output directory so it is refused during unpacking
    let archive = dir.join("evil.zip");
    {
        use std::io::Write;

        use zip::write::SimpleFileOptions;
        let file = std::fs::File::create(&archive).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("../escape.txt", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"payload").unwrap();
        zip.start_file("safe.txt", SimpleFileOptions::default()).unwrap();
        zip.write_all(b"safe").unwrap();
        zip.finish().unwrap();
    }

    let out = dir.join("out");
    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(&archive)
        .arg("-d")
        .arg(&out)
        .arg("--remove")
        .arg("--yes")
        .assert()
        .success();

    assert!(archive.exists(), "archive kept when an unsafe entry was refused");
    assert!(!out.join("escape.txt").exists(), "unsafe entry was not extracted");
}
