use std::{
    io::{self, Read},
    path::Path,
};

use colored::Colorize;

use super::decompressor::{DecompressionResult, Decompressor};
use crate::bytes::Bytes;
use crate::utils;
use crate::{extension::CompressionFormat, file::File};

struct DecompressorToMemory {}
pub struct GzipDecompressor {}
pub struct LzmaDecompressor {}
pub struct BzipDecompressor {}

fn get_decoder<'a>(
    format: CompressionFormat,
    buffer: Box<dyn io::Read + Send + 'a>,
) -> Box<dyn io::Read + Send + 'a> {
    match format {
        CompressionFormat::Bzip => Box::new(bzip2::read::BzDecoder::new(buffer)),
        CompressionFormat::Gzip => Box::new(flate2::read::MultiGzDecoder::new(buffer)),
        CompressionFormat::Lzma => Box::new(xz2::read::XzDecoder::new_multi_decoder(buffer)),
        _other => unreachable!(),
    }
}

impl DecompressorToMemory {
    fn unpack_file(path: &Path, format: CompressionFormat) -> crate::Result<Vec<u8>> {
        let file = std::fs::read(path)?;

        let mut reader = get_decoder(format, Box::new(&file[..]));

        let mut buffer = Vec::new();
        let bytes_read = reader.read_to_end(&mut buffer)?;

        println!(
            "{}: {:?} extracted into memory ({}).",
            "info".yellow(),
            path,
            Bytes::new(bytes_read as u64)
        );

        Ok(buffer)
    }

    fn decompress(
        from: File,
        format: CompressionFormat,
        into: &Option<File>,
    ) -> crate::Result<DecompressionResult> {
        let destination_path = utils::get_destination_path(into);

        utils::create_path_if_non_existent(destination_path)?;

        let bytes = Self::unpack_file(&from.path, format)?;

        Ok(DecompressionResult::FileInMemory(bytes))
    }
}

impl Decompressor for GzipDecompressor {
    fn decompress(
        &self,
        from: File,
        into: &Option<File>,
        _: &oof::Flags,
    ) -> crate::Result<DecompressionResult> {
        DecompressorToMemory::decompress(from, CompressionFormat::Gzip, into)
    }
}

impl Decompressor for BzipDecompressor {
    fn decompress(
        &self,
        from: File,
        into: &Option<File>,
        _: &oof::Flags,
    ) -> crate::Result<DecompressionResult> {
        DecompressorToMemory::decompress(from, CompressionFormat::Bzip, into)
    }
}

impl Decompressor for LzmaDecompressor {
    fn decompress(
        &self,
        from: File,
        into: &Option<File>,
        _: &oof::Flags,
    ) -> crate::Result<DecompressionResult> {
        DecompressorToMemory::decompress(from, CompressionFormat::Lzma, into)
    }
}
