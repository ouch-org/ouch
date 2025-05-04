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
use once_cell::sync::Lazy;

use self::{
    error::{Error, Result},
    utils::{
        logger::{shutdown_logger_and_wait, spawn_logger_thread},
        QuestionAction, QuestionPolicy,
    },
};

// Used in BufReader and BufWriter to perform less syscalls
const BUFFER_CAPACITY: usize = 1024 * 32;

/// Current directory or empty directory
static CURRENT_DIRECTORY: Lazy<PathBuf> = Lazy::new(|| env::current_dir().unwrap_or_default());

/// The status code returned from `ouch` on error
pub const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;

fn main() {
    spawn_logger_thread();

    let result = run();
    shutdown_logger_and_wait();

    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(EXIT_FAILURE);
    }
}

fn run() -> Result<()> {
    let (args, skip_questions_positively, file_visibility_policy) = CliArgs::parse_and_validate_args()?;

    // check args if case A: "decompress -d <outputdir>" or case B: "decompress -r" is used
    //if true
    //Case A:
    // write_dirs = outputdir
    //Case B:
    // write_dir = inputdir

    //init_sandbox( write_dirs );
    init_sandbox();

    commands::run(args, skip_questions_positively, file_visibility_policy)
}

// init_sandbox( write_dirs
fn init_sandbox() {

    //if empty write_dirs
    //{
        //restrict write permissions to the current workign directory
        let working_dir = get_current_working_dir().expect("Cannot get current working dir");
        let path_str = working_dir.to_str().expect("Cannot convert path");

    //}
    //else
        //path_str = write_dirs;
        let status = sandbox::restrict_paths(&[path_str]).expect("failed to build the ruleset");
    //}

    // todos:
    // check status and report error or warning if landlock restriction failed
    // add os detection to encapsulate this feature to be executed on linux only
    // add implementation for other OS
}

fn get_current_working_dir() -> std::io::Result<PathBuf> {
    env::current_dir()
}
