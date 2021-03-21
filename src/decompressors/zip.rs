use std::{
    fs,
    path::{Path, PathBuf},
};

use colored::Colorize;
use zip;

use crate::{error::{self, OuchResult}, utils};
use crate::file::File;

use super::decompressor::Decompressor;

pub struct ZipDecompressor {}

impl ZipDecompressor {
    fn unpack_files(from: &Path, into: &Path) -> OuchResult<Vec<PathBuf>> {
        // placeholder return
        Err(error::Error::IOError)
    }
}


impl Decompressor for ZipDecompressor {
    fn decompress(&self, from: &File, into: &Option<File>) -> OuchResult<Vec<PathBuf>> {
        let destination_path = utils::get_destination_path(into);

        utils::create_path_if_non_existent(destination_path)?;

        let files_unpacked = Self::unpack_files(&from.path, destination_path)?;

        // placeholder return
        Err(error::Error::IOError)
    }
}