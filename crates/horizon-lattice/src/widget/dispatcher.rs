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

use super::events::WidgetEvent;
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
}
