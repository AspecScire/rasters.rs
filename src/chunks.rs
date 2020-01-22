use gdal::raster::*;
use super::Result;

use num::Zero;
use types::GdalType;
use ndarray::Array2;

pub type Chunk<T> = (isize, Array2<T>);

/// Returns an iterator that loads memory-efficient chunks,
/// and returns padded chunks for processing. Useful to
/// process spatially-local filters on large rasters. This
/// is a wrapper around [`data_iterator`] and
/// [`offsets_iterator`] for the common case of scanning
/// through a band.
///
/// Note: this iterator is not [`Send`] or [`Sync`] if
/// called with a [`RasterBand`]. Use either a
/// [`DatasetReader`] for `Send`, or a `RasterPathReader`
/// for `Sync`.
///
/// # Arguments
///
/// - `reader` - raster band or generally a type that
///   implements [`ChunkReader`]
///
/// - `pad_size` - padding required
///
/// - `chunk_size` - required chunk size (incl. padding). An
///   optimal chunk_size respecting raster block sizes can
///   be calculated using [`size_with_padding`].
///
/// - `start` - offset to start from (incl. padding). The
///   first block of data starts at this offset.
///
/// - `end` - offset before which to end. The last block of
///   data ends at one before this offset.
///
/// # Panics
///
/// Panics if the chunk size is not (strictly) larger than
/// 2 * pad_size.
pub fn band_iterator<'a,
                     T: Zero + GdalType + Copy + 'a,
                     R: ChunkReader + 'a>(
    reader: R,
    pad_size: usize, chunk_size: usize,
) -> impl ExactSizeIterator<Item=Result<Chunk<T>>> + 'a {
    let (width, height) = reader.size();
    let iter = offsets_iterator(pad_size, chunk_size,
                                0, height as isize);
    data_iterator(reader, iter, 0, width)
}

/// Returns an iterator that loads memory-efficient chunks,
/// and returns padded chunks for processing. Useful to
/// process spatially-local filters on large rasters.
///
/// Note: this iterator is not [`Send`] or [`Sync`] if
/// called with a [`RasterBand`]. Use either a
/// [`DatasetReader`] for `Send`, or a `RasterPathReader`
/// for `Sync`.
///
/// # Arguments
///
/// - `reader` - raster band or generally a type that
///   implements [`ChunkReader`]
///
/// - `iter` - iterator that yields (start, load size, copy
///   size). Use [`offsets_iterator`] for a possible such
///   iterator.
///
/// - `x_size` - width of the raster
///
/// # Panics
///
/// Panics if the iterator yields a copy size bigger than
/// size of the previous load. In particular, it also panics
/// if the first element yielded has a non-zero copy size.
pub fn data_iterator<'a,
                     T: Zero + GdalType + Copy + 'a,
                     R: ChunkReader + 'a,
                     I: ExactSizeIterator<Item = (isize, usize, usize)>>(
    reader: R, iter: I,
    x_off: isize, x_size: usize,
) -> impl ExactSizeIterator<Item = Result<Chunk<T>>> + 'a
where <I as IntoIterator>::IntoIter: 'a {

    let mut buf = vec![];

    iter.into_iter().map(move |(start, load, copy)| {

        let buf_size = (load + copy) * x_size;
        let copy_size = copy * x_size;
        let mut outbuf = Vec::with_capacity(buf_size);

        // Copy previous data
        if buf.len() < copy_size {
            panic!("buffer did not contain enough data to copy");
        } else {
            let offset = buf.len() - copy_size;
            outbuf.extend_from_slice(&buf[offset..]);
            outbuf.resize(buf_size, T::zero());
        }

        if let Err(e) = reader.read_into_slice(
            &mut outbuf[copy_size..],
            (x_off, start as isize),
            (x_size, load),
        ) {
            return Err(e.into());
        }

        buf = outbuf[copy_size..].into();
        Ok((start - copy as isize,
            Array2::from_shape_vec((load+copy, x_size), outbuf)?))
    })

}

