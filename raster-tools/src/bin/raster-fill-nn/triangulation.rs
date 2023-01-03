use anyhow::{anyhow, bail};
use raster_tools::{utils::*, *};
use gdal::vector::LayerAccess;

#[derive(Clone)]
pub struct PointWithHeight {
    pub point: [f64; 2],
    pub gradient: [f64; 2],
    pub height: f64,
}

impl PointWithHeight {
    pub fn new(x: f64, y: f64, height: f64) -> Self {
        PointWithHeight {
            point: [x, y],
            height,
            gradient: [0., 0.],
        }
    }
}

impl HasPosition for PointWithHeight {
    type Point = [f64; 2];
    fn position(&self) -> [f64; 2] {
        self.point
    }
}

use spade::{delaunay::*, kernels::*, *};

type Triangles = DelaunayTriangulation<PointWithHeight, FloatKernel>;
pub fn get_triangles(args: &super::Args) -> Result<Triangles> {
    use std::time::*;
    let start = Instant::now();
    let ds = read_dataset(&args.source)?;
    let pts = get_points(ds, &args.prop_name)?;
    let triangles = get_triangulation(pts.clone());
    if triangles.num_triangles() < 1 {
        bail!("triangulation failed");
    }
    eprintln!(
        "Triangulation completed in {:.2} secs. {} vertices, {} faces.",
        start.elapsed().as_secs_f64(),
        triangles.num_vertices(),
        triangles.num_faces()
    );
    Ok(triangles)
}

pub type Triangulation =
    FloatDelaunayTriangulation<PointWithHeight, DelaunayTreeLocate<[f64; 2]>>;
pub fn get_triangulation<I: IntoIterator<Item = PointWithHeight>>(pts: I) -> Triangulation {
    let mut tr = FloatDelaunayTriangulation::with_tree_locate();
    for p in pts {
        tr.insert(p);
    }
    tr.estimate_gradients(&(|v| v.height), &(|v, g| v.gradient = g));
    return tr;
}

pub fn get_points(ds: gdal::Dataset, prop_name: &str) -> Result<Vec<PointWithHeight>> {
    let mut layer = ds.layer(0)?;
    let mut out = vec![];

    #[allow(non_upper_case_globals)]
    for f in layer.features() {
        let geo = f.geometry();
        let geometry_type = geo.geometry_type();

        use gdal_sys::OGRwkbGeometryType::*;
        let (x, y) = match geometry_type {
            wkbPoint | wkbPoint25D | wkbPointM | wkbPointZM => {
                let (x, y, _) = geo.get_point(0);
                (x, y)
            }
            _ => bail!("unknown geometry type: {}", geometry_type),
        };

        use gdal::vector::FieldValue::RealValue;
        let prop_value = f
            .field(prop_name)?
            .ok_or_else(|| anyhow!("field {} was null", prop_name))?;

        let z = match prop_value {
            RealValue(z) => z,
            _ => bail!(
                "unexpected type ({}) of field {}",
                prop_value.ogr_field_type(),
                prop_name
            ),
        };

        out.push(PointWithHeight::new(x, y, z));
    }

    Ok(out)
}
