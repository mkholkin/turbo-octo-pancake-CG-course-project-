// Модуль графического приложения - объединяет состояние, UI и обработку ввода
pub mod state;
pub mod ui;
pub mod input;

// Реэкспортируем основные типы для удобства использования
pub use state::{MyEguiApp, ViewMode};

