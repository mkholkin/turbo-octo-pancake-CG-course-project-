use crate::objects::Point;
use image::Rgb;

#[derive(Clone)]
pub struct LightSource {
    pub pos: Point,
    pub intensity: f64,
    pub color: Rgb<u8>,
}
