use std::path::PathBuf;

use crate::extensions::CompressionExtension;

#[derive(PartialEq, Eq, Debug)]
pub enum File {
    WithExtension((PathBuf, CompressionExtension)),
    WithoutExtension(PathBuf),
}