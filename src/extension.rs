//! Our representation of all the supported compression formats.

use std::{ffi::OsStr, fmt, path::Path};

use self::CompressionFormat::*;

#[derive(Clone, PartialEq, Eq, Debug)]
/// Accepted extensions for input and output
pub enum CompressionFormat {
    Gzip,  // .gz
    Bzip,  // .bz
    Lzma,  // .lzma
    Tar,   // .tar (technically not a compression extension, but will do for now)
    Tgz,   // .tgz
    Tbz,   // .tbz
    Tlzma, // .tlzma
    Tzst,  // .tzst
    Zstd,  // .zst
    Zip,   // .zip
}

impl CompressionFormat {
    pub fn is_archive_format(&self) -> bool {
        matches!(self, Tar | Tgz | Tbz | Tlzma | Tzst | Zip)
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
                Tgz => ".tgz",
                Tbz => ".tbz",
                Tlzma => ".tlz",
                Tzst => ".tzst",
                Zip => ".zip",
            }
        )
    }
}
impl CompressionFormat {
    pub fn is_archive(&self) -> bool {
        matches!(self, Tar | Tgz | Tbz | Tlzma | Tzst | Zip)
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
    while let Some(extension) = path.extension().and_then(OsStr::to_str) {
        extensions.push(match extension {
            "tar" => Tar,
            "tgz" => Tgz,
            "tbz" | "tbz2" => Tbz,
            "txz" | "tlz" | "tlzma" => Tlzma,
            "tzst" => Tzst,
            "zip" => Zip,
            "bz" | "bz2" => Bzip,
            "gz" => Gzip,
            "xz" | "lzma" | "lz" => Lzma,
            "zst" => Zstd,
            _ => break,
        });

        // Update for the next iteration
        path = if let Some(stem) = path.file_stem() { Path::new(stem) } else { Path::new("") };
    }
    // Put the extensions in the correct order: left to right
    extensions.reverse();

    (path, extensions)
}

pub fn extensions_from_path(path: &Path) -> Vec<CompressionFormat> {
    let (_, extensions) = separate_known_extensions_from_name(path);
    extensions
}
