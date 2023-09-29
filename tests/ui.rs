/// Snapshot tests for Ouch's output.
///
/// See CONTRIBUTING.md for a brief guide on how to use [`insta`] for these tests.
/// [`insta`]: https://docs.rs/insta

#[macro_use]
mod utils;

use std::{io, path::Path, process::Output};

#[cfg(not(windows))]
use insta::assert_display_snapshot as ui;

// Don't run these on Windows
#[cfg(windows)]
use self::ignore as ui;
use crate::utils::run_in;

fn testdir() -> io::Result<(tempfile::TempDir, &'static Path)> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().to_path_buf().into_boxed_path();
    Ok((dir, Box::leak(path)))
}

fn run_ouch(argv: &str, dir: &Path) -> String {
    let output = utils::cargo_bin()
        .args(argv.split_whitespace().skip(1))
        .current_dir(dir)
        .output()
        .unwrap_or_else(|err| {
            panic!(
                "Failed to run command\n\
                 argv: {argv}\n\
                 path: {dir:?}\n\
                 err: {err}"
            )
        });

    redact_paths(&output_to_string(output), dir)
}

// remove random tempdir paths from snapshots to make them deterministic
fn redact_paths(text: &str, path: &Path) -> String {
    let redacted = "<FOLDER>";

    let path = path.display();
    let path = if cfg!(target_os = "macos") {
        format!(r"/private{path}")
    } else {
        path.to_string()
    };

    text.replace(path.as_str(), redacted)
}

fn output_to_string(output: Output) -> String {
    String::from_utf8(output.stdout).unwrap() + std::str::from_utf8(&output.stderr).unwrap()
}

#[test]
fn ui_test_err_compress_missing_extension() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    run_in(dir, "touch", "input").unwrap();

    ui!(run_ouch("ouch compress input output", dir));
}

#[test]
fn ui_test_err_decompress_missing_extension() {
    let (_dropper, dir) = testdir().unwrap();

    run_in(dir, "touch", "a b.unknown").unwrap();

    ui!(run_ouch("ouch decompress a", dir));
    ui!(run_ouch("ouch decompress a b.unknown", dir));
    ui!(run_ouch("ouch decompress b.unknown", dir));
}

#[test]
fn ui_test_err_missing_files() {
    let (_dropper, dir) = testdir().unwrap();

    ui!(run_ouch("ouch compress a b", dir));
    ui!(run_ouch("ouch decompress a b", dir));
    ui!(run_ouch("ouch list a b", dir));
}

#[test]
fn ui_test_ok_compress() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    run_in(dir, "touch", "input").unwrap();

    ui!(run_ouch("ouch compress input output.zip", dir));
    ui!(run_ouch("ouch compress input output.gz", dir));
}

#[test]
fn ui_test_ok_decompress() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    run_in(dir, "touch", "input").unwrap();
    run_ouch("ouch compress input output.zst", dir);

    ui!(run_ouch("ouch decompress output.zst", dir));
}

#[test]
fn ui_test_usage_help_flag() {
    ui!(output_to_string(ouch!("--help")));
    ui!(output_to_string(ouch!("-h")));
}

#[allow(unused)]
#[macro_export]
macro_rules! ignore {
    ($expr:expr) => {{
        $expr
    }};
}
