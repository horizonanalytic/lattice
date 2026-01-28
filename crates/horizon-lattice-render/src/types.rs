//! Basic geometry and color types for rendering.
//!
//! This module provides fundamental types used throughout the rendering system.

use bytemuck::{Pod, Zeroable};

/// A point in 2D space.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Point {
    /// The x coordinate.
    pub x: f32,
    /// The y coordinate.
    pub y: f32,
}

impl Point {
    /// Create a new point.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The origin point (0, 0).
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Convert to a glam Vec2.
    #[inline]
    pub fn to_vec2(self) -> glam::Vec2 {
        glam::Vec2::new(self.x, self.y)
    }

    /// Create from a glam Vec2.
    #[inline]
    pub fn from_vec2(v: glam::Vec2) -> Self {
        Self { x: v.x, y: v.y }
    }
}

impl From<(f32, f32)> for Point {
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl From<[f32; 2]> for Point {
    fn from([x, y]: [f32; 2]) -> Self {
        Self { x, y }
    }
}

impl From<glam::Vec2> for Point {
    fn from(v: glam::Vec2) -> Self {
        Self::from_vec2(v)
    }
}

/// A size in 2D space (width and height).
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Size {
    /// The width dimension.
    pub width: f32,
    /// The height dimension.
    pub height: f32,
}

impl Size {
    /// Create a new size.
    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    /// Zero size.
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    /// Check if the size has zero area.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

impl From<(f32, f32)> for Size {
    fn from((width, height): (f32, f32)) -> Self {
        Self { width, height }
    }
}

impl From<(u32, u32)> for Size {
    fn from((width, height): (u32, u32)) -> Self {
        Self {
            width: width as f32,
            height: height as f32,
        }
    }
}

/// A rectangle defined by origin and size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// The top-left corner of the rectangle.
    pub origin: Point,
    /// The width and height of the rectangle.
    pub size: Size,
}

impl Rect {
    /// Create a new rectangle from origin and size.
    #[inline]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point { x, y },
            size: Size { width, height },
        }
    }

    /// Create a rectangle from two corners (min and max points).
    #[inline]
    pub fn from_corners(min: Point, max: Point) -> Self {
        Self {
            origin: min,
            size: Size {
                width: max.x - min.x,
                height: max.y - min.y,
            },
        }
    }

    /// Create a rectangle centered at a point.
    #[inline]
    pub fn from_center(center: Point, size: Size) -> Self {
        Self {
            origin: Point {
                x: center.x - size.width / 2.0,
                y: center.y - size.height / 2.0,
            },
            size,
        }
    }

    /// Empty rectangle at origin.
    pub const ZERO: Self = Self {
        origin: Point::ZERO,
        size: Size::ZERO,
    };

    /// Left edge x coordinate.
    #[inline]
    pub fn left(&self) -> f32 {
        self.origin.x
    }

    /// Top edge y coordinate.
    #[inline]
    pub fn top(&self) -> f32 {
        self.origin.y
    }

    /// Right edge x coordinate.
    #[inline]
    pub fn right(&self) -> f32 {
        self.origin.x + self.size.width
    }

    /// Bottom edge y coordinate.
    #[inline]
    pub fn bottom(&self) -> f32 {
        self.origin.y + self.size.height
    }

    /// Width of the rectangle.
    #[inline]
    pub fn width(&self) -> f32 {
        self.size.width
    }

    /// Height of the rectangle.
    #[inline]
    pub fn height(&self) -> f32 {
        self.size.height
    }

    /// Center point of the rectangle.
    #[inline]
    pub fn center(&self) -> Point {
        Point {
            x: self.origin.x + self.size.width / 2.0,
            y: self.origin.y + self.size.height / 2.0,
        }
    }

    /// Top-left corner.
    #[inline]
    pub fn top_left(&self) -> Point {
        self.origin
    }

    /// Top-right corner.
    #[inline]
    pub fn top_right(&self) -> Point {
        Point {
            x: self.right(),
            y: self.top(),
        }
    }

    /// Bottom-left corner.
    #[inline]
    pub fn bottom_left(&self) -> Point {
        Point {
            x: self.left(),
            y: self.bottom(),
        }
    }

    /// Bottom-right corner.
    #[inline]
    pub fn bottom_right(&self) -> Point {
        Point {
            x: self.right(),
            y: self.bottom(),
        }
    }

    /// Check if the rectangle is empty (zero or negative size).
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.size.is_empty()
    }

    /// Check if a point is inside the rectangle.
    #[inline]
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.left()
            && point.x < self.right()
            && point.y >= self.top()
            && point.y < self.bottom()
    }

    /// Compute the intersection of two rectangles.
    pub fn intersect(&self, other: &Rect) -> Option<Rect> {
        let left = self.left().max(other.left());
        let top = self.top().max(other.top());
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());

        if left < right && top < bottom {
            Some(Rect::new(left, top, right - left, bottom - top))
        } else {
            None
        }
    }

    /// Compute the union (bounding box) of two rectangles.
    pub fn union(&self, other: &Rect) -> Rect {
        let left = self.left().min(other.left());
        let top = self.top().min(other.top());
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(left, top, right - left, bottom - top)
    }

    /// Expand the rectangle by the given amount on all sides.
    #[inline]
    pub fn inflate(&self, amount: f32) -> Rect {
        Rect::new(
            self.origin.x - amount,
            self.origin.y - amount,
            self.size.width + amount * 2.0,
            self.size.height + amount * 2.0,
        )
    }

    /// Shrink the rectangle by the given amount on all sides.
    #[inline]
    pub fn deflate(&self, amount: f32) -> Rect {
        self.inflate(-amount)
    }

    /// Offset the rectangle by the given amount.
    #[inline]
    pub fn offset(&self, dx: f32, dy: f32) -> Rect {
        Rect {
            origin: Point {
                x: self.origin.x + dx,
                y: self.origin.y + dy,
            },
            size: self.size,
        }
    }
}

