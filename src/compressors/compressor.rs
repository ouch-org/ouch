use std::path::PathBuf;

use crate::file::File;

pub enum Entry<'a> {
    Files(Vec<PathBuf>),
    InMemory(File<'a>),
}

pub trait Compressor {
    fn compress(&self, from: Entry) -> crate::Result<Vec<u8>>;
}
