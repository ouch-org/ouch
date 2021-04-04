use std::ffi::OsStr;

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
            iter = text.encode_wide
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
