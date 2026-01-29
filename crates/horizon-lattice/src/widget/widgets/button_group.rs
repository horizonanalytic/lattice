//! Button group implementation for exclusive button selection.
//!
//! This module provides [`ButtonGroup`], a non-visual container that
//! manages exclusive selection among checkable buttons.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{ButtonGroup, RadioButton};
//!
//! // Create a button group
//! let mut group = ButtonGroup::new();
//!
//! // Create radio buttons and add to group
//! let mut rb1 = RadioButton::new("Option 1");
//! let mut rb2 = RadioButton::new("Option 2");
//! let mut rb3 = RadioButton::new("Option 3");
//!
//! group.add_button(rb1.object_id(), 1);
//! group.add_button(rb2.object_id(), 2);
//! group.add_button(rb3.object_id(), 3);
//!
//! // Connect to group signals
//! group.id_toggled.connect(|&(id, checked)| {
//!     if checked {
//!         println!("Button with ID {} was selected", id);
//!     }
//! });
//! ```

use std::collections::HashMap;

use horizon_lattice_core::{ObjectId, Signal};

/// Information about a button in the group.
#[derive(Clone, Copy, Debug)]
struct ButtonInfo {
    /// The unique ID assigned to this button (user-provided or auto-generated).
    id: i32,
    /// Whether this button is currently checked.
    checked: bool,
}

/// A non-visual container for organizing groups of buttons.
///
/// `ButtonGroup` provides exclusive selection behavior for checkable buttons.
/// When one button in an exclusive group is checked, all other buttons in the
/// group are automatically unchecked.
///
/// # Exclusive Mode
///
/// By default, button groups are exclusive (`exclusive = true`). In exclusive mode:
/// - Only one button can be checked at a time
/// - Clicking a checked button does NOT uncheck it (another button must be clicked)
/// - If no button is initially checked, clicking any button will check it
///
/// When `exclusive = false`:
/// - Multiple buttons can be checked simultaneously
/// - This is useful for grouping related but non-exclusive options
///
/// # Button IDs
///
/// Each button in the group can be assigned a numeric ID for easier identification.
/// - User-assigned IDs should be positive numbers
/// - Auto-assigned IDs are negative (starting from -2)
/// - ID -1 is reserved to indicate "no button"
///
/// # Usage Pattern
///
/// `ButtonGroup` is a coordinator, not a container. Buttons are added by their
/// `ObjectId`, and the group tracks their state. When buttons are clicked:
///
/// 1. The button notifies its group via `button_toggled()`
/// 2. The group updates its internal state
/// 3. The group emits appropriate signals
/// 4. The group returns a list of buttons that should be unchecked
pub struct ButtonGroup {
    /// Map from ObjectId to button info.
    buttons: HashMap<ObjectId, ButtonInfo>,

    /// Whether the group enforces exclusive selection.
    exclusive: bool,

    /// The next auto-generated ID (decrements from -2).
    next_auto_id: i32,

    /// The currently checked button's ObjectId (for exclusive mode).
    checked_button: Option<ObjectId>,

    /// Signal emitted when a button's checked state changes.
    /// Parameter is (button_id, checked).
    pub id_toggled: Signal<(i32, bool)>,

    /// Signal emitted when a button is clicked.
    /// Parameter is button_id.
    pub id_clicked: Signal<i32>,
}

impl ButtonGroup {
    /// Create a new exclusive button group.
    pub fn new() -> Self {
        Self {
            buttons: HashMap::new(),
            exclusive: true,
            next_auto_id: -2,
            checked_button: None,
            id_toggled: Signal::new(),
            id_clicked: Signal::new(),
        }
    }

    /// Create a new button group with specified exclusivity.
    pub fn with_exclusive(exclusive: bool) -> Self {
        Self {
            buttons: HashMap::new(),
            exclusive,
            next_auto_id: -2,
            checked_button: None,
            id_toggled: Signal::new(),
            id_clicked: Signal::new(),
        }
    }

    // =========================================================================
    // Exclusivity
    // =========================================================================

    /// Check if the group enforces exclusive selection.
    pub fn is_exclusive(&self) -> bool {
        self.exclusive
    }

    /// Set whether the group enforces exclusive selection.
    ///
    /// If switching from non-exclusive to exclusive mode with multiple buttons
    /// checked, only the first checked button remains checked.
    pub fn set_exclusive(&mut self, exclusive: bool) {
        if self.exclusive == exclusive {
            return;
        }

        self.exclusive = exclusive;

        // If switching to exclusive mode, ensure only one button is checked
        if exclusive {
            let mut found_checked = false;
            let mut buttons_to_uncheck = Vec::new();

            for (&object_id, info) in &self.buttons {
                if info.checked {
                    if found_checked {
                        buttons_to_uncheck.push(object_id);
                    } else {
                        found_checked = true;
                        self.checked_button = Some(object_id);
                    }
                }
            }

            // Mark additional checked buttons as unchecked in our records
            for object_id in buttons_to_uncheck {
                if let Some(info) = self.buttons.get_mut(&object_id) {
                    info.checked = false;
                    self.id_toggled.emit((info.id, false));
                }
            }
        }
    }

