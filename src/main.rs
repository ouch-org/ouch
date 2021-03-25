use std::convert::TryFrom;

mod cli;
mod error;
mod evaluator;
mod extension;
mod file;
mod test;
mod utils;

mod compressors;
mod decompressors;

fn main() -> error::OuchResult<()> {
    let print_error = |err| {
        println!("{}", err);
    };

    let matches = cli::get_matches();
    cli::Command::try_from(matches)
        .map(|command| evaluator::Evaluator::evaluate(command).unwrap_or_else(print_error))
        .unwrap_or_else(print_error);

    Ok(())
}

// fn main() -> error::OuchResult<()> {
//     use tar::{Builder};
//     use walkdir::WalkDir;
//
//     let mut b = Builder::new(Vec::new());
//
//     for entry in WalkDir::new("src") {
//         let entry = entry?;
//         let mut file = std::fs::File::open(entry.path())?;
//         b.append_file(entry.path(), &mut file)?;
//     }
//
//     // let mut file = std::fs::File::open("Cargo.toml")?;
//     // b.append_file("Cargo.toml", &mut file)?;
//
//     let bytes = b.into_inner()?;
//
//     std::fs::write("daaaaamn.tar", bytes)?;
//
//     Ok(())
// }
