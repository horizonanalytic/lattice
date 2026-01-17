//! Event dispatching and propagation for widgets.
//!
//! This module provides the core event dispatch logic including:
//! - Event filter invocation
//! - Event propagation (bubble-up from child to parent)
//!
//! # Event Flow
//!
//! When an event is sent to a widget, the following steps occur:
//!
//! 1. **Event Filters**: All event filters installed on the widget are invoked
//!    in reverse order (most recently installed first). If any filter returns
//!    `true`, the event is consumed and processing stops.
//!
//! 2. **Widget Handler**: If no filter consumed the event, the widget's
//!    `event()` method is called.
//!
//! 3. **Propagation**: If the widget didn't accept the event and the event
//!    type supports propagation, the event is sent to the parent widget.
//!    This continues up the tree until a widget accepts the event or the
//!    root is reached.
//!
//! # Usage
//!
//! The `EventDispatcher` is designed to work with any widget storage mechanism.
//! You provide a `WidgetAccess` implementation that knows how to get widgets by ID.
//!
//! ```ignore
//! use horizon_lattice::widget::{EventDispatcher, WidgetAccess, WidgetEvent};
//!
//! struct MyWidgetStorage {
//!     widgets: HashMap<ObjectId, Box<dyn Widget>>,
//! }
//!
//! impl WidgetAccess for MyWidgetStorage {
//!     fn get_widget(&self, id: ObjectId) -> Option<&dyn Widget> {
//!         self.widgets.get(&id).map(|w| w.as_ref())
//!     }
//!
//!     fn get_widget_mut(&mut self, id: ObjectId) -> Option<&mut dyn Widget> {
//!         self.widgets.get_mut(&id).map(|w| w.as_mut())
//!     }
//! }
//!
//! // Use the dispatcher
//! let result = EventDispatcher::send_event(&mut storage, target_id, &mut event);
//! ```

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::Point;

use super::base::ContextMenuPolicy;
use super::cursor::{CursorManager, CursorShape};
use super::events::{ContextMenuEvent, ContextMenuReason, WidgetEvent};
use super::Widget;

/// Result of dispatching an event to a widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchResult {
    /// The event was accepted/handled.
    Accepted,
    /// The event was not handled by any widget.
    Ignored,
    /// The event was consumed by an event filter.
    Filtered,
    /// The target widget was not found.
    WidgetNotFound,
}

impl DispatchResult {
    /// Check if the event was handled (accepted or filtered).
    pub fn was_handled(&self) -> bool {
        matches!(self, Self::Accepted | Self::Filtered)
    }
}

/// Trait for accessing widgets by their ObjectId.
///
/// Implement this trait for your widget storage mechanism to use
/// the `EventDispatcher`.
pub trait WidgetAccess {
    /// Get an immutable reference to a widget by its ID.
    fn get_widget(&self, id: ObjectId) -> Option<&dyn Widget>;

    /// Get a mutable reference to a widget by its ID.
    fn get_widget_mut(&mut self, id: ObjectId) -> Option<&mut dyn Widget>;

    /// Get the children of a widget in z-order (back to front).
    ///
    /// Default implementation returns an empty vec. Override for hit testing.
    fn get_children(&self, _id: ObjectId) -> Vec<ObjectId> {
        Vec::new()
    }
}

/// Event dispatcher for the widget system.
///
/// Provides methods for dispatching events to widgets with proper
/// event filter handling and parent propagation.
pub struct EventDispatcher;

