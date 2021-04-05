use std::path::PathBuf;

use crate::file::File;

// pub enum CompressionResult {
//     ZipArchive(Vec<u8>),
//     TarArchive(Vec<u8>),
//     FileInMemory(Vec<u8>)
// }

pub enum Entry<'a> {
    Files(Vec<PathBuf>),
    InMemory(File<'a>),
}

pub trait Compressor {
    fn compress(&self, from: Entry) -> crate::Result<Vec<u8>>;
}
