use crate::objects::model3d;
use crate::app::{MyEguiApp, ViewMode};
use eframe::egui::{Context, SidePanel, CentralPanel, Ui};

impl MyEguiApp {
    pub fn render_ui(&mut self, ctx: &Context) {
        // Правая панель с элементами управления
        SidePanel::right("controls_panel")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| {
                ui.heading("Управление");

                // UI для выбора файлов
                self.render_file_selection(ui);

                // Кнопка создания морфинга
                self.render_morph_controls(ui);

                // Управление трансформациями
                self.render_transform_controls(ui);

                // Параметры материала
                self.render_material_controls(ui);

                // Управление морфингом
                self.render_morph_instructions(ui);

                // Информация о FPS
                ui.separator();
                ui.label(format!("FPS: {}", self.fps as u32));
            });

        // Центральная панель с окном просмотра
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("Морфинг 3D объектов");

            // Режим просмотра сверху
            self.render_view_mode_controls(ui);

            ui.separator();

            // Отображение изображения
            self.render_viewport(ui);
        });

        // Модальное окно с ошибкой
        if let Some(error_msg) = &self.error_message.clone() {
            egui::Window::new("⚠ Ошибка")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(error_msg);
                    ui.separator();
                    if ui.button("OK").clicked() {
                        self.error_message = None;
                    }
                });
        }

        // Обновляем кадр
        self.update_frame(ctx);
    }

    fn render_file_selection(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label("Выбор OBJ файлов:");

        // Исходный файл
        ui.horizontal(|ui| {
            ui.label("Исходный:");
            ui.vertical(|ui| {
                // Кнопка для выбора любого файла
                if ui.button("📁 Выбрать файл...").clicked() {
                    self.open_file_dialog(false);
                }

                // Показываем текущий выбранный файл
                if !self.selected_source_file.is_empty() {
                    ui.label(format!("Выбран: {}", self.selected_source_file));
                }
            });
        });

        ui.separator();

        // Целевой файл
        ui.horizontal(|ui| {
            ui.label("Целевой:");
            ui.vertical(|ui| {
                // Кнопка для выбора любого файла
                if ui.button("📁 Выбрать файл...").clicked() {
                    self.open_file_dialog(true);
                }

                // Показываем текущий выбранный файл
                if !self.selected_target_file.is_empty() {
                    ui.label(format!("Выбран: {}", self.selected_target_file));
                }
            });
        });
    }

    fn render_morph_controls(&mut self, ui: &mut Ui) {
        ui.separator();
        let can_create_morph = self.source_mesh.is_some() && self.target_mesh.is_some();
        let button_text = if self.morph_created { "Пересоздать морфинг" } else { "Создать морфинг" };

        ui.horizontal(|ui| {
            if ui.add_enabled(can_create_morph, egui::Button::new(button_text)).clicked() {
                self.create_morph_object();
            }
            if !can_create_morph {
                ui.label("(Выберите исходный и целевой объекты)");
            }
        });
    }

    fn render_transform_controls(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label("Управление объектом:");

        let has_object = match self.view_mode {
            ViewMode::Source => self.source_mesh.is_some(),
            ViewMode::Target => self.target_mesh.is_some(),
            ViewMode::Morph => self.morph_object.is_some(),
        };

        if !has_object {
            ui.label("(Загрузите объект для управления)");
            return;
        }

        // Поворот
        ui.label("Поворот (градусы):");
        ui.horizontal(|ui| {
            if ui.button("↺ X +15°").clicked() {
                self.apply_button_rotation(15.0, 0.0, 0.0);
            }
            if ui.button("↻ X -15°").clicked() {
                self.apply_button_rotation(-15.0, 0.0, 0.0);
            }
        });
        ui.horizontal(|ui| {
            if ui.button("↺ Y +15°").clicked() {
                self.apply_button_rotation(0.0, 15.0, 0.0);
            }
            if ui.button("↻ Y -15°").clicked() {
                self.apply_button_rotation(0.0, -15.0, 0.0);
            }
        });
        ui.horizontal(|ui| {
            if ui.button("↺ Z +15°").clicked() {
                self.apply_button_rotation(0.0, 0.0, 15.0);
            }
            if ui.button("↻ Z -15°").clicked() {
                self.apply_button_rotation(0.0, 0.0, -15.0);
            }
        });

        ui.add_space(5.0);

        // Масштабирование
        ui.label("Масштабирование:");
        ui.horizontal(|ui| {
            if ui.button("🔍 Увеличить x1.1").clicked() {
                self.apply_button_scale(1.1);
            }
            if ui.button("🔍 Уменьшить x0.9").clicked() {
                self.apply_button_scale(0.9);
            }
        });

        ui.add_space(5.0);

        // Кнопка сброса
        if ui.button("🔄 Сбросить преобразования").clicked() {
            self.reset_current_object();
        }
    }

    fn render_view_mode_controls(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label("Режим просмотра:");

        let old_view_mode = self.view_mode.clone();

        ui.horizontal(|ui| {
            ui.radio_value(&mut self.view_mode, ViewMode::Source, "Исходный объект");
            ui.radio_value(&mut self.view_mode, ViewMode::Target, "Целевой объект");
            ui.add_enabled(self.morph_created, egui::RadioButton::new(self.view_mode == ViewMode::Morph, "Морфинг"));
            if ui.radio_value(&mut self.view_mode, ViewMode::Morph, "").clicked() && !self.morph_created {
                self.view_mode = old_view_mode.clone(); // Возврат к предыдущему режиму если морфинг не создан
            }
        });

        // Обновляем объекты сцены при смене режима
        if old_view_mode != self.view_mode {
            self.update_scene_objects();
            self.needs_redraw = true; // Требуется перерисовка при смене режима просмотра
        }
    }

    fn render_morph_instructions(&mut self, ui: &mut Ui) {
        if self.view_mode == ViewMode::Morph && self.morph_created {
            ui.separator();
            ui.label("Управление морфингом:");

            // Ползунок для управления фазой морфинга
            let old_phase = self.morph_phase;
            ui.horizontal(|ui| {
                ui.label("Фаза морфинга:");
                ui.add(egui::Slider::new(&mut self.morph_phase, 0.0..=1.0)
                    .step_by(0.01)
                    .text(""));
            });

            // Обновляем морф-объект, если фаза изменилась
            if (old_phase - self.morph_phase).abs() > f64::EPSILON {
                if let Some(ref mut morph) = self.morph_object {
                    morph.update(self.morph_phase);
                }
                self.needs_redraw = true; // Требуется перерисовка при изменении фазы морфинга
            }
        }
    }

    fn render_viewport(&mut self, ui: &mut Ui) {
        ui.separator();
        if let Some(texture) = &self.texture {
            let available_size = ui.available_size();
            let texture_size = texture.size_vec2();
            let aspect_ratio = texture_size.x / texture_size.y;

            // Масштабируем изображение чтобы поместилось в доступное пространство
            let display_size = if available_size.x / available_size.y > aspect_ratio {
                egui::Vec2::new(available_size.y * aspect_ratio, available_size.y)
            } else {
                egui::Vec2::new(available_size.x, available_size.x / aspect_ratio)
            };

            // Рисуем изображение и получаем Response, чтобы определить, наведен ли курсор
            let resp = ui.image((texture.id(), display_size));
            // Обновляем флаг наличия курсора над viewport
            self.viewport_has_pointer = resp.hovered();
        } else {
            // Текстуры нет — курсор над viewport отсутствует
            self.viewport_has_pointer = false;
        }
    }

    fn render_material_controls(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.label("Параметры материала:");

        // Показываем параметры только для исходного или целевого объекта
        let mut material_changed = false;

        match self.view_mode {
            ViewMode::Source => {
                if let Some(ref mut mesh) = self.source_mesh {
                    ui.label("Исходный объект:");
                    material_changed = Self::render_material_sliders_static(ui, &mut mesh.material);
                }
            },
            ViewMode::Target => {
                if let Some(ref mut mesh) = self.target_mesh {
                    ui.label("Целевой объект:");
                    material_changed = Self::render_material_sliders_static(ui, &mut mesh.material);
                }
            },
            ViewMode::Morph => {
                ui.label("Выберите исходный или целевой режим");
                ui.label("для редактирования материала");
            },
        }

        // Обновляем сцену после изменений, если были изменения
        if material_changed {
            self.update_scene_objects();
            self.needs_redraw = true; // Требуется перерисовка при изменении матери��ла
        }
    }

    fn render_material_sliders_static(ui: &mut Ui, material: &mut model3d::Material) -> bool {
        let mut changed = false;

        ui.horizontal(|ui| {
            ui.label("Цвет:");
            let mut color = [
                material.color.0[0] as f32 / 255.0,
                material.color.0[1] as f32 / 255.0,
                material.color.0[2] as f32 / 255.0,
            ];
            if ui.color_edit_button_rgb(&mut color).changed() {
                material.color = image::Rgb([
                    (color[0] * 255.0) as u8,
                    (color[1] * 255.0) as u8,
                    (color[2] * 255.0) as u8,
                ]);
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Диффузное отражение:");
            if ui.add(egui::Slider::new(&mut material.diffuse_reflectance_factor, 0.0..=1.0)
                .step_by(0.01)).changed() {
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Зеркальное отражение:");
            if ui.add(egui::Slider::new(&mut material.specular_reflectance_factor, 0.0..=1.0)
                .step_by(0.01)).changed() {
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Глянцевость:");
            if ui.add(egui::Slider::new(&mut material.gloss, 1.0..=128.0)
                .step_by(1.0)).changed() {
                changed = true;
            }
        });

        changed
    }
}
