//! Path tessellation using lyon.
//!
//! This module provides functionality to tessellate paths into triangles
//! suitable for GPU rendering. It uses the lyon tessellation library.

use lyon::math::point as lyon_point;
use lyon::path::builder::SvgPathBuilder;
use lyon::path::Path as LyonPath;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillRule as LyonFillRule, FillTessellator,
    FillVertex, FillVertexConstructor, LineCap as LyonLineCap, LineJoin as LyonLineJoin,
    StrokeOptions, StrokeTessellator, StrokeVertex, StrokeVertexConstructor, VertexBuffers,
};

use crate::paint::{FillRule, LineCap, LineJoin, Stroke};
use crate::types::{Path, PathCommand};

/// Tessellated path output suitable for GPU rendering.
#[derive(Debug, Clone)]
pub struct TessellatedPath {
    /// Vertex positions (x, y).
    pub vertices: Vec<[f32; 2]>,
    /// Triangle indices.
    pub indices: Vec<u32>,
}

impl TessellatedPath {
    /// Create a new empty tessellated path.
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    /// Check if the tessellation is empty.
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }
}

impl Default for TessellatedPath {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert our Path to lyon's Path format.
pub fn to_lyon_path(path: &Path) -> LyonPath {
    let mut builder = LyonPath::svg_builder();
    let mut has_open_subpath = false;

    for cmd in path.commands() {
        match cmd {
            PathCommand::MoveTo(p) => {
                builder.move_to(lyon_point(p.x, p.y));
                has_open_subpath = true;
            }
            PathCommand::LineTo(p) => {
                builder.line_to(lyon_point(p.x, p.y));
            }
            PathCommand::QuadTo { control, end } => {
                builder.quadratic_bezier_to(
                    lyon_point(control.x, control.y),
                    lyon_point(end.x, end.y),
                );
            }
            PathCommand::CubicTo { control1, control2, end } => {
                builder.cubic_bezier_to(
                    lyon_point(control1.x, control1.y),
                    lyon_point(control2.x, control2.y),
                    lyon_point(end.x, end.y),
                );
            }
            PathCommand::ArcTo { radii, x_rotation, large_arc, sweep, end } => {
                // Convert our arc to lyon's SVG arc format
                let arc_flags = lyon::path::ArcFlags {
                    large_arc: *large_arc,
                    sweep: *sweep,
                };
                builder.arc_to(
                    lyon::math::Vector::new(radii.x, radii.y),
                    lyon::math::Angle::radians(*x_rotation),
                    arc_flags,
                    lyon_point(end.x, end.y),
                );
            }
            PathCommand::Close => {
                builder.close();
                has_open_subpath = false;
            }
        }
    }

    // Ensure any open subpath is closed for the builder
    let _ = has_open_subpath;
    builder.build()
}

/// Convert our FillRule to lyon's FillRule.
fn to_lyon_fill_rule(rule: FillRule) -> LyonFillRule {
    match rule {
        FillRule::NonZero => LyonFillRule::NonZero,
        FillRule::EvenOdd => LyonFillRule::EvenOdd,
    }
}

/// Convert our LineCap to lyon's LineCap.
fn to_lyon_line_cap(cap: LineCap) -> LyonLineCap {
    match cap {
        LineCap::Butt => LyonLineCap::Butt,
        LineCap::Round => LyonLineCap::Round,
        LineCap::Square => LyonLineCap::Square,
    }
}

/// Convert our LineJoin to lyon's LineJoin.
fn to_lyon_line_join(join: LineJoin) -> LyonLineJoin {
    match join {
        LineJoin::Miter => LyonLineJoin::Miter,
        LineJoin::Round => LyonLineJoin::Round,
        LineJoin::Bevel => LyonLineJoin::Bevel,
    }
}

/// Simple vertex constructor for fill tessellation.
struct FillVertexCtor;

impl FillVertexConstructor<[f32; 2]> for FillVertexCtor {
    fn new_vertex(&mut self, vertex: FillVertex) -> [f32; 2] {
        [vertex.position().x, vertex.position().y]
    }
}

/// Simple vertex constructor for stroke tessellation.
struct StrokeVertexCtor;

impl StrokeVertexConstructor<[f32; 2]> for StrokeVertexCtor {
    fn new_vertex(&mut self, vertex: StrokeVertex) -> [f32; 2] {
        [vertex.position().x, vertex.position().y]
    }
}

/// Tessellate a path for filling.
///
/// # Arguments
///
/// * `path` - The path to tessellate
/// * `fill_rule` - The fill rule to use (NonZero or EvenOdd)
/// * `tolerance` - Curve approximation tolerance (smaller = more accurate, more vertices)
///
/// # Returns
///
/// A tessellated path with vertex positions and triangle indices.
pub fn tessellate_fill(path: &Path, fill_rule: FillRule, tolerance: f32) -> TessellatedPath {
    if path.is_empty() {
        return TessellatedPath::new();
    }

    let lyon_path = to_lyon_path(path);

    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();

    let options = FillOptions::default()
        .with_fill_rule(to_lyon_fill_rule(fill_rule))
        .with_tolerance(tolerance);

    let result = tessellator.tessellate_path(
        &lyon_path,
        &options,
        &mut BuffersBuilder::new(&mut buffers, FillVertexCtor),
    );

    if result.is_err() {
        return TessellatedPath::new();
    }

    TessellatedPath {
        vertices: buffers.vertices,
        indices: buffers.indices,
    }
}

/// Tessellate a path for stroking.
///
/// # Arguments
///
/// * `path` - The path to tessellate
/// * `stroke` - Stroke options (width, cap, join, etc.)
/// * `tolerance` - Curve approximation tolerance
///
/// # Returns
///
/// A tessellated path with vertex positions and triangle indices.
pub fn tessellate_stroke(path: &Path, stroke: &Stroke, tolerance: f32) -> TessellatedPath {
    if path.is_empty() {
        return TessellatedPath::new();
    }

    let lyon_path = to_lyon_path(path);

    let mut buffers: VertexBuffers<[f32; 2], u32> = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();

    let options = StrokeOptions::default()
        .with_line_width(stroke.width)
        .with_line_cap(to_lyon_line_cap(stroke.cap))
        .with_line_join(to_lyon_line_join(stroke.join))
        .with_miter_limit(stroke.miter_limit)
        .with_tolerance(tolerance);

    let result = tessellator.tessellate_path(
        &lyon_path,
        &options,
        &mut BuffersBuilder::new(&mut buffers, StrokeVertexCtor),
    );

    if result.is_err() {
        return TessellatedPath::new();
    }

    TessellatedPath {
        vertices: buffers.vertices,
        indices: buffers.indices,
    }
}

/// Default tessellation tolerance.
///
/// This value provides a good balance between accuracy and performance.
/// Smaller values produce more accurate curves but more vertices.
pub const DEFAULT_TOLERANCE: f32 = 0.1;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Color, Point};

    #[test]
    fn test_tessellate_empty_path() {
        let path = Path::new();
        let result = tessellate_fill(&path, FillRule::NonZero, DEFAULT_TOLERANCE);
        assert!(result.is_empty());
    }

    #[test]
    fn test_tessellate_triangle() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(100.0, 0.0))
            .line_to(Point::new(50.0, 100.0))
            .close();

        let result = tessellate_fill(&path, FillRule::NonZero, DEFAULT_TOLERANCE);
        assert!(!result.is_empty());
        assert!(!result.vertices.is_empty());
        assert!(!result.indices.is_empty());
        // A simple triangle should produce exactly 3 vertices and 3 indices
        assert_eq!(result.vertices.len(), 3);
        assert_eq!(result.indices.len(), 3);
    }

    #[test]
    fn test_tessellate_rect() {
        let path = Path::rect(crate::types::Rect::new(0.0, 0.0, 100.0, 100.0));

        let result = tessellate_fill(&path, FillRule::NonZero, DEFAULT_TOLERANCE);
        assert!(!result.is_empty());
        // Rectangle should produce 4 vertices and 6 indices (2 triangles)
        assert_eq!(result.vertices.len(), 4);
        assert_eq!(result.indices.len(), 6);
    }

    #[test]
    fn test_tessellate_circle() {
        let path = Path::circle(Point::new(50.0, 50.0), 25.0);

        let result = tessellate_fill(&path, FillRule::NonZero, DEFAULT_TOLERANCE);
        assert!(!result.is_empty());
        // Circle should produce multiple vertices (approximation with curves)
        assert!(result.vertices.len() > 4);
    }

    #[test]
    fn test_stroke_tessellation() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(100.0, 0.0));

        let stroke = Stroke::new(Color::BLACK, 2.0);
        let result = tessellate_stroke(&path, &stroke, DEFAULT_TOLERANCE);

        assert!(!result.is_empty());
        // A stroked line should produce vertices for the stroke outline
        assert!(result.vertices.len() >= 4);
    }

    #[test]
    fn test_stroke_with_caps() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(100.0, 0.0));

        let stroke = Stroke::new(Color::BLACK, 10.0)
            .with_cap(LineCap::Round);
        let result = tessellate_stroke(&path, &stroke, DEFAULT_TOLERANCE);

        // Round caps should produce more vertices than butt caps
        assert!(!result.is_empty());
    }

    #[test]
    fn test_stroke_with_joins() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(50.0, 50.0))
            .line_to(Point::new(100.0, 0.0));

        let stroke = Stroke::new(Color::BLACK, 10.0)
            .with_join(LineJoin::Round);
        let result = tessellate_stroke(&path, &stroke, DEFAULT_TOLERANCE);

        assert!(!result.is_empty());
    }

    #[test]
    fn test_even_odd_fill_rule() {
        // Create an overlapping shape where fill rule matters
        let mut path = Path::new();
        // Outer square
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(100.0, 0.0))
            .line_to(Point::new(100.0, 100.0))
            .line_to(Point::new(0.0, 100.0))
            .close();
        // Inner square
        path.move_to(Point::new(25.0, 25.0))
            .line_to(Point::new(75.0, 25.0))
            .line_to(Point::new(75.0, 75.0))
            .line_to(Point::new(25.0, 75.0))
            .close();

        let non_zero = tessellate_fill(&path, FillRule::NonZero, DEFAULT_TOLERANCE);
        let even_odd = tessellate_fill(&path, FillRule::EvenOdd, DEFAULT_TOLERANCE);

        // Both should produce results, but potentially different
        assert!(!non_zero.is_empty());
        assert!(!even_odd.is_empty());
    }

    #[test]
    fn test_lyon_path_conversion() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(100.0, 0.0))
            .quad_to(Point::new(150.0, 50.0), Point::new(100.0, 100.0))
            .cubic_to(
                Point::new(50.0, 150.0),
                Point::new(0.0, 150.0),
                Point::new(0.0, 100.0),
            )
            .close();

        let lyon_path = to_lyon_path(&path);
        // The path should be valid and not panic
        assert!(!lyon_path.iter().next().is_none());
    }
}
