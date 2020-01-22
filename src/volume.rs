use serde_derive::{Deserialize, Serialize};
use crate::geometry::CoordTransform;
use crate::Result;

use std::path::Path;
use std::fs::File;
pub fn write_bin<T: serde::Serialize>(
    path: &Path, data: &T
) -> Result<()> {
    let file = File::create(path)?;
    let buf = std::io::BufWriter::with_capacity(0x100000, file);
    serde_cbor::to_writer(buf, data)?;
    Ok(())
}

pub fn read_bin<T: for<'a> serde::Deserialize<'a>>(path: &Path) -> Result<T> {
    let file = std::fs::File::open(path)?;
    let file = unsafe { memmap::MmapOptions::new().map(&file)? };
    Ok(serde_cbor::from_slice(file.as_ref())?)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VolumePrecomputeMetadata {
    pub transform: CoordTransform,
    pub projection: String,
    pub levels: usize,
    pub chunks_y_offset: usize,
    pub levels_data: Vec<(usize, usize)>,
}

// #[derive(Serialize, Deserialize, Debug)]
// pub struct Volume {
//     pub volume: f64,
//     pub error: f64,
// }

// impl Default for Volume {
//     fn default() -> Volume {
//         Volume {
//             volume: 0.,
//             error: 0.,
//             moments:  Default::default(),
//         }
//     }
// }
// impl std::ops::AddAssign for Volume {
//     fn add_assign(&mut self, other: Volume) {
//         self.volume += other.volume;
//         self.error += other.error;
//         self.moments += other.moments;
//     }
// }
// impl std::iter::Sum<Volume> for Volume {
//     fn sum<I: Iterator<Item = Volume>>(iter: I) -> Self {
//         let mut out = Default::default();
//         for v in iter {
//             out += v;
//         }
//         out
//     }
// }

use cgmath::{Vector3, Matrix3};
#[derive(Serialize, Deserialize, Debug)]
pub struct Moments {
    pub count: usize,
    pub sum: Vector3<f64>,
    pub sum_2: Matrix3<f64>,
}
impl Default for Moments {
    fn default() -> Moments {
        use num::Zero;
        Moments {
            count: 0,
            sum: Vector3::zero(),
            sum_2: Matrix3::zero(),
        }
    }
}
impl std::ops::AddAssign for Moments {
    fn add_assign(&mut self, other: Moments) {
        self.count += other.count;
        self.sum += other.sum;
        self.sum_2 += other.sum_2;
    }
}
