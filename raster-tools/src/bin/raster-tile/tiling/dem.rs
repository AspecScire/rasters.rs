use ndarray::Array2;
use rasters::Result;

pub struct TileSet {
    tiles: Vec<Tile>,
    xrange: Dims,
    y: usize,
    zoom: usize,
}

impl TileSet {
    pub fn new<I: IntoIterator<Item = Tile>>(
        zoom: usize,
        xrange: Dims,
        y: usize,
        tiles: I,
    ) -> Self {
        let tiles: Vec<_> = tiles.into_iter().collect();

        let (left, right) = xrange;
        assert!(tiles.len() == right - left);

        TileSet {
            tiles,
            xrange,
            y,
            zoom,
        }
    }

    pub fn zoom(&self) -> usize {
        self.zoom
    }

    pub fn can_scale_down_with_top(&self) -> bool {
        self.y % 2 == 1
    }

    pub fn scale_down_as_top(&mut self) {
        assert!(!self.can_scale_down_with_top());
        let (left, right) = self.xrange;
        // eprintln!("Scaling down as top:");
        // eprintln!("\tzoom={}, left={}, right={}", self.zoom, left, right);
        // eprintln!("\ty={}", self.y);
        let tiles = std::mem::replace(&mut self.tiles, vec![]);

        let mut prev = None;
        for (x, tile) in (left..right).zip(tiles) {
            if x % 2 == 1 {
                let corners = [None, None, prev.take(), Some(tile)];
                self.tiles.push(Tile::scale_4_to_1(corners));
            } else if x == right - 1 {
                let corners = [None, None, Some(tile), None];
                self.tiles.push(Tile::scale_4_to_1(corners));
            } else {
                prev = Some(tile);
            }
        }
        self.xrange = (left / 2, (right - 1) / 2 + 1);
        self.y /= 2;
        self.zoom -= 1;
    }

    pub fn scale_down_with_top(&mut self, other: Option<Self>) {
        assert!(self.can_scale_down_with_top());

        let (left, right) = self.xrange;
        // eprintln!("Scaling down with top:");
        // eprintln!("\tzoom={}, left={}, right={}", self.zoom, left, right);
        // eprintln!("\ttop={}, bot={}",
        //           other.as_ref().map(|o| o.y).unwrap_or(0),
        //           self.y);
        let tiles = std::mem::replace(&mut self.tiles, vec![]);

        let pairs: Vec<_> = if let Some(other) = other {
            let otiles = other.tiles;
            assert!(tiles.len() == otiles.len());
            tiles
                .into_iter()
                .zip(otiles.into_iter().map(Some))
                .collect()
        } else {
            tiles.into_iter().map(|t| (t, None)).collect()
        };

        let mut oprev = None;
        let mut prev = None;
        for (x, (tile, otile)) in (left..right).zip(pairs) {
            if x % 2 == 1 {
                let corners = [prev.take(), Some(tile), oprev.take(), otile];
                self.tiles.push(Tile::scale_4_to_1(corners));
            } else if x == right - 1 {
                let corners = [Some(tile), None, otile, None];
                self.tiles.push(Tile::scale_4_to_1(corners));
            } else {
                prev = Some(tile);
                oprev = otile;
            }
        }

        self.xrange = (left / 2, (right - 1) / 2 + 1);
        self.y /= 2;
        self.zoom -= 1;
    }

    pub fn write(&self, base_path: &Path) -> Result<YIndex> {
        let base_path = base_path
            .join(&format!("{}", self.zoom))
            .join(&format!("{}", self.y));
        std::fs::create_dir_all(&base_path)?;

        use rayon::prelude::*;
        let idx = self
            .tiles
            .par_iter()
            .map(|tile| -> Result<_> {
                let (x, _) = tile.coords();
                let path = base_path.join(&format!("{}.bin", x));
                let cfg = tile.write(&path)?;
                Ok((x, cfg))
            })
            .try_fold(
                || YIndex::new(self.y),
                |mut idx, cfg| -> Result<_> {
                    let (x, cfg) = cfg?;
                    idx.add_to_index(x, cfg);
                    Ok(idx)
                },
            )
            .try_reduce(
                || YIndex::new(self.y),
                |mut idx1, idx2| {
                    idx1.combine(idx2);
                    Ok(idx1)
                },
            )?;
        Ok(idx)
    }
}

#[derive(Debug)]
pub struct Tile {
    data: Array2<f64>,
    data_range: (f64, f64),
    coords: Dims,
}

use std::path::Path;
impl Tile {
    pub fn from_aggregate(data: Array2<(f64, f64)>, coords: Dims) -> Self {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let data = data.map(|(val, count)| {
            let count = *count;
            if count.is_nan() {
                count
            } else {
                assert!(!val.is_nan());
                let x = val / count;
                max = max.max(x);
                min = min.min(x);
                x
            }
        });
        Tile {
            data,
            data_range: (min, max),
            coords,
        }
    }

