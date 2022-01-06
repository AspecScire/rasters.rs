/// # Raster-Mask
/// Utility for creating a no-data mask from a DEM / Mosaic
/// Expected functionality:
/// - [ ] Ability to create a mask of valid pixels and non-valid pixels
use crate::{arg, args_parser, opt};
use gdal::Dataset;
use rayon::prelude::*;
use std::sync::mpsc::Receiver;

use raster_tools::{utils::*, *};
use rasters::prelude::{Error, Result, *};

mod clipping;

// Main function
raster_tools::sync_main!(run());

fn run() -> Result<()> {
    // Parse command line args
    let args = parse_cmd_line();

    // Read input raster
    let ds = read_dataset(&args.input)?;
    let no_val = ds.rasterband(1)?.no_data_value().unwrap_or(0.0);
    let band_count = ds.raster_count();

    // Create output dataset
    let out_ds = create_output_raster::<u8>(&args.output, &ds, 1, None)?;
    out_ds.rasterband(1)?.set_no_data_value(0.0)?;

    // Configure chunking
    let chunks_cfg = ChunkConfig::for_dataset(&ds, Some(1..2))?.with_min_data_size(args.chunk_size);
    let chunks = chunks_cfg.into_par_iter();
    let tracker = Tracker::new("chunks", chunks.len());

    // Create channel for writer to receive chunks
    let (s, r) = std::sync::mpsc::channel();
    let writer = { std::thread::spawn(|| writer(r, out_ds, tracker)) };

    // Use map_init to initialize data per thread
    let total_chunks = chunks
        .into_par_iter()
        .map_init(
            || {
                let mut readers = Vec::with_capacity(band_count as usize);
                for i in 1..(band_count + 1) {
                    let dataset = read_dataset(&args.input).expect("reader initialization failed");
                    readers.push(DatasetReader(dataset, i));
                }

                readers
            },
            |readers, chunk| {
                let mut data_vector = Vec::with_capacity(readers.len());
                for reader in readers {
                    let data = reader.read_chunk(chunk)?;
                    data_vector.push(data)
                }

                Ok::<_, Error>((chunk.1, data_vector))
            },
        )
        .map_with(s, |s, data| {
            let (y, data_vector) = data?;
            let chunk = (y as isize, data_vector);
            let mask: Chunk<u8> = clipping::mask_chunk(&chunk, no_val);
            s.send(mask)?;
            Ok::<_, Error>(1)
        })
        .try_reduce(|| 0, |a, b| Ok(a + b));

    // Join spawned threads
    writer.join().expect("writer thread panicked")?;

    eprintln!("Wrote {} chunks", total_chunks?);
    Ok(())
}

fn writer(receiver: Receiver<Chunk<u8>>, out_ds: Dataset, progress: Tracker) -> Result<()> {
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
    /// Input filename
    pub input: InputArgs,
    /// Output filename
    pub output: OutputArgs,
    /// Chunk size to read input raster
    pub chunk_size: usize,
}

use clap::value_t;
use std::path::PathBuf;
fn parse_cmd_line() -> Args {
    let matches = args_parser!("raster-mask")
        .about("Creates a mask that represents the location where a raster has data, and where it doesn't.")
        .arg(
            arg!("input")
                .required(true)
                .help("Input path (raster dataset)"),
        )
        .arg(
            arg!("output")
                .required(true)
                .help("Output Mask Raster path (raster dataset)"),
        )
        .arg(
            opt!("driver")
                .short("d")
                .help("Output driver (default: GTIFF)"),
        )
        .arg(
            opt!("chunk size")
                .short("c")
                .help("Read chunk size (default: 64k pixels)"),
        )
        .get_matches();

    let input = value_t!(matches, "input", PathBuf).unwrap_or_else(|e| e.exit());
    let output = value_t!(matches, "output", PathBuf).unwrap_or_else(|e| e.exit());
    let driver = value_t!(matches, "driver", String).unwrap_or_else(|_| String::from("GTIFF"));
    let chunk_size = value_t!(matches, "chunk size", usize).unwrap_or_else(|_| 0x10000);

    let output = OutputArgs {
        path: output,
        driver,
    };

    Args {
        input,
        output,
        chunk_size,
    }
}
