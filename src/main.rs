use std::convert::TryFrom;

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

// fn main() -> error::OuchResult<()> {

//     use tar::{Builder};
//     use walkdir::WalkDir;

//     let mut b = Builder::new(Vec::new());
    
//     for entry in WalkDir::new("src") {
//         let entry = entry?;
//         let mut file = std::fs::File::open(entry.path())?;
//         b.append_file(entry.path(), &mut file)?;
//     }

//     // let mut file = std::fs::File::open("Cargo.toml")?;
//     // b.append_file("Cargo.toml", &mut file)?;

//     let bytes = b.into_inner()?;

//     std::fs::write("daaaaamn.tar", bytes)?;

//     Ok(())
// }