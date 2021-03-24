use std::{fs, io::{self, Read}, path::PathBuf};

use colored::Colorize;

use crate::{error::{Error, OuchResult}, extension::CompressionFormat, file::File};
use crate::utils::ensure_exists;

use super::{Compressor, Entry};

pub struct GzipCompressor {}

struct CompressorToMemory {}

impl CompressorToMemory {
    pub fn compress_files(files: Vec<PathBuf>, format: CompressionFormat) -> OuchResult<Vec<u8>> {
        

        if files.len() != 1 {
            eprintln!("{}: cannot compress multiple files directly to {:#?}.\n     Try using an intermediate archival method such as Tar.\n     Example: filename.tar{}", "error".red(), format, format);
            return Err(Error::InvalidInput);
        }

        let mut contents = Vec::new();
        let path = &files[0];
        ensure_exists(path)?;

        let bytes_written = {
            let bytes = fs::read(path)?;

            // let mut buffer = vec![];
            // let mut encoder = get_encoder(&format, Box::new(&mut buffer));
            // encoder.write_all(&*bytes)?;
            // bytes.as_slice().read_to_end(&mut contents)?
            Self::compress_bytes(&mut contents, bytes, format)?
        };

        println!(
            "{}: compressed {:?} into memory ({} bytes)",
            "info".yellow(),
            &path,
            bytes_written
        );

        Ok(contents)
    }

    pub fn compress_file_in_memory(file: File, format:CompressionFormat ) -> OuchResult<Vec<u8>> {
        let mut compressed_contents = Vec::new();
        let file_contents = match file.contents_in_memory {
            Some(bytes) => bytes,
            None => {
                unreachable!();
            }
        };

        let _bytes_written = Self::compress_bytes(&mut compressed_contents, file_contents, format);

        Ok(compressed_contents)
    }

    pub fn compress_bytes(mut contents: &mut Vec<u8>, bytes_to_compress: Vec<u8>, format: CompressionFormat) -> OuchResult<usize> {
        let mut buffer = vec![];
        let mut encoder = get_encoder(&format, Box::new(&mut buffer));
        encoder.write_all(&*bytes_to_compress)?;

        Ok(bytes_to_compress.as_slice().read_to_end(&mut contents)?)
    }


}

fn get_encoder<'a>(
    format: &CompressionFormat,
    buffer: Box<dyn io::Write + Send + 'a>,
) -> Box<dyn io::Write + Send + 'a> {
    match format {
        CompressionFormat::Gzip => Box::new(flate2::write::GzEncoder::new(
            buffer,
            flate2::Compression::default(),
        )),
        _other => unreachable!(),
    }
}

impl Compressor for GzipCompressor {
    fn compress(&self, from: Entry) -> OuchResult<Vec<u8>> {
        let format = CompressionFormat::Gzip;
        match from {
            Entry::Files(files) => Ok(
                CompressorToMemory::compress_files(files, format)?
            ),
            Entry::InMemory(file) => Ok(
                CompressorToMemory::compress_file_in_memory(file, format)?
            ),
        }
    }
}