/// A rectangle with rounded corners.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RoundedRect {
    /// The base rectangle.
    pub rect: Rect,
    /// Corner radii (top-left, top-right, bottom-right, bottom-left).
    pub radii: CornerRadii,
}

impl RoundedRect {
    /// Create a rounded rectangle with uniform corner radius.
    #[inline]
    pub fn new(rect: Rect, radius: f32) -> Self {
        Self {
            rect,
            radii: CornerRadii::uniform(radius),
        }
    }

    /// Create a rounded rectangle with per-corner radii.
    #[inline]
    pub fn with_radii(rect: Rect, radii: CornerRadii) -> Self {
        Self { rect, radii }
    }

    /// Check if all corners have zero radius (is a regular rectangle).
    #[inline]
    pub fn is_rect(&self) -> bool {
        self.radii.is_zero()
    }
}

/// Corner radii for rounded rectangles.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct CornerRadii {
    /// Radius for the top-left corner.
    pub top_left: f32,
    /// Radius for the top-right corner.
    pub top_right: f32,
    /// Radius for the bottom-right corner.
    pub bottom_right: f32,
    /// Radius for the bottom-left corner.
    pub bottom_left: f32,
}

impl CornerRadii {
    /// Create corner radii with the same value for all corners.
    #[inline]
    pub const fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    /// Zero radii (sharp corners).
    pub const ZERO: Self = Self::uniform(0.0);

    /// Check if all radii are zero.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.top_left == 0.0
            && self.top_right == 0.0
            && self.bottom_right == 0.0
            && self.bottom_left == 0.0
    }

    /// Get the maximum radius.
    #[inline]
    pub fn max(&self) -> f32 {
        self.top_left
            .max(self.top_right)
            .max(self.bottom_right)
            .max(self.bottom_left)
    }
}

/// An RGBA color with premultiplied alpha.
#[derive(Debug, Clone, Copy, PartialEq, Default, Pod, Zeroable)]
#[repr(C)]
pub struct Color {
    /// Red component (0.0-1.0, premultiplied).
    pub r: f32,
    /// Green component (0.0-1.0, premultiplied).
    pub g: f32,
    /// Blue component (0.0-1.0, premultiplied).
    pub b: f32,
    /// Alpha component (0.0-1.0).
    pub a: f32,
}

