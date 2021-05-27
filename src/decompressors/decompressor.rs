use std::path::PathBuf;

use crate::{file::File, oof};

pub enum DecompressionResult {
    FilesUnpacked(Vec<PathBuf>),
    FileInMemory(Vec<u8>),
}

pub trait Decompressor {
    fn decompress(
        &self,
        from: File,
        into: &Option<File>,
        flags: &oof::Flags,
    ) -> crate::Result<DecompressionResult>;
}
