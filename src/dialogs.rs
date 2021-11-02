//! Pretty (and colored) dialog for asking [Y/n] for the end user.
//!
//! Example:
//!   "Do you want to overwrite 'archive.tar.gz'? [Y/n]"

use std::{
    borrow::Cow,
    io::{self, Write},
};

use crate::utils::colors;

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
        Self { prompt, placeholder: pattern }
    }

    /// Creates user message and receives a boolean input to be used on the program
    pub fn ask(&self, substitute: Option<&'a str>) -> crate::Result<bool> {
        let message = match (self.placeholder, substitute) {
            (None, _) => Cow::Borrowed(self.prompt),
            (Some(_), None) => unreachable!("dev error, should be reported, we checked this won't happen"),
            (Some(placeholder), Some(subs)) => Cow::Owned(self.prompt.replace(placeholder, subs)),
        };

        // Ask the same question to end while no valid answers are given
        loop {
            print!("{} [{}Y{}/{}n{}] ", message, *colors::GREEN, *colors::RESET, *colors::RED, *colors::RESET);
            io::stdout().flush()?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;

            answer.make_ascii_lowercase();
            match answer.trim() {
                "" | "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => continue, // Try again
            }
        }
    }
}
