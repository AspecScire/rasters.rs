//! Process rasters in memory-efficient chunks.
//!
//! It is often inefficient to load a large rasters
//! completely into memory while processing it. This module
//! provides iterators to load data in smaller chunks.
//!
//! # Raster Memory Layout
//!
//! Large rasters are typically sub-divided internally into
//! rectangular blocks of a specific size. For instance,
//! each band of a GDAL raster may be configured with a
//! _block size_ and the total dimension is split into
//! consecutive blocks of the specified size.
//!
//! The individual blocks support _random access_ while data
//! within a block may require reading the entire block (eg.
//! if the blocks are compressed). While the GDAL API
//! supports reading an arbitary window of data, the
//! underlying driver implements this by reading all the
//! necessary blocks and copying the necessary data into the
//! buffer. Thus, it is more efficient to read along block
//! boundaries.
//!
//! # Memory Efficient Iteration
//!
//! In order to process with a small memory footprint, the
//! algorithm must satisfy a **locality constraint**: to
//! process data at pixel `(x, y)`, it is sufficient to
//! access a small window (say `5x5`) centered around the
//! pixel. In particular, the chunks supported by this
//! module have the following properties:
//!
//! - **Full Width.** Each chunk spans the full width of the
//! raster. This simplifies the iteration logic, and is
//! currently the only supported mode.
//!
//! - **Fixed Padding.** Each chunk may additionally use a
//! fixed number of rows above and below it.

/// Builder to configure chunking. Supports configuring the
/// following paramaters.
///
/// - `width`, `height` - the dimensions of the all the
/// raster bands to be processed (typically from the same
/// [`Dataset`]).
///
/// - `block_size` - the block size of the bands. For
/// multi-band data, this is the least common multiple of
/// the individual block sizes (see [`add_block_size`]).
///
/// - `data_height` - the minimum number of rows required in
/// each chunk of data. Does not include the padding. This
/// value is always maintained as an integer multiple of
/// `block_size` for efficiency.
///
/// - `padding` - the number of additional rows required on
/// either size of the data.
///
/// - `start`,`end` - the semi-open range (i.e. `start..end`
/// in the usual notation) to process (not including
/// padding). The `start` is always at least the `padding`
/// value.
///
/// [`add_block_size`]: ChunkConfig::add_block_size
/// [`Dataset`]: gdal::Dataset
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ChunkConfig {
    width: usize,
    height: usize,

    block_size: usize,
    data_height: usize,
    padding: usize,

    start: usize,
    end: usize,
}

/// The type of item produced by the iterations. Consists
/// of:
///
/// 1. reference to the underlying `ChunkConfig`
/// 1. the start index of this chunk
/// 1. the number of rows (incl. padding) for this chunk
pub type ChunkWindow<'a> = (&'a ChunkConfig, usize, usize);

mod builder;
mod iters;

#[cfg(feature = "use-rayon")]
mod par_iters;

#[inline]
fn mod_ceil(num: usize, m: usize) -> usize {
    let rem = num % m;
    if rem == 0 {
        num
    } else {
        num + (m - rem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn debug_cfg(cfg: ChunkConfig) {
        eprintln!("{:?}", cfg);
        for (_, ls, size) in &cfg {
            eprintln!("{} -> {}", ls, ls + size);
        }
    }

    fn check_cfg(cfg: ChunkConfig, output: Vec<(usize, usize)>) {
        assert_eq!(
            cfg.into_iter().map(|(_, a, b)| (a, b)).collect::<Vec<_>>(),
            output
        );
    }

    #[test]
    #[ignore]
    fn test_with_input() {
        use std::env::var;
        let cfg = var("CHUNK_CONFIG").expect("env: CHUNK_CONFIG not found");
        let nums: Vec<usize> = cfg
            .trim()
            .split(' ')
            .map(str::parse)
            .collect::<std::result::Result<Vec<_>, _>>()
            .expect("couldn't parse CHUNK_CONFIG as [usize; 6]");

        debug_cfg(
            ChunkConfig::with_dims(1, nums[0])
                .add_block_size(nums[1])
                .with_min_data_height(nums[2])
                .with_padding(nums[3])
                .with_start(nums[4])
                .with_end(nums[5]),
        );
    }

    #[test]
    fn test_simple() {
        check_cfg(
            ChunkConfig::with_dims(32, 20)
                .add_block_size(2)
                .with_padding(7)
                .with_end(10),
            vec![(0, 16), (2, 15)],
        )
    }
}
