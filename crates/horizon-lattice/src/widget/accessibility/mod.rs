//! Accessibility support for Horizon Lattice widgets.
//!
//! This module provides integration with platform accessibility APIs through
//! [AccessKit](https://accesskit.dev/). It enables screen reader support and
//! other assistive technology integration on Windows (UI Automation),
//! macOS (NSAccessibility), and Linux (AT-SPI).
//!
//! # Architecture
//!
//! The accessibility system consists of:
//!
//! - [`Accessible`] trait: Implemented by widgets to provide accessibility information
//! - [`AccessibilityManager`]: Per-window manager that builds and maintains the accessibility tree
//! - [`AccessibleRole`]: Widget accessibility roles (button, checkbox, etc.)
//!
//! # Example
//!
//! Widgets automatically provide accessibility information through their
//! [`Accessible`] implementation:
//!
//! ```ignore
//! use horizon_lattice::widget::{Accessible, AccessibleRole};
//!
//! impl Accessible for MyWidget {
//!     fn accessible_role(&self) -> AccessibleRole {
//!         AccessibleRole::Button
//!     }
//!
//!     fn accessible_name(&self) -> Option<String> {
//!         Some(self.label.clone())
//!     }
//! }
//! ```

mod manager;
mod node;
mod role;

pub use manager::AccessibilityManager;
pub use node::Accessible;
pub use role::AccessibleRole;

use accesskit::NodeId as AccessKitNodeId;
use horizon_lattice_core::ObjectId;

/// Convert an ObjectId to an AccessKit NodeId.
///
/// We use a hash of the ObjectId's components for a stable identifier.
pub(crate) fn object_id_to_node_id(id: ObjectId) -> AccessKitNodeId {
    // ObjectId contains version and index info. We convert to a stable u64.
    // We use the raw pointer representation which is stable during runtime.
    let raw = id.as_raw();
    AccessKitNodeId(raw)
}

/// Convert an AccessKit NodeId back to an ObjectId.
///
/// Returns None if the ObjectId is not valid in the registry.
pub(crate) fn node_id_to_object_id(id: AccessKitNodeId) -> Option<ObjectId> {
    ObjectId::from_raw(id.0)
}
