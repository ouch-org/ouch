//! Macros used on ouch.

use std::io;

/// Macro that prints \[INFO\] messages, wraps [`eprintln`].
///
/// There are essentially two different versions of the `info!()` macro:
/// - `info!(accessible, ...)` should only be used for short, important
///   information which is expected to be useful for e.g. blind users whose
///   text-to-speech systems read out every output line, which is why we
///   should reduce nonessential output to a minimum when running in
///   ACCESSIBLE mode
/// - `info!(inaccessible, ...)` can be used more carelessly / for less
///   important information. A seeing user can easily skim through more lines
///   of output, so e.g. reporting every single processed file can be helpful,
///   while it would generate long and hard to navigate text for blind people
///   who have to have each line of output read to them aloud, without to
///   ability to skip some lines deemed not important like a seeing person would.
#[macro_export]
macro_rules! info {
    // Accessible (short/important) info message.
    // Show info message even in ACCESSIBLE mode
    (accessible, $($arg:tt)*) => {{
        use ::std::io::{stderr, Write};

        use $crate::{macros::stderr_check, utils::colors::{YELLOW, RESET}};

        let mut stderr = stderr().lock();

        if $crate::accessible::is_running_in_accessible_mode() {
            stderr_check(write!(stderr, "{}Info:{} ", *YELLOW, *RESET));
        } else {
            stderr_check(write!(stderr, "{}[INFO]{} ", *YELLOW, *RESET));
        }

        stderr_check(writeln!(stderr, $($arg)*));
    }};
    // Inccessible (long/no important) info message.
    // Print info message if ACCESSIBLE is not turned on
    (inaccessible, $($arg:tt)*) => {{
        use ::std::io::{stderr, Write};

        use $crate::{macros::stderr_check, utils::colors::{YELLOW, RESET}};

        let mut stderr = stderr().lock();

        if !$crate::accessible::is_running_in_accessible_mode() {
            stderr_check(write!(stderr, "{}[INFO]{} ", *YELLOW, *RESET));
            stderr_check(writeln!(stderr, $($arg)*));
        }
    }};
}

/// Macro that prints WARNING messages, wraps [`eprintln`].
#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {{
        use ::std::io::{stderr, Write};

        use $crate::{macros::stderr_check, utils::colors::{ORANGE, RESET}};

        let mut stderr = stderr().lock();

        if $crate::accessible::is_running_in_accessible_mode() {
            stderr_check(write!(stderr, "{}Warning:{} ", *ORANGE, *RESET));
        } else {
            stderr_check(write!(stderr, "{}[WARNING]{} ", *ORANGE, *RESET));
        }

        stderr_check(writeln!(stderr, $($arg)*));
    }};
}

pub fn stderr_check(result: io::Result<()>) {
    result.expect("failed printing to stderr");
}
