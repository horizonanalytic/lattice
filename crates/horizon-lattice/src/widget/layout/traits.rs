//! Core Layout trait definition.
//!
//! The Layout trait defines the interface that all layout managers must implement.

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

use super::item::{LayoutItem, LayoutItemData};
use super::ContentMargins;
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::geometry::{SizeHint, SizePolicyPair};

/// The core trait for all layout managers.
///
/// Layout managers are responsible for automatically positioning and sizing
/// widgets. They collect size hints from their items, calculate the optimal
/// arrangement, and apply geometries to managed widgets.
///
/// # Layout Algorithm
///
/// Layouts use a two-pass algorithm:
///
/// 1. **Collection (bottom-up)**: The layout queries each item's size hint
///    and policy to determine its own size requirements. This happens in
///    `size_hint()` and `minimum_size()`.
///
/// 2. **Distribution (top-down)**: Given available space, the layout
///    calculates positions and sizes for each item based on their policies
///    and stretch factors. This happens in `calculate()` and `apply()`.
///
/// # Implementing a Custom Layout
///
/// To implement a custom layout:
///
/// 1. Store items in your layout struct
/// 2. Implement item management methods (`add_item`, `remove_item`, etc.)
/// 3. Implement size calculation (`size_hint`, `minimum_size`, `maximum_size`)
/// 4. Implement `calculate` to compute item geometries
/// 5. Implement `apply` to set widget geometries
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::layout::*;
///
/// struct MyLayout {
///     base: LayoutBase,
///     items: Vec<LayoutItem>,
/// }
///
/// impl Layout for MyLayout {
///     fn add_item(&mut self, item: LayoutItem) {
///         self.items.push(item);
///         self.base.invalidate();
///     }
///
///     fn calculate<S: WidgetAccess>(&mut self, storage: &S, available: Size) -> Size {
///         // Calculate positions for each item
///         // Return the actual size used
///     }
///
///     fn apply<S: WidgetAccess>(&self, storage: &mut S) {
///         // Set geometry on each widget item
///     }
///     // ... other methods
/// }
/// ```
pub trait Layout: Send + Sync {
    // =========================================================================
    // Item Management
    // =========================================================================

    /// Add an item to the layout.
    ///
    /// The item is added at the end of the layout's item list.
    /// This invalidates the layout for recalculation.
    fn add_item(&mut self, item: LayoutItem);

    /// Add a widget to the layout.
    ///
    /// Convenience method that wraps the widget ID in a `LayoutItem::Widget`.
    fn add_widget(&mut self, widget: ObjectId) {
        self.add_item(LayoutItem::Widget(widget));
    }

    /// Insert an item at a specific index.
    ///
    /// Items at and after the index are shifted right.
    /// Panics if index > item_count().
    fn insert_item(&mut self, index: usize, item: LayoutItem);

    /// Remove an item at the specified index.
    ///
    /// Returns the removed item, or None if the index is out of bounds.
    fn remove_item(&mut self, index: usize) -> Option<LayoutItem>;

    /// Remove a widget from the layout by its ObjectId.
    ///
    /// Returns true if the widget was found and removed.
    fn remove_widget(&mut self, widget: ObjectId) -> bool;

    /// Get the number of items in the layout.
    fn item_count(&self) -> usize;

    /// Get an item by index.
    fn item_at(&self, index: usize) -> Option<&LayoutItem>;

    /// Get a mutable reference to an item by index.
    fn item_at_mut(&mut self, index: usize) -> Option<&mut LayoutItem>;

    /// Clear all items from the layout.
    fn clear(&mut self);

    /// Check if the layout is empty.
    fn is_empty(&self) -> bool {
        self.item_count() == 0
    }

    // =========================================================================
    // Size Hints & Policies
    // =========================================================================

    /// Get the layout's size hint.
    ///
    /// The size hint represents the preferred size for the layout when
    /// all items are at their preferred sizes. This is calculated based
    /// on item size hints, spacing, and margins.
    fn size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint;

    /// Get the minimum size for the layout.
    ///
    /// The layout cannot be made smaller than this without clipping content.
    /// This is calculated based on item minimum sizes, spacing, and margins.
    fn minimum_size<S: WidgetAccess>(&self, storage: &S) -> Size;

