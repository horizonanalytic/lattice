//! Layout system for automatic widget positioning and sizing.
//!
//! This module provides the foundational layout architecture including:
//!
//! - [`Layout`] trait: The base trait for all layout managers
//! - [`LayoutItem`]: Items that can be managed by a layout
//! - [`LayoutBase`]: Common implementation for layout functionality
//! - [`ContentMargins`]: Spacing around layout content
//!
//! # Built-in Layouts
//!
//! - [`HBoxLayout`] / [`VBoxLayout`] - Horizontal and vertical box layouts
//! - [`GridLayout`] - Row/column grid layout
//! - [`FormLayout`] - Two-column form with labels and fields
//! - [`StackLayout`] - Stacked widgets (only one visible at a time)
//! - [`FlowLayout`] - Flow-based wrapping layout
//! - [`AnchorLayout`] - Constraint-based anchoring
//!
//! # Related Types
//!
//! - [`super::Widget`] - Widgets are positioned by layouts
//! - [`super::SizeHint`] - Size preferences for layout calculation
//! - [`super::SizePolicy`] - How widgets grow/shrink during layout
//! - [`SpacerItem`] - Flexible spacing between widgets
//!
//! # Overview
//!
//! The layout system follows Qt's design philosophy while being idiomatic Rust.
//! Layouts manage the positioning and sizing of widgets automatically based on
//! size hints, size policies, and available space.
//!
//! # Layout Algorithm
//!
//! Layouts use a two-pass algorithm:
//!
//! 1. **Bottom-up pass**: Collect size hints from all items to determine
//!    the layout's own size requirements.
//! 2. **Top-down pass**: Distribute available space to items based on their
//!    size policies and stretch factors.
//!
//! # Creating a Layout
//!
//! To create a custom layout:
//!
//! 1. Implement the [`Layout`] trait
//! 2. Use [`LayoutBase`] for common functionality
//! 3. Implement the layout algorithm in `calculate()` and `apply()`
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::layout::{HBoxLayout, ContentMargins, Layout};
//!
//! // Create a horizontal box layout
//! let mut layout = HBoxLayout::new();
//! layout.set_spacing(10.0);
//! layout.set_content_margins(ContentMargins::uniform(8.0));
//!
//! // Add widgets (widget IDs come from the widget system)
//! layout.add_widget(button1.id());
//! layout.add_widget(button2.id());
//! layout.add_stretch(1); // Flexible spacer
//! layout.add_widget(button3.id());
//! ```
//!
//! # Guide
//!
//! For a comprehensive guide on the layout system, see the
//! [Layout Guide](https://horizonanalytic.github.io/lattice/guides/layouts.html).

mod anchor_layout;
mod base;
mod box_layout;
mod flow_layout;
mod form_layout;
mod grid_layout;
mod invalidation;
mod item;
mod stack_layout;
mod traits;

pub use anchor_layout::{Anchor, AnchorLayout, AnchorLine, AnchorTarget};
pub use base::LayoutBase;
pub use box_layout::{Alignment, BoxLayout, HBoxLayout, Orientation, VBoxLayout};
pub use flow_layout::FlowLayout;
pub use form_layout::{FieldGrowthPolicy, FormItemRole, FormLayout, FormRow, RowWrapPolicy};
pub use grid_layout::{CellAlignment, GridLayout};
pub use invalidation::{InvalidationScope, LayoutInvalidator};
pub use item::{LayoutItem, SpacerItem, SpacerType};
pub use stack_layout::{StackLayout, StackSizeMode};
pub use traits::Layout;

use crate::widget::dispatcher::WidgetAccess;
use crate::widget::geometry::{SizeHint, SizePolicyPair};
use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

/// Content margins around a layout.
///
/// Margins define the spacing between the layout's content and its edges.
/// This is used to add padding around all items in a layout.
///
/// # Related
///
/// - [`Layout::set_content_margins`] - Set margins on a layout
/// - [`LayoutBase`] - Stores content margins
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ContentMargins {
    /// Left margin.
    pub left: f32,
    /// Top margin.
    pub top: f32,
    /// Right margin.
    pub right: f32,
    /// Bottom margin.
    pub bottom: f32,
}

impl ContentMargins {
    /// Create new content margins.
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Create uniform margins (same value on all sides).
    pub fn uniform(margin: f32) -> Self {
        Self::new(margin, margin, margin, margin)
    }

