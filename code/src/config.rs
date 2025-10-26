use image::Rgb;

pub const BACKGROUND_COLOR: Rgb<u8> = Rgb([30, 30, 30]);

// Camera settings
pub const FOV_DEGREES: f64 = 60.0;
pub const ASPECT_RATIO: f64 = 1.;
pub const NEAR_PLANE: f64 = 0.1;
pub const FAR_PLANE: f64 = 1000.0;

// Light behavior settings
pub const AMBIENT_INTENSITY: f32 = 0.1;
pub const DIFFUSION_FACTOR: f32 = 0.1;
pub const LIGHT_SCATTERING: f32 = 2.;

// User interaction settings
pub const SCALING_SENSITIVITY_FACTOR: f32 = 0.002;
pub const ROTATION_SENSITIVITY_FACTOR: f32 = 0.2;

// Morphing settings
pub const RELAXATION_ROUNDS_LIMIT: usize = 100000;