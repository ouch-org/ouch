use crate::utils::colors;
use std::{fmt, path::PathBuf};

pub enum Error {
    UnknownExtensionError(String),
    MissingExtensionError(PathBuf),
    // TODO: get rid of this error variant
    InvalidUnicode,
    InvalidInput,
    IoError(std::io::Error),
    FileNotFound(PathBuf),
    AlreadyExists,
    InvalidZipArchive(&'static str),
    PermissionDenied,
    UnsupportedZipArchive(&'static str),
    InternalError,
    OofError,
    CompressingRootFolder,
    MissingArgumentsForCompression,
    UnlistableFormat(String),
    CompressionTypo,
    WalkdirError,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::MissingExtensionError(filename) => {
                write!(f, "{}[ERROR]{} ", colors::red(), colors::reset())?;
                // TODO: show MIME type of the unsupported file
                write!(f, "cannot compress to {:?}, likely because it has an unsupported (or missing) extension.", filename)
            }
            Error::WalkdirError => {
                // Already printed in the From block
                write!(f, "")
            }
            Error::FileNotFound(file) => {
                write!(f, "{}[ERROR]{} ", colors::red(), colors::reset())?;
                if file == &PathBuf::from("") {
                    return write!(f, "file not found!");
                }
                write!(f, "file {:?} not found!", file)
            }
            Error::CompressingRootFolder => {
                write!(f, "{}[ERROR]{} ", colors::red(), colors::reset())?;
                let spacing = "        ";
                writeln!(f, "It seems you're trying to compress the root folder.")?;
                writeln!(
                    f,
                    "{}This is unadvisable since ouch does compressions in-memory.",
                    spacing
                )?;
                write!(
                    f,
                    "{}Use a more appropriate tool for this, such as {}rsync{}.",
                    spacing,
                    colors::green(),
                    colors::reset()
                )
            }
            Error::MissingArgumentsForCompression => {
                write!(f, "{}[ERROR]{} ", colors::red(), colors::reset())?;
                let spacing = "        ";
                writeln!(f,"The compress subcommands demands at least 2 arguments, an input file and an output file.")?;
                writeln!(f, "{}Example: `ouch compress img.jpeg img.zip", spacing)?;
                write!(f, "{}For more information, run `ouch --help`", spacing)
            }
            Error::InternalError => {
                write!(f, "{}[ERROR]{} ", colors::red(), colors::reset())?;
                write!(f, "You've reached an internal error! This really should not have happened.\nPlease file an issue at {}https://github.com/vrmiguel/ouch{}", colors::green(), colors::reset())
            }
            Error::IoError(io_err) => {
                write!(f, "{}[ERROR]{} {}", colors::red(), colors::reset(), io_err)
            }
            Error::CompressionTypo => {
                write!(
                    f,
                    "Did you mean {}ouch compress{}?",
                    colors::magenta(),
                    colors::reset()
                )
            }
            Error::UnlistableFormat(format) => {
                write!(f, "{}[ERROR]{} Cannot list files of archives with format {}.", colors::red(), colors::reset(), format)
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
            _other => Self::IoError(err),
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
        eprintln!("{}[ERROR]{} {}", colors::red(), colors::reset(), err);
        Self::WalkdirError
    }
}

impl<'t> From<oof::OofError<'t>> for Error {
    fn from(err: oof::OofError) -> Self {
        // To avoid entering a lifetime hell, we'll just print the Oof error here
        // and skip saving it into a variant of Self
        println!("{}[ERROR]{} {}", colors::red(), colors::reset(), err);
        Self::OofError
    }
}
