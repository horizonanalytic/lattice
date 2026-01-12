//! Core renderer trait defining the 2D drawing interface.
//!
//! This module defines the [`Renderer`] trait which provides a high-level API
//! for 2D drawing operations. Implementations can use immediate or retained-mode
//! rendering backends.

use crate::paint::{BlendMode, Paint, Stroke};
use crate::transform::{Transform2D, TransformStack};
use crate::types::{Color, Point, Rect, RoundedRect, Size};

/// Statistics from a frame render.
#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    /// Number of draw calls submitted.
    pub draw_calls: u32,
    /// Number of vertices rendered.
    pub vertices: u32,
    /// Number of state changes (shader, blend mode, etc.).
    pub state_changes: u32,
}

/// The core 2D rendering trait.
///
/// This trait defines the interface for all 2D drawing operations. Implementations
/// can use various backends (wgpu, software rendering, etc.) and architectures
/// (immediate-mode, retained-mode with scene graph, etc.).
///
/// # Frame Lifecycle
///
/// A typical frame looks like:
///
/// ```ignore
/// renderer.begin_frame(clear_color, viewport_size);
///
/// renderer.save();
/// renderer.translate(10.0, 10.0);
/// renderer.fill_rect(rect, Color::RED);
/// renderer.restore();
///
/// let stats = renderer.end_frame();
/// ```
///
/// # State Stack
///
/// The renderer maintains a state stack that can be saved and restored.
/// This includes transforms and clip regions.
pub trait Renderer {
    /// Begin a new frame.
    ///
    /// This must be called before any drawing operations. The frame will be
    /// cleared to the specified color.
    ///
    /// # Arguments
    ///
    /// * `clear_color` - The color to clear the frame to.
    /// * `viewport_size` - The size of the render target in pixels.
    fn begin_frame(&mut self, clear_color: Color, viewport_size: Size);

    /// End the current frame and present it.
    ///
    /// Returns statistics about the frame that was rendered.
    fn end_frame(&mut self) -> FrameStats;

    // =========================================================================
    // State Management
    // =========================================================================

    /// Save the current render state (transform, clip, etc.).
    fn save(&mut self);

    /// Restore the previously saved render state.
    fn restore(&mut self);

    /// Reset all state to defaults.
    fn reset(&mut self);

    // =========================================================================
    // Transform Operations
    // =========================================================================

    /// Get the current transform.
    fn transform(&self) -> &Transform2D;

    /// Set the current transform directly.
    fn set_transform(&mut self, transform: Transform2D);

    /// Concatenate a transform with the current transform.
    fn concat_transform(&mut self, transform: &Transform2D);

    /// Apply a translation to the current transform.
    fn translate(&mut self, tx: f32, ty: f32);

    /// Apply a scale to the current transform.
    fn scale(&mut self, sx: f32, sy: f32);

    /// Apply a rotation to the current transform (angle in radians).
    fn rotate(&mut self, angle: f32);

    // =========================================================================
    // Clipping
    // =========================================================================

    /// Set a rectangular clip region.
    ///
    /// Drawing will be clipped to this rectangle (intersected with any
    /// existing clip).
    fn clip_rect(&mut self, rect: Rect);

    /// Get the current clip bounds, if any.
    fn clip_bounds(&self) -> Option<Rect>;

    // =========================================================================
    // Drawing - Rectangles
    // =========================================================================

    /// Fill a rectangle with the specified paint.
    fn fill_rect(&mut self, rect: Rect, paint: impl Into<Paint>);

    /// Fill a rounded rectangle with the specified paint.
    fn fill_rounded_rect(&mut self, rect: RoundedRect, paint: impl Into<Paint>);

    /// Stroke the outline of a rectangle.
    fn stroke_rect(&mut self, rect: Rect, stroke: &Stroke);

    /// Stroke the outline of a rounded rectangle.
    fn stroke_rounded_rect(&mut self, rect: RoundedRect, stroke: &Stroke);

    // =========================================================================
    // Drawing - Lines
    // =========================================================================

    /// Draw a line between two points.
    fn draw_line(&mut self, from: Point, to: Point, stroke: &Stroke);

    /// Draw a polyline (connected line segments).
    fn draw_polyline(&mut self, points: &[Point], stroke: &Stroke);

    // =========================================================================
    // Drawing - Other
    // =========================================================================

    /// Fill an ellipse.
    fn fill_ellipse(&mut self, center: Point, radius_x: f32, radius_y: f32, paint: impl Into<Paint>);

    /// Stroke an ellipse.
    fn stroke_ellipse(&mut self, center: Point, radius_x: f32, radius_y: f32, stroke: &Stroke);

    /// Fill a circle (convenience method for fill_ellipse).
    #[inline]
    fn fill_circle(&mut self, center: Point, radius: f32, paint: impl Into<Paint>) {
        self.fill_ellipse(center, radius, radius, paint);
    }

    /// Stroke a circle (convenience method for stroke_ellipse).
    #[inline]
    fn stroke_circle(&mut self, center: Point, radius: f32, stroke: &Stroke) {
        self.stroke_ellipse(center, radius, radius, stroke);
    }

    // =========================================================================
    // Blend Mode
    // =========================================================================

    /// Set the blend mode for subsequent drawing operations.
    fn set_blend_mode(&mut self, mode: BlendMode);

