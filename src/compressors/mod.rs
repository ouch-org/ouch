mod tar;
mod zip;
mod bzip;
mod gzip;
mod lzma;
mod compressor;

pub use compressor::Compressor;
pub use self::compressor::Entry;
pub use self::tar::TarCompressor;
pub use self::zip::ZipCompressor;
pub use self::bzip::BzipCompressor;
pub use self::gzip::GzipCompressor;
pub use self::lzma::LzmaCompressor;