//! Our representation of all the supported compression formats.

use std::{ffi::OsStr, fmt, path::Path};

use bstr::ByteSlice;

use self::CompressionFormat::*;
use crate::{error::Error, warning};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["tar", "zip", "bz", "bz2", "gz", "lz4", "xz", "lzma", "sz", "zst"];
pub const SUPPORTED_ALIASES: &[&str] = &["tgz", "tbz", "tlz4", "txz", "tzlma", "tsz", "tzst"];
pub const PRETTY_SUPPORTED_EXTENSIONS: &str = "tar, zip, bz, bz2, gz, lz4, xz, lzma, sz, zst";
pub const PRETTY_SUPPORTED_ALIASES: &str = "tgz, tbz, tlz4, txz, tzlma, tsz, tzst";

/// A wrapper around `CompressionFormat` that allows combinations like `tgz`
#[derive(Debug, Clone, Eq)]
#[non_exhaustive]
pub struct Extension {
    /// One extension like "tgz" can be made of multiple CompressionFormats ([Tar, Gz])
    pub compression_formats: &'static [CompressionFormat],
    /// The input text for this extension, like "tgz", "tar" or "xz"
    display_text: String,
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
    fn is_archive_format(&self) -> bool {
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

fn to_extension(ext: &[u8]) -> Option<Extension> {
    Some(Extension::new(
        match ext {
            b"tar" => &[Tar],
            b"tgz" => &[Tar, Gzip],
            b"tbz" | b"tbz2" => &[Tar, Bzip],
            b"tlz4" => &[Tar, Lz4],
            b"txz" | b"tlzma" => &[Tar, Lzma],
            b"tsz" => &[Tar, Snappy],
            b"tzst" => &[Tar, Zstd],
            b"zip" => &[Zip],
            b"bz" | b"bz2" => &[Bzip],
            b"gz" => &[Gzip],
            b"lz4" => &[Lz4],
            b"xz" | b"lzma" => &[Lzma],
            b"sz" => &[Snappy],
            b"zst" => &[Zstd],
            _ => return None,
        },
        ext.to_str_lossy(),
    ))
}

fn split_extension(name: &mut &[u8]) -> Option<Extension> {
    let (new_name, ext) = name.rsplit_once_str(b".")?;
    if matches!(new_name, b"" | b"." | b"..") {
        return None;
    }
    let ext = to_extension(ext)?;
    *name = new_name;
    Some(ext)
}

pub fn parse_format(fmt: &OsStr) -> crate::Result<Vec<Extension>> {
    let fmt = <[u8] as ByteSlice>::from_os_str(fmt).ok_or_else(|| Error::InvalidFormat {
        reason: "Invalid UTF-8".into(),
    })?;

    let mut extensions = Vec::new();
    for extension in fmt.split_str(b".") {
        let extension = to_extension(extension).ok_or_else(|| Error::InvalidFormat {
            reason: format!("Unsupported extension: {}", extension.to_str_lossy()),
        })?;
        extensions.push(extension);
    }

    Ok(extensions)
}

/// Extracts extensions from a path.
///
/// Returns both the remaining path and the list of extension objects
pub fn separate_known_extensions_from_name(path: &Path) -> (&Path, Vec<Extension>) {
    let mut extensions = vec![];

    let Some(mut name) = path.file_name().and_then(<[u8] as ByteSlice>::from_os_str) else {
        return (path, extensions);
    };

    // While there is known extensions at the tail, grab them
    while let Some(extension) = split_extension(&mut name) {
        extensions.insert(0, extension);
    }

    if let Ok(name) = name.to_str() {
        let file_stem = name.trim_matches('.');
        if SUPPORTED_EXTENSIONS.contains(&file_stem) || SUPPORTED_ALIASES.contains(&file_stem) {
            warning!("Received a file with name '{file_stem}', but {file_stem} was expected as the extension.");
        }
    }

    (name.to_path().unwrap(), extensions)
}

/// Extracts extensions from a path, return only the list of extension objects
pub fn extensions_from_path(path: &Path) -> Vec<Extension> {
    let (_, extensions) = separate_known_extensions_from_name(path);
    extensions
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

/// Builds a suggested output file in scenarios where the user tried to compress
/// a folder into a non-archive compression format, for error message purposes
///
/// E.g.: `build_suggestion("file.bz.xz", ".tar")` results in `Some("file.tar.bz.xz")`
pub fn build_archive_file_suggestion(path: &Path, suggested_extension: &str) -> Option<String> {
    let path = path.to_string_lossy();
    let mut rest = &*path;
    let mut position_to_insert = 0;

    // Walk through the path to find the first supported compression extension
    while let Some(pos) = rest.find('.') {
        // Use just the text located after the dot we found
        rest = &rest[pos + 1..];
        position_to_insert += pos + 1;

        // If the string contains more chained extensions, clip to the immediate one
        let maybe_extension = {
            let idx = rest.find('.').unwrap_or(rest.len());
            &rest[..idx]
        };

        // If the extension we got is a supported extension, generate the suggestion
        // at the position we found
        if SUPPORTED_EXTENSIONS.contains(&maybe_extension) || SUPPORTED_ALIASES.contains(&maybe_extension) {
            let mut path = path.to_string();
            path.insert_str(position_to_insert - 1, suggested_extension);

            return Some(path);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[test]
    fn test_extensions_from_path() {
        let path = Path::new("bolovo.tar.gz");

        let extensions: Vec<Extension> = extensions_from_path(path);
        let formats: Vec<CompressionFormat> = flatten_compression_formats(&extensions);

        assert_eq!(formats, vec![Tar, Gzip]);
    }

    #[test]
    fn builds_suggestion_correctly() {
        assert_eq!(build_archive_file_suggestion(Path::new("linux.png"), ".tar"), None);
        assert_eq!(
            build_archive_file_suggestion(Path::new("linux.xz.gz.zst"), ".tar").unwrap(),
            "linux.tar.xz.gz.zst"
        );
        assert_eq!(
            build_archive_file_suggestion(Path::new("linux.pkg.xz.gz.zst"), ".tar").unwrap(),
            "linux.pkg.tar.xz.gz.zst"
        );
        assert_eq!(
            build_archive_file_suggestion(Path::new("linux.pkg.zst"), ".tar").unwrap(),
            "linux.pkg.tar.zst"
        );
        assert_eq!(
            build_archive_file_suggestion(Path::new("linux.pkg.info.zst"), ".tar").unwrap(),
            "linux.pkg.info.tar.zst"
        );
    }
}
