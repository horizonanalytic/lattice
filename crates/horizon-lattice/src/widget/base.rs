//! Widget base implementation.
//!
//! This module provides `WidgetBase`, the common implementation details
//! for all widgets. It handles geometry, visibility, enabled state, and
//! coordinates with the object system.

use horizon_lattice_core::{global_registry, Object, ObjectBase, ObjectId, ObjectResult, Signal, WidgetState};
use horizon_lattice_render::{Point, Rect, Size};

use super::geometry::{SizePolicy, SizePolicyPair};

/// Focus policy for a widget.
///
/// Determines how a widget can receive keyboard focus. This follows Qt's
/// design pattern where different widgets may accept focus through different
/// interaction methods.
///
/// # Examples
///
/// ```ignore
/// // A button accepts focus via Tab and mouse click
/// widget.set_focus_policy(FocusPolicy::StrongFocus);
///
/// // A label doesn't accept focus at all
/// widget.set_focus_policy(FocusPolicy::NoFocus);
///
/// // A scroll area only accepts focus via Tab (keyboard navigation)
/// widget.set_focus_policy(FocusPolicy::TabFocus);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusPolicy {
    /// Widget cannot receive keyboard focus.
    #[default]
    NoFocus,
    /// Widget accepts focus via Tab/Shift+Tab keyboard navigation only.
    TabFocus,
    /// Widget accepts focus via mouse click only.
    ClickFocus,
    /// Widget accepts focus via both Tab navigation and mouse click.
    /// This is typically used for interactive widgets like buttons, text fields.
    StrongFocus,
}

/// The base implementation for all widgets.
///
/// This struct provides common functionality that all widgets need:
/// - Object system integration (ID, parent-child relationships)
/// - Geometry management (position, size)
/// - Size hints and policies for layout
/// - Visibility and enabled state
/// - Coordinate mapping
/// - Event filtering
///
/// Widget implementations typically include this as a field and delegate
/// common operations to it.
///
/// # Event Filters
///
/// Event filters allow an object to intercept events destined for another widget.
/// This is useful for:
/// - Implementing global keyboard shortcuts
/// - Debugging/logging events
/// - Creating invisible widgets that modify events
///
/// ```ignore
/// // Install an event filter
/// target_widget.widget_base_mut().install_event_filter(filter_widget.object_id());
///
/// // The filter widget's event_filter() method will be called before
/// // any event reaches target_widget. If event_filter() returns true,
/// // the event is consumed and won't reach the target.
/// ```
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

    /// Focus policy determining how the widget can receive focus.
    focus_policy: FocusPolicy,

    /// Whether the widget currently has focus.
    focused: bool,

    /// Whether the mouse is currently over this widget.
    hovered: bool,

    /// Whether the widget is currently pressed (mouse button down on it).
    pressed: bool,

    /// Whether the widget needs to be repainted.
    needs_repaint: bool,

    /// The dirty region that needs repainting (in widget-local coordinates).
    /// If `Some`, only this region needs repainting. If `None`, no repaint needed.
    /// A full repaint sets this to the widget's rect().
    dirty_region: Option<Rect>,

    /// Whether this widget is opaque (paints all its pixels).
    /// Opaque widgets allow the painting system to skip painting parent regions
    /// that would be completely covered by this widget.
    opaque: bool,

    /// Event filters installed on this widget.
    ///
    /// When an event is sent to this widget, it first goes through all
    /// event filters (in reverse order - most recently installed first).
    /// If any filter returns `true`, the event is consumed and doesn't
    /// reach this widget.
    event_filters: Vec<ObjectId>,

    /// Signal emitted when the geometry changes.
    pub geometry_changed: Signal<Rect>,

    /// Signal emitted when pressed state changes.
    pub pressed_changed: Signal<bool>,

    /// Signal emitted when visibility changes.
    pub visible_changed: Signal<bool>,

    /// Signal emitted when enabled state changes.
    pub enabled_changed: Signal<bool>,

    /// Signal emitted when focus state changes.
    pub focus_changed: Signal<bool>,

    /// Signal emitted when the widget is about to be destroyed.
    ///
    /// This signal is emitted during the widget's destruction, before the
    /// underlying object is removed from the registry. It allows other objects
    /// to clean up references to this widget.
    ///
    /// The signal parameter is the ObjectId of the widget being destroyed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let widget_id = widget.object_id();
    /// widget.destroyed.connect(move |id| {
    ///     println!("Widget {:?} is being destroyed", id);
    ///     // Clean up any references to this widget
    /// });
    /// ```
    pub destroyed: Signal<ObjectId>,
}

