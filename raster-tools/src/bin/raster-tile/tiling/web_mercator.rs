//! Utilities related to web mercator tiling.

/// EPSG code for web mercator projection CRS.
pub const WEB_MERCATOR_EPSG: u32 = 3857;

use anyhow::{bail, Context};
use gdal::Dataset;
use nalgebra::{Matrix3, Point2};
use rasters::Result;

/// Construct a function to transform coordinates from
/// dataset pixel coordinates to web mercator coordinates.
/// Composes the geo. transform of the raster with a
/// transform from raster CRS to web mercator CRS.
pub fn wm_transform_for_raster(ds: &Dataset) -> Result<impl Fn(f64, f64) -> Result<(f64, f64)>> {
    use gdal::spatial_ref::*;
    let raster = SpatialRef::from_wkt(&ds.projection())
        .with_context(|| "couldn't load dataset transform")?;
    let wm =
        SpatialRef::from_epsg(WEB_MERCATOR_EPSG).with_context(|| "couldn't load wm transform")?;
    let proj_transform = CoordTransform::new(&raster, &wm)?;

    use rasters::geometry::transform_from_dataset;
    let pix_transform = transform_from_dataset(&ds);

    if pix_transform[(0, 1)].abs() > 1e-5 || pix_transform[(1, 0)].abs() > 1e-5 {
        bail!("transform is not north aligned");
    }
    if (pix_transform[(1, 1)].abs() - pix_transform[(0, 0)].abs()).abs() > 1e-2 {
        bail!("pixels are not square");
    }
    Ok(move |x, y| -> Result<(f64, f64)> {
        let world = pix_transform.transform_point(&Point2::new(x, y));
        let mut x = [world.x];
        let mut y = [world.y];
        let mut z = [0.];
        proj_transform.transform_coords(&mut x, &mut y, &mut z)?;

        Ok((x[0], y[0]))
    })
}

const MAX_COORD: f64 = 20037508.;

/// Compute the width (and height) of a tile in web mercator
/// CRS at a given zoom level.
pub fn tile_size(zoom: usize) -> f64 {
    2. * MAX_COORD / (1 << zoom) as f64
}

/// Compute the affine transformation matrix to convert web
/// mercator coordinates into tile index coordinates at a
/// given zoom level. The minimum coordinates is at index
/// coordinates `(0, 0)`, and the maximum coordinates is at
/// `(M, M)` where M is 1 << zoom.
pub fn tile_index_transform(zoom: usize) -> Matrix3<f64> {
    let ts = tile_size(zoom);
    Matrix3::new(
        1. / ts,
        0.,
        MAX_COORD / ts,
        0.,
        1. / ts,
        MAX_COORD / ts,
        0.,
        0.,
        1.,
    )
}

/// Compute the largest zoom with pixel size greater than
/// the reference pixel size.
pub fn largest_zoom_greater_than(ref_size: f64, tile_res: usize) -> usize {
    let base_pixel_size = tile_size(0) / tile_res as f64;
    (base_pixel_size / ref_size).log2().floor() as usize
}

/// Compute the largest zoom containing the complete bounds
/// in a single tile.
pub fn largest_zoom_containing(bounds: super::Bounds) -> usize {
    for zoom in 1.. {
        let (l, t) = tile_index(zoom, bounds.min().x_y());
        let (r, b) = tile_index(zoom, bounds.max().x_y());
        if l != r || t != b {
            return zoom - 1;
        };
    }
    unreachable!()
}

/// Compute the tile index of a given web mercator
/// coordinates. The minimum coordinates is at tile (0, 0)
/// and the index increases (in discrete steps) along with the
/// coordinates.
pub fn tile_index(zoom: usize, pt: (f64, f64)) -> (usize, usize) {
    let pt = tile_index_transform(zoom).transform_point(&Point2::new(pt.0, pt.1));
    (pt.x.floor() as usize, pt.y.floor() as usize)
}
