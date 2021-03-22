use std::{ffi::OsStr, fs, io::Write};

use colored::Colorize;

use crate::decompressors::Decompressor;
use crate::decompressors::TarDecompressor;
use crate::decompressors::ZipDecompressor;
use crate::{
    cli::{Command, CommandKind},
    decompressors::{DecompressionResult, NifflerDecompressor},
    error::{self, OuchResult},
    extension::CompressionFormat,
    file::File,
    utils,
};

pub struct Evaluator {
    command: Command,
    // verbosity: Verbosity
}

impl Evaluator {
    fn get_decompressor(
        file: &File,
    ) -> error::OuchResult<(Option<Box<dyn Decompressor>>, Box<dyn Decompressor>)> {
        if file.extension.is_none() {
            // This block *should* be unreachable
            eprintln!(
                "{}: reached Evaluator::get_decompressor without known extension.",
                "internal error".red()
            );
            return Err(error::Error::InvalidInput);
        }
        let extension = file.extension.clone().unwrap();

        let decompressor_from_format = |ext| -> Box<dyn Decompressor> {
            match ext {
                CompressionFormat::Tar => Box::new(TarDecompressor {}),

                CompressionFormat::Zip => Box::new(ZipDecompressor {}),

                CompressionFormat::Gzip | CompressionFormat::Lzma | CompressionFormat::Bzip => {
                    Box::new(NifflerDecompressor {})
                }
            }
        };

        let second_decompressor = decompressor_from_format(extension.second_ext);

        let first_decompressor = match extension.first_ext {
            Some(ext) => Some(decompressor_from_format(ext)),
            None => None,
        };

        Ok((first_decompressor, second_decompressor))
    }

    // todo: move this folder into decompressors/ later on
    fn decompress_file_in_memory(
        bytes: Vec<u8>,
        file: &File,
        decompressor: Option<Box<dyn Decompressor>>,
        output_file: &Option<File>,
    ) -> OuchResult<()> {

        let output_file = utils::get_destination_path(output_file);

        let mut filename = file.path.file_stem().unwrap_or(output_file.as_os_str());
        if filename == "." {
            // I believe this is only possible when the supplied inout has a name
            // of the sort `.tar` or `.zip' and no output has been supplied.
            filename = OsStr::new("ouch-output");
        }

        if decompressor.is_none() {
            // There is no more processing to be done on the input file (or there is but currently unsupported)
            // Therefore, we'll save what we have in memory into a file.

            println!("{}: saving to {:?}.", "info".yellow(), filename);

            let mut f = fs::File::create(output_file.join(filename))?;
            f.write_all(&bytes)?;
            return Ok(());
        }

        // If there is a decompressor to use, we'll create a file in-memory (to-do) and decompress it
        // TODO: change decompressor logic to use BufReader or something like that

        Ok(())
    }

    fn decompress_file(file: File, output: &Option<File>) -> error::OuchResult<()> {
        // let output_file = &command.output;
        let (first_decompressor, second_decompressor) = Self::get_decompressor(&file)?;

        let decompression_result = second_decompressor.decompress(&file, output)?;

        match decompression_result {
            DecompressionResult::FileInMemory(bytes) => {
                // We'll now decompress a file currently in memory.
                // This will currently happen in the case of .bz, .xz and .lzma
                Self::decompress_file_in_memory(bytes, &file, first_decompressor, output)?;
            }
            DecompressionResult::FilesUnpacked(files) => {
                // If the file's last extension was an archival method,
                // such as .tar, .zip or (to-do) .rar, then we won't look for
                // further processing.
                // The reason for this is that cases such as "file.xz.tar" are too rare
                // to worry about, at least at the moment.

                // TODO: use the `files` variable for something
            }
        }

        Ok(())
    }

    pub fn evaluate(command: Command) -> error::OuchResult<()> {
        let output = command.output.clone();
        
        match command.kind {
            CommandKind::Compression(files_to_compress) => {
                for _file in files_to_compress {
                    todo!();
                }
            }
            CommandKind::Decompression(files_to_decompress) => {
                for file in files_to_decompress {
                    Self::decompress_file(file, &output)?;
                }
            }
        }
        Ok(())
    }
}
