use clap::value_t;
use crate::{arg, args_parser, opt};
use std::path::PathBuf;

/// Program arguments
pub struct Args {
    /// Raster Input
    pub input: PathBuf,
    /// Minimum zoom
    pub min_zoom: Option<usize>,
    /// Maximum zoom
    pub max_zoom: Option<usize>,
    /// Output directory
    pub output: PathBuf,
    /// Tile size for output,
    pub tile_size: usize,
}

pub fn parse_cmd_line() -> Args {
    use clap::ErrorKind::InvalidValue;
    use clap::*;
    let matches = args_parser!("raster-tile")
        .about("Create EPSG 3857 tiles.")
        .arg(
            arg!("input")
                .required(true)
                .help("Input path (raster dataset)"),
        )
        .arg(
            arg!("output")
                .required(true)
                .help("Output directory (directory)"),
        )
        .arg(opt!("min zoom").help("Min zoom value to consider"))
        .arg(opt!("max zoom").help("Max zoom value to consider"))
        .arg(
            opt!("chunk size")
                .short("c")
                .help("Read chunk size (default: 64k pixels)"),
        )
        .arg(opt!("tile size").help("Read tile size (default: 256 pixels)"))
        .get_matches();

    let input = value_t!(matches, "input", PathBuf).unwrap_or_else(|e| e.exit());
    let output = value_t!(matches, "output", PathBuf).unwrap_or_else(|e| e.exit());

    let max_zoom = value_t!(matches, "max zoom", usize).ok();
    let min_zoom = value_t!(matches, "min zoom", usize).ok();

    let tile_size = value_t!(matches, "tile size", usize).unwrap_or_else(|_| 256);
    if tile_size % 2 != 0 {
        Error::with_description(
            &format!("tile_size must be even: got {}", tile_size),
            InvalidValue,
        )
        .exit();
    }

    Args {
        input,
        min_zoom,
        max_zoom,
        output,
        tile_size,
    }
}
