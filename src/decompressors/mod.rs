mod decompressor;
mod unified;
mod lzma;
mod tar;
mod zip;


pub use decompressor::Decompressor;
pub use decompressor::DecompressionResult;
pub use self::tar::TarDecompressor;
pub use self::zip::ZipDecompressor;
pub use self::unified::GzipDecompressor;
pub use self::unified::BzipDecompressor;
pub use self::lzma::LzmaDecompressor;