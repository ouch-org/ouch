use std::{convert::TryFrom, fs::File};

use cli::get_matches;

mod cli;
mod file;
mod extensions;
mod error;

fn main() {

    // Just testing

    // let args: Vec<String> = std::env::args().collect();

    // let file = std::fs::read(args[1].clone()).unwrap();

    // match niffler::sniff(Box::new(&file[..])) {
    //     Ok((reader, compression)) => {},
    //     Err(err) => {}
    // }
    
    // let (mut reader, compression) = niffler::sniff(Box::new(&file[..])).unwrap();

    // match compression {
    //     niffler::Format::Gzip => {}
    //     niffler::Format::Bzip => {}
    //     niffler::Format::Lzma => {}
    //     niffler::Format::No   => {}
    // }

    // let mut contents = String::new();
    // println!("Contents: {}", reader.read_to_string(&mut contents).unwrap());

    // dbg!(compression);

    let matches = get_matches();
    match cli::Command::try_from(matches) {
        Ok(vals) => { dbg!(vals); },
        Err(err) => {
            print!("{}\n", err);
        }
    }
    
}
