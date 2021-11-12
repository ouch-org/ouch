//! Macros used on ouch.

/// Macro that prints \[INFO\] messages, wraps [`println`].
///
/// Normally, this prints nothing if ACCESSIBLE mode is turned on,
/// except when called as `info!(a11y_show, "..", ..)`
#[macro_export]
macro_rules! info {
    // Show info message even in ACCESSIBLE mode
    (a11y_show, $($arg:tt)*) => {
        // if in ACCESSIBLE mode, suppress the "[INFO]" and just print the message
        if (!$crate::cli::ACCESSIBLE.get().unwrap()) {
            $crate::macros::_info_helper();
        }
        println!($($arg)*);
    };
    // Print info message if ACCESSIBLE is not turned on
    ($($arg:tt)*) => {
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