impl EventDispatcher {
    /// Send an event to a widget, invoking event filters and handling propagation.
    ///
    /// This is the main entry point for event dispatch. It:
    /// 1. Invokes all event filters installed on the target widget
    /// 2. Calls the widget's `event()` method if filters don't consume it
    /// 3. Propagates to parent widgets if the event supports propagation
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`.
    /// * `target_id` - The ObjectId of the widget to send the event to.
    /// * `event` - The event to dispatch.
    ///
    /// # Returns
    ///
    /// A `DispatchResult` indicating how the event was handled.
    pub fn send_event<S: WidgetAccess>(
        storage: &mut S,
        target_id: ObjectId,
        event: &mut WidgetEvent,
    ) -> DispatchResult {
        // Get the target widget's info without holding a borrow
        let (filters, parent_id) = {
            let Some(widget) = storage.get_widget(target_id) else {
                return DispatchResult::WidgetNotFound;
            };
            let base = widget.widget_base();
            (base.event_filters().to_vec(), base.parent_id())
        };

        // Step 1: Invoke event filters (in reverse order - most recent first)
        for filter_id in filters.iter().rev() {
            if let Some(filter_widget) = storage.get_widget_mut(*filter_id) {
                if filter_widget.event_filter(event, target_id) {
                    return DispatchResult::Filtered;
                }
            }
        }

        // Step 2: Send to the target widget
        let handled = {
            let Some(widget) = storage.get_widget_mut(target_id) else {
                return DispatchResult::WidgetNotFound;
            };
            widget.event(event)
        };

        if handled || event.is_accepted() {
            return DispatchResult::Accepted;
        }

        // Step 3: Propagate to parent if the event supports it
        if event.should_propagate() {
            if let Some(parent_id) = parent_id {
                return Self::send_event(storage, parent_id, event);
            }
        }

        DispatchResult::Ignored
    }

    /// Send an event without propagation (direct delivery only).
    ///
    /// This sends an event directly to a widget, invoking event filters,
    /// but does not propagate to parents even if the event is not accepted.
    ///
    /// Useful for events that should only go to a specific widget, like
    /// focus events or geometry change notifications.
    pub fn send_event_direct<S: WidgetAccess>(
        storage: &mut S,
        target_id: ObjectId,
        event: &mut WidgetEvent,
    ) -> DispatchResult {
        // Get the target widget's event filters
        let filters = {
            let Some(widget) = storage.get_widget(target_id) else {
                return DispatchResult::WidgetNotFound;
            };
            widget.widget_base().event_filters().to_vec()
        };

        // Invoke event filters (in reverse order)
        for filter_id in filters.iter().rev() {
            if let Some(filter_widget) = storage.get_widget_mut(*filter_id) {
                if filter_widget.event_filter(event, target_id) {
                    return DispatchResult::Filtered;
                }
            }
        }

        // Send to the target widget
        let handled = {
            let Some(widget) = storage.get_widget_mut(target_id) else {
                return DispatchResult::WidgetNotFound;
            };
            widget.event(event)
        };

        if handled || event.is_accepted() {
            DispatchResult::Accepted
        } else {
            DispatchResult::Ignored
        }
    }

    /// Walk up the widget tree from a starting widget, collecting ancestor IDs.
    ///
    /// Returns a vector of ObjectIds from the immediate parent to the root.
    pub fn get_ancestor_chain<S: WidgetAccess>(storage: &S, start_id: ObjectId) -> Vec<ObjectId> {
        let mut ancestors = Vec::new();
        let mut current = storage
            .get_widget(start_id)
            .and_then(|w| w.widget_base().parent_id());

        while let Some(parent_id) = current {
            ancestors.push(parent_id);
            current = storage
                .get_widget(parent_id)
                .and_then(|w| w.widget_base().parent_id());
        }

        ancestors
    }

    /// Find the widget at a given point in window coordinates.
    ///
    /// Performs hit testing by walking the widget tree from the root,
    /// checking which widgets contain the point, and returning the
    /// deepest (topmost in z-order) widget that contains the point.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess` with `get_children`.
    /// * `root_id` - The root widget to start the search from.
    /// * `window_point` - The point in window coordinates.
    ///
    /// # Returns
    ///
    /// The ObjectId of the deepest widget containing the point, or `None`
    /// if no widget contains the point.
    pub fn hit_test<S: WidgetAccess>(
        storage: &S,
        root_id: ObjectId,
        window_point: Point,
    ) -> Option<ObjectId> {
        Self::hit_test_recursive(storage, root_id, window_point, Point::ZERO)
    }

