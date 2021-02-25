use rayon::iter::Map;
use rayon::prelude::*;
use rayon::range::Iter;

use super::*;

impl ChunkConfig {
    /// Create an [ `IndexedParallelIterator` ] from the configuration.
    ///
    /// This function is only available with the "use-rayon" feature.
    pub fn par_iter(&self) -> impl IndexedParallelIterator<Item = ChunkWindow> {
        let (count, func) = self.iter_mapper();
        (0..count).into_par_iter().map(func)
    }
}

impl<'a> IntoParallelIterator for &'a ChunkConfig {
    type Item = ChunkWindow<'a>;
    type Iter = Map<Iter<usize>, Box<dyn Fn(usize) -> ChunkWindow<'a> + Send + Sync + 'a>>;

    fn into_par_iter(self) -> Self::Iter {
        let (count, func) = self.iter_mapper();
        (0..count).into_par_iter().map(Box::new(func))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_output() {
        let cfg = ChunkConfig::with_dims(1024, 1024)
            .add_block_size(7)
            .with_min_data_size(0x1000)
            .with_padding(3)
            .with_start(13)
            .with_end(999);

        let output1: Vec<_> = cfg
            .into_iter()
            // .map(|(_, a, b)| (a, b))
            .collect();

        let mut output2 = vec![];
        cfg.into_par_iter()
            // .map(|(_, a, b)| (a, b))
            .collect_into_vec(&mut output2);

        assert_eq!(output1, output2);
    }
}
