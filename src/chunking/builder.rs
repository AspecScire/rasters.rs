use crate::Result;
use anyhow::Context;
use gdal::Dataset;

use super::{mod_ceil, ChunkConfig};

/// Constructors
impl ChunkConfig {
    /// Construct a `ChunkConfig` with a given raster size.
    pub fn with_dims(width: usize, height: usize) -> Self {
        if width < 1 || height < 1 {
            panic!("dimensions must both be at least 1");
        }
        ChunkConfig {
            width,
            height,

            block_size: 1,
            data_height: 1,
            padding: 0,

            start: 0,
            end: height,
        }
    }

    /// Construct a `ChunkConfig` from a raster [`Dataset`],
    /// reading the size from it. An optional list of bands
    /// may be specified to configure the `block_size`.
    pub fn for_dataset<I: IntoIterator<Item = isize>>(
        ds: &Dataset,
        bands: Option<I>,
    ) -> Result<Self> {
        let size = ds.raster_size();
        let mut cfg = ChunkConfig::with_dims(size.0, size.1);

        if let Some(bands) = bands {
            for band_idx in bands {
                let band = ds
                    .rasterband(band_idx)
                    .with_context(|| format!("unable to open rasterband {}", band_idx))?;
                cfg = cfg.add_block_size(band.block_size().1);
            }
        }

        Ok(cfg)
    }
}

/// Builder methods to configure the parameters
impl ChunkConfig {
    /// Accumulate the given `block_size` to the
    /// configuration by calculating the least common
    /// multiple with the current value.
    pub fn add_block_size(mut self, block_size: usize) -> Self {
        if block_size < 1 {
            panic!("block_size should be at least 1");
        }
        self.block_size = lcm(self.block_size, block_size);
        self.adjust_block_height();
        self
    }
    /// Set the minimum `data_height` for the chunking. The
    /// actual `data_height` is the least multiple of
    /// `block_size` larger or equal to the given value.
    pub fn with_min_data_height(mut self, min_data_height: usize) -> Self {
        self.data_height = min_data_height.max(1);
        self.adjust_block_height();
        self
    }
    /// Set the minimum `data_height` by specifying minimum
    /// number of data pixels expected in each chunk.
    pub fn with_min_data_size(self, min_data_size: usize) -> Self {
        let min_height = (min_data_size + self.width - 1) / self.width;
        self.with_min_data_height(min_height)
    }

    /// Set the padding required for each chunk.
    pub fn with_padding(mut self, padding: usize) -> Self {
        self.padding = padding;
        self.adjust_start();
        self
    }

    /// Set the start index of the iteration range.
    pub fn with_start(mut self, start: usize) -> Self {
        self.start = start;
        self.adjust_start();
        self
    }

    /// Set the end (not included) index of the iteration
    /// range.
    pub fn with_end(mut self, end: usize) -> Self {
        self.end = end.min(self.height);
        self
    }

    /// Ensure that block height is non-zero, and a multiple
    /// of block size.
    #[inline]
    fn adjust_block_height(&mut self) {
        self.data_height = mod_ceil(self.data_height, self.block_size);
    }

    /// Ensure start is always greater than padding
    #[inline]
    fn adjust_start(&mut self) {
        self.start = self.start.max(self.padding);
    }
}

/// Getter methods to read the parameters of the config
impl ChunkConfig {
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn block_size(&self) -> usize {
        self.block_size
    }
    pub fn data_height(&self) -> usize {
        self.data_height
    }
    pub fn padding(&self) -> usize {
        self.padding
    }

    pub fn start(&self) -> usize {
        self.start
    }
    pub fn end(&self) -> usize {
        self.end
    }
}

#[inline]
fn lcm(a: usize, b: usize) -> usize {
    a / gcd(a, b) * b
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 {
        return a;
    }
    gcd(b, a % b)
}
