//! Random filesystem-related stuff used on ouch.

mod bytes;
mod fs;
mod question_policy;

pub use bytes::Bytes;
pub use fs::{
    cd_into_same_dir_as, colors, concatenate_list_of_os_str, create_dir_if_non_existent, dir_is_empty,
    nice_directory_display, strip_cur_dir, to_utf, walk_dir,
};
pub use question_policy::{create_or_ask_overwrite, user_wants_to_overwrite, QuestionPolicy};
