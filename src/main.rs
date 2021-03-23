use std::{convert::TryFrom};

use colored::Colorize;

mod cli;
mod error;
mod extension;
mod file;
mod test;
mod evaluator;
mod utils;

mod compressors;
mod decompressors;

fn main() -> error::OuchResult<()>{
    let print_error = |err| {
        println!("{}: {}", "error".red(), err);
    };
    let matches = cli::get_matches();
    match cli::Command::try_from(matches) {
        Ok(command) => {
            match evaluator::Evaluator::evaluate(command) {
                Ok(_) => {},
                Err(err) => print_error(err)
            }
        }
        Err(err) => {
            print_error(err)
        }
    }
    
    Ok(())
}
