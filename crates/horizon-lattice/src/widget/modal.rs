//! Modal dialog management.
//!
//! This module provides [`ModalManager`], which tracks active modal dialogs
//! and determines whether input should be blocked for non-modal windows.
//!
//! # Modal Types
//!
//! - **Non-modal**: Window doesn't block any other windows
//! - **Window-modal**: Blocks only its parent window
//! - **Application-modal**: Blocks all other windows in the application
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice::widget::{ModalManager, WindowModality};
//!
//! // Push a modal dialog onto the stack
//! ModalManager::push_modal(dialog_id, WindowModality::ApplicationModal, None);
//!
//! // Check if a window is blocked
//! if ModalManager::is_blocked(window_id) {
//!     // Don't deliver input events to this window
//! }
//!
//! // Pop the modal when dialog closes
//! ModalManager::pop_modal(dialog_id);
//! ```

use std::sync::Mutex;

use horizon_lattice_core::ObjectId;

use super::widgets::WindowModality;

/// Information about an active modal dialog.
#[derive(Debug, Clone)]
struct ModalEntry {
    /// The ObjectId of the modal dialog.
    dialog_id: ObjectId,
    /// The modality type.
    modality: WindowModality,
    /// The parent window ID (for WindowModal).
    parent_id: Option<ObjectId>,
}

/// Global state for modal dialog management.
static MODAL_STACK: Mutex<Vec<ModalEntry>> = Mutex::new(Vec::new());

/// Manages modal dialog state and input blocking.
///
/// `ModalManager` maintains a stack of active modal dialogs and provides
/// methods to check whether input should be blocked for specific windows.
///
/// Modal dialogs are ordered in a stack - the most recently opened modal
/// is on top and takes precedence for input.
///
/// # Thread Safety
///
/// All methods use interior mutability with a global lock, making them
/// safe to call from any thread.
pub struct ModalManager;

impl ModalManager {
    /// Push a modal dialog onto the stack.
    ///
    /// Call this when a modal dialog is opened/shown.
    ///
    /// # Arguments
    ///
    /// * `dialog_id` - The ObjectId of the modal dialog
    /// * `modality` - The type of modality
    /// * `parent_id` - The parent window (required for WindowModal)
    pub fn push_modal(
        dialog_id: ObjectId,
        modality: WindowModality,
        parent_id: Option<ObjectId>,
    ) {
        if modality.is_non_modal() {
            return;
        }

        let mut stack = MODAL_STACK.lock().unwrap();

        // Don't add duplicates
        if stack.iter().any(|e| e.dialog_id == dialog_id) {
            return;
        }

        stack.push(ModalEntry {
            dialog_id,
            modality,
            parent_id,
        });
    }

    /// Remove a modal dialog from the stack.
    ///
    /// Call this when a modal dialog is closed/hidden.
    ///
    /// # Arguments
    ///
    /// * `dialog_id` - The ObjectId of the modal dialog to remove
    ///
    /// # Returns
    ///
    /// `true` if the dialog was found and removed, `false` otherwise.
    pub fn pop_modal(dialog_id: ObjectId) -> bool {
        let mut stack = MODAL_STACK.lock().unwrap();

        if let Some(pos) = stack.iter().position(|e| e.dialog_id == dialog_id) {
            stack.remove(pos);
            true
        } else {
            false
        }
    }

    /// Check if there is any active modal dialog.
    pub fn has_modal() -> bool {
        let stack = MODAL_STACK.lock().unwrap();
        !stack.is_empty()
    }

    /// Get the currently active (topmost) modal dialog.
    ///
    /// # Returns
    ///
    /// The ObjectId of the topmost modal dialog, or `None` if no modals are active.
    pub fn active_modal() -> Option<ObjectId> {
        let stack = MODAL_STACK.lock().unwrap();
        stack.last().map(|e| e.dialog_id)
    }

    /// Check if input to a window should be blocked.
    ///
    /// A window is blocked if:
    /// - An application-modal dialog is active and the window is not that dialog
    /// - A window-modal dialog is active for this window's parent
    ///
    /// # Arguments
    ///
    /// * `window_id` - The ObjectId of the window to check
    ///
    /// # Returns
    ///
    /// `true` if input should be blocked, `false` if input should be allowed.
    pub fn is_blocked(window_id: ObjectId) -> bool {
        let stack = MODAL_STACK.lock().unwrap();

        // Check from top to bottom of stack
        for entry in stack.iter().rev() {
            // The modal itself is never blocked
            if entry.dialog_id == window_id {
                return false;
            }

            match entry.modality {
                WindowModality::ApplicationModal => {
                    // Application modal blocks everything except itself
                    return true;
                }
                WindowModality::WindowModal => {
                    // Window modal only blocks its parent
                    if entry.parent_id == Some(window_id) {
                        return true;
                    }
                }
                WindowModality::NonModal => {
                    // Non-modal doesn't block anything
                }
            }
        }

        false
    }

    /// Check if input to a window should be blocked, considering parent chain.
    ///
    /// This is similar to `is_blocked` but also considers whether any ancestor
    /// of the target window is blocked.
    ///
    /// # Arguments
    ///
    /// * `window_id` - The ObjectId of the window to check
    /// * `get_parent` - A function to get the parent of a window
    ///
    /// # Returns
    ///
    /// `true` if input should be blocked, `false` if input should be allowed.
    pub fn is_blocked_with_ancestors<F>(window_id: ObjectId, get_parent: F) -> bool
    where
        F: Fn(ObjectId) -> Option<ObjectId>,
    {
        // Check the window itself
        if Self::is_blocked(window_id) {
            return true;
        }

        // Check ancestors
        let mut current = get_parent(window_id);
        while let Some(parent_id) = current {
            if Self::is_blocked(parent_id) {
                return true;
            }
            current = get_parent(parent_id);
        }

        false
    }

