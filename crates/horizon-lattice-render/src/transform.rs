//! 2D transformations and transform stack.
//!
//! This module provides affine transformations for 2D rendering.

use crate::types::{Point, Rect};

/// A 2D affine transformation matrix.
///
/// Supports translation, rotation, scaling, and skewing operations.
/// Transforms can be composed together and applied to points and rectangles.
///
/// Stored as a 3x2 matrix in column-major order:
/// ```text
/// | m00 m10 m20 |   | scale_x  skew_x   translate_x |
/// | m01 m11 m21 | = | skew_y   scale_y  translate_y |
/// ```
///
/// # Examples
///
/// ## Basic Transforms
///
/// ```
/// use horizon_lattice_render::{Transform2D, Point};
///
/// // Translation
/// let translate = Transform2D::translate(100.0, 50.0);
/// let p = translate.transform_point(Point::new(0.0, 0.0));
/// assert_eq!(p, Point::new(100.0, 50.0));
///
/// // Uniform scaling
/// let scale = Transform2D::scale(2.0);
/// let p = scale.transform_point(Point::new(10.0, 10.0));
/// assert_eq!(p, Point::new(20.0, 20.0));
///
/// // Non-uniform scaling
/// let scale_xy = Transform2D::scale_xy(2.0, 3.0);
/// let p = scale_xy.transform_point(Point::new(10.0, 10.0));
/// assert_eq!(p, Point::new(20.0, 30.0));
///
/// // Rotation (90 degrees)
/// let rotate = Transform2D::rotate(std::f32::consts::FRAC_PI_2);
/// let p = rotate.transform_point(Point::new(1.0, 0.0));
/// assert!((p.x - 0.0).abs() < 0.0001);
/// assert!((p.y - 1.0).abs() < 0.0001);
/// ```
///
/// ## Composing Transforms
///
/// ```
/// use horizon_lattice_render::{Transform2D, Point};
///
/// // Transforms are composed right-to-left with `then`
/// // (first translate, then scale)
/// let transform = Transform2D::scale(2.0)
///     .then(&Transform2D::translate(10.0, 0.0));
///
/// let p = transform.transform_point(Point::new(5.0, 0.0));
/// // 5 + 10 = 15, then * 2 = 30
/// assert_eq!(p, Point::new(30.0, 0.0));
///
/// // Builder-style methods
/// let transform = Transform2D::IDENTITY
///     .translated(100.0, 50.0)
///     .scaled(2.0)
///     .rotated(std::f32::consts::FRAC_PI_4);
/// ```
///
/// ## Rotation Around a Point
///
/// ```
/// use horizon_lattice_render::{Transform2D, Point};
///
/// // Rotate 90 degrees around point (50, 50)
/// let center = Point::new(50.0, 50.0);
/// let rotate = Transform2D::rotate_around(
///     std::f32::consts::FRAC_PI_2,
///     center,
/// );
///
/// // A point at (100, 50) should move to (50, 100)
/// let p = rotate.transform_point(Point::new(100.0, 50.0));
/// assert!((p.x - 50.0).abs() < 0.001);
/// assert!((p.y - 100.0).abs() < 0.001);
/// ```
///
/// ## Inverse Transform
///
/// ```
/// use horizon_lattice_render::{Transform2D, Point};
///
/// let transform = Transform2D::translate(100.0, 50.0)
///     .scaled(2.0);
///
/// // Get the inverse
/// let inverse = transform.inverse().unwrap();
///
/// // Applying transform then inverse returns to original
/// let original = Point::new(25.0, 30.0);
/// let transformed = transform.transform_point(original);
/// let back = inverse.transform_point(transformed);
/// assert!((back.x - original.x).abs() < 0.001);
/// assert!((back.y - original.y).abs() < 0.001);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    /// Matrix elements in column-major order.
    m: [f32; 6],
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform2D {
    /// The identity transform (no transformation).
    pub const IDENTITY: Self = Self {
        m: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
    };

    /// Create a transform from raw matrix elements.
    ///
    /// Elements are in the order: m00, m01, m10, m11, m20, m21
    #[inline]
    pub const fn from_matrix(m00: f32, m01: f32, m10: f32, m11: f32, m20: f32, m21: f32) -> Self {
        Self {
            m: [m00, m01, m10, m11, m20, m21],
        }
    }

    /// Create a translation transform.
    #[inline]
    pub const fn translate(tx: f32, ty: f32) -> Self {
        Self {
            m: [1.0, 0.0, 0.0, 1.0, tx, ty],
        }
    }

    /// Create a uniform scaling transform.
    #[inline]
    pub const fn scale(s: f32) -> Self {
        Self::scale_xy(s, s)
    }

    /// Create a non-uniform scaling transform.
    #[inline]
    pub const fn scale_xy(sx: f32, sy: f32) -> Self {
        Self {
            m: [sx, 0.0, 0.0, sy, 0.0, 0.0],
        }
    }

    /// Create a rotation transform (angle in radians).
    #[inline]
    pub fn rotate(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self {
            m: [cos, sin, -sin, cos, 0.0, 0.0],
        }
    }

    /// Create a rotation transform around a point.
    #[inline]
    pub fn rotate_around(angle: f32, center: Point) -> Self {
        Self::translate(center.x, center.y)
            .then(&Self::rotate(angle))
            .then(&Self::translate(-center.x, -center.y))
    }

    /// Create a skew/shear transform.
    #[inline]
    pub fn skew(skew_x: f32, skew_y: f32) -> Self {
        Self {
            m: [1.0, skew_y.tan(), skew_x.tan(), 1.0, 0.0, 0.0],
        }
    }

    /// Concatenate this transform with another (self * other).
    ///
    /// The resulting transform first applies `other`, then `self`.
    #[inline]
    pub fn then(&self, other: &Self) -> Self {
        let a = &self.m;
        let b = &other.m;
        Self {
            m: [
                a[0] * b[0] + a[2] * b[1],
                a[1] * b[0] + a[3] * b[1],
                a[0] * b[2] + a[2] * b[3],
                a[1] * b[2] + a[3] * b[3],
                a[0] * b[4] + a[2] * b[5] + a[4],
                a[1] * b[4] + a[3] * b[5] + a[5],
            ],
        }
    }

    /// Apply a translation to this transform.
    #[inline]
    pub fn translated(&self, tx: f32, ty: f32) -> Self {
        self.then(&Self::translate(tx, ty))
    }

    /// Apply a scale to this transform.
    #[inline]
    pub fn scaled(&self, s: f32) -> Self {
        self.then(&Self::scale(s))
    }

    /// Apply a non-uniform scale to this transform.
    #[inline]
    pub fn scaled_xy(&self, sx: f32, sy: f32) -> Self {
        self.then(&Self::scale_xy(sx, sy))
    }

    /// Apply a rotation to this transform.
    #[inline]
    pub fn rotated(&self, angle: f32) -> Self {
        self.then(&Self::rotate(angle))
    }

    /// Transform a point.
    #[inline]
    pub fn transform_point(&self, p: Point) -> Point {
        Point {
            x: self.m[0] * p.x + self.m[2] * p.y + self.m[4],
            y: self.m[1] * p.x + self.m[3] * p.y + self.m[5],
        }
    }

    /// Transform a vector (ignores translation).
    #[inline]
    pub fn transform_vector(&self, x: f32, y: f32) -> (f32, f32) {
        (
            self.m[0] * x + self.m[2] * y,
            self.m[1] * x + self.m[3] * y,
        )
    }

    /// Compute the inverse of this transform, if it exists.
    pub fn inverse(&self) -> Option<Self> {
        let det = self.m[0] * self.m[3] - self.m[1] * self.m[2];
        if det.abs() < 1e-10 {
            return None;
        }

        let inv_det = 1.0 / det;
        Some(Self {
            m: [
                self.m[3] * inv_det,
                -self.m[1] * inv_det,
                -self.m[2] * inv_det,
                self.m[0] * inv_det,
                (self.m[2] * self.m[5] - self.m[3] * self.m[4]) * inv_det,
                (self.m[1] * self.m[4] - self.m[0] * self.m[5]) * inv_det,
            ],
        })
    }

    /// Get the translation component.
    #[inline]
    pub fn translation(&self) -> (f32, f32) {
        (self.m[4], self.m[5])
    }

    /// Get the determinant of the transform matrix.
    #[inline]
    pub fn determinant(&self) -> f32 {
        self.m[0] * self.m[3] - self.m[1] * self.m[2]
    }

    /// Check if this is the identity transform.
    #[inline]
    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }

    /// Check if this transform only contains translation.
    #[inline]
    pub fn is_translation_only(&self) -> bool {
        self.m[0] == 1.0 && self.m[1] == 0.0 && self.m[2] == 0.0 && self.m[3] == 1.0
    }

    /// Convert to a column-major 4x4 matrix for GPU use.
    #[inline]
    pub fn to_mat4(&self) -> glam::Mat4 {
        glam::Mat4::from_cols(
            glam::Vec4::new(self.m[0], self.m[1], 0.0, 0.0),
            glam::Vec4::new(self.m[2], self.m[3], 0.0, 0.0),
            glam::Vec4::new(0.0, 0.0, 1.0, 0.0),
            glam::Vec4::new(self.m[4], self.m[5], 0.0, 1.0),
        )
    }

    /// Get the raw matrix elements.
    #[inline]
    pub fn as_array(&self) -> &[f32; 6] {
        &self.m
    }

    /// Transform a rectangle's bounding box.
    ///
    /// Note: This returns the axis-aligned bounding box of the transformed rectangle,
    /// which may be larger than the original if rotation is involved.
    pub fn transform_rect(&self, rect: &Rect) -> Rect {
        let corners = [
            self.transform_point(rect.top_left()),
            self.transform_point(rect.top_right()),
            self.transform_point(rect.bottom_left()),
            self.transform_point(rect.bottom_right()),
        ];

        let min_x = corners.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let min_y = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
        let max_x = corners
            .iter()
            .map(|p| p.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y = corners
            .iter()
            .map(|p| p.y)
            .fold(f32::NEG_INFINITY, f32::max);

        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

/// A stack of transforms for save/restore functionality.
///
/// This allows pushing and popping transforms to manage nested
/// coordinate systems, similar to canvas save/restore.
///
/// # Examples
///
/// ```
/// use horizon_lattice_render::{TransformStack, Point};
///
/// let mut stack = TransformStack::new();
///
/// // Apply some transforms
/// stack.translate(100.0, 50.0);
///
/// // Save current state
/// stack.save();
///
/// // Apply more transforms
/// stack.scale(2.0);
/// stack.rotate(std::f32::consts::FRAC_PI_4);
///
/// // Transform a point with current state
/// let p1 = stack.transform_point(Point::new(10.0, 0.0));
///
/// // Restore to saved state (translation only)
/// stack.restore();
/// let p2 = stack.transform_point(Point::new(10.0, 0.0));
/// assert_eq!(p2, Point::new(110.0, 50.0));
///
/// // Check stack depth
/// assert_eq!(stack.depth(), 0);
/// ```
#[derive(Debug, Clone)]
pub struct TransformStack {
    /// The stack of saved transforms.
    stack: Vec<Transform2D>,
    /// The current accumulated transform.
    current: Transform2D,
}

impl Default for TransformStack {
    fn default() -> Self {
        Self::new()
    }
}

impl TransformStack {
    /// Create a new transform stack with identity transform.
    #[inline]
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            current: Transform2D::IDENTITY,
        }
    }

    /// Get the current transform.
    #[inline]
    pub fn current(&self) -> &Transform2D {
        &self.current
    }

    /// Set the current transform directly.
    #[inline]
    pub fn set(&mut self, transform: Transform2D) {
        self.current = transform;
    }

    /// Reset to identity transform.
    #[inline]
    pub fn reset(&mut self) {
        self.current = Transform2D::IDENTITY;
    }

    /// Save the current transform state.
    #[inline]
    pub fn save(&mut self) {
        self.stack.push(self.current);
    }

    /// Restore the previously saved transform state.
    ///
    /// Does nothing if the stack is empty.
    #[inline]
    pub fn restore(&mut self) {
        if let Some(transform) = self.stack.pop() {
            self.current = transform;
        }
    }

    /// Get the current stack depth.
    #[inline]
    pub fn depth(&self) -> usize {
        self.stack.len()
    }

    /// Apply a transform on top of the current transform.
    #[inline]
    pub fn concat(&mut self, transform: &Transform2D) {
        self.current = self.current.then(transform);
    }

    /// Apply a translation.
    #[inline]
    pub fn translate(&mut self, tx: f32, ty: f32) {
        self.current = self.current.translated(tx, ty);
    }

    /// Apply a uniform scale.
    #[inline]
    pub fn scale(&mut self, s: f32) {
        self.current = self.current.scaled(s);
    }

    /// Apply a non-uniform scale.
    #[inline]
    pub fn scale_xy(&mut self, sx: f32, sy: f32) {
        self.current = self.current.scaled_xy(sx, sy);
    }

    /// Apply a rotation.
    #[inline]
    pub fn rotate(&mut self, angle: f32) {
        self.current = self.current.rotated(angle);
    }

    /// Transform a point using the current transform.
    #[inline]
    pub fn transform_point(&self, p: Point) -> Point {
        self.current.transform_point(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    const EPSILON: f32 = 1e-5;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_identity() {
        let t = Transform2D::IDENTITY;
        let p = Point::new(5.0, 10.0);
        let tp = t.transform_point(p);
        assert_eq!(tp, p);
    }

    #[test]
    fn test_translation() {
        let t = Transform2D::translate(10.0, 20.0);
        let p = Point::new(5.0, 5.0);
        let tp = t.transform_point(p);
        assert_eq!(tp, Point::new(15.0, 25.0));
    }

    #[test]
    fn test_scale() {
        let t = Transform2D::scale_xy(2.0, 3.0);
        let p = Point::new(5.0, 10.0);
        let tp = t.transform_point(p);
        assert_eq!(tp, Point::new(10.0, 30.0));
    }

    #[test]
    fn test_rotation() {
        let t = Transform2D::rotate(PI / 2.0); // 90 degrees
        let p = Point::new(1.0, 0.0);
        let tp = t.transform_point(p);
        assert!(approx_eq(tp.x, 0.0));
        assert!(approx_eq(tp.y, 1.0));
    }

    #[test]
    fn test_concatenation() {
        // First translate, then scale
        let t1 = Transform2D::translate(10.0, 0.0);
        let t2 = Transform2D::scale(2.0);
        let combined = t2.then(&t1);

        let p = Point::new(5.0, 0.0);
        let tp = combined.transform_point(p);
        // translate(5, 0) -> (15, 0), then scale -> (30, 0)
        assert_eq!(tp, Point::new(30.0, 0.0));
    }

    #[test]
    fn test_inverse() {
        let t = Transform2D::translate(10.0, 20.0)
            .scaled(2.0)
            .rotated(PI / 4.0);

        let inv = t.inverse().unwrap();
        let combined = t.then(&inv);

        let p = Point::new(100.0, 50.0);
        let tp = combined.transform_point(p);
        assert!(approx_eq(tp.x, p.x));
        assert!(approx_eq(tp.y, p.y));
    }

    #[test]
    fn test_transform_stack() {
        let mut stack = TransformStack::new();

        stack.translate(10.0, 20.0);
        stack.save();
        stack.scale(2.0);

        let p = Point::new(5.0, 5.0);
        let tp = stack.transform_point(p);
        assert_eq!(tp, Point::new(20.0, 30.0)); // (5+10)*2 = 30, (5+20)*2 = 50? No...
                                                 // Actually: scale first (5*2=10, 5*2=10), then translate (10+10=20, 10+20=30)

        stack.restore();
        let tp2 = stack.transform_point(p);
        assert_eq!(tp2, Point::new(15.0, 25.0)); // Just translation
    }

    #[test]
    fn test_transform_rect() {
        let t = Transform2D::translate(10.0, 10.0);
        let r = Rect::new(0.0, 0.0, 100.0, 50.0);
        let tr = t.transform_rect(&r);
        assert_eq!(tr, Rect::new(10.0, 10.0, 100.0, 50.0));
    }
}
