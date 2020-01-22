pub mod chunks;
pub mod geometry;
pub mod volume;

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

mod tests {
    #[test]
    #[ignore]
    fn rayon_cancellation() {
        use std::time::*;
        use rayon::prelude::*;

        fn task(idx: usize, magic: usize) -> Result<usize, Instant> {
            let duration = Duration::from_millis(100);
            if idx == magic {
                eprintln!("Cancelling.");
                return Err(Instant::now());
            }
            std::thread::sleep(duration);
            // eprintln!("Completed task {}", idx);
            Ok(idx)
        }

        let num_tasks: usize = 800;
        let magic = std::env::var("MAGIC")
            .map(|s| s.parse().expect("can't parse magic"))
            .unwrap_or(2);

        let start = Instant::now();
        let err: Result<usize, Instant> = (0..num_tasks)
            .into_par_iter()
            .map(|i| task(i, magic))
            .try_reduce(|| 0, |a, b| Ok(a+b));

        let elapsed = start.elapsed().as_secs_f64();
        eprintln!("Finished in {:.2}s", elapsed);
        match err {
            Ok(ans) => eprintln!("Completed. Value is {}", ans),
            Err(start) => {
                let elapsed = start.elapsed().as_secs_f64();
                eprintln!("Cancellation took {:.2}s", elapsed);
            }
        }
    }
}
