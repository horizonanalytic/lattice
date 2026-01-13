//! Layout items that can be managed by a layout.
//!
//! A layout manages a collection of items which can be:
//! - Widgets (referenced by ObjectId)
//! - Spacers (fixed or expanding empty space)
//! - Nested layouts

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::Size;

use crate::widget::geometry::{SizeHint, SizePolicy, SizePolicyPair};

/// Type of spacer item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpacerType {
    /// Fixed-size spacer that does not expand.
    Fixed,
    /// Expanding spacer that grows to fill available space.
    Expanding,
    /// Minimum size with expansion (like MinimumExpanding policy).
    MinimumExpanding,
}

impl SpacerType {
    /// Convert to equivalent size policy.
    pub fn to_size_policy(self) -> SizePolicy {
        match self {
            SpacerType::Fixed => SizePolicy::Fixed,
            SpacerType::Expanding => SizePolicy::Expanding,
            SpacerType::MinimumExpanding => SizePolicy::MinimumExpanding,
        }
    }
}

/// A spacer item that adds empty space in a layout.
///
/// Spacers can be fixed-size or expanding. Expanding spacers grow to fill
/// available space, which is useful for pushing widgets apart or centering them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacerItem {
    /// The preferred/base size of the spacer.
    pub size: Size,
    /// The horizontal spacer type.
    pub horizontal: SpacerType,
    /// The vertical spacer type.
    pub vertical: SpacerType,
}

impl SpacerItem {
    /// Create a new spacer item.
    pub fn new(size: Size, horizontal: SpacerType, vertical: SpacerType) -> Self {
        Self {
            size,
            horizontal,
            vertical,
        }
    }

    /// Create a fixed-size spacer.
    pub fn fixed(width: f32, height: f32) -> Self {
        Self::new(
            Size::new(width, height),
            SpacerType::Fixed,
            SpacerType::Fixed,
        )
    }

    /// Create an expanding spacer (grows in both directions).
    pub fn expanding() -> Self {
        Self::new(Size::ZERO, SpacerType::Expanding, SpacerType::Expanding)
    }

    /// Create a horizontal expanding spacer.
    pub fn horizontal_expanding() -> Self {
        Self::new(Size::ZERO, SpacerType::Expanding, SpacerType::Fixed)
    }

    /// Create a vertical expanding spacer.
    pub fn vertical_expanding() -> Self {
        Self::new(Size::ZERO, SpacerType::Fixed, SpacerType::Expanding)
    }

    /// Create a horizontal fixed spacer.
    pub fn horizontal_fixed(width: f32) -> Self {
        Self::new(Size::new(width, 0.0), SpacerType::Fixed, SpacerType::Fixed)
    }

    /// Create a vertical fixed spacer.
    pub fn vertical_fixed(height: f32) -> Self {
        Self::new(Size::new(0.0, height), SpacerType::Fixed, SpacerType::Fixed)
    }

    /// Get the size hint for this spacer.
    pub fn size_hint(&self) -> SizeHint {
        match (self.horizontal, self.vertical) {
            (SpacerType::Fixed, SpacerType::Fixed) => SizeHint::fixed(self.size),
            _ => SizeHint::new(self.size),
        }
    }

    /// Get the size policy for this spacer.
    pub fn size_policy(&self) -> SizePolicyPair {
        SizePolicyPair::new(
            self.horizontal.to_size_policy(),
            self.vertical.to_size_policy(),
        )
    }
}

impl Default for SpacerItem {
    fn default() -> Self {
        Self::expanding()
    }
}

/// An item managed by a layout.
///
/// Layout items can be widgets, spacers, or nested layouts. Each item
/// participates in the layout algorithm to determine its final position
/// and size.
#[derive(Debug, Clone)]
pub enum LayoutItem {
    /// A widget managed by its ObjectId.
    Widget(ObjectId),

    /// A spacer item for adding empty space.
    Spacer(SpacerItem),

    /// A nested layout.
    ///
    /// Nested layouts allow creating complex arrangements. The nested layout
    /// is stored as a boxed trait object to allow different layout types
    /// to be nested.
    Layout(Box<dyn LayoutItemData + Send + Sync>),
}

