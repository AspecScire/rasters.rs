use cgmath::{ Point2, Matrix3, Vector3 };
pub type Loc = Point2<f64>;
pub type CoordTransform = Matrix3<f64>;

pub fn transform_from_gdal(t: &[f64]) -> CoordTransform {
    Matrix3 {
        x: Vector3::new(t[1], t[4], 0.),
        y: Vector3::new(t[2], t[5], 0.),
        z: Vector3::new(t[0], t[3], 1.),
    }
}
