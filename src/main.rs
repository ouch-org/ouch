pub mod accessible;
pub mod archive;
pub mod check;
pub mod cli;
pub mod commands;
pub mod error;
pub mod extension;
pub mod list;
pub mod utils;
pub mod sandbox;

use std::{env, path::PathBuf};

use cli::CliArgs;
use error::{Error, Result};
use once_cell::sync::Lazy;
use utils::{QuestionAction, QuestionPolicy};

use crate::utils::logger::spawn_logger_thread;

// Used in BufReader and BufWriter to perform less syscalls
const BUFFER_CAPACITY: usize = 1024 * 32;

/// Current directory or empty directory
static CURRENT_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| env::current_dir().unwrap_or_default());

/// The status code returned from `ouch` on error
pub const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;

fn main() {
    let handler = spawn_logger_thread();

    //restrict write permissions to the current workign directory
    let working_dir = get_current_working_dir().expect("Cannot get current working dir");
    let path_str = working_dir.to_str().expect("Cannot convert path");
    let status = sandbox::restrict_paths(&[path_str]).expect("failed to build the ruleset");

    //todo: check status and report error or warnign if landlock restriction failed

    let result = run();
    handler.shutdown_and_wait();

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(EXIT_FAILURE);
    }
}

fn run() -> Result<()> {
    let (args, skip_questions_positively, file_visibility_policy) = CliArgs::parse_and_validate_args()?;
    commands::run(args, skip_questions_positively, file_visibility_policy)
}

fn get_current_working_dir() -> std::io::Result<PathBuf> {
    env::current_dir()
}
