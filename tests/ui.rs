// Snapshot tests for Ouch's output.

#[macro_use]
mod utils;

use std::{io, path::Path, process::Output};

use insta::assert_display_snapshot as ui;

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
    let path = format!("{}/", path.display());

    text.replace(&path, "<FOLDER>/")
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

    run_in(dir, "touch", "a").unwrap();

    ui!(run_ouch("ouch decompress a", dir));
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