    /// Get the current blend mode.
    fn blend_mode(&self) -> BlendMode;

    // =========================================================================
    // Opacity
    // =========================================================================

    /// Set the global opacity for subsequent drawing operations.
    ///
    /// This is multiplied with paint colors.
    fn set_opacity(&mut self, opacity: f32);

    /// Get the current global opacity.
    fn opacity(&self) -> f32;
}

/// Saved renderer state for save/restore operations.
#[derive(Debug, Clone)]
pub struct RenderState {
    /// Transform at this state.
    pub transform: Transform2D,
    /// Clip rect at this state.
    pub clip: Option<Rect>,
    /// Blend mode at this state.
    pub blend_mode: BlendMode,
    /// Opacity at this state.
    pub opacity: f32,
}

impl Default for RenderState {
    fn default() -> Self {
        Self {
            transform: Transform2D::IDENTITY,
            clip: None,
            blend_mode: BlendMode::Normal,
            opacity: 1.0,
        }
    }
}

/// Common state management for renderers.
///
/// This struct provides a reusable implementation of state management
/// (save/restore, transforms, clips) that renderer implementations can use.
#[derive(Debug, Clone)]
pub struct RenderStateStack {
    /// Stack of saved states.
    stack: Vec<RenderState>,
    /// Current state.
    current: RenderState,
    /// Transform stack (for convenience).
    transform_stack: TransformStack,
}

impl Default for RenderStateStack {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderStateStack {
    /// Create a new state stack with default state.
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            current: RenderState::default(),
            transform_stack: TransformStack::new(),
        }
    }

    /// Get the current state.
    #[inline]
    pub fn current(&self) -> &RenderState {
        &self.current
    }

    /// Get mutable access to the current state.
    #[inline]
    pub fn current_mut(&mut self) -> &mut RenderState {
        &mut self.current
    }

    /// Save the current state.
    pub fn save(&mut self) {
        self.stack.push(self.current.clone());
        self.transform_stack.save();
    }

    /// Restore the previously saved state.
    pub fn restore(&mut self) {
        if let Some(state) = self.stack.pop() {
            self.current = state;
            self.transform_stack.restore();
        }
    }

    /// Reset to default state and clear the stack.
    pub fn reset(&mut self) {
        self.stack.clear();
        self.current = RenderState::default();
        self.transform_stack = TransformStack::new();
    }

    /// Get the current transform.
    #[inline]
    pub fn transform(&self) -> &Transform2D {
        self.transform_stack.current()
    }

    /// Set the current transform.
    #[inline]
    pub fn set_transform(&mut self, transform: Transform2D) {
        self.transform_stack.set(transform);
        self.current.transform = transform;
    }

    /// Concatenate a transform with the current transform.
    #[inline]
    pub fn concat_transform(&mut self, transform: &Transform2D) {
        self.transform_stack.concat(transform);
        self.current.transform = *self.transform_stack.current();
    }

    /// Apply a translation.
    #[inline]
    pub fn translate(&mut self, tx: f32, ty: f32) {
        self.transform_stack.translate(tx, ty);
        self.current.transform = *self.transform_stack.current();
    }

    /// Apply a scale.
    #[inline]
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.transform_stack.scale_xy(sx, sy);
        self.current.transform = *self.transform_stack.current();
    }

    /// Apply a rotation.
    #[inline]
    pub fn rotate(&mut self, angle: f32) {
        self.transform_stack.rotate(angle);
        self.current.transform = *self.transform_stack.current();
    }

    /// Set a clip rect, intersecting with any existing clip.
    pub fn clip_rect(&mut self, rect: Rect) {
        // Transform the clip rect
        let transformed = self.transform_stack.current().transform_rect(&rect);

        self.current.clip = match self.current.clip {
            Some(existing) => existing.intersect(&transformed),
            None => Some(transformed),
        };
    }

    /// Get the current clip bounds.
    #[inline]
    pub fn clip_bounds(&self) -> Option<Rect> {
        self.current.clip
    }

    /// Get the stack depth.
    #[inline]
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_state_default() {
        let state = RenderState::default();
        assert!(state.transform.is_identity());
        assert!(state.clip.is_none());
        assert_eq!(state.blend_mode, BlendMode::Normal);
        assert_eq!(state.opacity, 1.0);
    }

    #[test]
    fn test_render_state_stack() {
        let mut stack = RenderStateStack::new();

        stack.translate(10.0, 20.0);
        stack.save();
        stack.translate(5.0, 5.0);

        let (tx, ty) = stack.transform().translation();
        assert_eq!(tx, 15.0);
        assert_eq!(ty, 25.0);

        stack.restore();

        let (tx, ty) = stack.transform().translation();
        assert_eq!(tx, 10.0);
        assert_eq!(ty, 20.0);
    }

    #[test]
    fn test_clip_intersection() {
        let mut stack = RenderStateStack::new();

        stack.clip_rect(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert_eq!(stack.clip_bounds(), Some(Rect::new(0.0, 0.0, 100.0, 100.0)));

        stack.clip_rect(Rect::new(50.0, 50.0, 100.0, 100.0));
        assert_eq!(stack.clip_bounds(), Some(Rect::new(50.0, 50.0, 50.0, 50.0)));
    }
}
