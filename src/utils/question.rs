//! Utils related to asking [Y/n] questions to the user.
//!
//! Example:
//!   "Do you want to overwrite 'archive.tar.gz'? [Y/n]"

use std::{
    borrow::Cow,
    io::{stdin, BufRead, IsTerminal},
    path::Path,
};

use fs_err as fs;

use crate::{
    accessible::is_running_in_accessible_mode,
    error::{Error, FinalError, Result},
    utils::{self, colors, formatting::path_to_str, io::lock_and_flush_output_stdio, strip_cur_dir},
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Determines if overwrite questions should be skipped or asked to the user
pub enum QuestionPolicy {
    /// Ask the user every time
    Ask,
    /// Set by `--yes`, will say 'Y' to all overwrite questions
    AlwaysYes,
    /// Set by `--no`, will say 'N' to all overwrite questions
    AlwaysNo,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Determines which action is being questioned
pub enum QuestionAction {
    /// question called from a compression function
    Compression,
    /// question called from a decompression function
    Decompression,
}

#[derive(Default)]
pub enum FileConflitOperation {
    #[default]
    Cancel,
    Overwrite,
    Rename,
}

/// Check if QuestionPolicy flags were set, otherwise, ask user if they want to overwrite.
pub fn user_wants_to_overwrite(path: &Path, question_policy: QuestionPolicy) -> crate::Result<FileConflitOperation> {
    use FileConflitOperation as Op;

    match question_policy {
        QuestionPolicy::AlwaysYes => Ok(Op::Overwrite),
        QuestionPolicy::AlwaysNo => Ok(Op::Cancel),
        QuestionPolicy::Ask => ask_file_conflict_operation(path),
    }
}

/// Check if QuestionPolicy flags were set, otherwise, ask user if they want to overwrite.
pub fn ask_file_conflict_operation(path: &Path) -> Result<FileConflitOperation> {
    use FileConflitOperation as Op;

    let path = path_to_str(strip_cur_dir(path));

    ChoicePrompt::new(
        format!("Do you want to overwrite {path}?"),
        [
            ("yes", Op::Overwrite, *colors::GREEN),
            ("no", Op::Cancel, *colors::RED),
            ("rename", Op::Rename, *colors::BLUE),
        ],
    )
    .ask()
}

/// Create the file if it doesn't exist and if it does then ask to overwrite it.
/// If the user doesn't want to overwrite then we return [`Ok(None)`]
pub fn ask_to_create_file(path: &Path, question_policy: QuestionPolicy) -> Result<Option<fs::File>> {
    match fs::OpenOptions::new().write(true).create_new(true).open(path) {
        Ok(w) => Ok(Some(w)),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let action = match question_policy {
                QuestionPolicy::AlwaysYes => FileConflitOperation::Overwrite,
                QuestionPolicy::AlwaysNo => FileConflitOperation::Cancel,
                QuestionPolicy::Ask => ask_file_conflict_operation(path)?,
            };

            match action {
                FileConflitOperation::Overwrite => {
                    utils::remove_file_or_dir(path)?;
                    Ok(Some(fs::File::create(path)?))
                }
                FileConflitOperation::Cancel => Ok(None),
                FileConflitOperation::Rename => {
                    let renamed_file_path = utils::rename_for_available_filename(path);
                    Ok(Some(fs::File::create(renamed_file_path)?))
                }
            }
        }
        Err(e) => Err(Error::from(e)),
    }
}

/// Check if QuestionPolicy flags were set, otherwise, ask the user if they want to continue.
pub fn user_wants_to_continue(
    path: &Path,
    question_policy: QuestionPolicy,
    question_action: QuestionAction,
) -> crate::Result<bool> {
    match question_policy {
        QuestionPolicy::AlwaysYes => Ok(true),
        QuestionPolicy::AlwaysNo => Ok(false),
        QuestionPolicy::Ask => {
            let path = path_to_str(strip_cur_dir(path));
            let action = match question_action {
                QuestionAction::Compression => "compress",
                QuestionAction::Decompression => "decompress",
            };

            ChoicePrompt::new(
                format!("Do you want to {action} {path}?"),
                [("yes", true, *colors::GREEN), ("no", false, *colors::RED)],
            )
            .ask()
        }
    }
}

/// Choise dialog for end user with [option1/option2/...] question.
///
/// If the placeholder is found in the prompt text, it will be replaced to form the final message.
pub struct ChoicePrompt<'a, T: Default> {
    /// The message to be displayed with the placeholder text in it.
    /// e.g.: "Do you want to overwrite 'FILE'?"
    pub prompt: String,

    pub choises: Vec<Choice<'a, T>>,
}

pub struct Choice<'a, T: Default> {
    label: &'a str,
    value: T,
    color: &'a str,
}

