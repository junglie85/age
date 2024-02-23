use std::{
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

pub const fn v2i(x: i32, y: i32) -> Vec2i {
    Vec2i::new(x, y)
}

#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd, Eq)]
#[repr(C)]
pub struct Vec2i {
    pub x: i32,
    pub y: i32,
}

impl Vec2i {
    pub const ZERO: Self = Self::splat(0);
    pub const ONE: Self = Self::splat(1);

    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub const fn splat(x: i32) -> Self {
        v2i(x, x)
    }

    pub fn dot(&self, v: Vec2i) -> i32 {
        self.x * v.x + self.y * v.y
    }

    pub fn len_sq(&self) -> i32 {
        self.dot(*self)
    }

    pub fn perp(&self) -> Self {
        v2i(-self.y, self.x)
    }
}

impl Display for Vec2i {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.x, self.y)
    }
}

impl From<Vec2i> for (i32, i32) {
    fn from(v: Vec2i) -> Self {
        (v.x, v.y)
    }
}

impl From<Vec2i> for [i32; 2] {
    fn from(v: Vec2i) -> Self {
        [v.x, v.y]
    }
}

impl From<(i32, i32)> for Vec2i {
    fn from((x, y): (i32, i32)) -> Self {
        v2i(x, y)
    }
}

impl From<[i32; 2]> for Vec2i {
    fn from(value: [i32; 2]) -> Self {
        v2i(value[0], value[1])
    }
}

impl Neg for Vec2i {
    type Output = Vec2i;

    fn neg(self) -> Self::Output {
        v2i(-self.x, -self.y)
    }
}

impl Mul<i32> for Vec2i {
    type Output = Vec2i;

    fn mul(self, s: i32) -> Self::Output {
        v2i(self.x * s, self.y * s)
    }
}

impl MulAssign<i32> for Vec2i {
    fn mul_assign(&mut self, s: i32) {
        self.x *= s;
        self.y *= s;
    }
}

impl Div<i32> for Vec2i {
    type Output = Vec2i;

    fn div(self, s: i32) -> Self::Output {
        v2i(self.x / s, self.y / s)
    }
}

impl DivAssign<i32> for Vec2i {
    fn div_assign(&mut self, s: i32) {
        self.x /= s;
        self.y /= s;
    }
}

impl Add for Vec2i {
    type Output = Vec2i;

    fn add(self, v: Self) -> Self::Output {
        v2i(self.x + v.x, self.y + v.y)
    }
}

impl AddAssign for Vec2i {
    fn add_assign(&mut self, v: Self) {
        self.x += v.x;
        self.y += v.y;
    }
}

impl Sub for Vec2i {
    type Output = Vec2i;

    fn sub(self, v: Self) -> Self::Output {
        v2i(self.x - v.x, self.y - v.y)
    }
}

impl SubAssign for Vec2i {
    fn sub_assign(&mut self, v: Self) {
        self.x -= v.x;
        self.y -= v.y;
    }
}

impl Mul for Vec2i {
    type Output = Vec2i;

    fn mul(self, v: Self) -> Self::Output {
        v2i(self.x * v.x, self.y * v.y)
    }
}

impl MulAssign for Vec2i {
    fn mul_assign(&mut self, v: Self) {
        self.x *= v.x;
        self.y *= v.y;
    }
}

impl Div for Vec2i {
    type Output = Vec2i;

    fn div(self, v: Self) -> Self::Output {
        v2i(self.x / v.x, self.y / v.y)
    }
}

impl DivAssign for Vec2i {
    fn div_assign(&mut self, v: Self) {
        self.x /= v.x;
        self.y /= v.y;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vec_defaults_to_zero() {
        assert!(Vec2i::default() == Vec2i::ZERO);
    }

    #[test]
    fn vec2_has_value_one() {
        assert_eq!(Vec2i::splat(1), Vec2i::ONE);
    }

    #[test]
    fn vec2_can_be_destructured() {
        assert_eq!((2, 3), v2i(2, 3).into());
    }

    #[test]
    fn vec2_can_be_negated() {
        assert_eq!(-v2i(1, 2), v2i(-1, -2));
    }

    #[test]
    fn vec2_scalar_mul() {
        let mut v2i_assign = v2i(1, 2);
        v2i_assign *= 5;
        let v2i = v2i(1, 2) * 5;

        assert_eq!(v2i, v2i_assign);
    }

    #[test]
    fn vec2_scalar_div() {
        let mut v2i_assign = v2i(6, 10);
        v2i_assign /= 2;
        let v2i = v2i(6, 10) / 2;

        assert_eq!(v2i, v2i_assign);
    }

    #[test]
    fn vec2_add() {
        let mut v2i_assign = v2i(5, 10);
        v2i_assign += v2i(2, 4);
        let v2i = v2i(5, 10) + v2i(2, 4);

        assert_eq!(v2i, v2i_assign);
    }

    #[test]
    fn vec2_sub() {
        let mut v2i_assign = v2i(5, 10);
        v2i_assign -= v2i(2, 4);
        let v2i = v2i(5, 10) - v2i(2, 4);

        assert_eq!(v2i, v2i_assign);
    }

    #[test]
    fn vec2_mul() {
        let mut v2i_assign = v2i(5, 10);
        v2i_assign *= v2i(2, 4);
        let v2i = v2i(5, 10) * v2i(2, 4);

        assert_eq!(v2i, v2i_assign);
    }

    #[test]
    fn vec2_div() {
        let mut v2i_assign = v2i(5, 10);
        v2i_assign /= v2i(2, 4);
        let v2i = v2i(5, 10) / v2i(2, 4);

        assert_eq!(v2i, v2i_assign);
    }
}
