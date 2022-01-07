//! Abstractions to safely read GDAL datasets from multiple
//! threads.

use crate::chunking::ChunkConfig;
use crate::geometry::{RasterDims, RasterOffset};
use crate::Result;
use anyhow::{format_err, Context};
use gdal::{
    raster::{GdalType, RasterBand},
    Dataset,
};
use ndarray::Array2;

/// Abstracts reading chunks from raster.
pub trait ChunkReader {
    /// Emulate [`RasterBand::read_into_slice`].
    fn read_into_slice<T>(&self, out: &mut [T], off: RasterOffset, size: RasterDims) -> Result<()>
    where
        T: GdalType + Copy;

    /// Helper to read into an ndarray.
    fn read_as_array<T>(&self, off: RasterOffset, size: RasterDims) -> Result<Array2<T>>
    where
        T: GdalType + Copy,
    {
        let bufsize = size.0 * size.1;
        let mut buf = Vec::with_capacity(bufsize);

        // Safety: paradigm suggested in std docs
        // https://doc.rust-lang.org/std/vec/struct.Vec.html#examples-18
        unsafe {
            buf.set_len(bufsize);
        }

        self.read_into_slice(&mut buf[..], off, size)?;
        Ok(Array2::from_shape_vec((size.1, size.0), buf)?)
    }

    /// Helper to read into slice from output of
    /// [`ChunkConfig`] iterator
    fn read_chunk_into_slice<T>(
        &self,
        out: &mut [T],
        chunk: (&ChunkConfig, usize, usize),
    ) -> Result<()>
    where
        T: GdalType + Copy,
    {
        let (cfg, start, end) = chunk;
        let width = cfg.width();
        let height = end - start;
        self.read_into_slice(out, (0 as isize, start as isize), (width, height))
    }

    /// Helper to read ndarray from output of
    /// [`ChunkConfig`] iterator
    fn read_chunk<T>(&self, chunk: (&ChunkConfig, usize, usize)) -> Result<Array2<T>>
    where
        T: GdalType + Copy,
    {
        let (cfg, start, height) = chunk;
        let width = cfg.width();
        self.read_as_array((0 as isize, start as isize), (width, height))
    }
}

impl<'a> ChunkReader for RasterBand<'a> {
    fn read_into_slice<T>(&self, out: &mut [T], off: RasterOffset, size: RasterDims) -> Result<()>
    where
        T: GdalType + Copy,
    {
        Ok(self
            .read_into_slice(off, size, size, out, None)
            .with_context(|| {
                format_err!(
                    "reading window @ ({},{}) of dimension ({}x{})",
                    off.0,
                    off.1,
                    size.0,
                    size.1
                )
            })?)
    }
}

/// A `ChunkReader` that is `Send`, but not `Sync`. Obtains
/// a `RasterBand` handle for each read.
pub struct DatasetReader(pub Dataset, pub isize);

impl ChunkReader for DatasetReader {
    fn read_into_slice<T>(&self, out: &mut [T], off: RasterOffset, size: RasterDims) -> Result<()>
    where
        T: GdalType + Copy,
    {
        let band = self.0.rasterband(self.1)?;
        ChunkReader::read_into_slice(&band, out, off, size)
    }
}

/// A `ChunkReader` that is both `Send` and `Sync`. Opens
/// the dataset for each read. `P` may be set to [ `Path` ]
/// or a `PathBuf` for a `Send + Sync` reader.
pub struct RasterPathReader<'a, P: ?Sized>(pub &'a P, pub isize);

use std::path::Path;
impl<'a, P> ChunkReader for RasterPathReader<'a, P>
where
    P: AsRef<Path> + ?Sized,
{
    fn read_into_slice<T>(&self, out: &mut [T], off: RasterOffset, size: RasterDims) -> Result<()>
    where
        T: GdalType + Copy,
    {
        DatasetReader(Dataset::open(self.0.as_ref())?, self.1).read_into_slice(out, off, size)
    }
}
