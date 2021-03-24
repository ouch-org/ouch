use std::{convert::TryFrom, io::Write};

use colored::Colorize;
use walkdir::WalkDir;

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

// fn main() {
//     use zip::ZipWriter;

//     let buf = vec![];
//     let mut writer = zip::ZipWriter::new(std::io::Cursor::new(buf));

//     let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);


//     for entry in WalkDir::new("src/compressors/compressor.rs") {
//         let entry = entry.unwrap();
//         let entry_path = entry.path().clone();
//         if entry_path.is_dir() {
//             continue;
//         }
//         writer.start_file(entry_path.to_string_lossy(), options).unwrap();
//         let file_bytes = std::fs::read(entry.path()).unwrap();
//         writer.write(&*file_bytes).unwrap();
//     }

//     let bytes = writer.finish().unwrap();

//     std::fs::write("mainmain.rar", bytes.into_inner()).unwrap();
// }