//! Focus management for widget trees.
//!
//! This module provides [`FocusManager`], which coordinates keyboard focus
//! across a widget tree. Each widget tree (or window, in the future) has
//! its own focus manager that tracks which widget has focus and handles
//! focus navigation via Tab/Shift+Tab.
//!
//! # Architecture
//!
//! The focus manager operates at the widget tree level rather than globally.
//! This design allows for:
//! - Multiple independent widget trees (future multi-window support)
//! - Clear ownership of focus state
//! - Easier testing and reasoning about focus behavior
//!
//! # Tab Order
//!
//! Tab order is determined automatically using depth-first pre-order traversal
//! of the widget tree. This visits widgets in the same order they are painted
//! (parents before children, siblings in z-order). Only widgets with
//! [`FocusPolicy::TabFocus`] or [`FocusPolicy::StrongFocus`] participate in
//! tab navigation.
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice::widget::{FocusManager, WidgetAccess};
//!
//! // Create a focus manager for a widget tree
//! let mut focus_manager = FocusManager::new();
//!
//! // Set focus to a specific widget
//! focus_manager.set_focus(&mut storage, widget_id, FocusReason::Other);
//!
//! // Navigate to next focusable widget (Tab key)
//! focus_manager.focus_next(&mut storage, root_id);
//!
//! // Navigate to previous focusable widget (Shift+Tab)
//! focus_manager.focus_previous(&mut storage, root_id);
//! ```

use horizon_lattice_core::ObjectId;

use super::dispatcher::{EventDispatcher, WidgetAccess};
use super::events::{FocusInEvent, FocusOutEvent, FocusReason, WidgetEvent};

/// Manages keyboard focus for a widget tree.
///
/// The focus manager tracks which widget currently has focus and provides
/// methods to change focus and navigate through focusable widgets.
///
/// # Focus Change Events
///
/// When focus changes, the focus manager:
/// 1. Sends a [`FocusOutEvent`] to the widget losing focus (if any)
/// 2. Updates the internal focus state
/// 3. Sends a [`FocusInEvent`] to the widget gaining focus
///
/// Events are sent directly (without propagation) since focus events
/// are specific to the target widget.
#[derive(Debug, Default)]
pub struct FocusManager {
    /// The currently focused widget, if any.
    focused_widget: Option<ObjectId>,
}

impl FocusManager {
    /// Create a new focus manager.
    pub fn new() -> Self {
        Self {
            focused_widget: None,
        }
    }

    /// Get the currently focused widget.
    #[inline]
    pub fn focused_widget(&self) -> Option<ObjectId> {
        self.focused_widget
    }

    /// Check if a specific widget has focus.
    #[inline]
    pub fn has_focus(&self, widget_id: ObjectId) -> bool {
        self.focused_widget == Some(widget_id)
    }

    /// Set focus to a specific widget.
    ///
    /// This will:
    /// 1. Send `FocusOutEvent` to the currently focused widget (if any)
    /// 2. Update the focus state on both widgets
    /// 3. Send `FocusInEvent` to the new widget
    ///
    /// If the widget is not focusable (wrong policy, disabled, or hidden),
    /// this returns `false` and focus is unchanged.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`
    /// * `widget_id` - The widget to focus
    /// * `reason` - The reason for the focus change
    ///
    /// # Returns
    ///
    /// `true` if focus was successfully changed, `false` if the widget
    /// cannot receive focus.
    pub fn set_focus<S: WidgetAccess>(
        &mut self,
        storage: &mut S,
        widget_id: ObjectId,
        reason: FocusReason,
    ) -> bool {
        // Check if the widget can receive focus
        let can_focus = {
            let Some(widget) = storage.get_widget(widget_id) else {
                return false;
            };
            widget.is_focusable()
        };

        if !can_focus {
            return false;
        }

        // Don't do anything if already focused
        if self.focused_widget == Some(widget_id) {
            return true;
        }

        // Remove focus from current widget
        if let Some(old_id) = self.focused_widget.take() {
            self.unfocus_widget(storage, old_id, reason);
        }

        // Set focus on new widget
        self.focus_widget(storage, widget_id, reason);
        self.focused_widget = Some(widget_id);

        true
    }

    /// Clear focus from the currently focused widget.
    ///
    /// After calling this, no widget will have focus.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`
    /// * `reason` - The reason for clearing focus
    pub fn clear_focus<S: WidgetAccess>(&mut self, storage: &mut S, reason: FocusReason) {
        if let Some(old_id) = self.focused_widget.take() {
            self.unfocus_widget(storage, old_id, reason);
        }
    }

    /// Move focus to the next focusable widget in tab order.
    ///
    /// Tab order is determined by depth-first pre-order traversal of the
    /// widget tree, considering only widgets with `TabFocus` or `StrongFocus`
    /// policy that are enabled and visible.
    ///
    /// If no widget is currently focused, focuses the first focusable widget.
    /// If the current widget is the last in tab order, wraps to the first.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`
    /// * `root_id` - The root widget of the tree to navigate
    ///
    /// # Returns
    ///
    /// `true` if focus was moved to another widget, `false` if no focusable
    /// widget was found or only one exists.
    pub fn focus_next<S: WidgetAccess>(&mut self, storage: &mut S, root_id: ObjectId) -> bool {
        let tab_order = self.build_tab_order(storage, root_id);

        if tab_order.is_empty() {
            return false;
        }

        let next_id = match self.focused_widget {
            Some(current) => {
                // Find current position and move to next (with wrap)
                if let Some(pos) = tab_order.iter().position(|&id| id == current) {
                    let next_pos = (pos + 1) % tab_order.len();
                    tab_order[next_pos]
                } else {
                    // Current widget not in tab order, focus first
                    tab_order[0]
                }
            }
            None => {
                // No current focus, focus first widget
                tab_order[0]
            }
        };

        self.set_focus(storage, next_id, FocusReason::Tab)
    }

