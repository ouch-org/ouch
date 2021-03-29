use std::{ffi::OsStr, fs, io::Write, path::PathBuf};

use colored::Colorize;

use crate::{
    cli::{Flags, Command, CommandKind},
    compressors::{
        BzipCompressor, Compressor, Entry, GzipCompressor, LzmaCompressor, TarCompressor,
        ZipCompressor,
    },
    decompressors::{
        BzipDecompressor, DecompressionResult, Decompressor, GzipDecompressor, LzmaDecompressor,
        TarDecompressor, ZipDecompressor,
    },
    extension::{CompressionFormat, Extension},
    dialogs::Confirmation,
    file::File,
    utils,
};

pub struct Evaluator {}

impl Evaluator {
    pub fn get_compressor(
        file: &File,
    ) -> crate::Result<(Option<Box<dyn Compressor>>, Box<dyn Compressor>)> {
        let extension = match &file.extension {
            Some(extension) => extension.clone(),
            None => {
                // This block *should* be unreachable
                eprintln!(
                    "{}: reached Evaluator::get_decompressor without known extension.",
                    "internal error".red()
                );
                return Err(crate::Error::InvalidInput);
            }
        };

        // Supported first compressors:
        // .tar and .zip
        let first_compressor: Option<Box<dyn Compressor>> = match extension.first_ext {
            Some(ext) => match ext {
                CompressionFormat::Tar => Some(Box::new(TarCompressor {})),
                CompressionFormat::Zip => Some(Box::new(ZipCompressor {})),
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
            CompressionFormat::Zip => Box::new(ZipCompressor {}),
            CompressionFormat::Bzip => Box::new(BzipCompressor {}),
            CompressionFormat::Gzip => Box::new(GzipCompressor {}),
            CompressionFormat::Lzma => Box::new(LzmaCompressor {}),
        };

        Ok((first_compressor, second_compressor))
    }

    pub fn get_decompressor(
        file: &File,
    ) -> crate::Result<(Option<Box<dyn Decompressor>>, Box<dyn Decompressor>)> {
        let extension = match &file.extension {
            Some(extension) => extension.clone(),
            None => {
                // This block *should* be unreachable
                eprintln!(
                    "{}: reached Evaluator::get_decompressor without known extension.",
                    "internal error".red()
                );
                return Err(crate::Error::InvalidInput);
            }
        };

        let second_decompressor: Box<dyn Decompressor> = match extension.second_ext {
            CompressionFormat::Tar => Box::new(TarDecompressor {}),
            CompressionFormat::Zip => Box::new(ZipDecompressor {}),
            CompressionFormat::Gzip => Box::new(GzipDecompressor {}),
            CompressionFormat::Lzma => Box::new(LzmaDecompressor {}),
            CompressionFormat::Bzip => Box::new(BzipDecompressor {}),
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

    fn decompress_file_in_memory(
        bytes: Vec<u8>,
        file_path: PathBuf,
        decompressor: Option<Box<dyn Decompressor>>,
        output_file: &Option<File>,
        extension: Option<Extension>,
        flags: Flags
    ) -> crate::Result<()> {
        let output_file_path = utils::get_destination_path(output_file);

        let mut filename = file_path
            .file_stem()
            .unwrap_or_else(|| output_file_path.as_os_str());

        if filename == "." {
            // I believe this is only possible when the supplied input has a name
            // of the sort `.tar` or `.zip' and no output has been supplied.
            filename = OsStr::new("ouch-output");
        }
        let filename = PathBuf::from(filename);


        // If there is a decompressor to use, we'll create a file in-memory and decompress it
        let decompressor = match decompressor {
            Some(decompressor) => decompressor,
            None => {
                // There is no more processing to be done on the input file (or there is but currently unsupported)
                // Therefore, we'll save what we have in memory into a file.
                println!("{}: saving to {:?}.", "info".yellow(), filename);
                // TODO: use -y and -n flags
                let mut f = fs::File::create(output_file_path.join(filename))?;
                f.write_all(&bytes)?;
                return Ok(());
            }
        };

        let file = File {
            path: filename,
            contents_in_memory: Some(bytes),
            extension,
        };

        let decompression_result = decompressor.decompress(file, output_file, flags)?;
        if let DecompressionResult::FileInMemory(_) = decompression_result {
            // Should not be reachable.
            unreachable!();
        }

        Ok(())
    }

    fn compress_files(files: Vec<PathBuf>, mut output: File, flags: Flags) -> crate::Result<()> {
        let confirm = Confirmation::new("Do you want to overwrite 'FILE'?", Some("FILE"));
        let (first_compressor, second_compressor) = Self::get_compressor(&output)?;

        // TODO: use -y and -n here
        let output_path = output.path.clone();
        if output_path.exists() {
            if !utils::permission_for_overwriting(&output_path, flags, &confirm)? {
                // The user does not want to overwrite the file
                return Ok(());
            }
        }


        let bytes = match first_compressor {
            Some(first_compressor) => {
                let mut entry = Entry::Files(files);
                let bytes = first_compressor.compress(entry)?;

                output.contents_in_memory = Some(bytes);
                entry = Entry::InMemory(output);
                second_compressor.compress(entry)?
            }
            None => {
                let entry = Entry::Files(files);
                second_compressor.compress(entry)?
            }
        };

        println!(
            "{}: writing to {:?}. ({} bytes)",
            "info".yellow(),
            &output_path,
            bytes.len()
        );
        fs::write(output_path, bytes)?;

        Ok(())
    }

    fn decompress_file(file: File, output: &Option<File>, flags: Flags) -> crate::Result<()> {
        let (first_decompressor, second_decompressor) = Self::get_decompressor(&file)?;

        let file_path = file.path.clone();
        let extension = file.extension.clone();

        let decompression_result = second_decompressor.decompress(file, output, flags)?;

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
                    flags
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

    pub fn evaluate(command: Command, flags: Flags) -> crate::Result<()> {
        let output = command.output.clone();

        match command.kind {
            CommandKind::Compression(files_to_compress) => {
                // Safe to unwrap since output is mandatory for compression
                let output = output.unwrap();
                Self::compress_files(files_to_compress, output, flags)?;
            }
            CommandKind::Decompression(files_to_decompress) => {
                for file in files_to_decompress {
                    Self::decompress_file(file, &output, flags)?;
                }
            }
        }
        Ok(())
    }
}
