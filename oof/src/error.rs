use std::{error, ffi::OsString, fmt};

use crate::Flag;

#[derive(Debug)]
pub enum OofError<'t> {
    FlagValueConflict {
        flag: Flag,
        previous_value: OsString,
        new_value: OsString,
    },
    /// User supplied a flag containing invalid Unicode
    InvalidUnicode(OsString),
    /// User supplied an unrecognized short flag
    UnknownShortFlag(char),
    MisplacedShortArgFlagError(char),
    MissingValueToFlag(&'t Flag)
}

impl<'t> error::Error for OofError<'t> {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl<'t> fmt::Display for OofError<'t> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO: implement proper debug messages
        match self {
            OofError::FlagValueConflict {
                flag,
                previous_value,
                new_value,
            } => write!(
                f,
                "CLI flag value conflicted for flag '--{}', previous: {:?}, new: {:?}.",
                flag.long, previous_value, new_value
            ),
            OofError::InvalidUnicode(flag) => write!(f, "{:?} is not valid Unicode.", flag),
            OofError::UnknownShortFlag(ch) => write!(f, "Unknown argument '-{}'", ch),
            OofError::MisplacedShortArgFlagError(ch) => write!(f, "Invalid placement of `-{}`.\nOnly the last letter in a sequence of short flags can take values.", ch),
            OofError::MissingValueToFlag(flag) => write!(f, "Flag {} takes value but none was supplied.", flag)
        }
    }
}
