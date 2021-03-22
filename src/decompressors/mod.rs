mod decompressor;
mod tar;
mod zip;
mod niffler;

pub use decompressor::Decompressor;
pub use decompressor::DecompressionResult;
pub use self::tar::TarDecompressor;
pub use self::zip::ZipDecompressor;
pub use self::niffler::NifflerDecompressor;