use std::{fs, io::Write, path::PathBuf};

use colored::Colorize;

use super::{Compressor, Entry};
use crate::{
    extension::CompressionFormat,
    file::File,
    utils::{check_for_multiple_files, ensure_exists},
};

pub struct LzmaCompressor {}

impl LzmaCompressor {
    pub fn compress_files(
        files: Vec<PathBuf>,
        format: CompressionFormat,
    ) -> crate::Result<Vec<u8>> {
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

    pub fn compress_file_in_memory(file: File) -> crate::Result<Vec<u8>> {
        let file_contents = match file.contents_in_memory {
            Some(bytes) => bytes,
            None => {
                unreachable!();
            }
        };

        Self::compress_bytes(file_contents)
    }

    pub fn compress_bytes(bytes_to_compress: Vec<u8>) -> crate::Result<Vec<u8>> {
        let buffer = vec![];
        let mut encoder = xz2::write::XzEncoder::new(buffer, 6);
        encoder.write_all(&*bytes_to_compress)?;

        Ok(encoder.finish()?)
    }
}

impl Compressor for LzmaCompressor {
    fn compress(&self, from: Entry) -> crate::Result<Vec<u8>> {
        let format = CompressionFormat::Lzma;
        match from {
            Entry::Files(files) => Self::compress_files(files, format),
            Entry::InMemory(file) => Self::compress_file_in_memory(file),
        }
    }
}
