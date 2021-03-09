//! Utilities to accumulate first and second moments; min;
//! and max of a `f64` statistic incrementally.
use serde_derive::Serialize;
use std::ops::AddAssign;

/// Stores the statistics collected from a `f64` random
/// variable. Accumulation of the statistic is done by
/// add-assigning (using `+=`) one of the following.
///
/// - a `f64` value.  Adds a new sample
/// - a `(f64, f64)` tuple.  Adds the first component with weight specified by the second component.
/// - another `PixelStats` value.  Accumulates the statistic from the other into `self`.
#[derive(Debug, Serialize, Clone)]
pub struct PixelStats {
    max: f64,
    min: f64,
    sum: f64,
    sum_2: f64,
    count: f64,
}

impl Default for PixelStats {
    fn default() -> Self {
        use std::f64::*;
        PixelStats {
            max: NEG_INFINITY,
            min: INFINITY,
            sum: 0.,
            sum_2: 0.,
            count: 0.,
        }
    }
}
impl AddAssign<(f64, f64)> for PixelStats {
    fn add_assign(&mut self, other: (f64, f64)) {
        self.max = self.max.max(other.0);
        self.min = self.min.min(other.0);
        self.sum += other.0;
        self.sum_2 += other.0 * other.0;
        self.count += other.1;
    }
}

impl AddAssign<f64> for PixelStats {
    fn add_assign(&mut self, other: f64) {
        *self += (other, 1.);
    }
}
impl AddAssign<&PixelStats> for PixelStats {
    fn add_assign(&mut self, other: &PixelStats) {
        self.max = self.max.max(other.max);
        self.min = self.min.min(other.min);
        self.sum += other.sum;
        self.sum_2 += other.sum_2;
        self.count += other.count;
    }
}

impl PixelStats {
    #[inline]
    pub fn max(&self) -> f64 {
        self.max
    }

    #[inline]
    pub fn min(&self) -> f64 {
        self.min
    }

    #[inline]
    pub fn sum(&self) -> f64 {
        self.sum
    }

    #[inline]
    pub fn sum_2(&self) -> f64 {
        self.sum_2
    }

    #[inline]
    pub fn count(&self) -> f64 {
        self.count
    }

    #[inline]
    pub fn mean(&self) -> f64 {
        self.sum / self.count
    }

    #[inline]
    pub fn variance(&self) -> f64 {
        self.sum_2 / self.count
    }

    #[inline]
    pub fn std_deviation(&self) -> f64 {
        self.variance().sqrt()
    }
}
