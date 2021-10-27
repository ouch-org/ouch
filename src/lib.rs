//! This library is meant to be published, just used internally by our binary crate at `main.rs`.
//!
//! A module shall be public only if:
//! 1. It's required by `main.rs`, or
//! 2. It's required by some integration tests at tests/ folder.

#![warn(missing_docs)]

// Public modules
pub mod archive;
pub mod cli;
pub mod commands;

// Private modules
mod dialogs;
mod error;
mod extension;
mod macros;
mod utils;

pub use error::{Error, Result};

/// The status code ouch has when an error is encountered
pub const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;
