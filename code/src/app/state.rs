use crate::objects::camera::Camera;
use crate::objects::triangle_mesh::TriangleMesh;
use rfd::FileDialog;
use std::path::PathBuf;
use std::time::Instant;

use crate::config::{ASPECT_RATIO, BACKGROUND_COLOR, FAR_PLANE, FOV_DEGREES, NEAR_PLANE};
use crate::objects::light::LightSource;
use crate::objects::model3d::InteractiveModel;
use crate::objects::morph::Morph;
use crate::render::Renderer;
use crate::render::z_buffer::ZBufferPerformer;
use crate::scene::Scene;
use eframe::egui::{Context, TextureHandle};
use image::{Rgb, RgbImage};
use imageproc::definitions::HasWhite;
use nalgebra::{Point3, Vector3};

const IMG_WIDTH: u32 = 2000;
const IMG_HEIGHT: u32 = 2000;

#[derive(Debug, Clone, PartialEq)]
pub enum ViewMode {
    Source,
    Target,
    Morph,
}

pub struct MyEguiApp {
    pub texture: Option<TextureHandle>,
    pub frame: RgbImage,
    pub scene: Scene,
    pub renderer: Box<dyn Renderer>,

    pub fps: f64,
    pub last_frame_time: Instant,

    // UI state
    pub selected_source_file: String,
    pub selected_target_file: String,
    pub view_mode: ViewMode,
    // Флаг: курсор находится над окном просмотра
    pub viewport_has_pointer: bool,

    // Object states
    pub source_mesh: Option<TriangleMesh>,
    pub target_mesh: Option<TriangleMesh>,
    pub morph_object: Option<Morph>,
    pub morph_created: bool,

    // Morph animation state
    pub morph_phase: f64,

    // Error handling
    pub error_message: Option<String>,

    // Флаг необходимости перерисовки
    pub needs_redraw: bool,

    // Текущие размеры viewport
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl Default for MyEguiApp {
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
            intensity: 15.,
            color: Rgb::white(),
        };

        // Создаем пустую сцену изначально
        let scene = Scene {
            camera,
            light_source,
            active_object_idx: 0,
            objects: vec![],
        };

        Self {
            texture: None,
            frame: RgbImage::from_pixel(IMG_WIDTH, IMG_HEIGHT, BACKGROUND_COLOR),
            scene,
            renderer: Box::new(ZBufferPerformer::new(IMG_WIDTH, IMG_HEIGHT)),
            fps: 0.0,
            last_frame_time: Instant::now(),
            selected_source_file: String::new(),
            selected_target_file: String::new(),
            view_mode: ViewMode::Source,
            viewport_has_pointer: false,
            source_mesh: None,
            target_mesh: None,
            morph_object: None,
            morph_created: false,
            morph_phase: 0.0,
            error_message: None,
            needs_redraw: false,
            viewport_width: IMG_WIDTH,
            viewport_height: IMG_HEIGHT,
        }
    }
}

impl MyEguiApp {
    pub(super) fn get_current_view_object(&self) -> Option<&dyn InteractiveModel> {
        match self.view_mode {
            ViewMode::Source => self
                .source_mesh
                .as_ref()
                .map(|mesh| mesh as &dyn InteractiveModel),
            ViewMode::Target => self
                .target_mesh
                .as_ref()
                .map(|mesh| mesh as &dyn InteractiveModel),
            ViewMode::Morph => self
                .morph_object
                .as_ref()
                .map(|morph| morph as &dyn InteractiveModel),
        }
    }

    pub(super) fn get_current_view_object_mut(&mut self) -> Option<&mut dyn InteractiveModel> {
        match self.view_mode {
            ViewMode::Source => self
                .source_mesh
                .as_mut()
                .map(|mesh| mesh as &mut dyn InteractiveModel),
            ViewMode::Target => self
                .target_mesh
                .as_mut()
                .map(|mesh| mesh as &mut dyn InteractiveModel),
            ViewMode::Morph => self
                .morph_object
                .as_mut()
                .map(|morph| morph as &mut dyn InteractiveModel),
        }
    }

    pub fn update_frame(&mut self, ctx: &Context) {
        // Проверяем, нужно ли перерисовывать кадр
        if !self.needs_redraw {
            // Если перерисовка не нужна, просто обновляем текстуру если она есть
            if self.texture.is_some() {
                return;
            }
            // Если текстуры нет, нужно её создать
            self.needs_redraw = true;
        }

        // Рендерим объект в зависимости от режима просмотра
        match self.view_mode {
            ViewMode::Source => {
                if let Some(ref mesh) = self.source_mesh {
                    self.renderer.render_single_object(
                        &mut self.frame,
                        mesh,
                        &self.scene.camera,
                        &self.scene.light_source,
                    );
                } else {
                    self.frame
                        .pixels_mut()
                        .for_each(|px| *px = BACKGROUND_COLOR);
                }
            }
            ViewMode::Target => {
                if let Some(ref mesh) = self.target_mesh {
                    self.renderer.render_single_object(
                        &mut self.frame,
                        mesh,
                        &self.scene.camera,
                        &self.scene.light_source,
                    );
                } else {
                    self.frame
                        .pixels_mut()
                        .for_each(|px| *px = BACKGROUND_COLOR);
                }
            }
            ViewMode::Morph => {
                if let Some(ref morph) = self.morph_object {
                    self.renderer.render_single_object(
                        &mut self.frame,
                        morph,
                        &self.scene.camera,
                        &self.scene.light_source,
                    );
                } else {
                    self.frame
                        .pixels_mut()
                        .for_each(|px| *px = BACKGROUND_COLOR);
                }
            }
        }

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

        // Сбрасываем флаг после перерисовки
        self.needs_redraw = false;
    }

