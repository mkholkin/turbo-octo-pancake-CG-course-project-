use crate::objects::Point;
use image::Rgb;

pub struct LightSource {
    pub pos: Point,
    pub intensity: f32,
    pub color: Rgb<u8>,
}
