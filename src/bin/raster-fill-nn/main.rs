use gdal::raster::Dataset;
use cli::{DetailCounter, Progress};
use rasters::*;

pub mod triangulation;
pub mod interpolation;


// Main function
cli::sync_main!(run());

fn run() -> Result<()> {
    // Parse command line args
    let args = parse_cmd_line();

    // Read src pts and triangulate
    let triangles = triangulation::get_triangles(&args)?;

    // Read raster
    let ds = read_dataset(&args.input)?;
    let transform = geometry::transform_from_gdal(&ds.geo_transform()?);
    let band = ds.rasterband(1)?;
    let no_val = {
        use std::f64::NAN;
        band.no_data_value().unwrap_or(NAN)
    };
    let (width, height) = ds.size();

    // Create output dataset
    let out_ds = create_output_raster(
        &args.output, &ds, 1)?;

    // Calculate processing chunks
    let chunks = {
        let chunk_size = chunks::size_with_padding(
            band.block_size().1, args.chunk_size / width, 0
        );
        chunks::offsets_iterator(0, chunk_size,
                                 0, height as isize)
    };

    // Setup progress bar
    let progress = Arc::new(
        Progress::new(DetailCounter::new("chunks")));
    progress.value.total.store(chunks.len());

    // Spawn progress update
    let prog_bar = progress.clone().spawn_auto_update_thread(
        std::time::Duration::from_millis(500));

    // Create channel for writer to receive chunks
    let (s, r) = std::sync::mpsc::channel();
    let writer = std::thread::spawn(
        move || writer(r, out_ds, progress.clone()));

    // Process chunks in parallel
    use rayon::prelude::*;
    let chunks: Vec<_> = chunks
        .map(|(y, size, _)| (y, size, s.clone()))
        .collect();
    std::mem::drop(s);

    use chunks::ChunkReader;
    // For safe reading in different threads
    let reader = chunks::RasterPathReader(args.input.clone(), 1);
    let total_filled: usize = chunks
        .into_par_iter()
        .map(move |(y, size, s)| {
            // Load chunk
            let data = reader.read_as_array((0, y), (width, size))
                .unwrap_or_else(|e|
                    panic!(format!("chunk @ y={}: {}", y, e)));
            (y, data, s)
        })
        .map(move |(y, data, s)| {
            // Process chunk
            let mut chunk = (y, data);
            let count = interpolation::fill_chunk(&mut chunk, no_val,
                                   transform, &triangles,
                                   args.sibson);

            s.send(chunk).expect("channel send failed");
            count
        })
        .sum();

    // Join spawned threads
    writer.join()
        .expect("writer thread panicked");
    prog_bar.join()
        .expect("progress bar panicked");

    eprintln!("Filled {} values", total_filled);
    Ok(())
}

use chunks::Chunk;
use std::sync::{mpsc::Receiver, Arc};
fn writer(receiver: Receiver<Chunk<f64>>,
          out_ds: Dataset,
          progress: Arc<Progress<DetailCounter>>) {
    let out_band = out_ds.rasterband(1)
        .expect("could not open output band");
    for (y, data) in receiver {
        use gdal::raster::Buffer;
        let (ysize, xsize) = data.dim();
        out_band.write((0, y),
                       (xsize, ysize),
                       &Buffer::new((xsize, ysize),
                                    data.into_raw_vec()))
            .expect(&format!("write @y={} failed", y));
        progress.value.processed.fetch_add(1);
    }
    progress.finish();
}

/// Program arguments
use rasters::{InputArgs, OutputArgs};

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
    use cli::{arg, args_parser, opt};
    let matches = args_parser!("pc-interpolate")
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
        .arg(
            opt!("sibson")
                .help("Sibson smoothness parameter (default: 0.5)"),
        )
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