    // =========================================================================
    // Button Management
    // =========================================================================

    /// Add a button to the group with an auto-generated ID.
    ///
    /// Returns the auto-generated ID assigned to the button.
    pub fn add_button(&mut self, object_id: ObjectId) -> i32 {
        let id = self.next_auto_id;
        self.next_auto_id -= 1;
        self.add_button_with_id(object_id, id);
        id
    }

    /// Add a button to the group with a specific ID.
    ///
    /// If the button already exists in the group, its ID is updated.
    /// User-assigned IDs should be positive to avoid conflicts with
    /// auto-generated IDs.
    pub fn add_button_with_id(&mut self, object_id: ObjectId, id: i32) {
        let info = ButtonInfo { id, checked: false };
        self.buttons.insert(object_id, info);
    }

    /// Remove a button from the group.
    ///
    /// Returns the ID that was assigned to the button, or `None` if
    /// the button was not in the group.
    pub fn remove_button(&mut self, object_id: ObjectId) -> Option<i32> {
        if let Some(info) = self.buttons.remove(&object_id) {
            if self.checked_button == Some(object_id) {
                self.checked_button = None;
            }
            Some(info.id)
        } else {
            None
        }
    }

    /// Get the ID assigned to a button.
    ///
    /// Returns `-1` if the button is not in the group.
    pub fn id(&self, object_id: ObjectId) -> i32 {
        self.buttons.get(&object_id).map(|i| i.id).unwrap_or(-1)
    }

    /// Set the ID for a button already in the group.
    pub fn set_id(&mut self, object_id: ObjectId, id: i32) {
        if let Some(info) = self.buttons.get_mut(&object_id) {
            info.id = id;
        }
    }

    /// Get the ObjectId for a button with a specific ID.
    ///
    /// Returns `None` if no button has that ID.
    pub fn button(&self, id: i32) -> Option<ObjectId> {
        self.buttons
            .iter()
            .find(|(_, info)| info.id == id)
            .map(|(&object_id, _)| object_id)
    }

    /// Get all buttons in the group.
    pub fn buttons(&self) -> Vec<ObjectId> {
        self.buttons.keys().copied().collect()
    }

    /// Check if a button is in the group.
    pub fn contains(&self, object_id: ObjectId) -> bool {
        self.buttons.contains_key(&object_id)
    }

    // =========================================================================
    // Checked State
    // =========================================================================

    /// Get the ObjectId of the currently checked button.
    ///
    /// Returns `None` if no button is checked.
    pub fn checked_button(&self) -> Option<ObjectId> {
        self.checked_button
    }

    /// Get the ID of the currently checked button.
    ///
    /// Returns `-1` if no button is checked.
    pub fn checked_id(&self) -> i32 {
        self.checked_button
            .and_then(|id| self.buttons.get(&id))
            .map(|info| info.id)
            .unwrap_or(-1)
    }

    /// Called when a button in the group is toggled.
    ///
    /// This method should be called by buttons when their checked state changes.
    /// It returns a list of ObjectIds that should be unchecked (for exclusive mode).
    ///
    /// # Returns
    ///
    /// A vector of ObjectIds for buttons that should be unchecked.
    pub fn button_toggled(&mut self, object_id: ObjectId, checked: bool) -> Vec<ObjectId> {
        let mut buttons_to_uncheck = Vec::new();

        // Update our internal state
        if let Some(info) = self.buttons.get_mut(&object_id) {
            info.checked = checked;
            self.id_toggled.emit((info.id, checked));
        } else {
            // Button not in group, nothing to do
            return buttons_to_uncheck;
        }

        if checked && self.exclusive {
            // In exclusive mode, uncheck all other buttons
            for (&other_id, other_info) in &self.buttons {
                if other_id != object_id && other_info.checked {
                    buttons_to_uncheck.push(other_id);
                }
            }

            // Update our records for the buttons that will be unchecked
            for &other_id in &buttons_to_uncheck {
                if let Some(other_info) = self.buttons.get_mut(&other_id) {
                    other_info.checked = false;
                    self.id_toggled.emit((other_info.id, false));
                }
            }

            self.checked_button = Some(object_id);
        } else if !checked
            && self.checked_button == Some(object_id) {
                self.checked_button = None;
            }

        buttons_to_uncheck
    }

    /// Called when a button in the group is clicked.
    ///
    /// This emits the `id_clicked` signal.
    pub fn button_clicked(&mut self, object_id: ObjectId) {
        if let Some(info) = self.buttons.get(&object_id) {
            self.id_clicked.emit(info.id);
        }
    }

