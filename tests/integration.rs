#[macro_use]
mod utils;

use std::{
    iter::once,
    path::{Path, PathBuf},
};

use fs_err as fs;
use parse_display::Display;
use proptest::sample::size_range;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use tempfile::tempdir;
use test_strategy::{proptest, Arbitrary};

use crate::utils::{assert_same_directory, write_random_content};

// tar and zip extensions
#[derive(Arbitrary, Debug, Display)]
#[display(style = "lowercase")]
enum DirectoryExtension {
    #[display("7z")]
    SevenZ,
    Tar,
    Tbz,
    Tbz2,
    Tgz,
    Tlz4,
    Tlzma,
    Tsz,
    Txz,
    Tzst,
    Zip,
}

// extensions of single file compression formats
#[derive(Arbitrary, Debug, Display)]
#[display(style = "lowercase")]
enum FileExtension {
    Bz,
    Bz2,
    Gz,
    Lz4,
    Lzma,
    Sz,
    Xz,
    Zst,
}

#[derive(Arbitrary, Debug, Display)]
#[display("{0}")]
enum Extension {
    Directory(DirectoryExtension),
    File(FileExtension),
}

// converts a list of extension structs to string
fn merge_extensions(ext: impl ToString, exts: Vec<FileExtension>) -> String {
    once(ext.to_string())
        .chain(exts.into_iter().map(|x| x.to_string()))
        .collect::<Vec<_>>()
        .join(".")
}

// create random nested directories and files under the specified directory
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
        create_random_files(&tempfile::tempdir_in(dir).unwrap().into_path(), depth - 1, rng);
    }
}

// compress and decompress a single empty file
#[proptest(cases = 200)]
fn single_empty_file(ext: Extension, #[any(size_range(0..8).lift())] exts: Vec<FileExtension>) {
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
    ouch!("-A", "c", before_file, archive);
    ouch!("-A", "d", archive, "-d", after);
    assert_same_directory(before, after, false);
}

// compress and decompress a single file
#[proptest(cases = 250)]
fn single_file(
    ext: Extension,
    #[any(size_range(0..8).lift())] exts: Vec<FileExtension>,
    #[cfg_attr(not(target_arch = "arm"), strategy(proptest::option::of(0i16..12)))]
    // Decrease the value of --level flag for `arm` systems, because our GitHub
    // Actions CI runs QEMU which makes the memory consumption higher.
    #[cfg_attr(target_arch = "arm", strategy(proptest::option::of(0i16..8)))]
    level: Option<i16>,
) {
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let before = &dir.join("before");
    fs::create_dir(before).unwrap();
    let before_file = &before.join("file");
    let archive = &dir.join(format!("file.{}", merge_extensions(ext, exts)));
    let after = &dir.join("after");
    fs::write(before_file, []).unwrap();
    if let Some(level) = level {
        ouch!("-A", "c", "-l", level.to_string(), before_file, archive);
    } else {
        ouch!("-A", "c", before_file, archive);
    }
    ouch!("-A", "d", archive, "-d", after);
    assert_same_directory(before, after, false);
}

// compress and decompress a directory with random content generated with create_random_files
//
// this one runs only 50 times because there are only `.zip` and `.tar` to be tested, and
// single-file formats testing is done in the other test
#[proptest(cases = 50)]
fn multiple_files(
    ext: DirectoryExtension,
    #[any(size_range(0..5).lift())] exts: Vec<FileExtension>,
    #[strategy(0u8..4)] depth: u8,
) {
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let before = &dir.join("before");
    let before_dir = &before.join("dir");
    fs::create_dir_all(before_dir).unwrap();
    let archive = &dir.join(format!("archive.{}", merge_extensions(&ext, exts)));
    let after = &dir.join("after");
    create_random_files(before_dir, depth, &mut SmallRng::from_entropy());
    ouch!("-A", "c", before_dir, archive);
    ouch!("-A", "d", archive, "-d", after);
    assert_same_directory(before, after, !matches!(ext, DirectoryExtension::Zip));
}

// test .rar decompression
fn test_unpack_rar_single(input: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let dirpath = dir.path();
    let unpacked_path = &dirpath.join("testfile.txt");
    ouch!("-A", "d", input, "-d", dirpath);
    let content = fs::read_to_string(unpacked_path)?;
    assert_eq!(content, "Testing 123\n");

    Ok(())
}

#[test]
fn unpack_rar() -> Result<(), Box<dyn std::error::Error>> {
    let mut datadir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    datadir.push("tests/data");
    ["testfile.rar3.rar.gz", "testfile.rar5.rar"]
        .iter()
        .try_for_each(|path| test_unpack_rar_single(&datadir.join(path)))?;

    Ok(())
}
