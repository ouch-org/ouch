#[allow(dead_code, unused_variables)]
mod cli;
mod compressors;
mod decompressors;
mod error;
#[allow(dead_code, unused_variables)]
mod evaluator;
mod extension;
mod file;
mod test;
mod utils;

use std::{env, result::Result as StdResult};

use error::{Error, Result};
// use evaluator::Evaluator;

fn main() -> StdResult<(), cli::ArgParsingError> {
    let _command = cli::try_arg_parsing(env::args_os())?;
    // match command {
    //     Command::ShowHelp => {}
    // }

    // Evaluator::evaluate(command)
    Ok(())
}
