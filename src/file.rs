use std::path::Path;

use crate::extension::Extension;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File<'a> {
    /// File's (relative) path
    pub path: &'a Path,
    /// The bytes that compose the file.
    /// Only used when the whole file is kept in-memory
    pub contents_in_memory: Option<Vec<u8>>,
    /// Note: extension here might be a misleading name since
    /// we don't really care about any extension other than supported compression ones.
    ///
    /// So, for example, if a file has pathname "image.jpeg", it does have a JPEG extension but will
    /// be represented as a None over here since that's not an extension we're particularly interested in
    pub extension: Option<Extension>,
}

impl<'a> File<'a> {
    pub fn from(path: &'a Path) -> crate::Result<Self> {
        let extension = Extension::from(path.as_ref()).ok();

        Ok(File { path, contents_in_memory: None, extension })
    }
}
