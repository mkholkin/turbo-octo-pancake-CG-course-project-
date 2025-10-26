use super::state::MyEguiApp;
use crate::config::{ROTATION_SENSITIVITY_FACTOR, SCALING_SENSITIVITY_FACTOR};
use crate::objects::model3d::{Rotate, Scale};
use eframe::egui::Context;

impl MyEguiApp {
    pub fn mouse_wheel_scaling(&mut self, ctx: &Context) {
        // Масштабирование работает только если курсор над окном просмотра
        if !self.viewport_has_pointer {
            return;
        }
        let scroll_delta = ctx.input(|i| i.raw_scroll_delta);
        if scroll_delta.x == 0.0 && scroll_delta.y == 0.0 {
            return;
        }
        let scaling_factor =
            (1. + scroll_delta.y.max(-200.) * SCALING_SENSITIVITY_FACTOR).max(f32::EPSILON);

        // Применяем масштабирование к текущему объекту напрямую
        if let Some(object) = self.get_current_view_object_mut() {
            object.scale(scaling_factor.into());
        }

        self.needs_redraw = true; // Требуется перерисовка после масштабирования мышью
    }

    pub fn mouse_drag_rotation(&mut self, ctx: &Context) {
        // Вращение работает только если курсор над окном просмотра
        // if !self.viewport_has_pointer {
        //     return;
        // }
        if ctx.input(|i| i.pointer.primary_down()) {
            let delta = ctx.input(|i| i.pointer.delta());

            if delta.x == 0.0 && delta.y == 0.0 {
                return;
            }

            let rotation_x = delta.y * ROTATION_SENSITIVITY_FACTOR;
            let rotation_y = delta.x * ROTATION_SENSITIVITY_FACTOR;

            // Применяем поворот к текущему объекту напрямую
            if let Some(object) = self.get_current_view_object_mut() {
                object.rotate((
                    rotation_x.to_radians().into(),
                    rotation_y.to_radians().into(),
                    0.,
                ));
            }

            self.needs_redraw = true; // Требуется перерисовка после поворота мышью
        }
    }
}