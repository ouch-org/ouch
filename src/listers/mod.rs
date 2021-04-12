//! This module contains the Decompressor trait and an implementor for each format.

mod lister;
mod tar;
mod to_memory;
mod zip;

pub use lister::{Lister, ListingResult};

// pub use self::to_memory::{BzipLister, GzipLister, LzmaLister};
// The .tar and .zip decompressors are capable of decompressing directly to storage
pub use self::{tar::TarLister, zip::ZipLister};

pub use lister::list_file;