    /// Check if trying to uncheck the current button should be prevented.
    ///
    /// In exclusive mode, clicking the currently checked button should NOT
    /// uncheck it (users must click a different button to change selection).
    pub fn should_prevent_uncheck(&self, object_id: ObjectId) -> bool {
        self.exclusive && self.checked_button == Some(object_id)
    }
}

impl Default for ButtonGroup {
    fn default() -> Self {
        Self::new()
    }
}

// Ensure ButtonGroup is Send + Sync
static_assertions::assert_impl_all!(ButtonGroup: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use slotmap::SlotMap;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI32, Ordering};

    fn setup() {
        init_global_registry();
    }

    // Helper to create fake ObjectIds for testing
    fn make_test_ids(count: usize) -> Vec<ObjectId> {
        let mut map: SlotMap<ObjectId, ()> = SlotMap::with_key();
        (0..count).map(|_| map.insert(())).collect()
    }

    #[test]
    fn test_button_group_creation() {
        setup();
        let group = ButtonGroup::new();
        assert!(group.is_exclusive());
        assert!(group.buttons().is_empty());
        assert_eq!(group.checked_id(), -1);
    }

    #[test]
    fn test_add_buttons() {
        setup();
        let ids = make_test_ids(3);
        let mut group = ButtonGroup::new();

        let id1 = group.add_button(ids[0]);
        let id2 = group.add_button(ids[1]);
        group.add_button_with_id(ids[2], 100);

        assert_eq!(id1, -2);
        assert_eq!(id2, -3);
        assert_eq!(group.id(ids[2]), 100);
        assert_eq!(group.buttons().len(), 3);
    }

    #[test]
    fn test_remove_button() {
        setup();
        let ids = make_test_ids(2);
        let mut group = ButtonGroup::new();

        group.add_button_with_id(ids[0], 1);
        group.add_button_with_id(ids[1], 2);

        let removed_id = group.remove_button(ids[0]);
        assert_eq!(removed_id, Some(1));
        assert!(!group.contains(ids[0]));
        assert!(group.contains(ids[1]));
    }

    #[test]
    fn test_exclusive_mode() {
        setup();
        let ids = make_test_ids(3);
        let mut group = ButtonGroup::new();

        group.add_button_with_id(ids[0], 1);
        group.add_button_with_id(ids[1], 2);
        group.add_button_with_id(ids[2], 3);

        // Check first button
        let to_uncheck = group.button_toggled(ids[0], true);
        assert!(to_uncheck.is_empty());
        assert_eq!(group.checked_id(), 1);

        // Check second button - first should be in uncheck list
        let to_uncheck = group.button_toggled(ids[1], true);
        assert_eq!(to_uncheck.len(), 1);
        assert_eq!(to_uncheck[0], ids[0]);
        assert_eq!(group.checked_id(), 2);
    }

    #[test]
    fn test_non_exclusive_mode() {
        setup();
        let ids = make_test_ids(3);
        let mut group = ButtonGroup::with_exclusive(false);

        group.add_button_with_id(ids[0], 1);
        group.add_button_with_id(ids[1], 2);

        // Check first button
        let to_uncheck = group.button_toggled(ids[0], true);
        assert!(to_uncheck.is_empty());

        // Check second button - first should remain checked
        let to_uncheck = group.button_toggled(ids[1], true);
        assert!(to_uncheck.is_empty());
    }

    #[test]
    fn test_prevent_uncheck_in_exclusive() {
        setup();
        let ids = make_test_ids(2);
        let mut group = ButtonGroup::new();

        group.add_button_with_id(ids[0], 1);
        group.add_button_with_id(ids[1], 2);

        // Check first button
        group.button_toggled(ids[0], true);

        // Should prevent unchecking the checked button
        assert!(group.should_prevent_uncheck(ids[0]));
        assert!(!group.should_prevent_uncheck(ids[1]));
    }

    #[test]
    fn test_id_toggled_signal() {
        setup();
        let ids = make_test_ids(2);
        let mut group = ButtonGroup::new();

        group.add_button_with_id(ids[0], 1);
        group.add_button_with_id(ids[1], 2);

        let last_toggled = Arc::new(AtomicI32::new(0));
        let last_toggled_clone = last_toggled.clone();
        group.id_toggled.connect(move |&(id, checked)| {
            if checked {
                last_toggled_clone.store(id, Ordering::SeqCst);
            }
        });

        group.button_toggled(ids[0], true);
        assert_eq!(last_toggled.load(Ordering::SeqCst), 1);

        group.button_toggled(ids[1], true);
        assert_eq!(last_toggled.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_button_lookup() {
        setup();
        let ids = make_test_ids(2);
        let mut group = ButtonGroup::new();

        group.add_button_with_id(ids[0], 10);
        group.add_button_with_id(ids[1], 20);

        assert_eq!(group.button(10), Some(ids[0]));
        assert_eq!(group.button(20), Some(ids[1]));
        assert_eq!(group.button(30), None);
    }
}
