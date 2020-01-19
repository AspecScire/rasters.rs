use std::path::PathBuf;
pub type InputArgs = PathBuf;
pub struct OutputArgs {
    pub path: PathBuf,
    pub driver: String,
}

use failure::Error;
pub type Result<T> = std::result::Result<T, Error>;

use gdal::raster::Dataset;
pub fn read_dataset(args: &InputArgs) -> Result<Dataset> {
    Ok(Dataset::open(&args)?)
}

pub use gdal::vector::Dataset as VectorDataset;
pub fn read_vector_dataset(args: &InputArgs) -> Result<VectorDataset> {
    Ok(VectorDataset::open(&args)?)
}

pub fn create_output_raster(
    arg: &OutputArgs, ds: &Dataset,  num_bands: isize
) -> Result<Dataset> {
    let out_ds = {
        use gdal::raster::*;
        let driver = Driver::get(&arg.driver)?;
        let (width, height) = ds.size();
        driver.create_with_band_type::<f64>(
            &arg.path.to_string_lossy(),
            width as isize, height as isize, num_bands)?
    };
    out_ds.set_geo_transform(&ds.geo_transform()?)?;
    out_ds.set_projection(&ds.projection())?;
    Ok(out_ds)
}

pub mod chunks;
pub mod geometry;
