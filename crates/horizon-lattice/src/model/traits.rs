//! Core traits for the Model/View architecture.
//!
//! This module defines the fundamental traits that models must implement
//! to work with the view system.

use horizon_lattice_core::Signal;

use super::index::ModelIndex;
use super::role::{CheckState, ItemData, ItemRole};

/// Flags indicating what operations are allowed on an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemFlags {
    /// Item can be selected.
    pub selectable: bool,
    /// Item can be edited.
    pub editable: bool,
    /// Item can be dragged.
    pub drag_enabled: bool,
    /// Item can receive drops.
    pub drop_enabled: bool,
    /// Item has a checkbox.
    pub checkable: bool,
    /// Item is enabled (can interact).
    pub enabled: bool,
    /// Item has a tri-state checkbox.
    pub tristate: bool,
    /// Item should never have children (optimizes views).
    pub never_has_children: bool,
}

impl ItemFlags {
    /// Creates flags with all defaults (selectable and enabled only).
    pub fn new() -> Self {
        Self {
            selectable: true,
            enabled: true,
            ..Default::default()
        }
    }

    /// Creates flags for a disabled item.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Default::default()
        }
    }

    /// Creates flags for an editable item.
    pub fn editable() -> Self {
        Self {
            selectable: true,
            editable: true,
            enabled: true,
            ..Default::default()
        }
    }

    /// Creates flags for a checkable item.
    pub fn checkable() -> Self {
        Self {
            selectable: true,
            checkable: true,
            enabled: true,
            ..Default::default()
        }
    }

    /// Sets the selectable flag.
    pub fn with_selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Sets the editable flag.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Sets the checkable flag.
    pub fn with_checkable(mut self, checkable: bool) -> Self {
        self.checkable = checkable;
        self
    }

    /// Sets the enabled flag.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the drag enabled flag.
    pub fn with_drag(mut self, enabled: bool) -> Self {
        self.drag_enabled = enabled;
        self
    }

    /// Sets the drop enabled flag.
    pub fn with_drop(mut self, enabled: bool) -> Self {
        self.drop_enabled = enabled;
        self
    }
}

/// The core trait for item models in the Model/View architecture.
///
/// `ItemModel` provides a flexible interface for representing hierarchical
/// data. Views use this interface to query and display data without needing
/// to know the underlying data structure.
///
/// # Implementation Requirements
///
/// At minimum, you must implement:
/// - [`row_count`](ItemModel::row_count) - Number of rows under a parent
/// - [`column_count`](ItemModel::column_count) - Number of columns
/// - [`data`](ItemModel::data) - Data for a given index and role
/// - [`index`](ItemModel::index) - Create an index for a position
/// - [`parent`](ItemModel::parent) - Get the parent of an index
///
/// For editable models, also implement:
/// - [`set_data`](ItemModel::set_data) - Modify data at an index
/// - [`flags`](ItemModel::flags) - Return appropriate flags
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::model::{ItemModel, ModelIndex, ItemRole, ItemData, ItemFlags};
///
/// struct MyListModel {
///     items: Vec<String>,
///     signals: ModelSignals,
/// }
///
/// impl ItemModel for MyListModel {
///     fn row_count(&self, parent: &ModelIndex) -> usize {
///         if parent.is_valid() { 0 } else { self.items.len() }
///     }
///
///     fn column_count(&self, _parent: &ModelIndex) -> usize {
///         1
///     }
///
///     fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
///         if !index.is_valid() || index.row() >= self.items.len() {
///             return ItemData::None;
///         }
///         match role {
///             ItemRole::Display => ItemData::from(&self.items[index.row()]),
///             _ => ItemData::None,
///         }
///     }
///
///     fn index(&self, row: usize, col: usize, parent: &ModelIndex) -> ModelIndex {
///         if parent.is_valid() || row >= self.items.len() || col > 0 {
///             ModelIndex::invalid()
///         } else {
///             ModelIndex::new(row, col, parent.clone())
///         }
///     }
///
///     fn parent(&self, _index: &ModelIndex) -> ModelIndex {
///         ModelIndex::invalid() // Flat list has no parents
///     }
///
///     fn signals(&self) -> &ModelSignals {
///         &self.signals
///     }
/// }
/// ```
pub trait ItemModel: Send + Sync {
    /// Returns the number of rows under the given parent.
    ///
    /// For list models, return the item count when parent is invalid.
    /// For tree models, return the number of children of the parent item.
    fn row_count(&self, parent: &ModelIndex) -> usize;

