use anyhow::{anyhow, Context};
use nalgebra::Point2;
use ndarray::*;
use rayon::prelude::*;
use std::sync::mpsc::*;

use args::*;
use raster_tools::{utils::*, *};
use rasters::prelude::*;

mod args;
mod diff;
mod outputs;

// Main function
raster_tools::sync_main!(run());

fn run() -> Result<()> {
    // Parse command line
    let args = args::parse_cmd_line();

    // Read input raster
    let ds = read_dataset(&args.input_a)?;
    let transform_1 = transform_from_dataset(&ds);
    let no_val_1 = ds.rasterband(1)?.no_data_value().unwrap_or(f64::NAN);

    let ds_2 = read_dataset(&args.input_b)?;
    let transform_2 = transform_from_dataset(&ds_2);
    let no_val_2 = ds_2.rasterband(1)?.no_data_value().unwrap_or(f64::NAN);

    // Compute transform: raster 1 -> 2 (in pixels)
    let transform = transform_between(&ds, &ds_2)?;

    // Compute extent on raster 1 pixels
    let extent = {
        let inv = transform_1
            .try_inverse()
            .ok_or_else(|| anyhow!("input_a: couldn't invert transform"))?;
        args.polygon.as_ref().map(|poly| {
            use geo::algorithm::map_coords::MapCoords;
            poly.map_coords(|&(x, y)| {
                let pt = inv.transform_point(&Point2::new(x, y));
                (pt.x, pt.y)
            })
        })
    };

    #[derive(Clone)]
    enum OutputSender {
        ValueSender(Sender<Chunk<f64>>),
        DiscSender(Sender<Chunk<i32>>),
    }
    use OutputSender::*;

    let (sender, writer) = if let Some(out) = &args.output {
        match args.output_type {
            OutputType::Value => {
                let out_ds = create_output_raster::<f64>(&out, &ds, 1, Some(f64::NAN))?;
                let (s, r) = channel();
                let writer = std::thread::spawn(|| writer::<f64>(r, out_ds));
                (Some(ValueSender(s)), Some(writer))
            }
            OutputType::Discretized => {
                let out_ds = create_output_raster::<i32>(&out, &ds, 1, Some(-128.))?;
                let (s, r) = channel();
                let writer = std::thread::spawn(|| writer::<i32>(r, out_ds));
                (Some(DiscSender(s)), Some(writer))
            }
        }
    } else {
        (None, None)
    };

    // Calculate processing chunks
    let chunks_cfg = ChunkConfig::for_dataset(&ds, Some(1..2))?.with_min_data_size(args.chunk_size);

    let diff_proc = diff::processor(extent, transform, ds_2.raster_size(), no_val_1, no_val_2);
    let chunk_proc = chunks_cfg.into_par_iter().map_init(
        || {
            let ds_a = read_dataset(&args.input_a).expect("reader A initialization failed");
            let ds_b = read_dataset(&args.input_b).expect("reader B initialization failed");
            (DatasetReader(ds_a, 1), DatasetReader(ds_b, 1))
        },
        |(rd_1, rd_2), win_1| diff_proc.read_window(&*rd_1, &*rd_2, win_1),
    );
    let tracker = Tracker::new("chunks", chunk_proc.len());

    macro_rules! accumulate {
        ($init:expr, $proc:expr,) => {{
            chunk_proc
                .try_fold_with(($init(), sender), |out, res| {
                    let ((off_1, data_1), (off_2, data_2)) = res?;
                    let (mut out, sender) = out;

                    // If we need to output, allocate array
                    let (mut data, mut data_disc) = if let Some(s) = &sender {
                        match s {
                            ValueSender(_) => {
                                (Some(Array2::from_elem(data_1.dim(), f64::NAN)), None)
                            }
                            DiscSender(_) => (None, Some(Array2::from_elem(data_1.dim(), -128))),
                        }
                    } else {
                        (None, None)
                    };

                    diff_proc.process(
                        &mut |(i, j), val_1, val_2| {
                            let mut diff = val_2 - val_1;
                            if args.negate {
                                diff = -diff;
                            }

                            if let Some(d) = &mut data {
                                d[(i, j)] = diff;
                            } else if let Some(d) = &mut data_disc {
                                if let Some((cfg, _)) = &args.hist {
                                    use HistBin::*;
                                    let bins = cfg.len();
                                    d[(i, j)] = match cfg.bin_for(diff) {
                                        Min => -1,
                                        Bin(i) => i as i32,
                                        Max => bins as i32,
                                    }
                                }
                            }
                            out += $proc(val_1, val_2, diff);
                        },
                        &data_1,
                        off_1,
                        &data_2,
                        off_2,
                    );

                    if let Some(s) = &sender {
                        match s {
                            ValueSender(s) => {
                                s.send(((off_1.1, data.unwrap())))
                                    .with_context(|| anyhow!("send to writer"))?;
                            }
                            DiscSender(s) => {
                                s.send(((off_1.1, data_disc.unwrap())))
                                    .with_context(|| anyhow!("send to writer"))?;
                            }
                        };
                    }
                    tracker.increment();
                    Ok::<_, Error>((out, sender))
                })
                .map(|res| res.map(|(acc, _)| acc))
                .try_reduce($init, |mut acc_1, acc_2| {
                    acc_1 += acc_2;
                    Ok(acc_1)
                })
        }};
    }

    if let Some((cfg, path)) = &args.hist {
        let hist = accumulate!(|| Histogram::new(cfg), |_, _, diff| diff,)?;
        write_bin(&path, &hist)?;
    } else {
        let stats = accumulate!(Default::default, |val_1, val_2, _| (val_1, val_2),)?;
        print_json(&outputs::RasterDiffOutput {
            pix_area_1: transform_1.determinant().abs(),
            pix_area_2: transform_2.determinant().abs(),
            stats,
        })?;
    }

    if let Some(writer) = writer {
        writer.join().expect("writer thread panicked")?;
    }
    Ok(())
}

use gdal::raster::GdalType;
use gdal::Dataset;
fn writer<T: GdalType + Copy>(receiver: Receiver<Chunk<T>>, ds: Dataset) -> Result<()> {
    let band = ds.rasterband(1)?;
    for (y, data) in receiver {
        use gdal::raster::Buffer;
        let (ysize, xsize) = data.dim();
        band.write(
            (0, y),
            (xsize, ysize),
            &Buffer::new((xsize, ysize), data.into_raw_vec()),
        )?;
    }
    Ok(())
}
