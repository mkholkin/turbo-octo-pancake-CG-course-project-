use image::Rgba;

pub const BACKGROUND_COLOR: Rgba<u8> = Rgba([70, 70, 70, 255]);

// Camera settings
pub const FOV_DEGREES: f32 = 60.0;
pub const ASPECT_RATIO: f32 = 1.0;
pub const NEAR_PLANE: f32 = 0.1;
pub const FAR_PLANE: f32 = 1000.0;

// Light behavior settings
pub const AMBIENT_INTENSITY: f32 = 0.05;
pub const DIFFUSION_FACTOR: f32 = 0.1;
pub const LIGHT_SCATTERING: f32 = 0.1;

// User interaction settings
pub const SCALING_SENSITIVITY_FACTOR: f32 = 0.002;
pub const ROTATION_SENSITIVITY_FACTOR: f32 = 0.2;