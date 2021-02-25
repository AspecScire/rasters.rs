//! Library to efficiently process GDAL rasters.

/// The error type returned by this crate. Currently this is
/// a synonym for [ `anyhow::Error` ].
pub type Error = anyhow::Error;

/// The `Result` type returned by this crate.
pub type Result<T> = std::result::Result<T, Error>;

pub mod geometry;
pub mod histogram;
pub mod stats;

pub mod chunking;
pub mod reader;

pub mod align;

pub mod prelude;
