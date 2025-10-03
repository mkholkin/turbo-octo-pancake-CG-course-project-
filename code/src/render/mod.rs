pub mod transparency;
pub mod z_buffer;
pub mod wireframe_drawer;

use crate::config::{AMBIENT_INTENSITY, LIGHT_SCATTERING};
use crate::objects::light::LightSource;
use crate::objects::model3d::Material;
use crate::scene::Scene;
use image::{Rgb, RgbImage};
use nalgebra::{Point3, Vector3};

fn compute_reflection(
    light_direction: &Vector3<f64>,
    surface_normal: &Vector3<f64>,
) -> Vector3<f64> {
    let beta = 2. * light_direction.dot(surface_normal);
    (-1. * light_direction) + (beta * surface_normal)
}

fn calculate_color(
    material: &Material,
    normal: &Vector3<f64>,
    surface_point: &Point3<f64>,
    light_source: &LightSource,
    eye_pos: &Point3<f64>,
) -> Rgb<u8> {
    // let normal = Vector3::new(0., 0., 1.);
    // let surface_point = Point3::new(0., 0., 0.);
    let mut light_direction = light_source.pos - surface_point;
    let dist = light_direction.norm();

    light_direction.normalize_mut();
    let view_direction = (eye_pos - surface_point).normalize();

    let reflection_direction = compute_reflection(&light_direction, &normal);

    let light_intensity = light_source.intensity / (dist + LIGHT_SCATTERING as f64);

    let diffuse_intensity = material.diffuse_reflectance_factor
        * light_intensity
        * normal.dot(&light_direction).max(0.)
        + AMBIENT_INTENSITY as f64;
    let specular_intensity = material.specular_reflectance_factor
        * light_intensity
        * reflection_direction
            .dot(&view_direction)
            .max(0.)
            .powf(material.gloss);

    let r = (material.color[0] as f64 * diffuse_intensity
        + light_source.color[0] as f64 * specular_intensity)
        .clamp(0., 255.);
    let g = (material.color[1] as f64 * diffuse_intensity
        + light_source.color[1] as f64 * specular_intensity)
        .clamp(0., 255.);
    let b = (material.color[2] as f64 * diffuse_intensity
        + light_source.color[2] as f64 * specular_intensity)
        .clamp(0., 255.);

    Rgb([r.round() as u8, g.round() as u8, b.round() as u8])
}

pub trait Renderer {
    fn create_frame(&mut self, width: u32, height: u32, scene: &Scene) -> RgbImage {
        let mut image = RgbImage::new(width, height);
        self.create_frame_mut(&mut image, scene);
        image
    }
    fn create_frame_mut(&mut self, image: &mut RgbImage, scene: &Scene);
}