    /// Returns the number of columns for children of the given parent.
    ///
    /// Most models have a fixed column count, but tree models may have
    /// varying columns at different levels.
    fn column_count(&self, parent: &ModelIndex) -> usize;

    /// Returns the data stored under the given role for the item at index.
    ///
    /// Return `ItemData::None` if:
    /// - The index is invalid
    /// - The role is not supported
    /// - There's no data for that role
    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData;

    /// Creates a model index for the given row and column under parent.
    ///
    /// Return `ModelIndex::invalid()` if the position is out of bounds.
    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex;

    /// Returns the parent of the given index.
    ///
    /// Return `ModelIndex::invalid()` for:
    /// - Root-level items
    /// - Invalid indices
    /// - Flat (non-hierarchical) models
    fn parent(&self, index: &ModelIndex) -> ModelIndex;

    /// Returns the signals for this model.
    ///
    /// Views connect to these signals to receive notifications about
    /// data changes, insertions, removals, etc.
    fn signals(&self) -> &ModelSignals;

    // -------------------------------------------------------------------------
    // Optional methods with default implementations
    // -------------------------------------------------------------------------

    /// Sets the data for the given index and role.
    ///
    /// Returns `true` if the data was successfully set.
    /// The default implementation returns `false` (read-only).
    ///
    /// Implementations should emit `data_changed` signal after modifying data.
    fn set_data(&self, _index: &ModelIndex, _value: ItemData, _role: ItemRole) -> bool {
        false
    }

    /// Returns the flags for the item at the given index.
    ///
    /// The default returns selectable and enabled flags.
    fn flags(&self, _index: &ModelIndex) -> ItemFlags {
        ItemFlags::new()
    }

    /// Returns `true` if the item at parent has any children.
    ///
    /// The default implementation checks if `row_count(parent) > 0`.
    /// Override for performance if checking children is expensive.
    fn has_children(&self, parent: &ModelIndex) -> bool {
        self.row_count(parent) > 0
    }

    /// Returns header data for the given section (row or column header).
    ///
    /// - For horizontal headers, `section` is the column index
    /// - For vertical headers, `section` is the row index
    ///
    /// The default returns `ItemData::None`.
    fn header_data(&self, _section: usize, _orientation: Orientation, _role: ItemRole) -> ItemData {
        ItemData::None
    }

    /// Sets header data for the given section.
    ///
    /// Returns `true` if the data was successfully set.
    /// The default returns `false`.
    fn set_header_data(
        &self,
        _section: usize,
        _orientation: Orientation,
        _value: ItemData,
        _role: ItemRole,
    ) -> bool {
        false
    }

    /// Returns `true` if more data can be fetched for the given parent.
    ///
    /// Used for lazy loading / incremental data fetching.
    /// The default returns `false`.
    fn can_fetch_more(&self, _parent: &ModelIndex) -> bool {
        false
    }

    /// Fetches more data for the given parent.
    ///
    /// Called by views when they need more data and `can_fetch_more` is true.
    /// The default does nothing.
    fn fetch_more(&self, _parent: &ModelIndex) {}

    // -------------------------------------------------------------------------
    // Convenience methods
    // -------------------------------------------------------------------------

    /// Returns the display text for an item (convenience for `data(index, Display)`).
    fn display_text(&self, index: &ModelIndex) -> Option<String> {
        self.data(index, ItemRole::Display).into_string()
    }

    /// Returns the check state for an item.
    fn check_state(&self, index: &ModelIndex) -> Option<CheckState> {
        self.data(index, ItemRole::CheckState).as_check_state()
    }

    /// Sets the check state for an item (convenience for `set_data`).
    fn set_check_state(&self, index: &ModelIndex, state: CheckState) -> bool {
        self.set_data(index, ItemData::CheckState(state), ItemRole::CheckState)
    }

