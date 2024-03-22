use std::{
    fmt::{Debug, Display},
    ops::{Mul, MulAssign},
};

use crate::{v2, vec2::Vec2};

#[derive(Clone, Copy, PartialEq)]
pub struct Mat4 {
    // Col 0
    pub m00: f32,
    pub m10: f32,
    pub m20: f32,
    pub m30: f32,

    // Col 1
    pub m01: f32,
    pub m11: f32,
    pub m21: f32,
    pub m31: f32,

    // Col 2
    pub m02: f32,
    pub m12: f32,
    pub m22: f32,
    pub m23: f32,

    // Col 3
    pub m03: f32,
    pub m13: f32,
    pub m32: f32,
    pub m33: f32,
}

impl Default for Mat4 {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Mat4 {
    /// A 4x4 matrix set to the identity value.
    #[rustfmt::skip]
    pub const IDENTITY: Self =  Self::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 1.0, 0.0,
        0.0, 0.0, 0.0, 1.0,
    );

    /// A 4x4 matrix with all values set to zero.
    #[rustfmt::skip]
    pub const ZERO: Self =  Self::new(
        0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0,
        0.0, 0.0, 0.0, 0.0,
    );

    /// Creates a new 4x4 matrix.
    #[inline(always)]
    #[rustfmt::skip]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        m00: f32, m01: f32, m02: f32, m03: f32,
        m10: f32, m11: f32, m12: f32, m13: f32,
        m20: f32, m21: f32, m22: f32, m23: f32,
        m30: f32, m31: f32, m32: f32, m33: f32,
    ) -> Self {
        Self {
            m00, m01, m02, m03,
            m10, m11, m12, m13,
            m20, m21, m22, m23,
            m30, m31, m32, m33,
        }
    }

    /// Creates a new 4x4 orthographic projection matrix.
    #[inline(always)]
    #[rustfmt::skip]
    pub fn orthographic(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        // This is a right handed projection based on
        // http://learnwebgl.brown37.net/08_projections/projections_ortho.html#the-orthographic-projection-matrix
        // but for wgpu we need to restrict the depth range to 0 to 1 rather than -1 to 1 which is used
        // in OpenGL (and I couldn't find any explanations of). GLM has a compiler define of
        // `GLM_FORCE_DEPTH_ZERO_TO_ONE` to enable this range; glam has a separate function for
        // calculating the orthographic matrix depending on depth range (see `Mat4::orthographic_rh` and
        // `Mat4::orthographic_rh_gl`).

        let a = 2.0 / (right - left);
        let b = 2.0 / (top - bottom);
        let c = 1.0 / (near - far);
        let tx = (left + right) / (right - left);
        let ty = (top + bottom) / (top - bottom);
        let tz = near / (near - far);

        Self::new (
            a,   0.0,  0.0, -tx,
            0.0, b,    0.0, -ty,
            0.0, 0.0, c,   tz,
            0.0, 0.0,  0.0,  1.0,
        )

    }

    /// Creates a new 4x4 matrix from the given translation, rotation and scale values.
    #[inline(always)]
    pub fn translation_rotation_scale(translation: Vec2, rotation: f32, scale: Vec2) -> Self {
        Mat4::translation(translation) * Mat4::rotation(rotation) * Mat4::scale(scale)
    }

    /// Creates a new 4x4 transformation matrix for the given translation.
    #[inline(always)]
    #[rustfmt::skip]
    pub fn translation(v: Vec2) -> Self {
        // See https://www.brainvoyager.com/bv/doc/UsersGuide/CoordsAndTransforms/SpatialTransformationMatrices.html

        Self::new(
            1.0, 0.0, 0.0, v.x,
            0.0, 1.0, 0.0, v.y,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// Creates a new 4x4 transformation matrix for the given rotation.
    #[inline(always)]
    #[rustfmt::skip]
    pub fn rotation(r: f32) -> Self {
        // See https://www.brainvoyager.com/bv/doc/UsersGuide/CoordsAndTransforms/SpatialTransformationMatrices.html
        // Rotation about z-axis.

        let cos = r.cos();
        let sin = r.sin();

        Self::new(
            cos, -sin, 0.0, 0.0,
            sin,  cos, 0.0, 0.0,
            0.0,  0.0, 1.0, 0.0,
            0.0,  0.0, 0.0, 1.0,
        )
    }

    /// Creates a new 4x4 transformation matrix for the given rotation relative to the origin.
    #[inline(always)]
    pub fn rotation_origin(r: f32, origin: Vec2) -> Self {
        Self::translation(origin) * Self::rotation(r) * Self::translation(-origin)
    }

    /// Creates a new 4x4 transformation matrix for the given scale.
    #[inline(always)]
    #[rustfmt::skip]
    pub fn scale(v: Vec2) -> Self {
        // See https://www.brainvoyager.com/bv/doc/UsersGuide/CoordsAndTransforms/SpatialTransformationMatrices.html

        Self::new(
            v.x, 0.0, 0.0, 0.0,
            0.0, v.y, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        )
    }

    /// Calculates the inverse matrix. Returns the identity matrix if the determinant is zero.
    #[rustfmt::skip]
    pub fn inverse(&self) -> Self {
        let det = self.m00 * (self.m33 * self.m11 - self.m31 * self.m13) -
                  self.m10 * (self.m33 * self.m01 - self.m31 * self.m03) +
                  self.m30 * (self.m13 * self.m01 - self.m11 * self.m03);

        if det == 0.0 {
            return Self::IDENTITY;
        }

        let m00 =  (self.m33 * self.m11 - self.m31 * self.m13) / det;
        let m01 = -(self.m33 * self.m01 - self.m31 * self.m03) / det;
        let m03 =  (self.m13 * self.m01 - self.m11 * self.m03) / det;
        let m10 = -(self.m33 * self.m10 - self.m30 * self.m13) / det;
        let m11 =  (self.m33 * self.m00 - self.m30 * self.m03) / det;
        let m13 = -(self.m13 * self.m00 - self.m10 * self.m03) / det;
        let m30 =  (self.m31 * self.m10 - self.m30 * self.m11) / det;
        let m31 = -(self.m31 * self.m00 - self.m30 * self.m01) / det;
        let m33 =  (self.m11 * self.m00 - self.m10 * self.m01) / det;

        Self::new(
            m00, m01, 0.0, m03,
            m10, m11, 0.0, m13,
            0.0, 0.0, 1.0, 0.0,
            m30, m31, 0.0, m33,
        )
    }

    /// Creates an `[f32; 16]` array in column-major order.
    #[inline(always)]
    #[rustfmt::skip]
    pub const fn to_cols_array(&self) -> [f32; 16] {
        [
            self.m00, self.m10, self.m20, self.m30,
            self.m01, self.m11, self.m21, self.m31,
            self.m02, self.m12, self.m22, self.m32,
            self.m03, self.m13, self.m23, self.m33,
        ]
    }
}

impl Mul for Mat4 {
    type Output = Mat4;

    #[rustfmt::skip]
    fn mul(self, rhs: Self) -> Self::Output {
        fn dot(u: &[f32; 4], v: &[f32; 4]) -> f32 {
            u[0] * v[0] + u[1] * v[1] + u[2] * v[2] + u[3] * v[3]
        }

        let r0 = [self.m00, self.m01, self.m02,  self.m03];
        let r1 = [self.m10, self.m11, self.m12,  self.m13];
        let r2 = [self.m20, self.m21, self.m22,  self.m23];
        let r3 = [self.m30, self.m31, self.m32,  self.m33];

        let c0 = [rhs.m00, rhs.m10, rhs.m20, rhs.m30];
        let c1 = [rhs.m01, rhs.m11, rhs.m21, rhs.m31];
        let c2 = [rhs.m02, rhs.m12, rhs.m22, rhs.m32];
        let c3 = [rhs.m03, rhs.m13, rhs.m23, rhs.m33];

        Self::new(
            dot(&r0, &c0), dot(&r0, &c1), dot(&r0, &c2), dot(&r0, &c3),
            dot(&r1, &c0), dot(&r1, &c1), dot(&r1, &c2), dot(&r1, &c3),
            dot(&r2, &c0), dot(&r2, &c1), dot(&r2, &c2), dot(&r2, &c3),
            dot(&r3, &c0), dot(&r3, &c1), dot(&r3, &c2), dot(&r3, &c3),
        )
    }
}

impl MulAssign for Mat4 {
    fn mul_assign(&mut self, rhs: Self) {
        *self = self.mul(rhs);
    }
}

impl Mul<Vec2> for Mat4 {
    type Output = Vec2;

    fn mul(self, rhs: Vec2) -> Self::Output {
        let x = self.m00 * rhs.x + self.m01 * rhs.y + self.m03;
        let y = self.m10 * rhs.x + self.m11 * rhs.y + self.m13;

        v2(x, y)
    }
}

impl Debug for Mat4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mat4")
            .field("x0", &self.m00)
            .field("x1", &self.m10)
            .field("x2", &self.m20)
            .field("x3", &self.m30)
            .field("y0", &self.m01)
            .field("y1", &self.m11)
            .field("y2", &self.m21)
            .field("y3", &self.m31)
            .field("z0", &self.m02)
            .field("z1", &self.m12)
            .field("z2", &self.m22)
            .field("z3", &self.m32)
            .field("w0", &self.m03)
            .field("w1", &self.m13)
            .field("w2", &self.m23)
            .field("w3", &self.m33)
            .finish()
    }
}

