use super::Chunk;
use ndarray::Array2;

pub type MultiBandChunk<T> = (isize, Vec<Array2<T>>);

pub fn mask_chunk(input_chunk: &MultiBandChunk<f64>, no_val: f64) -> Chunk<u8> {
    let (ht, wid) = input_chunk.1[0].dim();
    let mut mask = Array2::<u8>::zeros((ht, wid));
    let band_count = input_chunk.1.len();

    let is_data = |x, y| {
        // For RGB without mask, no data if _all_ bands have no_val
        if band_count == 3 {
            let r_band = &input_chunk.1[0];
            let g_band = &input_chunk.1[1];
            let b_band = &input_chunk.1[2];

            !(r_band[(y, x)] == no_val && g_band[(y, x)] == no_val && b_band[(y, x)] == no_val)
        } else {
            let val = input_chunk.1[band_count - 1][(y, x)];

            !val.is_nan() && val != no_val
        }
    };

    for y in 0..ht {
        for x in 0..wid {
            mask[(y, x)] = if is_data(x, y) { 255 } else { 0 };
        }
    }
    (input_chunk.0, mask)
}
