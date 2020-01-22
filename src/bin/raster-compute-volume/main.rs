use rasters::*;

mod compute;

// Main function
cli::sync_main!(run());

fn run() -> Result<()> {
    // Parse command line args
    let args = parse_cmd_line();

    // Read metadata
    use volume::VolumePrecomputeMetadata;
    let metadata = volume::read_bin::<VolumePrecomputeMetadata>(
        &args.input.join("metadata.bin")
    )?;

    let pyramid_levels = if let Some(lev) = args.max_level {
        (lev + 1).min(metadata.levels)
    } else {
        metadata.levels
    };

    let last_level = if let Some(lev) = args.min_level {
        lev
    } else {
        0
    };

    println!("{}",
             compute::volume(
                 &args.input, &args.polygon, &metadata,
                 last_level, pyramid_levels - 1
             )?);
    Ok(())
}

use std::path::PathBuf;
/// Program arguments
pub struct Args {
    /// Input filename
    input: PathBuf,
    /// Polygon to compute
    polygon: geo::Polygon<f64>,
    min_level: Option<usize>,
    max_level: Option<usize>,
}

fn parse_cmd_line() -> Args {
    use clap::value_t;
    use cli::{arg, args_parser, opt};
    let matches = args_parser!("raster-compute-volume")
        .about("Compute volume from pyramids.")
        .arg(
            arg!("input")
                .required(true)
                .help("Input path (hdf5 dataset)"),
        )
        .arg(
            arg!("polygon")
                .required(true)
                .help("Polygon to compute (WKT string)"),
        )
        .arg(
            opt!("min level")
                .help("Pyramid min level to use (default: 0)"),
        )
        .arg(
            opt!("max level")
                .help("Pyramid max level to use (default: all)"),
        )

        .get_matches();

    let input = value_t!(matches, "input", PathBuf).unwrap_or_else(|e| e.exit());
    let max_level = value_t!(matches, "max level", usize).ok();
    let min_level = value_t!(matches, "min level", usize).ok();
    let polygon: geo::Polygon<f64> = {
        let wkt = value_t!(matches, "polygon", String).unwrap_or_else(|e| e.exit());
        use clap::Error;
        use clap::ErrorKind::InvalidValue;
        let geom = gdal::vector::Geometry::from_wkt(&wkt)
            .unwrap_or_else(|_| Error::with_description("cannot parse WKT", InvalidValue).exit())
            .into();
        use geo::Geometry::Polygon;
        if let Polygon(p) = geom { p }
        else { Error::with_description("WKT is not a polygon", InvalidValue).exit() }
    };

    Args {
        input,
        min_level,
        max_level,
        polygon,
    }
}