impl Display for Mat4 {
    #[rustfmt::skip]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}, {}, {},\n\
                    {}, {}, {}, {},\n\
                    {}, {}, {}, {},\n\
                    {}, {}, {}, {}]",
                self.m00, self.m01, self.m02, self.m03,
                self.m10, self.m11, self.m12, self.m13,
                self.m20, self.m21, self.m22, self.m23,
                self.m30, self.m31, self.m32, self.m33,
            )
    }
}

#[cfg(test)]
mod test {
    use crate::vec2::v2;

    use super::*;

    #[test]
    fn mat4_identity() {
        assert_eq!(Mat4::IDENTITY, glam::Mat4::IDENTITY);
    }

    #[test]
    fn mat4_default_is_identity() {
        assert_eq!(Mat4::IDENTITY, Mat4::default());
    }

    #[test]
    fn mat4_ortho() {
        assert_eq!(
            Mat4::orthographic(0.0, 1280.0, 720.0, 0.0, 100.0, 0.0),
            glam::Mat4::orthographic_rh(0.0, 1280.0, 720.0, 0.0, 100.0, 0.0)
        );
    }

    #[test]
    fn mat4_translation() {
        let tx = v2(50.0, 100.0);

        assert_eq!(
            Mat4::translation(tx),
            glam::Mat4::from_translation(glam::Vec3::new(tx.x, tx.y, 0.0))
        );
    }

