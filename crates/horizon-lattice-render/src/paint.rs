//! Paint styles for filling and stroking shapes.
//!
//! This module provides paint types for defining how shapes are rendered.

use crate::types::{Color, Point};

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
}
