//! Spacer widget implementation.
//!
//! This module provides [`Spacer`], a widget that occupies space in layouts
//! without any visible content. It's useful for creating flexible spacing
//! between other widgets.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::Spacer;
//!
//! // Create a fixed-size spacer
//! let fixed = Spacer::fixed(20.0, 20.0);
//!
//! // Create an expanding spacer (pushes widgets apart)
//! let expander = Spacer::expanding();
//!
//! // Create a horizontal spring
//! let h_spring = Spacer::horizontal_expanding();
//! ```

use horizon_lattice_core::{Object, ObjectId};
use horizon_lattice_render::Size;

use crate::widget::geometry::SizePolicy;
use crate::widget::{PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent};

/// A widget that provides empty space in layouts.
///
/// Spacer is an invisible widget that occupies space. It's used to:
/// - Add fixed gaps between widgets
/// - Create expanding space to push widgets apart
/// - Center widgets by adding spacers on both sides
///
/// # Types of Spacers
///
/// - **Fixed**: Takes up exactly the specified amount of space
/// - **Expanding**: Grows to fill available space, useful for pushing widgets
/// - **Minimum Expanding**: Has a minimum size but can grow
///
/// # Example Layout Usage
///
/// ```ignore
/// // Center a button by adding expanding spacers on both sides
/// layout.add_widget(Spacer::horizontal_expanding());
/// layout.add_widget(button);
/// layout.add_widget(Spacer::horizontal_expanding());
/// ```
pub struct Spacer {
    /// Widget base.
    base: WidgetBase,

    /// The preferred size of the spacer.
    size: Size,

    /// Whether the spacer expands horizontally.
    horizontal_expanding: bool,

    /// Whether the spacer expands vertically.
    vertical_expanding: bool,
}

impl Spacer {
    /// Create a new spacer with the specified size and expansion settings.
    pub fn new(width: f32, height: f32, h_expanding: bool, v_expanding: bool) -> Self {
        let mut base = WidgetBase::new::<Self>();

        // Set size policy based on expansion settings
        base.set_horizontal_policy(if h_expanding {
            SizePolicy::Expanding
        } else {
            SizePolicy::Fixed
        });
        base.set_vertical_policy(if v_expanding {
            SizePolicy::Expanding
        } else {
            SizePolicy::Fixed
        });

        Self {
            base,
            size: Size::new(width, height),
            horizontal_expanding: h_expanding,
            vertical_expanding: v_expanding,
        }
    }

    /// Create a fixed-size spacer.
    ///
    /// The spacer will always occupy exactly the specified size.
    pub fn fixed(width: f32, height: f32) -> Self {
        Self::new(width, height, false, false)
    }

    /// Create an expanding spacer that grows in both directions.
    ///
    /// This is useful for pushing widgets to the edges of a container.
    pub fn expanding() -> Self {
        Self::new(0.0, 0.0, true, true)
    }

    /// Create a horizontal expanding spacer.
    ///
    /// Expands horizontally but has zero vertical size.
    pub fn horizontal_expanding() -> Self {
        Self::new(0.0, 0.0, true, false)
    }

    /// Create a vertical expanding spacer.
    ///
    /// Expands vertically but has zero horizontal size.
    pub fn vertical_expanding() -> Self {
        Self::new(0.0, 0.0, false, true)
    }

    /// Create a horizontal fixed spacer.
    pub fn horizontal_fixed(width: f32) -> Self {
        Self::new(width, 0.0, false, false)
    }

    /// Create a vertical fixed spacer.
    pub fn vertical_fixed(height: f32) -> Self {
        Self::new(0.0, height, false, false)
    }

    /// Create a spacer with minimum size that can expand.
    pub fn minimum_expanding(min_width: f32, min_height: f32) -> Self {
        let mut spacer = Self::new(min_width, min_height, true, true);
        spacer.base.set_horizontal_policy(SizePolicy::MinimumExpanding);
        spacer.base.set_vertical_policy(SizePolicy::MinimumExpanding);
        spacer
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /// Get the spacer's size.
    pub fn size(&self) -> Size {
        self.size
    }

    /// Set the spacer's size.
    pub fn set_size(&mut self, size: Size) {
        if self.size != size {
            self.size = size;
            self.base.update();
        }
    }

    /// Set size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = Size::new(width, height);
        self
    }

