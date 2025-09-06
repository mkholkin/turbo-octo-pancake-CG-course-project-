pub mod transparency;
pub mod z_buffer;

use crate::objects::light::LightSource;
use crate::objects::model3d::Material;
use crate::scene::Scene;
use image::{Rgb, RgbImage};
use nalgebra::{Point3, Vector3};

fn color(
    light_source: &LightSource,
    surface_normal: &Vector3<f32>,
    surface_point: Point3<f32>,
    material: &Material,
) -> Rgb<u8> {
    todo!()
}

pub trait Renderer {
    fn create_frame(&mut self, width: u32, height: u32, scene: &Scene) -> RgbImage {
        let mut image = RgbImage::new(width, height);
        self.create_frame_mut(&mut image, scene);
        image
    }
    fn create_frame_mut(&mut self, image: &mut RgbImage, scene: &Scene);
}
