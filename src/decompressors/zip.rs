use std::{
    fs,
    path::{Path, PathBuf},
};

use colored::Colorize;
use zip;

use crate::error::OuchResult;
use crate::file::File;

use super::decompressor::Decompressor;

pub struct ZipDecompressor {}

impl Decompressor for ZipDecompressor {
    fn decompress(&self, from: &File, into: &Option<File>) -> OuchResult<Vec<PathBuf>> {
        todo!()
    }
}