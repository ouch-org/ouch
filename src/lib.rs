//! This library is meant to be published, just used internally by our binary crate at `main.rs`.
//!
//! A module shall be public only if:
//! 1. It's required by `main.rs`, or
//! 2. It's required by some integration tests at tests/ folder.

#![warn(missing_docs)]

// Macros should be declared before
pub mod macros;

pub mod archive;
pub mod cli;
pub mod commands;
pub mod dialogs;
pub mod error;
pub mod extension;
pub mod list;
pub mod utils;

/// CLI configuration step, uses definitions from `opts.rs`, also used to treat some inputs.
pub mod opts;

pub use error::{Error, Result};
pub use opts::{Opts, Subcommand};
pub use utils::QuestionPolicy;

/// The status code ouch has when an error is encountered
pub const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;
