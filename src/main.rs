mod cli;
mod compressors;
mod decompressors;
mod error;
#[allow(dead_code)]
mod evaluator;
mod extension;
mod file;
mod test;
mod utils;

#[allow(unreachable_code, unused_variables)]
use std::env;

use error::{Error, Result};
use evaluator::Evaluator;

fn main() {
    let command = cli::Command::from(env::args_os());
    // match command {
    //     Command::ShowHelp => {}
    // }

    // Evaluator::evaluate(command)
}
