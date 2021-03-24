use std::{
    io::{self, Read},
    path::{Path, PathBuf},
};

use bzip2::Compress;
use colored::Colorize;
// use niffler;

use crate::{extension::CompressionFormat, file::File};
use crate::{
    error::{self, OuchResult},
    utils,
};

use super::decompressor::DecompressionResult;
use super::decompressor::Decompressor;

pub struct UnifiedDecompressor {}
pub struct LzmaDecompressor {}
pub struct GzipDecompressor {}
pub struct BzipDecompressor {}

fn get_decoder<'a>(format: CompressionFormat, buffer: Box<dyn io::Read + Send + 'a>) -> Box<dyn io::Read + Send + 'a> {
    match format {
        CompressionFormat::Lzma => Box::new(xz2::read::XzDecoder::new(buffer)),
        CompressionFormat::Bzip => Box::new(bzip2::read::BzDecoder::new(buffer)),
        CompressionFormat::Gzip => Box::new(flate2::read::MultiGzDecoder::new(buffer)),
        other => unreachable!()
    }
}

impl UnifiedDecompressor {
    fn unpack_file(from: &Path, format: CompressionFormat) -> OuchResult<Vec<u8>> {
        // println!("{}: trying to decompress {:?}", "info".yellow(), from);

        let file = std::fs::read(from)?;

        let mut reader = get_decoder(format, Box::new(&file[..]));

        let mut buffer = Vec::new();
        let bytes_read = reader.read_to_end(&mut buffer)?;

        println!(
            "{}: {:?} extracted into memory ({} bytes).",
            "info".yellow(),
            from,
            bytes_read
        );

        Ok(buffer)
    }

    fn decompress(from: File, format: CompressionFormat, into: &Option<File>) -> OuchResult<DecompressionResult> {
        let destination_path = utils::get_destination_path(into);

        utils::create_path_if_non_existent(destination_path)?;

        let bytes = Self::unpack_file(&from.path, format)?;

        Ok(DecompressionResult::FileInMemory(bytes))
    }
}

impl Decompressor for LzmaDecompressor {
    fn decompress(&self, from: File, into: &Option<File>) -> OuchResult<DecompressionResult> {
        UnifiedDecompressor::decompress(from, CompressionFormat::Lzma, into)
    }
}

impl Decompressor for GzipDecompressor {
    fn decompress(&self, from: File, into: &Option<File>) -> OuchResult<DecompressionResult> {
        UnifiedDecompressor::decompress(from, CompressionFormat::Gzip, into)
    }
}

impl Decompressor for BzipDecompressor {
    fn decompress(&self, from: File, into: &Option<File>) -> OuchResult<DecompressionResult> {
        UnifiedDecompressor::decompress(from, CompressionFormat::Bzip, into)
    }
}