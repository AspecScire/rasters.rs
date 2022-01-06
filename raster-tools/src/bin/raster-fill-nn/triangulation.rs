use std::convert::TryInto;

use anyhow::bail;
use nalgebra::Point2;
use raster_tools::{utils::*, *};

#[derive(Clone)]
pub struct PointWithHeight {
    pub point: Point2<f64>,
    pub gradient: Point2<f64>,
    pub height: f64,
}

impl PointWithHeight {
    pub fn new(point: Point2<f64>, height: f64) -> Self {
        PointWithHeight {
            point,
            height,
            gradient: Point2::new(0., 0.),
        }
    }
}

impl HasPosition for PointWithHeight {
    type Point = Point2<f64>;
    fn position(&self) -> Point2<f64> {
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
    FloatDelaunayTriangulation<PointWithHeight, DelaunayTreeLocate<Point2<f64>>>;
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
    let mut ignore_count = 0;

    for f in layer.features() {
        let geom = f.geometry().clone().try_into()?;
        use geo::Geometry::Point;
        if let Point(p) = geom {
            use gdal::vector::FieldValue::RealValue;
            if let Some(RealValue(z)) = f.field(prop_name)? {
                out.push(PointWithHeight::new(Point2::new(p.x(), p.y()), z));
                continue;
            }
        }
        ignore_count += 1;
    }
    if ignore_count > 0 {
        eprintln!(
            "WARNING: ignored {} geometries (not wkbPoint or no z prop)",
            ignore_count
        );
    }

    Ok(out)
}
