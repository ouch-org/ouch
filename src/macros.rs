//! Macros used on ouch.

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
#[macro_export]
macro_rules! info {
    // Accessible (short/important) info message.
    // Show info message even in ACCESSIBLE mode
    (accessible, $($arg:tt)*) => {
        // if in ACCESSIBLE mode, suppress the "[INFO]" and just print the message
        if (!$crate::cli::ACCESSIBLE.get().unwrap()) {
            $crate::macros::_info_helper();
        }
        println!($($arg)*);
    };
    // Inccessible (long/no important) info message.
    // Print info message if ACCESSIBLE is not turned on
    (inaccessible, $($arg:tt)*) => {
        if (!$crate::cli::ACCESSIBLE.get().unwrap()) {
            $crate::macros::_info_helper();
            println!($($arg)*);
        }
    };
}

/// Helper to display "\[INFO\]", colored yellow
pub fn _info_helper() {
    use crate::utils::colors::{RESET, YELLOW};

    print!("{}[INFO]{} ", *YELLOW, *RESET);
}

/// Macro that prints \[WARNING\] messages, wraps [`println`].
#[macro_export]
macro_rules! warning {
    ($($arg:tt)*) => {
        $crate::macros::_warning_helper();
        println!($($arg)*);
    };
}

/// Helper to display "\[WARNING\]", colored orange
pub fn _warning_helper() {
    use crate::utils::colors::{ORANGE, RESET};

    if !crate::cli::ACCESSIBLE.get().unwrap() {
        print!("{}Warning:{} ", *ORANGE, *RESET);
    } else {
        print!("{}[WARNING]{} ", *ORANGE, *RESET);
    }
}
