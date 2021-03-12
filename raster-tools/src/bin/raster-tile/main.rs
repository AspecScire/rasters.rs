// Main function
raster_tools::sync_main!(run());

use raster_tools::{utils::*, *};
use rasters::prelude::*;

fn run() -> Result<()> {
    // Parse command line
    let args = parse_cmd_line();

    let ds = read_dataset(&args.input)?;
    let cfg = tiling::Config::for_raster(&ds, args.tile_size)?;

    let min_zoom = args.min_zoom.unwrap_or_else(|| cfg.min_zoom());
    eprintln!("min zoom: {}", min_zoom);

    let max_zoom = args.max_zoom.unwrap_or_else(|| {
        cfg.max_zoom()
    });
    eprintln!("max zoom: {}", max_zoom);

    let index = construct_base(max_zoom, min_zoom, &args, &cfg)?;
    write_json(&args.output.join("index.json"), &index)?;

    Ok(())
}

use args::Args;
use tiling::dem::*;
use tiling::Config;
fn construct_base(zoom: usize, min_zoom: usize, args: &Args, cfg: &Config) -> Result<Index> {
    let [left, top, right, bot] = cfg.tile_index_bounds(zoom);
    eprintln!("Constructing base of pyramid @ z={}...", zoom);
    // eprintln!("    lt tile coords: {},{}", left, top);
    // eprintln!("    rb tile coords: {},{}", right, bot);

    let proc = cfg.base_proc(zoom);
    use ndarray::Array2;
    use rayon::prelude::*;
    use tiling::dem::*;

    let write_update_idx = |map: &mut Index, ts: &TileSet| -> Result<()> {
        let idx = ts.write(&args.output)?;
        map.update_index(ts.zoom(), idx);
        Ok(())
    };

    let reducer = |acc: &mut (Vec<TileSet>, _), data| -> Result<_> {
        let sets = &mut acc.0;
        let map = &mut acc.1;

        // let (mut sets, mut map) = acc;

        let mut ts: TileSet = data?;
        write_update_idx(map, &ts)?;

        while let Some(top) = sets.pop() {
            if ts.can_scale_down_with_top() && ts.zoom() == top.zoom() && ts.zoom() > min_zoom {
                ts.scale_down_with_top(Some(top));
                write_update_idx(map, &ts)?;
            } else {
                sets.push(top);
                break;
            }
        }
        sets.push(ts);
        Ok(())
    };

    let ds = read_dataset(&args.input).expect("input dataset");
    let no_val = ds.rasterband(1)?.no_data_value();
    let size = ds.raster_size();

    let chunks = (top..bot).into_par_iter();
    let tracker = Tracker::new("chunks", chunks.len());

    let out = (top..bot)
        .into_par_iter()
        .map_init(
            || {
                let ds = read_dataset(&args.input).expect("input dataset");
                DatasetReader(ds, 1)
            },
            |reader, y| -> Result<_> {
                let pix_bounds = proc.get_pix_bounds(y, &cfg);

                let (off, size) = pix_bounds.window_from_bounds(size);
                let data = reader.read_as_array::<f64>(off, size)?;

                let chunk_proc = proc.chunk_processor(pix_bounds, off, size);

                let mut tiles: Vec<_> = (left..right)
                    .map(|_| Array2::from_elem((args.tile_size, args.tile_size), (0., f64::NAN)))
                    .collect();

                chunk_proc.process(&mut |(tx, _), (tpx, tpy), (px, py), mu| {
                    let pix = &mut tiles[tx][(tpy, tpx)];
                    let val = data[(py, px)];
                    if !val.is_nan() && (no_val.is_none() || val != no_val.unwrap()) {
                        if pix.1.is_nan() {
                            pix.1 = mu;
                        } else {
                            pix.1 += mu;
                        }
                        pix.0 += mu * data[(py, px)];
                    }
                });

                let tileset = TileSet::new(
                    zoom,
                    (left, right),
                    y,
                    tiles
                        .into_iter()
                        .zip(left..right)
                        .map(|(tile, x)| Tile::from_aggregate(tile, (x, y))),
                );

                Ok(tileset)
            },
        )
        .try_fold(Default::default, |mut acc, data| -> Result<_> {
            reducer(&mut acc, data)?;
            tracker.increment();
            Ok(acc)
        })
        .try_reduce(Default::default, |mut acc1, acc2| {
            let (tss2, idxes2) = acc2;
            for ts in tss2 {
                reducer(&mut acc1, Ok(ts))?;
            }

            acc1.1 += idxes2;
            Ok(acc1)
        })?;

    let (tss, mut idx) = out;

    // Final left-to-right scan
    let mut sets: Vec<TileSet> = vec![];
    for mut ts in tss {
        while ts.can_scale_down_with_top() && ts.zoom() > min_zoom {
            ts.scale_down_with_top(sets.pop());
            write_update_idx(&mut idx, &ts)?;
        }
        sets.push(ts);
    }

    // Final right-to-left scan
    while let Some(mut ts) = sets.pop() {
        while ts.zoom() > min_zoom {
            if ts.can_scale_down_with_top() {
                ts.scale_down_with_top(sets.pop());
                write_update_idx(&mut idx, &ts)?;
            } else {
                ts.scale_down_as_top();
                write_update_idx(&mut idx, &ts)?;
            }
        }
    }

    Ok(idx)
}

mod args;
use args::parse_cmd_line;

mod tiling;
