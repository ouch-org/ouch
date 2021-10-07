use std::{
    collections::{HashMap, HashSet},
    ffi::{OsStr, OsString},
};

/// Shallow type, created to indicate a `Flag` that accepts a argument.
///
/// ArgFlag::long(), is actually a Flag::long(), but sets a internal attribute.
///
/// Examples in here pls
#[derive(Debug)]
pub struct ArgFlag;

impl ArgFlag {
    pub fn long(name: &'static str) -> Flag {
        Flag { long: name, short: None, takes_value: true }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Flag {
    // Also the name
    pub long: &'static str,
    pub short: Option<char>,
    pub takes_value: bool,
}

impl std::fmt::Display for Flag {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.short {
            Some(short_flag) => write!(f, "-{}/--{}", short_flag, self.long),
            None => write!(f, "--{}", self.long),
        }
    }
}

impl Flag {
    pub fn long(name: &'static str) -> Self {
        Self { long: name, short: None, takes_value: false }
    }

    pub fn short(mut self, short_flag_char: char) -> Self {
        self.short = Some(short_flag_char);
        self
    }
}

#[derive(Default, PartialEq, Eq, Debug)]
pub struct Flags {
    pub boolean_flags: HashSet<&'static str>,
    pub argument_flags: HashMap<&'static str, OsString>,
}

impl Flags {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_present(&self, flag_name: &str) -> bool {
        self.boolean_flags.contains(flag_name) || self.argument_flags.contains_key(flag_name)
    }

    pub fn arg(&self, flag_name: &str) -> Option<&OsString> {
        self.argument_flags.get(flag_name)
    }

    pub fn take_arg(&mut self, flag_name: &str) -> Option<OsString> {
        self.argument_flags.remove(flag_name)
    }
}

#[derive(Debug)]
pub enum FlagType {
    None,
    Short,
    Long,
}

impl FlagType {
    pub fn from(text: impl AsRef<OsStr>) -> Self {
        let text = text.as_ref();

        let mut iter;

        #[cfg(target_family = "unix")]
        {
            use std::os::unix::ffi::OsStrExt;
            iter = text.as_bytes().iter();
        }
        #[cfg(target_family = "windows")]
        {
            use std::os::windows::ffi::OsStrExt;
            iter = text.encode_wide();
        }

        // 45 is the code for a hyphen
        // Typed as 45_u16 for Windows
        // Typed as 45_u8 for Unix
        if let Some(45) = iter.next() {
            if let Some(45) = iter.next() {
                Self::Long
            } else {
                Self::Short
            }
        } else {
            Self::None
        }
    }
}
