use std::{env, io::Write, path::PathBuf};

use assert_cmd::Command;
use fs_err as fs;
use rand::{Rng, RngCore};

#[macro_export]
macro_rules! ouch {
    ($($e:expr),*) => {
        $crate::utils::cargo_bin()
            $(.arg($e))*
            .unwrap();
    }
}

pub fn cargo_bin() -> Command {
    env::vars()
        .find_map(|(k, v)| {
            (k.starts_with("CARGO_TARGET_") && k.ends_with("_RUNNER")).then(|| {
                let mut runner = v.split_whitespace();
                let mut cmd = Command::new(runner.next().unwrap());
                cmd.args(runner).arg(assert_cmd::cargo::cargo_bin("ouch"));
                cmd
            })
        })
        .unwrap_or_else(|| Command::cargo_bin("ouch").expect("Failed to find ouch executable"))
}

// write random content to a file
pub fn write_random_content(file: &mut impl Write, rng: &mut impl RngCore) {
    let mut data = Vec::new();
    data.resize(rng.gen_range(0..8192), 0);
    rng.fill_bytes(&mut data);
    file.write_all(&data).unwrap();
}

// check that two directories have the exact same content recursively
// checks equility of file types if preserve_permissions is true, ignored on non-unix
pub fn assert_same_directory(x: impl Into<PathBuf>, y: impl Into<PathBuf>, preserve_permissions: bool) {
    fn read_dir(dir: impl Into<PathBuf>) -> impl Iterator<Item = fs::DirEntry> {
        let mut dir: Vec<_> = fs::read_dir(dir).unwrap().map(|entry| entry.unwrap()).collect();
        dir.sort_by_key(|x| x.file_name());
        dir.into_iter()
    }

    let mut x = read_dir(x);
    let mut y = read_dir(y);

    loop {
        match (x.next(), y.next()) {
            (Some(x), Some(y)) => {
                assert_eq!(x.file_name(), y.file_name());

                let meta_x = x.metadata().unwrap();
                let meta_y = y.metadata().unwrap();
                let ft_x = meta_x.file_type();
                let ft_y = meta_y.file_type();

                #[cfg(unix)]
                if preserve_permissions {
                    assert_eq!(ft_x, ft_y);
                }

                if ft_x.is_dir() && ft_y.is_dir() {
                    assert_same_directory(x.path(), y.path(), preserve_permissions);
                } else if ft_x.is_file() && ft_y.is_file() {
                    assert_eq!(meta_x.len(), meta_y.len());
                    assert_eq!(fs::read(x.path()).unwrap(), fs::read(y.path()).unwrap());
                } else {
                    panic!(
                        "entries should be both directories or both files\n  left: `{:?}`,\n right: `{:?}`",
                        x.path(),
                        y.path()
                    );
                }
            }

            (None, None) => break,

            (x, y) => {
                panic!(
                    "directories don't have the same number of entires\n  left: `{:?}`,\n right: `{:?}`",
                    x.map(|x| x.path()),
                    y.map(|y| y.path()),
                )
            }
        }
    }
}

#[test]
fn src_is_src() {
    assert_same_directory("src", "src", true);
}

#[test]
#[should_panic]
fn src_is_not_tests() {
    assert_same_directory("src", "tests", false);
}
