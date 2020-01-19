use rasters::chunks::Chunk;
use cgmath::{Matrix3, Point2};
use super::triangulation::Triangulation;

pub fn fill_chunk(
    chunk: &mut Chunk<f64>, no_val: f64,
    transform: Matrix3<f64>, triangulation: &Triangulation,
    sibson: f64,
) -> usize {
    use std::f64::NAN;

    let mut count = 0;
    let (ht, wid) = chunk.1.dim();
    let data = &mut chunk.1;
    let start_y = chunk.0;
    for y in 0..ht {
        for x in 0..wid {
            let val = data[(y,x)];
            if (val == NAN) || (val == no_val) {
                let pt = {
                    use cgmath::Vector3;
                    let pt = Vector3::new(
                        x as f64 + 0.5,
                        (y as isize + start_y) as f64 + 0.5,
                        1.,
                    );
                    let pt = transform * pt;
                    Point2::new(pt.x, pt.y)
                };
                // NN c1 sibson
                let val =  triangulation.nn_interpolation_c1_sibson(
                    &pt, sibson,
                    |v| v.height,
                    |_, v| v.gradient,
                ).unwrap();

                // Farin: slow
                // let val = triangulation.nn_interpolation_c1_farin(
                //     &pt, |v| v.height, |_, v| v.gradient,
                // ).unwrap();

                // Barycentric: very fast
                // let val = triangulation.barycentric_interpolation(
                //     &pt, |v| v.height).unwrap();

                data[(y,x)] = val;
                count += 1;
            }
        }
    }
    count
}
