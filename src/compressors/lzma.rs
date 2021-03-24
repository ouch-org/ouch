use std::{fs, io::Write, path::PathBuf};

use colored::Colorize;

use crate::{error::{OuchResult}, extension::CompressionFormat, file::File};
use crate::utils::{
    ensure_exists,
    check_for_multiple_files
};

use super::{Compressor, Entry};

pub struct LzmaCompressor {}

impl LzmaCompressor {
    pub fn compress_files(files: Vec<PathBuf>, format: CompressionFormat) -> OuchResult<Vec<u8>> {
        check_for_multiple_files(&files, &format)?;

        let path = &files[0];
        ensure_exists(path)?;

        let bytes = {
            let bytes = fs::read(path)?;
            Self::compress_bytes(bytes)?
        };

        println!(
            "{}: compressed {:?} into memory ({} bytes)",
            "info".yellow(),
            &path,
            bytes.len()
        );

        Ok(bytes)
    }

    pub fn compress_file_in_memory(file: File) -> OuchResult<Vec<u8>> {
        let file_contents = match file.contents_in_memory {
            Some(bytes) => bytes,
            None => {
                unreachable!();
            }
        };

        Ok(Self::compress_bytes(file_contents)?)
    }
    
    pub fn compress_bytes(bytes_to_compress: Vec<u8>) -> OuchResult<Vec<u8>> {
        let buffer = vec![];
        let mut encoder = xz2::write::XzEncoder::new(buffer, 6);
        encoder.write_all(&*bytes_to_compress)?;

        Ok(encoder.finish()?)
    }
}

impl Compressor for LzmaCompressor {
    fn compress(&self, from: Entry) -> OuchResult<Vec<u8>> {
        let format = CompressionFormat::Lzma;
        match from {
            Entry::Files(files) => Ok(
                Self::compress_files(files, format)?
            ),
            Entry::InMemory(file) => Ok(
                Self::compress_file_in_memory(file)?
            ),
        }
    }
}