    /// Creates a sibling index at the given row and column.
    ///
    /// This validates against the model, unlike `ModelIndex::sibling`.
    fn sibling(&self, index: &ModelIndex, row: usize, column: usize) -> ModelIndex {
        if !index.is_valid() {
            return ModelIndex::invalid();
        }
        self.index(row, column, &index.parent())
    }
}

/// Header orientation for `header_data`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Orientation {
    /// Horizontal header (column headers).
    Horizontal,
    /// Vertical header (row headers).
    Vertical,
}

/// Collection of signals emitted by item models.
///
/// Views connect to these signals to stay synchronized with the model.
/// Models should emit the appropriate signals when their data changes.
///
/// # Signal Usage
///
/// - **Before modifications**: Emit `rows_about_to_be_*` or `layout_about_to_change`
/// - **After modifications**: Emit `rows_*` or `layout_changed`
/// - **Data changes**: Emit `data_changed` for value modifications
/// - **Major restructuring**: Emit `model_reset` signals
pub struct ModelSignals {
    // -------------------------------------------------------------------------
    // Row modification signals
    // -------------------------------------------------------------------------
    /// Emitted just before rows are inserted.
    /// Args: (parent index, first row, last row)
    pub rows_about_to_be_inserted: Signal<(ModelIndex, usize, usize)>,

    /// Emitted after rows have been inserted.
    /// Args: (parent index, first row, last row)
    pub rows_inserted: Signal<(ModelIndex, usize, usize)>,

    /// Emitted just before rows are removed.
    /// Args: (parent index, first row, last row)
    pub rows_about_to_be_removed: Signal<(ModelIndex, usize, usize)>,

    /// Emitted after rows have been removed.
    /// Args: (parent index, first row, last row)
    pub rows_removed: Signal<(ModelIndex, usize, usize)>,

    /// Emitted just before rows are moved.
    /// Args: (source parent, source first, source last, dest parent, dest row)
    pub rows_about_to_be_moved: Signal<(ModelIndex, usize, usize, ModelIndex, usize)>,

    /// Emitted after rows have been moved.
    /// Args: (source parent, source first, source last, dest parent, dest row)
    pub rows_moved: Signal<(ModelIndex, usize, usize, ModelIndex, usize)>,

    // -------------------------------------------------------------------------
    // Column modification signals
    // -------------------------------------------------------------------------
    /// Emitted just before columns are inserted.
    pub columns_about_to_be_inserted: Signal<(ModelIndex, usize, usize)>,

    /// Emitted after columns have been inserted.
    pub columns_inserted: Signal<(ModelIndex, usize, usize)>,

    /// Emitted just before columns are removed.
    pub columns_about_to_be_removed: Signal<(ModelIndex, usize, usize)>,

    /// Emitted after columns have been removed.
    pub columns_removed: Signal<(ModelIndex, usize, usize)>,

    // -------------------------------------------------------------------------
    // Data change signals
    // -------------------------------------------------------------------------
    /// Emitted when data in existing items changes.
    /// Args: (top-left index, bottom-right index, changed roles)
    pub data_changed: Signal<(ModelIndex, ModelIndex, Vec<ItemRole>)>,

    /// Emitted when header data changes.
    /// Args: (orientation, first section, last section)
    pub header_data_changed: Signal<(Orientation, usize, usize)>,

    // -------------------------------------------------------------------------
    // Layout signals
    // -------------------------------------------------------------------------
    /// Emitted before a layout change (e.g., sorting).
    pub layout_about_to_change: Signal<()>,

    /// Emitted after a layout change.
    pub layout_changed: Signal<()>,

    // -------------------------------------------------------------------------
    // Reset signals
    // -------------------------------------------------------------------------
    /// Emitted before the model is reset.
    pub model_about_to_reset: Signal<()>,

    /// Emitted after the model has been reset.
    pub model_reset: Signal<()>,
}

