use super::state::{MyEguiApp, ViewMode};
use crate::objects::model3d;
use crate::objects::model3d::Model3D;
use eframe::egui::{Context, SidePanel, CentralPanel, Ui, Vec2, Color32, ScrollArea};

impl MyEguiApp {
    pub fn render_ui(&mut self, ctx: &Context) {
        // Настройка глобальных стилей
        self.setup_custom_styles(ctx);

        // Правая панель с элементами управления
        SidePanel::right("controls_panel")
            .resizable(true)
            .default_width(320.0)
            .show(ctx, |ui| {
                // Добавляем прокрутку для всего содержимого панели
                ScrollArea::vertical()
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        ui.style_mut().spacing.slider_width = 235.0;
                        ui.heading("⚙ Управление");
                        ui.add_space(10.0);

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

                        // Добавляем немного пространства внизу для удобства прокрутки
                        ui.add_space(10.0);
                    });
            });

        // Центральная панель с окном просмотра
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("🍎 Морфинг фруктов");
            ui.add_space(5.0);

            // Режим просмотра сверху
            self.render_view_mode_controls(ui);

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
                    if self.styled_button(ui, "OK", Vec2::new(120.0, 32.0)).clicked() {
                        self.error_message = None;
                    }
                });
        }

        // Обновляем кадр
        self.update_frame(ctx);
    }

    fn setup_custom_styles(&self, ctx: &Context) {
        let mut style = (*ctx.style()).clone();

        // Увеличиваем размер текста для заголовков
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(20.0, egui::FontFamily::Proportional),
        );

        // Увеличиваем размер обычного текста
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        );

        // Размер текста кнопок оставляем как есть (14.0 - хороший)
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(14.0, egui::FontFamily::Proportional),
        );

        // Увеличиваем отступы в кнопках
        style.spacing.button_padding = Vec2::new(10.0, 6.0);

        // Улучшаем интерактивность
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);

        ctx.set_style(style);
    }

    // Вспомогательная функция для создания стилизованных кнопок
    fn styled_button(&self, ui: &mut Ui, text: &str, min_size: Vec2) -> egui::Response {
        ui.add_sized(min_size, egui::Button::new(text))
    }

    fn render_file_selection(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.add_space(5.0);
        ui.label("📂 Выбор OBJ файлов:");
        ui.add_space(8.0);

        // Исходный файл
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                ui.label("Исходный объект:");
                ui.add_space(5.0);
                    if !self.selected_source_file.is_empty() {
                        ui.label(format!("{}", self.selected_source_file));
                    }
                });

                // Кнопка для выбора любого файла
                if self.styled_button(ui, "📁 Выбрать файл...", Vec2::new(ui.available_width(), 36.0)).clicked() {
                    self.open_file_dialog(false);
                }
            });
        });

        ui.add_space(8.0);

        // Целевой файл
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Целевой объект:");
                    ui.add_space(5.0);
                    if !self.selected_target_file.is_empty() {
                        ui.label(format!("{}", self.selected_target_file));
                    }
                });

                // Кнопка для выбора любого файла
                if self.styled_button(ui, "📁 Выбрать файл...", Vec2::new(ui.available_width(), 36.0)).clicked() {
                    self.open_file_dialog(true);
                }
            });
        });
    }

    fn render_morph_controls(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.add_space(10.0);

        let can_create_morph = self.source_mesh.is_some() && self.target_mesh.is_some();
        let button_text = if self.morph_created { "🔄 Пересоздать морфинг" } else { "✨ Создать морфинг" };

        ui.vertical(|ui| {
            let response = ui.add_enabled(
                can_create_morph,
                egui::Button::new(button_text)
                    .min_size(Vec2::new(ui.available_width(), 40.0))
            );

            if response.clicked() {
                self.create_morph_object();
            }

            if !can_create_morph {
                ui.add_space(3.0);
                ui.colored_label(Color32::from_rgb(200, 100, 100), "⚠ Выберите оба объекта");
            }
        });
    }

    fn render_transform_controls(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.add_space(10.0);
        ui.label("🎯 Управление объектом:");
        ui.add_space(5.0);

        let has_object = match self.view_mode {
            ViewMode::Source => self.source_mesh.is_some(),
            ViewMode::Target => self.target_mesh.is_some(),
            ViewMode::Morph => self.morph_object.is_some(),
        };

        if !has_object {
            ui.colored_label(Color32::from_rgb(200, 100, 100), "⚠ Загрузите объект для управления");
            return;
        }

        // Поворот
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("🔄 Поворот (градусы):");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    if self.styled_button(ui, "↺ X +15°", Vec2::new(140.0, 32.0)).clicked() {
                        self.apply_button_rotation(15.0, 0.0, 0.0);
                    }
                    if self.styled_button(ui, "↻ X -15°", Vec2::new(140.0, 32.0)).clicked() {
                        self.apply_button_rotation(-15.0, 0.0, 0.0);
                    }
                });

                ui.horizontal(|ui| {
                    if self.styled_button(ui, "↺ Y +15°", Vec2::new(140.0, 32.0)).clicked() {
                        self.apply_button_rotation(0.0, 15.0, 0.0);
                    }
                    if self.styled_button(ui, "↻ Y -15°", Vec2::new(140.0, 32.0)).clicked() {
                        self.apply_button_rotation(0.0, -15.0, 0.0);
                    }
                });

                ui.horizontal(|ui| {
                    if self.styled_button(ui, "↺ Z +15°", Vec2::new(140.0, 32.0)).clicked() {
                        self.apply_button_rotation(0.0, 0.0, 15.0);
                    }
                    if self.styled_button(ui, "↻ Z -15°", Vec2::new(140.0, 32.0)).clicked() {
                        self.apply_button_rotation(0.0, 0.0, -15.0);
                    }
                });
            });
        });

        ui.add_space(8.0);

        // Масштабирование
        ui.group(|ui| {
            ui.vertical(|ui| {
                ui.label("🔍 Масштабирование:");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    if self.styled_button(ui, "➕ Увеличить x1.1", Vec2::new(140.0, 32.0)).clicked() {
                        self.apply_button_scale(1.1);
                    }
                    if self.styled_button(ui, "➖ Уменьшить x0.9", Vec2::new(140.0, 32.0)).clicked() {
                        self.apply_button_scale(0.9);
                    }
                });
            });
        });

        ui.add_space(8.0);

        // Кнопка сброса
        if self.styled_button(ui, "🔄 Сбросить преобразования", Vec2::new(ui.available_width(), 36.0)).clicked() {
            self.reset_current_object();
        }
    }

    fn render_view_mode_controls(&mut self, ui: &mut Ui) {
        ui.separator();
        ui.add_space(5.0);

        // Строка с заголовком и FPS справа
        ui.horizontal(|ui| {
            ui.label("👁 Режим просмотра:");

            // Прижимаем FPS к правому краю
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(format!("FPS: {}", self.fps as u32));
                ui.label("📊");
            });
        });

        ui.add_space(8.0);

        let old_view_mode = self.view_mode.clone();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            // Кнопка исходного режима
            ui.selectable_value(&mut self.view_mode, ViewMode::Source, "📦 Исходный");

            // Кнопка целевого режима
            ui.selectable_value(&mut self.view_mode, ViewMode::Target, "🎯 Целевой");

            // Кнопка морфинга - добавляем enabled wrapper
            ui.add_enabled_ui(self.morph_created, |ui| {
                let response = ui.selectable_value(&mut self.view_mode, ViewMode::Morph, "✨ Морфинг");

                if !self.morph_created {
                    response.on_disabled_hover_text("Создайте морфинг для активации");
                }
            });
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
            ui.add_space(10.0);

            ui.group(|ui| {
                ui.vertical(|ui| {
                    ui.label("⏱ Управление морфингом:");
                    ui.add_space(8.0);

                    // Ползунок для управления фазой морфинга
                    let old_phase = self.morph_phase;
                    ui.vertical(|ui| {
                        ui.label("Фаза:");
                        ui.add_space(3.0);
                        ui.add_sized(
                            Vec2::new(ui.available_width(), 20.0),
                            egui::Slider::new(&mut self.morph_phase, 0.0..=1.0)
                                .step_by(0.01)
                                .fixed_decimals(2)
                        );
                    });

                    // Обновляем морф-объект, если фаза изменилась
                    if (old_phase - self.morph_phase).abs() > f64::EPSILON {
                        if let Some(ref mut morph) = self.morph_object {
                            morph.update(self.morph_phase);
                        }
                        self.needs_redraw = true; // Требуется перерисовка при изменении фазы морфинга
                    }
                });
            });
        }
    }

    fn render_viewport(&mut self, ui: &mut Ui) {
        ui.separator();

        // Получаем до��тупное пространство для viewport
        let available_size = ui.available_size();

        // Вычисляем максимально возможный размер изображения в пикселях
        // Используем коэффицент масштабирования для выс��окого разрешения
        let pixels_per_point = ui.ctx().pixels_per_point();
        let viewport_width = (available_size.x * pixels_per_point) as u32;
        let viewport_height = (available_size.y * pixels_per_point) as u32;

        // Обновляем размер viewport и камеру, если размер изменился
        if viewport_width > 0 && viewport_height > 0 {
            self.update_viewport_size(viewport_width, viewport_height);
        }

        if let Some(texture) = &self.texture {
            // Отображаем изображение на весь доступный размер
            let resp = ui.image((texture.id(), available_size));
            // Обновляем флаг наличия курсора над viewport
            self.viewport_has_pointer = resp.hovered();
        } else {
            // Текстуры нет — курсор над viewport отсутствует
            self.viewport_has_pointer = false;
        }
    }

    fn render_material_controls(&mut self, ui: &mut Ui) {
        // Не показываем параметры материала в режиме морфинга
        if self.view_mode == ViewMode::Morph {
            return;
        }

        ui.separator();
        ui.add_space(10.0);
        ui.label("🎨 Параметры материала:");
        ui.add_space(5.0);

        // Показываем параметры только для исходного или целевого объекта
        let mut material_changed = false;

        match self.view_mode {
            ViewMode::Source => {
                if let Some(ref mut mesh) = self.source_mesh {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Исходный объект:");
                            ui.add_space(5.0);
                            material_changed = Self::render_material_sliders_static(ui, &mut mesh.material);
                        });
                    });
                }
            },
            ViewMode::Target => {
                if let Some(ref mut mesh) = self.target_mesh {
                    ui.group(|ui| {
                        ui.vertical(|ui| {
                            ui.label("Целевой объект:");
                            ui.add_space(5.0);
                            material_changed = Self::render_material_sliders_static(ui, &mut mesh.material);
                        });
                    });
                }
            },
            ViewMode::Morph => {
                // В режиме морфинга не показываем редактирование материала
            },
        }

        // Обновляем сцену после изменений, если были изменения
        if material_changed {
            self.update_scene_objects();
            self.needs_redraw = true; // Требуется перерисовка при изменении материала
        }
    }

    fn render_material_sliders_static(ui: &mut Ui, material: &mut model3d::Material) -> bool {
        let mut changed = false;

        ui.vertical(|ui| {
            ui.label("Цвет:");
            ui.add_space(3.0);
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

        ui.add_space(8.0);

        ui.vertical(|ui| {
            ui.label("Диффузное отражение:");
            ui.add_space(3.0);
            if ui.add_sized(
                Vec2::new(ui.available_width(), 20.0),
                egui::Slider::new(&mut material.diffuse_reflectance_factor, 0.0..=1.0)
                    .step_by(0.01)
                    .fixed_decimals(2)
            ).changed() {
                changed = true;
            }
        });

        ui.add_space(5.0);

        ui.vertical(|ui| {
            ui.label("Зеркальное отражение:");
            ui.add_space(3.0);
            if ui.add_sized(
                Vec2::new(ui.available_width(), 20.0),
                egui::Slider::new(&mut material.specular_reflectance_factor, 0.0..=1.0)
                    .step_by(0.01)
                    .fixed_decimals(2)
            ).changed() {
                changed = true;
            }
        });

        ui.add_space(5.0);

        ui.vertical(|ui| {
            ui.label("Глянцевость:");
            ui.add_space(3.0);
            if ui.add_sized(
                Vec2::new(ui.available_width(), 20.0),
                egui::Slider::new(&mut material.gloss, 0.1..=15.0)
                    .step_by(0.01)
                    .fixed_decimals(2)
            ).changed() {
                changed = true;
            }
        });

        changed
    }
}
