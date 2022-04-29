//! Geometry manipulation utilities

use geo::Rect;
use nalgebra::Matrix3;

/// Matrix representation of the affine geo. transform from
/// pixel coordinates to "world" coordinates of a GDAL
/// dataset. Accomodates a translation, scaling and a
/// rotation. Represented by a 3x3 matrix.
pub type PixelTransform = Matrix3<f64>;

#[cfg(feature = "gdal")]
/// Read the geo. transform from a `Dataset`, and convert it
/// an affine translation `PixelTransform` matrix. Returns
/// the identity transformation if no geo. transform is
/// found.
pub fn transform_from_dataset(ds: &gdal::Dataset) -> PixelTransform {
    ds.geo_transform()
        .map_or_else(|_| Matrix3::identity(), |t| transform_from_gdal(&t))
}

#[cfg(feature = "gdal")]
/// Converts raw GDAL transform information `[f64; 6]` into
/// a `PixelTransform`
fn transform_from_gdal(t: &[f64]) -> PixelTransform {
    Matrix3::new(t[1], t[2], t[0], t[4], t[5], t[3], 0., 0., 1.)
}

/// Represents pixel offset into a raster.
pub type RasterOffset = (isize, isize);

/// Represents dimensions of a raster or a window.
pub type RasterDims = (usize, usize);

/// Represents a block of contiguous data in a raster.
pub type RasterWindow = (RasterOffset, RasterDims);

/// Represents axis-aligned rectangular region. The region
/// contains the left, and top edges, but _does not contain_
/// the right, and bottom edges.
pub type Bounds = Rect<f64>;

/// Utilities to calculate using [`Bounds`].
pub trait BoundsExt {
    /// Compute the area represented by the bounds.
    fn area(&self) -> f64;

    /// Compute the intersection of `self` with another
    /// bounds. Returns `None` if the two regions do not
    /// intersect.
    fn intersect(&self, other: &Self) -> Option<Self>
    where
        Self: Sized;

    /// Compute the largest valid `RasterWindow` within the
    /// region (including partial pixels). Returns a window
    /// with size `(0, 0)` if the region is completely
    /// outside the bounds.
    ///
    /// # Arguments
    ///
    /// - `bounds` - the region
    /// - `dim` - the dimensions of the raster
    fn window_from_bounds(&self, dim: RasterDims) -> RasterWindow;
}

impl BoundsExt for Bounds {
    fn area(&self) -> f64 {
        use geo::prelude::Area;
        Area::unsigned_area(self)
    }

    fn intersect(&self, other: &Self) -> Option<Self>
    where
        Self: Sized,
    {
        let min = (
            self.min().x.max(other.min().x),
            self.min().y.max(other.min().y),
        );
        let max = (
            self.max().x.min(other.max().x),
            self.max().y.min(other.max().y),
        );

        if min.0 < max.0 && min.1 < max.1 {
            Some(Rect::new(min, max))
        } else {
            None
        }
    }

    fn window_from_bounds(&self, dim: RasterDims) -> RasterWindow {
        let raster_bounds = Rect::new((0., 0.), (dim.0 as f64, dim.1 as f64));

        self.intersect(&raster_bounds).map_or_else(
            || ((0, 0), (0, 0)),
            |bounds| {
                let min_x = bounds.min().x.floor() as isize;
                let min_y = bounds.min().y.floor() as isize;

                let max_x = bounds.max().x.ceil() as isize;
                let max_y = bounds.max().y.ceil() as isize;

                let width = (max_x - min_x) as usize;
                let height = (max_y - min_y) as usize;

                ((min_x, min_y), (width, height))
            },
        )
    }
}

#[cfg(feature = "gdal")]
#[cfg(test)]
mod tests {
    use nalgebra::Point2;
    use gdal::Dataset;

    use super::*;
    use std::path::Path;

    #[test]
    #[ignore]
    fn test_with_input() {
        use std::env::var;
        let path = var("RASTER").expect("env: RASTER not found");
        let t = transform_from_dataset(&Dataset::open(Path::new(&path)).unwrap());
        for i in 0..3 {
            eprint!("[");
            for j in 0..3 {
                eprint!("{:15.3}", t[(i, j)]);
            }
            eprintln!("]")
        }

        let pt = t.transform_point(&Point2::new(0.0, 0.0));
        eprintln!("(0, 0) -> ({:15.3},{:15.3})", pt.x, pt.y);
    }
}
