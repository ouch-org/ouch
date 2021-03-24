mod decompressor;
mod tomemory;
mod tar;
mod zip;


pub use decompressor::Decompressor;
pub use decompressor::DecompressionResult;

pub use self::tar::TarDecompressor;
pub use self::zip::ZipDecompressor;

// These decompressors only decompress to memory,
// unlike {Tar, Zip}Decompressor which are capable of
// decompressing directly to storage
pub use self::tomemory::GzipDecompressor;
pub use self::tomemory::BzipDecompressor;
pub use self::tomemory::LzmaDecompressor;