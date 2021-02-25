use rasters::stats::PixelStats;
use serde_derive::Serialize;
use std::ops::AddAssign;

#[derive(Serialize, Clone)]
pub struct RasterDiffOutput {
    pub pix_area_1: f64,
    pub pix_area_2: f64,
    pub stats: RasterDiffStats,
}

#[derive(Serialize, Clone, Default)]
pub struct RasterDiffStats {
    count: usize,
    first: PixelStats,
    second: PixelStats,
    diff: PixelStats,
    abs_diff: PixelStats,
}
impl AddAssign<(f64, f64)> for RasterDiffStats {
    fn add_assign(&mut self, other: (f64, f64)) {
        self.count += 1;
        self.first += other.0;
        self.second += other.1;
        let diff = other.1 - other.0;
        self.diff += diff;
        self.abs_diff += diff.abs();
    }
}

impl AddAssign for RasterDiffStats {
    fn add_assign(&mut self, other: RasterDiffStats) {
        self.count += other.count;
        self.first += other.first;
        self.second += other.second;
        self.diff += other.diff;
        self.abs_diff += other.abs_diff;
    }
}
