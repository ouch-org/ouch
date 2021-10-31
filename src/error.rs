//! Error type definitions.
//!
//! All the unexpected user-side errors should be treated in this file, that does not include
//! errors made by devs in our implementation.
//!
//! TODO: wrap `FinalError` in a variant to keep all `FinalError::display_and_crash()` function
//! calls inside of this module.

use std::{
    fmt::{self, Display},
    path::{Path, PathBuf},
};

use crate::utils::colors::*;

#[derive(Debug, PartialEq)]
pub enum Error {
    UnknownExtensionError(String),
    MissingExtensionError(PathBuf),
    IoError { reason: String },
    FileNotFound(PathBuf),
    AlreadyExists,
    InvalidZipArchive(&'static str),
    PermissionDenied { error_title: String },
    UnsupportedZipArchive(&'static str),
    InternalError,
    CompressingRootFolder,
    MissingArgumentsForCompression,
    MissingArgumentsForDecompression,
    CompressionTypo,
    WalkdirError { reason: String },
    Custom { reason: FinalError },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FinalError {
    title: String,
    details: Vec<String>,
    hints: Vec<String>,
}

impl Display for FinalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Title
        writeln!(f, "{}[ERROR]{} {}", *RED, *RESET, self.title)?;

        // Details
        for detail in &self.details {
            writeln!(f, " {}-{} {}", *WHITE, *YELLOW, detail)?;
        }

        // Hints
        if !self.hints.is_empty() {
            // Separate by one blank line.
            writeln!(f)?;
            for hint in &self.hints {
                writeln!(f, "{}hint:{} {}", *GREEN, *RESET, hint)?;
            }
        }

        write!(f, "{}", *RESET)
    }
}

impl FinalError {
    pub fn with_title(title: impl ToString) -> Self {
        Self { title: title.to_string(), details: vec![], hints: vec![] }
    }

    pub fn detail(mut self, detail: impl ToString) -> Self {
        self.details.push(detail.to_string());
        self
    }

    pub fn hint(mut self, hint: impl ToString) -> Self {
        self.hints.push(hint.to_string());
        self
    }

    pub fn into_owned(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = match self {
            Error::MissingExtensionError(filename) => {
                FinalError::with_title(format!("Cannot compress to {:?}", filename))
                    .detail("Ouch could not detect the compression format")
                    .hint("Use a supported format extension, like '.zip' or '.tar.gz'")
                    .hint("Check https://github.com/vrmiguel/ouch for a full list of supported formats")
            }
            Error::WalkdirError { reason } => FinalError::with_title(reason),
            Error::FileNotFound(file) => {
                if file == Path::new("") {
                    FinalError::with_title("file not found!")
                } else {
                    FinalError::with_title(format!("file {:?} not found!", file))
                }
            }
            Error::CompressingRootFolder => {
                FinalError::with_title("It seems you're trying to compress the root folder.")
                    .detail("This is unadvisable since ouch does compressions in-memory.")
                    .hint("Use a more appropriate tool for this, such as rsync.")
            }
            Error::MissingArgumentsForCompression => {
                FinalError::with_title("Could not compress")
                    .detail("The compress command requires at least 2 arguments")
                    .hint("You must provide:")
                    .hint("  - At least one input argument.")
                    .hint("  - The output argument.")
                    .hint("")
                    .hint("Example: `ouch compress image.png img.zip`")
            }
            Error::MissingArgumentsForDecompression => {
                FinalError::with_title("Could not decompress")
                    .detail("The compress command requires at least one argument")
                    .hint("You must provide:")
                    .hint("  - At least one input argument.")
                    .hint("")
                    .hint("Example: `ouch decompress imgs.tar.gz`")
            }
            Error::InternalError => {
                FinalError::with_title("InternalError :(")
                    .detail("This should not have happened")
                    .detail("It's probably our fault")
                    .detail("Please help us improve by reporting the issue at:")
                    .detail(format!("    {}https://github.com/vrmiguel/ouch/issues ", *CYAN))
            }
            Error::IoError { reason } => FinalError::with_title(reason),
            Error::CompressionTypo => {
                FinalError::with_title("Possible typo detected")
                    .hint(format!("Did you mean '{}ouch compress{}'?", *MAGENTA, *RESET))
            }
            Error::UnknownExtensionError(_) => todo!(),
            Error::AlreadyExists => todo!(),
            Error::InvalidZipArchive(_) => todo!(),
            Error::PermissionDenied { error_title } => FinalError::with_title(error_title).detail("Permission denied"),
            Error::UnsupportedZipArchive(_) => todo!(),
            Error::Custom { reason } => reason.clone(),
        };

        write!(f, "{}", err)
    }
}

impl Error {
    pub fn with_reason(reason: FinalError) -> Self {
        Self::Custom { reason }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => todo!(),
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied { error_title: err.to_string() },
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            _other => Self::IoError { reason: err.to_string() },
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
        Self::WalkdirError { reason: err.to_string() }
    }
}

impl From<FinalError> for Error {
    fn from(err: FinalError) -> Self {
        Self::Custom { reason: err }
    }
}
