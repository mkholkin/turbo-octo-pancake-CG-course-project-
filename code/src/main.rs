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
use crate::objects::model3d::InteractiveModel;
use crate::objects::morph::Morph;
use crate::render::Renderer;
use crate::render::transparency::TransparencyPerformer;
use crate::render::wireframe_drawer::WireframePerformer;
use crate::render::z_buffer::ZBufferPerformer;
use crate::scene::Scene;
use eframe::egui::{CentralPanel, Context, TextureHandle};
use eframe::{App, Frame, NativeOptions};
use egui::{InputState, Key};
use image::{Rgb, RgbImage};
use imageproc::definitions::{HasBlack, HasWhite};
use nalgebra::{Point3, Vector3};

const IMG_WIDTH: u32 = 2000;
const IMG_HEIGHT: u32 = 2000;

struct MyEguiApp {
    texture: Option<TextureHandle>,
    frame: RgbImage,
    scene: Scene,
    renderer: Box<dyn Renderer>,

    fps: f64,
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
            NEAR_PLANE.into(),
            FAR_PLANE,
        );
        let light_source = LightSource {
            pos: Point3::new(0., 0., 3.),
            intensity: 10.,
            color: Rgb::white(),
        };

        // let mut a = Box::new(TriangleMesh::from_obj("data/cherry_single.obj").unwrap());
        // parametrize_mesh(&mut a);

        // let mut b = Box::new(TriangleMesh::from_obj("data/apple.obj").unwrap());
        // b.material.color = Rgb::black();
        // parametrize_mesh(&mut b);
        //
        // let c = Box::new(TriangleMesh::from(create_dcel_map(&a, &b)));

        // let objects: Vec<Box<dyn InteractiveModel>> = vec![a];

        let mut a = TriangleMesh::from_obj("data/apple2.obj").unwrap();
        a.material.diffuse_reflectance_factor = 0.5;
        let mut b = TriangleMesh::from_obj("data/pear.obj").unwrap();
        b.material.color = Rgb([206, 208, 51]);
        b.material.specular_reflectance_factor = 0.0;
        a.material.specular_reflectance_factor = 0.0;

        let morph = Box::new(Morph::new(
            a,
            b,
        ));

        let objects: Vec<Box<dyn InteractiveModel>> = vec![morph];

        let scene = Scene {
            camera,
            light_source,
            active_object_idx: 0,
            objects,
        };

        Self {
            texture: None,
            frame: RgbImage::from_pixel(IMG_WIDTH, IMG_HEIGHT, BACKGROUND_COLOR),
            scene,
            renderer: Box::new(ZBufferPerformer::new(IMG_WIDTH, IMG_HEIGHT)),
            // renderer: Box::new(TransparencyPerformer {}),
            // renderer: Box::new(WireframePerformer {}),
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
            for object in self.scene.objects.iter_mut() {
                object.scale(scaling_factor.into());
            }
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
            for object in self.scene.objects.iter_mut() {
                object.rotate((
                    rotation_x.to_radians().into(),
                    rotation_y.to_radians().into(),
                    0.,
                ));
            }
            self.update_frame(ctx);
        }
    }

    unsafe fn update_morph_phase(&mut self, ctx: &Context) {
        static mut T: f64 = 0.;
        let t_step = 0.05;

        ctx.input(|i: &InputState| unsafe {
            if i.key_pressed(Key::Plus) {
                T = (T + t_step).min(1.);
            }
            if i.key_pressed(Key::Minus) {
                T = (T - t_step).max(0.);
            }
        });

        self.scene.objects[self.scene.active_object_idx].update(T);
        self.update_frame(ctx);
    }

    fn update_fps(&mut self) {
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame_time).as_secs_f64();
        self.last_frame_time = now;
        self.fps = 1.0 / frame_time;
    }
}

impl App for MyEguiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        ctx.set_pixels_per_point(1.);
        self.update_fps();
        self.mouse_wheel_scaling(ctx);
        self.mouse_drag_rotation(ctx);
        unsafe {
            self.update_morph_phase(ctx);
        }

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
