use crate::Error;

pub fn no_support() -> Error {
    Error::UnsupportedFormat {
        reason: "BZip3 support is disabled for this build, possibly due to missing bindgen-cli dependency.".into(),
    }
}
