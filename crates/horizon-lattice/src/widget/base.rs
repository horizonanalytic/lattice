//! Widget base implementation.
//!
//! This module provides `WidgetBase`, the common implementation details
//! for all widgets. It handles geometry, visibility, enabled state, and
//! coordinates with the object system.

use horizon_lattice_core::{global_registry, Object, ObjectBase, ObjectId, ObjectResult, Signal};
use horizon_lattice_render::{Point, Rect, Size};

use super::geometry::{SizePolicy, SizePolicyPair};

/// The base implementation for all widgets.
///
/// This struct provides common functionality that all widgets need:
/// - Object system integration (ID, parent-child relationships)
/// - Geometry management (position, size)
/// - Size hints and policies for layout
/// - Visibility and enabled state
/// - Coordinate mapping
///
/// Widget implementations typically include this as a field and delegate
/// common operations to it.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::{Widget, WidgetBase, SizeHint};
///
/// struct MyButton {
///     base: WidgetBase,
///     label: String,
/// }
///
/// impl Widget for MyButton {
///     fn widget_base(&self) -> &WidgetBase { &self.base }
///     fn widget_base_mut(&mut self) -> &mut WidgetBase { &mut self.base }
///
///     fn size_hint(&self) -> SizeHint {
///         SizeHint::from_dimensions(100.0, 30.0)
///     }
///
///     // ... other methods
/// }
/// ```
pub struct WidgetBase {
    /// The underlying object base for Object trait implementation.
    object_base: ObjectBase,

    /// The widget's geometry (position relative to parent and size).
    geometry: Rect,

    /// The widget's size policy for layout.
    size_policy: SizePolicyPair,

    /// Whether the widget is visible.
    visible: bool,

    /// Whether the widget is enabled (can receive input).
    enabled: bool,

    /// Whether the widget can receive keyboard focus.
    focusable: bool,

    /// Whether the widget currently has focus.
    focused: bool,

    /// Whether the mouse is currently over this widget.
    hovered: bool,

    /// Whether the widget needs to be repainted.
    needs_repaint: bool,

    /// Signal emitted when the geometry changes.
    pub geometry_changed: Signal<Rect>,

    /// Signal emitted when visibility changes.
    pub visible_changed: Signal<bool>,

    /// Signal emitted when enabled state changes.
    pub enabled_changed: Signal<bool>,
}

impl WidgetBase {
    /// Create a new widget base.
    ///
    /// # Panics
    ///
    /// Panics if the global object registry is not initialized.
    pub fn new<T: Object + 'static>() -> Self {
        Self {
            object_base: ObjectBase::new::<T>(),
            geometry: Rect::ZERO,
            size_policy: SizePolicyPair::default(),
            visible: true,
            enabled: true,
            focusable: false,
            focused: false,
            hovered: false,
            needs_repaint: true,
            geometry_changed: Signal::new(),
            visible_changed: Signal::new(),
            enabled_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Object System Delegation
    // =========================================================================

    /// Get the widget's unique object ID.
    #[inline]
    pub fn object_id(&self) -> ObjectId {
        self.object_base.id()
    }

    /// Get the widget's name.
    pub fn name(&self) -> String {
        self.object_base.name()
    }

    /// Set the widget's name.
    pub fn set_name(&self, name: impl Into<String>) {
        self.object_base.set_name(name);
    }

    /// Get the parent widget's object ID.
    pub fn parent_id(&self) -> Option<ObjectId> {
        self.object_base.parent()
    }

    /// Set the parent widget.
    pub fn set_parent(&self, parent: Option<ObjectId>) -> ObjectResult<()> {
        self.object_base.set_parent(parent)
    }

    /// Get the IDs of child widgets.
    pub fn children_ids(&self) -> Vec<ObjectId> {
        self.object_base.children()
    }

    /// Find a child by name.
    pub fn find_child_by_name(&self, name: &str) -> Option<ObjectId> {
        self.object_base.find_child_by_name(name)
    }

    // =========================================================================
    // Geometry
    // =========================================================================

    /// Get the widget's geometry (position and size).
    #[inline]
    pub fn geometry(&self) -> Rect {
        self.geometry
    }

    /// Set the widget's geometry.
    ///
    /// This will emit `geometry_changed` if the geometry actually changed.
    pub fn set_geometry(&mut self, rect: Rect) {
        if self.geometry != rect {
            self.geometry = rect;
            self.needs_repaint = true;
            self.geometry_changed.emit(rect);
        }
    }

    /// Get the widget's position relative to its parent.
    #[inline]
    pub fn pos(&self) -> Point {
        self.geometry.origin
    }

    /// Set the widget's position relative to its parent.
    pub fn set_pos(&mut self, pos: Point) {
        if self.geometry.origin != pos {
            let new_geometry = Rect {
                origin: pos,
                size: self.geometry.size,
            };
            self.geometry = new_geometry;
            self.geometry_changed.emit(new_geometry);
        }
    }

    /// Move the widget to the specified position.
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.set_pos(Point::new(x, y));
    }