impl WidgetBase {
    /// Create a new widget base.
    ///
    /// # Panics
    ///
    /// Panics if the global object registry is not initialized.
    pub fn new<T: Object + 'static>() -> Self {
        let object_base = ObjectBase::new::<T>();

        // Initialize widget state in registry for state propagation queries
        if let Ok(registry) = global_registry() {
            let _ = registry.init_widget_state(
                object_base.id(),
                WidgetState {
                    visible: true,
                    enabled: true,
                },
            );
        }

        Self {
            object_base,
            geometry: Rect::ZERO,
            size_policy: SizePolicyPair::default(),
            visible: true,
            enabled: true,
            focus_policy: FocusPolicy::NoFocus,
            focused: false,
            hovered: false,
            pressed: false,
            needs_repaint: true,
            dirty_region: None, // Will be set when geometry is set
            opaque: false,
            event_filters: Vec::new(),
            geometry_changed: Signal::new(),
            pressed_changed: Signal::new(),
            visible_changed: Signal::new(),
            enabled_changed: Signal::new(),
            focus_changed: Signal::new(),
            destroyed: Signal::new(),
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
    // Z-Order / Sibling Ordering
    // =========================================================================

    /// Get this widget's index among its siblings.
    ///
    /// Index 0 is the back/bottom (painted first), higher indices are front/top (painted last).
    /// Returns `None` if the widget has no parent.
    pub fn sibling_index(&self) -> Option<usize> {
        self.object_base.sibling_index()
    }

    /// Get the next sibling widget (higher z-order / closer to front).
    pub fn next_sibling(&self) -> Option<ObjectId> {
        self.object_base.next_sibling()
    }

    /// Get the previous sibling widget (lower z-order / closer to back).
    pub fn previous_sibling(&self) -> Option<ObjectId> {
        self.object_base.previous_sibling()
    }

    /// Raise this widget to the front (highest z-order among siblings).
    ///
    /// The widget will be painted last (on top of siblings).
    pub fn raise(&self) -> ObjectResult<()> {
        self.object_base.raise()
    }

    /// Lower this widget to the back (lowest z-order among siblings).
    ///
    /// The widget will be painted first (behind siblings).
    pub fn lower(&self) -> ObjectResult<()> {
        self.object_base.lower()
    }

    /// Stack this widget under (behind) a sibling widget.
    ///
    /// The widget will be positioned just before the sibling in paint order.
    pub fn stack_under(&self, sibling: ObjectId) -> ObjectResult<()> {
        self.object_base.stack_under(sibling)
    }

    /// Stack this widget above (in front of) a sibling widget.
    ///
    /// The widget will be positioned just after the sibling in paint order.
    pub fn stack_above(&self, sibling: ObjectId) -> ObjectResult<()> {
        self.object_base.stack_above(sibling)
    }

    /// Get all sibling widgets (excluding this widget).
    pub fn siblings(&self) -> Vec<ObjectId> {
        self.object_base.siblings()
    }

    // =========================================================================
    // Tree Traversal
    // =========================================================================

    /// Get all ancestor widgets from immediate parent to root.
    pub fn ancestors(&self) -> Vec<ObjectId> {
        self.object_base.ancestors()
    }

    /// Get this widget and all descendants in depth-first pre-order.
    ///
    /// Order: self, child1, grandchild1, grandchild2, child2, ...
    /// This is the natural paint order (parent before children).
    pub fn depth_first_preorder(&self) -> Vec<ObjectId> {
        self.object_base.depth_first_preorder()
    }

    /// Get this widget and all descendants in depth-first post-order.
    ///
    /// Order: grandchild1, grandchild2, child1, child2, self
    /// Useful for bottom-up operations like destruction.
    pub fn depth_first_postorder(&self) -> Vec<ObjectId> {
        self.object_base.depth_first_postorder()
    }

    /// Get this widget and all descendants in breadth-first (level) order.
    ///
    /// Visits all nodes at depth N before any nodes at depth N+1.
    pub fn breadth_first(&self) -> Vec<ObjectId> {
        self.object_base.breadth_first()
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

            // Sync to registry for state propagation queries
            if let Ok(registry) = global_registry() {
                let _ = registry.set_widget_visible(self.object_id(), visible);
            }
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

    /// Check if the widget is effectively visible (considering ancestors).
    ///
    /// Returns `true` only if this widget AND all its ancestors are visible.
    /// A widget with `is_visible() == true` may still be effectively hidden
    /// if any ancestor is hidden.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // If parent.hide() is called:
    /// assert!(child.is_visible());           // Child's own flag is still true
    /// assert!(!child.is_effectively_visible()); // But child is effectively hidden
    /// ```
    pub fn is_effectively_visible(&self) -> bool {
        if !self.visible {
            return false;
        }

        // Query registry for effective visibility (checks all ancestors)
        match global_registry() {
            Ok(registry) => registry
                .is_effectively_visible(self.object_id())
                .ok()
                .flatten()
                .unwrap_or(self.visible),
            Err(_) => self.visible,
        }
    }

    /// Check if the widget is visible up to a specific ancestor.
    ///
    /// Returns `true` if this widget and all ancestors up to (but not including)
    /// `ancestor_id` are visible. If `ancestor_id` is `None`, checks all ancestors
    /// up to the root.
    pub fn is_visible_to(&self, ancestor_id: Option<ObjectId>) -> bool {
        if !self.visible {
            return false;
        }

        let registry = match global_registry() {
            Ok(r) => r,
            Err(_) => return self.visible,
        };

        // Use registry's effective visibility check
        if ancestor_id.is_none() {
            // Check all ancestors to root
            return registry
                .is_effectively_visible(self.object_id())
                .ok()
                .flatten()
                .unwrap_or(self.visible);
        }

        // Check ancestors up to specified ancestor
        let mut current = self.parent_id();
        while let Some(id) = current {
            if Some(id) == ancestor_id {
                break;
            }
            // Check this ancestor's visibility via registry
            if let Ok(Some(state)) = registry.widget_state(id) {
                if !state.visible {
                    return false;
                }
            }
            current = registry.parent(id).ok().flatten();
        }

        true
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

            // Sync to registry for state propagation queries
            if let Ok(registry) = global_registry() {
                let _ = registry.set_widget_enabled(self.object_id(), enabled);
            }
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

    /// Check if the widget is effectively enabled (considering ancestors).
    ///
    /// Returns `true` only if this widget AND all its ancestors are enabled.
    /// A widget with `is_enabled() == true` may still be effectively disabled
    /// if any ancestor is disabled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // If parent.disable() is called:
    /// assert!(child.is_enabled());           // Child's own flag is still true
    /// assert!(!child.is_effectively_enabled()); // But child is effectively disabled
    /// ```
    pub fn is_effectively_enabled(&self) -> bool {
        if !self.enabled {
            return false;
        }

        // Query registry for effective enabled state (checks all ancestors)
        match global_registry() {
            Ok(registry) => registry
                .is_effectively_enabled(self.object_id())
                .ok()
                .flatten()
                .unwrap_or(self.enabled),
            Err(_) => self.enabled,
        }
    }

    // =========================================================================
    // Focus
    // =========================================================================

    /// Get the widget's focus policy.
    #[inline]
    pub fn focus_policy(&self) -> FocusPolicy {
        self.focus_policy
    }

    /// Set the widget's focus policy.
    ///
    /// The focus policy determines how a widget can receive keyboard focus.
    /// See [`FocusPolicy`] for available options.
    pub fn set_focus_policy(&mut self, policy: FocusPolicy) {
        self.focus_policy = policy;
    }

    /// Check if the widget can receive keyboard focus.
    ///
    /// Returns `true` if the widget has a focus policy that allows focus
    /// AND the widget is enabled AND visible. A widget with `NoFocus` policy
    /// can never receive focus.
    #[inline]
    pub fn is_focusable(&self) -> bool {
        self.focus_policy != FocusPolicy::NoFocus && self.enabled && self.visible
    }

    /// Check if the widget accepts focus via Tab/Shift+Tab navigation.
    #[inline]
    pub fn accepts_tab_focus(&self) -> bool {
        matches!(self.focus_policy, FocusPolicy::TabFocus | FocusPolicy::StrongFocus)
            && self.enabled
            && self.visible
    }

    /// Check if the widget accepts focus via mouse click.
    #[inline]
    pub fn accepts_click_focus(&self) -> bool {
        matches!(self.focus_policy, FocusPolicy::ClickFocus | FocusPolicy::StrongFocus)
            && self.enabled
            && self.visible
    }

    /// Set whether the widget can receive keyboard focus.
    ///
    /// This is a convenience method that sets the focus policy to `StrongFocus`
    /// if `focusable` is `true`, or `NoFocus` if `false`. For more fine-grained
    /// control, use [`set_focus_policy()`](Self::set_focus_policy).
    pub fn set_focusable(&mut self, focusable: bool) {
        self.focus_policy = if focusable {
            FocusPolicy::StrongFocus
        } else {
            FocusPolicy::NoFocus
        };
    }

    /// Check if the widget currently has keyboard focus.
    #[inline]
    pub fn has_focus(&self) -> bool {
        self.focused
    }

    /// Set the focused state (used by the focus management system).
    ///
    /// This emits `focus_changed` signal when the state changes.
    pub(crate) fn set_focused(&mut self, focused: bool) {
        if self.focused != focused {
            self.focused = focused;
            self.needs_repaint = true;
            self.focus_changed.emit(focused);
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
    // Pressed State
    // =========================================================================

    /// Check if the widget is currently pressed.
    ///
    /// A widget is considered pressed when a mouse button is held down on it.
    /// This is typically used for visual feedback (e.g., button appears pushed).
    #[inline]
    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    /// Set the pressed state (used by the event system).
    ///
    /// This emits the `pressed_changed` signal when the state changes.
    pub(crate) fn set_pressed(&mut self, pressed: bool) {
        if self.pressed != pressed {
            self.pressed = pressed;
            self.needs_repaint = true;
            self.pressed_changed.emit(pressed);
        }
    }

    // =========================================================================
    // Opaque Widget
    // =========================================================================

    /// Check if this widget is opaque.
    ///
    /// Opaque widgets paint all their pixels, allowing the painting system
    /// to skip painting parent regions that would be completely covered.
    /// This is an optimization hint.
    #[inline]
    pub fn is_opaque(&self) -> bool {
        self.opaque
    }

    /// Set whether this widget is opaque.
    ///
    /// Set to `true` if this widget always paints all its pixels with opaque
    /// colors. This allows the painting system to skip painting parent regions
    /// underneath this widget.
    ///
    /// # Note
    ///
    /// If you set this to `true` but don't actually paint all pixels, you may
    /// see visual artifacts (uninitialized or stale pixels showing through).
    pub fn set_opaque(&mut self, opaque: bool) {
        self.opaque = opaque;
    }

    // =========================================================================
    // Repaint / Dirty Regions
    // =========================================================================

    /// Check if the widget needs to be repainted.
    #[inline]
    pub fn needs_repaint(&self) -> bool {
        self.needs_repaint
    }

    /// Get the dirty region that needs repainting.
    ///
    /// Returns `None` if no repaint is needed, or `Some(rect)` with the
    /// region in widget-local coordinates that needs repainting.
    #[inline]
    pub fn dirty_region(&self) -> Option<Rect> {
        self.dirty_region
    }

    /// Request a full repaint of the widget.
    ///
    /// This schedules a repaint of the entire widget for the next frame.
    /// Multiple calls to `update()` before the next paint are coalesced.
    pub fn update(&mut self) {
        self.needs_repaint = true;
        // Set dirty region to full widget rect
        self.dirty_region = Some(self.rect());
    }

    /// Request a partial repaint of a specific region.
    ///
    /// This schedules a repaint of only the specified region for the next frame.
    /// The region is in widget-local coordinates.
    ///
    /// Multiple calls are coalesced by computing the union of all dirty regions.
    ///
    /// # Arguments
    ///
    /// * `rect` - The region to repaint, in widget-local coordinates.
    pub fn update_rect(&mut self, rect: Rect) {
        // Skip empty rects
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }

        // Clip to widget bounds
        let widget_rect = self.rect();
        let clipped = match rect.intersect(&widget_rect) {
            Some(r) => r,
            None => return, // Outside widget bounds
        };

        self.needs_repaint = true;
        self.dirty_region = Some(match self.dirty_region {
            Some(existing) => existing.union(&clipped),
            None => clipped,
        });
    }

    /// Clear the repaint flag and dirty region (called after painting).
    pub(crate) fn clear_repaint_flag(&mut self) {
        self.needs_repaint = false;
        self.dirty_region = None;
    }

    /// Request an immediate repaint of the widget.
    ///
    /// Unlike `update()` which schedules a repaint for the next frame,
    /// this signals that the widget should be repainted immediately.
    /// This is useful for time-critical updates like video playback.
    ///
    /// Returns the dirty region for the immediate repaint.
    pub fn repaint(&mut self) -> Rect {
        let region = self.dirty_region.unwrap_or_else(|| self.rect());
        self.needs_repaint = true;
        self.dirty_region = Some(region);
        region
    }

    /// Request an immediate repaint of a specific region.
    ///
    /// Like `repaint()` but for a specific region of the widget.
    ///
    /// # Arguments
    ///
    /// * `rect` - The region to repaint immediately, in widget-local coordinates.
    pub fn repaint_rect(&mut self, rect: Rect) -> Option<Rect> {
        // Skip empty rects
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return None;
        }

        // Clip to widget bounds
        let widget_rect = self.rect();
        let clipped = rect.intersect(&widget_rect)?;

        self.needs_repaint = true;
        let region = match self.dirty_region {
            Some(existing) => existing.union(&clipped),
            None => clipped,
        };
        self.dirty_region = Some(region);
        Some(region)
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

    // =========================================================================
    // Event Filters
    // =========================================================================

    /// Install an event filter on this widget.
    ///
    /// The filter object's `event_filter()` method will be called for every
    /// event sent to this widget. If the filter returns `true`, the event
    /// is consumed and won't reach this widget.
    ///
    /// Multiple event filters can be installed. They are called in reverse
    /// order of installation (most recently installed first).
    ///
    /// # Arguments
    ///
    /// * `filter_id` - The ObjectId of the widget to use as an event filter.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Install filter_widget as an event filter on target_widget
    /// target.widget_base_mut().install_event_filter(filter.object_id());
    ///
    /// // Now filter.event_filter(&mut event, target.object_id()) will be called
    /// // for every event sent to target_widget.
    /// ```
    pub fn install_event_filter(&mut self, filter_id: ObjectId) {
        // Don't add duplicates
        if !self.event_filters.contains(&filter_id) {
            self.event_filters.push(filter_id);
        }
    }

    /// Remove an event filter from this widget.
    ///
    /// If the filter was not installed, this does nothing.
    pub fn remove_event_filter(&mut self, filter_id: ObjectId) {
        self.event_filters.retain(|&id| id != filter_id);
    }

    /// Get the list of event filters installed on this widget.
    ///
    /// Returns the filters in the order they were installed (oldest first).
    /// Note that filters are *called* in reverse order (newest first).
    pub fn event_filters(&self) -> &[ObjectId] {
        &self.event_filters
    }

    /// Check if an event filter is installed on this widget.
    pub fn has_event_filter(&self, filter_id: ObjectId) -> bool {
        self.event_filters.contains(&filter_id)
    }

    /// Clear all event filters from this widget.
    pub fn clear_event_filters(&mut self) {
        self.event_filters.clear();
    }

    // =========================================================================
    // Timers
    // =========================================================================

    /// Start a one-shot timer owned by this widget.
    ///
    /// When the timer fires, this widget will receive a `WidgetEvent::Timer` event.
    ///
    /// # Arguments
    ///
    /// * `duration` - How long until the timer fires
    ///
    /// # Returns
    ///
    /// The TimerId that can be used to stop the timer.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Start a 500ms timer
    /// let timer_id = self.base.start_timer(Duration::from_millis(500));
    ///
    /// // Later, in event handler:
    /// fn event(&mut self, event: &mut WidgetEvent) -> bool {
    ///     if let WidgetEvent::Timer(e) = event {
    ///         if e.id == self.my_timer_id {
    ///             // Timer fired!
    ///         }
    ///     }
    ///     false
    /// }
    /// ```
    pub fn start_timer(&self, duration: std::time::Duration) -> horizon_lattice_core::TimerId {
        super::widget_timer::start_widget_timer(self.object_id(), duration)
    }

    /// Start a repeating timer owned by this widget.
    ///
    /// When the timer fires, this widget will receive a `WidgetEvent::Timer` event.
    /// The timer will continue firing at the specified interval until stopped.
    ///
    /// # Arguments
    ///
    /// * `interval` - The interval between timer firings
    ///
    /// # Returns
    ///
    /// The TimerId that can be used to stop the timer.
    pub fn start_repeating_timer(
        &self,
        interval: std::time::Duration,
    ) -> horizon_lattice_core::TimerId {
        super::widget_timer::start_widget_repeating_timer(self.object_id(), interval)
    }

    /// Stop a timer owned by this widget.
    ///
    /// # Arguments
    ///
    /// * `timer_id` - The timer to stop
    ///
    /// # Returns
    ///
    /// `true` if the timer was found and stopped, `false` otherwise.
    pub fn stop_timer(&self, timer_id: horizon_lattice_core::TimerId) -> bool {
        super::widget_timer::stop_widget_timer(timer_id)
    }

    /// Check if a timer is currently active.
    pub fn is_timer_active(&self, timer_id: horizon_lattice_core::TimerId) -> bool {
        super::widget_timer::is_widget_timer_active(timer_id)
    }
}

impl Object for WidgetBase {
    fn object_id(&self) -> ObjectId {
        self.object_base.id()
    }
}

impl Drop for WidgetBase {
    fn drop(&mut self) {
        // Emit the destroyed signal before ObjectBase::drop() runs.
        // This allows connected slots to clean up references to this widget.
        let id = self.object_base.id();
        self.destroyed.emit(id);

        // Clean up any timers owned by this widget.
        super::widget_timer::remove_timers_for_widget(id);

        // ObjectBase::drop() will run after this, which removes the object from
        // the registry. The signal was emitted while the object was still valid.
    }
}
