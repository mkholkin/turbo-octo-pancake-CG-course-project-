use crate::objects::model3d::Rotate;
use crate::objects::model3d::Scale;
use crate::app::MyEguiApp;
use crate::config::{ROTATION_SENSITIVITY_FACTOR, SCALING_SENSITIVITY_FACTOR};
use eframe::egui::{Context};

impl MyEguiApp {
    pub fn mouse_wheel_scaling(&mut self, ctx: &Context) {
        // Масштабирование работает только если курсор над окном просмотра
        if !self.viewport_has_pointer {
            return;
        }
        let scroll_delta = ctx.input(|i| i.raw_scroll_delta);
        if scroll_delta.y != 0.0 {
            let scaling_factor =
                (1. + scroll_delta.y.max(-200.) * SCALING_SENSITIVITY_FACTOR).max(f32::EPSILON);

            // Применяем масштабирование к текущему объекту напрямую
            match self.view_mode {
                crate::app::ViewMode::Source => {
                    if let Some(ref mut mesh) = self.source_mesh {
                        mesh.scale(scaling_factor.into());
                    }
                }
                crate::app::ViewMode::Target => {
                    if let Some(ref mut mesh) = self.target_mesh {
                        mesh.scale(scaling_factor.into());
                    }
                }
                crate::app::ViewMode::Morph => {
                    if let Some(ref mut morph) = self.morph_object {
                        morph.scale(scaling_factor.into());
                    }
                }
            }

            self.update_frame(ctx);
        }
    }

    pub fn mouse_drag_rotation(&mut self, ctx: &Context) {
        // Вращение работает только если курсор над окном просмотра
        if !self.viewport_has_pointer {
            return;
        }
        if ctx.input(|i| i.pointer.is_decidedly_dragging()) {
            let delta = ctx.input(|i| i.pointer.delta());

            let rotation_x = delta.y * ROTATION_SENSITIVITY_FACTOR;
            let rotation_y = delta.x * ROTATION_SENSITIVITY_FACTOR;

            // Применяем поворот к текущему объекту напрямую
            match self.view_mode {
                crate::app::ViewMode::Source => {
                    if let Some(ref mut mesh) = self.source_mesh {
                        mesh.rotate((
                            rotation_x.to_radians().into(),
                            rotation_y.to_radians().into(),
                            0.,
                        ));
                    }
                }
                crate::app::ViewMode::Target => {
                    if let Some(ref mut mesh) = self.target_mesh {
                        mesh.rotate((
                            rotation_x.to_radians().into(),
                            rotation_y.to_radians().into(),
                            0.,
                        ));
                    }
                }
                crate::app::ViewMode::Morph => {
                    if let Some(ref mut morph) = self.morph_object {
                        morph.rotate((
                            rotation_x.to_radians().into(),
                            rotation_y.to_radians().into(),
                            0.,
                        ));
                    }
                }
            }

            self.update_frame(ctx);
        }
    }
}