    pub fn coords(&self) -> Dims {
        self.coords
    }

    pub fn scale_4_to_1(corners: [Option<Self>; 4]) -> Self {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let mut checked_average = |vals: [f64; 4]| -> f64 {
            let mut val = 0.;
            let mut count = 0;
            for v in vals.iter() {
                if !v.is_nan() {
                    val += v;
                    count += 1;
                }
            }
            if count > 0 {
                let val = val / count as f64;
                min = min.min(val);
                max = max.max(val);
                val
            } else {
                f64::NAN
            }
        };

        let (rows, cols, coords) = {
            let some = corners
                .iter()
                .find(|c| c.is_some())
                .expect("non-empty corner");
            let some = some.as_ref().unwrap();

            let (x, y) = some.coords;
            let (r, c) = some.data.dim();
            (r, c, (x / 2, y / 2))
        };

        assert!(rows % 2 == 0);
        assert!(cols % 2 == 0);

        let mut data = Array2::from_elem((rows, cols), f64::NAN);

        for r in 0..rows {
            for c in 0..cols {
                let sr = 2 * r;
                let sc = 2 * c;

                let mut sidx = 0;
                if sr >= rows {
                    sidx += 2;
                }
                if sc >= cols {
                    sidx += 1;
                }

                let sr = sr % rows;
                let sc = sc % cols;

                let val = corners[sidx]
                    .as_ref()
                    .map(|tile| {
                        checked_average([
                            tile.data[(sr, sc)],
                            tile.data[(sr + 1, sc)],
                            tile.data[(sr, sc + 1)],
                            tile.data[(sr + 1, sc + 1)],
                        ])
                    })
                    .unwrap_or(f64::NAN);
                data[(r, c)] = val;
            }
        }

        Tile {
            data,
            coords,
            data_range: (min, max),
        }
    }

    pub fn write(&self, path: &Path) -> Result<TileStats> {
        let file = std::fs::File::create(&path)?;
        let mut buf = std::io::BufWriter::with_capacity(0x100000, file);

        let bins = (1 << 16) - 1;
        let (min, max) = self.data_range;

        let mut err: f64 = 0.;

        let coeff = bins as f64 / (max - min);

        use std::io::Write;
        self.data.iter().try_for_each(|val| -> Result<()> {
            let mut val = *val;
            if val.is_nan() {
                buf.write(&[0, 0])?;
            } else {
                if val < min {
                    val = min;
                } else if val > max {
                    val = max;
                }

                let disc = (val - min) * coeff;
                let mut disc = disc.floor() as u16;

                let rec = min + (max - min) * disc as f64 / bins as f64;
                err = err.max((val - rec).abs());

                if disc < bins as u16 {
                    disc = disc + 1;
                }
                let msb = disc >> 8;
                let lsb = disc % (1 << 8);
                buf.write(&[msb as u8, lsb as u8])?;
            }
            Ok(())
        })?;

        Ok(TileStats {
            min,
            max,
            bins,
            err,
        })
    }
}

use serde_derive::Serialize;

#[derive(Serialize)]
pub struct TileStats {
    min: f64,
    max: f64,
    bins: usize,
    err: f64,
}

use std::collections::HashMap;

use super::Dims;
#[derive(Serialize)]
pub struct YIndex {
    y: usize,
    index: HashMap<usize, TileStats>,
}

impl YIndex {
    pub fn new(y: usize) -> Self {
        YIndex {
            y,
            index: Default::default(),
        }
    }

    pub fn add_to_index(&mut self, x: usize, cfg: TileStats) {
        self.index.insert(x, cfg);
    }
    pub fn combine(&mut self, other: YIndex) {
        assert!(self.y == other.y);
        self.index.extend(other.index);
    }
}

#[derive(Serialize, Default)]
pub struct Index {
    #[serde(flatten)]
    index: HashMap<usize, HashMap<usize, YIndex>>,
}
impl Index {
    pub fn update_index(&mut self, zoom: usize, idx: YIndex) {
        let y = idx.y;

        let map = &mut self.index;
        if !map.contains_key(&zoom) {
            map.insert(zoom, HashMap::new());
        }

        let inner_map = map.get_mut(&zoom).unwrap();
        inner_map.insert(y, idx);
    }
}

use std::ops::AddAssign;
impl AddAssign for Index {
    fn add_assign(&mut self, rhs: Self) {
        for (z, idx2) in rhs.index {
            if let Some(idx1) = self.index.get_mut(&z) {
                idx1.extend(idx2);
            } else {
                self.index.insert(z, idx2);
            }
        }
    }
}
