mod config;
mod objects;
mod render;
mod scene;
mod utils;
mod app;

use app::MyEguiApp;
use eframe::egui::{Context};
use eframe::{App, Frame, NativeOptions};

impl App for MyEguiApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.update_fps();
        self.mouse_wheel_scaling(ctx);
        self.mouse_drag_rotation(ctx);
        self.render_ui(ctx);

        ctx.request_repaint();
    }
}

fn main() -> Result<(), eframe::Error> {
    let app = MyEguiApp::default();
    let native_options = NativeOptions::default();
    eframe::run_native(
        "Морфинг фруктов",
        native_options,
        Box::new(|_cc| Ok(Box::new(app))),
    )
}
