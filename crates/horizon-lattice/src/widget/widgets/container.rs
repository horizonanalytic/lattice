//! Generic container widget implementation.
//!
//! This module provides [`ContainerWidget`], a simple container widget that can
//! hold child widgets and optionally manage them with a layout.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::ContainerWidget;
//! use horizon_lattice::widget::layout::LayoutKind;
//!
//! // Create a container with a vertical layout
//! let mut container = ContainerWidget::new();
//! container.set_layout(LayoutKind::vertical());
//!
//! // Add child widgets to the layout
//! container.add_child(button1_id);
//! container.add_child(button2_id);
//!
//! // Or create with background color
//! let panel = ContainerWidget::new()
//!     .with_background_color(Color::from_rgb8(240, 240, 240));
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Rect, Renderer, Size};

use crate::widget::dispatcher::WidgetAccess;
use crate::widget::layout::{ContentMargins, LayoutItem, LayoutKind};
use crate::widget::{
    PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

/// A generic container widget that can hold child widgets.
///
/// `ContainerWidget` is a simple, flexible container that:
/// - Holds child widgets
/// - Can be assigned any layout for automatic child positioning
/// - Optionally paints a background color
/// - Provides content margins for padding
///
/// This is analogous to Qt's `QWidget` when used as a container, providing
/// a base for grouping widgets without the border decoration of [`Frame`].
///
/// # Layout Support
///
/// When a layout is assigned to the container:
/// - Child widgets added via [`add_child`] are added to the layout
/// - The layout's [`calculate`] and [`apply`] methods can be called to position children
/// - The container's size hint is based on the layout's size requirements
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::widgets::ContainerWidget;
/// use horizon_lattice::widget::layout::LayoutKind;
///
/// let mut container = ContainerWidget::new();
/// container.set_layout(LayoutKind::vertical());
/// container.set_background_color(Some(Color::WHITE));
/// container.add_child(label_id);
/// container.add_child(input_id);
/// ```
///
/// # Signals
///
/// - `children_changed()`: Emitted when children are added or removed
///
/// [`Frame`]: super::Frame
/// [`add_child`]: ContainerWidget::add_child
/// [`calculate`]: LayoutKind::calculate
/// [`apply`]: LayoutKind::apply
pub struct ContainerWidget {
    /// Widget base.
    base: WidgetBase,

    /// Child widget IDs.
    children: Vec<ObjectId>,

    /// Optional layout for child positioning.
    layout: Option<LayoutKind>,

    /// Content margins around children.
    content_margins: ContentMargins,

    /// Background color (if any).
    background_color: Option<Color>,

    /// Signal emitted when children are added or removed.
    pub children_changed: Signal<()>,
}

impl ContainerWidget {
    /// Create a new container widget with default settings.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));

        Self {
            base,
            children: Vec::new(),
            layout: None,
            content_margins: ContentMargins::uniform(0.0),
            background_color: None,
            children_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Layout Management
    // =========================================================================

    /// Get the layout, if any.
    pub fn layout(&self) -> Option<&LayoutKind> {
        self.layout.as_ref()
    }

    /// Get a mutable reference to the layout, if any.
    pub fn layout_mut(&mut self) -> Option<&mut LayoutKind> {
        self.layout.as_mut()
    }

    /// Set the layout for this container.
    ///
    /// The layout will manage the positioning of child widgets.
    /// Existing children will be added to the new layout.
    pub fn set_layout(&mut self, layout: LayoutKind) {
        let mut new_layout = layout;
        new_layout.set_parent_widget(Some(self.base.object_id()));

        // Add existing children to the new layout
        for &child_id in &self.children {
            new_layout.add_widget(child_id);
        }

        self.layout = Some(new_layout);
        self.base.update();
    }

    /// Set layout using builder pattern.
    pub fn with_layout(mut self, layout: LayoutKind) -> Self {
        self.set_layout(layout);
        self
    }

    /// Take the layout from this container, removing it.
    pub fn take_layout(&mut self) -> Option<LayoutKind> {
        let layout = self.layout.take();
        self.base.update();
        layout
    }

    /// Check if the container has a layout.
    #[inline]
    pub fn has_layout(&self) -> bool {
        self.layout.is_some()
    }

    // =========================================================================
    // Child Management
    // =========================================================================

    /// Add a child widget to this container.
    ///
    /// If a layout is set, the widget is also added to the layout.
    /// Returns the index of the new child.
    pub fn add_child(&mut self, widget_id: ObjectId) -> usize {
        self.children.push(widget_id);

        if let Some(layout) = &mut self.layout {
            layout.add_widget(widget_id);
        }

        self.base.update();
        self.children_changed.emit(());
        self.children.len() - 1
    }

    /// Insert a child widget at the specified index.
    ///
    /// Returns the actual index where the widget was inserted.
    pub fn insert_child(&mut self, index: usize, widget_id: ObjectId) -> usize {
        let insert_pos = index.min(self.children.len());
        self.children.insert(insert_pos, widget_id);

        if let Some(layout) = &mut self.layout {
            layout.insert_item(insert_pos, LayoutItem::Widget(widget_id));
        }

        self.base.update();
        self.children_changed.emit(());
        insert_pos
    }

    /// Remove the child widget at the specified index.
    ///
    /// Returns the widget ID of the removed child, if any.
    pub fn remove_child(&mut self, index: usize) -> Option<ObjectId> {
        if index >= self.children.len() {
            return None;
        }

        let widget_id = self.children.remove(index);

        if let Some(layout) = &mut self.layout {
            layout.remove_item(index);
        }

        self.base.update();
        self.children_changed.emit(());
        Some(widget_id)
    }

    /// Remove a child widget by its ID.
    ///
    /// Returns `true` if the widget was found and removed.
    pub fn remove_child_by_id(&mut self, widget_id: ObjectId) -> bool {
        if let Some(index) = self.children.iter().position(|&id| id == widget_id) {
            self.remove_child(index);
            true
        } else {
            false
        }
    }

    /// Remove all children from the container.
    pub fn clear(&mut self) {
        self.children.clear();

        if let Some(layout) = &mut self.layout {
            layout.clear();
        }

        self.base.update();
        self.children_changed.emit(());
    }

    /// Get the number of children.
    #[inline]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Check if the container has no children.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Get the child widget IDs.
    #[inline]
    pub fn children(&self) -> &[ObjectId] {
        &self.children
    }

    /// Get the child at the specified index.
    #[inline]
    pub fn child_at(&self, index: usize) -> Option<ObjectId> {
        self.children.get(index).copied()
    }

    /// Find the index of a child widget.
    ///
    /// Returns `None` if the widget is not a child of this container.
    pub fn index_of(&self, widget_id: ObjectId) -> Option<usize> {
        self.children.iter().position(|&id| id == widget_id)
    }

    // =========================================================================
    // Content Margins
    // =========================================================================

    /// Get the content margins.
    #[inline]
    pub fn content_margins(&self) -> ContentMargins {
        self.content_margins
    }

    /// Set the content margins.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        if self.content_margins != margins {
            self.content_margins = margins;
            if let Some(layout) = &mut self.layout {
                layout.set_content_margins(margins);
            }
            self.base.update();
        }
    }

    /// Set uniform content margins.
    pub fn set_content_margin(&mut self, margin: f32) {
        self.set_content_margins(ContentMargins::uniform(margin));
    }

    /// Set content margins using builder pattern.
    pub fn with_content_margins(mut self, margins: ContentMargins) -> Self {
        self.set_content_margins(margins);
        self
    }

    /// Set uniform content margins using builder pattern.
    pub fn with_content_margin(mut self, margin: f32) -> Self {
        self.set_content_margin(margin);
        self
    }

    // =========================================================================
    // Background Color
    // =========================================================================

    /// Get the background color.
    #[inline]
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

    /// Clear the background color (transparent).
    pub fn clear_background(&mut self) {
        self.set_background_color(None);
    }

    // =========================================================================
    // Content Area
    // =========================================================================

    /// Get the content area rectangle (inside margins).
    pub fn contents_rect(&self) -> Rect {
        let rect = self.base.rect();

        Rect::new(
            self.content_margins.left,
            self.content_margins.top,
            (rect.width() - self.content_margins.horizontal()).max(0.0),
            (rect.height() - self.content_margins.vertical()).max(0.0),
        )
    }

    // =========================================================================
    // Layout Operations
    // =========================================================================

    /// Calculate and apply the layout using the provided widget storage.
    ///
    /// This method should be called when the container's geometry changes
    /// or when children are added/removed.
    ///
    /// # Arguments
    ///
    /// * `storage` - The widget storage that provides access to child widgets
    pub fn do_layout<S: WidgetAccess>(&mut self, storage: &mut S) {
        // Calculate layout rect before borrowing layout mutably
        let content_rect = self.contents_rect();
        let geo = self.base.geometry();

        // Translate content rect to parent coordinates
        let layout_rect = Rect::new(
            geo.origin.x + content_rect.origin.x,
            geo.origin.y + content_rect.origin.y,
            content_rect.width(),
            content_rect.height(),
        );

        if let Some(layout) = &mut self.layout {
            layout.set_geometry(layout_rect);
            layout.calculate(storage, layout_rect.size);
            layout.apply(storage);
        }
    }

    /// Invalidate the layout, marking it for recalculation.
    pub fn invalidate_layout(&mut self) {
        if let Some(layout) = &mut self.layout {
            layout.invalidate();
        }
        self.base.update();
    }
}