    /// Create symmetric margins (same horizontal and vertical).
    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self::new(horizontal, vertical, horizontal, vertical)
    }

    /// Total horizontal margin (left + right).
    #[inline]
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Total vertical margin (top + bottom).
    #[inline]
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    /// Size occupied by margins.
    #[inline]
    pub fn size(&self) -> Size {
        Size::new(self.horizontal(), self.vertical())
    }
}

/// Default spacing between items in a layout.
pub const DEFAULT_SPACING: f32 = 6.0;

/// Default content margins for layouts.
pub const DEFAULT_MARGINS: ContentMargins = ContentMargins {
    left: 9.0,
    top: 9.0,
    right: 9.0,
    bottom: 9.0,
};

/// An enum wrapping all concrete layout types.
///
/// Since the [`Layout`] trait is not dyn-safe (it has methods with generic parameters),
/// this enum provides a way to store any layout type and dispatch to the underlying
/// implementation. This is the recommended way to store layouts in widgets that need
/// to support multiple layout types.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::layout::{LayoutKind, BoxLayout, GridLayout};
///
/// // Create a layout kind from a box layout
/// let layout = LayoutKind::from(BoxLayout::horizontal());
///
/// // Or use convenience constructors
/// let layout = LayoutKind::horizontal();
/// let layout = LayoutKind::vertical();
/// ```
#[derive(Debug, Clone)]
pub enum LayoutKind {
    /// Box layout (horizontal or vertical).
    Box(BoxLayout),
    /// Grid layout.
    Grid(GridLayout),
    /// Form layout.
    Form(FormLayout),
    /// Stack layout.
    Stack(StackLayout),
    /// Flow layout.
    Flow(FlowLayout),
    /// Anchor layout.
    Anchor(AnchorLayout),
}

impl LayoutKind {
    /// Create a horizontal box layout.
    pub fn horizontal() -> Self {
        Self::Box(BoxLayout::horizontal())
    }

    /// Create a vertical box layout.
    pub fn vertical() -> Self {
        Self::Box(BoxLayout::vertical())
    }

    /// Create a grid layout.
    pub fn grid() -> Self {
        Self::Grid(GridLayout::new())
    }

    /// Create a form layout.
    pub fn form() -> Self {
        Self::Form(FormLayout::new())
    }

    /// Create a stack layout.
    pub fn stack() -> Self {
        Self::Stack(StackLayout::new())
    }

    /// Create a flow layout.
    pub fn flow() -> Self {
        Self::Flow(FlowLayout::new())
    }

    /// Create an anchor layout.
    pub fn anchor() -> Self {
        Self::Anchor(AnchorLayout::new())
    }

    // =========================================================================
    // Delegation Methods
    // =========================================================================

    /// Add an item to the layout.
    pub fn add_item(&mut self, item: LayoutItem) {
        match self {
            Self::Box(l) => l.add_item(item),
            Self::Grid(l) => l.add_item(item),
            Self::Form(l) => l.add_item(item),
            Self::Stack(l) => l.add_item(item),
            Self::Flow(l) => l.add_item(item),
            Self::Anchor(l) => l.add_item(item),
        }
    }

    /// Add a widget to the layout.
    pub fn add_widget(&mut self, widget: ObjectId) {
        self.add_item(LayoutItem::Widget(widget));
    }

    /// Insert an item at a specific index.
    pub fn insert_item(&mut self, index: usize, item: LayoutItem) {
        match self {
            Self::Box(l) => l.insert_item(index, item),
            Self::Grid(l) => l.insert_item(index, item),
            Self::Form(l) => l.insert_item(index, item),
            Self::Stack(l) => l.insert_item(index, item),
            Self::Flow(l) => l.insert_item(index, item),
            Self::Anchor(l) => l.insert_item(index, item),
        }
    }

    /// Remove an item at the specified index.
    pub fn remove_item(&mut self, index: usize) -> Option<LayoutItem> {
        match self {
            Self::Box(l) => l.remove_item(index),
            Self::Grid(l) => l.remove_item(index),
            Self::Form(l) => l.remove_item(index),
            Self::Stack(l) => l.remove_item(index),
            Self::Flow(l) => l.remove_item(index),
            Self::Anchor(l) => l.remove_item(index),
        }
    }

