use crate::objects::light::LightSource;
use crate::objects::model3d::Material;
use image::Rgb;
use nalgebra::{Point3, Vector3};

fn color(
    light_source: &LightSource,
    surface_normal: &Vector3<f32>,
    surface_point: Point3<f32>,
    material: &Material,
) -> Rgb<u8> {
    todo!()
}
