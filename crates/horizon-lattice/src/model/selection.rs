//! Selection model for item views.
//!
//! This module provides [`SelectionModel`], which manages selection state
//! for views like ListView, TableView, and TreeView.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::model::{SelectionModel, SelectionMode, SelectionFlags};
//!
//! let mut selection = SelectionModel::new();
//! selection.set_selection_mode(SelectionMode::ExtendedSelection);
//!
//! // Select an item
//! selection.select(index, SelectionFlags::CLEAR_AND_SELECT);
//!
//! // Check if selected
//! if selection.is_selected(&index) {
//!     println!("Item is selected");
//! }
//!
//! // Listen for changes
//! selection.selection_changed.connect(|(selected, deselected)| {
//!     println!("Selection changed: +{} -{}", selected.len(), deselected.len());
//! });
//! ```

use std::collections::HashSet;

use horizon_lattice_core::Signal;

use super::index::ModelIndex;

/// Selection behavior mode for views.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// No items can be selected.
    NoSelection,
    /// Only one item can be selected at a time (default).
    #[default]
    SingleSelection,
    /// Multiple items can be selected with Ctrl+click.
    MultiSelection,
    /// Range selection with Shift+click, extended by Ctrl+click.
    ExtendedSelection,
}

/// Selection behavior - what gets selected when user clicks.
///
/// This is distinct from [`SelectionMode`] which controls *how* selection works.
/// SelectionBehavior controls *what* gets selected (cells, rows, or columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SelectionBehavior {
    /// Select individual cells/items.
    #[default]
    SelectItems,
    /// Select entire rows when any cell in the row is clicked.
    SelectRows,
    /// Select entire columns when any cell in the column is clicked.
    SelectColumns,
}

/// Flags controlling selection operations.
///
/// These flags can be combined to perform complex selection operations.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectionFlags {
    /// Clear existing selection before applying operation.
    pub clear: bool,
    /// Select the specified indices.
    pub select: bool,
    /// Deselect the specified indices.
    pub deselect: bool,
    /// Toggle selection state of specified indices.
    pub toggle: bool,
    /// Set as current index (keyboard focus).
    pub current: bool,
    /// Update anchor point for range selection.
    pub anchor: bool,
}

impl SelectionFlags {
    /// No operation.
    pub const NONE: Self = Self::empty();

    /// Clear existing selection.
    pub const CLEAR: Self = Self {
        clear: true,
        ..Self::empty()
    };

    /// Select the index.
    pub const SELECT: Self = Self {
        select: true,
        ..Self::empty()
    };

    /// Deselect the index.
    pub const DESELECT: Self = Self {
        deselect: true,
        ..Self::empty()
    };

    /// Toggle selection of the index.
    pub const TOGGLE: Self = Self {
        toggle: true,
        ..Self::empty()
    };

    /// Clear existing selection and select the index.
    pub const CLEAR_AND_SELECT: Self = Self {
        clear: true,
        select: true,
        ..Self::empty()
    };

    /// Set as current index.
    pub const CURRENT: Self = Self {
        current: true,
        ..Self::empty()
    };

    /// Select and set as current.
    pub const SELECT_CURRENT: Self = Self {
        select: true,
        current: true,
        ..Self::empty()
    };

    /// Clear, select, and set as current.
    pub const CLEAR_SELECT_CURRENT: Self = Self {
        clear: true,
        select: true,
        current: true,
        ..Self::empty()
    };

    const fn empty() -> Self {
        Self {
            clear: false,
            select: false,
            deselect: false,
            toggle: false,
            current: false,
            anchor: false,
        }
    }

    /// Creates flags with clear set.
    pub fn with_clear(mut self) -> Self {
        self.clear = true;
        self
    }

    /// Creates flags with select set.
    pub fn with_select(mut self) -> Self {
        self.select = true;
        self
    }

    /// Creates flags with current set.
    pub fn with_current(mut self) -> Self {
        self.current = true;
        self
    }

    /// Creates flags with anchor set.
    pub fn with_anchor(mut self) -> Self {
        self.anchor = true;
        self
    }
}

