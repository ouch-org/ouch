//! Our representation of all the supported compression formats.

use std::{ffi::OsStr, fmt, path::Path};

use self::CompressionFormat::*;

/// A wrapper around `CompressionFormat` that allows combinations like `tgz`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extension {
    pub compression_formats: Vec<CompressionFormat>,
    pub display_text: String,
}

impl Extension {
    /// # Panics:
    ///   Will panic if `formats` is empty
    pub fn new(formats: impl Into<Vec<CompressionFormat>>, text: impl Into<String>) -> Self {
        let formats = formats.into();
        assert!(!formats.is_empty());
        Self { compression_formats: formats, display_text: text.into() }
    }

    /// Checks if the first format in `compression_formats` is an archive
    pub fn is_archive(&self) -> bool {
        // Safety: we check that `compression_formats` is not empty in `Self::new`
        self.compression_formats[0].is_archive_format()
    }

    pub fn iter(&self) -> impl Iterator<Item = &CompressionFormat> {
        self.compression_formats.iter()
    }
}

impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display_text)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
/// Accepted extensions for input and output
pub enum CompressionFormat {
    Gzip, // .gz
    Bzip, // .bz
    Lzma, // .lzma
    Tar,  // .tar (technically not a compression extension, but will do for now)
    Zstd, // .zst
    Zip,  // .zip
}

impl CompressionFormat {
    pub fn is_archive_format(&self) -> bool {
        // Keep this match like that without a wildcard `_` so we don't forget to update it
        match self {
            Tar | Zip => true,
            Gzip => false,
            Bzip => false,
            Lzma => false,
            Zstd => false,
        }
    }
}

impl fmt::Display for CompressionFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Gzip => ".gz",
                Bzip => ".bz",
                Zstd => ".zst",
                Lzma => ".lz",
                Tar => ".tar",
                Zip => ".zip",
            }
        )
    }
}

pub fn separate_known_extensions_from_name(mut path: &Path) -> (&Path, Vec<Extension>) {
    // // TODO: check for file names with the name of an extension
    // // TODO2: warn the user that currently .tar.gz is a .gz file named .tar
    //
    // let all = ["tar", "zip", "bz", "bz2", "gz", "xz", "lzma", "lz"];
    // if path.file_name().is_some() && all.iter().any(|ext| path.file_name().unwrap() == *ext) {
    //     todo!("we found a extension in the path name instead, what to do with this???");
    // }

    let mut extensions = vec![];

    // While there is known extensions at the tail, grab them
    while let Some(extension) = path.extension().and_then(OsStr::to_str) {
        extensions.push(match extension {
            "tar" => Extension::new([Tar], extension),
            "tgz" => Extension::new([Tar, Gzip], extension),
            "tbz" | "tbz2" => Extension::new([Tar, Bzip], extension),
            "txz" | "tlz" | "tlzma" => Extension::new([Tar, Lzma], extension),
            "tzst" => Extension::new([Tar, Zstd], ".tzst"),
            "zip" => Extension::new([Zip], extension),
            "bz" | "bz2" => Extension::new([Bzip], extension),
            "gz" => Extension::new([Gzip], extension),
            "xz" | "lzma" | "lz" => Extension::new([Lzma], extension),
            "zst" => Extension::new([Zstd], extension),
            _ => break,
        });

        // Update for the next iteration
        path = if let Some(stem) = path.file_stem() { Path::new(stem) } else { Path::new("") };
    }
    // Put the extensions in the correct order: left to right
    extensions.reverse();

    (path, extensions)
}

pub fn extensions_from_path(path: &Path) -> Vec<Extension> {
    let (_, extensions) = separate_known_extensions_from_name(path);
    extensions
}
