use std::{io::Read, path::Path};

use colored::Colorize;
use niffler;

use crate::file::File;
use crate::{
    error::{self, OuchResult},
    utils,
};

use super::decompressor::Decompressor;
use super::decompressor::DecompressionResult;

pub struct NifflerDecompressor {}

impl NifflerDecompressor {
    fn unpack_file(from: &Path) -> OuchResult<Vec<u8>> {
        
        println!("{}: trying to decompress {:?}", "info".yellow(), from);

        let file = std::fs::read(from)?;

        let (mut reader, compression) = niffler::get_reader(Box::new(&file[..]))?;

        match compression {
            niffler::Format::No => {
                return Err(error::Error::InvalidInput);
            },
            other => {
                println!("{}: {:?} detected.", "info".yellow(), other);
            }
        }

        let mut buffer = Vec::new();
        let bytes_read = reader.read_to_end(&mut buffer)?;

        println!("{}: {:?} extracted into memory ({} bytes).", "info".yellow(), from, bytes_read);

        Ok(buffer)
    }
}

impl Decompressor for NifflerDecompressor {
    fn decompress(&self, from: File, into: &Option<File>) -> OuchResult<DecompressionResult> {
        let destination_path = utils::get_destination_path(into);

        utils::create_path_if_non_existent(destination_path)?;

        let bytes = Self::unpack_file(&from.path)?;

        Ok(DecompressionResult::FileInMemory(bytes))
    }
}