    /// Get the maximum size for the layout.
    ///
    /// The layout will not benefit from being larger than this size.
    /// Returns a very large size if there's no practical maximum.
    fn maximum_size<S: WidgetAccess>(&self, _storage: &S) -> Size {
        Size::new(f32::MAX, f32::MAX)
    }

    /// Get the layout's size policy.
    ///
    /// Determines how the layout behaves when more or less space is available
    /// than its preferred size.
    fn size_policy(&self) -> SizePolicyPair {
        SizePolicyPair::default()
    }

    /// Get the height for a given width (for layouts with height-for-width).
    ///
    /// Some layouts (like flow layouts with wrapping text) need to adjust
    /// their height based on available width.
    fn height_for_width<S: WidgetAccess>(&self, _storage: &S, _width: f32) -> Option<f32> {
        None
    }

    /// Check if this layout has height-for-width dependency.
    fn has_height_for_width(&self) -> bool {
        false
    }

    // =========================================================================
    // Geometry & Margins
    // =========================================================================

    /// Get the layout's geometry (the rectangle it occupies).
    fn geometry(&self) -> Rect;

    /// Set the layout's geometry.
    ///
    /// This defines the available space for the layout. Call `apply()` after
    /// setting geometry to update widget positions.
    fn set_geometry(&mut self, rect: Rect);

    /// Get the content margins.
    fn content_margins(&self) -> ContentMargins;

    /// Set the content margins.
    fn set_content_margins(&mut self, margins: ContentMargins);

    /// Get the spacing between items.
    fn spacing(&self) -> f32;

    /// Set the spacing between items.
    fn set_spacing(&mut self, spacing: f32);

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    /// Calculate the layout given the available size.
    ///
    /// This method determines the position and size of each item based on
    /// the available space. The actual geometries are stored internally
    /// and applied when `apply()` is called.
    ///
    /// Returns the actual size used by the layout, which may be different
    /// from the available size if the layout cannot use all the space.
    fn calculate<S: WidgetAccess>(&mut self, storage: &S, available: Size) -> Size;

    /// Apply the calculated layout to widgets.
    ///
    /// This sets the geometry of each widget item to its calculated position
    /// and size. Should be called after `calculate()` or when the layout's
    /// geometry changes.
    fn apply<S: WidgetAccess>(&self, storage: &mut S);

    // =========================================================================
    // Invalidation
    // =========================================================================

    /// Invalidate the layout, marking it for recalculation.
    ///
    /// Call this when something changes that affects layout (item added/removed,
    /// widget size hint changed, etc.).
    fn invalidate(&mut self);

    /// Check if the layout needs recalculation.
    fn needs_recalculation(&self) -> bool;

    /// Activate the layout.
    ///
    /// If the layout is invalid, this recalculates and applies the layout.
    /// This is typically called automatically during the next paint cycle.
    fn activate<S: WidgetAccess>(&mut self, storage: &mut S) {
        if self.needs_recalculation() {
            let geo = self.geometry();
            self.calculate(storage, geo.size);
            self.apply(storage);
        }
    }

    // =========================================================================
    // Ownership
    // =========================================================================

    /// Get the parent widget that owns this layout.
    fn parent_widget(&self) -> Option<ObjectId>;

    /// Set the parent widget.
    fn set_parent_widget(&mut self, parent: Option<ObjectId>);
}

// Implement LayoutItemData for any Layout to allow nesting
impl<L: Layout + Clone + std::fmt::Debug + 'static> LayoutItemData for L {
    fn size_hint(&self) -> SizeHint {
        // For nested layouts, we need storage access. This is a limitation.
        // In practice, nested layouts should cache their size hints.
        SizeHint::default()
    }

    fn minimum_size(&self) -> Size {
        Size::ZERO
    }

    fn maximum_size(&self) -> Size {
        Size::new(f32::MAX, f32::MAX)
    }

    fn size_policy(&self) -> SizePolicyPair {
        Layout::size_policy(self)
    }

    fn is_empty(&self) -> bool {
        Layout::is_empty(self)
    }

    fn clone_box(&self) -> Box<dyn LayoutItemData + Send + Sync> {
        Box::new(self.clone())
    }
}
