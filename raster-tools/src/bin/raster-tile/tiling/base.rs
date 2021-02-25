use super::{Config, *};
use rasters::geometry::BoundsExt;

pub struct RowProc {
    zoom: usize,
    tile_size: usize,
    x_range: (usize, usize),
}

impl RowProc {
    pub fn new(zoom: usize, tile_size: usize, x_range: (usize, usize)) -> Self {
        RowProc {
            zoom,
            tile_size,
            x_range,
        }
    }

    pub fn get_bounds(&self, tile_y: usize) -> Bounds {
        let tt = web_mercator::tile_index_transform(self.zoom)
            .try_inverse()
            .unwrap();

        let tile_wm_coords = |x, y| {
            let pt = tt.transform_point(&Point2::new(x as f64, y as f64));
            (pt.x, pt.y)
        };

        let lt = tile_wm_coords(self.x_range.0, tile_y);
        let rb = tile_wm_coords(self.x_range.1, tile_y + 1);
        Bounds::new(lt, rb)
    }

    pub fn get_pix_bounds(&self, tile_y: usize, cfg: &Config) -> Bounds {
        cfg.wm_to_pix(self.get_bounds(tile_y))
    }

    pub fn chunk_processor(&self, pix_bounds: Bounds, off: ICoords, size: Dims) -> ChunkConfig {
        ChunkConfig {
            raster_pix_bounds: pix_bounds,

            data_offset: (off.0 as f64, off.1 as f64),
            data_size: size,

            tile_size: (self.tile_size, self.tile_size),
            tiles_size: ((self.x_range.1 - self.x_range.0), 1),
        }
    }
}

pub struct ChunkConfig {
    raster_pix_bounds: Bounds,

    data_offset: (f64, f64),
    data_size: Dims,

    tile_size: Dims,
    tiles_size: Dims,
}

impl ChunkConfig {
    pub fn process<F: FnMut(Dims, Dims, Dims, f64)>(&self, f: &mut F) {
        let (left, top) = self.raster_pix_bounds.min().x_y();
        let (right, bot) = self.raster_pix_bounds.max().x_y();

        let tpix_width = (right - left) / self.tiles_size.0 as f64 / self.tile_size.0 as f64;
        let tpix_height = (bot - top) / self.tiles_size.1 as f64 / self.tile_size.1 as f64;

        let tpix_size = (
            self.tiles_size.0 * self.tile_size.0,
            self.tiles_size.1 * self.tile_size.1,
        );

        let data_t = |col: usize, row: usize| {
            // Calculate left-top in tile pix coords
            let x = col as f64 + self.data_offset.0 - left;
            let y = row as f64 + self.data_offset.1 - top;

            let tpix_x = x / tpix_width;
            let tpix_y = y / tpix_height;
            (tpix_x, tpix_y)
        };

        let (cols, rows) = self.data_size;
        for r in 0..rows {
            for c in 0..cols {
                let pix_bounds = {
                    let (l, t) = data_t(c, r);
                    let (r, b) = data_t(c + 1, r + 1);
                    Bounds::new((l, t), (r, b))
                };

                let (off, size) = pix_bounds.window_from_bounds(tpix_size);

                for tr in off.1..(size.1 as isize + off.1) {
                    for tc in off.0..(size.0 as isize + off.0) {
                        {
                            let tc = tc as f64;
                            let tr = tr as f64;
                            Bounds::new((tc, tr), (tc + 1., tr + 1.))
                        }
                        .intersect(&pix_bounds)
                        .map(|tpix_bounds| {
                            let tpix_overlap = tpix_bounds.area();

                            assert!(tpix_overlap <= 1.);
                            assert!(tpix_overlap > 0.);

                            let tc = tc as usize;
                            let tr = tr as usize;
                            let tile = (tc / self.tile_size.0, tr / self.tile_size.1);

                            let tc = tc % self.tile_size.0;
                            let tr = tr % self.tile_size.1;

                            f(tile, (tc, tr), (c, r), tpix_overlap);
                        });
                    }
                }
            }
        }
    }
}
