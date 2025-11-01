use std::ops::{Add, Mul};

pub fn lerp<T>(a: T, b: T, t: f64) -> T
where
    T: Mul<f64, Output = T> + Add<T, Output = T> + Copy,
{
    a * (1.0 - t) + b * t
}
