//! Our representation of all the supported compression formats.

use std::{ffi::OsStr, fmt, path::Path};

use self::CompressionFormat::*;

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
    pub fn new(formats: &'static [CompressionFormat], text: impl Into<String>) -> Self {
        assert!(!formats.is_empty());
        Self { compression_formats: formats, display_text: text.into() }
    }

    /// Checks if the first format in `compression_formats` is an archive
    pub fn is_archive(&self) -> bool {
        // Safety: we check that `compression_formats` is not empty in `Self::new`
        self.compression_formats[0].is_archive_format()
    }

    /// Iteration to inner compression formats, useful for flat_mapping
    pub fn iter(&self) -> impl Iterator<Item = &CompressionFormat> {
        self.compression_formats.iter()
    }
}

impl fmt::Display for Extension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    /// tar, tgz, tbz, tbz2, txz, tlz, tlz4, tlzma, tzst
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
                Lz4 => ".lz4",
                Lzma => ".lz",
                Tar => ".tar",
                Zip => ".zip",
            }
        )
    }
}

// use crate::extension::CompressionFormat::*;
//

/// Extracts extensions from a path,
/// return both the remaining path and the list of extension objects
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
        let formats: &[CompressionFormat] = match extension {
            "tar" => &[Tar],
            "tgz" => &[Tar, Gzip],
            "tbz" | "tbz2" => &[Tar, Bzip],
            "tlz4" => &[Tar, Lz4],
            "txz" | "tlz" | "tlzma" => &[Tar, Lzma],
            "tzst" => &[Tar, Zstd],
            "zip" => &[Zip],
            "bz" | "bz2" => &[Bzip],
            "gz" => &[Gzip],
            "lz4" => &[Lz4],
            "xz" | "lzma" => &[Lzma],
            "zst" => &[Zstd],
            _ => break,
        };

        let extension = Extension::new(formats, extension);
        extensions.push(extension);

        // Update for the next iteration
        path = if let Some(stem) = path.file_stem() { Path::new(stem) } else { Path::new("") };
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
        let formats: Vec<&CompressionFormat> = extensions.iter().flat_map(Extension::iter).collect::<Vec<_>>();

        assert_eq!(formats, vec![&Tar, &Gzip]);
    }
}