impl Default for ModelSignals {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelSignals {
    /// Creates a new set of model signals.
    pub fn new() -> Self {
        Self {
            rows_about_to_be_inserted: Signal::new(),
            rows_inserted: Signal::new(),
            rows_about_to_be_removed: Signal::new(),
            rows_removed: Signal::new(),
            rows_about_to_be_moved: Signal::new(),
            rows_moved: Signal::new(),
            columns_about_to_be_inserted: Signal::new(),
            columns_inserted: Signal::new(),
            columns_about_to_be_removed: Signal::new(),
            columns_removed: Signal::new(),
            data_changed: Signal::new(),
            header_data_changed: Signal::new(),
            layout_about_to_change: Signal::new(),
            layout_changed: Signal::new(),
            model_about_to_reset: Signal::new(),
            model_reset: Signal::new(),
        }
    }

    // -------------------------------------------------------------------------
    // Convenience methods for emitting signals
    // -------------------------------------------------------------------------

    /// Emits signals for row insertion.
    ///
    /// Calls the provided function between the about_to_be_inserted and inserted signals.
    pub fn emit_rows_inserted<F>(&self, parent: ModelIndex, first: usize, last: usize, insert_fn: F)
    where
        F: FnOnce(),
    {
        self.rows_about_to_be_inserted
            .emit((parent.clone(), first, last));
        insert_fn();
        self.rows_inserted.emit((parent, first, last));
    }

    /// Emits signals for row removal.
    ///
    /// Calls the provided function between the about_to_be_removed and removed signals.
    pub fn emit_rows_removed<F>(&self, parent: ModelIndex, first: usize, last: usize, remove_fn: F)
    where
        F: FnOnce(),
    {
        self.rows_about_to_be_removed
            .emit((parent.clone(), first, last));
        remove_fn();
        self.rows_removed.emit((parent, first, last));
    }

    /// Emits the data_changed signal for a single item.
    pub fn emit_data_changed_single(&self, index: ModelIndex, roles: Vec<ItemRole>) {
        self.data_changed.emit((index.clone(), index, roles));
    }

    /// Emits signals for a model reset.
    ///
    /// Calls the provided function between the about_to_reset and reset signals.
    pub fn emit_reset<F>(&self, reset_fn: F)
    where
        F: FnOnce(),
    {
        self.model_about_to_reset.emit(());
        reset_fn();
        self.model_reset.emit(());
    }

    /// Emits signals for a layout change.
    ///
    /// Calls the provided function between the about_to_change and changed signals.
    pub fn emit_layout_changed<F>(&self, change_fn: F)
    where
        F: FnOnce(),
    {
        self.layout_about_to_change.emit(());
        change_fn();
        self.layout_changed.emit(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[test]
    fn test_item_flags() {
        let flags = ItemFlags::new();
        assert!(flags.selectable);
        assert!(flags.enabled);
        assert!(!flags.editable);
        assert!(!flags.checkable);

        let editable = ItemFlags::editable();
        assert!(editable.editable);
        assert!(editable.selectable);
    }

    #[test]
    fn test_model_signals_creation() {
        let signals = ModelSignals::new();
        assert_eq!(signals.rows_inserted.connection_count(), 0);
        assert_eq!(signals.data_changed.connection_count(), 0);
    }

    #[test]
    fn test_emit_rows_inserted() {
        let signals = ModelSignals::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        let recv_about = received.clone();
        signals
            .rows_about_to_be_inserted
            .connect(move |(parent, first, last)| {
                recv_about
                    .lock()
                    .push(("about", parent.row(), *first, *last));
            });

        let recv_done = received.clone();
        signals.rows_inserted.connect(move |(parent, first, last)| {
            recv_done.lock().push(("done", parent.row(), *first, *last));
        });

        let parent = ModelIndex::new(5, 0, ModelIndex::invalid());
        signals.emit_rows_inserted(parent, 0, 2, || {});

        let events = received.lock();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], ("about", 5, 0, 2));
        assert_eq!(events[1], ("done", 5, 0, 2));
    }

    #[test]
    fn test_emit_reset() {
        let signals = ModelSignals::new();
        let counter = Arc::new(Mutex::new(0));

        let c1 = counter.clone();
        signals.model_about_to_reset.connect(move |_| {
            *c1.lock() += 1;
        });

        let c2 = counter.clone();
        signals.model_reset.connect(move |_| {
            *c2.lock() += 10;
        });

        signals.emit_reset(|| {});
        assert_eq!(*counter.lock(), 11);
    }
}
