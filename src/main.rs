pub mod accessible;
pub mod archive;
pub mod check;
pub mod cli;
pub mod commands;
pub mod error;
pub mod extension;
pub mod list;
pub mod non_archive;
pub mod utils;

use std::{env, path::PathBuf, sync::LazyLock};

use cli::CliArgs;

pub use self::error::{Error, FinalError, Result};
use self::utils::{
    QuestionAction, QuestionPolicy,
    logger::{shutdown_logger_and_wait, spawn_logger_thread},
};

const BUFFER_CAPACITY: usize = 1024 * 32;
pub const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;

/// Current directory, canonicalized for consistent path comparisons across platforms
static INITIAL_CURRENT_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    let Ok(dir) = env::current_dir() else {
        panic!("can't read current directory");
    };

    let Ok(dir) = utils::canonicalize(&dir) else {
        panic!("can't canonicalize current directory");
    };

    dir
});

fn main() {
    force_lazy_locks_to_load();
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
    commands::run(args, skip_questions_positively, file_visibility_policy)
}

fn force_lazy_locks_to_load() {
    LazyLock::force(&INITIAL_CURRENT_DIR);
}
