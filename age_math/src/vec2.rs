use std::{
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    /// A vector with all values set to zero.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// A vector with all values set to one.
    pub const ONE: Self = Self::new(1.0, 1.0);

    /// Create a new vector.
    #[inline(always)]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Create a new vector with all values set to x.
    #[inline(always)]
    pub const fn splat(x: f32) -> Self {
        Self::new(x, x)
    }

    /// Get the elements of the vector as an element-wise array.
    #[inline(always)]
    pub const fn to_array(&self) -> [f32; 2] {
        [self.x, self.y]
    }

    /// Calculate the area of the vector, as if it were a rectangle.
    #[inline(always)]
    pub fn area(&self) -> f32 {
        self.x * self.y
    }

    /// Calculate the length of the vector.
    #[inline(always)]
    pub fn length(&self) -> f32 {
        self.length_sq().sqrt()
    }

    /// Calculate the length squared of the vector.
    #[inline(always)]
    pub fn length_sq(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Calculate normalized vector.
    #[inline(always)]
    pub fn normalize(&self) -> Self {
        let len = self.length();
        v2(self.x / len, self.y / len)
    }

    /// Calculate the perpendicular vector.
    #[inline(always)]
    pub fn perp(&self) -> Self {
        Self::new(-self.y, self.x)
    }

    /// Calculate the dot product between self and rhs.
    #[inline(always)]
    pub fn dot(&self, rhs: Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y
    }

    /// Calculate the cross product between self and rhs.
    #[inline(always)]
    pub fn cross(&self, rhs: Self) -> f32 {
        self.x * rhs.y - self.y * rhs.x
    }

    /// Calculates the floor of each element.
    #[inline(always)]
    pub fn floor(&self) -> Self {
        v2(self.x.floor(), self.y.floor())
    }

    /// Calculates the ceil of each element.
    #[inline(always)]
    pub fn ceil(&self) -> Self {
        v2(self.x.ceil(), self.y.ceil())
    }

    /// Calculates the element-wise minimum of self and other.
    #[inline(always)]
    pub fn min(&self, other: Self) -> Self {
        Self::new(self.x.min(other.x), self.y.min(other.y))
    }

    /// Calculates the element-wise maximum of self and other.
    #[inline(always)]
    pub fn max(&self, other: Self) -> Self {
        Self::new(self.x.max(other.x), self.y.max(other.y))
    }

    /// Claps the elements of self between the element-wise minimum and maximum of u and v.
    #[inline(always)]
    pub fn clamp(&self, u: Self, v: Self) -> Self {
        self.max(u).min(v)
    }

    /// Linear interpolation between self and v by amount t.
    #[inline(always)]
    pub fn lerp(&self, v: Self, t: f32) -> Self {
        *self * (1.0 - t) + v * t
    }
}

impl Neg for Vec2 {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl MulAssign<f32> for Vec2 {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Mul<Self> for Vec2 {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl MulAssign<Self> for Vec2 {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;

    fn div(self, rhs: f32) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl DivAssign<f32> for Vec2 {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl Div<Self> for Vec2 {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl DivAssign<Self> for Vec2 {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl Add<Self> for Vec2 {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl AddAssign<Self> for Vec2 {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub<Self> for Vec2 {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl SubAssign<Self> for Vec2 {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Display for Vec2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{},{}]", self.x, self.y)
    }
}

/// Shorthand for creating a new vector.
pub const fn v2(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// Calculate the normal of 2 vectors.
pub fn normal(u: Vec2, v: Vec2) -> Vec2 {
    let mut normal = (v - u).perp();

    let length = normal.length();
    if length != 0.0 {
        normal /= length;
    }

    normal
}

impl From<(f32, f32)> for Vec2 {
    fn from(value: (f32, f32)) -> Self {
        v2(value.0, value.1)
    }
}

impl From<Vec2> for (f32, f32) {
    fn from(v: Vec2) -> Self {
        (v.x, v.y)
    }
}

impl From<[f32; 2]> for Vec2 {
    fn from(value: [f32; 2]) -> Self {
        v2(value[0], value[1])
    }
}

impl From<Vec2> for [f32; 2] {
    fn from(v: Vec2) -> Self {
        [v.x, v.y]
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vec2_default_is_zero() {
        assert_eq!(Vec2::ZERO, Vec2::default());
    }

    #[test]
    fn vec2_splat() {
        assert_eq!(Vec2::ONE, Vec2::splat(1.0));
    }

    #[test]
    fn vec2_to_array() {
        assert_eq!([1.0, 2.0], v2(1.0, 2.0).to_array());
    }

    #[test]
    fn vec2_scalar_multiplication() {
        let mut f = v2(2.0, 3.0);
        f *= 2.0;
        assert_eq!(v2(4.0, 6.0), v2(2.0, 3.0) * 2.0);
        assert_eq!(v2(4.0, 6.0), f);
    }

    #[test]
    fn vec2_multiplication() {
        let mut f = v2(2.0, 3.0);
        f *= v2(5.0, 10.0);
        assert_eq!(v2(10.0, 30.0), v2(2.0, 3.0) * v2(5.0, 10.0));
        assert_eq!(v2(10.0, 30.0), f);
    }

    #[test]
    fn vec2_scalar_division() {
        let mut f = v2(4.0, 6.0);
        f /= 2.0;
        assert_eq!(v2(2.0, 3.0), v2(4.0, 6.0) / 2.0);
        assert_eq!(v2(2.0, 3.0), f);
    }

    #[test]
    fn vec2_division() {
        let mut f = v2(10.0, 30.0);
        f /= v2(5.0, 10.0);
        assert_eq!(v2(2.0, 3.0), v2(10.0, 30.0) / v2(5.0, 10.0));
        assert_eq!(v2(2.0, 3.0), f);
    }

    #[test]
    fn vec2_addition() {
        let mut f = v2(2.0, 3.0);
        f += v2(5.0, 10.0);
        assert_eq!(v2(7.0, 13.0), v2(2.0, 3.0) + v2(5.0, 10.0));
        assert_eq!(v2(7.0, 13.0), f);
    }

    #[test]
    fn vec2_subtraction() {
        let mut f = v2(5.0, 10.0);
        f -= v2(2.0, 3.0);
        assert_eq!(v2(3.0, 7.0), v2(5.0, 10.0) - v2(2.0, 3.0));
        assert_eq!(v2(3.0, 7.0), f);
    }

    #[test]
    fn vec2_negation() {
        assert_eq!(v2(-2.0, -3.0), -v2(2.0, 3.0));
    }

    #[test]
    fn vec2_calculate_area() {
        assert_eq!(20.0, v2(4.0, 5.0).area());
    }

    #[test]
    fn vec2_calculate_length() {
        assert_eq!(5.0, v2(3.0, 4.0).length());
    }

    #[test]
    fn vec2_calculate_length_squared() {
        assert_eq!(25.0, v2(3.0, 4.0).length_sq());
    }

    #[test]
    fn vec2_normalize() {
        assert_eq!(v2(3.0 / 5.0, 4.0 / 5.0), v2(3.0, 4.0).normalize());
    }

    #[test]
    fn vec2_perpendicular() {
        assert_eq!(v2(-4.0, 3.0), v2(3.0, 4.0).perp());
    }

    #[test]
    fn vec2_calculate_dot_product() {
        assert_eq!(11.0, v2(1.0, 2.0).dot(v2(3.0, 4.0)));
    }

    #[test]
    fn vec2_calculate_cross_product() {
        assert_eq!(-2.0, v2(1.0, 2.0).cross(v2(3.0, 4.0)));
    }

    #[test]
    fn vec2_floor() {
        assert_eq!(v2(2.0, 3.0), v2(2.7, 3.4).floor());
    }

    #[test]
    fn vec2_ceil() {
        assert_eq!(v2(3.0, 4.0), v2(2.7, 3.4).ceil());
    }

    #[test]
    fn vec2_element_wise_minimum() {
        assert_eq!(v2(1.0, 2.0), v2(1.0, 9.0).min(v2(3.0, 2.0)));
    }

    #[test]
    fn vec2_element_wise_maximum() {
        assert_eq!(v2(3.0, 9.0), v2(1.0, 9.0).max(v2(3.0, 2.0)));
    }

    #[test]
    fn vec2_element_wise_clamp() {
        assert_eq!(
            v2(2.0, 8.0),
            v2(0.0, 6.0).clamp(v2(4.0, 8.0), v2(2.0, 10.0))
        );
    }

    #[test]
    fn vec2_lerp() {
        // 5 -> 10; t = 0.5 => 7.5
        // 3 -> 8;  t = 0.5 => 5.5
        assert_eq!(v2(7.5, 5.5), v2(5.0, 3.0).lerp(v2(10.0, 8.0), 0.5));
    }

    #[test]
    fn vec2_calculate_normal() {
        assert_eq!(v2(-1.0, 0.0), normal(v2(0.0, 0.0), v2(0.0, 10.0))); // left
        assert_eq!(v2(0.0, 1.0), normal(v2(0.0, 0.0), v2(10.0, 0.0))); // up
        assert_eq!(v2(1.0, 0.0), normal(v2(0.0, 10.0), v2(0.0, 0.0))); // right
        assert_eq!(v2(0.0, -1.0), normal(v2(10.0, 0.0), v2(0.0, 0.0))); // down
    }

    #[test]
    fn vec2_from() {
        assert_eq!(v2(3.0, 4.0), (3.0, 4.0).into());
        assert_eq!(v2(3.0, 4.0), [3.0, 4.0].into());

        assert_eq!((3.0, 4.0), v2(3.0, 4.0).into());
        assert_eq!([3.0, 4.0], Into::<[f32; 2]>::into(v2(3.0, 4.0)));
    }
}
