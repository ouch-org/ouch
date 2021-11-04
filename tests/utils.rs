use std::{io::Write, path::PathBuf};

use fs_err as fs;
use rand::RngCore;

#[macro_export]
macro_rules! ouch {
    ($($e:expr),*) => {
        ::assert_cmd::Command::cargo_bin("ouch")
            .expect("Failed to find ouch executable")
            $(.arg($e))*
            .unwrap();
    }
}

pub fn create_file_random(file: &mut impl Write, rng: &mut impl RngCore) {
    let data = &mut Vec::with_capacity((rng.next_u32() % 8192) as usize);
    rng.fill_bytes(data);
    file.write_all(data).unwrap();
}

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
