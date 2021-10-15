#[macro_export]
macro_rules! info {
    ($writer:expr, $($arg:tt)*) => {
        use crate::macros::_info_helper;
        _info_helper();
        println!($writer, $($arg)*);
    };
    ($writer:expr) => {
        _info_helper();
        println!($writer);
    };
}

pub fn _info_helper() {
    use crate::utils::colors::{RESET, YELLOW};

    print!("{}[INFO]{} ", *YELLOW, *RESET);
}