    #[test]
    fn mat4_rotation() {
        let r = 90.0_f32.to_radians();

        assert_eq!(Mat4::rotation(r), glam::Mat4::from_rotation_z(r));
    }

    #[test]
    fn mat4_scale() {
        let s = v2(2.0, 0.5);

        assert_eq!(
            Mat4::scale(s),
            glam::Mat4::from_scale(glam::Vec3::new(s.x, s.y, 1.0))
        );
    }

    #[test]
    fn mat4_translation_rotation_scale() {
        let tx = v2(50.0, 100.0);
        let r = 90.0_f32.to_radians();
        let s = v2(2.0, 0.5);

        let g_tx = glam::Mat4::from_translation(glam::Vec3::new(tx.x, tx.y, 0.0));
        let g_r = glam::Mat4::from_rotation_z(r);
        let g_s = glam::Mat4::from_scale(glam::Vec3::new(s.x, s.y, 1.0));

        assert_eq!(Mat4::translation_rotation_scale(tx, r, s), g_tx * g_r * g_s);
    }

    #[test]
    fn mat4_inverse() {
        let tx = v2(50.0, 100.0);
        let r = 90.0_f32.to_radians();
        let s = v2(2.0, 0.5);

        let g_tx = glam::Mat4::from_translation(glam::Vec3::new(tx.x, tx.y, 0.0));
        let g_r = glam::Mat4::from_rotation_z(r);
        let g_s = glam::Mat4::from_scale(glam::Vec3::new(s.x, s.y, 1.0));
        let g_m4 = g_tx * g_r * g_s;

        assert_eq!(
            Mat4::translation_rotation_scale(tx, r, s).inverse(),
            g_m4.inverse()
        );
    }

    #[test]
    fn mat4_cols_array() {
        let tx = v2(50.0, 100.0);
        let r = 90.0_f32.to_radians();
        let s = v2(2.0, 0.5);

        let g_tx = glam::Mat4::from_translation(glam::Vec3::new(tx.x, tx.y, 0.0));
        let g_r = glam::Mat4::from_rotation_z(r);
        let g_s = glam::Mat4::from_scale(glam::Vec3::new(s.x, s.y, 1.0));
        let g_m4 = g_tx * g_r * g_s;

        assert_eq!(
            Mat4::translation_rotation_scale(tx, r, s).to_cols_array(),
            g_m4.to_cols_array()
        );
    }

    #[test]
    fn mat4_mul_vec2() {
        let p = v2(30.0, 90.0);
        let tx = v2(50.0, 100.0);
        let r = 90.0_f32.to_radians();
        let s = v2(2.0, 0.5);
        let m = Mat4::translation_rotation_scale(tx, r, s);
        let v = m * p;

        let g_tx = glam::Mat4::from_translation(glam::Vec3::new(tx.x, tx.y, 0.0));
        let g_r = glam::Mat4::from_rotation_z(r);
        let g_s = glam::Mat4::from_scale(glam::Vec3::new(s.x, s.y, 1.0));
        let g_m4 = g_tx * g_r * g_s;
        let g_v = g_m4 * glam::Vec4::new(p.x, p.y, 0.0, 0.0);

        assert_eq!(m.to_cols_array(), g_m4.to_cols_array());
        assert_eq!(g_v, glam::Vec4::new(v.x, v.y, 0.0, 0.0));
    }

    impl PartialEq<glam::Mat4> for Mat4 {
        fn eq(&self, other: &glam::Mat4) -> bool {
            let m = self;
            let o = other.to_cols_array();

            m.m00 - o[0] == 0.0
                && m.m10 - o[1] == 0.0
                && m.m20 - o[2] == 0.0
                && m.m30 - o[3] == 0.0
                && m.m01 - o[4] == 0.0
                && m.m11 - o[5] == 0.0
                && m.m21 - o[6] == 0.0
                && m.m31 - o[7] == 0.0
                && m.m02 - o[8] == 0.0
                && m.m12 - o[9] == 0.0
                && m.m22 - o[10] == 0.0
                && m.m32 - o[11] == 0.0
                && m.m03 - o[12] == 0.0
                && m.m13 - o[13] == 0.0
                && m.m23 - o[14] == 0.0
                && m.m33 - o[15] == 0.0
        }
    }
}
