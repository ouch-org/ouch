/// Snapshot tests for Ouch's output.
///
/// See CONTRIBUTING.md for a brief guide on how to use [`insta`] for these tests.
/// [`insta`]: https://docs.rs/insta

#[macro_use]
mod utils;

use std::{ffi::OsStr, io, path::Path, process::Output};

use insta::assert_snapshot as ui;
use regex::Regex;

use crate::utils::create_files_in;

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

/// Remove random tempdir paths from snapshots to make them deterministic.
fn redact_paths(text: &str, dir: &Path) -> String {
    let dir_name = dir.file_name().and_then(OsStr::to_str).unwrap();

    // this regex should be good as long as the path does not contain whitespace characters
    let re = Regex::new(&format!(r"\S*[/\\]{dir_name}[/\\]")).unwrap();
    re.replace_all(text, "<TMP_DIR>/").into()
}

fn output_to_string(output: Output) -> String {
    String::from_utf8(output.stdout).unwrap() + std::str::from_utf8(&output.stderr).unwrap()
}

#[test]
fn ui_test_err_compress_missing_extension() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    create_files_in(dir, &["input"]);

    ui!(run_ouch("ouch compress input output", dir));
}

#[test]
fn ui_test_err_decompress_missing_extension() {
    let (_dropper, dir) = testdir().unwrap();

    create_files_in(dir, &["a", "b.unknown"]);

    let name = {
        let suffix = if cfg!(feature = "unrar") {
            "with_rar"
        } else {
            "without_rar"
        };
        format!("ui_test_err_decompress_missing_extension_{suffix}")
    };
    ui!(format!("{name}-1"), run_ouch("ouch decompress a", dir));
    ui!(format!("{name}-2"), run_ouch("ouch decompress a b.unknown", dir));
    ui!(format!("{name}-3"), run_ouch("ouch decompress b.unknown", dir));
}

#[test]
fn ui_test_err_missing_files() {
    let (_dropper, dir) = testdir().unwrap();

    ui!(run_ouch("ouch compress a b", dir));
    ui!(run_ouch("ouch decompress a b", dir));
    ui!(run_ouch("ouch list a b", dir));
}

#[test]
fn ui_test_err_format_flag() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    create_files_in(dir, &["input"]);

    ui!(run_ouch("ouch compress input output --format tar.gz.unknown", dir));
    ui!(run_ouch("ouch compress input output --format targz", dir));
    ui!(run_ouch("ouch compress input output --format .tar.$#!@.rest", dir));
}

#[test]
fn ui_test_ok_format_flag() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    create_files_in(dir, &["input"]);

    ui!(run_ouch("ouch compress input output1 --format tar.gz", dir));
    ui!(run_ouch("ouch compress input output2 --format .tar.gz", dir));
}

#[test]
fn ui_test_ok_compress() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    create_files_in(dir, &["input"]);

    ui!(run_ouch("ouch compress input output.zip", dir));
    ui!(run_ouch("ouch compress input output.gz", dir));
}

#[test]
fn ui_test_ok_decompress() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    create_files_in(dir, &["input"]);
    run_ouch("ouch compress input output.zst", dir);

    ui!(run_ouch("ouch decompress output.zst", dir));
}

#[test]
fn ui_test_usage_help_flag() {
    insta::with_settings!({filters => vec![
        // binary name is `ouch.exe` on Windows and `ouch` on everywhere else
        (r"(Usage:.*\b)ouch(\.exe)?\b", "${1}<OUCH_BIN>"),
    ]}, {
        ui!(output_to_string(ouch!("--help")));
        ui!(output_to_string(ouch!("-h")));
    });
}
