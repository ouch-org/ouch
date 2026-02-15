use std::{ffi::OsStr, path::PathBuf};

/// Check, without allocating, if os_str can be converted into &str
pub fn is_invalid_utf8(os_str: impl AsRef<OsStr>) -> bool {
    os_str.as_ref().to_str().is_none()
}

/// Filter out list of paths that are not utf8 valid
pub fn get_invalid_utf8_paths(paths: &[PathBuf]) -> Vec<&PathBuf> {
    paths.iter().filter(|path| is_invalid_utf8(path)).collect()
}
