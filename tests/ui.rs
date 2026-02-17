/// Snapshot tests for Ouch's output.
///
/// See CONTRIBUTING.md for a brief guide on how to use [`insta`] for these tests.
/// [`insta`]: https://docs.rs/insta
#[macro_use]
mod utils;

use std::{ffi::OsStr, io, path::Path, process::Output};

use insta::{assert_snapshot as ui, Settings};
use regex::Regex;

use crate::utils::create_files_in;

fn testdir() -> io::Result<(tempfile::TempDir, &'static Path)> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().to_path_buf().into_boxed_path();
    Ok((dir, Box::leak(path)))
}

fn run_ouch(argv: &str, dir: &Path) -> String {
    run_ouch_with_stdin(argv, dir, None)
}

fn run_ouch_with_stdin(argv: &str, dir: &Path, stdin: Option<&str>) -> String {
    let mut command = utils::cargo_bin();

    if let Some(stdin) = stdin {
        command.write_stdin(stdin);
    }

    let output = command
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
    // Use [^\s"]* instead of \S* to avoid matching quote characters
    let slashes = r"(/|\\(\\)?)";
    let re = Regex::new(&format!(r#"[^\s"]*{slashes}{dir_name}{slashes}"#)).unwrap();
    re.replace_all(text, "<TMP_DIR>/").into()
}

fn output_to_string(output: Output) -> String {
    String::from_utf8(output.stdout).unwrap() + std::str::from_utf8(&output.stderr).unwrap()
}

/// Filter necessary for redactions/transformations, so the snapshot matches even
/// if the output has some randomness
fn insta_filter_settings() -> insta::Settings {
    let mut settings = Settings::new();
    // Sizes can change slightly between runs.
    settings.add_filter(r"\s+\b[[:xdigit:]]+\.[[:xdigit:]]+ (  |ki|Mi|Gi|Ti)B\b", " [SIZE]");
    // .exe shows up for Windows but not for Linux
    settings.add_filter(r"(Usage:.*\b)ouch(\.exe)?\b", "${1}[OUCH_BIN]");
    // Windows paths use `\` instead of `/`
    settings.add_filter(r"\\", "/");
    settings
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

    insta_filter_settings().bind(|| {
        ui!(run_ouch("ouch compress input output1 --format tar.gz", dir),);
        ui!(run_ouch("ouch compress input output2 --format .tar.gz", dir),);
    });
}

#[test]
fn ui_test_ok_compress() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    create_files_in(dir, &["input"]);

    insta_filter_settings().bind(|| {
        ui!(run_ouch("ouch compress input output.zip", dir));
        ui!(run_ouch("ouch compress input output.gz", dir));
    });
}

#[test]
fn ui_test_ok_decompress() {
    let (_dropper, dir) = testdir().unwrap();

    // prepare
    create_files_in(dir, &["input"]);
    run_ouch("ouch compress input output.zst", dir);

    insta_filter_settings().bind(|| {
        ui!(run_ouch("ouch decompress output.zst", dir));
    });
}

#[test]
fn ui_test_ok_decompress_multiple_files() {
    let (_dropper, dir) = testdir().unwrap();

    let inputs_dir = dir.join("input");
    std::fs::create_dir(&inputs_dir).unwrap();

    let output_dir = dir.join("output");
    std::fs::create_dir(&output_dir).unwrap();

    // prepare
    create_files_in(&inputs_dir, &["input", "input2", "input3"]);

    let compress_command = format!("ouch compress {} output.tar.zst", inputs_dir.display());
    run_ouch(&compress_command, dir);

    let decompress_command = format!("ouch decompress output.tar.zst --dir {}", output_dir.display());

    insta_filter_settings().bind(|| {
        let stdout = run_ouch(&decompress_command, dir);

        let mut lines: Vec<_> = stdout.lines().collect();
        lines.sort();
        ui!(lines.join("\n"));
    });
}

#[test]
fn ui_test_usage_help_flag() {
    insta_filter_settings().bind(|| {
        ui!(output_to_string(ouch!("--help")));
        ui!(output_to_string(ouch!("-h")));
    });
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

#[test]
fn ui_test_decompress_with_unknown_extension_shows_output_path() {
    let (_dropper, dir) = testdir().unwrap();

    create_files_in(dir, &["input.txt"]);

    // Compress with zst
    run_ouch("ouch compress input.txt compressed.zst", dir);

    // Rename to unknown extension
    std::fs::rename(dir.join("compressed.zst"), dir.join("file.unknown")).unwrap();

    std::fs::create_dir(dir.join("output")).unwrap();

    // Decompress with stdin input to confirm format detection
    let output = utils::cargo_bin()
        .args(["decompress", "file.unknown", "--dir", "output"])
        .current_dir(dir)
        .write_stdin("y\n")
        .output()
        .unwrap();

    insta_filter_settings().bind(|| {
        ui!(redact_paths(&output_to_string(output), dir));
    });
}

#[test]
fn ui_test_err_decompress_output_file_exists() {
    let (_dropper, dir) = testdir().unwrap();

    // Create a file "out" that will conflict with decompression output
    create_files_in(dir, &["input", "out"]);

    run_ouch("ouch compress input out.gz", dir);

    // Try to decompress out.gz, which would extract to "out" (already exists)
    // Answer "n" to the overwrite prompt
    ui!(run_ouch_with_stdin("ouch decompress out.gz", dir, Some("n\n")));
}