    /// Remove a widget by its ObjectId.
    pub fn remove_widget(&mut self, widget: ObjectId) -> bool {
        match self {
            Self::Box(l) => l.remove_widget(widget),
            Self::Grid(l) => l.remove_widget(widget),
            Self::Form(l) => l.remove_widget(widget),
            Self::Stack(l) => l.remove_widget(widget),
            Self::Flow(l) => l.remove_widget(widget),
            Self::Anchor(l) => l.remove_widget(widget),
        }
    }

    /// Get the number of items.
    pub fn item_count(&self) -> usize {
        match self {
            Self::Box(l) => l.item_count(),
            Self::Grid(l) => l.item_count(),
            Self::Form(l) => l.item_count(),
            Self::Stack(l) => l.item_count(),
            Self::Flow(l) => l.item_count(),
            Self::Anchor(l) => l.item_count(),
        }
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        match self {
            Self::Box(l) => l.clear(),
            Self::Grid(l) => l.clear(),
            Self::Form(l) => l.clear(),
            Self::Stack(l) => l.clear(),
            Self::Flow(l) => l.clear(),
            Self::Anchor(l) => l.clear(),
        }
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.item_count() == 0
    }

    /// Get the layout's geometry.
    pub fn geometry(&self) -> Rect {
        match self {
            Self::Box(l) => l.geometry(),
            Self::Grid(l) => l.geometry(),
            Self::Form(l) => l.geometry(),
            Self::Stack(l) => l.geometry(),
            Self::Flow(l) => l.geometry(),
            Self::Anchor(l) => l.geometry(),
        }
    }

    /// Set the layout's geometry.
    pub fn set_geometry(&mut self, rect: Rect) {
        match self {
            Self::Box(l) => l.set_geometry(rect),
            Self::Grid(l) => l.set_geometry(rect),
            Self::Form(l) => l.set_geometry(rect),
            Self::Stack(l) => l.set_geometry(rect),
            Self::Flow(l) => l.set_geometry(rect),
            Self::Anchor(l) => l.set_geometry(rect),
        }
    }

    /// Get content margins.
    pub fn content_margins(&self) -> ContentMargins {
        match self {
            Self::Box(l) => l.content_margins(),
            Self::Grid(l) => l.content_margins(),
            Self::Form(l) => l.content_margins(),
            Self::Stack(l) => l.content_margins(),
            Self::Flow(l) => l.content_margins(),
            Self::Anchor(l) => l.content_margins(),
        }
    }