    /// Move focus to the previous focusable widget in tab order.
    ///
    /// Similar to [`focus_next`](Self::focus_next) but moves backwards through
    /// the tab order (for Shift+Tab navigation).
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`
    /// * `root_id` - The root widget of the tree to navigate
    ///
    /// # Returns
    ///
    /// `true` if focus was moved to another widget, `false` if no focusable
    /// widget was found or only one exists.
    pub fn focus_previous<S: WidgetAccess>(&mut self, storage: &mut S, root_id: ObjectId) -> bool {
        let tab_order = self.build_tab_order(storage, root_id);

        if tab_order.is_empty() {
            return false;
        }

        let prev_id = match self.focused_widget {
            Some(current) => {
                // Find current position and move to previous (with wrap)
                if let Some(pos) = tab_order.iter().position(|&id| id == current) {
                    let prev_pos = if pos == 0 {
                        tab_order.len() - 1
                    } else {
                        pos - 1
                    };
                    tab_order[prev_pos]
                } else {
                    // Current widget not in tab order, focus last
                    tab_order[tab_order.len() - 1]
                }
            }
            None => {
                // No current focus, focus last widget
                tab_order[tab_order.len() - 1]
            }
        };

        self.set_focus(storage, prev_id, FocusReason::Backtab)
    }

    /// Build the tab order for a widget tree.
    ///
    /// Returns a list of widget IDs in tab order (depth-first pre-order),
    /// containing only widgets that accept tab focus.
    fn build_tab_order<S: WidgetAccess>(&self, storage: &S, root_id: ObjectId) -> Vec<ObjectId> {
        let mut order = Vec::new();
        self.collect_tab_order_recursive(storage, root_id, &mut order);
        order
    }

    /// Recursively collect widgets in tab order.
    fn collect_tab_order_recursive<S: WidgetAccess>(
        &self,
        storage: &S,
        widget_id: ObjectId,
        order: &mut Vec<ObjectId>,
    ) {
        let Some(widget) = storage.get_widget(widget_id) else {
            return;
        };

        // Skip hidden widgets and their children
        if !widget.is_visible() {
            return;
        }

        // Add this widget if it accepts tab focus
        if widget.widget_base().accepts_tab_focus() {
            order.push(widget_id);
        }

        // Recurse into children (in z-order, back to front)
        let children = storage.get_children(widget_id);
        for child_id in children {
            self.collect_tab_order_recursive(storage, child_id, order);
        }
    }

    /// Find the next focusable widget after a given widget.
    ///
    /// This is useful for focus navigation without changing focus immediately.
    pub fn find_next_focusable<S: WidgetAccess>(
        &self,
        storage: &S,
        root_id: ObjectId,
        current_id: ObjectId,
    ) -> Option<ObjectId> {
        let tab_order = self.build_tab_order(storage, root_id);

        if tab_order.is_empty() {
            return None;
        }

        if let Some(pos) = tab_order.iter().position(|&id| id == current_id) {
            let next_pos = (pos + 1) % tab_order.len();
            if tab_order[next_pos] != current_id {
                return Some(tab_order[next_pos]);
            }
        }

        // Current not found or only one widget, return first if different
        if !tab_order.is_empty() && tab_order[0] != current_id {
            return Some(tab_order[0]);
        }

        None
    }

    /// Find the previous focusable widget before a given widget.
    pub fn find_previous_focusable<S: WidgetAccess>(
        &self,
        storage: &S,
        root_id: ObjectId,
        current_id: ObjectId,
    ) -> Option<ObjectId> {
        let tab_order = self.build_tab_order(storage, root_id);

        if tab_order.is_empty() {
            return None;
        }

        if let Some(pos) = tab_order.iter().position(|&id| id == current_id) {
            let prev_pos = if pos == 0 {
                tab_order.len() - 1
            } else {
                pos - 1
            };
            if tab_order[prev_pos] != current_id {
                return Some(tab_order[prev_pos]);
            }
        }

        // Current not found or only one widget, return last if different
        let last = tab_order.len() - 1;
        if !tab_order.is_empty() && tab_order[last] != current_id {
            return Some(tab_order[last]);
        }

        None
    }

    // =========================================================================
    // Internal Helpers
    // =========================================================================

    /// Send FocusOutEvent and update widget state.
    fn unfocus_widget<S: WidgetAccess>(
        &self,
        storage: &mut S,
        widget_id: ObjectId,
        reason: FocusReason,
    ) {
        // Update the widget's focus state
        if let Some(widget) = storage.get_widget_mut(widget_id) {
            widget.widget_base_mut().set_focused(false);
        }

        // Send FocusOutEvent
        let mut event = WidgetEvent::FocusOut(FocusOutEvent::new(reason));
        EventDispatcher::send_event_direct(storage, widget_id, &mut event);
    }

    /// Send FocusInEvent and update widget state.
    fn focus_widget<S: WidgetAccess>(
        &self,
        storage: &mut S,
        widget_id: ObjectId,
        reason: FocusReason,
    ) {
        // Update the widget's focus state
        if let Some(widget) = storage.get_widget_mut(widget_id) {
            widget.widget_base_mut().set_focused(true);
        }

        // Send FocusInEvent
        let mut event = WidgetEvent::FocusIn(FocusInEvent::new(reason));
        EventDispatcher::send_event_direct(storage, widget_id, &mut event);
    }
}
