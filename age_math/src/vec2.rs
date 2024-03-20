use std::{
    fmt::{Debug, Display},
    ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

pub trait Arithmetic:
    Debug
    + Display
    + Default
    + Clone
    + Copy
    + PartialEq
    + Add<Output = Self>
    + AddAssign
    + Div<Output = Self>
    + DivAssign
    + Mul<Output = Self>
    + MulAssign
    + Sub<Output = Self>
    + SubAssign
{
}

impl<T> Arithmetic for T where
    T: Debug
        + Default
        + Display
        + Clone
        + Copy
        + PartialEq
        + Add<Output = Self>
        + AddAssign
        + Div<Output = Self>
        + DivAssign
        + Mul<Output = Self>
        + MulAssign
        + Sub<Output = Self>
        + SubAssign
{
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Vec2<T: Arithmetic> {
    pub x: T,
    pub y: T,
}

impl<T: Arithmetic> Vec2<T> {
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }

    pub fn to_array(&self) -> [T; 2] {
        [self.x, self.y]
    }
}

impl<T: Arithmetic + Neg<Output = T>> Neg for Vec2<T> {
    type Output = Vec2<T>;

    fn neg(self) -> Self::Output {
        Self::new(-self.x, -self.y)
    }
}

impl<T: Arithmetic> Mul<T> for Vec2<T> {
    type Output = Self;

    fn mul(self, rhs: T) -> Self::Output {
        Self::new(self.x * rhs, self.y * rhs)
    }
}

impl<T: Arithmetic> MulAssign<T> for Vec2<T> {
    fn mul_assign(&mut self, rhs: T) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl<T: Arithmetic> Mul<Self> for Vec2<T> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::new(self.x * rhs.x, self.y * rhs.y)
    }
}

impl<T: Arithmetic> MulAssign<Self> for Vec2<T> {
    fn mul_assign(&mut self, rhs: Self) {
        self.x *= rhs.x;
        self.y *= rhs.y;
    }
}

impl<T: Arithmetic> Div<T> for Vec2<T> {
    type Output = Self;

    fn div(self, rhs: T) -> Self::Output {
        Self::new(self.x / rhs, self.y / rhs)
    }
}

impl<T: Arithmetic> DivAssign<T> for Vec2<T> {
    fn div_assign(&mut self, rhs: T) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl<T: Arithmetic> Div<Self> for Vec2<T> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::new(self.x / rhs.x, self.y / rhs.y)
    }
}

impl<T: Arithmetic> DivAssign<Self> for Vec2<T> {
    fn div_assign(&mut self, rhs: Self) {
        self.x /= rhs.x;
        self.y /= rhs.y;
    }
}

impl<T: Arithmetic> Add<Self> for Vec2<T> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl<T: Arithmetic> AddAssign<Self> for Vec2<T> {
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl<T: Arithmetic> Sub<Self> for Vec2<T> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl<T: Arithmetic> SubAssign<Self> for Vec2<T> {
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl<T: Arithmetic> Display for Vec2<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{},{}]", self.x, self.y)
    }
}

pub type Vec2f = Vec2<f32>;
pub type Vec2i = Vec2<i32>;
pub type Vec2u = Vec2<u32>;

pub fn v2<T>(x: T, y: T) -> Vec2<T>
where
    T: Arithmetic,
{
    Vec2::new(x, y)
}

pub fn v2f(x: f32, y: f32) -> Vec2f {
    v2(x, y)
}

pub fn v2i(x: i32, y: i32) -> Vec2i {
    v2(x, y)
}

