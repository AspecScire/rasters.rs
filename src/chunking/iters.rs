use super::{mod_ceil, ChunkConfig, ChunkWindow};
use std::{iter::*, ops::Range};

impl<'a> IntoIterator for &'a ChunkConfig {
    type Item = ChunkWindow<'a>;
    type IntoIter = Map<Range<usize>, Box<dyn Fn(usize) -> ChunkWindow<'a> + 'a>>;

    fn into_iter(self) -> Self::IntoIter {
        let (count, func) = self.iter_mapper();
        (0..count).map(Box::new(func))
    }
}

impl ChunkConfig {
    fn check_preconditions(&self) {
        debug_assert!(
            self.block_size > 0
                && self.start >= self.padding
                && self.end <= self.height
                && self.data_height % self.block_size == 0,
            "ChunkConfig preconditions failed"
        );
    }

    fn calc_initial_chunk(&self) -> [usize; 3] {
        if self.start >= self.end {
            return [0, 0, 0];
        }

        let mut data_end = (self.start + self.data_height).min(self.end);
        debug_assert!(data_end > self.start);

        // For the initial chunk, we ensure the load ends at
        // a chunk boundary. This would increase the size of
        // the chunk, but by at most one block.
        let mut load_end = mod_ceil(data_end + self.padding, self.block_size).min(self.height);
        debug_assert!(load_end > self.start);

        // The whole raster may be too narrow for the given
        // padding, but we still yield it as the padding
        // might be an upper bound.
        data_end = (load_end - self.padding).max(self.start);

        // We may have extended load_end much more than
        // needed to find a block boundary if self.end is
        // much smaller. In this case the final load_end is
        // not at a block boundary, but the iterator has
        // only one element in this case.
        if data_end > self.end {
            data_end = self.end;
            load_end = data_end + self.padding;
        }

        let count = {
            let dcount = mod_ceil(self.end - data_end, self.data_height) / self.data_height;
            let lcount = mod_ceil(self.height - load_end, self.data_height) / self.data_height;
            dcount.min(lcount)
        } + 1;
        debug_assert!(count == 1 || load_end % self.block_size == 0);

        [count, data_end, load_end]
    }

    pub(super) fn iter_mapper<'a>(&'a self) -> (usize, impl Fn(usize) -> ChunkWindow<'a> + 'a) {
        self.check_preconditions();

        let [count, initial_data_end, initial_load_end] = self.calc_initial_chunk();

        (count, move |i| {
            let (data_start, _, load_end) = if i == 0 {
                (self.start, initial_data_end, initial_load_end)
            } else if i < count - 1 {
                let data_start = initial_data_end + (i - 1) * self.data_height;
                let data_end = data_start + self.data_height;
                let load_end = data_end + self.padding;
                (data_start, data_end, load_end)
            } else {
                let data_start = initial_data_end + (i - 1) * self.data_height;
                let data_end = (data_start + self.data_height).min(self.end);
                let load_end = (data_end + self.padding).min(self.height);
                let data_end = load_end - self.padding;
                (data_start, data_end, load_end)
            };
            let load_start = data_start - self.padding;
            (self, load_start, (load_end - load_start) as usize)
        })
    }

    /// Create an [ `ExactSizeIterator` ] from the configuration.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = ChunkWindow> + '_ {
        let (count, func) = self.iter_mapper();
        (0..count).map(func)
    }
}
