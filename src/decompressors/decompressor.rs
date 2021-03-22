use std::path::PathBuf;

use crate::{error::OuchResult, file::File};

pub enum DecompressionResult {
    FilesUnpacked(Vec<PathBuf>),
    FileInMemory(Vec<u8>)
}

pub trait Decompressor {
    fn decompress(&self, from: &File, into: &Option<File>) -> OuchResult<DecompressionResult>;
}