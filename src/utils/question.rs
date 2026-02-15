//! Utils related to asking [Y/n] questions to the user.
//!
//! Example:
//!   "Do you want to overwrite 'archive.tar.gz'? [Y/n]"

use std::{
    borrow::Cow,
    io::{self, stdin, BufRead},
    path::{Path, PathBuf},
};

use fs_err as fs;

use crate::{
    accessible::is_running_in_accessible_mode,
    error::{Error, FinalError, Result},
    utils::{
        self, colors,
        formatting::path_to_str,
        io::{is_stdin_dev_null, lock_and_flush_output_stdio},
        strip_cur_dir,
    },
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
/// Determines which action to do when there is a file conflict
pub enum FileConflitOperation {
    #[default]
    /// Cancel the operation
    Cancel,
    /// Overwrite the existing file with the new one
    Overwrite,
    /// Rename the file
    /// It'll be put "_1" at the end of the filename or "_2","_3","_4".. if already exists
    Rename,
    /// Merge conflicting folders
    Merge,
}

/// Check if QuestionPolicy flags were set, otherwise, ask user if they want to overwrite.
pub fn user_wants_to_overwrite(
    path: &Path,
    question_policy: QuestionPolicy,
    question_action: QuestionAction,
) -> crate::Result<FileConflitOperation> {
    use FileConflitOperation as Op;

    match question_policy {
        QuestionPolicy::AlwaysYes => Ok(Op::Overwrite),
        QuestionPolicy::AlwaysNo => Ok(Op::Cancel),
        QuestionPolicy::Ask => prompt_user_for_file_conflict_resolution(path, question_action),
    }
}

/// Ask the user if they want to overwrite or rename the &Path
pub fn prompt_user_for_file_conflict_resolution(
    path: &Path,
    question_action: QuestionAction,
) -> Result<FileConflitOperation> {
    use FileConflitOperation as Op;

    let path = path_to_str(strip_cur_dir(path));
    match question_action {
        QuestionAction::Compression => ChoicePrompt::new(
            format!("Do you want to overwrite {path}?"),
            [
                ("yes", Op::Overwrite, *colors::GREEN),
                ("no", Op::Cancel, *colors::RED),
                ("rename", Op::Rename, *colors::BLUE),
            ],
        )
        .ask(),
        QuestionAction::Decompression => ChoicePrompt::new(
            format!("Do you want to overwrite {path}?"),
            [
                ("yes", Op::Overwrite, *colors::GREEN),
                ("no", Op::Cancel, *colors::RED),
                ("rename", Op::Rename, *colors::BLUE),
                ("merge", Op::Merge, *colors::ORANGE),
            ],
        )
        .ask(),
    }
}

/// Create the file if it doesn't exist and if it does then ask to overwrite it.
///
/// If the user doesn't want to overwrite then we return [`Ok(None)`]
///
/// Returns the new file name in case the user asked to rename the file to avoid
/// the conflict.
pub fn create_file_or_prompt_on_conflict(
    path: &Path,
    question_policy: QuestionPolicy,
    question_action: QuestionAction,
) -> Result<Option<(fs::File, PathBuf)>> {
    let path = path.to_owned();

    match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(file) => return Ok(Some((file, path))),
        Err(e) if e.kind() != io::ErrorKind::AlreadyExists => return Err(Error::from(e)),

        Err(_file_already_exists) => {
            // Keep going, will prompt user to solve conflicts
        }
    }

    // Question policy override prompting
    let action = match question_policy {
        QuestionPolicy::AlwaysYes => FileConflitOperation::Overwrite,
        QuestionPolicy::AlwaysNo => FileConflitOperation::Cancel,
        QuestionPolicy::Ask => prompt_user_for_file_conflict_resolution(&path, question_action)?,
    };

    let path_to_create_file = match action {
        FileConflitOperation::Cancel => return Ok(None),
        FileConflitOperation::Merge => path,
        FileConflitOperation::Overwrite => {
            utils::remove_file_or_dir(&path)?;
            path
        }
        FileConflitOperation::Rename => {
            let renamed_file_path = utils::rename_for_available_filename(&path);
            renamed_file_path
        }
    };

    let file = fs::File::create(&path_to_create_file)?;
    Ok(Some((file, path_to_create_file)))
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
            let path = Some(&*path);
            let placeholder = Some("FILE");
            Confirmation::new(&format!("Do you want to {action} 'FILE'?"), placeholder).ask(path)
        }
    }
}

/// Choise dialog for end user with [option1/option2/...] question.
/// Each option is a [Choice] entity, holding a value "T" returned when that option is selected
pub struct ChoicePrompt<'a, T: Default> {
    /// The message to be displayed before the options
    /// e.g.: "Do you want to overwrite 'FILE'?"
    pub prompt: String,

    pub choises: Vec<Choice<'a, T>>,
}

/// A single choice showed as a option to user in a [ChoicePrompt]
/// It holds a label and a color to display to user and a real value to be returned
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

    /// Creates user message and receives a input to be compared with choises "label"
    /// and returning the real value of the choise selected
    pub fn ask(mut self) -> crate::Result<T> {
        let message = self.prompt;

        if is_stdin_dev_null()? {
            eprintln!("{message}");
            eprintln!("Stdin is null, can't read user input (bypass with --yes, but be careful)");
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

                format!("[{choises}]")
            };

            eprintln!("{message} {choice_prompt}");

            let mut answer = String::new();
            let bytes_read = stdin_lock.read_line(&mut answer)?;

            if bytes_read == 0 {
                let error = FinalError::with_title("Unexpected EOF when asking question.")
                    .detail("When asking the user:")
                    .detail(format!("  \"{message}\""))
                    .detail("Expected one of the options as answer, but found EOF instead.")
                    .hint("If using Ouch in scripting, consider using `--yes` and `--no`.");

                return Err(error.into());
            }

            answer.make_ascii_lowercase();
            let answer = answer.trim();

            let chosen_index = self.choises.iter().position(|choise| choise.label.starts_with(answer));

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

        if is_stdin_dev_null()? {
            eprintln!("{message}");
            eprintln!("Stdin is null, can't read user input (bypass with --yes, but be careful)");
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
