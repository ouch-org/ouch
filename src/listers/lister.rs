/// Defines the Lister trait
/// You'll notice listers share a lot of code
/// with the decompressors, because they basically are
/// cut-down versions of the decompressors.
/// Listing logic wasn't added to decompressors themselves in
/// order to keep both modules relatively uncomplicated

use std::path::{Path, PathBuf};

use crate::{Error, decompressors::Decompressor, file::File, utils::Either};
use crate::extension::CompressionFormat;
use super::{ZipLister, TarLister};

pub type ListingResult = Vec<PathBuf>;

pub trait Lister {
    fn list (
        &self,
        from: File,
    ) -> crate::Result<ListingResult>;
}


type MaybeADecompressor = Option<Box<dyn Decompressor>>;
type BoxedLister = Box<dyn Lister>;

fn get_directly_listable(file: &File) -> Option<BoxedLister> {
    let extension = match &file.extension {
        Some(ext) => ext,
        None => unreachable!("Dev error: file extension should have been checked in ::list_file")
    };

    match extension.first_ext {
        None => {
            // We're only dealing with a single extension, so we'll check if they are directly listable
            match extension.second_ext {
                CompressionFormat::Tar => Some(Box::new(TarLister)),
                CompressionFormat::Zip => Some(Box::new(ZipLister)),
                _ => None,
            }
        },
        Some(_) => None,
    }
}
/// Lists the files contained in the given archive
pub fn list_file(path: &Path) -> crate::Result<Vec<PathBuf>> {
    // The file to be decompressed
    let file = File::from(path)?;
    dbg!(&file);
    
    // The file must have a supported decompressible format
    if file.extension.is_none() {
        return Err(crate::Error::MissingExtensionError(
            PathBuf::from(path)
        ));
    }

    // Step 1: check for directly listable formats (.zip and .tar)
    match get_directly_listable(&file) {
        Some(lister) => {
            return lister.list(file);
        }
        None => {}
    }

    Ok(vec![])
}
