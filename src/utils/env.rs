use std::{
    env,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};

use crate::{Error, error::Result};

static CACHED_CURRENT_DIR: Mutex<PathBuf> = Mutex::new(PathBuf::new());

/// Returns current directory, but before change the process' directory to the
/// one that contains the file pointed to by `filename`.
pub fn cd_into_same_dir_as(filename: &Path) -> Result<PathBuf> {
    let previous_location = current_dir().to_path_buf();

    let parent = filename.parent().ok_or(Error::CompressingRootFolder)?;
    set_current_dir(parent)?;

    Ok(previous_location)
}

pub fn set_current_dir(dir: impl Into<PathBuf>) -> Result<()> {
    let dir = dir.into();
    #[allow(clippy::disallowed_methods)]
    env::set_current_dir(&dir)?;
    *CACHED_CURRENT_DIR.lock().unwrap() = dir;
    Ok(())
}

pub fn current_dir() -> MutexGuard<'static, PathBuf> {
    let mut cached = CACHED_CURRENT_DIR.lock().unwrap();
    let is_uninit = cached.as_os_str().is_empty();
    #[allow(clippy::disallowed_methods)]
    if is_uninit {
        *cached = env::current_dir().expect("Failed to read current directory");
    }
    cached
}
