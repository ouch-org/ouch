#[macro_use]
mod utils;

use std::{iter::once, path::PathBuf};

use fs_err as fs;
use parse_display::Display;
use proptest::sample::size_range;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use tempfile::tempdir;
use test_strategy::{proptest, Arbitrary};

use crate::utils::{assert_same_directory, write_random_content};

/// tar and zip extensions
#[derive(Arbitrary, Debug, Display)]
#[display(style = "lowercase")]
enum DirectoryExtension {
    #[display("7z")]
    SevenZ,
    Tar,
    Tbz,
    Tbz2,
    Tbz3,
    Tgz,
    Tlz4,
    Tlzma,
    Tsz,
    Txz,
    Tzst,
    Zip,
}

/// Extensions of single file compression formats
#[derive(Arbitrary, Debug, Display)]
#[display(style = "lowercase")]
enum FileExtension {
    Bz,
    Bz2,
    Bz3,
    Gz,
    Lz4,
    Lzma,
    Sz,
    Xz,
    Zst,
    Br,
}

#[derive(Arbitrary, Debug, Display)]
#[display("{0}")]
enum Extension {
    Directory(DirectoryExtension),
    File(FileExtension),
}

/// Converts a list of extension structs to string
fn merge_extensions(ext: impl ToString, exts: Vec<FileExtension>) -> String {
    once(ext.to_string())
        .chain(exts.into_iter().map(|x| x.to_string()))
        .collect::<Vec<_>>()
        .join(".")
}

/// Create random nested directories and files under the specified directory
fn create_random_files(dir: impl Into<PathBuf>, depth: u8, rng: &mut SmallRng) {
    if depth == 0 {
        return;
    }

    let dir = &dir.into();

    // create 0 to 4 random files
    for _ in 0..rng.gen_range(0..=4u32) {
        write_random_content(
            &mut tempfile::Builder::new().tempfile_in(dir).unwrap().keep().unwrap().0,
            rng,
        );
    }

    // create more random files in 0 to 2 new directories
    for _ in 0..rng.gen_range(0..=2u32) {
        create_random_files(tempfile::tempdir_in(dir).unwrap().into_path(), depth - 1, rng);
    }
}

/// Compress and decompress a single empty file
#[proptest(cases = 200)]
fn single_empty_file(ext: Extension, #[any(size_range(0..8).lift())] exts: Vec<FileExtension>) {
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let before = &dir.join("before");
    fs::create_dir(before).unwrap();
    let before_file = &before.join("file");
    let archive = &dir.join(format!("file.{}", merge_extensions(ext, exts)));
    let after = &dir.join("after");
    fs::write(before_file, []).unwrap();
    ouch!("-A", "c", before_file, archive);
    ouch!("-A", "d", archive, "-d", after);
    assert_same_directory(before, after, false);
}

/// Compress and decompress a single file
#[proptest(cases = 150)]
fn single_file(
    ext: Extension,
    #[any(size_range(0..6).lift())] exts: Vec<FileExtension>,
    // Use faster --level for slower CI targets
    #[cfg_attr(not(any(target_arch = "arm", target_abi = "eabihf")), strategy(proptest::option::of(0i16..12)))]
    #[cfg_attr(target_arch = "arm", strategy(proptest::option::of(0i16..6)))]
    level: Option<i16>,
) {
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let before = &dir.join("before");
    fs::create_dir(before).unwrap();
    let before_file = &before.join("file");
    let archive = &dir.join(format!("file.{}", merge_extensions(ext, exts)));
    let after = &dir.join("after");
    write_random_content(
        &mut fs::File::create(before_file).unwrap(),
        &mut SmallRng::from_entropy(),
    );
    if let Some(level) = level {
        ouch!("-A", "c", "-l", level.to_string(), before_file, archive);
    } else {
        ouch!("-A", "c", before_file, archive);
    }
    ouch!("-A", "d", archive, "-d", after);
    assert_same_directory(before, after, false);
}

/// Compress and decompress a single file over stdin.
#[proptest(cases = 200)]
fn single_file_stdin(
    ext: Extension,
    #[any(size_range(0..8).lift())] exts: Vec<FileExtension>,
    // Use faster --level for slower CI targets
    #[cfg_attr(not(any(target_arch = "arm", target_abi = "eabihf")), strategy(proptest::option::of(0i16..12)))]
    #[cfg_attr(target_arch = "arm", strategy(proptest::option::of(0i16..6)))]
    level: Option<i16>,
) {
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let before = &dir.join("before");
    fs::create_dir(before).unwrap();
    let before_file = &before.join("file");
    let format = merge_extensions(&ext, exts);
    let archive = &dir.join(format!("file.{}", format));
    let after = &dir.join("after");
    write_random_content(
        &mut fs::File::create(before_file).unwrap(),
        &mut SmallRng::from_entropy(),
    );
    if let Some(level) = level {
        ouch!("-A", "c", "-l", level.to_string(), before_file, archive);
    } else {
        ouch!("-A", "c", before_file, archive);
    }
    crate::utils::cargo_bin()
        .args(["-A", "-y", "d", "-", "-d", after.to_str().unwrap(), "--format", &format])
        .pipe_stdin(archive)
        .unwrap()
        .assert()
        .success();

    match ext {
        Extension::Directory(_) => {}
        // We don't know the original filename, so we create a file named stdin-output
        // Change the top-level "before" directory to match
        Extension::File(_) => fs::rename(before_file, before_file.with_file_name("stdin-output")).unwrap(),
    };

    assert_same_directory(before, after, false);
}

