//! Random and miscellaneous utils used in ouch.
//!
//! In here we have the logic for custom formatting, some file and directory utils, and user
//! stdin interaction helpers.

pub mod colors;
pub mod io;
pub mod logger;
pub mod threads;

pub use self::{file_visibility::*, formatting::*, fs::*, question::*, utf8::*};
mod file_visibility;
mod formatting;
mod fs;
mod question;
mod utf8;
