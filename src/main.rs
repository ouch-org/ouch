use std::{convert::TryFrom, ffi::OsStr, path::Path};

use colored::Colorize;

mod cli;
mod error;
mod extensions;
mod file;
mod test;
mod evaluator;

fn main() {
    // let matches = cli::get_matches();
    // match cli::Command::try_from(matches) {
    //     Ok(command) => {
    //         let mut eval = evaluator::Evaluator::new(command);
    //         eval.evaluate();
    //     }
    //     Err(err) => {
    //         print!("{}: {}\n", "error".red(), err);
    //     }
    // }

    dbg!(extensions::get_extension_from_filename("file"));
    // dbg!(get_extension_from_filename("file.zip"));
}
