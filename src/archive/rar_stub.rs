use crate::Error;

pub fn no_support() -> Error {
    Error::UnsupportedFormat {
        reason: "RAR support is disabled for this build, possibly due to licensing restrictions.".into(),
    }
}
