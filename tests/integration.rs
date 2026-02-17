#[macro_use]
mod utils;

use std::{
    io::Write,
    iter::once,
    path::{Path, PathBuf},
};

use bstr::ByteSlice;
use fs_err as fs;
use itertools::Itertools;
use memchr::memmem;
use parse_display::Display;
use pretty_assertions::assert_eq;
use proptest::sample::size_range;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use test_strategy::{proptest, Arbitrary};

use crate::utils::{assert_same_directory, testdir, write_random_content};

/// tar and zip extensions
#[derive(Arbitrary, Clone, Copy, Debug, Display)]
#[display(style = "lowercase")]
enum DirectoryExtension {
    #[display("7z")]
    SevenZ,
    Tar,
    Tbz,
    Tbz2,
    #[cfg(feature = "bzip3")]
    Tbz3,
    Tgz,
    Tlz,
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
    #[cfg(feature = "bzip3")]
    Bz3,
    Gz,
    Lz,
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
fn merge_extensions(ext: impl ToString, exts: &[FileExtension]) -> String {
    once(ext.to_string())
        .chain(exts.iter().map(|x| x.to_string()))
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

/// Create n random files on directory dir
fn create_n_random_files(n: usize, dir: impl Into<PathBuf>, rng: &mut SmallRng) {
    let dir: &PathBuf = &dir.into();

    for _ in 0..n {
        write_random_content(
            &mut tempfile::Builder::new()
                .prefix("file")
                .tempfile_in(dir)
                .unwrap()
                .keep()
                .unwrap()
                .0,
            rng,
        );
    }
}

/// Compress and decompress a single empty file
#[proptest(cases = 200)]
fn single_empty_file(ext: Extension, #[any(size_range(0..8).lift())] exts: Vec<FileExtension>) {
    let (_tempdir, dir) = testdir().unwrap();
    let before = &dir.join("before");
    fs::create_dir(before).unwrap();
    let before_file = &before.join("file");
    let archive = &dir.join(format!("file.{}", merge_extensions(ext, &exts)));
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
    let (_tempdir, dir) = testdir().unwrap();
    let before = &dir.join("before");
    fs::create_dir(before).unwrap();
    let before_file = &before.join("file");
    let archive = &dir.join(format!("file.{}", merge_extensions(ext, &exts)));
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
    let (_tempdir, dir) = testdir().unwrap();
    let before = &dir.join("before");
    fs::create_dir(before).unwrap();
    let before_file = &before.join("file");
    let format = merge_extensions(&ext, &exts);
    let archive = &dir.join(format!("file.{format}"));
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
        // Ouch writes to `ouch-output` when the input is stdin
        // Change the top-level "before" directory to match
        Extension::File(_) => fs::rename(before_file, before_file.with_file_name("ouch-output")).unwrap(),
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
    let (_tempdir, dir) = testdir().unwrap();
    let before = &dir.join("before");
    let before_dir = &before.join("dir");
    fs::create_dir_all(before_dir).unwrap();
    let archive = &dir.join(format!("archive.{}", merge_extensions(ext, &extra_extensions)));
    let after = &dir.join("after");
    create_random_files(before_dir, depth, &mut SmallRng::from_entropy());
    ouch!("-A", "c", before_dir, archive);
    ouch!("-A", "d", archive, "-d", after);
    assert_same_directory(before, after, !matches!(ext, DirectoryExtension::Zip));
}

#[proptest(cases = 25)]
fn multiple_files_with_conflict_and_choice_to_overwrite(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
    #[strategy(0u8..3)] depth: u8,
) {
    let (_tempdir, dir) = testdir().unwrap();

    let before = &dir.join("before");
    let before_dir = &before.join("dir");
    fs::create_dir_all(before_dir).unwrap();
    create_random_files(before_dir, depth, &mut SmallRng::from_entropy());

    let after = &dir.join("after");
    let after_dir = &after.join("dir");
    fs::create_dir_all(after_dir).unwrap();
    create_random_files(after_dir, depth, &mut SmallRng::from_entropy());

    let archive = &dir.join(format!("archive.{}", merge_extensions(ext, &extra_extensions)));
    ouch!("-A", "c", before_dir, archive);

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(after)
        .arg("--yes")
        .assert()
        .success();

    assert_same_directory(before, after, false);
}

#[proptest(cases = 25)]
fn multiple_files_with_conflict_and_choice_to_not_overwrite(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
    #[strategy(0u8..3)] depth: u8,
) {
    let (_tempdir, dir) = testdir().unwrap();

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

    let archive = &dir.join(format!("archive.{}", merge_extensions(ext, &extra_extensions)));
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

#[proptest(cases = 25)]
fn multiple_files_with_conflict_and_choice_to_rename(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
) {
    let (_tempdir, root_path) = testdir().unwrap();

    let src_files_path = root_path.join("src_files");
    fs::create_dir_all(&src_files_path).unwrap();
    create_n_random_files(5, &src_files_path, &mut SmallRng::from_entropy());

    // Make destiny already filled to force a conflict
    let dest_files_path = root_path.join("dest_files");
    fs::create_dir_all(&dest_files_path).unwrap();
    create_n_random_files(5, &dest_files_path, &mut SmallRng::from_entropy());

    let archive = &root_path.join(format!("archive.{}", merge_extensions(ext, &extra_extensions)));
    ouch!("-A", "c", &src_files_path, archive);

    let dest_files_path_renamed = &root_path.join("dest_files_1");
    assert_eq!(false, dest_files_path_renamed.exists());

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(&dest_files_path)
        .write_stdin("r")
        .assert()
        .success();

    assert_same_directory(src_files_path, dest_files_path_renamed.join("src_files"), false);
}

#[proptest(cases = 25)]
fn multiple_files_with_conflict_and_choice_to_rename_with_already_a_renamed(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
) {
    let (_tempdir, root_path) = testdir().unwrap();

    let src_files_path = root_path.join("src_files");
    fs::create_dir_all(&src_files_path).unwrap();
    create_n_random_files(5, &src_files_path, &mut SmallRng::from_entropy());

    // Make destiny already filled and destiny with '_1'
    let dest_files_path = root_path.join("dest_files");
    fs::create_dir_all(&dest_files_path).unwrap();
    create_n_random_files(5, &dest_files_path, &mut SmallRng::from_entropy());

    let dest_files_path_1 = root_path.join("dest_files_1");
    fs::create_dir_all(&dest_files_path_1).unwrap();
    create_n_random_files(5, &dest_files_path_1, &mut SmallRng::from_entropy());

    let archive = &root_path.join(format!("archive.{}", merge_extensions(ext, &extra_extensions)));
    ouch!("-A", "c", &src_files_path, archive);

    let dest_files_path_renamed = &root_path.join("dest_files_2");
    assert_eq!(false, dest_files_path_renamed.exists());

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(&dest_files_path)
        .write_stdin("r")
        .assert()
        .success();

    assert_same_directory(src_files_path, dest_files_path_renamed.join("src_files"), false);
}

#[cfg(feature = "unrar")]
#[test]
fn unpack_rar() -> Result<(), Box<dyn std::error::Error>> {
    fn test_unpack_rar_single(input: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        let (_tempdir, dirpath) = testdir()?;
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
        let (_tempdir, dirpath) = testdir()?;
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

#[proptest(cases = 25)]
fn symlink_pack_and_unpack(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
) {
    if matches!(ext, DirectoryExtension::SevenZ) {
        // Skip 7z because the 7z format does not support symlinks
        return Ok(());
    }

    let (_tempdir, root_path) = testdir()?;

    let src_files_path = root_path.join("src_files");
    let folder_path = src_files_path.join("folder");
    fs::create_dir_all(&folder_path)?;

    let mut files_path = ["file1.txt", "file2.txt", "file3.txt", "file4.txt", "file5.txt"]
        .into_iter()
        .map(|f| src_files_path.join(f))
        .inspect(|path| {
            let mut file = fs::File::create(path).unwrap();
            file.write_all("Some content".as_bytes()).unwrap();
        })
        .collect::<Vec<_>>();

    let dest_files_path = root_path.join("dest_files");
    fs::create_dir_all(&dest_files_path)?;

    let symlink_path = src_files_path.join(Path::new("symlink"));
    let symlink_folder_path = src_files_path.join(Path::new("symlink_folder"));
    #[cfg(unix)]
    std::os::unix::fs::symlink(&files_path[0], &symlink_path)?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(&folder_path, &symlink_folder_path)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&files_path[0], &symlink_path)?;
    #[cfg(windows)]
    std::os::windows::fs::symlink_dir(&folder_path, &symlink_folder_path)?;

    files_path.push(symlink_path);

    let archive = &root_path.join(format!("archive.{}", merge_extensions(ext, &extra_extensions)));

    crate::utils::cargo_bin()
        .arg("compress")
        .args(files_path.clone())
        .arg(archive)
        .assert()
        .success();

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(&dest_files_path)
        .assert()
        .success();

    // check the symlink stand still
    for f in dest_files_path.as_path().read_dir()? {
        let f = f?;
        if f.file_name() == "symlink" || f.file_name() == "symlink_folder" {
            assert!(f.file_type()?.is_symlink())
        }
    }

    fs::remove_file(archive)?;
    fs::remove_dir_all(&dest_files_path)?;

    crate::utils::cargo_bin()
        .arg("compress")
        .arg("--follow-symlinks")
        .args(files_path)
        .arg(archive)
        .assert()
        .success();

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(&dest_files_path)
        .assert()
        .success();

    // check there is no symlinks
    for f in dest_files_path.as_path().read_dir()? {
        let f = f?;
        assert!(!f.file_type().unwrap().is_symlink())
    }
}

#[test]
fn no_git_folder_after_decompression_with_gitignore_flag_active() {
    use std::process::Command;

    let (_tempdir, dir_path) = testdir().unwrap();

    let before = dir_path.join("before");

    let decompressed = dir_path.join("decompressed");

    // Create directory and a dummy file
    fs::create_dir(&before).unwrap();
    fs::write(before.join("hello.txt"), b"Hello, world!").unwrap();

    // Run `git init` inside it
    Command::new("git")
        .arg("init")
        .current_dir(&before)
        .output()
        .expect("failed to run git init");

    assert!(before.join(".git").exists(), ".git folder should exist after git init");

    // Compress it
    let archive = dir_path.join("archive.zip");
    ouch!("c", &before, &archive, "--gitignore");

    // Decompress it
    ouch!("d", &archive, "-d", &decompressed);

    // Find the subdirectory inside decompressed (e.g., "before")
    let decompressed_subdir = fs::read_dir(&decompressed)
        .unwrap()
        .find_map(Result::ok)
        .map(|entry| entry.path())
        .expect("Expected one directory inside decompressed");

    // Assert that the decompressed folder does not include `.git/`
    assert!(
        !decompressed_subdir.join(".git").exists(),
        ".git folder should not exist after decompression"
    );
}

#[proptest(cases = 25)]
fn enable_gitignore_flag_should_work_without_git(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
) {
    let (_tempdir, root_path) = testdir()?;
    let source_path = root_path.join(format!("in_{}", merge_extensions(ext, &extra_extensions)));
    fs::create_dir_all(&source_path)?;
    let out_path = root_path.join(format!("out_{}", merge_extensions(ext, &extra_extensions)));
    fs::create_dir_all(&out_path)?;

    let mut gitignore_file = fs::File::create(source_path.join(".gitignore"))?;
    gitignore_file.write_all(b"a")?;
    let mut ignore_file = fs::File::create(source_path.join(".ignore"))?;
    ignore_file.write_all(b"b")?;

    fs::File::create(source_path.join("a"))?;
    fs::File::create(source_path.join("b"))?;
    fs::File::create(source_path.join("c"))?;

    let archive = root_path.join(format!("archive.{}", merge_extensions(ext, &extra_extensions)));
    crate::utils::cargo_bin()
        .arg("compress")
        .arg("--gitignore")
        .arg("--hidden")
        .arg(&source_path)
        .arg(&archive)
        .assert()
        .success();

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(&out_path)
        .assert()
        .success();

    // only the file name 'c' exists
    assert_eq!(
        1,
        out_path
            .join(format!("in_{}", merge_extensions(ext, &extra_extensions)))
            .as_path()
            .read_dir()?
            .count()
    );
}

#[proptest(cases = 25)]
fn unpack_multiple_sources_into_the_same_destination_with_merge(
    ext: DirectoryExtension,
    #[any(size_range(0..1).lift())] extra_extensions: Vec<FileExtension>,
) {
    let (_tempdir, root_path) = testdir()?;
    let source_path = root_path
        .join(format!("example_{}", merge_extensions(ext, &extra_extensions)))
        .join("sub_a")
        .join("sub_b")
        .join("sub_c");

    fs::create_dir_all(&source_path)?;
    let archive = root_path.join(format!("archive.{}", merge_extensions(ext, &extra_extensions)));
    crate::utils::cargo_bin()
        .arg("compress")
        .args([
            fs::File::create(source_path.join("file1.txt"))?.path(),
            fs::File::create(source_path.join("file2.txt"))?.path(),
            fs::File::create(source_path.join("file3.txt"))?.path(),
        ])
        .arg(&archive)
        .assert()
        .success();

    fs::remove_dir_all(&source_path)?;
    fs::create_dir_all(&source_path)?;
    let archive1 = root_path.join(format!("archive1.{}", merge_extensions(ext, &extra_extensions)));
    crate::utils::cargo_bin()
        .arg("compress")
        .args([
            fs::File::create(source_path.join("file3.txt"))?.path(),
            fs::File::create(source_path.join("file4.txt"))?.path(),
            fs::File::create(source_path.join("file5.txt"))?.path(),
        ])
        .arg(&archive1)
        .assert()
        .success();

    let out_path = root_path.join(format!("out_{}", merge_extensions(ext, &extra_extensions)));
    fs::create_dir_all(&out_path)?;

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(&out_path)
        .assert()
        .success();

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive1)
        .arg("-d")
        .arg(&out_path)
        .write_stdin("m")
        .assert()
        .success();

    assert_eq!(5, out_path.as_path().read_dir()?.count());
}

#[test]
fn reading_nested_archives_with_two_archive_extensions_adjacent() {
    let archive_formats = ["tar", "zip", "7z"].into_iter();

    for (first_archive, second_archive) in archive_formats.clone().cartesian_product(archive_formats.rev()) {
        let (_tempdir, dir) = testdir().unwrap();
        let in_dir = |path: &str| format!("{}/{}", dir.display(), path);

        fs::write(in_dir("a.txt"), "contents").unwrap();

        let files = [
            "a.txt",
            &format!("b.{first_archive}"),
            &format!("c.{first_archive}.{second_archive}"),
        ];
        let transformations = [first_archive, second_archive];
        let compressed_path = in_dir(files.last().unwrap());

        for (window, format) in files.windows(2).zip(transformations.iter()) {
            let [a, b] = [window[0], window[1]].map(in_dir);
            crate::utils::cargo_bin()
                .args(["compress", &a, &b, "--format", format])
                .assert()
                .success();
        }

        let output = crate::utils::cargo_bin()
            .args(["list", &compressed_path, "--yes"])
            .assert()
            .failure()
            .get_output()
            .clone();
        let stderr = output.stderr.to_str().unwrap();
        assert!(memmem::find(stderr.as_bytes(), b"use `--format` to specify what format to use").is_some());

        let output = crate::utils::cargo_bin()
            .args(["decompress", &compressed_path, "--dir", &in_dir("out"), "--yes"])
            .assert()
            .failure()
            .get_output()
            .clone();
        let stderr = output.stderr.to_str().unwrap();
        assert!(memmem::find(stderr.as_bytes(), b"use `--format` to specify what format to use").is_some());
    }
}

#[test]
fn reading_nested_archives_with_two_archive_extensions_interleaved() {
    let archive_formats = ["tar", "zip", "7z"].into_iter();

    for (first_archive, second_archive) in archive_formats.clone().cartesian_product(archive_formats.rev()) {
        let (_tempdir, dir) = testdir().unwrap();
        let in_dir = |path: &str| format!("{}/{}", dir.display(), path);

        fs::write(in_dir("a.txt"), "contents").unwrap();

        let files = [
            "a.txt",
            &format!("c.{first_archive}"),
            &format!("d.{first_archive}.zst"),
            &format!("e.{first_archive}.zst.{second_archive}"),
            &format!("f.{first_archive}.zst.{second_archive}.lz4"),
        ];
        let transformations = [first_archive, "zst", second_archive, "lz4"];
        let compressed_path = in_dir(files.last().unwrap());

        for (window, format) in files.windows(2).zip(transformations.iter()) {
            let [a, b] = [window[0], window[1]].map(in_dir);
            crate::utils::cargo_bin()
                .args(["compress", &a, &b, "--format", format])
                .assert()
                .success();
        }

        let output = crate::utils::cargo_bin()
            .args(["list", &compressed_path, "--yes"])
            .assert()
            .failure()
            .get_output()
            .clone();
        let stderr = output.stderr.to_str().unwrap();
        assert!(memmem::find(stderr.as_bytes(), b"use `--format` to specify what format to use").is_some());

        let output = crate::utils::cargo_bin()
            .args(["decompress", &compressed_path, "--dir", &in_dir("out"), "--yes"])
            .assert()
            .failure()
            .get_output()
            .clone();
        let stderr = output.stderr.to_str().unwrap();
        assert!(memmem::find(stderr.as_bytes(), b"use `--format` to specify what format to use").is_some());
    }
}

#[test]
fn compressing_archive_with_two_archive_formats() {
    let archive_formats = ["tar", "zip", "7z"].into_iter();

    for (first_archive, second_archive) in archive_formats.clone().cartesian_product(archive_formats.rev()) {
        let (_tempdir, dir_path) = testdir().unwrap();
        let dir = dir_path.display().to_string();

        let output = crate::utils::cargo_bin()
            .args([
                "compress",
                "README.md",
                &format!("{dir}/out.{first_archive}.{second_archive}"),
                "--yes",
            ])
            .assert()
            .failure()
            .get_output()
            .clone();

        let stderr = output.stderr.to_str().unwrap();
        assert!(memmem::find(stderr.as_bytes(), b"use `--format` to specify what format to use").is_some());

        let output = crate::utils::cargo_bin()
            .args([
                "compress",
                "README.md",
                &format!("{dir}/out.{first_archive}.{second_archive}"),
                "--yes",
                "--format",
                &format!("{first_archive}.{second_archive}"),
            ])
            .assert()
            .failure()
            .get_output()
            .clone();

        let stderr = output.stderr.to_str().unwrap();
        assert!(memmem::find(
            stderr.as_bytes(),
            b"can only be used at the start of the file extension",
        )
        .is_some());

        crate::utils::cargo_bin()
            .args([
                "compress",
                "README.md",
                &format!("{dir}/out.{first_archive}.{second_archive}"),
                "--yes",
                "--format",
                first_archive,
            ])
            .assert()
            .success();
    }
}

#[test]
fn fail_when_compressing_archive_as_the_second_extension() {
    for archive_format in ["tar", "zip", "7z"] {
        let (_tempdir, dir_path) = testdir().unwrap();
        let dir = dir_path.display().to_string();

        let output = crate::utils::cargo_bin()
            .args([
                "compress",
                "README.md",
                &format!("{dir}/out.zst.{archive_format}"),
                "--yes",
            ])
            .assert()
            .failure()
            .get_output()
            .clone();

        let stderr = output.stderr.to_str().unwrap();
        assert!(memmem::find(stderr.as_bytes(), b"use `--format` to specify what format to use").is_some());

        let output = crate::utils::cargo_bin()
            .args([
                "compress",
                "README.md",
                &format!("{dir}/out_file"),
                "--yes",
                "--format",
                &format!("zst.{archive_format}"),
            ])
            .assert()
            .failure()
            .get_output()
            .clone();

        let stderr = output.stderr.to_str().unwrap();
        assert!(memmem::find(
            stderr.as_bytes(),
            format!("'{archive_format}' can only be used at the start of the file extension").as_bytes(),
        )
        .is_some());
    }
}

#[test]
fn sevenz_list_should_not_failed() {
    let (_tempdir, root_path) = testdir().unwrap();
    let src_files_path = root_path.join("src_files");
    fs::create_dir_all(&src_files_path).unwrap();

    let archive = root_path.join("archive.7z.gz");
    crate::utils::cargo_bin()
        .arg("compress")
        .arg("--yes")
        .arg(fs::File::create(src_files_path.join("README.md")).unwrap().path())
        .arg(&archive)
        .assert()
        .success();

    let res = crate::utils::cargo_bin()
        .arg("list")
        .arg("--yes")
        .arg(&archive)
        .assert()
        .success();

    assert!(res.get_output().stdout.find(b"README.md").is_some());
}

// TODO: for supporting windows hard link easier
// we should wait for this issue
// https://github.com/rust-lang/rust/issues/63010
#[cfg(unix)]
#[test]
fn tar_hardlink_pack_and_unpack() {
    use std::{fs::hard_link, os::unix::fs::MetadataExt};

    let (_tempdir, root_path) = testdir().unwrap();
    let source_path = root_path.join("hardlink");
    fs::create_dir_all(&source_path).unwrap();
    let out_path = root_path.join("out");
    fs::create_dir_all(&out_path).unwrap();

    let source = fs::File::create(source_path.join("source")).unwrap();
    let link1 = source_path.join("link1");
    let link2 = source_path.join("link2");
    hard_link(source.path(), link1.as_path()).unwrap();
    hard_link(source.path(), link2.as_path()).unwrap();

    let archive = root_path.join("archive.tar.gz");
    crate::utils::cargo_bin()
        .arg("compress")
        .arg(&source_path)
        .arg(&archive)
        .assert()
        .success();

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(archive)
        .arg("-d")
        .arg(&out_path)
        .assert()
        .success();

    let out_source_meta = fs::File::open(out_path.join("hardlink").join("source"))
        .unwrap()
        .metadata()
        .unwrap();
    let out_link1_meta = fs::File::open(out_path.join("hardlink").join("link1"))
        .unwrap()
        .metadata()
        .unwrap();
    let out_link2_meta = fs::File::open(out_path.join("hardlink").join("link2"))
        .unwrap()
        .metadata()
        .unwrap();

    assert!(out_source_meta.nlink() > 1);
    assert!(out_link1_meta.nlink() > 1);
    assert!(out_link2_meta.nlink() > 1);

    assert_eq!(out_source_meta.dev(), out_link1_meta.dev());
    assert_eq!(out_link1_meta.dev(), out_link2_meta.dev());

    assert_eq!(out_source_meta.ino(), out_link1_meta.ino());
    assert_eq!(out_link1_meta.ino(), out_link2_meta.ino());
}

#[test]
fn compress_with_rename_conflict() {
    let (_tempdir, root_path) = testdir().unwrap();

    let file_path = root_path.join("file.txt");
    fs::write(&file_path, "content").unwrap();

    let archive = root_path.join("archive.tar.gz");

    for _ in 0..3 {
        crate::utils::cargo_bin()
            .arg("compress")
            .arg(&file_path)
            .arg(&archive)
            .write_stdin("r\n")
            .assert()
            .success();
    }

    assert!(root_path.join("archive.tar.gz").exists());
    assert!(root_path.join("archive_1.tar.gz").exists());
    assert!(root_path.join("archive_2.tar.gz").exists());
}

#[test]
fn decompress_with_mismatched_extension_should_use_detected_format() {
    let (_tempdir, test_dir) = testdir().unwrap();

    let original_file = test_dir.join("input.txt");
    fs::write(&original_file, "Hello, world!").unwrap();

    let gzip_archive = test_dir.join("archive.gz");
    let misnamed_archive = test_dir.join("archive.zst");

    crate::utils::cargo_bin()
        .arg("compress")
        .arg(&original_file)
        .arg(&gzip_archive)
        .assert()
        .success();

    // Rename the .gz file to have a .zst extension (wrong extension)
    fs::rename(&gzip_archive, &misnamed_archive).unwrap();

    let output_dir = test_dir.join("output");
    fs::create_dir(&output_dir).unwrap();

    let output = crate::utils::cargo_bin()
        .arg("decompress")
        .arg(misnamed_archive)
        // TODO: --dir shouldn't be necessary here, this is a bug
        .arg("--dir")
        .arg(output_dir)
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Format mismatch"), "Expected format mismatch error");
    assert!(stderr.contains("--format"), "Expected hint about --format flag");
}

#[proptest(cases = 10)]
fn decompress_with_unknown_extension_should_detect_format_and_ask(
    ext: FileExtension,
    contains_extension_in_filename: bool,
) {
    let (_tempdir, test_dir) = testdir()?;

    // Brotli has no magic bytes
    if let FileExtension::Br = ext {
        return Ok(());
    }

    let original_file = test_dir.join("input.txt");
    let original_content = "Hello, world!";
    fs::write(&original_file, original_content)?;

    let compressed_archive = test_dir.join(format!("file.{ext}"));

    crate::utils::cargo_bin()
        .arg("compress")
        .arg(&original_file)
        .arg(&compressed_archive)
        .assert()
        .success();

    let erased_ext_filename = if contains_extension_in_filename {
        "file.unknown"
    } else {
        "file"
    };
    let unknown_path = test_dir.join(erased_ext_filename);
    // Rename to have an unknown extension (no recognized format)
    fs::rename(&compressed_archive, &unknown_path)?;

    let output_dir = test_dir.join("output");
    fs::create_dir(&output_dir)?;

    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(&unknown_path)
        .arg("--dir")
        .arg(&output_dir)
        .write_stdin("y\n")
        .assert()
        .success();

    // Ouch adds this suffix so it doesn't conflict with other stuff
    let decompressed_file = output_dir.join("file-output");
    let decompressed_content = fs::read_to_string(&decompressed_file)?;
    assert_eq!(decompressed_content, original_content);
}

/// Helper function to test decompression of concatenated streams (issue #855).
/// Takes a file extension and a compression function that compresses a single chunk.
fn test_concatenated_streams(extension: &str, compress_chunk: impl Fn(&[u8]) -> Vec<u8>) {
    use std::io::Write;

    let (_tempdir, root_path) = testdir().unwrap();

    // Create content for three separate streams
    let chunks: &[&[u8]] = &[
        b"First stream content - this is stream 1\n",
        b"Second stream content - this is stream 2\n",
        b"Third stream content - this is stream 3\n",
    ];

    // Create the concatenated file
    let concatenated_path = root_path.join(format!("concatenated.{extension}"));
    {
        let mut file = fs::File::create(&concatenated_path).unwrap();
        for chunk in chunks {
            file.write_all(&compress_chunk(chunk)).unwrap();
        }
    }

    // Decompress using ouch
    crate::utils::cargo_bin()
        .arg("decompress")
        .arg(&concatenated_path)
        .arg("-d")
        .arg(root_path)
        .arg("--yes")
        .assert()
        .success();

    // Verify the output contains all streams
    let output_path = root_path.join("concatenated");
    let output_content = fs::read(&output_path).unwrap();
    let expected_content: Vec<u8> = chunks.iter().flat_map(|c| c.iter().copied()).collect();
    assert_eq!(
        output_content, expected_content,
        "Decompressed content should contain all concatenated {extension} streams"
    );
}

/// Test that concatenated gzip streams are fully decompressed (issue #855)
#[test]
fn decompress_concatenated_gzip_streams() {
    use std::io::Write;

    use flate2::{write::GzEncoder, Compression};

    test_concatenated_streams("gz", |data| {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    });
}

/// Test that concatenated bzip2 streams are fully decompressed (related to issue #855)
#[test]
fn decompress_concatenated_bzip2_streams() {
    use std::io::Write;

    use bzip2::{write::BzEncoder, Compression};

    test_concatenated_streams("bz2", |data| {
        let mut encoder = BzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    });
}

/// Test that concatenated lz4 frames are fully decompressed (related to issue #855)
#[test]
fn decompress_concatenated_lz4_frames() {
    use std::io::Write;

    use lz4_flex::frame::FrameEncoder;

    test_concatenated_streams("lz4", |data| {
        let mut encoder = FrameEncoder::new(Vec::new());
        encoder.write_all(data).unwrap();
        encoder.finish().unwrap()
    });
}
