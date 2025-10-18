use nalgebra::{Point3, Vector3};

pub fn barycentric(
    p: &Point3<f64>,
    a: &Point3<f64>,
    b: &Point3<f64>,
    c: &Point3<f64>,
) -> Vector3<f64> {
    let v0 = *b - *a;
    let v1 = *c - *a;
    let v2 = *p - *a;

    // Calculate dot products
    let dot00 = v0.dot(&v0);
    let dot01 = v0.dot(&v1);
    let dot02 = v0.dot(&v2);
    let dot11 = v1.dot(&v1);
    let dot12 = v1.dot(&v2);

    // Calculate the inverse of the determinant.
    // if the triangle is degenerate (i.e., its vertices are collinear).
    let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);

    // Calculate the coordinates u and v.
    let v = (dot11 * dot02 - dot01 * dot12) * inv_denom;
    let w = (dot00 * dot12 - dot01 * dot02) * inv_denom;

    // The third coordinate w is derived from the fact that u + v + w = 1.
    let u = 1.0 - v - w;

    Vector3::new(u, v, w)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let a = Point3::new(1.0, 2.0, 0.);
        let b = Point3::new(4.0, 5.0, 0.);
        let c = Point3::new(0., 0., 0.);
        let p = c.clone();

        println!("{}", barycentric(&p, &a, &b, &c));
    }
}