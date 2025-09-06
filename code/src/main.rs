mod config;
mod dcel;
mod objects;
mod render;
mod scene;
mod utils;

use crate::objects::camera::Camera;
use crate::objects::model3d::{Material};
use crate::objects::triangle_mesh::TriangleMesh;

use crate::config::{
    AMBIENT_INTENSITY, ASPECT_RATIO, BACKGROUND_COLOR, FAR_PLANE, FOV_DEGREES, LIGHT_SCATTERING,
    NEAR_PLANE, ROTATION_SENSITIVITY_FACTOR, SCALING_SENSITIVITY_FACTOR,
};
use crate::objects::light::LightSource;
use crate::render::Renderer;
use crate::render::z_buffer::ZBufferPerformer;
use crate::scene::Scene;
use eframe::egui::{CentralPanel, Context, TextureHandle};
use eframe::{App, Frame, NativeOptions};
use image::{Rgb, RgbImage};
use imageproc::definitions::HasWhite;
use nalgebra::{Point3, Vector3, Vector4};

const IMG_WIDTH: u32 = 1000;
const IMG_HEIGHT: u32 = 1000;

struct MyEguiApp {
    texture: Option<TextureHandle>,
    frame_counter: u32,
    frame: RgbImage,
    scene: Scene,
    renderer: Box<dyn Renderer>,
}

impl<'a> Default for MyEguiApp {
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
        let object = Box::new(TriangleMesh::from_obj("data/Banana.obj").unwrap());

        let scene = Scene {
            camera,
            light_source,
            object,
        };

        Self {
            texture: None,
            frame_counter: 0,
            frame: RgbImage::from_pixel(IMG_WIDTH, IMG_HEIGHT, BACKGROUND_COLOR),
            scene,
            renderer: Box::new(ZBufferPerformer::new(IMG_WIDTH, IMG_HEIGHT)),
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
        self.renderer.create_frame_mut(&mut self.frame, &self.scene);

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
            self.scene.object.scale(scaling_factor);
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
            self.scene
                .object
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
