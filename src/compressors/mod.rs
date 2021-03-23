mod tar;
mod compressor;

pub use compressor::{Compressor, CompressionResult};
pub use self::tar::TarCompressor;