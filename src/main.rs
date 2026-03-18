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

use cli::CliArgs;

pub use self::error::{Error, FinalError, Result};
use self::utils::{
    QuestionAction, QuestionPolicy,
    logger::{shutdown_logger_and_wait, spawn_logger_thread},
};

const BUFFER_CAPACITY: usize = 1024 * 32;
const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;

fn main() {
    spawn_logger_thread();
    let result = run();
    shutdown_logger_and_wait();

    match result {
        Ok(_) => {}
        Err(Error::UserCancelled) => {
            eprintln!("User cancelled.");
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(EXIT_FAILURE);
        }
    }
}

fn run() -> Result<()> {
    let (args, skip_questions_positively, file_visibility_policy) = CliArgs::parse_and_validate_args()?;
    commands::run(args, skip_questions_positively, file_visibility_policy)
}
