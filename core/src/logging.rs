use std::sync::OnceLock;

pub static VERBOSE: OnceLock<bool> = OnceLock::new();

/// `println!` wrapper for verbose mode.
///
/// Note: `VERBOSE` from `secmon::logging` is used and does not need to be imported locally.
#[doc(hidden)]
#[macro_export]
macro_rules! __vprintln {
    ($($arg:tt)*) => {
        if *$crate::logging::VERBOSE.get().unwrap_or(&false) {
            println!($($arg)*);
        }
    };
}
pub use crate::__vprintln as vprintln;
