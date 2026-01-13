//! Paint styles for filling and stroking shapes.
//!
//! This module provides paint types for defining how shapes are rendered.

use crate::types::{Color, CornerRadii, Point, Rect};

/// A paint style for filling shapes.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    /// Solid color fill.
    Solid(Color),
    /// Linear gradient fill.
    LinearGradient(LinearGradient),
    /// Radial gradient fill.
    RadialGradient(RadialGradient),
}

impl Paint {
    /// Create a solid color paint.
    #[inline]
    pub const fn solid(color: Color) -> Self {
        Self::Solid(color)
    }

    /// Create a linear gradient paint.
    #[inline]
    pub fn linear_gradient(start: Point, end: Point, stops: Vec<GradientStop>) -> Self {
        Self::LinearGradient(LinearGradient { start, end, stops })
    }

    /// Create a radial gradient paint.
    #[inline]
    pub fn radial_gradient(
        center: Point,
        radius: f32,
        focus: Option<Point>,
        stops: Vec<GradientStop>,
    ) -> Self {
        Self::RadialGradient(RadialGradient {
            center,
            radius,
            focus,
            stops,
        })
    }

    /// Check if this is a solid color paint.
    #[inline]
    pub fn is_solid(&self) -> bool {
        matches!(self, Self::Solid(_))
    }

    /// Get the solid color, if this is a solid paint.
    #[inline]
    pub fn as_solid(&self) -> Option<Color> {
        match self {
            Self::Solid(c) => Some(*c),
            _ => None,
        }
    }
}

impl From<Color> for Paint {
    fn from(color: Color) -> Self {
        Self::Solid(color)
    }
}

impl Default for Paint {
    fn default() -> Self {
        Self::Solid(Color::BLACK)
    }
}

/// A linear gradient definition.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    /// Start point of the gradient.
    pub start: Point,
    /// End point of the gradient.
    pub end: Point,
    /// Color stops.
    pub stops: Vec<GradientStop>,
}

/// A radial gradient definition.
#[derive(Debug, Clone, PartialEq)]
pub struct RadialGradient {
    /// Center point of the gradient.
    pub center: Point,
    /// Radius of the gradient.
    pub radius: f32,
    /// Optional focal point (defaults to center if None).
    pub focus: Option<Point>,
    /// Color stops.
    pub stops: Vec<GradientStop>,
}

/// A gradient color stop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    /// Position along the gradient (0.0 to 1.0).
    pub offset: f32,
    /// Color at this stop.
    pub color: Color,
}

impl GradientStop {
    /// Create a new gradient stop.
    #[inline]
    pub const fn new(offset: f32, color: Color) -> Self {
        Self { offset, color }
    }
}

/// Stroke style options.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// Stroke paint (color or gradient).
    pub paint: Paint,
    /// Stroke width in pixels.
    pub width: f32,
    /// Line cap style.
    pub cap: LineCap,
    /// Line join style.
    pub join: LineJoin,
    /// Miter limit for miter joins.
    pub miter_limit: f32,
    /// Dash pattern (lengths of dashes and gaps).
    pub dash_pattern: Option<DashPattern>,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            paint: Paint::Solid(Color::BLACK),
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            dash_pattern: None,
        }
    }
}

impl Stroke {
    /// Create a new stroke with the given paint and width.
    #[inline]
    pub fn new(paint: impl Into<Paint>, width: f32) -> Self {
        Self {
            paint: paint.into(),
            width,
            ..Default::default()
        }
    }

    /// Set the line cap style.
    #[inline]
    pub fn with_cap(mut self, cap: LineCap) -> Self {
        self.cap = cap;
        self
    }

    /// Set the line join style.
    #[inline]
    pub fn with_join(mut self, join: LineJoin) -> Self {
        self.join = join;
        self
    }

    /// Set the miter limit.
    #[inline]
    pub fn with_miter_limit(mut self, limit: f32) -> Self {
        self.miter_limit = limit;
        self
    }