    /// Check if a specific dialog can receive input.
    ///
    /// A dialog can receive input if it is on top of the modal stack
    /// or if there are no modal dialogs above it.
    ///
    /// # Arguments
    ///
    /// * `dialog_id` - The ObjectId of the dialog to check
    ///
    /// # Returns
    ///
    /// `true` if the dialog can receive input, `false` if it's blocked
    /// by another modal.
    pub fn can_receive_input(dialog_id: ObjectId) -> bool {
        let stack = MODAL_STACK.lock().unwrap();

        // If no modals, everything can receive input
        if stack.is_empty() {
            return true;
        }

        // Find this dialog in the stack
        let Some(pos) = stack.iter().position(|e| e.dialog_id == dialog_id) else {
            // Dialog not in stack - check if blocked
            drop(stack);
            return !Self::is_blocked(dialog_id);
        };

        // Check if any modals above us would block us
        for entry in stack.iter().skip(pos + 1) {
            match entry.modality {
                WindowModality::ApplicationModal => return false,
                WindowModality::WindowModal => {
                    if entry.parent_id == Some(dialog_id) {
                        return false;
                    }
                }
                WindowModality::NonModal => {}
            }
        }

        true
    }

    /// Clear all modal dialogs from the stack.
    ///
    /// This is primarily useful for testing or cleanup scenarios.
    pub fn clear() {
        let mut stack = MODAL_STACK.lock().unwrap();
        stack.clear();
    }

    /// Get the number of active modal dialogs.
    pub fn modal_count() -> usize {
        let stack = MODAL_STACK.lock().unwrap();
        stack.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
        ModalManager::clear();
    }

    fn make_id(n: u64) -> ObjectId {
        // Create a unique ObjectId for testing
        // We use a base value that's valid for slotmap (needs non-zero generation)
        // The formula creates valid slotmap keys: index in lower 32 bits, generation (non-zero) in upper 32
        ObjectId::from_raw((1_u64 << 32) | n).expect("valid test id")
    }

    #[test]
    fn test_no_modal_nothing_blocked() {
        setup();

        let window = make_id(1);
        assert!(!ModalManager::is_blocked(window));
        assert!(!ModalManager::has_modal());
    }

    #[test]
    fn test_application_modal_blocks_all() {
        setup();

        let dialog = make_id(1);
        let window1 = make_id(2);
        let window2 = make_id(3);

        ModalManager::push_modal(dialog, WindowModality::ApplicationModal, None);

        assert!(ModalManager::has_modal());
        assert_eq!(ModalManager::active_modal(), Some(dialog));

        // Dialog itself is not blocked
        assert!(!ModalManager::is_blocked(dialog));

        // Other windows are blocked
        assert!(ModalManager::is_blocked(window1));
        assert!(ModalManager::is_blocked(window2));
    }

    #[test]
    fn test_window_modal_blocks_parent_only() {
        setup();

        let parent = make_id(1);
        let dialog = make_id(2);
        let other_window = make_id(3);

        ModalManager::push_modal(dialog, WindowModality::WindowModal, Some(parent));

        // Dialog is not blocked
        assert!(!ModalManager::is_blocked(dialog));

        // Parent is blocked
        assert!(ModalManager::is_blocked(parent));

        // Other windows are not blocked
        assert!(!ModalManager::is_blocked(other_window));
    }

    #[test]
    fn test_pop_modal() {
        setup();

        let dialog = make_id(1);
        let window = make_id(2);

        ModalManager::push_modal(dialog, WindowModality::ApplicationModal, None);
        assert!(ModalManager::is_blocked(window));

        assert!(ModalManager::pop_modal(dialog));
        assert!(!ModalManager::is_blocked(window));
        assert!(!ModalManager::has_modal());
    }

    #[test]
    fn test_modal_stack_order() {
        setup();

        let dialog1 = make_id(1);
        let dialog2 = make_id(2);
        let window = make_id(3);

        ModalManager::push_modal(dialog1, WindowModality::ApplicationModal, None);
        ModalManager::push_modal(dialog2, WindowModality::ApplicationModal, None);

        // Topmost modal is dialog2
        assert_eq!(ModalManager::active_modal(), Some(dialog2));

        // Window is blocked
        assert!(ModalManager::is_blocked(window));

        // Both dialogs can receive input check (dialog1 is blocked by dialog2)
        assert!(ModalManager::can_receive_input(dialog2));
        assert!(!ModalManager::can_receive_input(dialog1));

        // Pop dialog2, dialog1 is now active
        ModalManager::pop_modal(dialog2);
        assert_eq!(ModalManager::active_modal(), Some(dialog1));
        assert!(ModalManager::can_receive_input(dialog1));
    }

    #[test]
    fn test_non_modal_not_tracked() {
        setup();

        let dialog = make_id(1);
        let window = make_id(2);

        ModalManager::push_modal(dialog, WindowModality::NonModal, None);

        // Non-modal dialogs are not tracked
        assert!(!ModalManager::has_modal());
        assert!(!ModalManager::is_blocked(window));
    }

    #[test]
    fn test_duplicate_push_ignored() {
        setup();

        let dialog = make_id(1);

        ModalManager::push_modal(dialog, WindowModality::ApplicationModal, None);
        ModalManager::push_modal(dialog, WindowModality::ApplicationModal, None);

        assert_eq!(ModalManager::modal_count(), 1);
    }
}
