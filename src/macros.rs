#[macro_export]
macro_rules! info {

    ($writer:expr, $($arg:tt)*) => {
        use crate::utils::colors::{reset, yellow};
        print!("{}[INFO]{} ", yellow(), reset());
        println!($writer, $($arg)*);
    };
    ($writer:expr) => {
        print!("{}[INFO]{} ", yellow(), reset());
        println!($writer);
    };
}
