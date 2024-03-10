pub use glam::*;

pub fn v2(a: f32, b: f32) -> Vec2 {
    Vec2::new(a, b)
}

pub fn normal(u: Vec2, v: Vec2) -> Vec2 {
    let mut normal = (v - u).perp();

    let length = normal.length();
    if length != 0.0 {
        normal /= length;
    }

    normal
}
