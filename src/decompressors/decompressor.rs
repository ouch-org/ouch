use std::path::PathBuf;

use crate::{error::OuchResult, file::File};

/// This file should/could store a Decompressor trait

pub trait Decompressor {
    fn decompress(&self, from: &File, into: &Option<File>) -> OuchResult<Vec<PathBuf>>;
}