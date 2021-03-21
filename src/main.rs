use std::{convert::TryFrom};

use colored::Colorize;

mod cli;
mod error;
mod extension;
mod file;
mod test;
mod evaluator;

fn main() {
    let matches = cli::get_matches();
    match cli::Command::try_from(matches) {
        Ok(command) => {
            let mut eval = evaluator::Evaluator::new(command);
            eval.evaluate();
        }
        Err(err) => {
            print!("{}: {}\n", "error".red(), err);
        }
    }
}