    /// Set a dash pattern.
    #[inline]
    pub fn with_dash(mut self, pattern: DashPattern) -> Self {
        self.dash_pattern = Some(pattern);
        self
    }
}

/// Line cap style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    /// Flat cap at the exact endpoint.
    #[default]
    Butt,
    /// Rounded cap extending past the endpoint.
    Round,
    /// Square cap extending past the endpoint.
    Square,
}

/// Line join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    /// Sharp corner (may be limited by miter limit).
    #[default]
    Miter,
    /// Rounded corner.
    Round,
    /// Beveled corner.
    Bevel,
}

/// Dash pattern for stroked lines.
#[derive(Debug, Clone, PartialEq)]
pub struct DashPattern {
    /// Alternating lengths of dashes and gaps.
    pub pattern: Vec<f32>,
    /// Offset into the pattern to start.
    pub offset: f32,
}

impl DashPattern {
    /// Create a new dash pattern.
    #[inline]
    pub fn new(pattern: Vec<f32>, offset: f32) -> Self {
        Self { pattern, offset }
    }

    /// Create a simple dash pattern with equal dash and gap lengths.
    #[inline]
    pub fn simple(dash_length: f32, gap_length: f32) -> Self {
        Self {
            pattern: vec![dash_length, gap_length],
            offset: 0.0,
        }
    }
}

/// Blend mode for compositing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlendMode {
    /// Normal (source-over) blending.
    #[default]
    Normal,
    /// Multiply colors.
    Multiply,
    /// Screen colors.
    Screen,
    /// Overlay.
    Overlay,
    /// Darken (min).
    Darken,
    /// Lighten (max).
    Lighten,
    /// Color dodge.
    ColorDodge,
    /// Color burn.
    ColorBurn,
    /// Hard light.
    HardLight,
    /// Soft light.
    SoftLight,
    /// Difference.
    Difference,
    /// Exclusion.
    Exclusion,
    /// Source (replace destination completely).
    Source,
    /// Destination (keep destination, ignore source).
    Destination,
    /// Source in (source where destination alpha).
    SourceIn,
    /// Destination in.
    DestinationIn,
    /// Source out.
    SourceOut,
    /// Destination out.
    DestinationOut,
    /// Source atop.
    SourceAtop,
    /// Destination atop.
    DestinationAtop,
    /// XOR.
    Xor,
    /// Additive blending.
    Add,
}

/// A box shadow definition (CSS box-shadow model).
///
/// Box shadows are rendered using an analytical approximation of Gaussian blur,
/// providing efficient single-pass shadow rendering on the GPU.
///
/// # Example
///
/// ```ignore
/// // Simple drop shadow
/// let shadow = BoxShadow::new(Color::from_rgba(0.0, 0.0, 0.0, 0.3))
///     .with_offset(4.0, 4.0)
///     .with_blur(8.0);
///
/// // Glow effect (no offset, large blur, spread)
/// let glow = BoxShadow::new(Color::from_rgba(0.0, 0.5, 1.0, 0.5))
///     .with_blur(20.0)
///     .with_spread(4.0);
///
/// // Inset shadow
/// let inset = BoxShadow::new(Color::from_rgba(0.0, 0.0, 0.0, 0.2))
///     .with_offset(2.0, 2.0)
///     .with_blur(4.0)
///     .inset();
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxShadow {
    /// Shadow color (with alpha for transparency).
    pub color: Color,
    /// Horizontal offset in pixels. Positive moves shadow right.
    pub offset_x: f32,
    /// Vertical offset in pixels. Positive moves shadow down.
    pub offset_y: f32,
    /// Blur radius in pixels. Larger values create softer shadows.
    /// The CSS spec defines this as 2 * sigma for Gaussian blur.
    pub blur_radius: f32,
    /// Spread radius in pixels. Expands (positive) or contracts (negative) the shadow shape.
    pub spread_radius: f32,
    /// Whether this is an inset (inner) shadow.
    pub inset: bool,
}

