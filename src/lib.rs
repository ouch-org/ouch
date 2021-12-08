//! This library is just meant to supply needs for the `ouch` binary crate.

#![warn(missing_docs)]
// Bare URLs in doc comments are not a problem since this project is primarily
// used as a binary. Since `clap` doesn't remove URL markup in it's help output,
// we don't mark them as URLs. This suppresses the relevant rustdoc warning:
#![allow(rustdoc::bare_urls)]
// Useful to detect broken symlinks when compressing. (So we can safely ignore them)
#![feature(is_symlink)]

// Macros should be declared before
pub mod macros;

pub mod archive;
pub mod cli;
pub mod commands;
pub mod error;
pub mod extension;
pub mod list;
pub mod progress;
pub mod utils;

/// CLI argparsing definitions, using `clap`.
pub mod opts;

pub use error::{Error, Result};
pub use opts::{Opts, Subcommand};
pub use utils::QuestionPolicy;

/// The status code returned from `ouch` on error
pub const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;