impl Default for ContainerWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ContainerWidget {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ContainerWidget {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Calculate minimum size based on margins
        let min_width = self.content_margins.horizontal();
        let min_height = self.content_margins.vertical();

        // If we have a layout, use its size hint
        // Note: Layout size hint calculation requires WidgetAccess, which we don't have here.
        // The actual size hint would need to be calculated externally or cached.

        // Default preferred size
        let preferred = Size::new(min_width.max(100.0), min_height.max(100.0));

        SizeHint::new(preferred).with_minimum(Size::new(min_width, min_height))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Draw background if set
        if let Some(bg_color) = self.background_color {
            ctx.renderer().fill_rect(rect, bg_color);
        }

        // Child widgets are painted separately by the paint system
        // They are not painted here directly
    }

    fn event(&mut self, _event: &mut WidgetEvent) -> bool {
        // Container doesn't handle events itself, they pass through to children
        false
    }
}

// Ensure ContainerWidget is Send + Sync
static_assertions::assert_impl_all!(ContainerWidget: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_container_creation() {
        setup();
        let container = ContainerWidget::new();
        assert!(container.is_empty());
        assert!(!container.has_layout());
        assert!(container.background_color().is_none());
    }

    #[test]
    fn test_container_builder_pattern() {
        setup();
        let container = ContainerWidget::new()
            .with_background_color(Color::WHITE)
            .with_content_margin(8.0);

        assert_eq!(container.background_color(), Some(Color::WHITE));
        assert_eq!(container.content_margins().left, 8.0);
    }

    #[test]
    fn test_container_with_layout() {
        setup();
        let mut container = ContainerWidget::new();

        container.set_layout(LayoutKind::vertical());

        assert!(container.has_layout());
        assert!(container.layout().is_some());
    }

    #[test]
    fn test_child_management() {
        setup();
        let mut container = ContainerWidget::new();
        let container2 = ContainerWidget::new();
        let child1_id = container2.object_id();

        // Add a child
        let index = container.add_child(child1_id);
        assert_eq!(index, 0);
        assert_eq!(container.child_count(), 1);
        assert_eq!(container.child_at(0), Some(child1_id));

        // Find child index
        assert_eq!(container.index_of(child1_id), Some(0));

        // Remove child
        let removed = container.remove_child(0);
        assert_eq!(removed, Some(child1_id));
        assert!(container.is_empty());
    }

    #[test]
    fn test_child_management_with_layout() {
        setup();
        let mut container = ContainerWidget::new();
        container.set_layout(LayoutKind::vertical());

        let child_widget = ContainerWidget::new();
        let child_id = child_widget.object_id();

        // Add child - should also be added to layout
        container.add_child(child_id);
        assert_eq!(container.child_count(), 1);
        assert_eq!(container.layout().unwrap().item_count(), 1);

        // Remove child - should also be removed from layout
        container.remove_child(0);
        assert!(container.is_empty());
        assert_eq!(container.layout().unwrap().item_count(), 0);
    }

    #[test]
    fn test_insert_child() {
        setup();
        let mut container = ContainerWidget::new();
        let c1 = ContainerWidget::new();
        let c2 = ContainerWidget::new();
        let c3 = ContainerWidget::new();
        let id1 = c1.object_id();
        let id2 = c2.object_id();
        let id3 = c3.object_id();

        container.add_child(id1);
        container.add_child(id3);

        // Insert at index 1
        container.insert_child(1, id2);

        assert_eq!(container.children(), &[id1, id2, id3]);
    }

    #[test]
    fn test_content_margins() {
        setup();
        let mut container = ContainerWidget::new().with_content_margin(10.0);

        container
            .widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));

        let content = container.contents_rect();
        assert_eq!(content.origin.x, 10.0);
        assert_eq!(content.origin.y, 10.0);
        assert_eq!(content.width(), 80.0);
        assert_eq!(content.height(), 80.0);
    }

    #[test]
    fn test_contents_rect_with_asymmetric_margins() {
        setup();
        let mut container =
            ContainerWidget::new().with_content_margins(ContentMargins::new(5.0, 10.0, 15.0, 20.0));

        container
            .widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));

        let content = container.contents_rect();
        // left=5, top=10, right=15, bottom=20
        assert_eq!(content.origin.x, 5.0);
        assert_eq!(content.origin.y, 10.0);
        assert_eq!(content.width(), 80.0); // 100 - 5 - 15
        assert_eq!(content.height(), 70.0); // 100 - 10 - 20
    }

    #[test]
    fn test_take_layout() {
        setup();
        let mut container = ContainerWidget::new();
        container.set_layout(LayoutKind::horizontal());

        assert!(container.has_layout());

        let taken = container.take_layout();
        assert!(taken.is_some());
        assert!(!container.has_layout());
    }

    #[test]
    fn test_clear_children() {
        setup();
        let mut container = ContainerWidget::new();
        container.set_layout(LayoutKind::vertical());

        let c1 = ContainerWidget::new();
        let c2 = ContainerWidget::new();
        container.add_child(c1.object_id());
        container.add_child(c2.object_id());

        assert_eq!(container.child_count(), 2);

        container.clear();
        assert!(container.is_empty());
        assert_eq!(container.layout().unwrap().item_count(), 0);
    }

    #[test]
    fn test_size_hint() {
        setup();
        let container = ContainerWidget::new().with_content_margin(20.0);

        let hint = container.size_hint();

        // Minimum should account for margins (20 on each side = 40 total)
        assert!(hint.effective_minimum().width >= 40.0);
        assert!(hint.effective_minimum().height >= 40.0);
    }
}
