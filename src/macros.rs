//! Macros used on ouch.

#[macro_export]
/// Macro that prints message in INFO mode
macro_rules! info {
    ($($arg:tt)*) => {
        $crate::macros::_info_helper();
        println!($($arg)*);
    };
}

/// Prints the `[Info]` tag
pub fn _info_helper() {
    use crate::utils::colors::{RESET, YELLOW};

    print!("{}[INFO]{} ", *YELLOW, *RESET);
}
