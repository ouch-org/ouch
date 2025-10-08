/// Snapshot tests for Ouch's output.
///
/// See CONTRIBUTING.md for a brief guide on how to use [`insta`] for these tests.
/// [`insta`]: https://docs.rs/insta
#[macro_use]
mod utils;

#[cfg(target_os = "linux")]
use std::collections::BTreeSet;
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

    let snapshot = concat_snapshot_filename_rar_feature("ui_test_err_decompress_missing_extension");
    ui!(format!("{snapshot}-1"), run_ouch("ouch decompress a", dir));
    ui!(format!("{snapshot}-2"), run_ouch("ouch decompress a b.unknown", dir));
    ui!(format!("{snapshot}-3"), run_ouch("ouch decompress b.unknown", dir));
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

    let snapshot = concat_snapshot_filename_rar_feature("ui_test_err_format_flag");
    ui!(
        format!("{snapshot}-1"),
        run_ouch("ouch compress input output --format tar.gz.unknown", dir),
    );
    ui!(
        format!("{snapshot}-2"),
        run_ouch("ouch compress input output --format targz", dir),
    );
    ui!(
        format!("{snapshot}-3"),
        run_ouch("ouch compress input output --format .tar.$#!@.rest", dir),
    );
}

#[test]
fn ui_test_ok_format_flag() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    create_files_in(dir, &["input"]);

    let snapshot = concat_snapshot_filename_rar_feature("ui_test_ok_format_flag");
    ui!(
        format!("{snapshot}-1"),
        run_ouch("ouch compress input output1 --format tar.gz", dir),
    );
    ui!(
        format!("{snapshot}-2"),
        run_ouch("ouch compress input output2 --format .tar.gz", dir),
    );
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

#[cfg(target_os = "linux")]
#[test]
fn ui_test_ok_decompress_multiple_files() {
    let (_dropper, dir) = testdir().unwrap();

    let inputs_dir = dir.join("inputs");
    std::fs::create_dir(&inputs_dir).unwrap();

    let outputs_dir = dir.join("outputs");
    std::fs::create_dir(&outputs_dir).unwrap();

    // prepare
    create_files_in(&inputs_dir, &["input", "input2", "input3"]);

    let compress_command = format!("ouch compress {} output.tar.zst", inputs_dir.to_str().unwrap());
    run_ouch(&compress_command, dir);

    let decompress_command = format!("ouch decompress output.tar.zst --dir {}", outputs_dir.to_str().unwrap());
    let stdout = run_ouch(&decompress_command, dir);
    let stdout_lines = stdout.split('\n').collect::<BTreeSet<_>>();
    insta::assert_debug_snapshot!(stdout_lines);
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

#[test]
fn ui_test_rar_feature_toggle_in_list_formats() {
    let (_dropper, dir) = testdir().unwrap();

    // Run and strip ANSI for stable checks.
    let raw = run_ouch("ouch list-formats", dir);
    let ansi_re = Regex::new(r"\x1b\[[0-9;]*[mK]").unwrap();

    // Lowercase so the regex is simple, and don't anchor to line starts because of box chars.
    let clean = ansi_re.replace_all(&raw, "").to_lowercase();

    // Match "rar" as a standalone token (surrounded by non-alphanumerics or boundaries).
    let rar_token = Regex::new(r"(?:^|[^a-z0-9])rar(?:[^a-z0-9]|$)").unwrap();

    if cfg!(feature = "unrar") {
        assert!(
            rar_token.is_match(&clean),
            "RAR should be shown in `list-formats` when the `unrar` feature is enabled.\n{clean}"
        );
    } else {
        assert!(
            !rar_token.is_match(&clean),
            "RAR should NOT be shown in `list-formats` when the `unrar` feature is disabled.\n{clean}"
        );
    }
}

#[test]
fn ui_test_rar_flag_parsing_respects_feature_toggle() {
    let (_dropper, dir) = testdir().unwrap();
    create_files_in(dir, &["a"]); // dummy input

    // Force the format via flag; we only care about the error message shape.
    let out = run_ouch("ouch decompress a --format rar", dir);

    let ansi_re = Regex::new(r"\x1b\[[0-9;]*[mK]").unwrap();
    let clean = ansi_re.replace_all(&out, "").to_string();

    if cfg!(feature = "unrar") {
        // With feature enabled, parsing should NOT reject 'rar'.
        assert!(
            !clean.contains("Unsupported extension 'rar'"),
            "With `unrar` enabled, '--format rar' must be recognized at parse time.\n{clean}"
        );
    } else {
        // Without feature, it must be rejected at parse time.
        assert!(
            clean.contains("Unsupported extension 'rar'"),
            "Without `unrar`, '--format rar' must be rejected at parse time.\n{clean}"
        );
    }
}

/// Concatenates `with_rar` or `without_rar` if the feature is toggled or not.
fn concat_snapshot_filename_rar_feature(name: &str) -> String {
    let suffix = if cfg!(feature = "unrar") {
        "with_rar"
    } else {
        "without_rar"
    };

    format!("{name}_{suffix}")
}
