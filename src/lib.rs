//! This library is just meant to supply needs for the `ouch` binary crate.

#![warn(missing_docs)]

// Macros should be declared before
pub mod macros;

pub mod archive;
pub mod cli;
pub mod commands;
pub mod error;
pub mod extension;
pub mod list;
pub mod utils;

/// CLI argparsing definitions, using `clap`.
pub mod opts;

pub use error::{Error, Result};
pub use opts::{Opts, Subcommand};
pub use utils::QuestionPolicy;

/// The status code returned from `ouch` on error
pub const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;
