use crate::objects::camera::Camera;
use crate::objects::triangle_mesh::TriangleMesh;
use rfd::FileDialog;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
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
    pub source_mesh: Option<Rc<RefCell<TriangleMesh>>>,
    pub target_mesh: Option<Rc<RefCell<TriangleMesh>>>,
    pub morph_object: Option<Rc<RefCell<Morph>>>,
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

    // Сцена
    pub scene: Scene,
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

        let scene = Scene {
            camera,
            light_source,
            object: None,
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
    pub fn update_frame(&mut self, ctx: &Context) {
        // Проверяем, нужно ли перерисовывать кадр
        if !self.needs_redraw {
            if self.texture.is_some() {
                return;
            }
            self.needs_redraw = true;
        }

        // Рендерим сцену
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
                    self.target_mesh = Some(Rc::new(RefCell::new(mesh)));
                    if let Some(file_name) = PathBuf::from(file_path).file_name() {
                        self.selected_target_file = file_name.to_string_lossy().to_string();
                    }
                } else {
                    self.source_mesh = Some(Rc::new(RefCell::new(mesh)));
                    if let Some(file_name) = PathBuf::from(file_path).file_name() {
                        self.selected_source_file = file_name.to_string_lossy().to_string();
                    }
                }
                self.morph_created = false;
                self.update_scene_object();
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
            .set_directory("./code/models")
            .pick_file()
        {
            let path_str = path.to_string_lossy().to_string();
            self.load_mesh_from_path(&path_str, is_target);
        }
    }

    pub fn create_morph_object(&mut self) {
        if self.source_mesh.is_none() || self.target_mesh.is_none() {
            return;
        }

        let source_mesh = self.source_mesh.as_ref().unwrap().borrow().clone();
        let target_mesh = self.target_mesh.as_ref().unwrap().borrow().clone();

        match Morph::new(source_mesh, target_mesh) {
            Ok(morph) => {
                self.morph_object = Some(Rc::new(RefCell::new(morph)));
                self.morph_created = true;
                self.morph_phase = 0.0; // Сброс фазы морфинга
                self.update_scene_object();
            }
            Err(e) => {
                eprintln!("Ошибка создания морфинга: {}", e);
                self.error_message =
                    Some("Не удалось создать морфинг: сетка повреждена или не замкнута)".into());
                self.morph_created = false;
            }
        }
    }

    pub fn reset_current_object(&mut self) {
        if let Some(object_to_reset) = self.scene.object.as_ref() {
            object_to_reset.borrow_mut().reset_transformations();
        }
        self.needs_redraw = true; // Требуется перерисовка после сброса трансформаций
    }

    pub fn apply_button_rotation(&mut self, x: f64, y: f64, z: f64) {
        if let Some(object) = self.scene.object.as_ref() {
            object
                .borrow_mut()
                .rotate((x.to_radians(), y.to_radians(), z.to_radians()));
        }
        self.needs_redraw = true; // Требуется перерисовка после поворота
    }

    pub fn apply_button_scale(&mut self, factor: f64) {
        if let Some(object) = self.scene.object.as_ref() {
            object.borrow_mut().scale(factor);
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
                NEAR_PLANE,
                FAR_PLANE,
            );

            // Помечаем что нужна перерисовка
            self.needs_redraw = true;
        }
    }

    /// Устанавливает новый режим просмотра и помечает необходимость перерисо��ки
    pub fn set_view_mode(&mut self, new_mode: ViewMode) {
        if self.view_mode != new_mode {
            self.view_mode = new_mode;
            self.update_scene_object();
        }
    }

    pub fn update_scene_object(&mut self) {
        let object_to_set = match self.view_mode {
            ViewMode::Source => self
                .source_mesh
                .as_ref()
                .map(|rc| rc.clone() as Rc<RefCell<dyn InteractiveModel>>),
            ViewMode::Target => self
                .target_mesh
                .as_ref()
                .map(|rc| rc.clone() as Rc<RefCell<dyn InteractiveModel>>),
            ViewMode::Morph => self
                .morph_object
                .as_ref()
                .map(|rc| rc.clone() as Rc<RefCell<dyn InteractiveModel>>),
        };
        self.scene.object = object_to_set;
        self.needs_redraw = true;
    }
}
