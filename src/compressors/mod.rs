mod bzip;
mod compressor;
mod gzip;
mod lzma;
mod tar;
mod zip;

pub use compressor::Compressor;

pub use self::{
    bzip::BzipCompressor, compressor::Entry, gzip::GzipCompressor, lzma::LzmaCompressor,
    tar::TarCompressor, zip::ZipCompressor,
};
