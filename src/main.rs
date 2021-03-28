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

use std::convert::TryFrom;

use error::{Error, Result};
use evaluator::Evaluator;

fn main() -> crate::Result<()> {
    let matches = cli::get_matches();
    let command = cli::Command::try_from(matches)?;
    Evaluator::evaluate(command)
}

// fn main() -> crate::Result<()> {
//     let dialog = dialogs::Confirmation::new("Do you want to overwrite 'FILE'?", Some("FILE"));

//     match dialog.ask(Some("file.tar.gz"))? {
//         true => println!("deleting"),
//         false => println!("keeping")
//     };

//     Ok(())
// }