    pub fn update_fps(&mut self) {
        let now = Instant::now();
        let frame_time = now.duration_since(self.last_frame_time).as_secs_f64();
        self.last_frame_time = now;
        self.fps = 1.0 / frame_time;
    }

    pub fn load_mesh_from_path(&mut self, file_path: &str, is_target: bool) {
        match TriangleMesh::from_obj(file_path) {
            Ok(mesh) => {
                if is_target {
                    self.target_mesh = Some(mesh);
                    if let Some(file_name) = PathBuf::from(file_path).file_name() {
                        self.selected_target_file = file_name.to_string_lossy().to_string();
                    }
                } else {
                    self.source_mesh = Some(mesh);
                    if let Some(file_name) = PathBuf::from(file_path).file_name() {
                        self.selected_source_file = file_name.to_string_lossy().to_string();
                    }
                }
                self.update_scene_objects();
                self.morph_created = false;
                self.needs_redraw = true; // Требуется перерисовка после загрузки модели
            }
            Err(e) => {
                eprintln!("Ошибка загрузки модели {}: {}", file_path, e);
                self.error_message = Some(format!("Ошибка загрузки модели {}: {}", file_path, e));
            }
        }
    }

    pub fn open_file_dialog(&mut self, is_target: bool) {
        if let Some(path) = FileDialog::new()
            .add_filter("OBJ файлы", &["obj"])
            .set_directory("./code/data")
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            self.load_mesh_from_path(&path_str, is_target);
        }
    }

    pub fn create_morph_object(&mut self) {
        if let (Some(source), Some(target)) = (&self.source_mesh, &self.target_mesh) {
            match Morph::new(source.clone(), target.clone()) {
                Ok(morph) => {
                    self.morph_object = Some(morph);
                    self.morph_created = true;
                    self.morph_phase = 0.0; // Сброс фазы морфинга
                    self.update_scene_objects();
                    self.needs_redraw = true; // Требуется перерисовка после создания морфинга
                }
                Err(e) => {
                    eprintln!("Ошибка создания морфинга: {}", e);
                    self.error_message = Some(
                        "Не удалось создать морфинг: сетка повреждена или не замкнута)".into(),
                    );
                    self.morph_created = false;
                }
            }
        }
    }

    pub fn update_scene_objects(&mut self) {
        self.scene.objects.clear();
        self.scene.active_object_idx = 0;
    }

    pub fn reset_current_object(&mut self) {
        let object_to_reset = self.get_current_view_object_mut();
        if let Some(object_to_reset) = object_to_reset {
            object_to_reset.reset_transformations();
        }
        self.needs_redraw = true; // Требуется перерисовка после сброса трансформаций
    }

    pub fn apply_button_rotation(&mut self, x: f64, y: f64, z: f64) {
        if let Some(object) = self.get_current_view_object_mut() {
            object.rotate((x.to_radians(), y.to_radians(), z.to_radians()));
        }
        self.needs_redraw = true; // Требуется перерисовка после поворота
    }

    pub fn apply_button_scale(&mut self, factor: f64) {
        if let Some(object) = self.get_current_view_object_mut() {
            object.scale(factor);
        }
        self.needs_redraw = true; // Требуется перерисовка после масштабирования
    }

    pub fn update_viewport_size(&mut self, width: u32, height: u32) {
        // Проверяем, изменился ли размер viewport
        if self.viewport_width != width || self.viewport_height != height {
            self.viewport_width = width;
            self.viewport_height = height;

            // Пересоздаем изображение с новым размером
            self.frame = RgbImage::from_pixel(width, height, BACKGROUND_COLOR);

            // Обновляем aspect ratio камеры
            let new_aspect_ratio = width as f64 / height as f64;
            self.scene.camera = Camera::new(
                self.scene.camera.pos,
                Point3::new(0.0, 0.0, 0.0),
                Vector3::new(0.0, 1.0, 0.0),
                FOV_DEGREES.to_radians(),
                new_aspect_ratio,
                NEAR_PLANE.into(),
                FAR_PLANE,
            );

            // Помечаем что нужна перерисовка
            self.needs_redraw = true;
        }
    }
}
