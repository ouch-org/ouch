#[macro_use]
mod utils;

use std::{io::Write, iter::once, path::PathBuf};

use fs_err as fs;
use parse_display::Display;
use proptest::sample::size_range;
use rand::{rngs::SmallRng, RngCore, SeedableRng};
use tempfile::tempdir;
use test_strategy::{proptest, Arbitrary};

use crate::utils::{assert_same_directory, write_random_content};

// tar and zip extensions
#[derive(Arbitrary, Debug, Display)]
#[display(style = "lowercase")]
enum DirectoryExtension {
    Tar,
    Tbz,
    Tgz,
    Tlz4,
    Tlz,
    Tlzma,
    Txz,
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
    Xz,
}

#[derive(Arbitrary, Debug, Display)]
#[display("{0}")]
enum Extension {
    Directory(DirectoryExtension),
    File(FileExtension),
}

// converts a list of extension structs to string
fn merge_extensions(ext: impl ToString, exts: Vec<FileExtension>) -> String {
    once(ext.to_string()).chain(exts.into_iter().map(|x| x.to_string())).collect::<Vec<_>>().join(".")
}

// create random nested directories and files under the specified directory
fn create_random_files(dir: impl Into<PathBuf>, depth: u8, rng: &mut SmallRng) {
    if depth == 0 {
        return;
    }

    let dir = &dir.into();

    // create 0 to 7 random files
    for _ in 0..rng.next_u32() % 8 {
        write_random_content(&mut tempfile::Builder::new().tempfile_in(dir).unwrap().keep().unwrap().0, rng);
    }

    // create more random files in 0 to 3 new directories
    for _ in 0..rng.next_u32() % 4 {
        create_random_files(&tempfile::tempdir_in(dir).unwrap().into_path(), depth - 1, rng);
    }
}

// compress and decompress a single empty file
#[proptest(cases = 512)]
fn single_empty_file(ext: Extension, #[any(size_range(0..8).lift())] exts: Vec<FileExtension>) {
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let before = &dir.join("before");
    fs::create_dir(before).unwrap();
    let before_file = &before.join("file");
    let archive = &dir.join(format!("file.{}", merge_extensions(ext, exts)));
    let after = &dir.join("after");
    write_random_content(&mut fs::File::create(before_file).unwrap(), &mut SmallRng::from_entropy());
    ouch!("c", before_file, archive);
    ouch!("d", archive, "-d", after);
    assert_same_directory(before, after, false);
}

// compress and decompress a single file
#[proptest(cases = 512)]
fn single_file(ext: Extension, #[any(size_range(0..8).lift())] exts: Vec<FileExtension>) {
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let before = &dir.join("before");
    fs::create_dir(before).unwrap();
    let before_file = &before.join("file");
    let archive = &dir.join(format!("file.{}", merge_extensions(ext, exts)));
    let after = &dir.join("after");
    fs::write(before_file, []).unwrap();
    ouch!("c", before_file, archive);
    ouch!("d", archive, "-d", after);
    assert_same_directory(before, after, false);
}

// compress and decompress a directory with random content generated with create_random_files
#[proptest(cases = 512)]
fn multiple_files(
    ext: DirectoryExtension,
    #[any(size_range(0..8).lift())] exts: Vec<FileExtension>,
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
    ouch!("c", before_dir, archive);
    ouch!("d", archive, "-d", after);
    assert_same_directory(before, after, !matches!(ext, DirectoryExtension::Zip));
}

#[test]
fn test_compress_decompress() {
    let dir = tempdir().unwrap();
    let dir = dir.path();
    let i1 = dir.join("i1");
    std::fs::write(&i1, "ouch").unwrap();
    let o1 = dir.join("o1.tar");
    let out = dir.join("out");
    std::fs::create_dir(&out).unwrap();

    {
        assert!(ouch_interactive!("c", &i1, &dir.join("o1.tar")).0.wait().unwrap().success());
    }
    {
        let (_ouch, mut sin, sout) = ouch_interactive!("d", &o1, "-d", &out);
        assert_eq!(sout.recv().unwrap(), format!("Do you want to overwrite '{}'? [Y/n] ", out.display()));
        writeln!(&mut sin, "n").unwrap();
    }
    {
        let (_ouch, mut sin, sout) = ouch_interactive!("d", &o1, "-d", &out);
        assert_eq!(sout.recv().unwrap(), format!("Do you want to overwrite '{}'? [Y/n] ", out.display()));
        writeln!(&mut sin, "Y").unwrap();
        assert_eq!(sout.recv().unwrap(), format!("[INFO] directory {} created.", out.display()));
        assert_eq!(sout.recv().unwrap(), format!("[INFO] \"{}\" extracted. (4.00 B)", out.join("i1").display()));
        assert_eq!(sout.recv().unwrap(), format!("[INFO] Successfully decompressed archive in '{}'.", out.display()));
        assert_eq!(sout.recv().unwrap(), "[INFO] Files unpacked: 1");
        assert_eq!(std::fs::read(&dir.join("out/i1")).unwrap(), b"ouch");
    }
}
