//! Macros used on ouch.

/// Macro that prints [INFO] messages, wraps [`println`].
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::macros::_info_helper();
        println!($($arg)*);
    };
}

/// Helper to display "[INFO]", colored yellow
pub fn _info_helper() {
    use crate::utils::colors::{RESET, YELLOW};

    print!("{}[INFO]{} ", *YELLOW, *RESET);
}
