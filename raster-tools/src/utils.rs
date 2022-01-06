//! Utilities to create, read and write raster datasets.

use gdal::DatasetOptions;
use gdal::GdalOpenFlags;
use rasters::Result;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;

pub type InputArgs = PathBuf;
pub struct OutputArgs {
    pub path: PathBuf,
    pub driver: String,
}

use anyhow::Context;
use gdal::{Dataset, Driver};

pub fn read_dataset(path: &Path) -> Result<Dataset> {
    Ok(Dataset::open(&path).with_context(|| format!("reading dataset {}", path.display()))?)
}

pub fn edit_dataset(path: &Path) -> Result<Dataset> {
    Ok(Dataset::open_ex(
        &path,
        DatasetOptions {
            open_flags: GdalOpenFlags::GDAL_OF_UPDATE,
            ..Default::default()
        },
    )
    .with_context(|| format!("editing dataset {}", path.display()))?)
}

use gdal::raster::GdalType;
pub fn create_output_raster<T: GdalType>(
    arg: &OutputArgs,
    ds: &Dataset,
    num_bands: isize,
    no_val: Option<f64>,
) -> Result<Dataset> {
    let mut out_ds = {
        let driver = Driver::get(&arg.driver)?;
        let (width, height) = ds.raster_size();
        driver
            .create_with_band_type::<T, _>(&arg.path, width as isize, height as isize, num_bands)
            .with_context(|| format!("creating dataset {}", arg.path.display()))?
    };
    if let Some(no_val) = no_val {
        for i in 1..=num_bands {
            out_ds.rasterband(i)?.set_no_data_value(no_val)?;
        }
    }
    if let Ok(gt) = ds.geo_transform() {
        out_ds.set_geo_transform(&gt)?;
    }
    out_ds.set_projection(&ds.projection())?;
    Ok(out_ds)
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::*;
    use tempdir::TempDir;

    const WIDTH: usize = 16;
    const HEIGHT: usize = 32;

    #[test]
    fn create_read_update_ds() -> Result<()> {
        let driver = Driver::get("GTIFF")?;
        let tmp_dir = TempDir::new("rasters_test").unwrap();
        let path = tmp_dir.path().join("foo.tif");

        // Create empty raster
        {
            driver.create_with_band_type::<f64, _>(&path, WIDTH as isize, HEIGHT as isize, 1)?;
        }

        // Create random data
        let data = {
            use gdal::raster::Buffer;
            let mut data: Vec<f64> = Vec::with_capacity(WIDTH * HEIGHT);

            let mut rng = thread_rng();
            for _ in 0..(WIDTH * HEIGHT) {
                data.push(rng.gen());
            }
            Buffer::new((WIDTH, HEIGHT), data)
        };

        // Write some dataset
        {
            let ds = edit_dataset(&path)?;
            let mut band = ds.rasterband(1)?;
            let (width, height) = ds.raster_size();

            assert_eq!(width, WIDTH);
            assert_eq!(height, HEIGHT);
            assert_eq!(ds.raster_count(), 1);

            band.write((0, 0), (width, height), &data)?;
        }

        // Read data
        {
            let ds = read_dataset(&path)?;
            let band = ds.rasterband(1)?;
            let rdata = band.read_band_as::<f64>()?;

            assert_eq!(rdata.data, data.data);
        }

        Ok(())
    }
}

pub fn write_bin<T: serde::Serialize>(path: &Path, data: &T) -> Result<()> {
    let file = File::create(path)?;
    let buf = std::io::BufWriter::with_capacity(0x100000, file);
    serde_cbor::to_writer(buf, data)?;
    Ok(())
}

pub fn read_bin<T: for<'a> serde::Deserialize<'a>>(path: &Path) -> Result<T> {
    let file = File::open(path)?;
    let file = unsafe { memmap::MmapOptions::new().map(&file)? };
    Ok(serde_cbor::from_slice(file.as_ref())?)
}

use serde::Serialize;
pub fn write_json<T: Serialize>(path: &Path, json: &T) -> Result<()> {
    let file = File::create(path)?;
    let buf = std::io::BufWriter::with_capacity(0x100000, file);
    Ok(serde_json::to_writer(buf, json)?)
}

pub fn print_json<T: Serialize>(json: &T) -> Result<()> {
    let writer = std::io::BufWriter::new(std::io::stdout());
    Ok(serde_json::to_writer(writer, json)?)
}
