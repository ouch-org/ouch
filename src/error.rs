use std::fmt;

use colored::Colorize;

#[derive(PartialEq, Eq, Debug)]
pub enum Error {
    UnknownExtensionError(String),
    MissingExtensionError(String),
    // TODO: get rid of this error variant
    InvalidUnicode,
    InvalidInput,
    IOError,
    FileNotFound,
    AlreadyExists,
    InvalidZipArchive(&'static str),
    PermissionDenied,
    UnsupportedZipArchive(&'static str),
    InputsMustHaveBeenDecompressible(String),
}

pub type OuchResult<T> = Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {

        match self {
            Error::MissingExtensionError(filename) => {
                write!(f, "cannot compress to \'{}\', likely because it has an unsupported (or missing) extension.", filename)
            },
            Error::InputsMustHaveBeenDecompressible(file) => {
                write!(f, "file '{}' is not decompressible", file.red())
            },
            // TODO: find out a way to attach the missing file in question here
            Error::FileNotFound => {
                write!(f, "file not found!")
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
            std::io::ErrorKind::NotFound => Self::FileNotFound,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            _other => {
                println!("{}: {:#?}", "IO error".red(), err);
                Self::IOError
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
            FileNotFound => Self::FileNotFound,
            UnsupportedArchive(filename) => Self::UnsupportedZipArchive(filename)
        }
    }
}

// impl From<niffler::error::Error> for Error {
//     fn from(err: niffler::error::Error) -> Self {
//         use niffler::error::Error as NifErr;
//         match err {
//             NifErr::FeatureDisabled => {
//                 // Ouch is using Niffler with all its features so
//                 // this should be unreachable.
//                 unreachable!();
//             },
//             NifErr::FileTooShort => Self::FileTooShort,
//             NifErr::IOError(io_err) => Self::from(io_err)
//         }
//     }
// }

impl From<walkdir::Error> for Error {
    fn from(err: walkdir::Error) -> Self {
        eprintln!("{}: {}", "error".red(), err);

        Self::InvalidInput
    }
}