impl BoxShadow {
    /// Create a new box shadow with the given color.
    ///
    /// Default values: no offset, no blur, no spread, outer shadow.
    #[inline]
    pub fn new(color: Color) -> Self {
        Self {
            color,
            offset_x: 0.0,
            offset_y: 0.0,
            blur_radius: 0.0,
            spread_radius: 0.0,
            inset: false,
        }
    }

    /// Create a typical drop shadow with reasonable defaults.
    #[inline]
    pub fn drop_shadow(color: Color, blur: f32) -> Self {
        Self {
            color,
            offset_x: 0.0,
            offset_y: blur * 0.5,
            blur_radius: blur,
            spread_radius: 0.0,
            inset: false,
        }
    }

    /// Set the shadow offset.
    #[inline]
    pub fn with_offset(mut self, x: f32, y: f32) -> Self {
        self.offset_x = x;
        self.offset_y = y;
        self
    }

    /// Set the blur radius.
    #[inline]
    pub fn with_blur(mut self, radius: f32) -> Self {
        self.blur_radius = radius.max(0.0);
        self
    }

    /// Set the spread radius.
    #[inline]
    pub fn with_spread(mut self, radius: f32) -> Self {
        self.spread_radius = radius;
        self
    }

    /// Make this an inset (inner) shadow.
    #[inline]
    pub fn inset(mut self) -> Self {
        self.inset = true;
        self
    }

    /// Set the shadow color.
    #[inline]
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Calculate the expanded bounds needed to render this shadow.
    ///
    /// The shadow may extend beyond the original shape bounds due to
    /// offset, blur, and spread.
    pub fn expanded_bounds(&self, rect: Rect) -> Rect {
        if self.inset {
            // Inset shadows don't expand bounds
            rect
        } else {
            // The shadow extends by blur + spread in each direction, plus offset
            let expand = self.blur_radius + self.spread_radius.max(0.0);
            let left = rect.left() - expand + self.offset_x.min(0.0);
            let top = rect.top() - expand + self.offset_y.min(0.0);
            let right = rect.right() + expand + self.offset_x.max(0.0);
            let bottom = rect.bottom() + expand + self.offset_y.max(0.0);
            Rect::from_corners(Point::new(left, top), Point::new(right, bottom))
        }
    }

    /// Calculate the sigma (standard deviation) for Gaussian blur.
    ///
    /// Per CSS spec, sigma = blur_radius / 2.
    #[inline]
    pub fn sigma(&self) -> f32 {
        (self.blur_radius / 2.0).max(0.001)
    }
}

impl Default for BoxShadow {
    fn default() -> Self {
        Self::new(Color::from_rgba(0.0, 0.0, 0.0, 0.25))
            .with_offset(0.0, 2.0)
            .with_blur(4.0)
    }
}

/// Parameters for rendering a box shadow on a rounded rectangle.
#[derive(Debug, Clone, Copy)]
pub struct BoxShadowParams {
    /// The rectangle to cast the shadow from.
    pub rect: Rect,
    /// Corner radii of the rectangle.
    pub radii: CornerRadii,
    /// The shadow definition.
    pub shadow: BoxShadow,
}

impl BoxShadowParams {
    /// Create shadow parameters for a rectangle.
    #[inline]
    pub fn new(rect: Rect, shadow: BoxShadow) -> Self {
        Self {
            rect,
            radii: CornerRadii::ZERO,
            shadow,
        }
    }

    /// Create shadow parameters for a rounded rectangle.
    #[inline]
    pub fn rounded(rect: Rect, radii: CornerRadii, shadow: BoxShadow) -> Self {
        Self {
            rect,
            radii,
            shadow,
        }
    }

