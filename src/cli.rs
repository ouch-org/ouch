use std::{
    convert::TryFrom,
    fs,
    path::{Path, PathBuf},
    vec::Vec,
};

use clap::{Arg, Values};
use colored::Colorize;

use crate::{extension::Extension, file::File};

#[derive(PartialEq, Eq, Debug)]
pub enum CommandKind {
    Compression(
        /// Files to be compressed
        Vec<PathBuf>,
    ),
    Decompression(
        /// Files to be decompressed and their extensions
        Vec<File>,
    ),
}

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum Flags {
    // No flags supplied
    None,
    // Flag -y, --yes supplied
    AlwaysYes,
    // Flag -n, --no supplied
    AlwaysNo,
}

#[derive(PartialEq, Eq, Debug)]
pub struct Command {
    pub kind: CommandKind,
    pub output: Option<File>,
}

pub fn clap_app<'a, 'b>() -> clap::App<'a, 'b> {
    clap::App::new("ouch")
        .version("0.1.4")
        .about("ouch is a unified compression & decompression utility")
        .after_help(
"ouch infers what to based on the extensions of the input files and output file received.
Examples: `ouch -i movies.tar.gz classes.zip -o Videos/` in order to decompress files into a folder.
          `ouch -i headers/ sources/ Makefile -o my-project.tar.gz`
          `ouch -i image{1..50}.jpeg -o images.zip`
Please relate any issues or contribute at https://github.com/vrmiguel/ouch")
        .author("Vinícius R. Miguel")
        .help_message("Displays this message and exits")
        .settings(&[
            clap::AppSettings::ColoredHelp,
            clap::AppSettings::ArgRequiredElseHelp,
        ])
        .arg(
            Arg::with_name("input")
                .required(true)
                .multiple(true)
                .long("input")
                .short("i")
                .help("The input files or directories.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                // --output/-o not required when output can be inferred from the input files
                .required(false)
                .multiple(false)
                .long("output")
                .short("o")
                .help("The output directory or compressed file.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("yes")
                .required(false)
                .multiple(false)
                .long("yes")
                .short("y")
                .help("Says yes to all confirmation dialogs.")
                .conflicts_with("no")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("no")
                .required(false)
                .multiple(false)
                .long("no")
                .short("n")
                .help("Says no to all confirmation dialogs.")
                .conflicts_with("yes")
                .takes_value(false),
        )
}

pub fn get_matches() -> clap::ArgMatches<'static> {
    clap_app().get_matches()
}

pub fn parse_matches(matches: clap::ArgMatches<'static>) -> crate::Result<(Command, Flags)> {
    let flag = match (matches.is_present("yes"), matches.is_present("no")) {
        (true, true) => unreachable!(),
        (true, _) => Flags::AlwaysYes,
        (_, true) => Flags::AlwaysNo,
        (_, _) => Flags::None,
    };

    Ok((Command::try_from(matches)?, flag))
}

impl TryFrom<clap::ArgMatches<'static>> for Command {
    type Error = crate::Error;

    fn try_from(matches: clap::ArgMatches<'static>) -> crate::Result<Command> {
        let process_decompressible_input = |input_files: Values| {
            let input_files =
                input_files.map(|filename| (Path::new(filename), Extension::new(filename)));

            for file in input_files.clone() {
                match file {
                    (filename, Ok(_)) => {
                        let path = Path::new(filename);
                        if !path.exists() {
                            return Err(crate::Error::FileNotFound(filename.into()));
                        }
                    }
                    (filename, Err(_)) => {
                        return Err(crate::Error::InputsMustHaveBeenDecompressible(
                            filename.into(),
                        ));
                    }
                }
            }

            Ok(input_files
                .map(|(filename, extension)| {
                    (fs::canonicalize(filename).unwrap(), extension.unwrap())
                })
                .map(File::from)
                .collect::<Vec<_>>())
        };

        // Possibilities:
        //   * Case 1: output not supplied, therefore try to infer output by checking if all input files are decompressible
        //   * Case 2: output supplied

        let output_was_supplied = matches.is_present("output");

        let input_files = matches.values_of("input").unwrap(); // Safe to unwrap since input is an obligatory argument

        if output_was_supplied {
            let output_file = matches.value_of("output").unwrap(); // Safe unwrap since we've established that output was supplied

            let output_file_extension = Extension::new(output_file);

            let output_is_compressible = output_file_extension.is_ok();
            if output_is_compressible {
                // The supplied output is compressible, so we'll compress our inputs to it

                let canonical_paths = input_files.clone().map(Path::new).map(fs::canonicalize);
                for (filename, canonical_path) in input_files.zip(canonical_paths.clone()) {
                    if let Err(err) = canonical_path {
                        let path = PathBuf::from(filename);
                        if !path.exists() {
                            return Err(crate::Error::FileNotFound(path));
                        }

                        eprintln!("{} {}", "[ERROR]".red(), err);
                        return Err(crate::Error::IoError);
                    }
                }

                let input_files = canonical_paths.map(Result::unwrap).collect();

                Ok(Command {
                    kind: CommandKind::Compression(input_files),
                    output: Some(File {
                        path: output_file.into(),
                        contents_in_memory: None,
                        // extension: output_file_extension.ok(),
                        extension: Some(output_file_extension.unwrap()),
                    }),
                })
            } else {
                // Output not supplied
                // Checking if input files are decompressible

                let input_files = process_decompressible_input(input_files)?;

                Ok(Command {
                    kind: CommandKind::Decompression(input_files),
                    output: Some(File {
                        path: output_file.into(),
                        contents_in_memory: None,
                        extension: None,
                    }),
                })
            }
        } else {
            // else: output file not supplied
            // Case 1: all input files are decompressible
            // Case 2: error
            let input_files = process_decompressible_input(input_files)?;

            Ok(Command {
                kind: CommandKind::Decompression(input_files),
                output: None,
            })
        }
    }
}