    /// Get the widget's size.
    #[inline]
    pub fn size(&self) -> Size {
        self.geometry.size
    }

    /// Set the widget's size.
    pub fn set_size(&mut self, size: Size) {
        if self.geometry.size != size {
            let new_geometry = Rect {
                origin: self.geometry.origin,
                size,
            };
            self.geometry = new_geometry;
            self.needs_repaint = true;
            self.geometry_changed.emit(new_geometry);
        }
    }

    /// Resize the widget.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.set_size(Size::new(width, height));
    }

    /// Get the widget's width.
    #[inline]
    pub fn width(&self) -> f32 {
        self.geometry.size.width
    }

    /// Get the widget's height.
    #[inline]
    pub fn height(&self) -> f32 {
        self.geometry.size.height
    }

    /// Get a rectangle representing the widget's local coordinate space.
    ///
    /// This is always positioned at (0, 0) with the widget's size.
    #[inline]
    pub fn rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.geometry.size.width, self.geometry.size.height)
    }

    // =========================================================================
    // Size Policy
    // =========================================================================

    /// Get the widget's size policy.
    #[inline]
    pub fn size_policy(&self) -> SizePolicyPair {
        self.size_policy
    }

    /// Set the widget's size policy.
    pub fn set_size_policy(&mut self, policy: SizePolicyPair) {
        self.size_policy = policy;
    }

    /// Set horizontal size policy.
    pub fn set_horizontal_policy(&mut self, policy: SizePolicy) {
        self.size_policy.horizontal = policy;
    }

    /// Set vertical size policy.
    pub fn set_vertical_policy(&mut self, policy: SizePolicy) {
        self.size_policy.vertical = policy;
    }

    // =========================================================================
    // Visibility
    // =========================================================================

    /// Check if the widget is visible.
    ///
    /// Note: A widget may be visible but still not shown on screen if an
    /// ancestor is hidden.
    #[inline]
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set whether the widget is visible.
    pub fn set_visible(&mut self, visible: bool) {
        if self.visible != visible {
            self.visible = visible;
            self.needs_repaint = true;
            self.visible_changed.emit(visible);
        }
    }

    /// Show the widget.
    pub fn show(&mut self) {
        self.set_visible(true);
    }

    /// Hide the widget.
    pub fn hide(&mut self) {
        self.set_visible(false);
    }

    /// Check if the widget is actually visible (considering ancestors).
    ///
    /// Returns true only if this widget and all its ancestors are visible.
    pub fn is_visible_to(&self, ancestor_id: Option<ObjectId>) -> bool {
        if !self.visible {
            return false;
        }

        // Check if we need to traverse ancestors
        if ancestor_id.is_none() {
            return self.visible;
        }

        // Traverse up the tree checking visibility
        let registry = match global_registry() {
            Ok(r) => r,
            Err(_) => return self.visible,
        };

        let mut current = self.parent_id();
        while let Some(id) = current {
            // We would need to access the widget's visible state, but we only
            // have ObjectId here. For now, assume ancestors are visible.
            // Full implementation requires widget registry.
            if Some(id) == ancestor_id {
                break;
            }
            current = registry.parent(id).ok().flatten();
        }

        self.visible
    }

    // =========================================================================
    // Enabled State
    // =========================================================================

    /// Check if the widget is enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Set whether the widget is enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled != enabled {
            self.enabled = enabled;
            self.needs_repaint = true;
            self.enabled_changed.emit(enabled);
        }
    }

    /// Enable the widget.
    pub fn enable(&mut self) {
        self.set_enabled(true);
    }

    /// Disable the widget.
    pub fn disable(&mut self) {
        self.set_enabled(false);
    }

    // =========================================================================
    // Focus
    // =========================================================================

    /// Check if the widget can receive keyboard focus.
    #[inline]
    pub fn is_focusable(&self) -> bool {
        self.focusable && self.enabled && self.visible
    }

    /// Set whether the widget can receive keyboard focus.
    pub fn set_focusable(&mut self, focusable: bool) {
        self.focusable = focusable;
    }

    /// Check if the widget currently has keyboard focus.
    #[inline]
    pub fn has_focus(&self) -> bool {
        self.focused
    }

    /// Set the focused state (used by the focus management system).
    pub(crate) fn set_focused(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.needs_repaint = true;
        }
    }

    // =========================================================================
    // Hover State
    // =========================================================================

    /// Check if the mouse is currently over this widget.
    #[inline]
    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    /// Set the hover state (used by the event system).
    pub(crate) fn set_hovered(&mut self, hovered: bool) {
        if self.hovered != hovered {
            self.hovered = hovered;
            self.needs_repaint = true;
        }
    }

    // =========================================================================
    // Repaint
    // =========================================================================

    /// Check if the widget needs to be repainted.
    #[inline]
    pub fn needs_repaint(&self) -> bool {
        self.needs_repaint
    }

    /// Request a repaint of the widget.
    pub fn update(&mut self) {
        self.needs_repaint = true;
    }

    /// Clear the repaint flag (called after painting).
    pub(crate) fn clear_repaint_flag(&mut self) {
        self.needs_repaint = false;
    }

    // =========================================================================
    // Coordinate Mapping
    // =========================================================================

    /// Map a point from widget-local coordinates to parent coordinates.
    #[inline]
    pub fn map_to_parent(&self, point: Point) -> Point {
        Point::new(
            point.x + self.geometry.origin.x,
            point.y + self.geometry.origin.y,
        )
    }

    /// Map a point from parent coordinates to widget-local coordinates.
    #[inline]
    pub fn map_from_parent(&self, point: Point) -> Point {
        Point::new(
            point.x - self.geometry.origin.x,
            point.y - self.geometry.origin.y,
        )
    }

    /// Map a rectangle from widget-local coordinates to parent coordinates.
    #[inline]
    pub fn map_rect_to_parent(&self, rect: Rect) -> Rect {
        Rect {
            origin: self.map_to_parent(rect.origin),
            size: rect.size,
        }
    }

    /// Map a rectangle from parent coordinates to widget-local coordinates.
    #[inline]
    pub fn map_rect_from_parent(&self, rect: Rect) -> Rect {
        Rect {
            origin: self.map_from_parent(rect.origin),
            size: rect.size,
        }
    }

    /// Check if a point (in local coordinates) is inside the widget.
    #[inline]
    pub fn contains_point(&self, point: Point) -> bool {
        self.rect().contains(point)
    }
}

impl Object for WidgetBase {
    fn object_id(&self) -> ObjectId {
        self.object_base.id()
    }
}

// WidgetBase doesn't implement Drop because ObjectBase handles cleanup.
