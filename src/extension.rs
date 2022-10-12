//! Our representation of all the supported compression formats.

use std::{ffi::OsStr, fmt, path::Path};

use self::CompressionFormat::*;
use crate::warning;

/// A wrapper around `CompressionFormat` that allows combinations like `tgz`
#[derive(Debug, Clone, Eq)]
#[non_exhaustive]
pub struct Extension {
    /// One extension like "tgz" can be made of multiple CompressionFormats ([Tar, Gz])
    pub compression_formats: &'static [CompressionFormat],
    /// The input text for this extension, like "tgz", "tar" or "xz"
    pub display_text: String,
}
// The display_text should be ignored when comparing extensions
impl PartialEq for Extension {
    fn eq(&self, other: &Self) -> bool {
        self.compression_formats == other.compression_formats
    }
}

impl Extension {
    /// # Panics:
    ///   Will panic if `formats` is empty
    pub fn new(formats: &'static [CompressionFormat], text: impl ToString) -> Self {
        assert!(!formats.is_empty());
        Self {
            compression_formats: formats,
            display_text: text.to_string(),
        }
    }

    /// Checks if the first format in `compression_formats` is an archive
    pub fn is_archive(&self) -> bool {
        // Safety: we check that `compression_formats` is not empty in `Self::new`
        self.compression_formats[0].is_archive_format()
    }
}

impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.display_text.fmt(f)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
/// Accepted extensions for input and output
pub enum CompressionFormat {
    /// .gz
    Gzip,
    /// .bz .bz2
    Bzip,
    /// .lz4
    Lz4,
    /// .xz .lzma
    Lzma,
    /// .sz
    Snappy,
    /// tar, tgz, tbz, tbz2, txz, tlz4, tlzma, tsz, tzst
    Tar,
    /// .zst
    Zstd,
    /// .zip
    Zip,
}

impl CompressionFormat {
    /// Currently supported archive formats are .tar (and aliases to it) and .zip
    pub fn is_archive_format(&self) -> bool {
        // Keep this match like that without a wildcard `_` so we don't forget to update it
        match self {
            Tar | Zip => true,
            Gzip => false,
            Bzip => false,
            Lz4 => false,
            Lzma => false,
            Snappy => false,
            Zstd => false,
        }
    }
}

impl fmt::Display for CompressionFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self {
            Gzip => ".gz",
            Bzip => ".bz",
            Zstd => ".zst",
            Lz4 => ".lz4",
            Lzma => ".lz",
            Snappy => ".sz",
            Tar => ".tar",
            Zip => ".zip",
        };

        write!(f, "{text}")
    }
}

pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    "tar", "tgz", "tbz", "tlz4", "txz", "tzlma", "tsz", "tzst", "zip", "bz", "bz2", "gz", "lz4", "xz", "lzma", "sz",
    "zst",
];

/// Extracts extensions from a path.
///
/// Returns both the remaining path and the list of extension objects
pub fn separate_known_extensions_from_name(mut path: &Path) -> (&Path, Vec<Extension>) {
    let mut extensions = vec![];

    if let Some(file_stem) = path.file_stem().and_then(OsStr::to_str) {
        let file_stem = file_stem.trim_matches('.');

        if SUPPORTED_EXTENSIONS.contains(&file_stem) {
            warning!("Received a file with name '{file_stem}', but {file_stem} was expected as the extension.");
        }
    }

    // While there is known extensions at the tail, grab them
    while let Some(extension) = path.extension().and_then(OsStr::to_str) {
        let formats: &[CompressionFormat] = match extension {
            "tar" => &[Tar],
            "tgz" => &[Tar, Gzip],
            "tbz" | "tbz2" => &[Tar, Bzip],
            "tlz4" => &[Tar, Lz4],
            "txz" | "tlzma" => &[Tar, Lzma],
            "tsz" => &[Tar, Snappy],
            "tzst" => &[Tar, Zstd],
            "zip" => &[Zip],
            "bz" | "bz2" => &[Bzip],
            "gz" => &[Gzip],
            "lz4" => &[Lz4],
            "xz" | "lzma" => &[Lzma],
            "sz" => &[Snappy],
            "zst" => &[Zstd],
            _ => break,
        };

        let extension = Extension::new(formats, extension);
        extensions.push(extension);

        // Update for the next iteration
        path = if let Some(stem) = path.file_stem() {
            Path::new(stem)
        } else {
            Path::new("")
        };
    }
    // Put the extensions in the correct order: left to right
    extensions.reverse();

    (path, extensions)
}

/// Extracts extensions from a path, return only the list of extension objects
pub fn extensions_from_path(path: &Path) -> Vec<Extension> {
    let (_, extensions) = separate_known_extensions_from_name(path);
    extensions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extensions_from_path() {
        use CompressionFormat::*;
        let path = Path::new("bolovo.tar.gz");

        let extensions: Vec<Extension> = extensions_from_path(path);
        let formats: Vec<CompressionFormat> = flatten_compression_formats(&extensions);

        assert_eq!(formats, vec![Tar, Gzip]);
    }
}

// Panics if formats has an empty list of compression formats
pub fn split_first_compression_format(formats: &[Extension]) -> (CompressionFormat, Vec<CompressionFormat>) {
    let mut extensions: Vec<CompressionFormat> = flatten_compression_formats(formats);
    let first_extension = extensions.remove(0);
    (first_extension, extensions)
}

pub fn flatten_compression_formats(extensions: &[Extension]) -> Vec<CompressionFormat> {
    extensions
        .iter()
        .flat_map(|extension| extension.compression_formats.iter())
        .copied()
        .collect()
}
