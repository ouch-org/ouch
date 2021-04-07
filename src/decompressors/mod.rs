//! This module contains the Decompressor trait and an implementor for each format.

mod decompressor;
mod tar;
mod to_memory;
mod zip;

pub use decompressor::{DecompressionResult, Decompressor};

pub use self::to_memory::{BzipDecompressor, GzipDecompressor, LzmaDecompressor};
// The .tar and .zip decompressors are capable of decompressing directly to storage
pub use self::{tar::TarDecompressor, zip::ZipDecompressor};
