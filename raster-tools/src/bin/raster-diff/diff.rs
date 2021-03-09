//! Align and process a pair of rasters.

use geo::MultiPolygon;
use nalgebra::Vector2;
use ndarray::Array2;

use rasters::prelude::*;

pub struct Diff {
    transform: PixelTransform,
    no_val_1: f64,
    no_val_2: f64,
    extent: Option<MultiPolygon<f64>>,
    dim_2: (usize, usize),
}

pub fn processor(
    extent: Option<MultiPolygon<f64>>,
    transform: PixelTransform,
    dim_2: (usize, usize),
    no_val_1: f64,
    no_val_2: f64,
) -> Diff {
    Diff {
        extent,
        transform,
        dim_2,
        no_val_1,
        no_val_2,
    }
}

pub type ReadChunk = (RasterOffset, Array2<f64>);

impl Diff {
    /// Transform `win` from raster 1 and calculate the
    /// corresponding window to read from raster 2.
    pub fn transform_window(&self, win: ChunkWindow<'_>) -> RasterWindow {
        let off = (0, win.1 as isize);
        let size = (win.0.width(), win.2);
        transform_window((off, size), self.transform, self.dim_2)
    }

    /// Read a pair of chunks from the two rasters.
    pub fn read_window<R1: ChunkReader, R2: ChunkReader>(
        &self,
        reader_1: &R1,
        reader_2: &R2,
        win_1: ChunkWindow<'_>,
    ) -> Result<(ReadChunk, ReadChunk)> {
        let data = reader_1.read_chunk::<f64>(win_1)?;

        let win_2 = self.transform_window(win_1);
        let data_2 = reader_2.read_as_array::<f64>(win_2.0, win_2.1)?;

        Ok((((0, win_1.1 as isize), data), (win_2.0, data_2)))
    }

    pub fn process<F: FnMut((usize, usize), f64, f64)>(
        &self,
        f: &mut F,
        arr_1: &Array2<f64>,
        off_1: RasterOffset,
        arr_2: &Array2<f64>,
        off_2: RasterOffset,
    ) {
        // Early exit if either array is empty.
        if arr_1.len() == 0 || arr_2.len() == 0 {
            return;
        }

        let off_1 = Vector2::new(off_1.0 as f64 + 0.5, off_1.1 as f64 + 0.5);
        let off_2 = Vector2::new(off_2.0 as f64, off_2.1 as f64);
        let chunk_t = chunk_transform(&self.transform, off_1, off_2);

        // Input extent is in raster_1 pixel coords
        // We translate it to arr_1 cell-center coords
        // by subtracting off_1 + 0.5
        let extent = self.extent.as_ref().map(|poly| {
            use geo::algorithm::map_coords::MapCoords;
            poly.map_coords(|&(x, y)| (x - off_1.x, y - off_1.y))
        });

        let (rows, cols) = arr_1.dim();
        let idx_t = {
            let (r, c) = arr_2.dim();
            index_transformer(chunk_t, (c, r))
        };

        for i in 0..rows {
            for j in 0..cols {
                // Read raster 1 value
                let val_1 = arr_1[(i, j)];

                // Ignore if no-data or NAN
                if val_1 == self.no_val_1 || val_1.is_nan() {
                    continue;
                }

                // Ignore if point is outside extents
                use geo::algorithm::contains::Contains;
                use geo::Point;
                if let Some(poly) = &extent {
                    if !poly.contains(&Point::new(j as f64, i as f64)) {
                        continue;
                    }
                }

                idx_t((i, j)).map(|(i_2, j_2)| {
                    // Read raster 2 value
                    let val_2 = arr_2[(i_2 as usize, j_2 as usize)];

                    // Ignore if value is no-data or NAN
                    if val_2.is_nan() || val_2 == self.no_val_2 { return; }
                    f((i, j), val_1, val_2);
                });
            }
        }
    }
}
