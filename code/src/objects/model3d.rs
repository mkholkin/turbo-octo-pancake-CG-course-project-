use crate::objects::Point;
use crate::utils::math::lerp;
use image::Rgb;
use nalgebra::{Matrix4, Vector4};

pub type Triangle = (usize, usize, usize);

pub trait Model3D {
    /// List of triangle faces
    fn triangles(&self) -> &Vec<Triangle>;
    // fn edges(&'a self) -> &'a Vec<>;

    /// List of normalized external normals
    fn normals(&self) -> &Vec<Vector4<f64>>;

    /// List of vertices
    fn vertices(&self) -> &Vec<Point>;

    /// List of vertices multiplied by transformation matrix
    fn vertices_world(&self) -> &Vec<Point>;

    /// Return material
    fn material(&self) -> &Material;

    /// Return true if external normals were calculated otherwise - false
    fn has_normals(&self) -> bool;

    /// Calculate external normals
    fn compute_normals(&mut self);

    /// Get model's transformation matrix
    fn model_matrix(&self) -> &Matrix4<f64>;

    fn update(&mut self, t: f64) {}
}

pub trait Translate {
    fn translate(&mut self, translation: (f64, f64, f64));
}

pub trait Rotate {
    fn rotate(&mut self, axis_angle_radians: (f64, f64, f64));
}

pub trait Scale {
    fn scale(&mut self, scaling: f64);
}

pub trait InteractiveModel: Model3D + Rotate + Scale {}

#[derive(Clone)]
pub struct Material {
    pub diffuse_reflectance_factor: f64,
    pub specular_reflectance_factor: f64,
    pub gloss: f64,
    pub color: Rgb<u8>,
    pub opacity: f64,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            diffuse_reflectance_factor: 0.45,
            specular_reflectance_factor: 0.02,
            gloss: 1.,
            color: Rgb([208, 43, 43]),
            opacity: 0.1,
        }
    }
}

impl Material {
    pub fn lerp(a: &Material, b: &Material, t: f64) -> Material {
        let diffuse_reflectance_factor = lerp(
            a.diffuse_reflectance_factor,
            b.diffuse_reflectance_factor,
            t,
        );
        let specular_reflectance_factor = lerp(
            a.specular_reflectance_factor,
            b.specular_reflectance_factor,
            t,
        );
        let gloss = lerp(a.gloss, b.gloss, t);
        let opacity = lerp(a.opacity, b.opacity, t);

        let r = lerp(a.color[0] as f64, b.color[0] as f64, t).round() as u8;
        let g = lerp(a.color[1] as f64, b.color[1] as f64, t).round() as u8;
        let b = lerp(a.color[2] as f64, b.color[2] as f64, t).round() as u8;
        let color = Rgb([r, g, b]);

        Material {
            diffuse_reflectance_factor,
            specular_reflectance_factor,
            gloss,
            color,
            opacity,
        }
    }
}
