// Public modules
pub mod cli;
pub mod evaluator;

// Private modules
mod compressors;
mod decompressors;
mod dialogs;
mod error;
mod extension;
mod file;
mod utils;

const VERSION: &str = "0.1.5";

pub use error::{Error, Result};