    /// Check if the spacer expands horizontally.
    pub fn is_horizontal_expanding(&self) -> bool {
        self.horizontal_expanding
    }

    /// Check if the spacer expands vertically.
    pub fn is_vertical_expanding(&self) -> bool {
        self.vertical_expanding
    }

    /// Set whether the spacer expands horizontally.
    pub fn set_horizontal_expanding(&mut self, expanding: bool) {
        if self.horizontal_expanding != expanding {
            self.horizontal_expanding = expanding;
            self.base.set_horizontal_policy(if expanding {
                SizePolicy::Expanding
            } else {
                SizePolicy::Fixed
            });
            self.base.update();
        }
    }

    /// Set whether the spacer expands vertically.
    pub fn set_vertical_expanding(&mut self, expanding: bool) {
        if self.vertical_expanding != expanding {
            self.vertical_expanding = expanding;
            self.base.set_vertical_policy(if expanding {
                SizePolicy::Expanding
            } else {
                SizePolicy::Fixed
            });
            self.base.update();
        }
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::expanding()
    }
}

impl Object for Spacer {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Spacer {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let preferred = self.size;
        let minimum = if self.horizontal_expanding || self.vertical_expanding {
            // Expanding spacers have zero minimum
            Size::ZERO
        } else {
            // Fixed spacers have their size as both minimum and maximum
            self.size
        };

        if !self.horizontal_expanding && !self.vertical_expanding {
            SizeHint::fixed(self.size)
        } else {
            SizeHint::new(preferred).with_minimum(minimum)
        }
    }

    fn paint(&self, _ctx: &mut PaintContext<'_>) {
        // Spacers are invisible - nothing to paint
    }

    fn event(&mut self, _event: &mut WidgetEvent) -> bool {
        // Spacers don't handle events
        false
    }
}

// Ensure Spacer is Send + Sync
static_assertions::assert_impl_all!(Spacer: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_spacer_fixed() {
        setup();
        let spacer = Spacer::fixed(20.0, 10.0);
        assert_eq!(spacer.size().width, 20.0);
        assert_eq!(spacer.size().height, 10.0);
        assert!(!spacer.is_horizontal_expanding());
        assert!(!spacer.is_vertical_expanding());

        let hint = spacer.size_hint();
        assert!(hint.is_fixed());
    }

    #[test]
    fn test_spacer_expanding() {
        setup();
        let spacer = Spacer::expanding();
        assert!(spacer.is_horizontal_expanding());
        assert!(spacer.is_vertical_expanding());

        let policy = spacer.widget_base().size_policy();
        assert_eq!(policy.horizontal, SizePolicy::Expanding);
        assert_eq!(policy.vertical, SizePolicy::Expanding);
    }

    #[test]
    fn test_spacer_horizontal_expanding() {
        setup();
        let spacer = Spacer::horizontal_expanding();
        assert!(spacer.is_horizontal_expanding());
        assert!(!spacer.is_vertical_expanding());

        let policy = spacer.widget_base().size_policy();
        assert_eq!(policy.horizontal, SizePolicy::Expanding);
        assert_eq!(policy.vertical, SizePolicy::Fixed);
    }

    #[test]
    fn test_spacer_vertical_expanding() {
        setup();
        let spacer = Spacer::vertical_expanding();
        assert!(!spacer.is_horizontal_expanding());
        assert!(spacer.is_vertical_expanding());

        let policy = spacer.widget_base().size_policy();
        assert_eq!(policy.horizontal, SizePolicy::Fixed);
        assert_eq!(policy.vertical, SizePolicy::Expanding);
    }

    #[test]
    fn test_spacer_size_change() {
        setup();
        let mut spacer = Spacer::fixed(10.0, 10.0);
        spacer.set_size(Size::new(20.0, 30.0));
        assert_eq!(spacer.size().width, 20.0);
        assert_eq!(spacer.size().height, 30.0);
    }

    #[test]
    fn test_spacer_expansion_change() {
        setup();
        let mut spacer = Spacer::fixed(10.0, 10.0);
        assert!(!spacer.is_horizontal_expanding());

        spacer.set_horizontal_expanding(true);
        assert!(spacer.is_horizontal_expanding());
        assert_eq!(
            spacer.widget_base().size_policy().horizontal,
            SizePolicy::Expanding
        );
    }
}
