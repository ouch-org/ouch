use std::{
    convert::TryFrom,
    ffi::OsStr,
    fmt::Display,
    path::{Path, PathBuf},
};

use CompressionFormat::*;

use crate::{debug, utils::to_utf};

/// Represents the extension of a file, but only really caring about
/// compression formats (and .tar).
/// Ex.: Extension::new("file.tar.gz") == Extension { first_ext: Some(Tar), second_ext: Gzip }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Extension {
    pub first_ext: Option<CompressionFormat>,
    pub second_ext: CompressionFormat,
}

pub fn get_extension_from_filename(file_name: &OsStr) -> Option<(&OsStr, &OsStr)> {
    let path = Path::new(file_name);

    let ext = path.extension()?;

    let previous_extension = path.file_stem().and_then(get_extension_from_filename);

    if let Some((_, prev)) = previous_extension {
        Some((prev, ext))
    } else {
        Some((OsStr::new(""), ext))
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
    pub fn from(file_name: &OsStr) -> crate::Result<Self> {
        let compression_format_from = |ext: &OsStr| match ext {
            _ if ext == "zip" => Ok(Zip),
            _ if ext == "tar" => Ok(Tar),
            _ if ext == "gz" => Ok(Gzip),
            _ if ext == "bz" || ext == "bz2" => Ok(Bzip),
            _ if ext == "xz" || ext == "lz" || ext == "lzma" => Ok(Lzma),
            other => Err(crate::Error::UnknownExtensionError(to_utf(other))),
        };

        let (first_ext, second_ext) = match get_extension_from_filename(&file_name) {
            Some(extension_tuple) => match extension_tuple {
                (os_str, snd) if os_str.is_empty() => (None, snd),
                (fst, snd) => (Some(fst), snd),
            },
            None => return Err(crate::Error::MissingExtensionError(to_utf(file_name))),
        };

        let (first_ext, second_ext) = match (first_ext, second_ext) {
            (None, snd) => {
                let ext = compression_format_from(snd)?;
                (None, ext)
            }
            (Some(fst), snd) => {
                let snd = compression_format_from(snd)?;
                let fst = compression_format_from(fst).ok();
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

fn extension_from_os_str(ext: &OsStr) -> Result<CompressionFormat, crate::Error> {
    // let ext = Path::new(ext);

    let ext = match ext.to_str() {
        Some(str) => str,
        None => return Err(crate::Error::InvalidUnicode),
    };

    match ext {
        "zip" => Ok(Zip),
        "tar" => Ok(Tar),
        "gz" => Ok(Gzip),
        "bz" | "bz2" => Ok(Bzip),
        "xz" | "lzma" | "lz" => Ok(Lzma),
        other => Err(crate::Error::UnknownExtensionError(other.into())),
    }
}

impl TryFrom<&PathBuf> for CompressionFormat {
    type Error = crate::Error;

    fn try_from(ext: &PathBuf) -> Result<Self, Self::Error> {
        let ext = match ext.extension() {
            Some(ext) => ext,
            None => {
                return Err(crate::Error::MissingExtensionError(String::new()));
            }
        };
        extension_from_os_str(ext)
    }
}

impl TryFrom<&str> for CompressionFormat {
    type Error = crate::Error;

    fn try_from(file_name: &str) -> Result<Self, Self::Error> {
        let file_name = Path::new(file_name);
        let ext = match file_name.extension() {
            Some(ext) => ext,
            None => return Err(crate::Error::MissingExtensionError(String::new())),
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
