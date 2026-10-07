mod utils;

use std::io::Write;

use fs_err as fs;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

fn custom_extension_zip(dir: &std::path::Path) -> Vec<u8> {
    let file = fs::File::create(dir.join("package.sublime-package")).unwrap();
    let mut writer = ZipWriter::new(file);
    writer
        .start_file(
            "note.txt",
            SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
        )
        .unwrap();
    writer.write_all(b"synthetic archive contents").unwrap();
    writer.finish().unwrap();
    fs::read(dir.join("package.sublime-package")).unwrap()
}

#[test]
fn list_custom_extension_asks_to_list_and_does_not_extract() {
    let (_tempdir, dir) = utils::testdir().unwrap();
    let archive = custom_extension_zip(dir);
    let output = utils::cargo_bin()
        .current_dir(dir)
        .args(["list", "package.sublime-package", "--tree"])
        .write_stdin("y\n")
        .assert()
        .success()
        .get_output()
        .clone();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Do you want to list '"), "{stderr}");
    assert!(!stderr.contains("Do you want to decompress '"), "{stderr}");
    assert!(String::from_utf8(output.stdout).unwrap().contains("note.txt"));
    assert_eq!(fs::read(dir.join("package.sublime-package")).unwrap(), archive);
    assert_eq!(fs::read_dir(dir).unwrap().count(), 1);
}

#[test]
fn custom_extension_confirmation_policies_preserve_listing_and_extraction() {
    for operation in ["list", "decompress"] {
        for (flags, answer, proceed, prompt, format_warning) in [
            (vec![], "y\n", true, true, true),
            (vec![], "n\n", false, true, true),
            (vec!["--yes"], "", true, false, true),
            (vec!["--no"], "", false, false, true),
            (vec!["--format", "zip"], "", true, false, false),
            (vec!["--quiet"], "y\n", true, true, false),
        ] {
            let (_tempdir, dir) = utils::testdir().unwrap();
            let archive = custom_extension_zip(dir);
            let mut command = utils::cargo_bin();
            command
                .current_dir(dir)
                .args([operation, "package.sublime-package"])
                .args(&flags);
            if operation == "list" {
                command.arg("--tree");
            } else {
                command.args(["--dir", "output"]);
            }
            let output = command.write_stdin(answer).assert().success().get_output().clone();
            let stderr = String::from_utf8(output.stderr).unwrap();
            let stdout = String::from_utf8(output.stdout).unwrap();
            let expected = format!("Do you want to {operation} '");
            assert_eq!(stderr.contains(&expected), prompt, "{operation} {flags:?}: {stderr}");
            assert_eq!(
                stderr.contains("[WARNING] No recognized extensions in "),
                format_warning,
                "{operation} {flags:?}: {stderr}"
            );
            if operation == "list" {
                assert!(!stderr.contains("Do you want to decompress '"), "{stderr}");
                assert_eq!(stdout.contains("note.txt"), proceed, "{stdout}");
                assert_eq!(fs::read_dir(dir).unwrap().count(), 1);
            } else if proceed {
                assert_eq!(
                    fs::read(dir.join("output/note.txt")).unwrap(),
                    b"synthetic archive contents"
                );
                assert_eq!(fs::read_dir(dir).unwrap().count(), 2);
            } else {
                assert_eq!(fs::read_dir(dir).unwrap().count(), 1);
            }
            assert_eq!(fs::read(dir.join("package.sublime-package")).unwrap(), archive);
        }
    }
}

#[test]
fn layered_archive_listing_confirmation_uses_list() {
    for format in ["zip", "7z"] {
        for answer in ["y\n", "n\n"] {
            let (_tempdir, dir) = utils::testdir().unwrap();
            let zip = custom_extension_zip(dir);
            let archive = if format == "zip" {
                zip
            } else {
                fs::write(dir.join("note.txt"), b"synthetic archive contents").unwrap();
                utils::cargo_bin()
                    .current_dir(dir)
                    .args(["compress", "note.txt", "archive.7z", "--yes"])
                    .assert()
                    .success();
                let bytes = fs::read(dir.join("archive.7z")).unwrap();
                fs::remove_file(dir.join("archive.7z")).unwrap();
                fs::remove_file(dir.join("note.txt")).unwrap();
                bytes
            };
            fs::remove_file(dir.join("package.sublime-package")).unwrap();
            let name = format!("archive.{format}.gz");
            let file = fs::File::create(dir.join(&name)).unwrap();
            let mut encoder = flate2::write::GzEncoder::new(file, flate2::Compression::default());
            encoder.write_all(&archive).unwrap();
            encoder.finish().unwrap();
            let original = fs::read(dir.join(&name)).unwrap();
            let output = utils::cargo_bin()
                .current_dir(dir)
                .args(["list", &name, "--tree"])
                .write_stdin(answer)
                .assert()
                .success()
                .get_output()
                .clone();
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(stderr.contains("[WARNING]"), "{stderr}");
            assert!(stderr.contains("Do you want to list '"), "{stderr}");
            assert!(!stderr.contains("Do you want to decompress '"), "{stderr}");
            assert_eq!(
                String::from_utf8(output.stdout).unwrap().contains("note.txt"),
                answer == "y\n"
            );
            assert_eq!(fs::read(dir.join(&name)).unwrap(), original);
            assert_eq!(fs::read_dir(dir).unwrap().count(), 1);
        }
    }
}
