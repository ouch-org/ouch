use std::io::{self, Write};

use crate::utils::colors;

/// Represents a confirmation dialog
pub struct Confirmation<'a> {
    pub prompt: &'a str,
    pub placeholder: Option<&'a str>,
}


impl<'a> Confirmation<'a> {
    pub const fn new(prompt: &'a str, pattern: Option<&'a str>) -> Self {
        Self { prompt, placeholder: pattern }
    }

    pub fn ask(&self, substitute: Option<&'a str>) -> crate::Result<bool> {
        let message = match (self.placeholder, substitute) {
            (None, _) => self.prompt.into(),
            (Some(_), None) => return Err(crate::Error::InternalError),
            (Some(placeholder), Some(subs)) => self.prompt.replace(placeholder, subs),
        };

        loop {
            print!("{} [{}Y{}/{}n{}] ", message, colors::green(), colors::reset(), colors::red(), colors::reset());
            io::stdout().flush()?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            let trimmed_answer = answer.trim();

            if trimmed_answer.is_empty() {
                return Ok(true);
            }

            match trimmed_answer.to_ascii_lowercase().as_ref() {
                "y" | "yes" => return Ok(true),
                "n" | "no" => return Ok(false),
                _ => {}
            }
        }
    }
}
