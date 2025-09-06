mod config;
mod dcel;
mod objects;
mod render;
mod scene;
mod utils;

use crate::objects::camera::Camera;
use crate::objects::model3d::{Material, Scale};
use crate::objects::triangle_mesh::TriangleMesh;

use crate::config::{
    AMBIENT_INTENSITY, ASPECT_RATIO, BACKGROUND_COLOR, FAR_PLANE, FOV_DEGREES, LIGHT_SCATTERING,
    NEAR_PLANE, ROTATION_SENSITIVITY_FACTOR, SCALING_SENSITIVITY_FACTOR,
};
use crate::objects::light::LightSource;
use crate::objects::model3d::{Model3D, Rotate};
use crate::utils::triangles::barycentric;
use eframe::egui::{CentralPanel, Context, TextureHandle};
use eframe::{App, Frame, NativeOptions};
use egui::Scene;
use image::{GenericImage, Rgb, RgbImage};
use imageproc::definitions::HasWhite;
use nalgebra::{Matrix4, Point3, Vector3, Vector4};

const IMG_WIDTH: u32 = 1000;
const IMG_HEIGHT: u32 = 1000;

struct MyEguiApp {
    texture: Option<TextureHandle>,
    frame_counter: u32,
    frame: RgbImage,
    camera: Camera,
    object: TriangleMesh,
    light_source: LightSource,
    renderer: ZBufferPerformer,
    // renderer: TransparencyPerformer,
}

impl Default for MyEguiApp {
    fn default() -> Self {
        let camera = Camera::new(
            Point3::new(0., 0., 3.),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            FOV_DEGREES.to_radians(),
            ASPECT_RATIO,
            NEAR_PLANE,
            FAR_PLANE,
        );
        let light_source = LightSource {
            pos: Point3::new(0., 0., 3.),
            intensity: 15.,
            color: Rgb::white(),
        };
        let object = TriangleMesh::from_obj("data/Banana.obj").unwrap();
        Self {
            texture: None,
            frame_counter: 0,
            frame: RgbImage::from_pixel(IMG_WIDTH, IMG_HEIGHT, BACKGROUND_COLOR),
            camera,
            object,
            light_source,
            // renderer: TransparencyPerformer {},
            renderer: ZBufferPerformer::new(IMG_WIDTH, IMG_HEIGHT),
        }
    }
}

#[derive(Default)]
struct ZBufferPerformer {
    width: u32,
    height: u32,
    z_buffer: Vec<f32>,
}

