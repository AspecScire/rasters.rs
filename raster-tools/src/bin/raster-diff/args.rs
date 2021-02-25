use clap::*;
use raster_tools::{ utils::*, * };

use rasters::histogram::Config as HistConfig;
use std::path::PathBuf;
/// Program arguments
pub struct Args {
    /// First input
    pub input_a: PathBuf,
    /// Second input
    pub input_b: PathBuf,
    /// Operand order
    pub negate: bool,
    /// Histogram config
    pub hist: Option<(HistConfig, PathBuf)>,
    /// Polygon to restrict compute to
    pub polygon: Option<geo::MultiPolygon<f64>>,
    /// Output filename
    pub output: Option<OutputArgs>,
    /// Output type
    pub output_type: OutputType,
    /// Chunk size to read input raster
    pub chunk_size: usize,
}

pub enum OutputType {
    Value,
    Discretized,
}

pub fn parse_cmd_line() -> Args {
    use clap::ErrorKind::*;
    use clap::*;
    let matches = args_parser!("raster-diff")
        .about("Compute raster difference stats.")
        .arg(
            arg!("input_a")
                .required(true)
                .help("First input path (raster dataset)"),
        )
        .arg(
            arg!("input_b")
                .required(true)
                .help("Second input path (raster dataset)"),
        )
        .arg(
            opt!("negate")
                .help("Negate order of operands (default: second - first)")
                .takes_value(false),
        )
        .arg(
            opt!("hist")
                .help("Generate histogram (requires min, max, bins|step)")
                .requires_all(&["min", "max", "binning"]),
        )
        .arg(
            opt!("min")
                .allow_hyphen_values(true)
                .requires("hist")
                .help("Min value to consider"),
        )
        .arg(
            opt!("max")
                .allow_hyphen_values(true)
                .requires("hist")
                .help("Max value to consider"),
        )
        .arg(opt!("bins").help("Number of bins (overrides step size)"))
        .arg(opt!("step").help("Bin size for histogram"))
        .group(
            ArgGroup::with_name("binning")
                .args(&["bins", "step"])
                .requires("hist"),
        )
        .arg(opt!("polygon").help("Region to restrict to (Polygon or MultiPolygon WKT)"))
        .arg(
            opt!("output type")
                .help("Output type: discretized or the default, value")
                .requires("output"),
        )
        .arg(opt!("output").help("Output path (raster dataset)"))
        .arg(
            opt!("driver")
                .requires("output")
                .help("Output driver (default: GTIFF)"),
        )
        .arg(
            opt!("chunk size")
                .short("c")
                .help("Read chunk size (default: 64k pixels)"),
        )
        .get_matches();

    let input_a = value_t!(matches, "input_a", PathBuf).unwrap_or_else(|e| e.exit());
    let input_b = value_t!(matches, "input_b", PathBuf).unwrap_or_else(|e| e.exit());

    let hist_file = value_t!(matches, "hist", PathBuf).ok();
    let hist = if let Some(hist_file) = hist_file {
        let hist = {
            let min = value_t!(matches, "min", f64).unwrap_or_else(|e| e.exit());
            let max = value_t!(matches, "max", f64).unwrap_or_else(|e| e.exit());
            let bins = value_t!(matches, "bins", usize).ok();
            if let Some(bins) = bins {
                HistConfig::from_min_max_bins(min, max, bins)
            } else {
                HistConfig::from_min_max_step(
                    min,
                    max,
                    value_t!(matches, "step", f64).unwrap_or_else(|e| e.exit()),
                )
            }
        };
        Some((hist, hist_file))
    } else {
        None
    };

    let negate = matches.is_present("negate");
    let output = if matches.is_present("output") {
        let o = value_t!(matches, "output", PathBuf).unwrap_or_else(|e| e.exit());
        let driver = value_t!(matches, "driver", String).unwrap_or_else(|_| String::from("GTIFF"));
        Some(OutputArgs { path: o, driver })
    } else {
        None
    };

    let output_type = {
        let output_type =
            value_t!(matches, "output type", String).unwrap_or_else(|_| String::from("value"));
        if output_type == "value" {
            OutputType::Value
        } else if output_type == "discretized" {
            OutputType::Discretized
        } else {
            Error::with_description(
                &format!("invalid output type: {}", output_type),
                InvalidValue,
            )
            .exit()
        }
    };

    if let OutputType::Discretized = output_type {
        if hist.is_none() {
            Error::with_description(
                "`discretized' output requires generating histogram (`--hist')",
                InvalidValue,
            )
            .exit()
        }
    }

    let chunk_size = value_t!(matches, "chunk size", usize).unwrap_or_else(|_| 0x10000);
    let polygon = value_t!(matches, "polygon", String).ok().map(|wkt| {
        let geom = gdal::vector::Geometry::from_wkt(&wkt)
            .unwrap_or_else(|_| Error::with_description("cannot parse WKT", InvalidValue).exit())
            .into();
        use geo::Geometry::{MultiPolygon, Polygon};
        match geom {
            Polygon(p) => p.into(),
            MultiPolygon(p) => p,
            _ => Error::with_description("WKT is not a (multi)-polygon", InvalidValue).exit(),
        }
    });

    Args {
        input_a,
        input_b,
        hist,
        negate,
        polygon,
        chunk_size,
        output,
        output_type,
    }
}
