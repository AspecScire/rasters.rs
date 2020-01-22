pub fn num_levels(mut num: usize) -> usize {
    let mut count = 1;
    while num > 1 {
        num = (num + 1) / 2;
        count += 1;
    }
    count
}

type Levels = Vec<(usize, usize)>;
pub fn levels_data(num_levels: usize,
                   mut width: usize,
                   mut num_chunks: usize) -> Levels {
    let mut out = Vec::with_capacity(num_levels);
    for _ in 0..num_levels {
        out.push((num_chunks, width));
        width = (width + 1) / 2;
        num_chunks = (num_chunks + 1) / 2;
    }
    out
}

use std::path::Path;
use rasters::chunks::RasterPathReader;
use rasters::Result;
pub fn block_processor<'a>(
    base: &'a Path, input: &'a RasterPathReader,
    levels: &'a Levels, no_val: f64, y_offset: usize
) -> BlockProcess<'a> {
    use rasters::chunks::*;
    let processor = BlockProcess {
        base, input, levels,
        no_val, y_offset,
        raster_height: input.size().1,
    };
    processor
}

// Current rust does not support recursive blocks.
pub struct BlockProcess<'a> {
    base: &'a Path,
    input: &'a RasterPathReader<'a>,
    levels: &'a Levels,
    raster_height: usize,
    y_offset: usize,
    no_val: f64,
}

impl BlockProcess<'_> {
    pub fn process(&self, idx: usize) -> Result<()> {
        self.process_level(self.levels.len()-1, idx)?;
        Ok(())
    }

    fn process_level(&self, level: usize, idx: usize) -> Result<Chunk> {
        use failure::*;
        let chunk = if level == 0 {
            // Base level: read from raster and write
            use rasters::chunks::*;
            let y = (self.y_offset * idx) as isize;
            let (_, width) = self.levels[0];
            let y_size = if y as usize + self.y_offset > self.raster_height {
                self.raster_height - y as usize
            } else {
                self.y_offset
            };

            let mut data = self.input
                .read_as_array((0, y), (width, y_size))
                .with_context(|e| format_err!("chunk @ y={}: {}", y, e))?;
            for item in data.iter_mut() {
                if *item == self.no_val { *item = std::f64::NAN; }
            }

            (y, data)
        } else {
            let r_idx = 2 * idx;
            let r_level = level - 1;
            if self.levels[r_level].0 > r_idx + 1 {
                // Recurse: compute, and join blocks
                // we use par_iter for error prop.
                use rayon::prelude::*;
                (0..2usize)
                    .into_par_iter()
                    .map(
                        |i| Some(self.process_level(r_level, r_idx + i)).transpose())
                    .try_reduce(
                        || None,
                        |a, b| match (a, b) {
                            (None, b) | (b, None) => Ok(b),
                            (Some(a), Some(b)) => Ok(Some(stack_chunks(&a, &b))),
                        }
                    )?.unwrap()
            } else {
                self.process_level(r_level, r_idx)?
            }
        };

        let (y, data) = chunk;
        rasters::volume::write_bin(
            &self.base.join(&format!("raster-{}-{}.bin", level, y)),
            &data
        ).with_context(
            |e| format_err!("writing chunk @ y={}: {}", y, e))?;
        Ok((y/2, scaled_by_2(&data)))
    }
}

type Chunk = rasters::chunks::Chunk<f64>;
fn stack_chunks(first: &Chunk, second: &Chunk) -> Chunk {
    use ndarray::stack;
    use ndarray::Axis;
    (first.0, stack![Axis(0), first.1, second.1])
}

use ndarray::Array2;
pub fn scaled_by_2(data: &Array2<f64>) -> Array2<f64> {
    let (dim1, dim2) = data.dim();

    let value = |i, j| {
        let val = data[(i, j)];
        if val.is_nan() {
            0.
        } else {
            val
        }
    };

    let odim1 = (dim1 + 1) / 2;
    let odim2 = (dim2 + 1) / 2;
    let mut output = Vec::with_capacity(odim1 * odim2);
    for i in (0..dim1).step_by(2) {
        for j in (0..dim2).step_by(2) {
            let mut sum = value(i, j);
            if i < dim1 - 1 {
                sum += value(i+1, j);
                if j < dim2 - 1 {
                    sum += value(i+1, j+1);
                    sum += value(i, j+1);
                }
            } else {
                if j < dim2 - 1 {
                    sum += value(i, j+1);
                }
            }
            sum /= 4.;
            output.push(sum);
        }
    }
    Array2::from_shape_vec((odim1, odim2), output).unwrap()
}
