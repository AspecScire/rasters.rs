pub mod args;
pub mod progress;
pub use progress::Progress;
pub mod counters;
pub use counters::{Counter, DetailCounter};

use std::fmt::Display;
#[inline]
pub fn unwrap_or_exit<T, E: Display>(res: Result<T, E>) -> T {
    match res {
        Err(e) => {
            eprintln!("Error: {:#}", e);
            std::process::exit(1)
        }
        Ok(t) => t,
    }
}

#[macro_export]
macro_rules! async_main {
    ($name:expr) => {
        #[async_std::main]
        async fn main() {
            $crate::cli::unwrap_or_exit({ $name }.await);
        }
    };
}

#[macro_export]
macro_rules! sync_main {
    ($name:expr) => {
        fn main() {
            $crate::cli::unwrap_or_exit({ $name });
        }
    };
}
