//! Random and miscellaneous utils used in ouch.

pub mod colors;
mod formatting;
mod fs;
mod question;

pub use formatting::{concatenate_os_str_list, nice_directory_display, strip_cur_dir, to_utf, Bytes};
pub use fs::{cd_into_same_dir_as, create_dir_if_non_existent, dir_is_empty, try_infer_extension};
pub use question::{
    create_or_ask_overwrite, user_wants_to_continue_decompressing, user_wants_to_overwrite, QuestionPolicy,
};
