mod bytes;
mod cli;
mod compressors;
mod decompressors;
mod dialogs;
mod error;
mod evaluator;
mod extension;
mod file;
mod test;
mod utils;

use error::{Error, Result};
use evaluator::Evaluator;

use crate::cli::ParsedArgs;

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        std::process::exit(127);
    }
}

fn run() -> crate::Result<()> {
    let ParsedArgs { command, flags } = cli::parse_args()?;
    debug!(&command);
    Evaluator::evaluate(command, &flags)
}
