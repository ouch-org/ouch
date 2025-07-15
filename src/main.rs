pub mod accessible;
pub mod archive;
pub mod check;
pub mod cli;
pub mod commands;
pub mod error;
pub mod extension;
pub mod list;
pub mod utils;

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

    // Get the output dir if specified, else use current dir
    //let working_dir = args.output_dir
    //    .clone()
    //    .unwrap_or_else(|| env::current_dir().unwrap_or_default());
    // restrict filesystem access to working_dir;
    // 1. working_dir is either the output_dir specified by the -d option or
    // 2. it is the temporary .tmp-ouch-XXXXXX directory that is renamed after decompression
    //
    //Case 1: Files are directly written to the output_directory, which may be created by ouch
    //      full landlock permissions granted inside the specified directory
    //Case 2: Files are written to the .tmp-ouch directory, requiring make_dir permissions on the
    //      parent (cwd) for renaming and full permissions within the tmp-ouch directory itself
    //
    // Since either the specified output directory is created if it did not exist, or the .ouch-tmp
    // directory is created in the current working directory, the parent directory of the target
    // directory requires LANDLOCK_ACCESS_FS_MAKE_DIR
    // expects either the .tmp-ouch-XXXXXX path or the specified output directory (-d option)
    //utils::landlock::init_sandbox(&working_dir);

    commands::run(args, skip_questions_positively, file_visibility_policy)
}
