use nalgebra::{Point, Point3, Vector3};

pub fn barycentric(
    p: &Point3<f32>,
    a: &Point3<f32>,
    b: &Point3<f32>,
    c: &Point3<f32>,
) -> Vector3<f32> {
    let denominator = (b.y - c.y) * (a.x - c.x) + (c.x - b.x) * (a.y - c.y);
    let u = ((b.y - c.y) * (p.x - c.x) + (c.x - b.x) * (p.y - c.y)) / denominator;
    let v = ((c.y - a.y) * (p.x - c.x) + (a.x - c.x) * (p.y - c.y)) / denominator;
    let w = 1.0 - u - v;

    Vector3::new(u, v, w)
}
