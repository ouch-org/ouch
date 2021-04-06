// Public modules
pub mod cli;
pub mod evaluator;

// Private modules
mod bytes;
mod compressors;
mod decompressors;
mod dialogs;
mod error;
mod extension;
mod file;
mod test;
mod utils;

const VERSION: &str = "0.1.5";

pub use error::{Error, Result};