    fn hit_test_recursive<S: WidgetAccess>(
        storage: &S,
        widget_id: ObjectId,
        window_point: Point,
        parent_offset: Point,
    ) -> Option<ObjectId> {
        let widget = storage.get_widget(widget_id)?;

        // Check if widget is visible for hit testing
        if !widget.is_visible() {
            return None;
        }

        // Calculate the widget's position in window coordinates
        let geometry = widget.geometry();
        let widget_window_pos = Point::new(
            parent_offset.x + geometry.origin.x,
            parent_offset.y + geometry.origin.y,
        );

        // Check if the point is within this widget's bounds
        let local_point = Point::new(
            window_point.x - widget_window_pos.x,
            window_point.y - widget_window_pos.y,
        );

        if !widget.contains_point(local_point) {
            return None;
        }

        // Point is in this widget. Now check children (in reverse z-order for front-to-back).
        let children = storage.get_children(widget_id);

        // Check children in reverse order (front-most first)
        for child_id in children.into_iter().rev() {
            if let Some(hit_id) =
                Self::hit_test_recursive(storage, child_id, window_point, widget_window_pos)
            {
                return Some(hit_id);
            }
        }

        // No child was hit, so this widget is the target
        Some(widget_id)
    }

    /// Handle a context menu request for a widget.
    ///
    /// This should be called by the application layer when a context menu trigger
    /// is detected (e.g., right-click or Menu key press). It handles the context
    /// menu based on the widget's policy:
    ///
    /// - `DefaultContextMenu`: Creates and dispatches a `ContextMenuEvent` to the widget
    /// - `CustomContextMenu`: Emits the widget's `context_menu_requested` signal
    /// - `NoContextMenu`: Ignores the request
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`
    /// * `target_id` - The ObjectId of the widget that should receive the context menu
    /// * `local_pos` - Position in widget-local coordinates
    /// * `window_pos` - Position in window coordinates
    /// * `global_pos` - Position in global screen coordinates
    /// * `reason` - Why the context menu was requested
    ///
    /// # Returns
    ///
    /// A `DispatchResult` indicating how the context menu request was handled.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In your mouse event handler when right-click is detected:
    /// if event.button == MouseButton::Right {
    ///     EventDispatcher::trigger_context_menu(
    ///         &mut storage,
    ///         target_widget_id,
    ///         event.local_pos,
    ///         event.window_pos,
    ///         event.global_pos,
    ///         ContextMenuReason::Mouse,
    ///     );
    /// }
    /// ```
    pub fn trigger_context_menu<S: WidgetAccess>(
        storage: &mut S,
        target_id: ObjectId,
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        reason: ContextMenuReason,
    ) -> DispatchResult {
        // Get the widget's context menu policy
        let policy = {
            let Some(widget) = storage.get_widget(target_id) else {
                return DispatchResult::WidgetNotFound;
            };
            widget.widget_base().context_menu_policy()
        };

        match policy {
            ContextMenuPolicy::NoContextMenu => {
                // Widget doesn't want context menus - ignore
                DispatchResult::Ignored
            }
            ContextMenuPolicy::CustomContextMenu => {
                // Emit the signal for custom handling
                let Some(widget) = storage.get_widget(target_id) else {
                    return DispatchResult::WidgetNotFound;
                };
                widget.widget_base().context_menu_requested.emit(local_pos);
                DispatchResult::Accepted
            }
            ContextMenuPolicy::DefaultContextMenu => {
                // Create and dispatch a ContextMenuEvent
                let mut event = WidgetEvent::ContextMenu(ContextMenuEvent::new(
                    local_pos, window_pos, global_pos, reason,
                ));
                Self::send_event(storage, target_id, &mut event)
            }
        }
    }

    // =========================================================================
    // Cursor Management
    // =========================================================================

