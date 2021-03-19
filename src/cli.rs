use std::{convert::TryFrom, ffi::OsStr, path::PathBuf, vec::Vec};

use clap::{Arg};
use colored::Colorize;

use crate::error;
use crate::extensions::CompressionExtension;
use crate::file::File;

#[derive(Debug)]
pub enum CommandType {
    Compression(
        // Files to be compressed
        Vec<PathBuf>,
    ),
    Decompression(
        // Files to be decompressed and their extensions
        Vec<(PathBuf, CompressionExtension)>,
    ),
}

#[derive(Debug)]
pub struct Command {
    pub command_type: CommandType,
    pub output: Option<File>,
}

pub fn get_matches() -> clap::ArgMatches<'static> {
    clap::App::new("ouch")
        .version("0.1.0")
        .about("ouch is a unified compression & decompression utility")
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
                .help("Input files (TODO description)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("output")
                // --output/-o not required when output can be inferred from the input files
                .required(false)
                .multiple(false)
                .long("output")
                .short("o")
                .help("Output file (TODO description)")
                .takes_value(true),
        )
        .get_matches()
}

// holy spaghetti code
impl TryFrom<clap::ArgMatches<'static>> for Command {

    type Error = error::Error;

        fn try_from(matches: clap::ArgMatches<'static>) -> error::OuchResult<Command> {
            // Possibilities:
            //   * Case 1: output not supplied, therefore try to infer output by checking if all input files are decompressable
            //   * Case 2: output supplied
            
            let output_was_supplied = matches.is_present("output");

            if output_was_supplied {
                let output_file = matches
                    .value_of("output")
                    .unwrap(); // Safe unwrap since we've established that output was supplied

                let input_files = matches
                    .values_of("input")
                    .unwrap(); // Safe to unwrap since input is an obligatory argument
                    // .map(PathBuf::from)
                    // .collect();
                

                let output_file_extension = CompressionExtension::try_from(output_file);
                let output_is_compressable = output_file_extension.is_ok();
                if output_is_compressable {
                    println!("{}: trying to compress input files into '{}'", "info".yellow(), output_file);

                    let input_files = input_files.map(PathBuf::from).collect();

                    return Ok(
                        Command {
                            command_type: CommandType::Compression(input_files),
                            output: Some(File::WithExtension(
                              (output_file.into(), output_file_extension.unwrap())  
                            ))
                        }
                    );

                } 
                else {
                    // Checking if input files are decompressable
                    let input_files = input_files
                        .map(|filename| (filename, CompressionExtension::try_from(filename)));                        
                    
                    for file in input_files.clone() {
                        if let (file, Err(_)) = file {
                            eprintln!("{}: file '{}' is not decompressable.", "error".red(), file);
                            return Err(error::Error::InputsMustHaveBeenDecompressable(file.into()));
                        }
                    }

                    let input_files = 
                        input_files
                        .map(|(filename, extension)| 
                            (PathBuf::from(filename), extension.unwrap())
                        )
                        .collect();

                    println!("{}: attempting to decompress input files into {}", "info".yellow(), output_file);
                    return Ok(
                        Command {
                            command_type: CommandType::Decompression(input_files),
                            output: Some(File::WithoutExtension(output_file.into()))
                        }
                    );
                }
            } else {
                // BIG TODO
                Err(error::Error::MissingExtensionError("placeholder result".into()))
            }
        }
}

// impl TryFrom<clap::ArgMatches<'static>> for ArgValues {
//     type Error = error::OuchError;
//     fn try_from(matches: clap::ArgMatches<'static>) -> error::OuchResult<ArgValues> {
//         // Case 1: -o was set
//         //   Case 1.1: -o was set and has a (supported) compression file extension
//         //   |--> Compress all input files into the supplied output file (no extension checks on inputs)
//         //   Case 1.2: -o was set and is not a supported expression
//         // Case 2: -o was not set
//         //   Case 2.1: -o was not set and all input files are (supported) compression file extensions
//         //   |--> Decompress input files into inferred filenames or directories
//         //   Case 2.2: -o was not set and not all input files are (supported) compression file extensions
//         //   |--> Issue an error

//         let inputs = matches
//             .values_of("input")
//             .unwrap() // Safe to unwrap since this is a required argument
//             .map(|input: &str| {
//                 (
//                     PathBuf::from(input),
//                     CompressionExtension::try_from(input).ok(),
//                 )
//             });

//         let output_was_supplied = matches.is_present("output");
//         let inputs_are_compressed_files = inputs.clone().all(|(_, ext)| ext.is_some());

//         match (output_was_supplied, inputs_are_compressed_files) {
//             (true, true) => {
//                 // -o was set and inputs are all valid compressed files

//                 let output = matches.value_of("output").unwrap();
//                 let output = PathBuf::from(output);
//                 match CompressionExtension::try_from(&output) {
//                     Ok(ext) => {
//                         // If the output file is a valid compressed file, then we compress the input files into it
//                         Ok(Self {
//                             command_type: CommandType::Compress(
//                                 inputs.map(|(path, _)| path).collect(),
//                             ),
//                             output: Some((output, ext)),
//                         })
//                     }
//                     Err(_) => {
//                         // If the output file is not a compressed file, then we decompress the input files into it
//                         Ok(Self {
//                             command_type: CommandType::Decompress(
//                                 inputs.map(|(path, ext)| (path, ext.unwrap())).collect(),
//                             ),
//                             output: Some((output, CompressionExtension::NotCompressed)),
//                         })
//                     }
//                 }
//             }
//             (true, false) => {
//                 // -o was set and inputs are not (all) valid compressed files
//                 let output_str = matches.value_of("output").unwrap();
//                 let output = PathBuf::from(output_str);
//                 let output_ext = match CompressionExtension::try_from(&output) {
//                     Ok(ext) => ext,
//                     Err(_) => {
//                         return Err(error::OuchError::MissingExtensionError(output_str.into()));
//                     }
//                 };

//                 Ok(Self {
//                     command_type: CommandType::Compress(inputs.map(|(path, _)| path).collect()),
//                     output: Some((output, output_ext)),
//                 })
//             }
//             (false, true) => {
//                 // Case 2.1: -o was not set and all input files are (supported) compression file extensions
//                 Ok(Self {
//                     command_type: CommandType::Decompress(
//                         inputs.map(|(path, ext)| (path, ext.unwrap())).collect(),
//                     ),
//                     output: None,
//                 })
//             }
//             (false, false) => {
//                 // Case 2.2: -o was not set and not all input files are not (supported) compression file extensions
//                 Err(error::OuchError::InvalidInput)
//             }
//         }
//     }
// }