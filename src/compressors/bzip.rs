use std::{fs, io::Write, path::PathBuf};

use colored::Colorize;

use super::{Compressor, Entry};
use crate::{
    extension::CompressionFormat,
    file::File,
    utils::{check_for_multiple_files, ensure_exists},
};

pub struct BzipCompressor {}

impl BzipCompressor {
    fn compress_files(files: Vec<PathBuf>, format: CompressionFormat) -> crate::Result<Vec<u8>> {
        check_for_multiple_files(&files, &format)?;
        let path = &files[0];
        ensure_exists(path)?;
        let contents = {
            let bytes = fs::read(path)?;
            Self::compress_bytes(&*bytes)?
        };

        println!(
            "{}: compressed {:?} into memory ({} bytes)",
            "info".yellow(),
            &path,
            contents.len()
        );

        Ok(contents)
    }

    fn compress_file_in_memory(file: File) -> crate::Result<Vec<u8>> {
        // Ensure that our file has in-memory content
        let bytes = match file.contents_in_memory {
            Some(bytes) => bytes,
            None => {
                // TODO: error message,
                return Err(crate::Error::InvalidInput);
            }
        };

        Self::compress_bytes(&*bytes)
    }

    fn compress_bytes(bytes: &[u8]) -> crate::Result<Vec<u8>> {
        let buffer = vec![];
        let mut encoder = bzip2::write::BzEncoder::new(buffer, bzip2::Compression::new(6));
        encoder.write_all(bytes)?;
        Ok(encoder.finish()?)
    }
}

// TODO: customizable compression level
impl Compressor for BzipCompressor {
    fn compress(&self, from: Entry) -> crate::Result<Vec<u8>> {
        match from {
            Entry::Files(files) => Ok(Self::compress_files(files, CompressionFormat::Bzip)?),
            Entry::InMemory(file) => Ok(Self::compress_file_in_memory(file)?),
        }
    }
}
