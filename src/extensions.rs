use std::{
    convert::TryFrom,
    path::{Path, PathBuf},
};

use crate::error;
#[derive(Debug)]
/// Accepted extensions for input and output
pub enum CompressionExtension {
    // .gz
    Gzip,
    // .bz
    Bzip,
    // .lzma
    Lzma,
    // .tar (technically not a compression extension, but will do for now)
    Tar,
    // .zip
    Zip,
    // Not a supported compressed file extension (any other file)
    // TODO: it makes no sense for this variant to exist here
    // NotCompressed
}

impl TryFrom<&PathBuf> for CompressionExtension {
    type Error = error::Error;

    fn try_from(ext: &PathBuf) -> Result<Self, Self::Error> {
        use CompressionExtension::*;

        let ext = match ext.extension() {
            Some(ext) => ext,
            None => {
                return Err(error::Error::MissingExtensionError(String::new()));
            }
        };

        let ext = match ext.to_str() {
            Some(str) => str,
            None => return Err(error::Error::InvalidUnicode),
        };

        match ext {
            "zip" => Ok(Zip),
            "tar" => Ok(Tar),
            other => Err(error::Error::UnknownExtensionError(other.into())),
        }
    }
}

impl TryFrom<&str> for CompressionExtension {
    type Error = error::Error;

    fn try_from(filename: &str) -> Result<Self, Self::Error> {
        use CompressionExtension::*;

        let filename = Path::new(filename);
        let ext = match filename.extension() {
            Some(ext) => ext,
            None => return Err(error::Error::MissingExtensionError(String::new())),
        };

        let ext = match ext.to_str() {
            Some(str) => str,
            None => return Err(error::Error::InvalidUnicode),
        };

        match ext {
            "zip" => Ok(Zip),
            "tar" => Ok(Tar),
            other => Err(error::Error::UnknownExtensionError(other.into())),
        }
    }
}
