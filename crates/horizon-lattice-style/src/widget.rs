//! Widget styling integration.
//!
//! This module provides traits and helpers for integrating the style system
//! with widgets.

use crate::style::{ComputedStyle, StyleProperties};
use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Color, CornerRadii, Rect};

/// Trait for widgets that support CSS-like styling.
///
/// Implement this trait on your widgets to enable selector matching,
/// class-based styling, and inline styles.
///
/// # Example
///
/// ```ignore
/// struct MyButton {
///     base: ObjectBase,
///     classes: Vec<String>,
///     inline_style: Option<StyleProperties>,
///     computed_style: Option<ComputedStyle>,
/// }
///
/// impl StyledWidget for MyButton {
///     fn widget_id(&self) -> ObjectId {
///         self.base.id()
///     }
///
///     fn widget_type_name(&self) -> &'static str {
///         "Button"
///     }
///
///     fn style_classes(&self) -> &[String] {
///         &self.classes
///     }
///
///     fn add_class(&mut self, class: impl Into<String>) {
///         self.classes.push(class.into());
///     }
///
///     fn remove_class(&mut self, class: &str) -> bool {
///         if let Some(pos) = self.classes.iter().position(|c| c == class) {
///             self.classes.remove(pos);
///             true
///         } else {
///             false
///         }
///     }
///
///     fn inline_style(&self) -> Option<&StyleProperties> {
///         self.inline_style.as_ref()
///     }
///
///     fn set_inline_style(&mut self, style: Option<StyleProperties>) {
///         self.inline_style = style;
///     }
///
///     fn computed_style(&self) -> Option<&ComputedStyle> {
///         self.computed_style.as_ref()
///     }
///
///     fn set_computed_style(&mut self, style: ComputedStyle) {
///         self.computed_style = Some(style);
///     }
/// }
/// ```
pub trait StyledWidget {
    /// Get the widget's unique object ID.
    fn widget_id(&self) -> ObjectId;

    /// Get the widget's type name for type selector matching.
    ///
    /// This should return a short type name like "Button", "Label", "TextInput",
    /// not the full Rust type path.
    fn widget_type_name(&self) -> &'static str;

    /// Get the widget's name/ID for #id selector matching.
    ///
    /// Returns `None` if the widget has no name set.
    fn widget_name(&self) -> Option<&str> {
        None
    }

    /// Get the widget's CSS classes.
    fn style_classes(&self) -> &[String];

    /// Add a CSS class to the widget.
    fn add_class(&mut self, class: impl Into<String>);

    /// Remove a CSS class from the widget.
    ///
    /// Returns `true` if the class was found and removed.
    fn remove_class(&mut self, class: &str) -> bool;

    /// Check if the widget has a specific class.
    fn has_class(&self, class: &str) -> bool {
        self.style_classes().iter().any(|c| c == class)
    }

    /// Toggle a CSS class on the widget.
    ///
    /// Returns `true` if the class was added, `false` if it was removed.
    fn toggle_class(&mut self, class: &str) -> bool {
        if self.has_class(class) {
            self.remove_class(class);
            false
        } else {
            self.add_class(class.to_string());
            true
        }
    }

    /// Get the widget's inline style (highest priority).
    fn inline_style(&self) -> Option<&StyleProperties>;

    /// Set the widget's inline style.
    fn set_inline_style(&mut self, style: Option<StyleProperties>);

    /// Get the widget's computed style.
    fn computed_style(&self) -> Option<&ComputedStyle>;

    /// Set the widget's computed style.
    fn set_computed_style(&mut self, style: ComputedStyle);

    /// Clear the computed style to force recalculation.
    fn invalidate_style(&mut self) {
        // Default implementation does nothing; override if needed
    }
}

// ============================================================================
// Paint Helpers
// ============================================================================

/// Paint context abstraction for style rendering.
///
/// This is a simplified interface for rendering styled content.
/// Real implementations will use the GPU renderer.
pub trait StylePaintContext {
    /// Fill a rectangle with a color.
    fn fill_rect(&mut self, rect: Rect, color: Color);

    /// Fill a rounded rectangle.
    fn fill_rounded_rect(&mut self, rect: Rect, radii: CornerRadii, color: Color);

    /// Draw a border around a rectangle.
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: f32);

    /// Draw a rounded border.
    fn stroke_rounded_rect(&mut self, rect: Rect, radii: CornerRadii, color: Color, width: f32);

    /// Save the current clip state.
    fn save_clip(&mut self);

    /// Restore the previous clip state.
    fn restore_clip(&mut self);

    /// Set a clip rectangle.
    fn clip_rect(&mut self, rect: Rect);
}

/// Paint the background of a styled widget.
///
/// This draws the background color within the border box.
pub fn paint_background(ctx: &mut dyn StylePaintContext, rect: Rect, style: &ComputedStyle) {
    // Extract color from Paint - for now only support solid colors
    let bg_color = match &style.background {
        horizon_lattice_render::Paint::Solid(color) => *color,
        _ => return, // Gradients not yet supported in paint helpers
    };

    // Skip if fully transparent
    if bg_color.a == 0.0 {
        return;
    }

    let radii = style.border_radius;

    if radii.top_left == 0.0
        && radii.top_right == 0.0
        && radii.bottom_left == 0.0
        && radii.bottom_right == 0.0
    {
        ctx.fill_rect(rect, bg_color);
    } else {
        ctx.fill_rounded_rect(rect, radii, bg_color);
    }
}

