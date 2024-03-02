use crate::math::{v2, Mat4, Vec2f};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    pos: Vec2f,
    zoom: f32,
    rotation: f32,
}

impl Camera {
    pub fn new(left: f32, right: f32, bottom: f32, top: f32) -> Self {
        Self {
            left,
            right,
            bottom,
            top,
            pos: Vec2f::ZERO,
            zoom: 1.0,
            rotation: 0.0,
        }
    }

    pub fn get_view_projection_matrix(&self) -> Mat4 {
        let left = self.left / self.zoom;
        let right = self.right / self.zoom;
        let bottom = self.bottom / self.zoom;
        let top = self.top / self.zoom;
        let proj = Mat4::ortho(left, right, bottom, top, 100.0, 0.0);

        let width = self.right - self.left;
        let height = self.bottom - self.top;
        let origin = self.pos + v2(width, height) / 2.0;
        let view = (Mat4::translation(self.pos)
            * Mat4::translation(origin)
            * Mat4::rotation(self.rotation)
            * Mat4::translation(-origin)
            * Mat4::scale(Vec2f::ONE))
        .inverse();

        proj * view
    }
}
