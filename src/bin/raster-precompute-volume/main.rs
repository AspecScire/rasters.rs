use rasters::*;

pub mod pyramid;

// Main function
cli::sync_main!(run());

fn run() -> Result<()> {
    // Parse command line args
    let args = parse_cmd_line();

    // Create output directory
    std::fs::create_dir_all(&args.output)?;

    // Read input raster
    let ds = read_dataset(&args.input)?;
    let band = ds.rasterband(1)?;
    let no_val = band.no_data_value().unwrap_or(std::f64::NAN);
    let (width, height) = band.size();

    // Calc pyramid levels to generate
    let pyramid_levels = {
        let max_levels = pyramid::num_levels(width.max(height));
        (match args.levels {
            Some(l) => if l > 0 {l as usize} else {max_levels - ((-l) as usize)},
            None => max_levels,
        })
    };

    // Calc chunk height
    let chunk_size = {
        let mut block_size = band.block_size().1;
        // Chunk height must be even
        if block_size % 2 != 0 { block_size *= 2; }
        chunks::size_with_padding(block_size, args.chunk_size / width, 0)
    };

    // Calculate chunk dims
    let chunks = chunks::offsets_iterator(0, chunk_size,
                                          0, height as isize);
    let levels_data = pyramid::levels_data(pyramid_levels,
                                           width, chunks.len());

    // Write metadata
    let metadata = volume::VolumePrecomputeMetadata {
        chunks_y_offset: chunk_size,
        levels: pyramid_levels,
        projection: ds.projection(),
        transform: geometry::transform_from_gdal(&ds.geo_transform()?),
        levels_data,
    };

    volume::write_bin(&args.output.join("metadata.bin"), &metadata)?;
    // Calculate pyramid blocks
    use chunks::*;
    use rayon::prelude::*;
    let reader = RasterPathReader(&args.input, 1);
    let processor = pyramid::block_processor(
        &args.output, &reader, &metadata.levels_data,
        no_val, chunk_size);

    eprintln!("Generating pyramid with {} levels", pyramid_levels);
    (0..metadata.levels_data[pyramid_levels-1].0)
        .into_par_iter()
        .try_for_each(|i| processor.process(i))
}


use std::path::PathBuf;
/// Program arguments
pub struct Args {
    /// Input filename
    input: InputArgs,
    /// Output filename
    output: PathBuf,
    /// chunk size
    chunk_size: usize,
    /// Levels of pyramids
    levels: Option<isize>,
}

fn parse_cmd_line() -> Args {
    use clap::value_t;
    use cli::{arg, args_parser, opt};
    let matches = args_parser!("raster-precompute-volume")
        .setting(clap::AppSettings::AllowLeadingHyphen)
        .about("Compute volume pyramids from raster.")
        .arg(
            arg!("input")
                .required(true)
                .help("Input path (raster dataset)"),
        )
        .arg(
            arg!("output")
                .required(true)
                .help("Output path (directory)"),
        )
        .arg(
            opt!("chunk size")
                .short("c")
                .help("Read chunk size (default: 1M pixels)"),
        )
        .arg(
            opt!("levels")
                .short("l")
                .help("Pyramid levels (use negative for stop before hitting 1x1)"),
        )
        .get_matches();

    let input = value_t!(matches, "input", PathBuf).unwrap_or_else(|e| e.exit());
    let output = value_t!(matches, "output", PathBuf).unwrap_or_else(|e| e.exit());
    let chunk_size = value_t!(matches, "chunk size", usize).unwrap_or_else(|_| 0x100000);
    let levels = value_t!(matches, "levels", isize).ok();

    Args {
        input,
        output,
        chunk_size,
        levels,
    }
}
