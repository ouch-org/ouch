mod decompressor;
mod tar;
mod tomemory;
mod zip;

pub use decompressor::{DecompressionResult, Decompressor};

// These decompressors only decompress to memory,
// unlike {Tar, Zip}Decompressor which are capable of
// decompressing directly to storage
pub use self::{
    tar::TarDecompressor,
    tomemory::{BzipDecompressor, GzipDecompressor, LzmaDecompressor},
    zip::ZipDecompressor,
};
