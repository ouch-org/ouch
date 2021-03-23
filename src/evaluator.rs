use std::{ffi::OsStr, fs, io::Write, path::PathBuf};

use colored::Colorize;

use crate::{compressors::TarCompressor, decompressors::TarDecompressor};
use crate::decompressors::ZipDecompressor;
use crate::{
    cli::{Command, CommandKind},
    decompressors::{
        Decompressor,
        DecompressionResult, 
        NifflerDecompressor
    },
    compressors::Compressor,
    error::{self, OuchResult},
    extension::{
        Extension,
        CompressionFormat,
    },
    file::File,
    utils,
};


pub struct Evaluator {
    // verbosity: Verbosity
}

impl Evaluator {
    fn get_compressor(
        file: &File,
    ) -> error::OuchResult<(Option<Box<dyn Compressor>>, Box<dyn Compressor>)> {
        if file.extension.is_none() {
            // This block *should* be unreachable
            eprintln!(
                "{}: reached Evaluator::get_decompressor without known extension.",
                "internal error".red()
            );
            return Err(error::Error::InvalidInput);
        }
        let extension = file.extension.clone().unwrap();
        
        // Supported first compressors:
        // .tar and .zip
        let first_compressor: Option<Box<dyn Compressor>>  = match extension.first_ext {
            Some(ext) => match ext {
                CompressionFormat::Tar => Some(Box::new(TarCompressor {})),

                // CompressionFormat::Zip => Some(Box::new(ZipCompressor {})),

                // _other => Some(Box::new(NifflerCompressor {})),
                _other => {
                    todo!();
                }
            },
            None => None,
        };

        // Supported second compressors:
        // any
        let second_compressor: Box<dyn Compressor> = match extension.second_ext {
            CompressionFormat::Tar => Box::new(TarCompressor {}),
            _other => todo!()
            //   
        };

        Ok((first_compressor, second_compressor))
    }

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

        let second_decompressor: Box<dyn Decompressor> = match extension.second_ext {
            CompressionFormat::Tar => Box::new(TarDecompressor {}),

            CompressionFormat::Zip => Box::new(ZipDecompressor {}),

            CompressionFormat::Gzip | CompressionFormat::Lzma | CompressionFormat::Bzip => {
                Box::new(NifflerDecompressor {})
            }
        };

        let first_decompressor: Option<Box<dyn Decompressor>> = match extension.first_ext {
            Some(ext) => match ext {
                CompressionFormat::Tar => Some(Box::new(TarDecompressor {})),

                CompressionFormat::Zip => Some(Box::new(ZipDecompressor {})),

                _other => None,
            },
            None => None,
        };

        Ok((first_decompressor, second_decompressor))
    }

    // todo: move this folder into decompressors/ later on
    fn decompress_file_in_memory(
        bytes: Vec<u8>,
        file_path: PathBuf,
        decompressor: Option<Box<dyn Decompressor>>,
        output_file: &Option<File>,
        extension: Option<Extension>,
    ) -> OuchResult<()> {
        let output_file_path = utils::get_destination_path(output_file);

        let mut filename = file_path
            .file_stem()
            .unwrap_or(output_file_path.as_os_str());
        if filename == "." {
            // I believe this is only possible when the supplied inout has a name
            // of the sort `.tar` or `.zip' and no output has been supplied.
            filename = OsStr::new("ouch-output");
        }

        let filename = PathBuf::from(filename);

        if decompressor.is_none() {
            // There is no more processing to be done on the input file (or there is but currently unsupported)
            // Therefore, we'll save what we have in memory into a file.

            println!("{}: saving to {:?}.", "info".yellow(), filename);

            let mut f = fs::File::create(output_file_path.join(filename))?;
            f.write_all(&bytes)?;
            return Ok(());
        }

        let file = File {
            path: filename,
            contents: Some(bytes),
            extension,
        };

        let decompressor = decompressor.unwrap();

        // If there is a decompressor to use, we'll create a file in-memory and decompress it

        let decompression_result = decompressor.decompress(file, output_file)?;
        if let DecompressionResult::FileInMemory(_) = decompression_result {
            // Should not be reachable.
            unreachable!();
        }

        Ok(())
    }

    fn compress_files(files: Vec<PathBuf>, output: File) -> error::OuchResult<()> {
        let (first_decompressor, second_decompressor) = Self::get_compressor(&output)?;
        Ok(())
    }

    fn decompress_file(file: File, output: &Option<File>) -> error::OuchResult<()> {
        // let output_file = &command.output;
        let (first_decompressor, second_decompressor) = Self::get_decompressor(&file)?;

        let file_path = file.path.clone();
        let extension = file.extension.clone();

        let decompression_result = second_decompressor.decompress(file, output)?;

        match decompression_result {
            DecompressionResult::FileInMemory(bytes) => {
                // We'll now decompress a file currently in memory.
                // This will currently happen in the case of .bz, .xz and .lzma
                Self::decompress_file_in_memory(
                    bytes,
                    file_path,
                    first_decompressor,
                    output,
                    extension,
                )?;
            }
            DecompressionResult::FilesUnpacked(_files) => {
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
                // Safe to unwrap since output is mandatory for compression
                let output = output.unwrap();
                Self::compress_files(files_to_compress, output)?;
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