    /// Resolve and apply the effective cursor for a widget.
    ///
    /// This traverses up the widget tree from the specified widget to find
    /// the first explicitly set cursor, and applies it to the cursor manager.
    /// If no cursor is found in the tree, the default arrow cursor is used.
    ///
    /// Call this when:
    /// - The mouse enters a widget
    /// - A widget changes its cursor
    /// - A widget that might affect cursor is destroyed
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`
    /// * `widget_id` - The widget to resolve the cursor for
    ///
    /// # Returns
    ///
    /// The resolved cursor shape that was applied.
    pub fn resolve_cursor<S: WidgetAccess>(storage: &S, widget_id: ObjectId) -> CursorShape {
        let cursor = Self::get_effective_cursor(storage, widget_id);
        CursorManager::set_widget_cursor(cursor);
        cursor
    }

    /// Get the effective cursor for a widget without applying it.
    ///
    /// Traverses up the widget tree to find the first explicitly set cursor.
    /// Returns the default arrow cursor if none is found.
    pub fn get_effective_cursor<S: WidgetAccess>(storage: &S, widget_id: ObjectId) -> CursorShape {
        let mut current_id = Some(widget_id);

        while let Some(id) = current_id {
            if let Some(widget) = storage.get_widget(id) {
                // Check if this widget has an explicit cursor
                if let Some(cursor) = widget.widget_base().cursor() {
                    return cursor;
                }
                // Move to parent
                current_id = widget.widget_base().parent_id();
            } else {
                break;
            }
        }

        // No explicit cursor found, use default
        CursorShape::Arrow
    }

    // =========================================================================
    // Drag and Drop
    // =========================================================================

    /// Find the drop target widget at a given point.
    ///
    /// Similar to `hit_test`, but only returns widgets that have
    /// `accepts_drops() == true`. This is used to determine which widget
    /// should receive drag/drop events.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess` with `get_children`.
    /// * `root_id` - The root widget to start the search from.
    /// * `window_point` - The point in window coordinates.
    ///
    /// # Returns
    ///
    /// The ObjectId of the deepest widget that accepts drops and contains
    /// the point, or `None` if no drop target is found.
    pub fn find_drop_target<S: WidgetAccess>(
        storage: &S,
        root_id: ObjectId,
        window_point: Point,
    ) -> Option<ObjectId> {
        Self::find_drop_target_recursive(storage, root_id, window_point, Point::ZERO)
    }

    fn find_drop_target_recursive<S: WidgetAccess>(
        storage: &S,
        widget_id: ObjectId,
        window_point: Point,
        parent_offset: Point,
    ) -> Option<ObjectId> {
        let widget = storage.get_widget(widget_id)?;

        // Check if widget is visible
        if !widget.is_visible() {
            return None;
        }

        // Calculate the widget's position in window coordinates
        let geometry = widget.geometry();
        let widget_window_pos = Point::new(
            parent_offset.x + geometry.origin.x,
            parent_offset.y + geometry.origin.y,
        );

        // Check if the point is within this widget's bounds
        let local_point = Point::new(
            window_point.x - widget_window_pos.x,
            window_point.y - widget_window_pos.y,
        );

        if !widget.contains_point(local_point) {
            return None;
        }

        // Check children first (in reverse z-order for front-to-back)
        let children = storage.get_children(widget_id);

        for child_id in children.into_iter().rev() {
            if let Some(drop_target) =
                Self::find_drop_target_recursive(storage, child_id, window_point, widget_window_pos)
            {
                return Some(drop_target);
            }
        }

        // If this widget accepts drops, it's a valid target
        if widget.widget_base().accepts_drops() {
            Some(widget_id)
        } else {
            None
        }
    }

    /// Calculate the local position for a widget given a window position.
    ///
    /// This traverses the widget tree from root to the target widget,
    /// accumulating offsets to convert window coordinates to widget-local
    /// coordinates.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`.
    /// * `widget_id` - The target widget.
    /// * `window_point` - The point in window coordinates.
    ///
    /// # Returns
    ///
    /// The point in widget-local coordinates.
    pub fn window_to_local<S: WidgetAccess>(
        storage: &S,
        widget_id: ObjectId,
        window_point: Point,
    ) -> Point {
        // Get the ancestor chain from widget to root
        let mut ancestors = Self::get_ancestor_chain(storage, widget_id);
        ancestors.reverse(); // Now from root to parent

        // Accumulate offsets
        let mut offset = Point::ZERO;

        for ancestor_id in ancestors {
            if let Some(ancestor) = storage.get_widget(ancestor_id) {
                let geometry = ancestor.geometry();
                offset.x += geometry.origin.x;
                offset.y += geometry.origin.y;
            }
        }

        // Add the target widget's own offset
        if let Some(widget) = storage.get_widget(widget_id) {
            let geometry = widget.geometry();
            offset.x += geometry.origin.x;
            offset.y += geometry.origin.y;
        }

        // Convert to local coordinates
        Point::new(window_point.x - offset.x, window_point.y - offset.y)
    }
}
