use crate::{arg, args_parser, opt};
use gdal::Dataset;
use rayon::prelude::*;
use std::sync::mpsc::Receiver;

use raster_tools::{utils::*, *};
use rasters::prelude::*;

mod interpolation;
mod triangulation;

// Main function
raster_tools::sync_main!(run());

fn run() -> Result<()> {
    // Parse command line args
    let args = parse_cmd_line();

    // Read src pts and triangulate
    let triangles = triangulation::get_triangles(&args)?;

    // Read input raster
    let ds = read_dataset(&args.input)?;
    let transform = transform_from_dataset(&ds);
    let band = ds.rasterband(1)?;
    let no_val = band.no_data_value().unwrap_or(f64::NAN);

    // Create output dataset
    let out_ds = create_output_raster::<f64>(&args.output, &ds, 1, Some(f64::NAN))?;

    // Calculate processing chunks
    let chunks_cfg = ChunkConfig::for_dataset(&ds, Some(1..2))?.with_min_data_size(args.chunk_size);
    let chunks = chunks_cfg.into_par_iter();
    let tracker = Tracker::new("chunks", chunks.len());

    // Create channel for writer to receive chunks
    let (s, r) = std::sync::mpsc::channel();
    let writer = { std::thread::spawn(|| writer(r, out_ds, tracker)) };

    // For safe reading in different threads.
    // Use map_init to initialize data per thread
    let total_filled = chunks
        .map_init(
            || {
                let ds = read_dataset(&args.input).expect("reader initialization failed");
                DatasetReader(ds, 1)
            },
            |reader, chunk| {
                let data = reader.read_chunk(chunk)?;
                Ok::<_, Error>((chunk.1, data))
            },
        )
        .map_with(s, |s, data| {
            let (y, data) = data?;
            // Process chunk
            let mut chunk = (y as isize, data);
            let count =
                interpolation::fill_chunk(&mut chunk, no_val, transform, &triangles, args.sibson);

            s.send(chunk)?;
            Ok::<_, Error>(count)
        })
        .try_reduce(|| 0, |a, b| Ok(a + b));

    // Join spawned threads
    writer.join().expect("writer thread panicked")?;

    eprintln!("Filled {} values", total_filled?);
    Ok(())
}

fn writer(receiver: Receiver<Chunk<f64>>, out_ds: Dataset, progress: Tracker) -> Result<()> {
    for (y, data) in receiver {
        use gdal::raster::Buffer;
        let (ysize, xsize) = data.dim();
        out_ds.rasterband(1)?.write(
            (0, y),
            (xsize, ysize),
            &Buffer::new((xsize, ysize), data.into_raw_vec()),
        )?;
        progress.increment();
    }
    Ok(())
}

/// Program arguments
pub struct Args {
    /// Points source filename
    pub source: InputArgs,
    /// Input filename
    pub input: InputArgs,
    /// Output filename
    pub output: OutputArgs,
    /// Property name of height value
    pub prop_name: String,
    /// Chunk size to read input raster
    pub chunk_size: usize,
    /// Sibson smoothness parameter
    pub sibson: f64,
}

use clap::value_t;
use std::path::PathBuf;
fn parse_cmd_line() -> Args {
    let matches = args_parser!("raster-fill-nn")
        .about("Interpolates holes in raster from points (using natural neighbors).")
        .arg(
            arg!("input")
                .required(true)
                .help("Input path (raster dataset)"),
        )
        .arg(
            arg!("output")
                .required(true)
                .help("Output path (raster dataset)"),
        )
        .arg(
            opt!("source")
                .required(true)
                .short("s")
                .help("Source points path (vector dataset)"),
        )
        .arg(
            opt!("driver")
                .short("d")
                .help("Output driver (default: GTIFF)"),
        )
        .arg(
            opt!("property")
                .short("p")
                .required(true)
                .help("Name of property containing z value"),
        )
        .arg(opt!("sibson").help("Sibson smoothness parameter (default: 0.5)"))
        .arg(
            opt!("chunk size")
                .short("c")
                .help("Read chunk size (default: 64k pixels)"),
        )
        .get_matches();

    let input = value_t!(matches, "input", PathBuf).unwrap_or_else(|e| e.exit());
    let source = value_t!(matches, "source", PathBuf).unwrap_or_else(|e| e.exit());
    let output = value_t!(matches, "output", PathBuf).unwrap_or_else(|e| e.exit());
    let driver = value_t!(matches, "driver", String).unwrap_or_else(|_| String::from("GTIFF"));
    let chunk_size = value_t!(matches, "chunk size", usize).unwrap_or_else(|_| 0x10000);
    let sibson = value_t!(matches, "sibson", f64).unwrap_or_else(|_| 0.5);
    let output = OutputArgs {
        path: output,
        driver,
    };
    let prop_name = value_t!(matches, "property", String).unwrap_or_else(|e| e.exit());

    Args {
        input,
        output,
        source,
        prop_name,
        chunk_size,
        sibson,
    }
}
