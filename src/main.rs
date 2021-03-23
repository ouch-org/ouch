use std::{convert::TryFrom, fs, path::{Path, PathBuf}};

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

use compressors::{CompressionResult, Compressor, TarCompressor};
use error::OuchResult;
use file::File;

fn main() -> OuchResult<()>{
    // let print_error = |err| {
    //     println!("{}: {}", "error".red(), err);
    // };
    // let matches = cli::get_matches();
    // match cli::Command::try_from(matches) {
    //     Ok(command) => {
    //         match evaluator::Evaluator::evaluate(command) {
    //             Ok(_) => {},
    //             Err(err) => print_error(err)
    //         }
    //     }
    //     Err(err) => {
    //         print_error(err)
    //     }
    // }

    let compressor = TarCompressor {};

    let file = File {
        path: PathBuf::from("target"),
        contents: None,
        extension: None,
    };

    let ok = compressor.compress(vec![file])?;

    let ok = match ok {
        CompressionResult::TarArchive(bytes) => bytes,
        _ => unreachable!()
    };

    fs::write(Path::new("great.tar"), ok)?;

    Ok(())
}
