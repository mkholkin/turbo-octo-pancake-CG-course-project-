use crate::objects::Point;
use image::Rgb;

pub struct LightSource {
    pub pos: Point,
    pub intensity: f64,
    pub color: Rgb<u8>,
}
