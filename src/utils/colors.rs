//! Colored output in ouch with bright colors.

#![allow(dead_code)]

use std::{
    env,
    io::{self, IsTerminal},
    ops::Not,
};

use once_cell::sync::Lazy;

static DISABLE_COLORED_TEXT: Lazy<bool> = Lazy::new(|| {
    io::stdout().is_terminal().not() || io::stderr().is_terminal().not() || env::var_os("NO_COLOR").is_some()
});

macro_rules! color {
    ($name:ident = $value:literal) => {
        #[cfg(target_family = "unix")]
        /// Inserts color onto text based on configuration
        pub static $name: Lazy<&str> = Lazy::new(|| if *DISABLE_COLORED_TEXT { "" } else { $value });
        #[cfg(not(target_family = "unix"))]
        pub static $name: &&str = &"";
    };
}

color!(RESET = "\u{1b}[39m");
color!(BLACK = "\u{1b}[38;5;8m");
color!(BLUE = "\u{1b}[38;5;12m");
color!(CYAN = "\u{1b}[38;5;14m");
color!(GREEN = "\u{1b}[38;5;10m");
color!(MAGENTA = "\u{1b}[38;5;13m");
color!(RED = "\u{1b}[38;5;9m");
color!(WHITE = "\u{1b}[38;5;15m");
color!(YELLOW = "\u{1b}[38;5;11m");
// Requires true color support
color!(ORANGE = "\u{1b}[38;2;255;165;0m");
color!(STYLE_BOLD = "\u{1b}[1m");
color!(STYLE_RESET = "\u{1b}[0m");
color!(ALL_RESET = "\u{1b}[0;39m");
