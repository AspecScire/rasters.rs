//! Utilities to compute histogram

use serde_derive::Serialize;

/// Configuration to generate histogram. Can be constructed
/// from min, max and either step-size or number of bins.
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Config {
    min: f64,
    max: f64,
    step: f64,
    len: usize,
}

impl Config {
    pub fn from_min_max_step(min: f64, max: f64, step: f64) -> Self {
        assert!(min <= max, "min must be smaller than max");
        let len = ((max - min) / step).ceil() as usize;
        Config {
            min,
            max,
            step,
            len,
        }
    }
    pub fn from_min_max_bins(min: f64, max: f64, len: usize) -> Self {
        assert!(min <= max, "min must be smaller than max");
        let step = (max - min) / len as f64;
        Config {
            min,
            max,
            step,
            len,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn step(&self) -> f64 {
        self.step
    }

    #[inline]
    pub fn max(&self) -> f64 {
        self.max
    }

    #[inline]
    pub fn min(&self) -> f64 {
        self.min
    }

    #[inline]
    pub fn bin_for(&self, val: f64) -> HistBin {
        use HistBin::*;
        if val >= self.max {
            Max
        } else if val < self.min {
            Min
        } else {
            let bin = ((val - self.min) / self.step).floor() as usize;
            if bin >= self.len {
                Max
            } else {
                Bin(bin)
            }
        }
    }
}

/// Represent the location of a value with respect to a
/// histogram configuration.
pub enum HistBin {
    Min,
    Max,
    Bin(usize),
}

/// A histogram that can be built by accumulating individual
/// values, or other histograms.
#[derive(Clone, Serialize)]
pub struct Histogram<'a> {
    cfg: &'a Config,
    hist: Vec<usize>,
    min: usize,
    max: usize,
    count: usize,
}
impl<'a> Histogram<'a> {
    pub fn new(cfg: &'a Config) -> Self {
        Histogram {
            cfg,
            hist: vec![0; cfg.len()],
            min: 0,
            max: 0,
            count: 0,
        }
    }
}

use std::ops::AddAssign;
impl<'a, 'b> AddAssign<Histogram<'b>> for Histogram<'a> {
    fn add_assign(&mut self, other: Histogram<'b>) {
        assert!(
            self.cfg == other.cfg,
            "adding histogram with a different config"
        );
        for (a, b) in self.hist.iter_mut().zip(other.hist.iter()) {
            *a += *b;
        }
        self.min += other.min;
        self.max += other.max;
        self.count += other.count;
    }
}
impl<'a> AddAssign<f64> for Histogram<'a> {
    fn add_assign(&mut self, other: f64) {
        use HistBin::*;
        match self.cfg.bin_for(other) {
            Min => {
                self.min += 1;
            }
            Max => {
                self.max += 1;
            }
            Bin(bin) => {
                self.hist[bin] += 1;
            }
        }
        self.count += 1;
    }
}