pub fn v2u(x: u32, y: u32) -> Vec2u {
    v2(x, y)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vec2_default_is_default_t() {
        let vf = Vec2::<f32>::default();
        let vi = Vec2::<i32>::default();
        let vu = Vec2::<u32>::default();

        assert_eq!(v2(0.0, 0.0), vf);
        assert_eq!(v2(0_i32, 0_i32), vi);
        assert_eq!(v2(0_u32, 0_u32), vu);
    }

    #[test]
    fn vec2_to_array() {
        assert_eq!([1.0, 2.0], v2f(1.0, 2.0).to_array());
        assert_eq!([1, 2], v2i(1, 2).to_array());
        assert_eq!([1, 2], v2u(1, 2).to_array());
    }

    #[test]
    fn vec2_scalar_multiplication() {
        let mut f = v2f(2.0, 3.0);
        f *= 2.0;
        assert_eq!(v2f(4.0, 6.0), v2f(2.0, 3.0) * 2.0);
        assert_eq!(v2f(4.0, 6.0), f);

        let mut i = v2i(2, 3);
        i *= 2;
        assert_eq!(v2i(4, 6), v2i(2, 3) * 2);
        assert_eq!(v2i(4, 6), i);

        let mut u = v2u(2, 3);
        u *= 2;
        assert_eq!(v2u(4, 6), v2u(2, 3) * 2);
        assert_eq!(v2u(4, 6), u);
    }

    #[test]
    fn vec2_multiplication() {
        let mut f = v2f(2.0, 3.0);
        f *= v2f(5.0, 10.0);
        assert_eq!(v2f(10.0, 30.0), v2f(2.0, 3.0) * v2f(5.0, 10.0));
        assert_eq!(v2f(10.0, 30.0), f);

        let mut i = v2i(2, 3);
        i *= v2i(5, 10);
        assert_eq!(v2i(10, 30), v2i(2, 3) * v2i(5, 10));
        assert_eq!(v2i(10, 30), i);

        let mut u = v2u(2, 3);
        u *= v2u(5, 10);
        assert_eq!(v2u(10, 30), v2u(2, 3) * v2u(5, 10));
        assert_eq!(v2u(10, 30), u);
    }

    #[test]
    fn vec2_scalar_division() {
        let mut f = v2f(4.0, 6.0);
        f /= 2.0;
        assert_eq!(v2f(2.0, 3.0), v2f(4.0, 6.0) / 2.0);
        assert_eq!(v2f(2.0, 3.0), f);

        let mut i = v2i(4, 6);
        i /= 2;
        assert_eq!(v2i(2, 3), v2i(4, 6) / 2);
        assert_eq!(v2i(2, 3), i);

        let mut u = v2u(4, 6);
        u /= 2;
        assert_eq!(v2u(2, 3), v2u(4, 6) / 2);
        assert_eq!(v2u(2, 3), u);
    }

    #[test]
    fn vec2_division() {
        let mut f = v2f(10.0, 30.0);
        f /= v2f(5.0, 10.0);
        assert_eq!(v2f(2.0, 3.0), v2f(10.0, 30.0) / v2f(5.0, 10.0));
        assert_eq!(v2f(2.0, 3.0), f);

        let mut i = v2i(10, 30);
        i /= v2i(5, 10);
        assert_eq!(v2i(2, 3), v2i(10, 30) / v2i(5, 10));
        assert_eq!(v2i(2, 3), i);

        let mut u = v2u(10, 30);
        u /= v2u(5, 10);
        assert_eq!(v2u(2, 3), v2u(10, 30) / v2u(5, 10));
        assert_eq!(v2u(2, 3), u);
    }

    #[test]
    fn vec2_addition() {
        let mut f = v2f(2.0, 3.0);
        f += v2f(5.0, 10.0);
        assert_eq!(v2f(7.0, 13.0), v2f(2.0, 3.0) + v2f(5.0, 10.0));
        assert_eq!(v2f(7.0, 13.0), f);

        let mut i = v2i(2, 3);
        i += v2i(5, 10);
        assert_eq!(v2i(7, 13), v2i(2, 3) + v2i(5, 10));
        assert_eq!(v2i(7, 13), i);

        let mut u = v2u(2, 3);
        u += v2u(5, 10);
        assert_eq!(v2u(7, 13), v2u(2, 3) + v2u(5, 10));
        assert_eq!(v2u(7, 13), u);
    }

    #[test]
    fn vec2_subtraction() {
        let mut f = v2f(5.0, 10.0);
        f -= v2f(2.0, 3.0);
        assert_eq!(v2f(3.0, 7.0), v2f(5.0, 10.0) - v2f(2.0, 3.0));
        assert_eq!(v2f(3.0, 7.0), f);

        let mut i = v2i(5, 10);
        i -= v2i(2, 3);
        assert_eq!(v2i(3, 7), v2i(5, 10) - v2i(2, 3));
        assert_eq!(v2i(3, 7), i);

        let mut u = v2u(5, 10);
        u -= v2u(2, 3);
        assert_eq!(v2u(3, 7), v2u(5, 10) - v2u(2, 3));
        assert_eq!(v2u(3, 7), u);
    }

    #[test]
    fn vec2_negation() {
        assert_eq!(v2f(-2.0, -3.0), -v2f(2.0, 3.0));
        assert_eq!(v2i(-2, -3), -v2i(2, 3));
    }
}
