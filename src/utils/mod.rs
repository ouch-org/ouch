//! Random and miscellaneous utils used in ouch.
//!
//! In here we have the logic for custom formatting, some file and directory utils, and user
//! stdin interaction helpers.

pub mod colors;
mod formatting;
mod fs;
mod question;

pub use formatting::{concatenate_os_str_list, nice_directory_display, strip_cur_dir, to_utf, Bytes};
pub use fs::{cd_into_same_dir_as, create_dir_if_non_existent, dir_is_empty, try_infer_extension};
pub use question::{
    create_or_ask_overwrite, user_wants_to_continue_decompressing, user_wants_to_overwrite, QuestionPolicy,
};
pub use utf8::{get_invalid_utf8_paths, is_invalid_utf8};

mod utf8 {
    use std::path::{Path, PathBuf};

    pub fn is_invalid_utf8(path: &Path) -> bool {
        #[cfg(unix)]
        {
            use std::{os::unix::prelude::OsStrExt, str};

            let bytes = path.as_os_str().as_bytes();
            str::from_utf8(bytes).is_err()
        }
        #[cfg(not(unix))]
        {
            path.to_str().is_none()
        }
    }

    pub fn get_invalid_utf8_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
        paths.iter().filter_map(|path| is_invalid_utf8(&path).then(|| path.clone())).collect()
    }
}
