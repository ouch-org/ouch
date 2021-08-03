use std::{
    ffi::OsStr,
    fmt,
    path::{Path, PathBuf},
};

use CompressionFormat::*;

use crate::utils;

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

impl Extension {
    pub fn from(file_name: &OsStr) -> crate::Result<Self> {
        let compression_format_from = |ext: &OsStr| match ext {
            _ if ext == "zip" => Ok(Zip),
            _ if ext == "tar" => Ok(Tar),
            _ if ext == "gz" => Ok(Gzip),
            _ if ext == "bz" || ext == "bz2" => Ok(Bzip),
            _ if ext == "xz" || ext == "lz" || ext == "lzma" => Ok(Lzma),
            other => Err(crate::Error::UnknownExtensionError(utils::to_utf(other))),
        };

        let (first_ext, second_ext) = match get_extension_from_filename(file_name) {
            Some(extension_tuple) => match extension_tuple {
                (os_str, snd) if os_str.is_empty() => (None, snd),
                (fst, snd) => (Some(fst), snd),
            },
            None => return Err(crate::Error::MissingExtensionError(PathBuf::from(file_name))),
        };

        let (first_ext, second_ext) = match (first_ext, second_ext) {
            (None, snd) => {
                let ext = compression_format_from(snd)?;
                (None, ext)
            },
            (Some(fst), snd) => {
                let snd = compression_format_from(snd)?;
                let fst = compression_format_from(fst).ok();
                (fst, snd)
            },
        };

        Ok(Self { first_ext, second_ext })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
/// Accepted extensions for input and output
pub enum CompressionFormat {
    Gzip, // .gz
    Bzip, // .bz
    Lzma, // .lzma
    Tar,  // .tar (technically not a compression extension, but will do for now)
    Zip,  // .zip
}

impl fmt::Display for CompressionFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            Gzip => ".gz",
            Bzip => ".bz",
            Lzma => ".lz",
            Tar => ".tar",
            Zip => ".zip",
        })
    }
}

pub fn separate_known_extensions_from_name(mut path: &Path) -> (&Path, Vec<CompressionFormat>) {
    // // TODO: check for file names with the name of an extension
    // // TODO2: warn the user that currently .tar.gz is a .gz file named .tar
    //
    // let all = ["tar", "zip", "bz", "bz2", "gz", "xz", "lzma", "lz"];
    // if path.file_name().is_some() && all.iter().any(|ext| path.file_name().unwrap() == *ext) {
    //     todo!("we found a extension in the path name instead, what to do with this???");
    // }

    let mut extensions = vec![];

    // While there is known extensions at the tail, grab them
    while let Some(extension) = path.extension() {
        let extension = match () {
            _ if extension == "tar" => Tar,
            _ if extension == "zip" => Zip,
            _ if extension == "bz" => Bzip,
            _ if extension == "gz" || extension == "bz2" => Gzip,
            _ if extension == "xz" || extension == "lzma" || extension == "lz" => Lzma,
            _ => break,
        };

        extensions.push(extension);

        // Update for the next iteration
        path = if let Some(stem) = path.file_stem() { Path::new(stem) } else { Path::new("") };
    }
    // Put the extensions in the correct order: left to right
    extensions.reverse();

    (path, extensions)
}
