use std::path::Path;
use geo::{Polygon, Rect};
use cgmath::{Matrix3, Point2};
use ndarray::Array2;

use rasters::*;
use rasters::geometry::*;
use rasters::volume::*;

type Levels = Vec<(usize, usize)>;
pub struct ComputeArgs<'a> {
    base: &'a Path,
    polygon: &'a Polygon<f64>,
    levels_data: &'a Levels,
    transform: &'a CoordTransform,
    chunks_y_offset: usize,
    base_level: usize,
}

pub fn volume<'a>(base: &'a Path, polygon: &'a Polygon<f64>,
                  metadata: &'a VolumePrecomputeMetadata,
                  base_level: usize, top_level: usize
) -> Result<f64> {
    ComputeArgs {
        base, polygon, base_level,
        levels_data: &metadata.levels_data,
        transform: &metadata.transform,
        chunks_y_offset: metadata.chunks_y_offset,
    }.scan_volume(top_level)
}

use rayon::prelude::*;
impl ComputeArgs<'_> {
    pub fn scan_volume(&self, level: usize) -> Result<f64> {
        let (count, xsize) = self.levels_data[level];

        let scale = (1 << level) as f64;
        let transform = scale_transform(self.transform, scale, scale);

        let out = (0..count)
            .into_par_iter()
            .filter_map(|i| {
                use geo::algorithm::intersects::Intersects;
                let offset = self.chunks_y_offset;
                let y = i * offset;
                let rect = rectangle(&transform,
                                     0, y, xsize, y + offset);
                if self.polygon.intersects(&rect) {
                    Some(y)
                } else {
                    None
                }
            })
            .map(|y| self.block_volume(level, y, None))
            .try_reduce(
                || 0.,
                |a, b| Ok(a+b),
            );
        Ok(out?)
    }

    pub fn block_volume(
        &self,
        level: usize, y: usize,
        tpl: Option<&[bool]>
    ) -> Result<f64> {

        // Read data for this block
        let data: Array2<f64> = read_bin(
            &self.base.join(&format!("raster-{}-{}.bin", level, y))
        )?;
        let (rows, cols) = data.dim();
        let data = data.into_raw_vec();

        // let (mut top, mut bot) = (false, false);
        let recurse = level > self.base_level;
        let mut new_tpl = if recurse {
            Some(vec![false; rows*cols])
        } else { None };

        use cgmath::SquareMatrix;

        // Compute cell area
        let scale = (1 << level) as f64;
        let det = self.transform.determinant().abs();
        let cell_area = scale * scale * det;

        let rows_data: Vec<_> = (0..rows)
            .collect();

        let tpl_cols = (cols + 1) / 2;
        let mut vol: f64 = rows_data
            .into_iter()
            .map(|i| {
                let tpl_offset = (i/2) * tpl_cols;
                self.line_volume(y+i, cell_area, cols,
                                 &data[i*cols..],
                                 tpl.map(|tpl| &tpl[tpl_offset..]),
                                 new_tpl.as_mut().map(|tpl| &mut tpl[i*cols..]))
                                 // recurse_tpl)
            })
            .sum();
        if recurse {
            let half_offset_size = self.chunks_y_offset * cols / 2;
            let tpl = new_tpl.unwrap();
            let y = y*2;

            vol += if half_offset_size >= tpl.len() {
                self.block_volume(level-1, y, Some(&tpl))?
            } else {
                let (top, bot) = tpl.split_at(half_offset_size);
                [(y, top), (y + self.chunks_y_offset, bot)]
                    .into_par_iter()
                    .map(|(y, tpl)| self.block_volume(level-1, *y, Some(tpl)))
                    .try_reduce(|| 0., |a, b| Ok(a+b))?
            };

        }
        Ok(vol)
    }

    fn line_volume(
        &self,
        y: usize, cell_area: f64, cols: usize,
        data: &[f64], tpl: Option<&[bool]>, mut recurse_tpl: Option<&mut [bool]>,
    ) -> f64 {
        use geo::algorithm::intersects::Intersects;
        use geo::algorithm::contains::Contains;

        let mut vol = 0.;
        for j in 0..cols {
            let x = j;
            if let Some(tpl) = tpl {
                if !tpl[j/2] {
                    continue;
                }
            }

            // Calc cell rectangle
            let rect: geo::Polygon<_> = rectangle(
                &self.transform,
                x, y, x+1, y+1
            ).into();

            if self.polygon.contains(&rect) {
                if !data[j].is_nan() {
                    vol += data[j] * cell_area;
                }
            } else if self.polygon.intersects(&rect) {
                if let Some(ref mut new_tpl) = recurse_tpl {
                    new_tpl[j] = true;
                } else {
                    if !data[j].is_nan() {
                        use geo_booleanop::boolean::BooleanOp;
                        use geo::area::Area;
                        let iarea = self.polygon.intersection(&rect).area();
                        vol += data[j] * iarea;
                    }
                }
            }
        }
        vol
    }
}

pub fn rectangle(
    transform: &Matrix3<f64>,
    left: usize, top: usize, right: usize, bottom: usize
) -> Rect<f64> {
    use cgmath::Transform;
    let (left, top) = transform.transform_point(
        Point2::new(left as f64, top as f64)).into();
    let (right, bottom) = transform.transform_point(
        Point2::new(right as f64, bottom as f64)).into();
    let hmin = left.min(right);
    let hmax = left.max(right);
    let vmin = top.min(bottom);
    let vmax = top.max(bottom);
    Rect::new((hmin, vmin), (hmax, vmax))
}

#[inline]
pub fn scale_transform(t: &Matrix3<f64>, scale_x: f64, scale_y: f64)
                       -> Matrix3<f64> {
    Matrix3 {
        x: t.x * scale_x,
        y: t.y * scale_y,
        z: t.z,
    }
}