impl<'a, T: Default> ChoicePrompt<'a, T> {
    /// Creates a new Confirmation.
    pub fn new(prompt: impl Into<String>, choises: impl IntoIterator<Item = (&'a str, T, &'a str)>) -> Self {
        Self {
            prompt: prompt.into(),
            choises: choises
                .into_iter()
                .map(|(label, value, color)| Choice { label, value, color })
                .collect(),
        }
    }

    /// Creates user message and receives a boolean input to be used on the program
    pub fn ask(mut self) -> crate::Result<T> {
        let message = self.prompt;

        if !stdin().is_terminal() {
            eprintln!("{}", message);
            eprintln!("Pass --yes to proceed");
            return Ok(T::default());
        }

        let _locks = lock_and_flush_output_stdio()?;
        let mut stdin_lock = stdin().lock();

        // Ask the same question to end while no valid answers are given
        loop {
            let choice_prompt = if is_running_in_accessible_mode() {
                self.choises
                    .iter()
                    .map(|choise| format!("{}{}{}", choise.color, choise.label, *colors::RESET))
                    .collect::<Vec<_>>()
                    .join("/")
            } else {
                let choises = self
                    .choises
                    .iter()
                    .map(|choise| {
                        format!(
                            "{}{}{}",
                            choise.color,
                            choise
                                .label
                                .chars()
                                .nth(0)
                                .expect("dev error, should be reported, we checked this won't happen"),
                            *colors::RESET
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("/");

                format!("[{}]", choises)
            };

            eprintln!("{} {}", message, choice_prompt);

            let mut answer = String::new();
            let bytes_read = stdin_lock.read_line(&mut answer)?;

            if bytes_read == 0 {
                let error = FinalError::with_title("Unexpected EOF when asking question.")
                    .detail("When asking the user:")
                    .detail(format!("  \"{message}\""))
                    .detail("Expected 'y' or 'n' as answer, but found EOF instead.")
                    .hint("If using Ouch in scripting, consider using `--yes` and `--no`.");

                return Err(error.into());
            }

            answer.make_ascii_lowercase();
            let answer = answer.trim();

            let chosen_index = self
                .choises
                .iter()
                .position(|choise| answer == &choise.label[0..answer.len()]);

            if let Some(i) = chosen_index {
                return Ok(self.choises.remove(i).value);
            }
        }
    }
}

/// Confirmation dialog for end user with [Y/n] question.
///
/// If the placeholder is found in the prompt text, it will be replaced to form the final message.
pub struct Confirmation<'a> {
    /// The message to be displayed with the placeholder text in it.
    /// e.g.: "Do you want to overwrite 'FILE'?"
    pub prompt: &'a str,

    /// The placeholder text that will be replaced in the `ask` function:
    /// e.g.: Some("FILE")
    pub placeholder: Option<&'a str>,
}

impl<'a> Confirmation<'a> {
    /// Creates a new Confirmation.
    pub const fn new(prompt: &'a str, pattern: Option<&'a str>) -> Self {
        Self {
            prompt,
            placeholder: pattern,
        }
    }

    /// Creates user message and receives a boolean input to be used on the program
    pub fn ask(&self, substitute: Option<&'a str>) -> crate::Result<bool> {
        let message = match (self.placeholder, substitute) {
            (None, _) => Cow::Borrowed(self.prompt),
            (Some(_), None) => unreachable!("dev error, should be reported, we checked this won't happen"),
            (Some(placeholder), Some(subs)) => Cow::Owned(self.prompt.replace(placeholder, subs)),
        };

        if !stdin().is_terminal() {
            eprintln!("{}", message);
            eprintln!("Pass --yes to proceed");
            return Ok(false);
        }

        let _locks = lock_and_flush_output_stdio()?;
        let mut stdin_lock = stdin().lock();

        // Ask the same question to end while no valid answers are given
        loop {
            if is_running_in_accessible_mode() {
                eprintln!(
                    "{} {}yes{}/{}no{}: ",
                    message,
                    *colors::GREEN,
                    *colors::RESET,
                    *colors::RED,
                    *colors::RESET
                );
            } else {
                eprintln!(
                    "{} [{}Y{}/{}n{}] ",
                    message,
                    *colors::GREEN,
                    *colors::RESET,
                    *colors::RED,
                    *colors::RESET
                );
            }

            let mut answer = String::new();
            let bytes_read = stdin_lock.read_line(&mut answer)?;

            if bytes_read == 0 {
                let error = FinalError::with_title("Unexpected EOF when asking question.")
                    .detail("When asking the user:")
                    .detail(format!("  \"{message}\""))
                    .detail("Expected 'y' or 'n' as answer, but found EOF instead.")
                    .hint("If using Ouch in scripting, consider using `--yes` and `--no`.");

                return Err(error.into());
            }

            answer.make_ascii_lowercase();
            match answer.trim() {
                "" | "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => continue, // Try again
            }
        }
    }
}