/// Manages selection state for item views.
///
/// SelectionModel tracks which items are selected, the current (focused) item,
/// and the anchor point for range selections. It works with any model through
/// `ModelIndex`.
///
/// # Signals
///
/// - `selection_changed`: Emitted when selection changes, with (selected, deselected) indices
/// - `current_changed`: Emitted when current index changes, with (new, old) indices
pub struct SelectionModel {
    /// Current selection mode.
    mode: SelectionMode,

    /// Selection behavior (items, rows, or columns).
    behavior: SelectionBehavior,

    /// The current index (has keyboard focus).
    current: ModelIndex,

    /// Anchor index for range selection.
    anchor: ModelIndex,

    /// Set of selected item IDs for O(1) lookup.
    selected_ids: HashSet<u64>,

    /// Ordered list of selected indices.
    selected_indices: Vec<ModelIndex>,

    /// Emitted when selection changes. Args: (selected, deselected)
    pub selection_changed: Signal<(Vec<ModelIndex>, Vec<ModelIndex>)>,

    /// Emitted when current index changes. Args: (new, old)
    pub current_changed: Signal<(ModelIndex, ModelIndex)>,
}

impl Default for SelectionModel {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectionModel {
    /// Creates a new selection model with default settings.
    pub fn new() -> Self {
        Self {
            mode: SelectionMode::default(),
            behavior: SelectionBehavior::default(),
            current: ModelIndex::invalid(),
            anchor: ModelIndex::invalid(),
            selected_ids: HashSet::new(),
            selected_indices: Vec::new(),
            selection_changed: Signal::new(),
            current_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Selection Mode
    // =========================================================================

    /// Gets the current selection mode.
    pub fn selection_mode(&self) -> SelectionMode {
        self.mode
    }

    /// Sets the selection mode.
    ///
    /// Changing mode does not clear existing selection, but subsequent
    /// selections will follow the new mode's behavior.
    pub fn set_selection_mode(&mut self, mode: SelectionMode) {
        self.mode = mode;
    }

    // =========================================================================
    // Selection Behavior
    // =========================================================================

    /// Gets the current selection behavior.
    pub fn selection_behavior(&self) -> SelectionBehavior {
        self.behavior
    }

    /// Sets the selection behavior (items, rows, or columns).
    ///
    /// Changing behavior does not clear existing selection.
    pub fn set_selection_behavior(&mut self, behavior: SelectionBehavior) {
        self.behavior = behavior;
    }

    // =========================================================================
    // Current Index
    // =========================================================================

    /// Gets the current (focused) index.
    pub fn current_index(&self) -> &ModelIndex {
        &self.current
    }

    /// Sets the current index with optional selection flags.
    ///
    /// The current index represents keyboard focus and is distinct from
    /// selection, though they often move together.
    pub fn set_current_index(&mut self, index: ModelIndex, flags: SelectionFlags) {
        let old_current = std::mem::replace(&mut self.current, index.clone());

        if flags.current && old_current != index {
            self.current_changed.emit((index.clone(), old_current));
        }

        // Apply selection flags
        if flags.clear || flags.select || flags.deselect || flags.toggle {
            self.select(index.clone(), flags);
        }

        if flags.anchor {
            self.anchor = index;
        }
    }

    // =========================================================================
    // Anchor (for range selection)
    // =========================================================================

    /// Gets the anchor index for range selection.
    pub fn anchor_index(&self) -> &ModelIndex {
        &self.anchor
    }

    /// Sets the anchor index for range selection.
    pub fn set_anchor_index(&mut self, index: ModelIndex) {
        self.anchor = index;
    }

    // =========================================================================
    // Selection Queries
    // =========================================================================

    /// Checks if an index is selected.
    pub fn is_selected(&self, index: &ModelIndex) -> bool {
        if !index.is_valid() {
            return false;
        }
        self.selected_ids.contains(&index.internal_id())
    }

    /// Checks if a row is selected (any column in that row).
    pub fn is_row_selected(&self, row: usize) -> bool {
        self.selected_indices.iter().any(|idx| idx.row() == row)
    }

    /// Returns true if any items are selected.
    pub fn has_selection(&self) -> bool {
        !self.selected_indices.is_empty()
    }

    /// Returns the number of selected items.
    pub fn selected_count(&self) -> usize {
        self.selected_indices.len()
    }

    /// Returns the selected indices in selection order.
    pub fn selected_indices(&self) -> &[ModelIndex] {
        &self.selected_indices
    }

    /// Returns the selected rows.
    pub fn selected_rows(&self) -> Vec<usize> {
        let mut rows: Vec<usize> = self.selected_indices.iter().map(|idx| idx.row()).collect();
        rows.sort_unstable();
        rows.dedup();
        rows
    }

    /// Checks if a column is selected (any cell in that column).
    pub fn is_column_selected(&self, column: usize) -> bool {
        self.selected_indices.iter().any(|idx| idx.column() == column)
    }

    /// Returns the selected columns.
    pub fn selected_columns(&self) -> Vec<usize> {
        let mut cols: Vec<usize> = self.selected_indices.iter().map(|idx| idx.column()).collect();
        cols.sort_unstable();
        cols.dedup();
        cols
    }

    /// Checks if a specific cell (row, column) is selected.
    pub fn is_cell_selected(&self, row: usize, column: usize) -> bool {
        self.selected_indices
            .iter()
            .any(|idx| idx.row() == row && idx.column() == column)
    }

    /// Returns the selected cells as (row, column) tuples.
    pub fn selected_cells(&self) -> Vec<(usize, usize)> {
        self.selected_indices
            .iter()
            .map(|idx| (idx.row(), idx.column()))
            .collect()
    }

    // =========================================================================
    // Selection Operations
    // =========================================================================

    /// Performs a selection operation on an index.
    ///
    /// The behavior depends on the flags:
    /// - `clear`: Deselects all items first
    /// - `select`: Adds the index to selection
    /// - `deselect`: Removes the index from selection
    /// - `toggle`: Toggles the selection state
    pub fn select(&mut self, index: ModelIndex, flags: SelectionFlags) {
        if self.mode == SelectionMode::NoSelection {
            return;
        }

        let mut newly_selected = Vec::new();
        let mut newly_deselected = Vec::new();

        // Clear existing selection if requested
        if flags.clear && !self.selected_indices.is_empty() {
            newly_deselected = std::mem::take(&mut self.selected_indices);
            self.selected_ids.clear();
        }

        // Apply operation
        if index.is_valid() {
            let id = index.internal_id();
            let was_selected = self.selected_ids.contains(&id);

            if flags.toggle {
                if was_selected {
                    self.selected_ids.remove(&id);
                    self.selected_indices.retain(|idx| idx.internal_id() != id);
                    if !newly_deselected.iter().any(|idx| idx.internal_id() == id) {
                        newly_deselected.push(index.clone());
                    }
                } else {
                    self.add_to_selection(index.clone());
                    newly_selected.push(index.clone());
                }
            } else if flags.select && !was_selected {
                self.add_to_selection(index.clone());
                newly_selected.push(index.clone());
            } else if flags.deselect && was_selected {
                self.selected_ids.remove(&id);
                self.selected_indices.retain(|idx| idx.internal_id() != id);
                if !newly_deselected.iter().any(|idx| idx.internal_id() == id) {
                    newly_deselected.push(index.clone());
                }
            }
        }

        // Enforce single selection mode
        if self.mode == SelectionMode::SingleSelection && self.selected_indices.len() > 1 {
            // Keep only the most recently selected
            let keep = self.selected_indices.pop().unwrap();
            for removed in self.selected_indices.drain(..) {
                if !newly_deselected.iter().any(|idx| idx.internal_id() == removed.internal_id()) {
                    newly_deselected.push(removed.clone());
                }
                self.selected_ids.remove(&removed.internal_id());
            }
            self.selected_indices.push(keep);
        }

        // Remove duplicates from newly_deselected (items that were cleared but then re-selected)
        newly_deselected.retain(|idx| !self.selected_ids.contains(&idx.internal_id()));

        // Emit signal if selection actually changed
        if !newly_selected.is_empty() || !newly_deselected.is_empty() {
            self.selection_changed
                .emit((newly_selected, newly_deselected));
        }
    }

    /// Selects a range of indices from start to end (inclusive).
    ///
    /// This is used for Shift+click behavior in ExtendedSelection mode.
    pub fn select_range(&mut self, start_row: usize, end_row: usize, flags: SelectionFlags) {
        if self.mode == SelectionMode::NoSelection {
            return;
        }

        let (first, last) = if start_row <= end_row {
            (start_row, end_row)
        } else {
            (end_row, start_row)
        };

        let mut newly_selected = Vec::new();
        let mut newly_deselected = Vec::new();

        // Clear if requested
        if flags.clear && !self.selected_indices.is_empty() {
            newly_deselected = std::mem::take(&mut self.selected_indices);
            self.selected_ids.clear();
        }

        // Select the range
        for row in first..=last {
            let index = ModelIndex::new(row, 0, ModelIndex::invalid());
            let id = index.internal_id();

            if !self.selected_ids.contains(&id) {
                self.add_to_selection(index.clone());
                newly_selected.push(index);
            }
        }

        // Remove from deselected any that were re-selected
        newly_deselected.retain(|idx| !self.selected_ids.contains(&idx.internal_id()));

        if !newly_selected.is_empty() || !newly_deselected.is_empty() {
            self.selection_changed
                .emit((newly_selected, newly_deselected));
        }
    }

    /// Selects a 2D rectangular range of cells.
    ///
    /// This is used for Shift+click behavior in TableView with cell selection.
    /// Selects all cells from (start_row, start_col) to (end_row, end_col) inclusive.
    pub fn select_range_2d(
        &mut self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
        flags: SelectionFlags,
    ) {
        if self.mode == SelectionMode::NoSelection {
            return;
        }

        let (first_row, last_row) = if start_row <= end_row {
            (start_row, end_row)
        } else {
            (end_row, start_row)
        };

        let (first_col, last_col) = if start_col <= end_col {
            (start_col, end_col)
        } else {
            (end_col, start_col)
        };

        let mut newly_selected = Vec::new();
        let mut newly_deselected = Vec::new();

        // Clear if requested
        if flags.clear && !self.selected_indices.is_empty() {
            newly_deselected = std::mem::take(&mut self.selected_indices);
            self.selected_ids.clear();
        }

        // Select the rectangular range
        for row in first_row..=last_row {
            for col in first_col..=last_col {
                let index = ModelIndex::new(row, col, ModelIndex::invalid());
                let id = index.internal_id();

                if !self.selected_ids.contains(&id) {
                    self.add_to_selection(index.clone());
                    newly_selected.push(index);
                }
            }
        }

        // Remove from deselected any that were re-selected
        newly_deselected.retain(|idx| !self.selected_ids.contains(&idx.internal_id()));

        if !newly_selected.is_empty() || !newly_deselected.is_empty() {
            self.selection_changed
                .emit((newly_selected, newly_deselected));
        }
    }

    /// Selects an entire column (all cells in that column).
    ///
    /// Requires knowing the row count to select all cells in the column.
    pub fn select_column(&mut self, column: usize, row_count: usize, flags: SelectionFlags) {
        if self.mode == SelectionMode::NoSelection {
            return;
        }

        let mut newly_selected = Vec::new();
        let mut newly_deselected = Vec::new();

        // Clear if requested
        if flags.clear && !self.selected_indices.is_empty() {
            newly_deselected = std::mem::take(&mut self.selected_indices);
            self.selected_ids.clear();
        }

        // Select all cells in the column
        for row in 0..row_count {
            let index = ModelIndex::new(row, column, ModelIndex::invalid());
            let id = index.internal_id();

            if flags.toggle {
                if self.selected_ids.contains(&id) {
                    self.selected_ids.remove(&id);
                    self.selected_indices.retain(|idx| idx.internal_id() != id);
                    if !newly_deselected.iter().any(|idx| idx.internal_id() == id) {
                        newly_deselected.push(index);
                    }
                } else {
                    self.add_to_selection(index.clone());
                    newly_selected.push(index);
                }
            } else if flags.select && !self.selected_ids.contains(&id) {
                self.add_to_selection(index.clone());
                newly_selected.push(index);
            } else if flags.deselect && self.selected_ids.contains(&id) {
                self.selected_ids.remove(&id);
                self.selected_indices.retain(|idx| idx.internal_id() != id);
                if !newly_deselected.iter().any(|idx| idx.internal_id() == id) {
                    newly_deselected.push(index);
                }
            }
        }

        // Remove from deselected any that were re-selected
        newly_deselected.retain(|idx| !self.selected_ids.contains(&idx.internal_id()));

        if !newly_selected.is_empty() || !newly_deselected.is_empty() {
            self.selection_changed
                .emit((newly_selected, newly_deselected));
        }
    }

    /// Selects an entire row (all cells in that row).
    ///
    /// Requires knowing the column count to select all cells in the row.
    pub fn select_row(&mut self, row: usize, column_count: usize, flags: SelectionFlags) {
        if self.mode == SelectionMode::NoSelection {
            return;
        }

        let mut newly_selected = Vec::new();
        let mut newly_deselected = Vec::new();

        // Clear if requested
        if flags.clear && !self.selected_indices.is_empty() {
            newly_deselected = std::mem::take(&mut self.selected_indices);
            self.selected_ids.clear();
        }

        // Select all cells in the row
        for col in 0..column_count {
            let index = ModelIndex::new(row, col, ModelIndex::invalid());
            let id = index.internal_id();

            if flags.toggle {
                if self.selected_ids.contains(&id) {
                    self.selected_ids.remove(&id);
                    self.selected_indices.retain(|idx| idx.internal_id() != id);
                    if !newly_deselected.iter().any(|idx| idx.internal_id() == id) {
                        newly_deselected.push(index);
                    }
                } else {
                    self.add_to_selection(index.clone());
                    newly_selected.push(index);
                }
            } else if flags.select && !self.selected_ids.contains(&id) {
                self.add_to_selection(index.clone());
                newly_selected.push(index);
            } else if flags.deselect && self.selected_ids.contains(&id) {
                self.selected_ids.remove(&id);
                self.selected_indices.retain(|idx| idx.internal_id() != id);
                if !newly_deselected.iter().any(|idx| idx.internal_id() == id) {
                    newly_deselected.push(index);
                }
            }
        }

        // Remove from deselected any that were re-selected
        newly_deselected.retain(|idx| !self.selected_ids.contains(&idx.internal_id()));

        if !newly_selected.is_empty() || !newly_deselected.is_empty() {
            self.selection_changed
                .emit((newly_selected, newly_deselected));
        }
    }

    /// Selects all items (for use with Ctrl+A).
    ///
    /// Requires knowing the row count from the model.
    pub fn select_all(&mut self, row_count: usize) {
        if self.mode == SelectionMode::NoSelection || self.mode == SelectionMode::SingleSelection {
            return;
        }

        let mut newly_selected = Vec::new();

        for row in 0..row_count {
            let index = ModelIndex::new(row, 0, ModelIndex::invalid());
            let id = index.internal_id();

            if !self.selected_ids.contains(&id) {
                self.add_to_selection(index.clone());
                newly_selected.push(index);
            }
        }

        if !newly_selected.is_empty() {
            self.selection_changed.emit((newly_selected, Vec::new()));
        }
    }

    /// Clears all selection.
    pub fn clear_selection(&mut self) {
        if self.selected_indices.is_empty() {
            return;
        }

        let deselected = std::mem::take(&mut self.selected_indices);
        self.selected_ids.clear();
        self.selection_changed.emit((Vec::new(), deselected));
    }

    /// Clears all selection and resets current/anchor.
    pub fn clear(&mut self) {
        self.clear_selection();
        self.current = ModelIndex::invalid();
        self.anchor = ModelIndex::invalid();
    }

    /// Resets the selection model (called when model is reset).
    pub fn reset(&mut self) {
        self.clear();
    }

    // =========================================================================
    // Internal Helpers
    // =========================================================================

    fn add_to_selection(&mut self, index: ModelIndex) {
        let id = index.internal_id();
        if self.selected_ids.insert(id) {
            self.selected_indices.push(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn test_selection_model_creation() {
        let model = SelectionModel::new();
        assert_eq!(model.selection_mode(), SelectionMode::SingleSelection);
        assert!(!model.current_index().is_valid());
        assert!(!model.has_selection());
    }

    #[test]
    fn test_single_selection() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::SingleSelection);

        let idx1 = ModelIndex::new(0, 0, ModelIndex::invalid());
        let idx2 = ModelIndex::new(1, 0, ModelIndex::invalid());

        // Select first item
        model.select(idx1.clone(), SelectionFlags::SELECT);
        assert!(model.is_selected(&idx1));
        assert_eq!(model.selected_count(), 1);

        // Select second item (should replace first in single selection mode)
        model.select(idx2.clone(), SelectionFlags::CLEAR_AND_SELECT);
        assert!(!model.is_selected(&idx1));
        assert!(model.is_selected(&idx2));
        assert_eq!(model.selected_count(), 1);
    }

    #[test]
    fn test_multi_selection() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        let idx1 = ModelIndex::new(0, 0, ModelIndex::invalid());
        let idx2 = ModelIndex::new(1, 0, ModelIndex::invalid());

        model.select(idx1.clone(), SelectionFlags::SELECT);
        model.select(idx2.clone(), SelectionFlags::SELECT);

        assert!(model.is_selected(&idx1));
        assert!(model.is_selected(&idx2));
        assert_eq!(model.selected_count(), 2);
    }

    #[test]
    fn test_toggle_selection() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        let idx = ModelIndex::new(0, 0, ModelIndex::invalid());

        // Toggle on
        model.select(idx.clone(), SelectionFlags::TOGGLE);
        assert!(model.is_selected(&idx));

        // Toggle off
        model.select(idx.clone(), SelectionFlags::TOGGLE);
        assert!(!model.is_selected(&idx));
    }

    #[test]
    fn test_clear_selection() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        model.select(ModelIndex::new(0, 0, ModelIndex::invalid()), SelectionFlags::SELECT);
        model.select(ModelIndex::new(1, 0, ModelIndex::invalid()), SelectionFlags::SELECT);
        assert_eq!(model.selected_count(), 2);

        model.clear_selection();
        assert!(!model.has_selection());
    }

