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

use crate::{oof, utils::colors::*};

#[derive(Debug, PartialEq)]
pub enum Error {
    UnknownExtensionError(String),
    MissingExtensionError(PathBuf),
    IoError { reason: String },
    FileNotFound(PathBuf),
    AlreadyExists,
    InvalidZipArchive(&'static str),
    PermissionDenied,
    UnsupportedZipArchive(&'static str),
    InternalError,
    OofError(oof::OofError),
    CompressingRootFolder,
    MissingArgumentsForCompression,
    MissingArgumentsForDecompression,
    CompressionTypo,
    WalkdirError { reason: String },
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
pub struct FinalError {
    title: String,
    details: Vec<String>,
    hints: Vec<String>,
}

impl Display for FinalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Title
        writeln!(f, "{}[ERROR]{} {}", red(), reset(), self.title)?;

        // Details
        for detail in &self.details {
            writeln!(f, " {}-{} {}", white(), yellow(), detail)?;
        }

        // Hints
        if !self.hints.is_empty() {
            // Separate by one blank line.
            writeln!(f)?;
            for hint in &self.hints {
                writeln!(f, "{}hint:{} {}", green(), reset(), hint)?;
            }
        }

        write!(f, "{}", reset())
    }
}

impl FinalError {
    pub fn with_title(title: impl ToString) -> Self {
        Self { title: title.to_string(), details: vec![], hints: vec![] }
    }

    pub fn detail(&mut self, detail: impl ToString) -> &mut Self {
        self.details.push(detail.to_string());
        self
    }

    pub fn hint(&mut self, hint: impl ToString) -> &mut Self {
        self.hints.push(hint.to_string());
        self
    }

    pub fn to_owned(&mut self) -> Self {
        std::mem::take(self)
    }

    pub fn display_and_crash(&self) -> ! {
        eprintln!("{}", self);
        std::process::exit(crate::EXIT_FAILURE)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let err = match self {
            Error::MissingExtensionError(filename) => {
                let error = FinalError::with_title(format!("Cannot compress to {:?}", filename))
                    .detail("Ouch could not detect the compression format")
                    .hint("Use a supported format extension, like '.zip' or '.tar.gz'")
                    .hint("Check https://github.com/vrmiguel/ouch for a full list of supported formats")
                    .to_owned();

                error
            }
            Error::WalkdirError { reason } => FinalError::with_title(reason),
            Error::FileNotFound(file) => {
                let error = if file == Path::new("") {
                    FinalError::with_title("file not found!")
                } else {
                    FinalError::with_title(format!("file {:?} not found!", file))
                };

                error
            }
            Error::CompressingRootFolder => {
                let error = FinalError::with_title("It seems you're trying to compress the root folder.")
                    .detail("This is unadvisable since ouch does compressions in-memory.")
                    .hint("Use a more appropriate tool for this, such as rsync.")
                    .to_owned();

                error
            }
            Error::MissingArgumentsForCompression => {
                let error = FinalError::with_title("Could not compress")
                    .detail("The compress command requires at least 2 arguments")
                    .hint("You must provide:")
                    .hint("  - At least one input argument.")
                    .hint("  - The output argument.")
                    .hint("")
                    .hint("Example: `ouch compress image.png img.zip`")
                    .to_owned();

                error
            }
            Error::MissingArgumentsForDecompression => {
                let error = FinalError::with_title("Could not decompress")
                    .detail("The compress command requires at least one argument")
                    .hint("You must provide:")
                    .hint("  - At least one input argument.")
                    .hint("")
                    .hint("Example: `ouch decompress imgs.tar.gz`")
                    .to_owned();

                error
            }
            Error::InternalError => {
                let error = FinalError::with_title("InternalError :(")
                    .detail("This should not have happened")
                    .detail("It's probably our fault")
                    .detail("Please help us improve by reporting the issue at:")
                    .detail(format!("    {}https://github.com/vrmiguel/ouch/issues ", cyan()))
                    .to_owned();

                error
            }
            Error::OofError(err) => FinalError::with_title(err),
            Error::IoError { reason } => FinalError::with_title(reason),
            Error::CompressionTypo => FinalError::with_title("Possible typo detected")
                .hint(format!("Did you mean '{}ouch compress{}'?", magenta(), reset()))
                .to_owned(),
            _err => {
                todo!();
            }
        };

        write!(f, "{}", err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => panic!("{}", err),
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
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

impl From<oof::OofError> for Error {
    fn from(err: oof::OofError) -> Self {
        Self::OofError(err)
    }
}
