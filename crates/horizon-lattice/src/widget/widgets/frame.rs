//! Frame widget implementation.
//!
//! This module provides [`Frame`], a container widget with optional border decoration.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Frame, FrameShape, FrameShadow};
//!
//! // Create a simple box frame
//! let frame = Frame::new()
//!     .with_shape(FrameShape::Box)
//!     .with_shadow(FrameShadow::Plain);
//!
//! // Create a sunken panel
//! let panel = Frame::new()
//!     .with_shape(FrameShape::Panel)
//!     .with_shadow(FrameShadow::Sunken);
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Size, Stroke};

use crate::widget::layout::ContentMargins;
use crate::widget::{PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent};

/// The shape of the frame border.
///
/// This determines the basic geometry of the frame's border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameShape {
    /// No frame is drawn.
    #[default]
    NoFrame,
    /// A simple rectangular border.
    Box,
    /// A panel-style border (typically with shadow effects).
    Panel,
    /// A styled panel with more pronounced visual effects.
    StyledPanel,
}

/// The shadow effect applied to the frame border.
///
/// This determines the 3D effect of the frame's border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FrameShadow {
    /// No shadow effect, flat appearance.
    #[default]
    Plain,
    /// Border appears raised above the surface.
    Raised,
    /// Border appears sunken below the surface.
    Sunken,
}

/// A container widget with an optional decorative border.
///
/// Frame provides a visual container with customizable border styles.
/// It can be used to group related widgets or create visual sections
/// in your UI.
///
/// # Frame Styles
///
/// The appearance is controlled by two properties:
/// - [`FrameShape`]: The basic border geometry (NoFrame, Box, Panel, StyledPanel)
/// - [`FrameShadow`]: The 3D effect (Plain, Raised, Sunken)
///
/// # Content
///
/// Frame supports content margins to provide padding around its contents.
/// Child widgets can be managed through the parent-child object system.
///
/// # Signals
///
/// - `shape_changed`: Emitted when the frame shape changes
/// - `shadow_changed`: Emitted when the frame shadow changes
pub struct Frame {
    /// Widget base.
    base: WidgetBase,

    /// The frame shape.
    shape: FrameShape,

    /// The frame shadow effect.
    shadow: FrameShadow,

    /// Border width in pixels.
    line_width: f32,

    /// Mid-line width for double borders (StyledPanel).
    mid_line_width: f32,

    /// Content margins (padding inside the frame).
    content_margins: ContentMargins,

    /// Background color (if any).
    background_color: Option<Color>,

    /// Border color.
    border_color: Color,

    /// Signal emitted when shape changes.
    pub shape_changed: Signal<FrameShape>,

    /// Signal emitted when shadow changes.
    pub shadow_changed: Signal<FrameShadow>,
}

impl Frame {
    /// Create a new frame with default settings (no frame).
    pub fn new() -> Self {
        Self {
            base: WidgetBase::new::<Self>(),
            shape: FrameShape::NoFrame,
            shadow: FrameShadow::Plain,
            line_width: 1.0,
            mid_line_width: 0.0,
            content_margins: ContentMargins::uniform(0.0),
            background_color: None,
            border_color: Color::from_rgb8(128, 128, 128),
            shape_changed: Signal::new(),
            shadow_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Shape and Shadow
    // =========================================================================

    /// Get the frame shape.
    pub fn shape(&self) -> FrameShape {
        self.shape
    }

    /// Set the frame shape.
    pub fn set_shape(&mut self, shape: FrameShape) {
        if self.shape != shape {
            self.shape = shape;
            self.base.update();
            self.shape_changed.emit(shape);
        }
    }

    /// Set shape using builder pattern.
    pub fn with_shape(mut self, shape: FrameShape) -> Self {
        self.shape = shape;
        self
    }

    /// Get the frame shadow.
    pub fn shadow(&self) -> FrameShadow {
        self.shadow
    }

    /// Set the frame shadow.
    pub fn set_shadow(&mut self, shadow: FrameShadow) {
        if self.shadow != shadow {
            self.shadow = shadow;
            self.base.update();
            self.shadow_changed.emit(shadow);
        }
    }

    /// Set shadow using builder pattern.
    pub fn with_shadow(mut self, shadow: FrameShadow) -> Self {
        self.shadow = shadow;
        self
    }

    /// Get the combined frame style (shape + shadow).
    pub fn frame_style(&self) -> (FrameShape, FrameShadow) {
        (self.shape, self.shadow)
    }

    /// Set both shape and shadow at once.
    pub fn set_frame_style(&mut self, shape: FrameShape, shadow: FrameShadow) {
        let shape_changed = self.shape != shape;
        let shadow_changed = self.shadow != shadow;

        if shape_changed || shadow_changed {
            self.shape = shape;
            self.shadow = shadow;
            self.base.update();

            if shape_changed {
                self.shape_changed.emit(shape);
            }
            if shadow_changed {
                self.shadow_changed.emit(shadow);
            }
        }
    }

    // =========================================================================
    // Line Width
    // =========================================================================

    /// Get the border line width.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }

    /// Set the border line width.
    pub fn set_line_width(&mut self, width: f32) {
        if (self.line_width - width).abs() > f32::EPSILON {
            self.line_width = width.max(0.0);
            self.base.update();
        }
    }

    /// Set line width using builder pattern.
    pub fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.0);
        self
    }

