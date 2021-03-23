use std::{fs::File, path::Path};

use tar::Builder;
use walkdir::WalkDir;

use crate::{decompressors::TarDecompressor, error::OuchResult};
use crate::compressors::Compressor;
use super::compressor::CompressionResult;

pub struct TarCompressor {}

impl TarCompressor {
    fn make_archive_in_memory(input_files: Vec<crate::file::File>) -> OuchResult<Vec<u8>> {
    
        let buf = Vec::new();
        let mut b = Builder::new(buf);
    
        for file in input_files {
            for entry in WalkDir::new(&file.path) {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }
                b.append_file(path, &mut File::open(path).unwrap()).unwrap();
            }
        }
        
    
        Ok(b.into_inner()?)
    }
}

impl Compressor for TarCompressor {
    fn compress(&self, from: Vec<crate::file::File>) -> OuchResult<CompressionResult> {
        Ok(CompressionResult::TarArchive(
            TarCompressor::make_archive_in_memory(from)?
        ))
    }
}

// fn compress(&self, from: Vec<File>, into: &Option<File>) -> OuchResult<CompressionResult>;