    /// Get the expanded bounds needed to render this shadow.
    #[inline]
    pub fn expanded_bounds(&self) -> Rect {
        self.shadow.expanded_bounds(self.rect)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solid_paint() {
        let p = Paint::solid(Color::RED);
        assert!(p.is_solid());
        assert_eq!(p.as_solid(), Some(Color::RED));
    }

    #[test]
    fn test_gradient_stops() {
        let stops = vec![
            GradientStop::new(0.0, Color::RED),
            GradientStop::new(0.5, Color::GREEN),
            GradientStop::new(1.0, Color::BLUE),
        ];

        let gradient = Paint::linear_gradient(Point::new(0.0, 0.0), Point::new(100.0, 0.0), stops);

        assert!(!gradient.is_solid());
    }

    #[test]
    fn test_stroke_builder() {
        let stroke = Stroke::new(Color::BLUE, 2.0)
            .with_cap(LineCap::Round)
            .with_join(LineJoin::Bevel);

        assert_eq!(stroke.width, 2.0);
        assert_eq!(stroke.cap, LineCap::Round);
        assert_eq!(stroke.join, LineJoin::Bevel);
    }

    #[test]
    fn test_dash_pattern() {
        let dash = DashPattern::simple(5.0, 3.0);
        assert_eq!(dash.pattern, vec![5.0, 3.0]);
        assert_eq!(dash.offset, 0.0);
    }

    #[test]
    fn test_box_shadow_builder() {
        let shadow = BoxShadow::new(Color::BLACK)
            .with_offset(4.0, 4.0)
            .with_blur(8.0)
            .with_spread(2.0);

        assert_eq!(shadow.offset_x, 4.0);
        assert_eq!(shadow.offset_y, 4.0);
        assert_eq!(shadow.blur_radius, 8.0);
        assert_eq!(shadow.spread_radius, 2.0);
        assert!(!shadow.inset);
    }

    #[test]
    fn test_box_shadow_inset() {
        let shadow = BoxShadow::new(Color::BLACK).inset();
        assert!(shadow.inset);
    }

    #[test]
    fn test_box_shadow_drop_shadow() {
        let shadow = BoxShadow::drop_shadow(Color::BLACK, 10.0);
        assert_eq!(shadow.blur_radius, 10.0);
        assert_eq!(shadow.offset_y, 5.0); // half of blur
        assert_eq!(shadow.offset_x, 0.0);
    }

    #[test]
    fn test_box_shadow_expanded_bounds() {
        let rect = Rect::new(100.0, 100.0, 200.0, 100.0);
        let shadow = BoxShadow::new(Color::BLACK)
            .with_offset(10.0, 10.0)
            .with_blur(20.0)
            .with_spread(5.0);

        let bounds = shadow.expanded_bounds(rect);

        // Left should expand by blur+spread (25) but offset doesn't pull left
        assert_eq!(bounds.left(), 100.0 - 25.0);
        // Top should expand similarly
        assert_eq!(bounds.top(), 100.0 - 25.0);
        // Right should expand by blur+spread+offset (25+10=35)
        assert_eq!(bounds.right(), 300.0 + 25.0 + 10.0);
        // Bottom similar
        assert_eq!(bounds.bottom(), 200.0 + 25.0 + 10.0);
    }

    #[test]
    fn test_box_shadow_sigma() {
        let shadow = BoxShadow::new(Color::BLACK).with_blur(10.0);
        assert_eq!(shadow.sigma(), 5.0); // sigma = blur_radius / 2
    }

    #[test]
    fn test_box_shadow_params() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let shadow = BoxShadow::default();
        let params = BoxShadowParams::new(rect, shadow);

        assert_eq!(params.rect, rect);
        assert!(params.radii.is_zero());
    }

    #[test]
    fn test_box_shadow_params_rounded() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let radii = CornerRadii::uniform(10.0);
        let shadow = BoxShadow::default();
        let params = BoxShadowParams::rounded(rect, radii, shadow);

        assert_eq!(params.radii.top_left, 10.0);
    }
}
