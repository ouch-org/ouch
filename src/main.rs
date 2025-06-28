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
use std::path::Path;

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

    // Get the output dir if specified, else use current dir
    let working_dir = args.output_dir
        .clone()
        .unwrap_or_else(|| env::current_dir().unwrap_or_default());

    // restrict filesystem access to working_dir;
    init_sandbox(&working_dir);

    commands::run(args, skip_questions_positively, file_visibility_policy)
}

fn init_sandbox(allowed_dir: &Path) {

    if std::env::var("CI").is_ok() {
       return;
    }


    if utils::landlock_support::is_landlock_supported() {

        let path_str = allowed_dir.to_str().expect("Cannot convert path");
        match sandbox::restrict_paths(&[path_str]) {
            Ok(status) => {
                //check
            }
            Err(e) => {
                //log warning
                std::process::exit(EXIT_FAILURE);
            }
        }
    } else {
//        warn!("Landlock is NOT supported on this platform or kernel (<5.19).");
    }

}

