use std::{fs, io::{self, Read, Write}, path::PathBuf};

use colored::Colorize;

use crate::{error::{Error, OuchResult}, extension::CompressionFormat, file::File};
use crate::utils::ensure_exists;

use super::{Compressor, Entry};

pub struct BzipCompressor {}

struct CompressorToMemory {}

// impl CompressorToMemory {
//     pub fn compress_files(files: Vec<PathBuf>, format: CompressionFormat) -> OuchResult<Vec<u8>> {
//         let mut buffer = vec![];

//         if files.len() != 1 {
//             eprintln!("{}: cannot compress multiple files directly to {:#?}.\n     Try using an intermediate archival method such as Tar.\n     Example: filename.tar{}", "error".red(), format, format);
//             return Err(Error::InvalidInput);
//         }

//         let mut contents = Vec::new();
//         let path = &files[0];
//         ensure_exists(path)?;

//         let bytes_read = {
//             let bytes = fs::read(path)?;
//             let mut encoder = get_encoder(&format, Box::new(&mut buffer));
//             encoder.write_all(&*bytes)?;
//             bytes.as_slice().read_to_end(&mut contents)?
//         };

//         println!("{}: compressed {:?} into memory ({} bytes)", "info".yellow(), &path, bytes_read);
        
//         Ok(contents)
//     }

//     pub fn compress_bytes(file: File) {
        
//     }
// }

impl BzipCompressor {
    fn compress_files(files: Vec<PathBuf>, format: CompressionFormat) -> OuchResult<Vec<u8>> {
        if files.len() != 1 {
            eprintln!("{}: cannot compress multiple files directly to {:#?}.\n     Try using an intermediate archival method such as Tar.\n     Example: filename.tar{}", "error".red(), format, format);
            return Err(Error::InvalidInput);
        }
        let path = &files[0];
        ensure_exists(path)?;
        let contents = {
            let bytes = fs::read(path)?;
            Self::compress_bytes(&*bytes)?
        };

        println!("{}: compressed {:?} into memory ({} bytes)", "info".yellow(), &path, contents.len());
        
        Ok(contents)
    }

    fn compress_file_in_memory(file: File) -> OuchResult<Vec<u8>> {
        // Ensure that our file has in-memory content
        let bytes = match file.contents_in_memory {
            Some(bytes) => bytes,
            None => {
                // TODO: error message,
                return Err(Error::InvalidInput);
            }
        };

        Ok(Self::compress_bytes(&*bytes)?)
    }

    fn compress_bytes(bytes: &[u8]) -> OuchResult<Vec<u8>> {
        let buffer = vec![];
        let mut encoder = bzip2::write::BzEncoder::new(buffer, bzip2::Compression::new(6));
        encoder.write_all(bytes)?;
        Ok(encoder.finish()?)
    }

}

// TODO: customizable compression level
fn get_encoder<'a>(format: &CompressionFormat, buffer: Box<dyn io::Write + Send + 'a>) -> Box<dyn io::Write + Send + 'a> {
    match format {
        CompressionFormat::Bzip => Box::new(bzip2::write::BzEncoder::new(buffer, bzip2::Compression::new(4))),
        _other => unreachable!()
    }
}

impl Compressor for BzipCompressor {
    fn compress(&self, from: Entry) -> OuchResult<Vec<u8>> {
        match from {
            Entry::Files(files) => Ok(
                Self::compress_files(files, CompressionFormat::Bzip)?
            ),
            Entry::InMemory(file) => Ok(
                Self::compress_file_in_memory(file)?
            ),
        }
    }
}