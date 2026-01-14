//! Separator widget implementation.
//!
//! This module provides [`Separator`], a widget that draws a horizontal or vertical
//! dividing line to visually separate content.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Separator, SeparatorOrientation};
//!
//! // Create a horizontal separator
//! let hsep = Separator::horizontal();
//!
//! // Create a vertical separator
//! let vsep = Separator::vertical();
//!
//! // Customize appearance
//! let custom = Separator::horizontal()
//!     .with_color(Color::from_rgb8(200, 200, 200))
//!     .with_thickness(2.0);
//! ```

use horizon_lattice_core::{Object, ObjectId};
use horizon_lattice_render::{Color, Point, Renderer, Size, Stroke};

use crate::widget::geometry::SizePolicy;
use crate::widget::{PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent};

/// The orientation of a separator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeparatorOrientation {
    /// Horizontal line (divides content vertically).
    #[default]
    Horizontal,
    /// Vertical line (divides content horizontally).
    Vertical,
}

/// A visual separator widget that draws a line to divide content.
///
/// Separator is used to create visual divisions between sections of a UI.
/// It draws a simple line (horizontal or vertical) with configurable
/// appearance.
///
/// # Orientation
///
/// - `Horizontal`: Draws a horizontal line, expands horizontally by default
/// - `Vertical`: Draws a vertical line, expands vertically by default
///
/// # Styling
///
/// The separator's appearance can be customized with:
/// - Color: The line color
/// - Thickness: The line width in pixels
pub struct Separator {
    /// Widget base.
    base: WidgetBase,

    /// The separator orientation.
    orientation: SeparatorOrientation,

    /// The line color.
    color: Color,

    /// The line thickness in pixels.
    thickness: f32,
}

impl Separator {
    /// Create a new separator with the specified orientation.
    pub fn new(orientation: SeparatorOrientation) -> Self {
        let mut base = WidgetBase::new::<Self>();

        // Set size policy based on orientation
        match orientation {
            SeparatorOrientation::Horizontal => {
                base.set_horizontal_policy(SizePolicy::Expanding);
                base.set_vertical_policy(SizePolicy::Fixed);
            }
            SeparatorOrientation::Vertical => {
                base.set_horizontal_policy(SizePolicy::Fixed);
                base.set_vertical_policy(SizePolicy::Expanding);
            }
        }

        Self {
            base,
            orientation,
            color: Color::from_rgb8(192, 192, 192),
            thickness: 1.0,
        }
    }

    /// Create a horizontal separator.
    pub fn horizontal() -> Self {
        Self::new(SeparatorOrientation::Horizontal)
    }

    /// Create a vertical separator.
    pub fn vertical() -> Self {
        Self::new(SeparatorOrientation::Vertical)
    }

    // =========================================================================
    // Orientation
    // =========================================================================

    /// Get the separator orientation.
    pub fn orientation(&self) -> SeparatorOrientation {
        self.orientation
    }

    /// Set the separator orientation.
    pub fn set_orientation(&mut self, orientation: SeparatorOrientation) {
        if self.orientation != orientation {
            self.orientation = orientation;

            // Update size policy
            match orientation {
                SeparatorOrientation::Horizontal => {
                    self.base.set_horizontal_policy(SizePolicy::Expanding);
                    self.base.set_vertical_policy(SizePolicy::Fixed);
                }
                SeparatorOrientation::Vertical => {
                    self.base.set_horizontal_policy(SizePolicy::Fixed);
                    self.base.set_vertical_policy(SizePolicy::Expanding);
                }
            }

            self.base.update();
        }
    }

    /// Set orientation using builder pattern.
    pub fn with_orientation(mut self, orientation: SeparatorOrientation) -> Self {
        self.set_orientation(orientation);
        self
    }

    /// Check if horizontal.
    pub fn is_horizontal(&self) -> bool {
        self.orientation == SeparatorOrientation::Horizontal
    }

