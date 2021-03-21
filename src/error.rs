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
    InputsMustHaveBeenDecompressible(String),
}

// This should be placed somewhere else
pub type OuchResult<T> = Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;

        match self {
            InvalidInput => write!(
                f,
                "When `-o/--output` is omitted, all input files should be compressed files."
            ),
            Error::MissingExtensionError(filename) => {
                write!(f, "cannot compress to \'{}\', likely because it has an unsupported (or missing) extension.", filename)
            }
            Error::InputsMustHaveBeenDecompressible(file) => {
                write!(f, "file '{}' is not decompressible", file.red())
            }
            _ => {
                // TODO
                write!(f, "todo: missing description for error")
            }
        }
    }
}


impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        // Ideally I'd store `err` as a variant of ouch's Error
        // but I need Error to have Eq, which std::io::Error does not
        // implement.
        println!("{}: {:#?}", "error".red(), err);

        Self::IOError
    }
}