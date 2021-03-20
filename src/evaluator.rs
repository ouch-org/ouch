use std::{convert::TryFrom, path::{PathBuf}};

use colored::Colorize;

use crate::{cli::{Command, CommandKind}, error, extensions::CompressionFormat, file::File};

pub struct Evaluator {   
    command: Command,
    // verbosity: Verbosity
}

impl Evaluator {
    pub fn new(command: Command) -> Self {
        Self {
            command
        }
    }

    fn handle_compression(files_to_compress: &[PathBuf], output_file: &Option<File>) {
        
    }

    fn decompress_file(mut filename: &PathBuf, mut extension: CompressionFormat, output_file: &Option<File>) -> error::OuchResult<()> {
        loop {
            println!("{}: attempting to decompress '{:?}'", "ouch".bright_blue(), filename);
        }
    }

    fn handle_decompression(files_to_decompress: &[(PathBuf, CompressionFormat)], output_file: &Option<File>) {
        for (filename, extension) in files_to_decompress {
            // println!("file: {:?}, extension: {:?}", filename, extension);

            // TODO: actually decompress anything ;-;

            // Once decompressed, check if the file can be decompressed further
            // e.g.: "foobar.tar.gz" -> "foobar.tar"

            

            let filename: &PathBuf = &filename.as_path().file_stem().unwrap().into();
            match CompressionFormat::try_from(filename) {
                Ok(extension) => {
                    println!("{}: attempting to decompress {:?}, ext: {:?}", "info".yellow(), filename, extension);
                },
                Err(err) => {
                    continue;
                }
            }
        }
    }

    pub fn evaluate(&mut self) {
        match &self.command.kind {
            CommandKind::Compression(files_to_compress) => {
                Evaluator::handle_compression(files_to_compress, &self.command.output);
            }
            CommandKind::Decompression(files_to_decompress) => {
                Evaluator::handle_decompression(files_to_decompress, &self.command.output);
            }
        }
    }
}