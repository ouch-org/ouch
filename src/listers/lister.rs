/// Defines the Lister trait
/// You'll notice listers share a lot of code
/// with the decompressors, because they basically are
/// cut-down versions of the decompressors.
/// Listing logic wasn't added to decompressors themselves in
/// order to keep both modules relatively uncomplicated
use std::path::{Path, PathBuf};

use super::{TarLister, ZipLister};
use crate::decompressors::DecompressionResult;
use crate::{decompressors::Decompressor, file::File, Error};
use crate::{
    decompressors::{
        BzipDecompressor, GzipDecompressor, LzmaDecompressor, TarDecompressor, ZipDecompressor,
    },
    extension::CompressionFormat,
};

pub type ListingResult = Vec<PathBuf>;

pub trait Lister {
    fn list(&self, from: File) -> crate::Result<ListingResult>;
}

type BoxedLister = Box<dyn Lister>;

fn get_directly_listable(fmt: &CompressionFormat) -> Option<BoxedLister> {
    match fmt {
        // Only those two formats are currently directly listable
        CompressionFormat::Tar => Some(Box::new(TarLister)),
        CompressionFormat::Zip => Some(Box::new(ZipLister)),
        _ => None,
    }
}

/// Lists the files contained in the given archive
pub fn list_file(path: &Path) -> crate::Result<Vec<PathBuf>> {
    // The file to be decompressed
    let file = File::from(path)?;

    // The file must have a supported decompressible format
    if file.extension.is_none() {
        return Err(crate::Error::MissingExtensionError(PathBuf::from(path)));
    }

    // Step 1: check for directly listable formats (.zip and .tar)
    // if let Some(lister) = get_directly_listable(&file) {
    //     return lister.list(file);
    // }

    let extension = match &file.extension {
        Some(ext) => ext,
        None => unreachable!(),
    };

    if let Some(first_ext) = extension.first_ext {
        // We're dealing with extensions like .zip.{.bz, .gz, .lz, ..}, .tar.

        let extension = file.extension.clone().unwrap();
        let second_ext = &extension.second_ext;
        let decompressor = decompressor_from_format(second_ext);
        let extension = file.extension;
        match decompressor.decompress(file, &None, &oof::Flags::default())? {
            DecompressionResult::FileInMemory(bytes) => {
                // We had a file, such as .tar.gz, and now have the .tar stored in-memory.
                // We must now make a new file and call the respective lister for that format
                let file = File {
                    path,
                    contents_in_memory: Some(bytes),
                    extension,
                };
                let lister = if let Some(lister) = get_directly_listable(&first_ext) {
                    lister
                } else {
                    return Err(Error::UnlistableFormat(format!(
                        "{}{}",
                        first_ext, second_ext
                    )));
                };

                lister.list(file)
            }
            DecompressionResult::FilesUnpacked(_) => {
                unreachable!("unreachable code in lister::list_file")
            }
        }
    } else {
        // We're dealing with extensions like .zip, .tar, .gz, .bz, .lz
        match extension.second_ext {
            CompressionFormat::Gzip | CompressionFormat::Bzip | CompressionFormat::Lzma => {
                todo!("not sure what to do here yet")
            }
            CompressionFormat::Tar | CompressionFormat::Zip => {
                let lister = get_directly_listable(&extension.second_ext).unwrap();
                return lister.list(file);
            }
        }
    }
}

fn decompressor_from_format(fmt: &CompressionFormat) -> Box<dyn Decompressor> {
    match fmt {
        CompressionFormat::Gzip => Box::new(GzipDecompressor),
        CompressionFormat::Bzip => Box::new(BzipDecompressor),
        CompressionFormat::Lzma => Box::new(LzmaDecompressor),
        CompressionFormat::Tar => Box::new(TarDecompressor),
        CompressionFormat::Zip => Box::new(ZipDecompressor),
    }
}