    /// Check if vertical.
    pub fn is_vertical(&self) -> bool {
        self.orientation == SeparatorOrientation::Vertical
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the separator color.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Set the separator color.
    pub fn set_color(&mut self, color: Color) {
        if self.color != color {
            self.color = color;
            self.base.update();
        }
    }

    /// Set color using builder pattern.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Get the line thickness.
    pub fn thickness(&self) -> f32 {
        self.thickness
    }

    /// Set the line thickness.
    pub fn set_thickness(&mut self, thickness: f32) {
        let thickness = thickness.max(0.1);
        if (self.thickness - thickness).abs() > f32::EPSILON {
            self.thickness = thickness;
            self.base.update();
        }
    }

    /// Set thickness using builder pattern.
    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness.max(0.1);
        self
    }
}

impl Default for Separator {
    fn default() -> Self {
        Self::horizontal()
    }
}

impl Object for Separator {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Separator {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        match self.orientation {
            SeparatorOrientation::Horizontal => {
                // Horizontal: minimal height, expanding width
                let preferred = Size::new(40.0, self.thickness);
                let minimum = Size::new(1.0, self.thickness);
                SizeHint::new(preferred).with_minimum(minimum)
            }
            SeparatorOrientation::Vertical => {
                // Vertical: expanding height, minimal width
                let preferred = Size::new(self.thickness, 40.0);
                let minimum = Size::new(self.thickness, 1.0);
                SizeHint::new(preferred).with_minimum(minimum)
            }
        }
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        let stroke = Stroke::new(self.color, self.thickness);

        match self.orientation {
            SeparatorOrientation::Horizontal => {
                // Draw horizontal line centered vertically
                let y = rect.origin.y + rect.height() / 2.0;
                ctx.renderer().draw_line(
                    Point::new(rect.origin.x, y),
                    Point::new(rect.origin.x + rect.width(), y),
                    &stroke,
                );
            }
            SeparatorOrientation::Vertical => {
                // Draw vertical line centered horizontally
                let x = rect.origin.x + rect.width() / 2.0;
                ctx.renderer().draw_line(
                    Point::new(x, rect.origin.y),
                    Point::new(x, rect.origin.y + rect.height()),
                    &stroke,
                );
            }
        }
    }

    fn event(&mut self, _event: &mut WidgetEvent) -> bool {
        // Separator doesn't handle events
        false
    }
}

// Ensure Separator is Send + Sync
static_assertions::assert_impl_all!(Separator: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_separator_creation() {
        setup();
        let sep = Separator::horizontal();
        assert!(sep.is_horizontal());
        assert_eq!(sep.thickness(), 1.0);

        let vsep = Separator::vertical();
        assert!(vsep.is_vertical());
    }

    #[test]
    fn test_separator_builder_pattern() {
        setup();
        let sep = Separator::horizontal()
            .with_color(Color::RED)
            .with_thickness(2.0);

        assert_eq!(sep.color(), Color::RED);
        assert_eq!(sep.thickness(), 2.0);
    }

    #[test]
    fn test_separator_orientation_change() {
        setup();
        let mut sep = Separator::horizontal();
        assert!(sep.is_horizontal());

        sep.set_orientation(SeparatorOrientation::Vertical);
        assert!(sep.is_vertical());
    }

    #[test]
    fn test_separator_size_hint() {
        setup();
        let hsep = Separator::horizontal().with_thickness(2.0);
        let hint = hsep.size_hint();
        assert_eq!(hint.effective_minimum().height, 2.0);

        let vsep = Separator::vertical().with_thickness(3.0);
        let hint = vsep.size_hint();
        assert_eq!(hint.effective_minimum().width, 3.0);
    }

    #[test]
    fn test_separator_size_policy() {
        setup();
        let hsep = Separator::horizontal();
        let policy = hsep.widget_base().size_policy();
        assert_eq!(policy.horizontal, SizePolicy::Expanding);
        assert_eq!(policy.vertical, SizePolicy::Fixed);

        let vsep = Separator::vertical();
        let policy = vsep.widget_base().size_policy();
        assert_eq!(policy.horizontal, SizePolicy::Fixed);
        assert_eq!(policy.vertical, SizePolicy::Expanding);
    }
}