/// Compress and decompress a directory with random content generated with `create_random_files`
#[proptest(cases = 25)]
fn multiple_files(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
    #[strategy(0u8..3)] depth: u8,
) {
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let before = &dir.join("before");
    let before_dir = &before.join("dir");
    fs::create_dir_all(before_dir).unwrap();
    let archive = &dir.join(format!("archive.{}", merge_extensions(&ext, extra_extensions)));
    let after = &dir.join("after");
    create_random_files(before_dir, depth, &mut SmallRng::from_entropy());
    ouch!("-A", "c", before_dir, archive);
    ouch!("-A", "d", archive, "-d", after);
    assert_same_directory(before, after, !matches!(ext, DirectoryExtension::Zip));
}

#[proptest(cases = 25)]
fn multiple_files_with_conflict_and_choce_to_overwrite(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
    #[strategy(0u8..3)] depth: u8,
) {
    let dir = tempdir().unwrap();
    let dir = dir.path();

    let before = &dir.join("before");
    let before_dir = &before.join("dir");
    fs::create_dir_all(before_dir).unwrap();
    create_random_files(before_dir, depth, &mut SmallRng::from_entropy());
    
    let after = &dir.join("after");
    let after_dir = &after.join("dir");
    fs::create_dir_all(after_dir).unwrap();
    create_random_files(after_dir, depth, &mut SmallRng::from_entropy());
    
    let archive = &dir.join(format!("archive.{}", merge_extensions(&ext, extra_extensions)));
    ouch!("-A", "c", before_dir, archive);

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(after)
        .arg("--yes")
        //.write_stdin("y")
        .assert()
        .success();

    assert_same_directory(before, after, false);
}

#[proptest(cases = 25)]
fn multiple_files_with_conflict_and_choce_to_not_overwrite(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
    #[strategy(0u8..3)] depth: u8,
) {
    let dir = tempdir().unwrap();
    let dir = dir.path();

    let before = &dir.join("before");
    let before_dir = &before.join("dir");
    fs::create_dir_all(before_dir).unwrap();
    create_random_files(before_dir, depth, &mut SmallRng::from_entropy());
    
    let after = &dir.join("after");
    let after_dir = &after.join("dir");
    fs::create_dir_all(after_dir).unwrap();
    
    let after_backup = &dir.join("after_backup");
    let after_backup_dir = &after_backup.join("dir");
    fs::create_dir_all(after_backup_dir).unwrap();

    // Create a file with the same name as one of the files in the after directory
    fs::write(after_dir.join("something.txt"), "Some content").unwrap();
    fs::copy(after_dir.join("something.txt"), after_backup_dir.join("something.txt")).unwrap();
    
    let archive = &dir.join(format!("archive.{}", merge_extensions(&ext, extra_extensions)));
    ouch!("-A", "c", before_dir, archive);

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(after)
        .arg("--no")
        .assert()
        .success();

    assert_same_directory(after, after_backup, false);
}

#[cfg(feature = "allow_piped_choice")]
#[proptest(cases = 25)]
fn multiple_files_with_conflict_and_choce_to_rename(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
    #[strategy(0u8..3)] depth: u8,
) {
    let dir = tempdir().unwrap();
    let dir = dir.path();

    let before = &dir.join("before");
    let before_dir = &before.join("dir");
    fs::create_dir_all(before_dir).unwrap();
    create_random_files(before_dir, depth, &mut SmallRng::from_entropy());
    
    let after = &dir.join("after");
    let after_dir = &after.join("dir");
    fs::create_dir_all(after_dir).unwrap();
    create_random_files(after_dir, depth, &mut SmallRng::from_entropy());
    
    let archive = &dir.join(format!("archive.{}", merge_extensions(&ext, extra_extensions)));
    ouch!("-A", "c", before_dir, archive);

    let after_renamed_dir = &after.join("dir_1");
    assert_eq!(false, after_renamed_dir.exists());

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(after)
        .write_stdin("r")
        .assert()
        .success();

    assert_same_directory(before_dir, after_renamed_dir, false);
}

#[cfg(feature = "unrar")]
#[test]
fn unpack_rar() -> Result<(), Box<dyn std::error::Error>> {
    fn test_unpack_rar_single(input: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let dirpath = dir.path();
        let unpacked_path = &dirpath.join("testfile.txt");
        ouch!("-A", "d", input, "-d", dirpath);
        let content = fs::read_to_string(unpacked_path)?;
        assert_eq!(content, "Testing 123\n");

        Ok(())
    }

    let mut datadir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    datadir.push("tests/data");
    ["testfile.rar3.rar.gz", "testfile.rar5.rar"]
        .iter()
        .try_for_each(|path| test_unpack_rar_single(&datadir.join(path)))?;

    Ok(())
}

#[cfg(feature = "unrar")]
#[test]
fn unpack_rar_stdin() -> Result<(), Box<dyn std::error::Error>> {
    fn test_unpack_rar_single(input: &std::path::Path, format: &str) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let dirpath = dir.path();
        let unpacked_path = &dirpath.join("testfile.txt");
        crate::utils::cargo_bin()
            .args([
                "-A",
                "-y",
                "d",
                "-",
                "-d",
                dirpath.to_str().unwrap(),
                "--format",
                format,
            ])
            .pipe_stdin(input)
            .unwrap()
            .assert()
            .success();
        let content = fs::read_to_string(unpacked_path)?;
        assert_eq!(content, "Testing 123\n");

        Ok(())
    }

    let mut datadir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    datadir.push("tests/data");
    [("testfile.rar3.rar.gz", "rar.gz"), ("testfile.rar5.rar", "rar")]
        .iter()
        .try_for_each(|(path, format)| test_unpack_rar_single(&datadir.join(path), format))?;

    Ok(())
}
