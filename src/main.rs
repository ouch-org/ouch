use std::{convert::TryFrom, fs::File, path::{Path, PathBuf}};

use colored::Colorize;
use tar::Archive;

mod cli;
mod error;
mod extension;
mod file;
mod test;
mod evaluator;

mod decompressors;

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


    // Testing tar unarchival
    let file = File::open("ouch.tar").unwrap();
    let mut a = Archive::new(file);

    for file in a.entries().unwrap() {
        // Make sure there wasn't an I/O error
        let mut file = file.unwrap();

        let path = if let Ok(path) = file.path() {
            path
        } else {
            continue;
        };
        let name = path.to_string_lossy();

        file.unpack(format!("{}", name)).unwrap();
    }
}
