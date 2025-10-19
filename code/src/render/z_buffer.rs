use crate::config::BACKGROUND_COLOR;
use crate::objects::camera::Camera;
use crate::objects::light::LightSource;
use crate::objects::model3d::{InteractiveModel, Model3D};
use crate::render::{Renderer, calculate_color};
use crate::scene::Scene;
use image::{Rgb, RgbImage};
use nalgebra::{Matrix4, Point3};

#[derive(Default)]
pub struct ZBufferPerformer {
    width: u32,
    height: u32,
    z_buffer: Vec<f64>,
}

impl ZBufferPerformer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            z_buffer: vec![f64::INFINITY; (width * height) as usize],
        }
    }

    fn reset(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.z_buffer
            .resize((width * height) as usize, f64::INFINITY);
        self.z_buffer.fill(f64::INFINITY);
    }

    /// Sets the depth value at a specific coordinate.
    fn set_depth(&mut self, x: u32, y: u32, depth: f64) {
        let index = (y * self.width + x) as usize;
        self.z_buffer[index] = depth;
    }

    /// Gets the depth value at a specific coordinate.
    fn get_depth(&self, x: u32, y: u32) -> f64 {
        let index = (y * self.width + x) as usize;
        self.z_buffer[index]
    }

    fn draw_triangle(
        &mut self,
        image: &mut RgbImage,
        tri: &[Point3<f64>; 3],
        tri_colors: &[Rgb<u8>; 3],
    ) {
        let [p1, p2, p3] = *tri;

        // Find the bounding box, clamping to the image dimensions.
        let min_x = (p1.x.min(p2.x).min(p3.x).round() as u32).max(0);
        let max_x = (p1.x.max(p2.x).max(p3.x).round() as u32).min(self.width - 1);
        let min_y = (p1.y.min(p2.y).min(p3.y).round() as u32).max(0);
        let max_y = (p1.y.max(p2.y).max(p3.y).round() as u32).min(self.height - 1);

        // Pre-calculate common components to avoid redundant calculations inside the loop.
        let denom = (p2.x - p1.x) * (p3.y - p1.y) - (p2.y - p1.y) * (p3.x - p1.x);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // Calculate barycentric coordinates.
                let u =
                    ((p3.x - p2.x) * (y as f64 - p2.y) - (p3.y - p2.y) * (x as f64 - p2.x)) / denom;
                let v =
                    ((p1.x - p3.x) * (y as f64 - p3.y) - (p1.y - p3.y) * (x as f64 - p3.x)) / denom;

                let bary = Point3::new(u, v, 1.0 - u - v);

                // Check if the pixel is inside the triangle.
                if bary.x > -f64::EPSILON && bary.y > -f64::EPSILON && bary.z > -f64::EPSILON {
                    let z = p1.z * bary.x + p2.z * bary.y + p3.z * bary.z;

                    // Perform the Z-buffer test.
                    if z < self.get_depth(x, y) {
                        self.set_depth(x, y, z);

                        // Interpolate colors correctly for each channel.
                        let r = (bary.x * tri_colors[0].0[0] as f64
                            + bary.y * tri_colors[1].0[0] as f64
                            + bary.z * tri_colors[2].0[0] as f64)
                            .clamp(0.0, 255.0) as u8;
                        let g = (bary.x * tri_colors[0].0[1] as f64
                            + bary.y * tri_colors[1].0[1] as f64
                            + bary.z * tri_colors[2].0[1] as f64)
                            .clamp(0.0, 255.0) as u8;
                        let b = (bary.x * tri_colors[0].0[2] as f64
                            + bary.y * tri_colors[1].0[2] as f64
                            + bary.z * tri_colors[2].0[2] as f64)
                            .clamp(0.0, 255.0) as u8;

                        image.put_pixel(x, y, Rgb([r, g, b]));
                    }
                }
            }
        }
    }

    fn draw_object(
        &mut self,
        image: &mut RgbImage,
        model: &dyn Model3D,
        camera: &Camera,
        light_source: &LightSource,
    ) {
        // Calculate the MVPV matrix once
        let (width, height) = image.dimensions();
        let mvp_matrix = camera.camera_matrix * model.model_matrix();
        let viewport_matrix = Matrix4::new(
            width as f64 / 2.,
            0.,
            0.,
            width as f64 / 2.,
            0.,
            -(height as f64 / 2.),
            0.,
            height as f64 / 2.,
            0.,
            0.,
            1.,
            0.,
            0.,
            0.,
            0.,
            1.,
        );
        let mvpv_matrix = viewport_matrix * mvp_matrix;

        // Transform the world-space vertices once
        let camera_dim_v: Vec<Point3<f64>> = model
            .vertices()
            .iter()
            .map(|v| {
                Point3::from_homogeneous(mvpv_matrix * v.to_homogeneous())
                    .expect("Perspective division failed.")
            })
            .collect();

        for (i, tri) in model.triangles().iter().enumerate() {
            let tri_colors = [tri.0, tri.1, tri.2].map(|v_idx| {
                calculate_color(
                    &model.material(),
                    &model.normals()[i].xyz(),
                    &model.vertices_world()[v_idx],
                    &light_source,
                    &camera.pos,
                )
            });

            self.draw_triangle(
                image,
                &[
                    camera_dim_v[tri.0],
                    camera_dim_v[tri.1],
                    camera_dim_v[tri.2],
                ],
                &tri_colors,
            );
        }
    }
}
impl Renderer for ZBufferPerformer {
    fn create_frame_mut(&mut self, image: &mut RgbImage, scene: &Scene) {
        let (width, height) = image.dimensions();
        self.reset(width, height);
        image.pixels_mut().for_each(|px| *px = BACKGROUND_COLOR);

        for object in &scene.objects {
            self.draw_object(image, &**object, &scene.camera, &scene.light_source);
        }
    }

    fn render_single_object(
        &mut self,
        image: &mut RgbImage,
        object: &dyn InteractiveModel,
        camera: &Camera,
        light: &LightSource,
    ) {
        let (width, height) = image.dimensions();
        self.reset(width, height);
        image.pixels_mut().for_each(|px| *px = BACKGROUND_COLOR);
        self.draw_object(image, object, camera, light);
    }
}
