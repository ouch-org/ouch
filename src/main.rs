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
//     let bytes = fs::read("extension.tar.lzma")?;

//     let mut ret = vec![];

//     xz2::read::XzDecoder::new_multi_decoder(&*bytes)
//         .read_to_end(&mut ret)
//         .unwrap();
    

//     fs::write("extension.tar", &*bytes).unwrap();

//     Ok(())
// }