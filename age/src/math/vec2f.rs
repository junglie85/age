use std::{
    fmt::Display,
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use super::vec2i::{v2i, Vec2i};

pub const fn v2(x: f32, y: f32) -> Vec2f {
    Vec2f::new(x, y)
}

pub fn normal(u: Vec2f, v: Vec2f) -> Vec2f {
    let mut normal = (v - u).perp();

    let len = normal.len();
    if len != 0.0 {
        normal /= len;
    }

    normal
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Vec2f {
    pub x: f32,
    pub y: f32,
}

impl Vec2f {
    pub const ZERO: Self = Self::splat(0.0);
    pub const ONE: Self = Self::splat(1.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub const fn splat(x: f32) -> Self {
        v2(x, x)
    }

    pub fn dot(&self, v: Self) -> f32 {
        self.x * v.x + self.y * v.y
    }

    pub fn len_sq(&self) -> f32 {
        self.dot(*self)
    }

    pub fn perp(&self) -> Self {
        v2(-self.y, self.x)
    }

    pub fn cast(&self) -> Vec2i {
        v2i(self.x as i32, self.y as i32)
    }

    pub fn len(&self) -> f32 {
        f32::sqrt(self.len_sq())
    }

    pub fn recip(&self) -> Self {
        v2(1.0 / self.x, 1.0 / self.y)
    }

    pub fn normalize(&self) -> Self {
        self.mul(self.len().recip())
    }
}

impl From<Vec2i> for Vec2f {
    fn from(v: Vec2i) -> Self {
        v2(v.x as f32, v.y as f32)
    }
}

impl Display for Vec2f {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.x, self.y)
    }
}

impl From<Vec2f> for (f32, f32) {
    fn from(v: Vec2f) -> Self {
        (v.x, v.y)
    }
}

impl From<Vec2f> for [f32; 2] {
    fn from(v: Vec2f) -> Self {
        [v.x, v.y]
    }
}

impl From<(f32, f32)> for Vec2f {
    fn from((x, y): (f32, f32)) -> Self {
        v2(x, y)
    }
}

impl From<[f32; 2]> for Vec2f {
    fn from(value: [f32; 2]) -> Self {
        v2(value[0], value[1])
    }
}

impl Neg for Vec2f {
    type Output = Vec2f;

    fn neg(self) -> Self::Output {
        v2(-self.x, -self.y)
    }
}

impl Mul<f32> for Vec2f {
    type Output = Vec2f;

    fn mul(self, s: f32) -> Self::Output {
        v2(self.x * s, self.y * s)
    }
}

impl MulAssign<f32> for Vec2f {
    fn mul_assign(&mut self, s: f32) {
        self.x *= s;
        self.y *= s;
    }
}

impl Div<f32> for Vec2f {
    type Output = Vec2f;

    fn div(self, s: f32) -> Self::Output {
        v2(self.x / s, self.y / s)
    }
}

impl DivAssign<f32> for Vec2f {
    fn div_assign(&mut self, s: f32) {
        self.x /= s;
        self.y /= s;
    }
}

impl Add for Vec2f {
    type Output = Vec2f;

    fn add(self, v: Self) -> Self::Output {
        v2(self.x + v.x, self.y + v.y)
    }
}

impl AddAssign for Vec2f {
    fn add_assign(&mut self, v: Self) {
        self.x += v.x;
        self.y += v.y;
    }
}

impl Sub for Vec2f {
    type Output = Vec2f;

    fn sub(self, v: Self) -> Self::Output {
        v2(self.x - v.x, self.y - v.y)
    }
}

impl SubAssign for Vec2f {
    fn sub_assign(&mut self, v: Self) {
        self.x -= v.x;
        self.y -= v.y;
    }
}

impl Mul for Vec2f {
    type Output = Vec2f;

    fn mul(self, v: Self) -> Self::Output {
        v2(self.x * v.x, self.y * v.y)
    }
}

impl MulAssign for Vec2f {
    fn mul_assign(&mut self, v: Self) {
        self.x *= v.x;
        self.y *= v.y;
    }
}

impl Div for Vec2f {
    type Output = Vec2f;

    fn div(self, v: Self) -> Self::Output {
        v2(self.x / v.x, self.y / v.y)
    }
}

impl DivAssign for Vec2f {
    fn div_assign(&mut self, v: Self) {
        self.x /= v.x;
        self.y /= v.y;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vec2f_defaults_to_zero() {
        assert!(Vec2f::default() == Vec2f::ZERO);
    }

    #[test]
    fn vec2f_has_value_one() {
        assert_eq!(Vec2f::splat(1.0), Vec2f::ONE);
    }

    #[test]
    fn vec2f_can_be_destructured() {
        assert_eq!((2.0, 3.0), v2(2.0, 3.0).into());
    }

    #[test]
    fn vec2f_from_vec2i() {
        let v = v2i(2, 3);
        assert_eq!(v2(2.0, 3.0), Vec2f::from(v));
    }

    #[test]
    fn vec2f_cast_vec2i() {
        let v = v2(2.0, 3.0);
        assert_eq!(v2i(2, 3), v.cast());
    }

    #[test]
    fn vec2f_can_be_negated() {
        assert_eq!(-v2(1.0, 2.0), v2(-1.0, -2.0));
    }

    #[test]
    fn vec2f_scalar_mul() {
        let mut v2f_assign = v2(1.0, 2.0);
        v2f_assign *= 5.0;
        let v2f = v2(1.0, 2.0) * 5.0;

        assert_eq!(v2f, v2f_assign);
    }

    #[test]
    fn vec2f_scalar_div() {
        let mut v2f_assign = v2(6.0, 10.0);
        v2f_assign /= 2.0;
        let v2f = v2(6.0, 10.0) / 2.0;

        assert_eq!(v2f, v2f_assign);
    }

    #[test]
    fn vec2f_add() {
        let mut v2f_assign = v2(5.0, 10.0);
        v2f_assign += v2(2.0, 4.0);
        let v2f = v2(5.0, 10.0) + v2(2.0, 4.0);

        assert_eq!(v2f, v2f_assign);
    }

    #[test]
    fn vec2f_sub() {
        let mut v2f_assign = v2(5.0, 10.0);
        v2f_assign -= v2(2.0, 4.0);
        let v2f = v2(5.0, 10.0) - v2(2.0, 4.0);

        assert_eq!(v2f, v2f_assign);
    }

    #[test]
    fn vec2f_mul() {
        let mut v2f_assign = v2(5.0, 10.0);
        v2f_assign *= v2(2.0, 4.0);
        let v2f = v2(5.0, 10.0) * v2(2.0, 4.0);

        assert_eq!(v2f, v2f_assign);
    }

    #[test]
    fn vec2f_div() {
        let mut v2f_assign = v2(5.0, 10.0);
        v2f_assign /= v2(2.0, 4.0);
        let v2f = v2(5.0, 10.0) / v2(2.0, 4.0);

        assert_eq!(v2f, v2f_assign);
    }

    #[test]
    fn vec2f_dot() {
        assert_eq!(23.0, v2(2.0, 3.0).dot(v2(4.0, 5.0)));
    }

    #[test]
    fn vec_length_squared() {
        assert_eq!(13.0, v2(2.0, 3.0).len_sq());
    }

    #[test]
    fn vec2f_perpendicular() {
        assert_eq!(v2(-10.0, 5.0), v2(5.0, 10.0).perp());
    }

    #[test]
    fn vec2f_length() {
        assert_eq!(f32::sqrt(125.0), v2(5.0, 10.0).len());
    }

    #[test]
    fn vec2f_reciprocal() {
        assert_eq!(v2(0.5, 0.25), v2(2.0, 4.0).recip());
    }

    #[test]
    fn vec2f_normalize() {
        assert_eq!(
            v2(2.0 / f32::sqrt(20.0), 4.0 / f32::sqrt(20.0)),
            v2(2.0, 4.0).normalize()
        );
    }

    #[test]
    fn vec2f_normal_between_vecs() {
        assert_eq!(
            // n = (u - v).perp(); n / n.len().
            v2(5.0 / f32::sqrt(125.0), 10.0 / f32::sqrt(125.0)),
            normal(v2(0.0, 5.0), v2(10.0, 0.0))
        );
    }
}
