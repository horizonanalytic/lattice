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
//! # Model Implementations
//!
//! - [`ListModel`]: Simple list of items with trait-based or closure-based data extraction
//! - [`TableModel`]: 2D grid with rows and columns, supports headers
//! - [`TreeModel`]: Hierarchical tree structure with parent-child relationships
//! - [`ProxyModel`]: Wraps another model to provide filtering and sorting
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::model::*;
//!
//! // Create a list model with trait-based items
//! let model = ListModel::new(vec!["Apple".to_string(), "Banana".to_string()]);
//!
//! // Query the model
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

mod delegate;
mod index;
mod list_model;
mod proxy_model;
mod role;
mod table_model;
mod traits;
mod tree_model;

pub use delegate::{
    ClickRegion, DefaultItemDelegate, DelegatePaintContext, DelegateTheme, DecorationPosition,
    ItemDelegate, StyleOptionViewItem, ViewItemFeatures, ViewItemState,
};
pub use index::ModelIndex;
pub use list_model::{DataExtractor, ExtractorListModel, FlagsExtractor, ListItem, ListModel};
pub use proxy_model::{CompareFn, FilterFn, ProxyModel, ProxyModelBuilder};
pub use role::{
    CheckState, HorizontalAlignment, ItemData, ItemRole, TextAlignment, VerticalAlignment,
};
pub use table_model::{CellExtractor, HeaderExtractor, SimpleTableModel, TableModel};
pub use traits::{ItemFlags, ItemModel, ModelSignals, Orientation};
pub use tree_model::{ExtractorTreeModel, TreeModel, TreeNodeData};
