use crate::render::Renderer;
use crate::render::calculate_color;
use crate::scene::Scene;
use crate::utils::triangles::barycentric;
use image::{Rgb, RgbImage};
use nalgebra::{Matrix4, Point3};

pub struct TransparencyPerformer {}

impl TransparencyPerformer {
    fn draw_triangle(
        &mut self,
        image: &mut RgbImage,
        tri: &[Point3<f32>; 3],
        color: Rgb<u8>,
        alpha: f32,
    ) {
        let [p1, p2, p3] = *tri;

        // Find the bounding box of the triangle to optimize rasterization.
        let min_x = p1.x.min(p2.x).min(p3.x).round() as u32;
        let max_x = p1.x.max(p2.x).max(p3.x).round() as u32;
        let min_y = p1.y.min(p2.y).min(p3.y).round() as u32;
        let max_y = p1.y.max(p2.y).max(p3.y).round() as u32;

        // Clamp bounding box to image boundaries.
        let min_x = min_x.max(0);
        let max_x = max_x.min(image.width() - 1);
        let min_y = min_y.max(0);
        let max_y = max_y.min(image.height() - 1);

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let bary = barycentric(&Point3::new(x as f32, y as f32, 0.), &p1, &p2, &p3);

                // Check if the pixel is inside the triangle.
                if bary.x >= 0.0 && bary.y >= 0.0 && bary.z >= 0.0 {
                    let old_pixel = image.get_pixel(x, y);
                    let final_r = (color[0] as f32 * alpha) + (old_pixel[0] as f32 * (1.0 - alpha));
                    let final_g = (color[1] as f32 * alpha) + (old_pixel[1] as f32 * (1.0 - alpha));
                    let final_b = (color[2] as f32 * alpha) + (old_pixel[2] as f32 * (1.0 - alpha));

                    image.put_pixel(
                        x,
                        y,
                        Rgb([
                            final_r.round() as u8,
                            final_g.round() as u8,
                            final_b.round() as u8,
                        ]),
                    );
                }
            }
        }
    }
}

impl Renderer for TransparencyPerformer {
    fn create_frame_mut(&mut self, image: &mut RgbImage, scene: &Scene) {
        let (width, height) = image.dimensions();

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
            let surface_point = &scene.object.vertices_world()[tri.0];
            let normal = if scene.object.normals()[i]
                .dot(&(scene.light_source.pos - surface_point).to_homogeneous())
                > 0.0
            {
                scene.object.normals()[i]
            } else {
                scene.object.normals()[i] * -1.
            };

            let color = calculate_color(
                &scene.object.material(),
                &normal,
                surface_point,
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
                Rgb([color[0], color[1], color[2]]),
                scene.object.material().opacity,
            )
        }
    }
}
