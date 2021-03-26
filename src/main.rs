use std::convert::TryFrom;

mod cli;
mod error;
mod evaluator;
mod extension;
mod file;
mod test;
mod utils;

mod compressors;
mod decompressors;

use evaluator::Evaluator;

fn main() -> error::OuchResult<()> {
    let print_error = |err| {
        println!("{}", err);
        err
    };

    let matches = cli::get_matches();
    let command = match cli::Command::try_from(matches) {
        Ok(command) => command,
        Err(err) => return Err(print_error(err))
    };

    match Evaluator::evaluate(command) {
        Ok(_) => Ok(()),
        Err(err) => Err(print_error(err))
    }
}
