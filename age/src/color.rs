#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0.0, 0.0, 0.0, 0.0);
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);

    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn rgb_u8(r: u8, g: u8, b: u8) -> Self {
        Self::rgba_u8(r, g, b, 255)
    }

    pub fn rgba_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        let r = r as f32 / 255.0;
        let g = g as f32 / 255.0;
        let b = b as f32 / 255.0;
        let a = a as f32 / 255.0;

        Self { r, g, b, a }
    }

    pub const fn to_array_f32(self) -> [f32; 4] {
        let r = self.r;
        let g = self.g;
        let b = self.b;
        let a = self.a;

        [r, g, b, a]
    }

    pub fn to_array_u8(self) -> [u8; 4] {
        let r = (self.r * 255.0) as u8;
        let g = (self.g * 255.0) as u8;
        let b = (self.b * 255.0) as u8;
        let a = (self.a * 255.0) as u8;

        [r, g, b, a]
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::TRANSPARENT
    }
}

impl From<Color> for wgpu::Color {
    fn from(color: Color) -> Self {
        wgpu::Color {
            r: color.r as f64,
            g: color.g as f64,
            b: color.b as f64,
            a: color.a as f64,
        }
    }
}