impl Color {
    /// Create a new color from RGBA components (0.0-1.0 range).
    ///
    /// Note: This expects premultiplied alpha. Use [`from_rgba`](Self::from_rgba)
    /// for non-premultiplied input.
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a color from non-premultiplied RGBA components.
    #[inline]
    pub fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: r * a,
            g: g * a,
            b: b * a,
            a,
        }
    }

    /// Create a color from 8-bit RGBA components (0-255 range).
    #[inline]
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self::from_rgba(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    /// Create an opaque color from RGB components.
    #[inline]
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create an opaque color from 8-bit RGB components.
    #[inline]
    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgb(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0)
    }

    /// Create a color from a 32-bit RGBA value (0xRRGGBBAA).
    #[inline]
    pub fn from_u32(rgba: u32) -> Self {
        Self::from_rgba8(
            ((rgba >> 24) & 0xFF) as u8,
            ((rgba >> 16) & 0xFF) as u8,
            ((rgba >> 8) & 0xFF) as u8,
            (rgba & 0xFF) as u8,
        )
    }

    /// Create a color from a hex string (e.g., "#FF0000" or "#FF0000FF").
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        let len = hex.len();

        if len != 6 && len != 8 {
            return None;
        }

        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        let a = if len == 8 {
            u8::from_str_radix(&hex[6..8], 16).ok()?
        } else {
            255
        };

        Some(Self::from_rgba8(r, g, b, a))
    }

    /// Return a new color with modified alpha.
    #[inline]
    pub fn with_alpha(self, alpha: f32) -> Self {
        if self.a == 0.0 {
            return Self::new(0.0, 0.0, 0.0, alpha);
        }
        // Unpremultiply, then repremultiply with new alpha
        let factor = alpha / self.a;
        Self {
            r: self.r * factor,
            g: self.g * factor,
            b: self.b * factor,
            a: alpha,
        }
    }

    /// Linear interpolation between two colors.
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Convert to an array [r, g, b, a].
    #[inline]
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert to 8-bit RGBA array.
    #[inline]
    pub fn to_rgba8(self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }

    /// Convert to a 32-bit RGBA value (0xRRGGBBAA).
    #[inline]
    pub fn to_u32(self) -> u32 {
        let [r, g, b, a] = self.to_rgba8();
        ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
    }

    /// Convert to wgpu Color.
    #[inline]
    pub fn to_wgpu(self) -> wgpu::Color {
        wgpu::Color {
            r: self.r as f64,
            g: self.g as f64,
            b: self.b as f64,
            a: self.a as f64,
        }
    }

    /// Fully transparent (invisible) color.
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    /// Pure black.
    pub const BLACK: Self = Self::from_rgb(0.0, 0.0, 0.0);
    /// Pure white.
    pub const WHITE: Self = Self::from_rgb(1.0, 1.0, 1.0);
    /// Pure red.
    pub const RED: Self = Self::from_rgb(1.0, 0.0, 0.0);
    /// Pure green.
    pub const GREEN: Self = Self::from_rgb(0.0, 1.0, 0.0);
    /// Pure blue.
    pub const BLUE: Self = Self::from_rgb(0.0, 0.0, 1.0);
    /// Yellow (red + green).
    pub const YELLOW: Self = Self::from_rgb(1.0, 1.0, 0.0);
    /// Cyan (green + blue).
    pub const CYAN: Self = Self::from_rgb(0.0, 1.0, 1.0);
    /// Magenta (red + blue).
    pub const MAGENTA: Self = Self::from_rgb(1.0, 0.0, 1.0);
    /// Medium gray (50% brightness).
    pub const GRAY: Self = Self::from_rgb(0.5, 0.5, 0.5);
    /// Dark gray (25% brightness).
    pub const DARK_GRAY: Self = Self::from_rgb(0.25, 0.25, 0.25);
    /// Light gray (75% brightness).
    pub const LIGHT_GRAY: Self = Self::from_rgb(0.75, 0.75, 0.75);

    /// Create an opaque color from HSV components.
    ///
    /// - `h`: Hue in degrees (0.0-360.0)
    /// - `s`: Saturation (0.0-1.0)
    /// - `v`: Value/brightness (0.0-1.0)
    pub fn from_hsv(h: f32, s: f32, v: f32) -> Self {
        Self::from_hsva(h, s, v, 1.0)
    }

    /// Create a color from HSVA components.
    ///
    /// - `h`: Hue in degrees (0.0-360.0)
    /// - `s`: Saturation (0.0-1.0)
    /// - `v`: Value/brightness (0.0-1.0)
    /// - `a`: Alpha (0.0-1.0)
    pub fn from_hsva(h: f32, s: f32, v: f32, a: f32) -> Self {
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);
        let h = h.rem_euclid(360.0);

        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Self::from_rgba(r + m, g + m, b + m, a)
    }

    /// Convert this color to HSV components.
    ///
    /// Returns `(h, s, v)` where:
    /// - `h`: Hue in degrees (0.0-360.0)
    /// - `s`: Saturation (0.0-1.0)
    /// - `v`: Value/brightness (0.0-1.0)
    pub fn to_hsv(&self) -> (f32, f32, f32) {
        let (h, s, v, _) = self.to_hsva();
        (h, s, v)
    }

    /// Convert this color to HSVA components.
    ///
    /// Returns `(h, s, v, a)` where:
    /// - `h`: Hue in degrees (0.0-360.0)
    /// - `s`: Saturation (0.0-1.0)
    /// - `v`: Value/brightness (0.0-1.0)
    /// - `a`: Alpha (0.0-1.0)
    pub fn to_hsva(&self) -> (f32, f32, f32, f32) {
        // Unpremultiply alpha to get actual RGB values
        let (r, g, b) = if self.a > 0.0 {
            (self.r / self.a, self.g / self.a, self.b / self.a)
        } else {
            (0.0, 0.0, 0.0)
        };

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;

        let s = if max > 0.0 { delta / max } else { 0.0 };

        let h = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        let h = if h < 0.0 { h + 360.0 } else { h };

        (h, s, v, self.a)
    }

    /// Convert to hex string representation (e.g., "#FF0000" or "#FF000080").
    ///
    /// If alpha is 1.0, returns 6-character hex. Otherwise returns 8-character hex with alpha.
    pub fn to_hex(&self) -> String {
        // Unpremultiply alpha
        let (r, g, b) = if self.a > 0.0 {
            (
                (self.r / self.a * 255.0).round() as u8,
                (self.g / self.a * 255.0).round() as u8,
                (self.b / self.a * 255.0).round() as u8,
            )
        } else {
            (0, 0, 0)
        };
        let a = (self.a * 255.0).round() as u8;

        if a == 255 {
            format!("#{:02X}{:02X}{:02X}", r, g, b)
        } else {
            format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
        }
    }
}