impl LayoutItem {
    /// Create a widget layout item.
    pub fn widget(id: ObjectId) -> Self {
        Self::Widget(id)
    }

    /// Create a spacer layout item.
    pub fn from_spacer(spacer: SpacerItem) -> Self {
        Self::Spacer(spacer)
    }

    /// Create a fixed spacer layout item.
    pub fn fixed_spacer(width: f32, height: f32) -> Self {
        Self::Spacer(SpacerItem::fixed(width, height))
    }

    /// Create an expanding spacer layout item.
    pub fn stretch() -> Self {
        Self::Spacer(SpacerItem::expanding())
    }

    /// Check if this item is a widget.
    pub fn is_widget(&self) -> bool {
        matches!(self, Self::Widget(_))
    }

    /// Check if this item is a spacer.
    pub fn is_spacer(&self) -> bool {
        matches!(self, Self::Spacer(_))
    }

    /// Check if this item is a nested layout.
    pub fn is_layout(&self) -> bool {
        matches!(self, Self::Layout(_))
    }

    /// Get the widget ID if this is a widget item.
    pub fn widget_id(&self) -> Option<ObjectId> {
        match self {
            Self::Widget(id) => Some(*id),
            _ => None,
        }
    }

    /// Get the spacer if this is a spacer item.
    pub fn spacer(&self) -> Option<&SpacerItem> {
        match self {
            Self::Spacer(s) => Some(s),
            _ => None,
        }
    }
}

/// Trait for getting layout item data.
///
/// This trait is implemented by layouts to allow them to be used as nested
/// layout items. It provides the essential information needed for the parent
/// layout to incorporate the nested layout in its calculations.
pub trait LayoutItemData: std::fmt::Debug {
    /// Get the size hint for this item.
    fn size_hint(&self) -> SizeHint;

    /// Get the minimum size for this item.
    fn minimum_size(&self) -> Size;

    /// Get the maximum size for this item.
    fn maximum_size(&self) -> Size;

    /// Get the size policy for this item.
    fn size_policy(&self) -> SizePolicyPair;

    /// Check if this item is empty (contains no visible content).
    fn is_empty(&self) -> bool;

    /// Clone the layout item data into a box.
    fn clone_box(&self) -> Box<dyn LayoutItemData + Send + Sync>;
}

impl Clone for Box<dyn LayoutItemData + Send + Sync> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spacer_item_fixed() {
        let spacer = SpacerItem::fixed(10.0, 20.0);
        assert_eq!(spacer.size, Size::new(10.0, 20.0));
        assert_eq!(spacer.horizontal, SpacerType::Fixed);
        assert_eq!(spacer.vertical, SpacerType::Fixed);

        let hint = spacer.size_hint();
        assert!(hint.is_fixed());
    }

    #[test]
    fn test_spacer_item_expanding() {
        let spacer = SpacerItem::expanding();
        assert_eq!(spacer.size, Size::ZERO);
        assert_eq!(spacer.horizontal, SpacerType::Expanding);
        assert_eq!(spacer.vertical, SpacerType::Expanding);

        let policy = spacer.size_policy();
        assert!(policy.horizontal.wants_to_grow());
        assert!(policy.vertical.wants_to_grow());
    }

    #[test]
    fn test_layout_item_widget() {
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        // We can't easily create a real ObjectId in tests, so just test the enum
        let spacer_item = LayoutItem::stretch();
        assert!(spacer_item.is_spacer());
        assert!(!spacer_item.is_widget());
        assert!(spacer_item.widget_id().is_none());
    }

    #[test]
    fn test_content_margins() {
        use crate::widget::layout::ContentMargins;

        let margins = ContentMargins::new(1.0, 2.0, 3.0, 4.0);
        assert_eq!(margins.horizontal(), 4.0);
        assert_eq!(margins.vertical(), 6.0);

        let uniform = ContentMargins::uniform(5.0);
        assert_eq!(uniform.left, 5.0);
        assert_eq!(uniform.right, 5.0);
        assert_eq!(uniform.horizontal(), 10.0);
    }
}
