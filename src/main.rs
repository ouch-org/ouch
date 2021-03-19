use std::convert::TryFrom;

mod cli;
mod error;
mod extensions;
mod file;
mod test;
mod evaluator;

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

    let matches = cli::get_matches();
    match cli::Command::try_from(matches) {
        Ok(vals) => {
            dbg!(vals);
        }
        Err(err) => {
            print!("{}\n", err);
        }
    }
}
