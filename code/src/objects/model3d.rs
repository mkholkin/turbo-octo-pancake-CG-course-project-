use crate::objects::Point;
use image::Rgb;
use nalgebra::{Matrix4, Vector4};

pub type Triangle = (usize, usize, usize);

pub trait Model3D {
    /// List of triangle faces
    fn triangles(&self) -> &Vec<Triangle>;
    // fn edges(&'a self) -> &'a Vec<>;

    /// List of normalized external normals
    fn normals(&self) -> Vec<Vector4<f32>>;

    /// List of vertices
    fn vertices(&self) -> &Vec<Point>;

    /// List of vertices multiplied by transformation matrix
    fn vertices_world(&self) -> Vec<Point>;

    /// Return material
    fn material(&self) -> &Material;

    /// Return true if external normals were calculated otherwise - false
    fn has_normals(&self) -> bool;

    /// Calculate external normals
    fn compute_normals(&mut self);

    /// Get model's transformation matrix
    fn model_matrix(&self) -> &Matrix4<f32>;
}

pub trait Translate {
    fn translate(&mut self, translation: (f32, f32, f32));
}

pub trait Rotate {
    fn rotate(&mut self, axis_angle_radians: (f32, f32, f32));
}

pub trait Scale {
    fn scale(&mut self, scaling: f32);
}

pub trait InteractiveModel: Model3D + Rotate + Scale {}

pub struct Material {
    pub diffuse_reflectance_factor: f32,
    pub specular_reflectance_factor: f32,
    pub gloss: f32,
    pub color: Rgb<u8>,
    pub opacity: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self {
            diffuse_reflectance_factor: 0.5,
            specular_reflectance_factor: 0.02,
            gloss: 3.,
            color: Rgb([208, 43, 43]),
            opacity: 0.1,
        }
    }
}
