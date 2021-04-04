use std::{error, ffi::OsString, fmt};

use crate::Flag;

#[derive(Debug)]
pub enum OofError {
    FlagValueConflict {
        flag: Flag,
        previous_value: OsString,
        new_value: OsString,
    },
}

impl error::Error for OofError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl fmt::Display for OofError {
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
        }
    }
}
