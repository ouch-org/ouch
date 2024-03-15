//! Accessibility mode functions.
//!
//! # Problem
//!
//! `Ouch`'s default output contains symbols which make it visually easier to
//! read, but harder for people who are visually impaired and rely on
//! text-to-voice readers.
//!
//! On top of that, people who use text-to-voice tools can't easily skim
//! through verbose lines of text, so they strongly benefit from fewer lines
//! of output.
//!
//! # Solution
//!
//! To tackle that, `Ouch` has an accessibility mode that filters out most of
//! the verbose logging, displaying only the most important pieces of
//! information.
//!
//! Accessible mode also changes how logs are displayed, to remove symbols
//! which are "noise" to text-to-voice tools and change formatting of error
//! messages.
//!
//! # Are impaired people actually benefiting from this?
//!
//! So far we don't know. Most CLI tools aren't accessible, so we can't expect
//! many impaired people to be using the terminal and CLI tools, including
//! `Ouch`.
//!
//! I consider this to be an experiment, and a tiny step towards the right
//! direction, `Ouch` shows that this is possible and easy to do, hopefully
//! we can use our experience to later create guides or libraries for other
//! developers.

use once_cell::sync::OnceCell;

/// Global flag for accessible mode.
pub static ACCESSIBLE: OnceCell<bool> = OnceCell::new();

/// Check if `Ouch` is running in accessible mode.
///
/// Check the module-level documentation for more details.
pub fn is_running_in_accessible_mode() -> bool {
    ACCESSIBLE.get().copied().unwrap_or(false)
}

/// Set the value of the global [`ACCESSIBLE`] flag.
///
/// Check the module-level documentation for more details.
pub fn set_accessible(value: bool) {
    if ACCESSIBLE.get().is_none() {
        ACCESSIBLE.set(value).unwrap();
    }
}
