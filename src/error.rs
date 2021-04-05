use std::{fmt, path::PathBuf};

use colored::Colorize;

#[derive(PartialEq, Eq)]
pub enum Error {
    UnknownExtensionError(String),
    MissingExtensionError(String),
    // TODO: get rid of this error variant
    InvalidUnicode,
    InvalidInput,
    IoError,
    FileNotFound(PathBuf),
    AlreadyExists,
    InvalidZipArchive(&'static str),
    PermissionDenied,
    UnsupportedZipArchive(&'static str),
    // InputsMustBeDecompressible(PathBuf),
    InternalError,
    CompressingRootFolder,
    WalkdirError,
}

pub type Result<T> = std::result::Result<T, Error>;

// impl std::error::Error for Error {
//     fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//         // TODO: get rid of PartialEq and Eq in self::Error in order to
//         // correctly use `source`.
//         None
//     }
// }

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::MissingExtensionError(filename) => {
                write!(f, "{} ", "[ERROR]".red())?;
                write!(f, "cannot compress to \'{}\', likely because it has an unsupported (or missing) extension.", filename)
            }
            // Error::InputsMustBeDecompressible(file) => {
            //     write!(f, "{} ", "[ERROR]".red())?;
            //     write!(f, "file '{:?}' is not decompressible", file)
            // }
            Error::WalkdirError => {
                // Already printed in the From block
                write!(f, "")
            }
            Error::FileNotFound(file) => {
                write!(f, "{} ", "[ERROR]".red())?;
                // TODO: check if file == ""
                write!(f, "file {:?} not found!", file)
            }
            Error::CompressingRootFolder => {
                write!(f, "{} ", "[ERROR]".red())?;
                let spacing = "        ";
                writeln!(f, "It seems you're trying to compress the root folder.")?;
                writeln!(
                    f,
                    "{}This is unadvisable since ouch does compressions in-memory.",
                    spacing
                )?;
                write!(
                    f,
                    "{}Use a more appropriate tool for this, such as {}.",
                    spacing,
                    "rsync".green()
                )
            }
            Error::InternalError => {
                write!(f, "{} ", "[ERROR]".red())?;
                write!(f, "You've reached an internal error! This really should not have happened.\nPlease file an issue at {}", "https://github.com/vrmiguel/ouch".green())
            }
            _err => {
                // TODO
                write!(f, "")
            }
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => panic!("{}", err),
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            _other => {
                println!("{} {}", "[IO error]".red(), err);
                Self::IoError
            }
        }
    }
}

impl From<zip::result::ZipError> for Error {
    fn from(err: zip::result::ZipError) -> Self {
        use zip::result::ZipError::*;
        match err {
            Io(io_err) => Self::from(io_err),
            InvalidArchive(filename) => Self::InvalidZipArchive(filename),
            FileNotFound => Self::FileNotFound("".into()),
            UnsupportedArchive(filename) => Self::UnsupportedZipArchive(filename),
        }
    }
}

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Self {
        eprintln!("{} {}", "[ERROR]".red(), err);
        Self::WalkdirError
    }
}

impl From<oof::OofError> for Error {
    fn from(_err: oof::OofError) -> Self {
        todo!("We need to implement this properly");
    }
}
