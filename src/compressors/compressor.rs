use std::path::PathBuf;

use crate::{error::OuchResult, file::File};

pub enum CompressionResult {
    FilesUnpacked(Vec<PathBuf>),
    FileInMemory(Vec<u8>)
}

pub trait Compressor {
    fn compress(&self, from: Vec<File>, into: &Option<File>) -> OuchResult<DecompressionResult>;
}

// 
//
//
//
//
//
//
//
//