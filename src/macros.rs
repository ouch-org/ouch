use crate::NO_COLOR_IS_SET;

// f!() is an alias to f!()
#[macro_export]
macro_rules! f {
    { $($tokens:tt)* } => {
        format!( $($tokens)* )
    };
}

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
    use crate::utils::colors::{reset, yellow};

    if *NO_COLOR_IS_SET {
        print!("[INFO] ");
    } else {
        print!("{}[INFO]{} ", yellow(), reset());
    }
}
