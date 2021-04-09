// use std::{
//     io::{self, Read},
//     path::Path,
// };


// use utils::colors;

// use super::lister::{ListingResult, Lister};
// use crate::{extension::CompressionFormat, file::File, utils};

// struct DecompressorToMemory;
// pub struct GzipLister;
// pub struct LzmaLister;
// pub struct BzipLister;

// fn get_decoder<'a>(
//     format: CompressionFormat,
//     buffer: Box<dyn io::Read + Send + 'a>,
// ) -> Box<dyn io::Read + Send + 'a> {
//     match format {
//         CompressionFormat::Bzip => Box::new(bzip2::read::BzDecoder::new(buffer)),
//         CompressionFormat::Gzip => Box::new(flate2::read::MultiGzDecoder::new(buffer)),
//         CompressionFormat::Lzma => Box::new(xz2::read::XzDecoder::new_multi_decoder(buffer)),
//         _other => unreachable!(),
//     }
// }

// impl DecompressorToMemory {
//     fn unpack_file(path: &Path, format: CompressionFormat) -> crate::Result<Vec<u8>> {
//         let file = std::fs::read(path)?;

//         let mut reader = get_decoder(format, Box::new(&file[..]));

//         let mut buffer = Vec::new();
//         let bytes_read = reader.read_to_end(&mut buffer)?;

//         println!(
//             "{}[INFO]{} {:?} extracted into memory ({}).",
//             colors::yellow(),
//             colors::reset(),
//             path,
//             utils::Bytes::new(bytes_read as u64)
//         );

//         Ok(buffer)
//     }

//     fn decompress(
//         from: File,
//         format: CompressionFormat,
//     ) -> crate::Result<ListingResult> {
        
//         let bytes = Self::unpack_file(&from.path, format)?;

//         Ok(ListingResult::FileInMemory(bytes))
//     }
// }

// impl Lister for GzipLister {
//     fn list(
//         &self,
//         from: File,
//     ) -> crate::Result<ListingResult> {
//         DecompressorToMemory::decompress(from, CompressionFormat::Gzip)
//     }
// }

// impl Lister for BzipLister {
//     fn list(
//         &self,
//         from: File,
//     ) -> crate::Result<ListingResult> {
//         DecompressorToMemory::decompress(from, CompressionFormat::Bzip)
//     }
// }

// impl Lister for LzmaLister {
//     fn list(
//         &self,
//         from: File,
//     ) -> crate::Result<ListingResult> {
//         DecompressorToMemory::decompress(from, CompressionFormat::Lzma)
//     }
// }
