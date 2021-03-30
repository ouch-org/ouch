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
    let matches = cli::get_matches();
    let (command, flags) = cli::parse_matches(matches)?;
    Evaluator::evaluate(command, flags)
}
