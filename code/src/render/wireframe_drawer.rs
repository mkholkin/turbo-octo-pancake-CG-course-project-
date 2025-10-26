use crate::objects::camera::Camera;
use crate::objects::model3d::Model3D;
use crate::render::Renderer;
use crate::scene::Scene;
use image::RgbImage;
use imageproc::drawing::{draw_filled_rect_mut, draw_hollow_polygon_mut};
use imageproc::point::Point;
use imageproc::rect::Rect;
use nalgebra::{Matrix4, Point3};

pub struct WireframePerformer;
impl WireframePerformer {
    fn draw_object(
        image: &mut RgbImage,
        camera: &Camera,
        model: &dyn Model3D,
    ) {
        let (width, height) = image.dimensions();
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

        // Calculate the MVPV matrix once
        let mvp_matrix = camera.camera_matrix;
        let mvpv_matrix = viewport_matrix * mvp_matrix;

        // Transform the world-space vertices once
        let camera_dim_v: Vec<Point3<f64>> = model
            .vertices_world()
            .iter()
            .map(|v| {
                Point3::from_homogeneous(mvpv_matrix * v.to_homogeneous())
                    .expect("Perspective division failed.")
            })
            .collect();

        for tri in model.triangles() {
            let v0 = camera_dim_v[tri.0];
            let v1 = camera_dim_v[tri.1];
            let v2 = camera_dim_v[tri.2];

            draw_hollow_polygon_mut(
                image,
                &[
                    Point::new(v0.x as f32, v0.y as f32),
                    Point::new(v1.x as f32, v1.y as f32),
                    Point::new(v2.x as f32, v2.y as f32),
                ],
                model.material().color,
            );
        }

        for v in &camera_dim_v {
            draw_filled_rect_mut(
                image,
                Rect::at(v.x.round() as i32 - 2, v.y.round() as i32 - 2).of_size(4, 4),
                model.material().color,
            );
        }
    }
}
//
// impl Renderer for WireframePerformer {
//     fn create_frame_mut(&mut self, image: &mut RgbImage, scene: &Scene) {
//         image.fill(255);
//         for model in &scene.objects {
//             WireframePerformer::draw_object(image, &scene.camera, &**model);
//         }
//     }
// }
