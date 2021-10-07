//! This library is meant to be published, just used internally by our binary crate at `main.rs`.
//!
//! A module shall be public only if:
//! 1. It's required by `main.rs`, or
//! 2. It's required by some integration tests at tests/ folder.

#[macro_use]
mod macros;

// Public modules
pub mod cli;
pub mod commands;
pub mod oof;

// Private modules
pub mod archive;
mod dialogs;
mod error;
mod extension;
mod utils;

pub use error::{Error, Result};

use lazy_static::lazy_static;

/// The status code ouch has when an error is encountered
pub const EXIT_FAILURE: i32 = libc::EXIT_FAILURE;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");

lazy_static! {
    static ref NO_COLOR_IS_SET: bool = {
        use std::env;

        env::var("NO_COLOR").is_ok() || atty::isnt(atty::Stream::Stdout) || atty::isnt(atty::Stream::Stderr)
    };
}

fn help_command() {
    use utils::colors::*;

    println!(
        "\
{cyan}ouch{reset} - Obvious Unified Compression files Helper

{cyan}USAGE:{reset}
    {green}ouch decompress {magenta}<files...>{reset}             Decompresses files.

    {green}ouch compress {magenta}<files...> OUTPUT.EXT{reset}    Compresses files into {magenta}OUTPUT.EXT{reset},
                                           where {magenta}EXT{reset} must be a supported format.

{cyan}ALIASES:{reset}
    {green}d    decompress {reset}
    {green}c    compress {reset}

{cyan}FLAGS:{reset}
    {yellow}-h{white}, {yellow}--help{reset}    Display this help information.
    {yellow}-y{white}, {yellow}--yes{reset}     Skip overwrite questions.
    {yellow}-n{white}, {yellow}--no{reset}      Skip overwrite questions.
    {yellow}--version{reset}     Display version information.

{cyan}SPECIFIC FLAGS:{reset}
    {yellow}-o{reset}, {yellow}--output{reset} FOLDER_PATH    When decompressing, to decompress files to
                                another folder.

Visit https://github.com/ouch-org/ouch for more usage examples.",
        magenta = magenta(),
        white = white(),
        green = green(),
        yellow = yellow(),
        reset = reset(),
        cyan = cyan()
    );
}

#[inline]
fn version_command() {
    use utils::colors::*;
    println!("{green}ouch{reset} {}", crate::VERSION, green = green(), reset = reset());
}