/// A 2D path for complex vector shapes.
///
/// Paths are constructed using a series of commands that describe how to
/// draw lines and curves. They can be filled, stroked, or used as clip regions.
///
/// # Example
///
/// ```ignore
/// let mut path = Path::new();
/// path.move_to(Point::new(0.0, 0.0))
///     .line_to(Point::new(100.0, 0.0))
///     .line_to(Point::new(50.0, 100.0))
///     .close();
///
/// renderer.fill_path(&path, Color::RED, FillRule::NonZero);
/// ```
#[derive(Debug, Clone, Default)]
pub struct Path {
    /// Path commands.
    commands: Vec<PathCommand>,
    /// Cached bounding box (computed lazily).
    bounds: Option<Rect>,
}

/// Commands that make up a path.
#[derive(Debug, Clone, Copy)]
pub enum PathCommand {
    /// Move to a point without drawing.
    MoveTo(Point),
    /// Draw a line to a point.
    LineTo(Point),
    /// Draw a quadratic bezier curve.
    QuadTo {
        /// The control point for the curve.
        control: Point,
        /// The end point of the curve.
        end: Point,
    },
    /// Draw a cubic bezier curve.
    CubicTo {
        /// The first control point.
        control1: Point,
        /// The second control point.
        control2: Point,
        /// The end point of the curve.
        end: Point,
    },
    /// Draw an arc.
    ArcTo {
        /// Radii of the ellipse.
        radii: Point,
        /// X-axis rotation in radians.
        x_rotation: f32,
        /// Whether to use the large arc.
        large_arc: bool,
        /// Whether to sweep clockwise.
        sweep: bool,
        /// End point of the arc.
        end: Point,
    },
    /// Close the current subpath.
    Close,
}