    #[test]
    fn test_no_selection_mode() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::NoSelection);

        model.select(ModelIndex::new(0, 0, ModelIndex::invalid()), SelectionFlags::SELECT);
        assert!(!model.has_selection());
    }

    #[test]
    fn test_range_selection() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::ExtendedSelection);

        model.select_range(2, 5, SelectionFlags::CLEAR_AND_SELECT);
        assert_eq!(model.selected_count(), 4);
        assert!(model.is_row_selected(2));
        assert!(model.is_row_selected(3));
        assert!(model.is_row_selected(4));
        assert!(model.is_row_selected(5));
    }

    #[test]
    fn test_select_all() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::ExtendedSelection);

        model.select_all(10);
        assert_eq!(model.selected_count(), 10);
    }

    #[test]
    fn test_selection_signal() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        let selected_count = Arc::new(AtomicUsize::new(0));
        let count_clone = selected_count.clone();

        model.selection_changed.connect(move |(selected, _)| {
            count_clone.fetch_add(selected.len(), Ordering::SeqCst);
        });

        model.select(ModelIndex::new(0, 0, ModelIndex::invalid()), SelectionFlags::SELECT);
        model.select(ModelIndex::new(1, 0, ModelIndex::invalid()), SelectionFlags::SELECT);

        assert_eq!(selected_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_current_changed_signal() {
        let mut model = SelectionModel::new();

        let changed_count = Arc::new(AtomicUsize::new(0));
        let count_clone = changed_count.clone();

        model.current_changed.connect(move |_| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        model.set_current_index(ModelIndex::new(0, 0, ModelIndex::invalid()), SelectionFlags::CURRENT);
        model.set_current_index(ModelIndex::new(1, 0, ModelIndex::invalid()), SelectionFlags::CURRENT);

        assert_eq!(changed_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_selected_rows() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        model.select(ModelIndex::new(5, 0, ModelIndex::invalid()), SelectionFlags::SELECT);
        model.select(ModelIndex::new(2, 0, ModelIndex::invalid()), SelectionFlags::SELECT);
        model.select(ModelIndex::new(8, 0, ModelIndex::invalid()), SelectionFlags::SELECT);

        let rows = model.selected_rows();
        assert_eq!(rows, vec![2, 5, 8]);
    }

    // =========================================================================
    // SelectionBehavior and 2D Selection Tests
    // =========================================================================

    #[test]
    fn test_selection_behavior_default() {
        let model = SelectionModel::new();
        assert_eq!(model.selection_behavior(), SelectionBehavior::SelectItems);
    }

    #[test]
    fn test_selection_behavior_set() {
        let mut model = SelectionModel::new();
        model.set_selection_behavior(SelectionBehavior::SelectRows);
        assert_eq!(model.selection_behavior(), SelectionBehavior::SelectRows);

        model.set_selection_behavior(SelectionBehavior::SelectColumns);
        assert_eq!(model.selection_behavior(), SelectionBehavior::SelectColumns);
    }

    #[test]
    fn test_select_column() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        // Select column 2 with 5 rows
        model.select_column(2, 5, SelectionFlags::CLEAR_AND_SELECT);

        // All cells in column 2 should be selected
        for row in 0..5 {
            assert!(model.is_cell_selected(row, 2));
        }

        // Cells in other columns should not be selected
        assert!(!model.is_cell_selected(0, 0));
        assert!(!model.is_cell_selected(0, 1));
        assert!(!model.is_cell_selected(0, 3));

        // is_column_selected checks if ANY cell in that column is selected
        assert!(model.is_column_selected(2));
        assert!(!model.is_column_selected(1));
    }

    #[test]
    fn test_select_row() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        // Select row 3 with 4 columns
        model.select_row(3, 4, SelectionFlags::CLEAR_AND_SELECT);

        // All cells in row 3 should be selected
        for col in 0..4 {
            assert!(model.is_cell_selected(3, col));
        }

        // Cells in other rows should not be selected
        assert!(!model.is_cell_selected(0, 0));
        assert!(!model.is_cell_selected(2, 0));

        assert!(model.is_row_selected(3));
        assert!(!model.is_row_selected(2));
    }

    #[test]
    fn test_select_range_2d() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        // Select a 3x2 region from (1, 2) to (3, 3)
        model.select_range_2d(1, 2, 3, 3, SelectionFlags::CLEAR_AND_SELECT);

        // Selected cells
        assert!(model.is_cell_selected(1, 2));
        assert!(model.is_cell_selected(1, 3));
        assert!(model.is_cell_selected(2, 2));
        assert!(model.is_cell_selected(2, 3));
        assert!(model.is_cell_selected(3, 2));
        assert!(model.is_cell_selected(3, 3));

        // Not selected
        assert!(!model.is_cell_selected(0, 2));
        assert!(!model.is_cell_selected(1, 1));
        assert!(!model.is_cell_selected(4, 2));
    }

    #[test]
    fn test_selected_columns() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        // Select full columns 1 and 3 (with 5 rows)
        model.select_column(1, 5, SelectionFlags::SELECT);
        model.select_column(3, 5, SelectionFlags::SELECT);

        let cols = model.selected_columns();
        assert_eq!(cols, vec![1, 3]);
    }

    #[test]
    fn test_selected_cells() {
        let mut model = SelectionModel::new();
        model.set_selection_mode(SelectionMode::MultiSelection);

        model.select(ModelIndex::new(1, 2, ModelIndex::invalid()), SelectionFlags::SELECT);
        model.select(ModelIndex::new(3, 4, ModelIndex::invalid()), SelectionFlags::SELECT);

        let cells = model.selected_cells();
        assert_eq!(cells.len(), 2);
        assert!(cells.contains(&(1, 2)));
        assert!(cells.contains(&(3, 4)));
    }
}
