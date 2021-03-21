
use colored::Colorize;

use crate::{cli::{Command, CommandKind}, error, extension::CompressionFormat, file::File};
use crate::decompressors::tar;

pub struct Evaluator {   
    command: Command,
    // verbosity: Verbosity
}

impl Evaluator {
    // todo: remove this?
    pub fn new(command: Command) -> Self {
        Self {
            command
        }
    }

    fn decompress_file(&self, file: &File) -> error::OuchResult<()> {
        println!("{}: attempting to decompress {:?}", "ouch".bright_blue(), file.path);
        if file.extension.is_none() {
            // This block *should* be unreachable
            eprintln!("{}: reached Evaluator::decompress_file without known extension.", "internal error".red());
            return Err(error::Error::InvalidInput);
        }
        let extension = file.extension.clone().unwrap();
        let output_file = &self.command.output;

        match extension.second_ext {
            CompressionFormat::Tar => { 
                let _ = tar::Decompressor::decompress(file, output_file)?;
            },
            _ => { 
                todo!()
            }
        }    

        // TODO: decompress first extension

        Ok(())
    }

    pub fn evaluate(&mut self) -> error::OuchResult<()> {
        match &self.command.kind {
            CommandKind::Compression(files_to_compress) => {
                for _file in files_to_compress {
                    todo!();
                }
            }
            CommandKind::Decompression(files_to_decompress) => {
                for file in files_to_decompress {
                    self.decompress_file(file)?;
                }
            }
        }
        Ok(())
    }
}