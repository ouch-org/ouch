use std::path::PathBuf;

use crate::extensions::CompressionFormat;

#[derive(PartialEq, Eq, Debug)]
pub enum File {
    WithExtension((PathBuf, CompressionFormat)),
    WithoutExtension(PathBuf),
}