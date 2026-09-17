mod utils;

use fs_err as fs;

// unpacking one file asks before overwriting a file that already exists
#[test]
fn single_file_asks_before_overwriting_existing_file() {
    let (_tempdir, dir) = crate::utils::testdir().unwrap();

    fs::write(dir.join("report.txt"), "new").unwrap();
    crate::utils::cargo_bin()
        .current_dir(dir)
        .args(["compress", "report.txt", "report.txt.gz"])
        .assert()
        .success();
    fs::remove_file(dir.join("report.txt")).unwrap();

    let folder = dir.join("report");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("report.txt"), "old").unwrap();

    // merge the folder then skip the file so the old content is kept
    crate::utils::cargo_bin()
        .current_dir(dir)
        .arg("decompress")
        .arg("report.txt.gz")
        .write_stdin("m\ns\n")
        .assert()
        .success();
    assert_eq!("old", fs::read_to_string(folder.join("report.txt")).unwrap());

    // merge the folder then overwrite the file so the new content wins
    crate::utils::cargo_bin()
        .current_dir(dir)
        .arg("decompress")
        .arg("report.txt.gz")
        .write_stdin("m\no\n")
        .assert()
        .success();
    assert_eq!("new", fs::read_to_string(folder.join("report.txt")).unwrap());
}
