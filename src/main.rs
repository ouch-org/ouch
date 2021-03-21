use std::{convert::TryFrom, fs::File, path::{Path, PathBuf}};

use colored::Colorize;
use error::{Error, OuchResult};
use tar::Archive;

mod cli;
mod error;
mod extension;
mod file;
mod test;
mod evaluator;

mod decompressors;

fn main() -> OuchResult<()>{
    let matches = cli::get_matches();
    match cli::Command::try_from(matches) {
        Ok(command) => {
            let mut eval = evaluator::Evaluator::new(command);
            eval.evaluate()?;
        }
        Err(err) => {
            print!("{}: {}\n", "error".red(), err);
        }
    }
    Ok(())
}
