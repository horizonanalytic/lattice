//! Model/View architecture for Horizon Lattice.
//!
//! This module provides the foundational types for the Model/View pattern,
//! which separates data representation from display logic. This enables:
//!
//! - Multiple views of the same data
//! - Consistent data access patterns
//! - Efficient updates via change notifications
//! - Support for hierarchical (tree) data structures
//!
//! # Core Types
//!
//! - [`ModelIndex`]: Identifies an item's position in a model
//! - [`ItemRole`]: Specifies what type of data to access
//! - [`ItemData`]: Type-erased container for item data
//! - [`ItemModel`]: The trait that models implement
//! - [`ModelSignals`]: Signals for change notifications
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::model::*;
//!
//! // Query a model
//! let root = ModelIndex::invalid();
//! let first_item = model.index(0, 0, &root);
//!
//! if first_item.is_valid() {
//!     if let Some(text) = model.display_text(&first_item) {
//!         println!("First item: {}", text);
//!     }
//! }
//!
//! // Connect to change notifications
//! model.signals().data_changed.connect(|(top_left, bottom_right, roles)| {
//!     println!("Data changed from {:?} to {:?}", top_left, bottom_right);
//! });
//! ```
//!
//! # Architecture Overview
//!
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │    Model    │────>│   Signals   │────>│    View     │
//! │ (ItemModel) │     │             │     │             │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!       │                                       │
//!       │         ┌─────────────┐               │
//!       └────────>│ ModelIndex  │<──────────────┘
//!                 │  ItemRole   │
//!                 │  ItemData   │
//!                 └─────────────┘
//! ```
//!
//! Views query models using `ModelIndex` and `ItemRole` to get `ItemData`.
//! Models emit signals when data changes, which views listen to for updates.

mod index;
mod role;
mod traits;

pub use index::ModelIndex;
pub use role::{
    CheckState, HorizontalAlignment, ItemData, ItemRole, TextAlignment, VerticalAlignment,
};
pub use traits::{ItemFlags, ItemModel, ModelSignals, Orientation};
