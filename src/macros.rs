//! Macros used on ouch.

use crate::accessible::is_running_in_accessible_mode;

/// Macro that prints \[INFO\] messages, wraps [`println`].
///
/// There are essentially two different versions of the `info!()` macro:
/// - `info!(accessible, ...)` should only be used for short, important
///   information which is expected to be useful for e.g. blind users whose
///   text-to-speach systems read out every output line, which is why we
///   should reduce nonessential output to a minimum when running in
///   ACCESSIBLE mode
/// - `info!(inaccessible, ...)` can be used more carelessly / for less
///   important information. A seeing user can easily skim through more lines
///   of output, so e.g. reporting every single processed file can be helpful,
///   while it would generate long and hard to navigate text for blind people
///   who have to have each line of output read to them aloud, whithout to
///   ability to skip some lines deemed not important like a seeing person would.
///
/// By default `info` outputs to Stdout, if you want to specify the output you can use
/// `@log_out` modifier

#[macro_export]
macro_rules! info {
    // Accessible (short/important) info message.
    // Show info message even in ACCESSIBLE mode
    (accessible, $($arg:tt)*) => {
        info!(@::std::io::stderr(), accessible, $($arg)*);
    };
    (@$log_out: expr, accessible, $($arg:tt)*) => {{
        // if in ACCESSIBLE mode, suppress the "[INFO]" and just print the message
        if !$crate::accessible::is_running_in_accessible_mode() {
            $log_out.output_line_info(format_args!($($arg)*));
        } else {
            $log_out.output_line(format_args!($($arg)*));
        }
    }};
    // Inccessible (long/no important) info message.
    // Print info message if ACCESSIBLE is not turned on
    (inaccessible, $($arg:tt)*) => {
        info!(@::std::io::stderr(), inaccessible, $($arg)*);
    };
    (@$log_out: expr, inaccessible, $($arg:tt)*) => {{
        if !$crate::accessible::is_running_in_accessible_mode() {
            $log_out.output_line_info(format_args!($($arg)*));
        }
    }};
}

/// Macro that prints \[WARNING\] messages, wraps [`eprintln`].
#[macro_export]
macro_rules! warning {
    (@$log_out: expr, $($arg:tt)*) => {
        if !$crate::accessible::is_running_in_accessible_mode() {
            $log_out.output_line_warning(format_args!($($arg)*));
        }
    };
    ($($arg:tt)*) => {
        $crate::macros::_warning_helper();
        eprintln!($($arg)*);
    };
}

/// Helper to display "\[WARNING\]", colored orange
pub fn _warning_helper() {
    use crate::utils::colors::{ORANGE, RESET};

    if is_running_in_accessible_mode() {
        eprint!("{}Warning:{} ", *ORANGE, *RESET);
    } else {
        eprint!("{}[WARNING]{} ", *ORANGE, *RESET);
    }
}
