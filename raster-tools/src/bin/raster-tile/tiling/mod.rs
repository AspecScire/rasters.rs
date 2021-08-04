use anyhow::bail;
use base::RowProc;
use gdal::Dataset;
use nalgebra::{Matrix3, Point2};
use rasters::{geometry, Result};

use self::web_mercator::wm_transform_for_raster;

pub type Dims = geometry::RasterDims;
pub type ICoords = geometry::RasterOffset;
pub type Bounds = geometry::Bounds;

pub struct Config {
    tile_size: usize,
    wm_bounds: Bounds,
    wm_to_pix: Matrix3<f64>,
}
impl Config {
    pub fn for_raster(ds: &Dataset, tile_size: usize) -> Result<Self> {
        fn wm_bounds_for_raster(ds: &Dataset) -> Result<[f64; 4]> {
            let pix_to_wm = wm_transform_for_raster(ds)?;

            let (left, top) = pix_to_wm(0., 0.)?;
            let dim = ds.raster_size();
            let (right, bot) = pix_to_wm(dim.0 as f64, dim.1 as f64)?;

            let rt = pix_to_wm(dim.0 as f64, 0.)?;
            let lb = pix_to_wm(0., dim.1 as f64)?;

            if (rt.0 - right).abs() / right > 1e-5
                || (rt.1 - top).abs() / top > 1e-5
                || (lb.0 - left).abs() / left > 1e-5
                || (lb.1 - bot).abs() / bot > 1e-5 {
                    bail!("transform is not north aligned");
                }

            Ok([left, top, right, bot])
        }

        let [left, top, right, bot] = wm_bounds_for_raster(&ds)?;
        let dim = ds.raster_size();
        let x_res = (right - left) / dim.0 as f64;
        let y_res = (bot - top) / dim.1 as f64;

        let wm_to_pix = Matrix3::new(
            1. / x_res,
            0.,
            -left / x_res,
            0.,
            1. / y_res,
            -top / y_res,
            0.,
            0.,
            1.,
        );
        if (x_res.abs() - y_res.abs()).abs() / x_res.abs().min(y_res.abs()) > 0.25 {
            bail!("pixels are not square in web. merc. coords");
        }

        let wm_bounds = Bounds::new((left, top), (right, bot));
        Ok(Config {
            tile_size,
            wm_bounds,
            wm_to_pix,
        })
    }

    pub fn wm_to_pix(&self, wm_bounds: Bounds) -> Bounds {
        Bounds::new(
            {
                let (l, t) = wm_bounds.min().x_y();
                let pt = self.wm_to_pix.transform_point(&Point2::new(l, t));
                (pt.x, pt.y)
            },
            {
                let (r, b) = wm_bounds.max().x_y();
                let pt = self.wm_to_pix.transform_point(&Point2::new(r, b));
                (pt.x, pt.y)
            },
        )
    }

    pub fn max_zoom(&self) -> usize {
        web_mercator::zoom_for_resolution(1. / self.wm_to_pix[(0, 0)].abs(), self.tile_size).ceil() as usize
    }

    pub fn min_zoom(&self) -> usize {
        web_mercator::largest_zoom_containing(self.wm_bounds)
    }

    pub fn tile_index_bounds(&self, zoom: usize) -> [usize; 4] {
        use web_mercator::tile_index;
        let bounds = self.wm_bounds;
        let (left, top) = tile_index(zoom, bounds.min().x_y());
        let (right, bot) = tile_index(zoom, bounds.max().x_y());
        [left, top, right + 1, bot + 1]
    }

    pub fn base_proc(&self, zoom: usize) -> RowProc {
        let [left, _, right, _] = self.tile_index_bounds(zoom);
        RowProc::new(zoom, self.tile_size, (left, right))
    }
}

pub mod base;
pub mod dem;
pub mod web_mercator;
