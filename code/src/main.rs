mod config;
mod objects;
mod render;
mod scene;
mod utils;

use crate::objects::camera::Camera;
use crate::objects::triangle_mesh::TriangleMesh;
use std::time::Instant;

use crate::config::{
    ASPECT_RATIO, BACKGROUND_COLOR, FAR_PLANE, FOV_DEGREES, NEAR_PLANE,
    ROTATION_SENSITIVITY_FACTOR, SCALING_SENSITIVITY_FACTOR,
};
use crate::objects::light::LightSource;
use crate::render::Renderer;
use crate::render::z_buffer::ZBufferPerformer;
use crate::scene::Scene;
use eframe::egui::{CentralPanel, Context, TextureHandle};
use eframe::{App, Frame, NativeOptions};
use egui::{InputState, Key};
use image::{Rgb, RgbImage};
use imageproc::definitions::HasWhite;
use nalgebra::{Point3, Vector3};
use crate::objects::model3d::{Rotate, Scale};
use crate::objects::morph::Morph;
use crate::render::transparency::TransparencyPerformer;

const IMG_WIDTH: u32 = 1000;
const IMG_HEIGHT: u32 = 1000;

struct MyEguiApp {
    texture: Option<TextureHandle>,
    frame: RgbImage,
    scene: Scene,
    renderer: Box<dyn Renderer>,

    fps: f32,
    last_frame_time: Instant,
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
            intensity: 10.,
            color: Rgb::white(),
        };
        // let mut object = Box::new(TriangleMesh::from_obj("data/Banana.obj").unwrap());

        let a = TriangleMesh::from_obj("data/cube.obj").unwrap();
        let mut b = TriangleMesh::from_obj("data/fixed_sphere.obj").unwrap();
        b.material.color = Rgb([7, 149, 210]);
        b.material.specular_reflectance_factor = 0.7;

        let object = Box::new(Morph::new(
            a,
            b,
        ));

        let scene = Scene {
            camera,
            light_source,
            object,
        };

        Self {
            texture: None,
            frame: RgbImage::from_pixel(IMG_WIDTH, IMG_HEIGHT, BACKGROUND_COLOR),
            scene,
            renderer: Box::new(ZBufferPerformer::new(IMG_WIDTH, IMG_HEIGHT)),
            // renderer: Box::new(TransparencyPerformer {}),
            fps: 0.0,
            last_frame_time: Instant::now(),
        }
    }
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

    unsafe fn update_morph_phase(&mut self, ctx: &Context) {
        static mut T: f32 = 0.;
        let t_step = 0.05;

        ctx.input(|i: &InputState| unsafe {
            if i.key_pressed(Key::Plus) {
                T = (T + t_step).min(1.);
            }
            if i.key_pressed(Key::Minus) {
                T = (T - t_step).max(0.);
            }
        });

        self.scene.object.update(T);
        self.update_frame(ctx);
    }

    fn update_fps(&mut self) {
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        self.fps = 1.0 / frame_time;
    }
}

impl App for MyEguiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.update_fps();
        self.mouse_wheel_scaling(ctx);
        self.mouse_drag_rotation(ctx);
        unsafe { self.update_morph_phase(ctx); }


        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Анимация в egui");
            ui.label(format!("FPS: {}", self.fps as u32));

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
