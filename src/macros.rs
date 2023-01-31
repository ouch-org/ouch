//! Macros used on ouch.

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
        use $crate::utils::colors::{YELLOW, RESET};

        if $crate::accessible::is_running_in_accessible_mode() {
            eprint!("{}Info:{} ", *YELLOW, *RESET);
        } else {
            eprint!("{}[INFO]{} ", *YELLOW, *RESET);
        }

        eprintln!($($arg)*);
    }};
    // Inccessible (long/no important) info message.
    // Print info message if ACCESSIBLE is not turned on
    (inaccessible, $($arg:tt)*) => {{
        use $crate::utils::colors::{YELLOW, RESET};

        if !$crate::accessible::is_running_in_accessible_mode() {
            eprint!("{}[INFO]{} ", *YELLOW, *RESET);
            eprintln!($($arg)*);
        }
    }};
}

/// Macro that prints WARNING messages, wraps [`eprintln`].
#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {{
        use $crate::utils::colors::{ORANGE, RESET};

        if $crate::accessible::is_running_in_accessible_mode() {
            eprint!("{}Warning:{} ", *ORANGE, *RESET);
        } else {
            eprint!("{}[WARNING]{} ", *ORANGE, *RESET);
        }

        eprintln!($($arg)*);
    }};
}
