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

/// Generic prompt for user choices
pub struct Prompt<'a, T> {
    message: Cow<'a, str>,
    choices: &'a [PromptChoice<'a, T>],
}

pub struct PromptChoice<'a, T> {
    label: &'a str,
    value: T,
    color: &'a str,
    is_default: bool,
}

impl<'a, T: Default + Copy> Prompt<'a, T> {
    // Create a new prompt with multiple choices
    pub fn new(message: impl Into<Cow<'a, str>>, choices: &'a [PromptChoice<'a, T>]) -> Self {
        Self {
            message: message.into(),
            choices,
        }
    }

    pub fn ask(&self) -> crate::Result<T> {
        // Check if stdin is a terminal
        #[cfg(not(feature = "allow_piped_choice"))]
        if !stdin().is_terminal() {
            eprintln!("{}", self.message);
            eprintln!("Pass --yes to proceed");
            return Ok(T::default());
        }

        let _locks = lock_and_flush_output_stdio()?;
        let mut stdin_lock = stdin().lock();

        // Keep asking until we get a valid response
        loop {
            // Format choices based on accessibility mode
            let choice_prompt = if is_running_in_accessible_mode() {
                // Full word format (yes/no)
                self.choices
                    .iter()
                    .map(|choice| format!("{}{}{}", choice.color, choice.label, *colors::RESET))
                    .collect::<Vec<_>>()
                    .join("/")
            } else {
                // First letter format [Y/n]
                let choices = self
                    .choices
                    .iter()
                    .map(|choice| {
                        let first_char = choice
                            .label
                            .chars()
                            .next()
                            .expect("dev error, choice label should not be empty");

                        // Uppercase for default choice, lowercase for others
                        let char_display = if choice.is_default {
                            first_char.to_uppercase().to_string()
                        } else {
                            first_char.to_lowercase().to_string()
                        };

                        format!("{}{}{}", choice.color, char_display, *colors::RESET)
                    })
                    .collect::<Vec<_>>()
                    .join("/");

                format!("[{}]", choices)
            };

            eprintln!("{} {}", self.message, choice_prompt);

            // Read user input
            let mut answer = String::new();
            let bytes_read = stdin_lock.read_line(&mut answer)?;

            // Handle EOF (e.g., piped input that ended)
            if bytes_read == 0 {
                let error = FinalError::with_title("Unexpected EOF when asking question.")
                    .detail("When asking the user:")
                    .detail(format!("  \"{}\"", self.message))
                    .detail("Expected a valid choice as answer, but found EOF instead.")
                    .hint("If using Ouch in scripting, consider using `--yes` and `--no`.");

                return Err(error.into());
            }

            // Process the answer
            answer.make_ascii_lowercase();
            let answer = answer.trim();

            // Empty response selects the default option
            if answer.is_empty() {
                if let Some(default_choice) = self.choices.iter().find(|choice| choice.is_default) {
                    return Ok(default_choice.value);
                }
            }

            // Check if the answer matches any choice
            for choice in self.choices {
                if choice.label.starts_with(answer) {
                    return Ok(choice.value);
                }
            }

            // No match found, continue the loop to ask again
        }
    }
}

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

#[derive(Default, Clone, Copy)]
/// Determines which action to do when there is a file conflict
pub enum FileConflictOperation {
    #[default]
    /// Cancel the operation
    Cancel,
    /// Overwrite the existing file with the new one
    Overwrite,
    /// Rename the file
    /// It'll be put "_1" at the end of the filename or "_2","_3","_4".. if already exists
    Rename,
}

/// Check if QuestionPolicy flags were set, otherwise, ask user if they want to overwrite.
pub fn user_wants_to_overwrite(path: &Path, question_policy: QuestionPolicy) -> crate::Result<FileConflictOperation> {
    use FileConflictOperation as Op;

    match question_policy {
        QuestionPolicy::AlwaysYes => Ok(Op::Overwrite),
        QuestionPolicy::AlwaysNo => Ok(Op::Cancel),
        QuestionPolicy::Ask => ask_file_conflict_operation(path),
    }
}

/// Ask the user if they want to overwrite or rename the &Path
pub fn ask_file_conflict_operation(path: &Path) -> Result<FileConflictOperation> {
    let path = path_to_str(strip_cur_dir(path));

    Prompt::new(
        format!("Do you want to overwrite {path}?"),
        &[
            PromptChoice {
                label: "yes",
                value: FileConflictOperation::Overwrite,
                color: *colors::GREEN,
                is_default: true,
            },
            PromptChoice {
                label: "no",
                value: FileConflictOperation::Cancel,
                color: *colors::RED,
                is_default: true,
            },
            PromptChoice {
                label: "rename",
                value: FileConflictOperation::Rename,
                color: *colors::BLUE,
                is_default: true,
            },
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
                QuestionPolicy::AlwaysYes => FileConflictOperation::Overwrite,
                QuestionPolicy::AlwaysNo => FileConflictOperation::Cancel,
                QuestionPolicy::Ask => ask_file_conflict_operation(path)?,
            };

            match action {
                FileConflictOperation::Overwrite => {
                    utils::remove_file_or_dir(path)?;
                    Ok(Some(fs::File::create(path)?))
                }
                FileConflictOperation::Cancel => Ok(None),
                FileConflictOperation::Rename => {
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
            let action = match question_action {
                QuestionAction::Compression => "compress",
                QuestionAction::Decompression => "decompress",
            };
            let path = path_to_str(strip_cur_dir(path));

            Prompt::new(
                format!("Do you want to {action} '{path}'?"),
                &[
                    PromptChoice {
                        label: "yes",
                        value: true,
                        color: *colors::GREEN,
                        is_default: true,
                    },
                    PromptChoice {
                        label: "no",
                        value: false,
                        color: *colors::RED,
                        is_default: false,
                    },
                ],
            )
            .ask()
        }
    }
}