    /// Set content margins.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        match self {
            Self::Box(l) => l.set_content_margins(margins),
            Self::Grid(l) => l.set_content_margins(margins),
            Self::Form(l) => l.set_content_margins(margins),
            Self::Stack(l) => l.set_content_margins(margins),
            Self::Flow(l) => l.set_content_margins(margins),
            Self::Anchor(l) => l.set_content_margins(margins),
        }
    }

    /// Get spacing.
    pub fn spacing(&self) -> f32 {
        match self {
            Self::Box(l) => l.spacing(),
            Self::Grid(l) => l.spacing(),
            Self::Form(l) => l.spacing(),
            Self::Stack(l) => l.spacing(),
            Self::Flow(l) => l.spacing(),
            Self::Anchor(l) => l.spacing(),
        }
    }

    /// Set spacing.
    pub fn set_spacing(&mut self, spacing: f32) {
        match self {
            Self::Box(l) => l.set_spacing(spacing),
            Self::Grid(l) => l.set_spacing(spacing),
            Self::Form(l) => l.set_spacing(spacing),
            Self::Stack(l) => l.set_spacing(spacing),
            Self::Flow(l) => l.set_spacing(spacing),
            Self::Anchor(l) => l.set_spacing(spacing),
        }
    }

    /// Get size hint.
    pub fn size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        match self {
            Self::Box(l) => l.size_hint(storage),
            Self::Grid(l) => l.size_hint(storage),
            Self::Form(l) => l.size_hint(storage),
            Self::Stack(l) => l.size_hint(storage),
            Self::Flow(l) => l.size_hint(storage),
            Self::Anchor(l) => l.size_hint(storage),
        }
    }

    /// Get minimum size.
    pub fn minimum_size<S: WidgetAccess>(&self, storage: &S) -> Size {
        match self {
            Self::Box(l) => l.minimum_size(storage),
            Self::Grid(l) => l.minimum_size(storage),
            Self::Form(l) => l.minimum_size(storage),
            Self::Stack(l) => l.minimum_size(storage),
            Self::Flow(l) => l.minimum_size(storage),
            Self::Anchor(l) => l.minimum_size(storage),
        }
    }

    /// Get size policy.
    pub fn size_policy(&self) -> SizePolicyPair {
        match self {
            Self::Box(l) => l.size_policy(),
            Self::Grid(l) => l.size_policy(),
            Self::Form(l) => l.size_policy(),
            Self::Stack(l) => l.size_policy(),
            Self::Flow(l) => l.size_policy(),
            Self::Anchor(l) => l.size_policy(),
        }
    }

    /// Calculate the layout.
    pub fn calculate<S: WidgetAccess>(&mut self, storage: &S, available: Size) -> Size {
        match self {
            Self::Box(l) => l.calculate(storage, available),
            Self::Grid(l) => l.calculate(storage, available),
            Self::Form(l) => l.calculate(storage, available),
            Self::Stack(l) => l.calculate(storage, available),
            Self::Flow(l) => l.calculate(storage, available),
            Self::Anchor(l) => l.calculate(storage, available),
        }
    }

    /// Apply the layout.
    pub fn apply<S: WidgetAccess>(&self, storage: &mut S) {
        match self {
            Self::Box(l) => l.apply(storage),
            Self::Grid(l) => l.apply(storage),
            Self::Form(l) => l.apply(storage),
            Self::Stack(l) => l.apply(storage),
            Self::Flow(l) => l.apply(storage),
            Self::Anchor(l) => l.apply(storage),
        }
    }

    /// Invalidate the layout.
    pub fn invalidate(&mut self) {
        match self {
            Self::Box(l) => l.invalidate(),
            Self::Grid(l) => l.invalidate(),
            Self::Form(l) => l.invalidate(),
            Self::Stack(l) => l.invalidate(),
            Self::Flow(l) => l.invalidate(),
            Self::Anchor(l) => l.invalidate(),
        }
    }

    /// Check if layout needs recalculation.
    pub fn needs_recalculation(&self) -> bool {
        match self {
            Self::Box(l) => l.needs_recalculation(),
            Self::Grid(l) => l.needs_recalculation(),
            Self::Form(l) => l.needs_recalculation(),
            Self::Stack(l) => l.needs_recalculation(),
            Self::Flow(l) => l.needs_recalculation(),
            Self::Anchor(l) => l.needs_recalculation(),
        }
    }

    /// Get the parent widget.
    pub fn parent_widget(&self) -> Option<ObjectId> {
        match self {
            Self::Box(l) => l.parent_widget(),
            Self::Grid(l) => l.parent_widget(),
            Self::Form(l) => l.parent_widget(),
            Self::Stack(l) => l.parent_widget(),
            Self::Flow(l) => l.parent_widget(),
            Self::Anchor(l) => l.parent_widget(),
        }
    }

    /// Set the parent widget.
    pub fn set_parent_widget(&mut self, parent: Option<ObjectId>) {
        match self {
            Self::Box(l) => l.set_parent_widget(parent),
            Self::Grid(l) => l.set_parent_widget(parent),
            Self::Form(l) => l.set_parent_widget(parent),
            Self::Stack(l) => l.set_parent_widget(parent),
            Self::Flow(l) => l.set_parent_widget(parent),
            Self::Anchor(l) => l.set_parent_widget(parent),
        }
    }
}

impl From<BoxLayout> for LayoutKind {
    fn from(layout: BoxLayout) -> Self {
        Self::Box(layout)
    }
}

impl From<GridLayout> for LayoutKind {
    fn from(layout: GridLayout) -> Self {
        Self::Grid(layout)
    }
}

impl From<FormLayout> for LayoutKind {
    fn from(layout: FormLayout) -> Self {
        Self::Form(layout)
    }
}

impl From<StackLayout> for LayoutKind {
    fn from(layout: StackLayout) -> Self {
        Self::Stack(layout)
    }
}

impl From<FlowLayout> for LayoutKind {
    fn from(layout: FlowLayout) -> Self {
        Self::Flow(layout)
    }
}

impl From<AnchorLayout> for LayoutKind {
    fn from(layout: AnchorLayout) -> Self {
        Self::Anchor(layout)
    }
}
