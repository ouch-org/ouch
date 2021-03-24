use std::{fs, io::Read};

use colored::Colorize;

use crate::{error::OuchResult, utils};
use crate::file::File;

use super::decompressor::{DecompressionResult, Decompressor};

pub struct LzmaDecompressor {}

impl LzmaDecompressor {
    fn extract_to_memory(from: File) -> OuchResult<Vec<u8>> {
        let mut ret = vec![];

        let from_path = from.path;
        if !from_path.exists() {
            eprintln!("{}: could not find {:?}", "error".red(), from_path);
        }

        let input_bytes = fs::read(&from_path)?;


        xz2::read::XzDecoder::new_multi_decoder(&*input_bytes)
            .read_to_end(&mut ret)?;

        println!("{}: extracted {:?} into memory. ({} bytes)", "info".yellow(), from_path, ret.len());

        Ok(ret)
    }
}

impl Decompressor for LzmaDecompressor {
    fn decompress(&self, from: File, into: &Option<File>) -> OuchResult<DecompressionResult> {
        let destination_path = utils::get_destination_path(into);

        utils::create_path_if_non_existent(destination_path)?;
        
        Ok(
            DecompressionResult::FileInMemory(
                Self::extract_to_memory(from)?
            ) 
        )
    }
}