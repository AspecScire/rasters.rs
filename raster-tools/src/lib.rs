pub mod utils;
pub use rasters::{Error, Result};

pub mod proc;
pub use proc::*;

pub mod cli;

use ndarray::Array2;
pub type Chunk<T> = (isize, Array2<T>);
