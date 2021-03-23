mod tar;
mod compressor;

pub use compressor::{Compressor};
pub use self::tar::TarCompressor;
pub use self::compressor::Entry;