/// Paint the border of a styled widget.
///
/// This draws borders with the specified widths and colors.
pub fn paint_border(ctx: &mut dyn StylePaintContext, rect: Rect, style: &ComputedStyle) {
    // Simple uniform border for now
    let width = style.border_top_width;
    let color = style.border_color;

    // Skip if no border
    if width == 0.0 || color.a == 0.0 {
        return;
    }

    let radii = style.border_radius;

    // Inset the rect by half the border width for centered strokes
    let inset = width / 2.0;
    let border_rect = Rect::new(
        rect.origin.x + inset,
        rect.origin.y + inset,
        rect.width() - width,
        rect.height() - width,
    );

    if radii.top_left == 0.0
        && radii.top_right == 0.0
        && radii.bottom_left == 0.0
        && radii.bottom_right == 0.0
    {
        ctx.stroke_rect(border_rect, color, width);
    } else {
        // Adjust radii for the inset
        let adjusted_radii = CornerRadii {
            top_left: (radii.top_left - inset).max(0.0),
            top_right: (radii.top_right - inset).max(0.0),
            bottom_left: (radii.bottom_left - inset).max(0.0),
            bottom_right: (radii.bottom_right - inset).max(0.0),
        };
        ctx.stroke_rounded_rect(border_rect, adjusted_radii, color, width);
    }
}

/// Paint a complete styled widget box (background, border, shadows).
///
/// Call this helper in your widget's paint method to handle all box styling.
/// After calling this, paint your widget's content within the content rectangle.
///
/// # Example
///
/// ```ignore
/// fn paint(&self, ctx: &mut PaintContext) {
///     if let Some(style) = self.computed_style() {
///         let bounds = self.bounds();
///         paint_styled_box(ctx, bounds, style);
///
///         // Now paint content within the content rect
///         let content_rect = style.content_rect(bounds);
///         // ... paint content ...
///     }
/// }
/// ```
pub fn paint_styled_box(ctx: &mut dyn StylePaintContext, rect: Rect, style: &ComputedStyle) {
    // 1. Paint outer box shadows (not yet implemented - would go here)

    // 2. Paint background
    paint_background(ctx, rect, style);

    // 3. Paint border
    paint_border(ctx, rect, style);

    // 4. Paint inset box shadows (not yet implemented - would go here)
}

/// Calculate the content rectangle given a bounding rect and computed style.
///
/// This accounts for padding, border, and margin to determine where
/// content should be placed.
pub fn content_rect(rect: Rect, style: &ComputedStyle) -> Rect {
    let left = style.padding_left + style.border_left_width;
    let top = style.padding_top + style.border_top_width;
    let right = style.padding_right + style.border_right_width;
    let bottom = style.padding_bottom + style.border_bottom_width;

    Rect::new(
        rect.origin.x + left,
        rect.origin.y + top,
        (rect.width() - left - right).max(0.0),
        (rect.height() - top - bottom).max(0.0),
    )
}

/// Calculate the border box rectangle given content dimensions and style.
///
/// This is the inverse of `content_rect` - given the desired content size,
/// calculate the outer box size.
pub fn border_box_size(
    content_width: f32,
    content_height: f32,
    style: &ComputedStyle,
) -> (f32, f32) {
    let h_space = style.padding_left
        + style.padding_right
        + style.border_left_width
        + style.border_right_width;
    let v_space = style.padding_top
        + style.padding_bottom
        + style.border_top_width
        + style.border_bottom_width;

    (content_width + h_space, content_height + v_space)
}

/// Calculate the margin box rectangle given a border box rect and style.
pub fn margin_rect(border_rect: Rect, style: &ComputedStyle) -> Rect {
    Rect::new(
        border_rect.origin.x - style.margin_left,
        border_rect.origin.y - style.margin_top,
        border_rect.width() + style.margin_left + style.margin_right,
        border_rect.height() + style.margin_top + style.margin_bottom,
    )
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    fn make_style() -> ComputedStyle {
        let mut style = ComputedStyle::default();
        style.padding_top = 10.0;
        style.padding_right = 10.0;
        style.padding_bottom = 10.0;
        style.padding_left = 10.0;
        style.border_top_width = 2.0;
        style.border_right_width = 2.0;
        style.border_bottom_width = 2.0;
        style.border_left_width = 2.0;
        style
    }

    #[test]
    fn test_content_rect_calculation() {
        let style = make_style();
        let bounds = Rect::new(0.0, 0.0, 100.0, 80.0);

        let content = content_rect(bounds, &style);

        // Expected: 12px inset on each side (10 padding + 2 border)
        assert_eq!(content.origin.x, 12.0);
        assert_eq!(content.origin.y, 12.0);
        assert_eq!(content.width(), 76.0); // 100 - 12 - 12
        assert_eq!(content.height(), 56.0); // 80 - 12 - 12
    }

    #[test]
    fn test_border_box_size_calculation() {
        let style = make_style();

        let (width, height) = border_box_size(50.0, 30.0, &style);

        // Expected: 24px added (12 on each side)
        assert_eq!(width, 74.0); // 50 + 12 + 12
        assert_eq!(height, 54.0); // 30 + 12 + 12
    }

    #[test]
    fn test_margin_rect_calculation() {
        let mut style = make_style();
        style.margin_top = 5.0;
        style.margin_right = 5.0;
        style.margin_bottom = 5.0;
        style.margin_left = 5.0;

        let border_rect = Rect::new(10.0, 10.0, 100.0, 80.0);
        let margin = margin_rect(border_rect, &style);

        assert_eq!(margin.origin.x, 5.0);
        assert_eq!(margin.origin.y, 5.0);
        assert_eq!(margin.width(), 110.0);
        assert_eq!(margin.height(), 90.0);
    }
}
