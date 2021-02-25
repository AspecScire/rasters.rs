use rayon::prelude::*;

use clap::*;

use anyhow::{anyhow, bail};
use raster_tools::{utils::*, *, Result, Tracker};
use rasters::prelude::*;

// Main function
raster_tools::sync_main!(run());

fn run() -> Result<()> {
    // Parse command line
    let args = parse_cmd_line();

    // Read input raster
    let ds = &read_dataset(&args.input)?;
    let transform = transform_from_dataset(&ds);
    let no_val = ds.rasterband(1)?.no_data_value().unwrap_or(f64::NAN);

    use anyhow::*;
    use nalgebra::*;

    // Project polygons on raster pixels
    let polygons: Vec<_> = {
        let inv = transform
            .try_inverse()
            .ok_or_else(|| anyhow!("input: couldn't invert geo transform"))?;
        args.polygons
            .iter()
            .map(|poly| {
                use geo::algorithm::map_coords::MapCoords;
                poly.as_ref().map(|poly| {
                    poly.map_coords(|&(x, y)| {
                        let pt = inv.transform_point(&Point2::new(x, y));
                        (pt.x, pt.y)
                    })
                })
            })
            .collect()
    };

    // Calculate processing chunks
    let chunks_cfg = ChunkConfig::for_dataset(&ds, Some(1..2))?.with_min_data_size(args.chunk_size);
    let chunks = chunks_cfg.into_par_iter();
    let tracker = Tracker::new("chunks", chunks.len());

    let init = || vec![PixelStats::default(); polygons.len()];

    let stats = chunks
        .map_init(
            || DatasetReader(
                read_dataset(&args.input).expect("reader initialization failed"),
                1,
            ),
            |rd, chunk| (rd.read_chunk::<f64>(chunk), chunk.1),
        )
        .try_fold(init, |mut stats, (data, y)| {
            let arr = data?;
            let (rows, cols) = arr.dim();
            for i in 0..rows {
                for j in 0..cols {
                    let val = arr[(i, j)];
                    if val == no_val || val.is_nan() {
                        continue;
                    }

                    use geo::algorithm::contains::Contains;
                    use geo::Point;
                    let pt = Point::new(j as f64 + 0.5, y as f64 + i as f64 + 0.5);
                    for (k, poly) in polygons.iter().enumerate() {
                        if let Some(poly) = &poly {
                            if !poly.contains(&pt) {
                                continue;
                            }
                        }
                        stats[k] += val;
                    }
                }
            }
            tracker.increment();
            Ok::<_, Error>(stats)
        })
        .try_reduce(init, |mut acc_1, acc_2| {
            for (i, acc) in acc_1.iter_mut().enumerate() {
                *acc += &acc_2[i];
            }
            Ok(acc_1)
        })?;

    print_json(&stats)?;
    Ok(())
}

use std::path::{Path, PathBuf};
/// Program arguments
pub struct Args {
    /// First input
    input: PathBuf,
    /// Polygon to restrict compute to
    polygons: Vec<Option<geo::MultiPolygon<f64>>>,
    /// Chunk size to read input raster
    chunk_size: usize,
}

fn read_polygons(path: &Path) -> Result<Vec<Option<geo::MultiPolygon<f64>>>> {
    let mut ds = read_dataset(path)?;
    let layer = ds.layer(0)?;
    layer.features()
        .map(|feature| -> Result<_> {
            Some(multipoly_from_wkt(&feature.geometry().wkt()?))
                .transpose()
        })
        .collect()
}

fn multipoly_from_wkt(wkt: &str) -> Result<geo::MultiPolygon<f64>> {
    let geom = gdal::vector::Geometry::from_wkt(wkt)?.into();
    use geo::Geometry::{MultiPolygon, Polygon};
    Ok(match geom {
        Polygon(p) => p.into(),
        MultiPolygon(p) => p,
        _ => bail!("polygon WKT is not a (multi)-polygon"),
    })
}

fn parse_cmd_line() -> Args {
    use clap::Error;
    use clap::ErrorKind::InvalidValue;
    let matches = args_parser!("raster-stats")
        .about("Compute raster stats.")
        .arg(
            arg!("input")
                .required(true)
                .help("Input path (raster dataset)"),
        )
        .arg(
            opt!("polygon")
                .conflicts_with("polygons file")
                .help("Region to restrict to (Polygon or MultiPolygon WKT)"),
        )
        .arg(opt!("polygons file").help("Path to polygons (vector dataset)"))
        .arg(
            opt!("chunk size")
                .short("c")
                .help("Read chunk size (default: 64k pixels)"),
        )
        .get_matches();

    let input = value_t!(matches, "input", PathBuf).unwrap_or_else(|e| e.exit());
    let chunk_size = value_t!(matches, "chunk size", usize).unwrap_or_else(|_| 0x10000);

    let polygons = if let Some(wkt) = value_t!(matches, "polygon", String).ok() {
        vec![Some(multipoly_from_wkt(&wkt).unwrap_or_else(|e| {
            Error::with_description(&format!("cannot parse input WKT: {}", e), InvalidValue).exit()
        }))]
    } else if let Some(path) = value_t!(matches, "polygons file", PathBuf).ok() {
        read_polygons(&path).unwrap_or_else(|e| {
            Error::with_description(
                &format!("reading polygons in {}: {}", path.display(), e),
                InvalidValue,
            )
            .exit()
        })
    } else {
        vec![None]
    };

    Args {
        input,
        chunk_size,
        polygons,
    }
}
