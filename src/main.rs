mod cli;
mod compressors;
mod decompressors;
mod error;
mod evaluator;
mod extension;
mod file;
mod test;
mod utils;

use std::convert::TryFrom;

use error::{Error, Result};
use evaluator::Evaluator;

fn main() -> crate::Result<()> {
    let matches = cli::get_matches();
    let command = cli::Command::try_from(matches)?;
    Evaluator::evaluate(command)
}
