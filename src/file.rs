use std::path::PathBuf;

use crate::extension::{CompressionFormat, Extension};


#[derive(Debug, PartialEq, Eq)]
pub struct File {
    /// File's (relative) path
    pub path: PathBuf,
    /// Note: extension here might be a misleading name since
    /// we don't really care about any extension other than supported compression ones.
    ///
    /// So, for example, if a file has pathname "image.jpeg", it does have a JPEG extension but will
    /// be represented as a None over here since that's not an extension we're particularly interested in
    pub extension: Option<Extension>
}

impl From<(PathBuf, CompressionFormat)> for File {
    fn from((path, format): (PathBuf, CompressionFormat)) -> Self {
        Self {
            path,
            extension: Some(Extension::from(format)),
        }
    }
}