impl ZBufferPerformer {
    fn new(width: u32, height: u32) -> Self {
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
        let max_x = max_x.min(IMG_WIDTH - 1);
        let min_y = min_y.max(0);
        let max_y = max_y.min(IMG_HEIGHT - 1);

        for y in min_y..=max_y.min(IMG_HEIGHT - 1) {
            for x in min_x..=max_x.min(IMG_WIDTH - 1) {
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

    pub fn create_frame<'a>(
        &mut self,
        image: &'a mut RgbImage,
        obj: &'a impl Model3D<'a>,
        camera: &'a Camera,
        light_source: &'a LightSource,
    ) {
        let width = image.width();
        let height = image.height();

        self.reset(width, height);
        image.fill(70);

        // TODO: organize this transformations
        let mvp_matrix = camera.camera_matrix * obj.model_matrix();
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
        let camera_dim_v: Vec<Point3<f32>> = obj
            .vertices()
            .iter()
            .map(|v| {
                Point3::from_homogeneous(mvpv_matrix * v.to_homogeneous())
                    .expect("Perspective division failed.")
            })
            .collect();

        for (i, tri) in obj.trigons().iter().enumerate() {
            let color = calculate_color(
                &obj.material(),
                &obj.normals()[i],
                &obj.vertices_world()[tri.0],
                &light_source,
                &camera.pos,
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

struct TransparencyPerformer {}

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
        let max_x = max_x.min(IMG_WIDTH - 1);
        let min_y = min_y.max(0);
        let max_y = max_y.min(IMG_HEIGHT - 1);

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

    pub fn create_frame<'a>(
        &mut self,
        image: &'a mut RgbImage,
        obj: &'a impl Model3D<'a>,
        camera: &'a Camera,
        light_source: &'a LightSource,
    ) {
        let width = image.width();
        let height = image.height();

        image.fill(70);

        // TODO: organize this transformations
        let mvp_matrix = camera.camera_matrix * obj.model_matrix();
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
        let camera_dim_v: Vec<Point3<f32>> = obj
            .vertices()
            .iter()
            .map(|v| {
                Point3::from_homogeneous(mvpv_matrix * v.to_homogeneous())
                    .expect("Perspective division failed.")
            })
            .collect();

        for (i, tri) in obj.trigons().iter().enumerate() {
            let surface_point = &obj.vertices_world()[tri.0];
            let normal =
                if obj.normals()[i].dot(&(light_source.pos - surface_point).to_homogeneous()) > 0.0
                {
                    obj.normals()[i]
                } else {
                    obj.normals()[i] * -1.
                };

            let color = calculate_color(
                &obj.material(),
                &normal,
                surface_point,
                &light_source,
                &camera.pos,
            );

            self.draw_triangle(
                image,
                &[
                    camera_dim_v[tri.0],
                    camera_dim_v[tri.1],
                    camera_dim_v[tri.2],
                ],
                Rgb([color[0], color[1], color[2]]),
                obj.material().opacity,
            )
        }
    }
}

fn compute_reflection(
    light_direction: &Vector4<f32>,
    surface_normal: &Vector4<f32>,
) -> Vector4<f32> {
    let beta = 2. * light_direction.dot(surface_normal);
    (-1. * light_direction) + (beta * surface_normal)
}

fn calculate_color(
    material: &Material,
    normal: &Vector4<f32>,
    surface_point: &Point3<f32>,
    light_source: &LightSource,
    eye_pos: &Point3<f32>,
) -> Rgb<u8> {
    let light_direction = light_source.pos - surface_point;
    let dist = light_direction.norm();

    let light_direction = light_direction.normalize().to_homogeneous();
    let view_direction = (eye_pos - surface_point).normalize().to_homogeneous();

    let reflection_direction = compute_reflection(&light_direction, &normal);

    let light_intensity = light_source.intensity / (dist + LIGHT_SCATTERING);

    let diffuse_intensity = material.diffuse_reflectance_factor
        * light_intensity
        * normal.dot(&light_direction).max(0.)
        + AMBIENT_INTENSITY;
    let specular_intensity = material.specular_reflectance_factor
        * light_intensity
        * reflection_direction
            .dot(&view_direction)
            .max(0.)
            .powf(material.gloss);

    let r = (material.color[0] as f32 * diffuse_intensity
        + light_source.color[0] as f32 * specular_intensity)
        .clamp(0., 255.);
    let g = (material.color[1] as f32 * diffuse_intensity
        + light_source.color[1] as f32 * specular_intensity)
        .clamp(0., 255.);
    let b = (material.color[2] as f32 * diffuse_intensity
        + light_source.color[2] as f32 * specular_intensity)
        .clamp(0., 255.);

    Rgb([r.round() as u8, g.round() as u8, b.round() as u8])
}

impl MyEguiApp {
    fn update_frame(&mut self, ctx: &Context) {
        self.renderer.create_frame(
            &mut self.frame,
            &self.object,
            &self.camera,
            &self.light_source,
        );

        let egui_image = egui::ColorImage::from_rgb(
            [self.frame.width() as usize, self.frame.height() as usize],
            self.frame.as_raw(),
        );

        if self.texture.is_none() {
            self.texture = Some(ctx.load_texture("rendered_image", egui_image, Default::default()));
        } else {
            self.texture
                .as_mut()
                .unwrap()
                .set(egui_image, Default::default());
        }
    }

    fn mouse_wheel_scaling(&mut self, ctx: &Context) {
        let scroll_delta = ctx.input(|i| i.raw_scroll_delta);
        if scroll_delta.y != 0.0 {
            let scaling_factor =
                (1. + scroll_delta.y.max(-200.) * SCALING_SENSITIVITY_FACTOR).max(f32::EPSILON);
            self.object.scale(scaling_factor);
            self.update_frame(ctx);
        }
    }

    fn mouse_drag_rotation(&mut self, ctx: &Context) {
        if ctx.input(|i| i.pointer.is_decidedly_dragging()) {
            let delta = ctx.input(|i| i.pointer.delta());

            // Create a new rotation based on mouse delta
            let rotation_x = delta.y * ROTATION_SENSITIVITY_FACTOR;
            let rotation_y = delta.x * ROTATION_SENSITIVITY_FACTOR;

            // Apply the new rotation to the existing one
            self.object
                .rotate((rotation_x.to_radians(), rotation_y.to_radians(), 0.));
            self.update_frame(ctx);
        }
    }
}

impl App for MyEguiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Увеличиваем счетчик кадров для анимации
        self.frame_counter += 1;

        self.mouse_wheel_scaling(ctx);
        self.mouse_drag_rotation(ctx);

        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Анимация в egui");
            ui.label(format!("Текущий кадр: {}", self.frame_counter));

            // 6. Отображаем изображение с помощью обработчика текстуры
            if let Some(texture) = &self.texture {
                let size = texture.size_vec2();
                ui.image((texture.id(), size));
            }
        });

        // 7. Просим egui перерисовать экран, чтобы получить плавную анимацию
        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    let app = MyEguiApp::default();
    let native_options = NativeOptions::default();
    eframe::run_native(
        "Моё egui приложение",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
