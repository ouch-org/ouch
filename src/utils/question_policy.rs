use std::path::Path;

use fs_err as fs;

use super::{strip_cur_dir, to_utf};
use crate::{
    dialogs::Confirmation,
    error::{Error, Result},
};

#[derive(Debug, PartialEq, Clone, Copy)]
/// Determines if overwrite questions should be skipped or asked to the user
pub enum QuestionPolicy {
    /// Ask the user every time
    Ask,
    /// Set by `--yes`, will say 'Y' to all overwrite questions
    AlwaysYes,
    /// Set by `--no`, will say 'N' to all overwrite questions
    AlwaysNo,
}

/// Check if QuestionPolicy flags were set, otherwise, ask user if they want to overwrite.
pub fn user_wants_to_overwrite(path: &Path, question_policy: QuestionPolicy) -> crate::Result<bool> {
    match question_policy {
        QuestionPolicy::AlwaysYes => Ok(true),
        QuestionPolicy::AlwaysNo => Ok(false),
        QuestionPolicy::Ask => {
            let path = to_utf(strip_cur_dir(path));
            let path = Some(path.as_str());
            let placeholder = Some("FILE");
            Confirmation::new("Do you want to overwrite 'FILE'?", placeholder).ask(path)
        }
    }
}

/// Create the file if it doesn't exist and if it does then ask to overwrite it.
/// If the user doesn't want to overwrite then we return [`Ok(None)`]
pub fn create_or_ask_overwrite(path: &Path, question_policy: QuestionPolicy) -> Result<Option<fs::File>> {
    match fs::OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(w) => Ok(Some(w)),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if user_wants_to_overwrite(path, question_policy)? {
                if path.is_dir() {
                    // We can't just use `fs::File::create(&path)` because it would return io::ErrorKind::IsADirectory
                    // ToDo: Maybe we should emphasise that `path` is a directory and everything inside it will be gone?
                    fs::remove_dir_all(path)?;
                }
                Ok(Some(fs::File::create(path)?))
            } else {
                Ok(None)
            }
        }
        Err(e) => Err(Error::from(e)),
    }
}
