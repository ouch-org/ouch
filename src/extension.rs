use std::{
    convert::TryFrom,
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
};

use crate::error;
use CompressionFormat::*;

/// Represents the extension of a file, but only really caring about
/// compression formats (and .tar).
/// Ex.: Extension::new("file.tar.gz") == Extension { first_ext: Some(Tar), second_ext: Gzip }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    pub first_ext: Option<CompressionFormat>,
    pub second_ext: CompressionFormat,
}

pub fn get_extension_from_filename(filename: &str) -> Option<(&str, &str)> {
    let path = Path::new(filename);

    let ext = path.extension().and_then(OsStr::to_str)?;

    let previous_extension = path
        .file_stem()
        .and_then(OsStr::to_str)
        .and_then(get_extension_from_filename);

    if let Some((_, prev)) = previous_extension {
        Some((prev, ext))
    } else {
        Some(("", ext))
    }
}

impl From<CompressionFormat> for Extension {
    fn from(second_ext: CompressionFormat) -> Self {
        Self {
            first_ext: None,
            second_ext,
        }
    }
}

impl Extension {
    pub fn new(filename: &str) -> error::OuchResult<Self> {
        let ext_from_str = |ext| match ext {
            "zip" => Ok(Zip),
            "tar" => Ok(Tar),
            "gz" => Ok(Gzip),
            "bz" | "bz2" => Ok(Bzip),
            "lz" | "lzma" => Ok(Lzma),
            other => Err(error::Error::UnknownExtensionError(other.into())),
        };

        let (first_ext, second_ext) = match get_extension_from_filename(filename) {
            Some(extension_tuple) => match extension_tuple {
                ("", snd) => (None, snd),
                (fst, snd) => (Some(fst), snd),
            },
            None => return Err(error::Error::MissingExtensionError(filename.into())),
        };

        let (first_ext, second_ext) = match (first_ext, second_ext) {
            (None, snd) => {
                let ext = ext_from_str(snd)?;
                (None, ext)
            }
            (Some(fst), snd) => {
                let snd = ext_from_str(snd)?;
                let fst = ext_from_str(fst).ok();
                (fst, snd)
            }
        };

        Ok(Self {
            first_ext,
            second_ext,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
/// Accepted extensions for input and output
pub enum CompressionFormat {
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
}

fn extension_from_os_str(ext: &OsStr) -> Result<CompressionFormat, error::Error> {
    // let ext = Path::new(ext);

    let ext = match ext.to_str() {
        Some(str) => str,
        None => return Err(error::Error::InvalidUnicode),
    };

    match ext {
        "zip" => Ok(Zip),
        "tar" => Ok(Tar),
        "gz" => Ok(Gzip),
        "bz" | "bz2" => Ok(Bzip),
        "lzma" | "lz" => Ok(Lzma),
        other => Err(error::Error::UnknownExtensionError(other.into())),
    }
}

impl TryFrom<&PathBuf> for CompressionFormat {
    type Error = error::Error;

    fn try_from(ext: &PathBuf) -> Result<Self, Self::Error> {
        let ext = match ext.extension() {
            Some(ext) => ext,
            None => {
                return Err(error::Error::MissingExtensionError(String::new()));
            }
        };
        extension_from_os_str(ext)
    }
}

impl TryFrom<&str> for CompressionFormat {
    type Error = error::Error;

    fn try_from(filename: &str) -> Result<Self, Self::Error> {
        let filename = Path::new(filename);
        let ext = match filename.extension() {
            Some(ext) => ext,
            None => return Err(error::Error::MissingExtensionError(String::new())),
        };

        extension_from_os_str(ext)
    }
}

impl Display for CompressionFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Gzip => ".gz",
                Bzip => ".bz",
                Lzma => ".lz",
                Tar => ".tar",
                Zip => ".zip",
            }
        )
    }
}
