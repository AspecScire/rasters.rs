//! Align a pair of rasters by their geo. transform.
//!
//! This module provides utilities to process a pair of
//! rasters with geographic alignment:
//!
//! - Given two raster bands `A` and `B` that don't
//! necessarily belong to the same raster, compute the
//! unique pixel `(k, l)` of `B` that contains the center of
//! the pixel `(i, j)` in `A`.
//!
//! - Extend the above functionality efficiently to work
//! with chunks of `A`.

use anyhow::*;
use gdal::Dataset;
use geo::Rect;
use nalgebra::{Point2, Vector2, Vector3};

use crate::prelude::{transform_from_dataset, BoundsExt, PixelTransform, RasterDims, RasterWindow};

/// Transforms a `RasterWindow` from one raster to another,
/// possibly truncating to ensure the output is valid for
/// the target raster. The rasters are expected to be
/// axis-aligned.
///
/// # Arguments
///
/// - win: the `RasterWindow` in the source raster
/// - t: the `PixelTransform` from source to target raster
/// - dim: the dimensions of the target raster
///
/// Returns the `RasterWindow` in the target raster.
pub fn transform_window(win: RasterWindow, t: PixelTransform, dim: RasterDims) -> RasterWindow {
    let offset = win.0;
    let size = win.1;

    let t_lt = t.transform_point(&Point2::new(offset.0 as f64, offset.1 as f64));
    let t_rb = t.transform_point(&Point2::new(
        offset.0 as f64 + size.0 as f64,
        offset.1 as f64 + size.1 as f64,
    ));

    Rect::new((t_lt.x, t_lt.y), (t_rb.x, t_rb.y)).window_from_bounds(dim)
}

/// Compute affine transform to transfer from pixel
/// coordinates of the first dataset to the second dataset.
pub fn transform_between(ds_1: &Dataset, ds_2: &Dataset) -> Result<PixelTransform> {
    let transform_1 = transform_from_dataset(&ds_1);
    let transform_2 = transform_from_dataset(&ds_2);

    transform_2
        .try_inverse()
        .ok_or_else(|| anyhow!("input_b: couldn't invert transform"))
        .map(|inv| inv * transform_1)
}

/// Calculate residue of an transform for a pair of offsets.
/// This is used to succinctly convert from array
/// coordinates of a chunk of one raster, to the array
/// coordinates of the corresponding chunk of another
/// raster.
///
/// # Arguments

/// - `transform` - [`PixelTransform`] between the pixel
/// coordinates of the two rasters. May be computed using [
/// `transform_between` ].
///
/// - `off_1` - starting coordinates of the chunk of the
/// first raster (a.k.a source chunk). Shift by `(0.5, 0.5)`
/// to map the center of the source pixel.
///
/// - `off_2` - starting coordinates of the corresponding
/// chunk of the second raster (a.k.a target chunk). The
/// extents of this is typically calculated using
/// [`transform_window`][crate::prelude::transform_window].
///
/// Returns a `PixelTransform` that transforms an array
/// index of the source chunk into array index of the target
/// chunk. Both indices are floating-point tuples,
/// representing interpolated position in the chunks.
///
/// # Derivation
///
/// Suppose `(x, y)` and `(X, Y)` represent the pixel
/// coordinates of the source and target rasters
/// respectively.   Then:
///
/// `(X, Y) = transform(x, y)`
///
/// `off_2 + (J, I) = transform(off_1 + (j, i))`
///
/// `(J, I) = transform(off_1) - off_2 + transform(j, i)`
pub fn chunk_transform(
    transform: &PixelTransform,
    off_1: Vector2<f64>,
    off_2: Vector2<f64>,
) -> PixelTransform {
    let residue = residue(transform, off_1, off_2);

    let mut transform = transform.clone();
    transform[(0, 2)] += residue.x;
    transform[(1, 2)] += residue.y;
    transform
}

fn residue(transform: &PixelTransform, off_1: Vector2<f64>, off_2: Vector2<f64>) -> Vector2<f64> {
    let off_1 = Vector3::new(off_1.x, off_1.y, 0.);
    let off_2 = Vector3::new(off_2.x, off_2.y, 0.);

    let result = transform * off_1 - off_2;
    Vector2::new(result.x, result.y)
}

/// Converts a [`chunk_transform`] into a function that maps
/// input (integer) indices to indices on the output raster
/// if it falls within the given dimension (`dim`), and
/// otherwise `None`.
pub fn index_transformer(
    chunk_t: PixelTransform,
    dim: RasterDims,
) -> impl Fn(RasterDims) -> Option<RasterDims> {
    let (cols, rows) = dim;

    move |(i, j)| {
        // Transform indices
        let pt = chunk_t.transform_point(&Point2::new(j as f64, i as f64));

        if pt.x < 0. || pt.y < 0. {
            return None;
        }
        let j_2 = pt.x.floor() as usize;
        let i_2 = pt.y.floor() as usize;

        if j_2 >= cols || i_2 >= rows {
            None
        } else {
            Some((i_2, j_2))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn print_mat3x3(t: &PixelTransform) {
        for i in 0..3 {
            eprint!("[");
            for j in 0..3 {
                eprint!("{:15.5},", t[(i, j)]);
            }
            eprintln!("],")
        }
    }

    #[test]
    #[ignore]
    fn test_with_input() {
        use std::env::var;
        let path1 = var("RASTER1").expect("env: RASTER1 not found");
        let path2 = var("RASTER2").expect("env: RASTER2 not found");
        let ds1 = Dataset::open(Path::new(&path1)).unwrap();
        let ds2 = Dataset::open(Path::new(&path2)).unwrap();

        let t1 = transform_from_dataset(&ds1);
        let t2 = transform_from_dataset(&ds2);
        eprintln!("ds1 transform: ");
        print_mat3x3(&t1);
        eprintln!("ds2 transform: ");
        print_mat3x3(&t2);

        let tbet = transform_between(&ds1, &ds2).unwrap();
        eprintln!("transform between: ");
        print_mat3x3(&tbet);

        let tchunk = chunk_transform(&tbet, Vector2::new(0., 0.), Vector2::new(10., 0.));
        eprintln!("transform chunk: ");
        print_mat3x3(&tchunk);
    }
}
