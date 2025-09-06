use crate::render::{Renderer, calculate_color};
use crate::scene::Scene;
use crate::utils::triangles::barycentric;
use image::{Rgb, RgbImage};
use nalgebra::{Matrix4, Point3};

#[derive(Default)]
pub struct ZBufferPerformer {
    width: u32,
    height: u32,
    z_buffer: Vec<f32>,
}

impl ZBufferPerformer {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            z_buffer: vec![f32::INFINITY; (width * height) as usize],
        }
    }

    fn reset(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        self.z_buffer
            .resize((width * height) as usize, f32::INFINITY);
        self.z_buffer.fill(f32::INFINITY);
    }

    /// Sets the depth value at a specific coordinate.
    fn set_depth(&mut self, x: u32, y: u32, depth: f32) {
        let index = (y * self.width + x) as usize;
        self.z_buffer[index] = depth;
    }

    /// Gets the depth value at a specific coordinate.
    fn get_depth(&self, x: u32, y: u32) -> f32 {
        let index = (y * self.width + x) as usize;
        self.z_buffer[index]
    }

    fn draw_triangle(&mut self, image: &mut RgbImage, tri: &[Point3<f32>; 3], color: Rgb<u8>) {
        let [p1, p2, p3] = *tri;

        // Find the bounding box of the triangle to optimize rasterization.
        let min_x = p1.x.min(p2.x).min(p3.x).round() as u32;
        let max_x = p1.x.max(p2.x).max(p3.x).round() as u32;
        let min_y = p1.y.min(p2.y).min(p3.y).round() as u32;
        let max_y = p1.y.max(p2.y).max(p3.y).round() as u32;

        // Clamp bounding box to image boundaries.
        let min_x = min_x.max(0);
        let max_x = max_x.min(self.width - 1);
        let min_y = min_y.max(0);
        let max_y = max_y.min(self.height - 1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let bary = barycentric(&Point3::new(x as f32, y as f32, 0.), &p1, &p2, &p3);

                // Check if the pixel is inside the triangle.
                if bary.x >= 0.0 && bary.y >= 0.0 && bary.z >= 0.0 {
                    // Interpolate the Z-value using barycentric coordinates.
                    let z = p1.z * bary.x + p2.z * bary.y + p3.z * bary.z;

                    // Perform the Z-buffer test.
                    if z < self.get_depth(x, y) {
                        // If the new pixel is closer, update the Z-buffer and draw the pixel.
                        self.set_depth(x, y, z);
                        image.put_pixel(x, y, color);
                    }
                }
            }
        }
    }
}

impl Renderer for ZBufferPerformer {
    fn create_frame_mut(&mut self, image: &mut RgbImage, scene: &Scene) {
        let (width, height) = image.dimensions();

        self.reset(width, height);
        image.fill(70);

        // TODO: organize this transformations
        let mvp_matrix = scene.camera.camera_matrix * scene.object.model_matrix();
        let viewport_matrix = Matrix4::new(
            width as f32 / 2.,
            0.,
            0.,
            width as f32 / 2.,
            0.,
            -(height as f32 / 2.),
            0.,
            height as f32 / 2.,
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
        let camera_dim_v: Vec<Point3<f32>> = scene
            .object
            .vertices()
            .iter()
            .map(|v| {
                Point3::from_homogeneous(mvpv_matrix * v.to_homogeneous())
                    .expect("Perspective division failed.")
            })
            .collect();

        for (i, tri) in scene.object.trigons().iter().enumerate() {
            let color = calculate_color(
                &scene.object.material(),
                &scene.object.normals()[i],
                &scene.object.vertices_world()[tri.0],
                &scene.light_source,
                &scene.camera.pos,
            );

            self.draw_triangle(
                image,
                &[
                    camera_dim_v[tri.0],
                    camera_dim_v[tri.1],
                    camera_dim_v[tri.2],
                ],
                color,
            )
        }
    }
}