    /// Get the mid-line width (for StyledPanel).
    pub fn mid_line_width(&self) -> f32 {
        self.mid_line_width
    }

    /// Set the mid-line width.
    pub fn set_mid_line_width(&mut self, width: f32) {
        if (self.mid_line_width - width).abs() > f32::EPSILON {
            self.mid_line_width = width.max(0.0);
            self.base.update();
        }
    }

    /// Set mid-line width using builder pattern.
    pub fn with_mid_line_width(mut self, width: f32) -> Self {
        self.mid_line_width = width.max(0.0);
        self
    }

    // =========================================================================
    // Content Margins
    // =========================================================================

    /// Get the content margins.
    pub fn content_margins(&self) -> ContentMargins {
        self.content_margins
    }

    /// Set the content margins.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        if self.content_margins != margins {
            self.content_margins = margins;
            self.base.update();
        }
    }

    /// Set uniform content margins.
    pub fn set_content_margin(&mut self, margin: f32) {
        self.set_content_margins(ContentMargins::uniform(margin));
    }

    /// Set content margins using builder pattern.
    pub fn with_content_margins(mut self, margins: ContentMargins) -> Self {
        self.content_margins = margins;
        self
    }

    /// Set uniform content margins using builder pattern.
    pub fn with_content_margin(mut self, margin: f32) -> Self {
        self.content_margins = ContentMargins::uniform(margin);
        self
    }

    // =========================================================================
    // Colors
    // =========================================================================

    /// Get the background color.
    pub fn background_color(&self) -> Option<Color> {
        self.background_color
    }

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Option<Color>) {
        if self.background_color != color {
            self.background_color = color;
            self.base.update();
        }
    }

    /// Set background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Get the border color.
    pub fn border_color(&self) -> Color {
        self.border_color
    }

    /// Set the border color.
    pub fn set_border_color(&mut self, color: Color) {
        if self.border_color != color {
            self.border_color = color;
            self.base.update();
        }
    }

    /// Set border color using builder pattern.
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    // =========================================================================
    // Content Area
    // =========================================================================

    /// Get the content area rectangle (inside the frame and margins).
    pub fn contents_rect(&self) -> Rect {
        let rect = self.base.rect();
        let frame_width = self.frame_width();

        Rect::new(
            frame_width + self.content_margins.left,
            frame_width + self.content_margins.top,
            (rect.width() - 2.0 * frame_width - self.content_margins.left - self.content_margins.right).max(0.0),
            (rect.height() - 2.0 * frame_width - self.content_margins.top - self.content_margins.bottom).max(0.0),
        )
    }

    /// Calculate the total frame width (line width + mid line for styled panels).
    fn frame_width(&self) -> f32 {
        match self.shape {
            FrameShape::NoFrame => 0.0,
            FrameShape::Box => self.line_width,
            FrameShape::Panel => self.line_width,
            FrameShape::StyledPanel => self.line_width + self.mid_line_width,
        }
    }

    // =========================================================================
    // Painting Helpers
    // =========================================================================

    /// Get light and dark colors for 3D effects.
    fn get_3d_colors(&self) -> (Color, Color) {
        let base = self.border_color;

        // Create lighter and darker versions for 3D effect
        let light = Color::from_rgba(
            (base.r * 1.3).min(1.0),
            (base.g * 1.3).min(1.0),
            (base.b * 1.3).min(1.0),
            base.a,
        );
        let dark = Color::from_rgba(
            base.r * 0.6,
            base.g * 0.6,
            base.b * 0.6,
            base.a,
        );

        (light, dark)
    }

    /// Paint the frame border.
    fn paint_frame(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }

        match self.shape {
            FrameShape::NoFrame => {}
            FrameShape::Box => self.paint_box_frame(ctx, rect),
            FrameShape::Panel => self.paint_panel_frame(ctx, rect),
            FrameShape::StyledPanel => self.paint_styled_panel_frame(ctx, rect),
        }
    }

    /// Paint a simple box frame.
    fn paint_box_frame(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        if self.line_width <= 0.0 {
            return;
        }

        let stroke = Stroke::new(self.border_color, self.line_width);

        // Inset the rect by half the stroke width for proper rendering
        let inset = self.line_width / 2.0;
        let frame_rect = Rect::new(
            rect.origin.x + inset,
            rect.origin.y + inset,
            rect.width() - self.line_width,
            rect.height() - self.line_width,
        );

        ctx.renderer().stroke_rect(frame_rect, &stroke);
    }

    /// Paint a panel frame with 3D effect.
    fn paint_panel_frame(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        if self.line_width <= 0.0 {
            return;
        }

        let (light, dark) = self.get_3d_colors();
        let (top_left_color, bottom_right_color) = match self.shadow {
            FrameShadow::Plain => (self.border_color, self.border_color),
            FrameShadow::Raised => (light, dark),
            FrameShadow::Sunken => (dark, light),
        };

        // Draw the 3D border using lines
        let lw = self.line_width;

        // Top edge
        let stroke_tl = Stroke::new(top_left_color, lw);
        ctx.renderer().draw_line(
            Point::new(rect.origin.x, rect.origin.y + lw / 2.0),
            Point::new(rect.origin.x + rect.width(), rect.origin.y + lw / 2.0),
            &stroke_tl,
        );

        // Left edge
        ctx.renderer().draw_line(
            Point::new(rect.origin.x + lw / 2.0, rect.origin.y),
            Point::new(rect.origin.x + lw / 2.0, rect.origin.y + rect.height()),
            &stroke_tl,
        );

        // Bottom edge
        let stroke_br = Stroke::new(bottom_right_color, lw);
        ctx.renderer().draw_line(
            Point::new(rect.origin.x, rect.origin.y + rect.height() - lw / 2.0),
            Point::new(rect.origin.x + rect.width(), rect.origin.y + rect.height() - lw / 2.0),
            &stroke_br,
        );

        // Right edge
        ctx.renderer().draw_line(
            Point::new(rect.origin.x + rect.width() - lw / 2.0, rect.origin.y),
            Point::new(rect.origin.x + rect.width() - lw / 2.0, rect.origin.y + rect.height()),
            &stroke_br,
        );
    }

    /// Paint a styled panel frame with double 3D effect.
    fn paint_styled_panel_frame(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        // Draw outer panel
        self.paint_panel_frame(ctx, rect);

        // Draw inner panel (inverted shadow effect)
        if self.mid_line_width > 0.0 {
            let inner_rect = Rect::new(
                rect.origin.x + self.line_width,
                rect.origin.y + self.line_width,
                rect.width() - 2.0 * self.line_width,
                rect.height() - 2.0 * self.line_width,
            );

            let (light, dark) = self.get_3d_colors();
            // Inverted colors for inner border
            let (top_left_color, bottom_right_color) = match self.shadow {
                FrameShadow::Plain => (self.border_color, self.border_color),
                FrameShadow::Raised => (dark, light),
                FrameShadow::Sunken => (light, dark),
            };

            let lw = self.mid_line_width;

            // Inner top edge
            let stroke_tl = Stroke::new(top_left_color, lw);
            ctx.renderer().draw_line(
                Point::new(inner_rect.origin.x, inner_rect.origin.y + lw / 2.0),
                Point::new(inner_rect.origin.x + inner_rect.width(), inner_rect.origin.y + lw / 2.0),
                &stroke_tl,
            );

            // Inner left edge
            ctx.renderer().draw_line(
                Point::new(inner_rect.origin.x + lw / 2.0, inner_rect.origin.y),
                Point::new(inner_rect.origin.x + lw / 2.0, inner_rect.origin.y + inner_rect.height()),
                &stroke_tl,
            );

            // Inner bottom edge
            let stroke_br = Stroke::new(bottom_right_color, lw);
            ctx.renderer().draw_line(
                Point::new(inner_rect.origin.x, inner_rect.origin.y + inner_rect.height() - lw / 2.0),
                Point::new(inner_rect.origin.x + inner_rect.width(), inner_rect.origin.y + inner_rect.height() - lw / 2.0),
                &stroke_br,
            );

            // Inner right edge
            ctx.renderer().draw_line(
                Point::new(inner_rect.origin.x + inner_rect.width() - lw / 2.0, inner_rect.origin.y),
                Point::new(inner_rect.origin.x + inner_rect.width() - lw / 2.0, inner_rect.origin.y + inner_rect.height()),
                &stroke_br,
            );
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for Frame {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Frame {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Calculate minimum size based on frame and margins
        let frame_width = self.frame_width();
        let min_width = 2.0 * frame_width + self.content_margins.left + self.content_margins.right;
        let min_height = 2.0 * frame_width + self.content_margins.top + self.content_margins.bottom;

        // Default preferred size
        let preferred = Size::new(
            min_width.max(100.0),
            min_height.max(100.0),
        );

        SizeHint::new(preferred).with_minimum(Size::new(min_width, min_height))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Draw background if set
        if let Some(bg_color) = self.background_color {
            ctx.renderer().fill_rect(rect, bg_color);
        }

        // Draw frame
        self.paint_frame(ctx);
    }

    fn event(&mut self, _event: &mut WidgetEvent) -> bool {
        // Frame doesn't handle events itself, they pass through to children
        false
    }
}

// Ensure Frame is Send + Sync
static_assertions::assert_impl_all!(Frame: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_frame_creation() {
        setup();
        let frame = Frame::new();
        assert_eq!(frame.shape(), FrameShape::NoFrame);
        assert_eq!(frame.shadow(), FrameShadow::Plain);
        assert_eq!(frame.line_width(), 1.0);
    }

    #[test]
    fn test_frame_builder_pattern() {
        setup();
        let frame = Frame::new()
            .with_shape(FrameShape::Panel)
            .with_shadow(FrameShadow::Sunken)
            .with_line_width(2.0)
            .with_content_margin(8.0);

        assert_eq!(frame.shape(), FrameShape::Panel);
        assert_eq!(frame.shadow(), FrameShadow::Sunken);
        assert_eq!(frame.line_width(), 2.0);
        assert_eq!(frame.content_margins().left, 8.0);
    }

    #[test]
    fn test_frame_style_change() {
        setup();
        let mut frame = Frame::new();

        frame.set_frame_style(FrameShape::Box, FrameShadow::Raised);
        assert_eq!(frame.shape(), FrameShape::Box);
        assert_eq!(frame.shadow(), FrameShadow::Raised);
    }

    #[test]
    fn test_frame_width_calculation() {
        setup();
        let mut frame = Frame::new();

        // NoFrame
        frame.set_shape(FrameShape::NoFrame);
        assert_eq!(frame.frame_width(), 0.0);

        // Box
        frame.set_shape(FrameShape::Box);
        frame.set_line_width(2.0);
        assert_eq!(frame.frame_width(), 2.0);

        // StyledPanel with mid-line
        frame.set_shape(FrameShape::StyledPanel);
        frame.set_mid_line_width(1.0);
        assert_eq!(frame.frame_width(), 3.0);
    }

    #[test]
    fn test_contents_rect() {
        setup();
        let mut frame = Frame::new()
            .with_shape(FrameShape::Box)
            .with_line_width(2.0)
            .with_content_margin(4.0);

        // Set geometry
        frame.widget_base_mut().set_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));

        let content = frame.contents_rect();
        // Content should be inset by frame_width (2) + margin (4) = 6 on each side
        assert_eq!(content.origin.x, 6.0);
        assert_eq!(content.origin.y, 6.0);
        assert_eq!(content.width(), 88.0);
        assert_eq!(content.height(), 88.0);
    }

    #[test]
    fn test_frame_size_hint() {
        setup();
        let frame = Frame::new()
            .with_shape(FrameShape::Panel)
            .with_line_width(2.0)
            .with_content_margin(8.0);

        let hint = frame.size_hint();
        // Minimum should account for frame + margins on both sides
        // frame_width = 2, margins = 8 each side
        // min = 2*2 + 8 + 8 = 20
        assert!(hint.effective_minimum().width >= 20.0);
        assert!(hint.effective_minimum().height >= 20.0);
    }
}