impl Path {
    /// Create a new empty path.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            bounds: None,
        }
    }

    /// Create a new path with pre-allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            commands: Vec::with_capacity(capacity),
            bounds: None,
        }
    }

    /// Move to a point without drawing.
    pub fn move_to(&mut self, p: Point) -> &mut Self {
        self.commands.push(PathCommand::MoveTo(p));
        self.bounds = None; // Invalidate cache
        self
    }

    /// Draw a line to a point.
    pub fn line_to(&mut self, p: Point) -> &mut Self {
        self.commands.push(PathCommand::LineTo(p));
        self.bounds = None;
        self
    }

    /// Draw a quadratic bezier curve.
    pub fn quad_to(&mut self, control: Point, end: Point) -> &mut Self {
        self.commands.push(PathCommand::QuadTo { control, end });
        self.bounds = None;
        self
    }

    /// Draw a cubic bezier curve.
    pub fn cubic_to(&mut self, control1: Point, control2: Point, end: Point) -> &mut Self {
        self.commands.push(PathCommand::CubicTo { control1, control2, end });
        self.bounds = None;
        self
    }

    /// Draw an arc.
    ///
    /// # Arguments
    ///
    /// * `radii` - The x and y radii of the ellipse
    /// * `x_rotation` - Rotation of the ellipse in radians
    /// * `large_arc` - If true, use the larger of the two possible arcs
    /// * `sweep` - If true, sweep in the positive angle direction
    /// * `end` - The endpoint of the arc
    pub fn arc_to(&mut self, radii: Point, x_rotation: f32, large_arc: bool, sweep: bool, end: Point) -> &mut Self {
        self.commands.push(PathCommand::ArcTo {
            radii,
            x_rotation,
            large_arc,
            sweep,
            end,
        });
        self.bounds = None;
        self
    }

    /// Close the current subpath.
    pub fn close(&mut self) -> &mut Self {
        self.commands.push(PathCommand::Close);
        self
    }

    /// Get the path commands.
    pub fn commands(&self) -> &[PathCommand] {
        &self.commands
    }

    /// Check if the path is empty.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Clear all commands from the path.
    pub fn clear(&mut self) {
        self.commands.clear();
        self.bounds = None;
    }

    /// Get the bounding box of the path.
    ///
    /// Returns `None` if the path is empty or contains only MoveTo commands.
    pub fn bounds(&mut self) -> Option<Rect> {
        if self.bounds.is_some() {
            return self.bounds;
        }

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut has_points = false;

        let mut extend_bounds = |p: Point| {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
            has_points = true;
        };

        for cmd in &self.commands {
            match cmd {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => {
                    extend_bounds(*p);
                }
                PathCommand::QuadTo { control, end } => {
                    extend_bounds(*control);
                    extend_bounds(*end);
                }
                PathCommand::CubicTo { control1, control2, end } => {
                    extend_bounds(*control1);
                    extend_bounds(*control2);
                    extend_bounds(*end);
                }
                PathCommand::ArcTo { end, radii, .. } => {
                    // Conservative bounds: include endpoint and extend by radii
                    extend_bounds(*end);
                    extend_bounds(Point::new(end.x - radii.x, end.y - radii.y));
                    extend_bounds(Point::new(end.x + radii.x, end.y + radii.y));
                }
                PathCommand::Close => {}
            }
        }

        if has_points {
            let bounds = Rect::new(min_x, min_y, max_x - min_x, max_y - min_y);
            self.bounds = Some(bounds);
            Some(bounds)
        } else {
            None
        }
    }

    /// Create a rectangle path.
    pub fn rect(rect: Rect) -> Self {
        let mut path = Self::with_capacity(5);
        path.move_to(Point::new(rect.left(), rect.top()))
            .line_to(Point::new(rect.right(), rect.top()))
            .line_to(Point::new(rect.right(), rect.bottom()))
            .line_to(Point::new(rect.left(), rect.bottom()))
            .close();
        path
    }

    /// Create a circle path.
    ///
    /// Uses cubic bezier curves to approximate the circle with high accuracy.
    pub fn circle(center: Point, radius: f32) -> Self {
        Self::ellipse(center, radius, radius)
    }

    /// Create an ellipse path.
    ///
    /// Uses cubic bezier curves to approximate the ellipse.
    pub fn ellipse(center: Point, radius_x: f32, radius_y: f32) -> Self {
        // Approximation constant for bezier control points
        // https://spencermortensen.com/articles/bezier-circle/
        const C: f32 = 0.5519150244935105707435627;

        let mut path = Self::with_capacity(6);

        let cx = radius_x * C;
        let cy = radius_y * C;

        // Start at the rightmost point
        path.move_to(Point::new(center.x + radius_x, center.y));

        // Bottom-right quadrant
        path.cubic_to(
            Point::new(center.x + radius_x, center.y + cy),
            Point::new(center.x + cx, center.y + radius_y),
            Point::new(center.x, center.y + radius_y),
        );

        // Bottom-left quadrant
        path.cubic_to(
            Point::new(center.x - cx, center.y + radius_y),
            Point::new(center.x - radius_x, center.y + cy),
            Point::new(center.x - radius_x, center.y),
        );

        // Top-left quadrant
        path.cubic_to(
            Point::new(center.x - radius_x, center.y - cy),
            Point::new(center.x - cx, center.y - radius_y),
            Point::new(center.x, center.y - radius_y),
        );

        // Top-right quadrant
        path.cubic_to(
            Point::new(center.x + cx, center.y - radius_y),
            Point::new(center.x + radius_x, center.y - cy),
            Point::new(center.x + radius_x, center.y),
        );

        path.close();
        path
    }

    /// Create a rounded rectangle path.
    pub fn rounded_rect(rect: Rect, radii: CornerRadii) -> Self {
        // If no rounding, return simple rect
        if radii.is_zero() {
            return Self::rect(rect);
        }

        let mut path = Self::with_capacity(13);

        let tl = radii.top_left;
        let tr = radii.top_right;
        let br = radii.bottom_right;
        let bl = radii.bottom_left;

        // Start at top-left corner, after the rounded part
        path.move_to(Point::new(rect.left() + tl, rect.top()));

        // Top edge
        path.line_to(Point::new(rect.right() - tr, rect.top()));

        // Top-right corner
        if tr > 0.0 {
            path.quad_to(
                Point::new(rect.right(), rect.top()),
                Point::new(rect.right(), rect.top() + tr),
            );
        }

        // Right edge
        path.line_to(Point::new(rect.right(), rect.bottom() - br));

        // Bottom-right corner
        if br > 0.0 {
            path.quad_to(
                Point::new(rect.right(), rect.bottom()),
                Point::new(rect.right() - br, rect.bottom()),
            );
        }

        // Bottom edge
        path.line_to(Point::new(rect.left() + bl, rect.bottom()));

        // Bottom-left corner
        if bl > 0.0 {
            path.quad_to(
                Point::new(rect.left(), rect.bottom()),
                Point::new(rect.left(), rect.bottom() - bl),
            );
        }

        // Left edge
        path.line_to(Point::new(rect.left(), rect.top() + tl));

        // Top-left corner
        if tl > 0.0 {
            path.quad_to(
                Point::new(rect.left(), rect.top()),
                Point::new(rect.left() + tl, rect.top()),
            );
        }

        path.close();
        path
    }

    /// Create a line path (a single line segment).
    pub fn line(from: Point, to: Point) -> Self {
        let mut path = Self::with_capacity(2);
        path.move_to(from).line_to(to);
        path
    }

    /// Create a polygon path from a list of points.
    pub fn polygon(points: &[Point]) -> Self {
        if points.is_empty() {
            return Self::new();
        }

        let mut path = Self::with_capacity(points.len() + 1);
        path.move_to(points[0]);
        for p in &points[1..] {
            path.line_to(*p);
        }
        path.close();
        path
    }

    /// Create a polyline path from a list of points (not closed).
    pub fn polyline(points: &[Point]) -> Self {
        if points.is_empty() {
            return Self::new();
        }

        let mut path = Self::with_capacity(points.len());
        path.move_to(points[0]);
        for p in &points[1..] {
            path.line_to(*p);
        }
        path
    }

    /// Create a star path.
    pub fn star(center: Point, outer_radius: f32, inner_radius: f32, points: usize) -> Self {
        if points < 2 {
            return Self::new();
        }

        let mut path = Self::with_capacity(points * 2 + 1);
        let angle_step = std::f32::consts::PI / points as f32;
        let start_angle = -std::f32::consts::FRAC_PI_2; // Start at top

        for i in 0..points * 2 {
            let radius = if i % 2 == 0 { outer_radius } else { inner_radius };
            let angle = start_angle + i as f32 * angle_step;
            let p = Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );

            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }

        path.close();
        path
    }

    /// Transform the path by the given transform.
    pub fn transform(&mut self, transform: &crate::transform::Transform2D) {
        for cmd in &mut self.commands {
            match cmd {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => {
                    *p = transform.transform_point(*p);
                }
                PathCommand::QuadTo { control, end } => {
                    *control = transform.transform_point(*control);
                    *end = transform.transform_point(*end);
                }
                PathCommand::CubicTo { control1, control2, end } => {
                    *control1 = transform.transform_point(*control1);
                    *control2 = transform.transform_point(*control2);
                    *end = transform.transform_point(*end);
                }
                PathCommand::ArcTo { end, .. } => {
                    // For arcs, we only transform the endpoint
                    // Full arc transformation would require decomposition
                    *end = transform.transform_point(*end);
                }
                PathCommand::Close => {}
            }
        }
        self.bounds = None;
    }

    /// Create a transformed copy of the path.
    pub fn transformed(&self, transform: &crate::transform::Transform2D) -> Self {
        let mut path = self.clone();
        path.transform(transform);
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_creation() {
        let p = Point::new(1.0, 2.0);
        assert_eq!(p.x, 1.0);
        assert_eq!(p.y, 2.0);

        let p2: Point = (3.0, 4.0).into();
        assert_eq!(p2.x, 3.0);
        assert_eq!(p2.y, 4.0);
    }

    #[test]
    fn test_rect_geometry() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert_eq!(r.left(), 10.0);
        assert_eq!(r.top(), 20.0);
        assert_eq!(r.right(), 110.0);
        assert_eq!(r.bottom(), 70.0);
        assert_eq!(r.width(), 100.0);
        assert_eq!(r.height(), 50.0);
        assert_eq!(r.center(), Point::new(60.0, 45.0));
    }

    #[test]
    fn test_rect_contains() {
        let r = Rect::new(0.0, 0.0, 100.0, 100.0);
        assert!(r.contains(Point::new(50.0, 50.0)));
        assert!(r.contains(Point::new(0.0, 0.0)));
        assert!(!r.contains(Point::new(100.0, 100.0))); // Right/bottom edge is exclusive
        assert!(!r.contains(Point::new(-1.0, 50.0)));
    }

    #[test]
    fn test_rect_intersect() {
        let r1 = Rect::new(0.0, 0.0, 100.0, 100.0);
        let r2 = Rect::new(50.0, 50.0, 100.0, 100.0);

        let intersection = r1.intersect(&r2).unwrap();
        assert_eq!(intersection, Rect::new(50.0, 50.0, 50.0, 50.0));

        let r3 = Rect::new(200.0, 200.0, 50.0, 50.0);
        assert!(r1.intersect(&r3).is_none());
    }

    #[test]
    fn test_color_from_hex() {
        let c = Color::from_hex("#FF0000").unwrap();
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);

        let c2 = Color::from_hex("#00FF0080").unwrap();
        // Premultiplied alpha: g = 1.0 * 0.5 = 0.5
        assert!((c2.g - 0.5).abs() < 0.01);
        assert!((c2.a - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_color_lerp() {
        let black = Color::BLACK;
        let white = Color::WHITE;
        let gray = black.lerp(white, 0.5);
        assert!((gray.r - 0.5).abs() < 0.001);
        assert!((gray.g - 0.5).abs() < 0.001);
        assert!((gray.b - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_rounded_rect() {
        let rr = RoundedRect::new(Rect::new(0.0, 0.0, 100.0, 100.0), 10.0);
        assert_eq!(rr.radii.top_left, 10.0);
        assert!(!rr.is_rect());

        let rr2 = RoundedRect::new(Rect::new(0.0, 0.0, 100.0, 100.0), 0.0);
        assert!(rr2.is_rect());
    }

    #[test]
    fn test_path_creation() {
        let path = Path::new();
        assert!(path.is_empty());
        assert_eq!(path.commands().len(), 0);
    }

    #[test]
    fn test_path_commands() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(100.0, 0.0))
            .line_to(Point::new(100.0, 100.0))
            .close();

        assert!(!path.is_empty());
        assert_eq!(path.commands().len(), 4);

        // Check command types
        assert!(matches!(path.commands()[0], PathCommand::MoveTo(_)));
        assert!(matches!(path.commands()[1], PathCommand::LineTo(_)));
        assert!(matches!(path.commands()[2], PathCommand::LineTo(_)));
        assert!(matches!(path.commands()[3], PathCommand::Close));
    }

    #[test]
    fn test_path_rounded_rect() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let radii = CornerRadii::uniform(10.0);
        let path = Path::rounded_rect(rect, radii);

        // Should have: move + 4 sides + 4 corners + close
        assert!(!path.is_empty());

        // First command should be MoveTo
        assert!(matches!(path.commands()[0], PathCommand::MoveTo(_)));

        // Last command should be Close
        assert!(matches!(path.commands().last(), Some(PathCommand::Close)));
    }

    #[test]
    fn test_color_hsv_red() {
        // Red: H=0, S=1, V=1
        let red = Color::from_hsv(0.0, 1.0, 1.0);
        assert!((red.r - 1.0).abs() < 0.001);
        assert!(red.g.abs() < 0.001);
        assert!(red.b.abs() < 0.001);

        // Round-trip conversion
        let (h, s, v) = red.to_hsv();
        assert!(h.abs() < 0.1 || (h - 360.0).abs() < 0.1); // Hue can be 0 or 360
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_hsv_green() {
        // Green: H=120, S=1, V=1
        let green = Color::from_hsv(120.0, 1.0, 1.0);
        assert!(green.r.abs() < 0.001);
        assert!((green.g - 1.0).abs() < 0.001);
        assert!(green.b.abs() < 0.001);

        let (h, s, v) = green.to_hsv();
        assert!((h - 120.0).abs() < 0.1);
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_hsv_blue() {
        // Blue: H=240, S=1, V=1
        let blue = Color::from_hsv(240.0, 1.0, 1.0);
        assert!(blue.r.abs() < 0.001);
        assert!(blue.g.abs() < 0.001);
        assert!((blue.b - 1.0).abs() < 0.001);

        let (h, s, v) = blue.to_hsv();
        assert!((h - 240.0).abs() < 0.1);
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_hsv_white_black_gray() {
        // White: any H, S=0, V=1
        let white = Color::from_hsv(0.0, 0.0, 1.0);
        assert!((white.r - 1.0).abs() < 0.001);
        assert!((white.g - 1.0).abs() < 0.001);
        assert!((white.b - 1.0).abs() < 0.001);

        // Black: any H, any S, V=0
        let black = Color::from_hsv(0.0, 0.0, 0.0);
        assert!(black.r.abs() < 0.001);
        assert!(black.g.abs() < 0.001);
        assert!(black.b.abs() < 0.001);

        // Gray: any H, S=0, V=0.5
        let gray = Color::from_hsv(0.0, 0.0, 0.5);
        assert!((gray.r - 0.5).abs() < 0.001);
        assert!((gray.g - 0.5).abs() < 0.001);
        assert!((gray.b - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_color_hsv_with_alpha() {
        let color = Color::from_hsva(60.0, 1.0, 1.0, 0.5);
        let (h, s, v, a) = color.to_hsva();
        assert!((h - 60.0).abs() < 0.1);
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);
        assert!((a - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_color_to_hex() {
        let red = Color::from_rgb8(255, 0, 0);
        assert_eq!(red.to_hex(), "#FF0000");

        let green = Color::from_rgb8(0, 255, 0);
        assert_eq!(green.to_hex(), "#00FF00");

        let blue_semi = Color::from_rgba8(0, 0, 255, 128);
        assert_eq!(blue_semi.to_hex(), "#0000FF80");
    }
}