/// Returns an iterator over memory-efficient chunk
/// boundaries. The iterator yields (offset, load_size,
/// copy_size) where:
///
/// - `offset: isize` is the offset to load data.
/// - `load_size: usize` is the size of block to load.
/// - `copy_size: usize` is the size to copy from end
///   of the previous block.
///
/// # Arguments
///
/// - `pad_size` - padding required
///
/// - `chunk_size` - required chunk size (incl. padding). An
///   optimal chunk_size respecting raster block sizes can
///   be calculated using [`size_with_padding`].
///
/// - `start` - offset to start from (incl. padding). The
///   first block of data starts at this offset.
///
/// - `end` - offset before which to end. The last block of
///   data ends at one before this offset.
///
/// # Panics
///
/// Panics if the chunk size is not (strictly) larger than
/// 2 * pad_size.
pub fn offsets_iterator(
    pad_size: usize, chunk_size: usize,
    start: isize, end: isize,
) -> impl ExactSizeIterator<Item = (isize, usize, usize)> {

    let pad_size = 2 * pad_size;
    if chunk_size <= pad_size {
        panic!("chunk_size too small for req. padding");
    }

    (start..end)
        .step_by(chunk_size)
        .map(move |idx| {

            let load_size = chunk_size.min((end - idx) as usize);

            if idx == start {
                (idx, load_size, 0)
            } else {
                (idx, load_size, pad_size)
            }

        })

}

/// Calculate optimal size to load data from rasters.
/// Calculated as the smallest multiple of the raster's
/// block size that is larger than 2*pad_size +
/// max(chunk_size, 1).
///
/// # Arguments
///
/// - `block_size` - the block size of the raster
/// - `chunk_size` - the required chunk size (without padding)
/// - `pad_size`   - the padding required (at both ends)
pub fn size_with_padding(block_size: usize, chunk_size: usize, pad_size: usize) -> usize {
    let pad_size = pad_size * 2;
    let chunk_size = chunk_size.max(1);
    let chunk_size = (chunk_size + pad_size + block_size - 1) / block_size;
    let chunk_size = chunk_size * block_size;

    chunk_size
}

pub trait ChunkReader {
    fn read_into_slice<T: Copy + GdalType>(
        &self, out: &mut [T],
        off: (isize, isize),
        size: (usize, usize),
    ) -> Result<()>;

    fn size(&self) -> (usize, usize);

    fn read_as_array<T: Copy + GdalType + Zero>(
        &self, off: (isize, isize), size: (usize, usize)
    ) -> Result<Array2<T>> {
        let bufsize = size.0 * size.1;
        let mut buf = Vec::with_capacity(bufsize);
        buf.resize(bufsize, T::zero());
        self.read_into_slice(&mut buf[..], off, size)?;
        Ok(Array2::from_shape_vec((size.1, size.0), buf)?)
    }
}

impl<'a> ChunkReader for RasterBand<'a> {
    fn read_into_slice<T: Copy + GdalType>(
        &self, out: &mut [T],
        off: (isize, isize),
        size: (usize, usize),
    ) -> Result<()> {
        Ok(self.read_into_slice(off, size, size, out)?)
    }

    fn size(&self) -> (usize, usize) {
        RasterBand::size(self)
    }
}

pub struct DatasetReader(pub Dataset, pub isize);
impl ChunkReader for DatasetReader {
    fn read_into_slice<T: Copy + GdalType>(
        &self, out: &mut [T],
        off: (isize, isize),
        size: (usize, usize),
    ) -> Result<()> {
        let band = self.0.rasterband(self.1)?;
        ChunkReader::read_into_slice(&band, out, off, size)
    }

    fn size(&self) -> (usize, usize) {
        Dataset::size(&self.0)
    }
}

use std::path::Path;
pub struct RasterPathReader<'a>(pub &'a Path, pub isize);
impl<'a> ChunkReader for RasterPathReader<'a> {
    fn read_into_slice<T: Copy + GdalType>(
        &self, out: &mut [T],
        off: (isize, isize),
        size: (usize, usize),
    ) -> Result<()> {
        let ds = Dataset::open(&self.0)?;
        let reader = DatasetReader(ds, self.1);
        ChunkReader::read_into_slice(&reader, out, off, size)
    }

    fn size(&self) -> (usize, usize) {
        Dataset::open(&self.0).unwrap().size()
    }
}

mod tests {
    #[test]
    fn size_with_padding() {
        assert_eq!(
            super::size_with_padding(10, 5, 3),
            20,
        );
        assert_eq!(
            super::size_with_padding(1, 0, 3),
            7,
        );
    }

    #[test]
    fn offsets_iterator() {
        let offsets: Vec<_> = super::offsets_iterator(1, 3, 1, 14)
            .collect();
        assert_eq!(offsets,
                   vec![(1, 3, 0),
                        (4, 3, 2),
                        (7, 3, 2),
                        (10, 3, 2),
                        (13, 1, 2)]);
    }
}
