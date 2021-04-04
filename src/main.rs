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

fn main() {
    if let Err(err) = run() {
        println!("{}", err);
        std::process::exit(127);
    }
}

fn run() -> crate::Result<()> {
    let command = cli::parse_args_and_flags()?;
    Evaluator::evaluate(command)
}
