//! Grid layout for arranging widgets in a row/column grid.
//!
//! `GridLayout` arranges items in a two-dimensional grid with configurable
//! row/column stretch factors, cell spanning, and independent horizontal/vertical
//! spacing.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::layout::*;
//!
//! // Create a grid layout
//! let mut layout = GridLayout::new();
//!
//! // Add widgets at specific positions
//! layout.add_widget_at(label_id, 0, 0);           // Row 0, Col 0
//! layout.add_widget_at(input_id, 0, 1);           // Row 0, Col 1
//! layout.add_widget_spanning(button_id, 1, 0, 1, 2); // Row 1, spans 2 columns
//!
//! // Configure stretch factors
//! layout.set_column_stretch(1, 1); // Column 1 gets extra space
//! layout.set_row_stretch(0, 0);    // Row 0 stays at preferred size
//! ```

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

use super::base::LayoutBase;
use super::box_layout::Alignment;
use super::item::LayoutItem;
use super::traits::Layout;
use super::ContentMargins;
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::geometry::{SizeHint, SizePolicy, SizePolicyPair};

/// Information about an item placed in the grid.
#[derive(Debug, Clone)]
struct GridCell {
    /// The layout item in this cell.
    item: LayoutItem,
    /// The starting row.
    row: usize,
    /// The starting column.
    col: usize,
    /// Number of rows this item spans.
    row_span: usize,
    /// Number of columns this item spans.
    col_span: usize,
    /// Alignment within the cell.
    alignment: CellAlignment,
}

impl GridCell {
    fn new(item: LayoutItem, row: usize, col: usize) -> Self {
        Self {
            item,
            row,
            col,
            row_span: 1,
            col_span: 1,
            alignment: CellAlignment::default(),
        }
    }

    fn with_span(mut self, row_span: usize, col_span: usize) -> Self {
        self.row_span = row_span.max(1);
        self.col_span = col_span.max(1);
        self
    }

    fn with_alignment(mut self, alignment: CellAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Check if this cell occupies the given position.
    fn occupies(&self, row: usize, col: usize) -> bool {
        row >= self.row
            && row < self.row + self.row_span
            && col >= self.col
            && col < self.col + self.col_span
    }
}

/// Alignment within a grid cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellAlignment {
    /// Horizontal alignment.
    pub horizontal: Alignment,
    /// Vertical alignment.
    pub vertical: Alignment,
}

impl CellAlignment {
    /// Create a new cell alignment.
    pub fn new(horizontal: Alignment, vertical: Alignment) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }

    /// Fill the entire cell (default).
    pub fn fill() -> Self {
        Self::new(Alignment::Stretch, Alignment::Stretch)
    }

    /// Center in both directions.
    pub fn center() -> Self {
        Self::new(Alignment::Center, Alignment::Center)
    }

    /// Align to top-left.
    pub fn top_left() -> Self {
        Self::new(Alignment::Start, Alignment::Start)
    }

    /// Align to top-right.
    pub fn top_right() -> Self {
        Self::new(Alignment::End, Alignment::Start)
    }

    /// Align to bottom-left.
    pub fn bottom_left() -> Self {
        Self::new(Alignment::Start, Alignment::End)
    }

    /// Align to bottom-right.
    pub fn bottom_right() -> Self {
        Self::new(Alignment::End, Alignment::End)
    }
}

/// A grid layout that arranges items in rows and columns.
///
/// `GridLayout` provides a two-dimensional layout where items can be placed
/// at specific row/column positions and optionally span multiple rows or columns.
///
/// # Features
///
/// - Arbitrary row/column placement
/// - Cell spanning (rowspan, colspan)
/// - Per-row and per-column stretch factors
/// - Minimum row heights and column widths
/// - Independent horizontal and vertical spacing
/// - Per-cell alignment (fill, center, corners)
///
/// # Layout Algorithm
///
/// 1. Determine grid dimensions from placed items
/// 2. Calculate column widths (collect hints, distribute horizontal space)
/// 3. Calculate row heights (collect hints, distribute vertical space)
/// 4. Position each item within its cell bounds based on alignment
#[derive(Debug, Clone)]
pub struct GridLayout {
    /// Common layout base functionality.
    base: LayoutBase,
    /// Items placed in the grid with their positions and spans.
    cells: Vec<GridCell>,
    /// Number of rows (calculated from placed items).
    row_count: usize,
    /// Number of columns (calculated from placed items).
    col_count: usize,
    /// Stretch factors for each row (0 = no extra space, >0 = proportional).
    row_stretch: Vec<u8>,
    /// Stretch factors for each column.
    col_stretch: Vec<u8>,
    /// Minimum height for each row.
    row_min_height: Vec<f32>,
    /// Minimum width for each column.
    col_min_width: Vec<f32>,
    /// Horizontal spacing between columns.
    horizontal_spacing: f32,
    /// Vertical spacing between rows.
    vertical_spacing: f32,
    /// Calculated column widths (after calculate()).
    col_widths: Vec<f32>,
    /// Calculated row heights (after calculate()).
    row_heights: Vec<f32>,
    /// Calculated column positions (x coordinate of each column start).
    col_positions: Vec<f32>,
    /// Calculated row positions (y coordinate of each row start).
    row_positions: Vec<f32>,
}

impl GridLayout {
    /// Create a new empty grid layout.
    pub fn new() -> Self {
        Self {
            base: LayoutBase::new(),
            cells: Vec::new(),
            row_count: 0,
            col_count: 0,
            row_stretch: Vec::new(),
            col_stretch: Vec::new(),
            row_min_height: Vec::new(),
            col_min_width: Vec::new(),
            horizontal_spacing: super::DEFAULT_SPACING,
            vertical_spacing: super::DEFAULT_SPACING,
            col_widths: Vec::new(),
            row_heights: Vec::new(),
            col_positions: Vec::new(),
            row_positions: Vec::new(),
        }
    }

    // =========================================================================
    // Grid-Specific Item Addition
    // =========================================================================

    /// Add a widget at the specified row and column.
    pub fn add_widget_at(&mut self, widget: ObjectId, row: usize, col: usize) {
        self.add_item_at(LayoutItem::Widget(widget), row, col);
    }

    /// Add a widget spanning multiple rows and/or columns.
    pub fn add_widget_spanning(
        &mut self,
        widget: ObjectId,
        row: usize,
        col: usize,
        row_span: usize,
        col_span: usize,
    ) {
        self.add_item_spanning(LayoutItem::Widget(widget), row, col, row_span, col_span);
    }

    /// Add a widget at the specified position with alignment.
    pub fn add_widget_aligned(
        &mut self,
        widget: ObjectId,
        row: usize,
        col: usize,
        alignment: CellAlignment,
    ) {
        self.add_item_aligned(LayoutItem::Widget(widget), row, col, alignment);
    }

    /// Add a widget spanning with alignment.
    pub fn add_widget_spanning_aligned(
        &mut self,
        widget: ObjectId,
        row: usize,
        col: usize,
        row_span: usize,
        col_span: usize,
        alignment: CellAlignment,
    ) {
        self.add_item_spanning_aligned(
            LayoutItem::Widget(widget),
            row,
            col,
            row_span,
            col_span,
            alignment,
        );
    }

    /// Add a layout item at the specified row and column.
    pub fn add_item_at(&mut self, item: LayoutItem, row: usize, col: usize) {
        let cell = GridCell::new(item, row, col);
        self.insert_cell(cell);
    }

    /// Add a layout item spanning multiple rows and/or columns.
    pub fn add_item_spanning(
        &mut self,
        item: LayoutItem,
        row: usize,
        col: usize,
        row_span: usize,
        col_span: usize,
    ) {
        let cell = GridCell::new(item, row, col).with_span(row_span, col_span);
        self.insert_cell(cell);
    }

    /// Add a layout item with alignment.
    pub fn add_item_aligned(
        &mut self,
        item: LayoutItem,
        row: usize,
        col: usize,
        alignment: CellAlignment,
    ) {
        let cell = GridCell::new(item, row, col).with_alignment(alignment);
        self.insert_cell(cell);
    }

    /// Add a layout item spanning with alignment.
    pub fn add_item_spanning_aligned(
        &mut self,
        item: LayoutItem,
        row: usize,
        col: usize,
        row_span: usize,
        col_span: usize,
        alignment: CellAlignment,
    ) {
        let cell = GridCell::new(item, row, col)
            .with_span(row_span, col_span)
            .with_alignment(alignment);
        self.insert_cell(cell);
    }

    /// Internal method to insert a cell and update grid dimensions.
    fn insert_cell(&mut self, cell: GridCell) {
        // Update grid dimensions
        let new_row_count = (cell.row + cell.row_span).max(self.row_count);
        let new_col_count = (cell.col + cell.col_span).max(self.col_count);

        // Expand row/column arrays if needed
        if new_row_count > self.row_count {
            self.row_stretch.resize(new_row_count, 0);
            self.row_min_height.resize(new_row_count, 0.0);
            self.row_count = new_row_count;
        }
        if new_col_count > self.col_count {
            self.col_stretch.resize(new_col_count, 0);
            self.col_min_width.resize(new_col_count, 0.0);
            self.col_count = new_col_count;
        }

        // Also add to base for basic item management
        self.base.add_item(cell.item.clone());
        self.cells.push(cell);
        self.base.invalidate();
    }

    // =========================================================================
    // Row/Column Configuration
    // =========================================================================

    /// Set the stretch factor for a row.
    ///
    /// Rows with higher stretch factors receive more extra space proportionally.
    /// A stretch of 0 means the row only gets its minimum/preferred size.
    pub fn set_row_stretch(&mut self, row: usize, stretch: u8) {
        self.ensure_row(row);
        if self.row_stretch[row] != stretch {
            self.row_stretch[row] = stretch;
            self.base.invalidate();
        }
    }

    /// Get the stretch factor for a row.
    pub fn row_stretch(&self, row: usize) -> u8 {
        self.row_stretch.get(row).copied().unwrap_or(0)
    }

    /// Set the stretch factor for a column.
    pub fn set_column_stretch(&mut self, col: usize, stretch: u8) {
        self.ensure_column(col);
        if self.col_stretch[col] != stretch {
            self.col_stretch[col] = stretch;
            self.base.invalidate();
        }
    }

    /// Get the stretch factor for a column.
    pub fn column_stretch(&self, col: usize) -> u8 {
        self.col_stretch.get(col).copied().unwrap_or(0)
    }

    /// Set the minimum height for a row.
    pub fn set_row_minimum_height(&mut self, row: usize, min_height: f32) {
        self.ensure_row(row);
        if (self.row_min_height[row] - min_height).abs() > f32::EPSILON {
            self.row_min_height[row] = min_height;
            self.base.invalidate();
        }
    }

    /// Get the minimum height for a row.
    pub fn row_minimum_height(&self, row: usize) -> f32 {
        self.row_min_height.get(row).copied().unwrap_or(0.0)
    }

    /// Set the minimum width for a column.
    pub fn set_column_minimum_width(&mut self, col: usize, min_width: f32) {
        self.ensure_column(col);
        if (self.col_min_width[col] - min_width).abs() > f32::EPSILON {
            self.col_min_width[col] = min_width;
            self.base.invalidate();
        }
    }

    /// Get the minimum width for a column.
    pub fn column_minimum_width(&self, col: usize) -> f32 {
        self.col_min_width.get(col).copied().unwrap_or(0.0)
    }

    /// Ensure row arrays are large enough.
    fn ensure_row(&mut self, row: usize) {
        if row >= self.row_count {
            let new_count = row + 1;
            self.row_stretch.resize(new_count, 0);
            self.row_min_height.resize(new_count, 0.0);
            self.row_count = new_count;
        }
    }

    /// Ensure column arrays are large enough.
    fn ensure_column(&mut self, col: usize) {
        if col >= self.col_count {
            let new_count = col + 1;
            self.col_stretch.resize(new_count, 0);
            self.col_min_width.resize(new_count, 0.0);
            self.col_count = new_count;
        }
    }

    // =========================================================================
    // Spacing
    // =========================================================================

    /// Get horizontal spacing between columns.
    #[inline]
    pub fn horizontal_spacing(&self) -> f32 {
        self.horizontal_spacing
    }

    /// Set horizontal spacing between columns.
    pub fn set_horizontal_spacing(&mut self, spacing: f32) {
        if (self.horizontal_spacing - spacing).abs() > f32::EPSILON {
            self.horizontal_spacing = spacing;
            self.base.invalidate();
        }
    }

    /// Get vertical spacing between rows.
    #[inline]
    pub fn vertical_spacing(&self) -> f32 {
        self.vertical_spacing
    }

    /// Set vertical spacing between rows.
    pub fn set_vertical_spacing(&mut self, spacing: f32) {
        if (self.vertical_spacing - spacing).abs() > f32::EPSILON {
            self.vertical_spacing = spacing;
            self.base.invalidate();
        }
    }

    // =========================================================================
    // Grid Inspection
    // =========================================================================

    /// Get the number of rows in the grid.
    #[inline]
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Get the number of columns in the grid.
    #[inline]
    pub fn column_count(&self) -> usize {
        self.col_count
    }

    /// Get the item at a specific row and column.
    ///
    /// Returns the first item that occupies the given cell position.
    pub fn item_at_position(&self, row: usize, col: usize) -> Option<&LayoutItem> {
        self.cells
            .iter()
            .find(|cell| cell.occupies(row, col))
            .map(|cell| &cell.item)
    }

    /// Get the rectangle for a specific cell (after layout calculation).
    ///
    /// Returns the bounds of the cell at (row, col), or None if the
    /// position is out of bounds or layout hasn't been calculated.
    pub fn cell_rect(&self, row: usize, col: usize) -> Option<Rect> {
        if row >= self.row_count || col >= self.col_count {
            return None;
        }
        if self.col_positions.is_empty() || self.row_positions.is_empty() {
            return None;
        }

        let x = self.col_positions.get(col).copied()?;
        let y = self.row_positions.get(row).copied()?;
        let width = self.col_widths.get(col).copied()?;
        let height = self.row_heights.get(row).copied()?;

        Some(Rect::new(x, y, width, height))
    }

    /// Get the rectangle spanning multiple cells (after layout calculation).
    pub fn cell_rect_spanning(
        &self,
        row: usize,
        col: usize,
        row_span: usize,
        col_span: usize,
    ) -> Option<Rect> {
        if row >= self.row_count || col >= self.col_count {
            return None;
        }
        if self.col_positions.is_empty() || self.row_positions.is_empty() {
            return None;
        }

        let x = self.col_positions.get(col).copied()?;
        let y = self.row_positions.get(row).copied()?;

        // Calculate total width including spacing
        let end_col = (col + col_span).min(self.col_count);
        let mut width: f32 = 0.0;
        for c in col..end_col {
            width += self.col_widths.get(c).copied().unwrap_or(0.0);
            if c < end_col - 1 {
                width += self.horizontal_spacing;
            }
        }

        // Calculate total height including spacing
        let end_row = (row + row_span).min(self.row_count);
        let mut height: f32 = 0.0;
        for r in row..end_row {
            height += self.row_heights.get(r).copied().unwrap_or(0.0);
            if r < end_row - 1 {
                height += self.vertical_spacing;
            }
        }

        Some(Rect::new(x, y, width, height))
    }

    // =========================================================================
    // Size Calculation Helpers
    // =========================================================================

    /// Calculate size hints for all columns.
    fn calculate_column_hints<S: WidgetAccess>(&self, storage: &S) -> Vec<(f32, f32, f32)> {
        let mut col_hints: Vec<(f32, f32, f32)> = vec![(0.0, 0.0, f32::MAX); self.col_count];

        for cell in &self.cells {
            if !self.base.is_item_visible(storage, &cell.item) {
                continue;
            }

            let hint = self.base.get_item_size_hint(storage, &cell.item);

            // For non-spanning items, directly update the column
            if cell.col_span == 1 {
                let col = cell.col;
                let pref = hint.preferred.width;
                let min = hint.effective_minimum().width.max(self.col_min_width[col]);
                let max = hint.effective_maximum().width;

                col_hints[col].0 = col_hints[col].0.max(pref);
                col_hints[col].1 = col_hints[col].1.max(min);
                col_hints[col].2 = col_hints[col].2.min(max);
            }
            // For spanning items, distribute hint across columns
            // This is a simplified approach - full implementation would be iterative
        }

        // Apply minimum width constraints
        for (col, hints) in col_hints.iter_mut().enumerate() {
            hints.1 = hints.1.max(self.col_min_width.get(col).copied().unwrap_or(0.0));
            hints.0 = hints.0.max(hints.1); // preferred >= minimum
        }

        col_hints
    }

    /// Calculate size hints for all rows.
    fn calculate_row_hints<S: WidgetAccess>(&self, storage: &S) -> Vec<(f32, f32, f32)> {
        let mut row_hints: Vec<(f32, f32, f32)> = vec![(0.0, 0.0, f32::MAX); self.row_count];

        for cell in &self.cells {
            if !self.base.is_item_visible(storage, &cell.item) {
                continue;
            }

            let hint = self.base.get_item_size_hint(storage, &cell.item);

            // For non-spanning items, directly update the row
            if cell.row_span == 1 {
                let row = cell.row;
                let pref = hint.preferred.height;
                let min = hint.effective_minimum().height.max(self.row_min_height[row]);
                let max = hint.effective_maximum().height;

                row_hints[row].0 = row_hints[row].0.max(pref);
                row_hints[row].1 = row_hints[row].1.max(min);
                row_hints[row].2 = row_hints[row].2.min(max);
            }
        }

        // Apply minimum height constraints
        for (row, hints) in row_hints.iter_mut().enumerate() {
            hints.1 = hints.1.max(self.row_min_height.get(row).copied().unwrap_or(0.0));
            hints.0 = hints.0.max(hints.1); // preferred >= minimum
        }

        row_hints
    }

    /// Calculate the aggregate size hint for the grid.
    fn calculate_size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        if self.row_count == 0 || self.col_count == 0 {
            return SizeHint::default();
        }

        let col_hints = self.calculate_column_hints(storage);
        let row_hints = self.calculate_row_hints(storage);

        // Sum column widths
        let mut total_width_pref: f32 = 0.0;
        let mut total_width_min: f32 = 0.0;
        let mut total_width_max: f32 = 0.0;

        for (pref, min, max) in &col_hints {
            total_width_pref += pref;
            total_width_min += min;
            if *max < f32::MAX - total_width_max {
                total_width_max += max;
            } else {
                total_width_max = f32::MAX;
            }
        }

        // Sum row heights
        let mut total_height_pref: f32 = 0.0;
        let mut total_height_min: f32 = 0.0;
        let mut total_height_max: f32 = 0.0;

        for (pref, min, max) in &row_hints {
            total_height_pref += pref;
            total_height_min += min;
            if *max < f32::MAX - total_height_max {
                total_height_max += max;
            } else {
                total_height_max = f32::MAX;
            }
        }

        // Add spacing
        let h_spacing = self.horizontal_spacing * (self.col_count.saturating_sub(1)) as f32;
        let v_spacing = self.vertical_spacing * (self.row_count.saturating_sub(1)) as f32;

        total_width_pref += h_spacing;
        total_width_min += h_spacing;
        if total_width_max < f32::MAX - h_spacing {
            total_width_max += h_spacing;
        }

        total_height_pref += v_spacing;
        total_height_min += v_spacing;
        if total_height_max < f32::MAX - v_spacing {
            total_height_max += v_spacing;
        }

        // Add margins
        let margins = self.base.content_margins();
        total_width_pref += margins.horizontal();
        total_width_min += margins.horizontal();
        if total_width_max < f32::MAX - margins.horizontal() {
            total_width_max += margins.horizontal();
        }

        total_height_pref += margins.vertical();
        total_height_min += margins.vertical();
        if total_height_max < f32::MAX - margins.vertical() {
            total_height_max += margins.vertical();
        }

        SizeHint {
            preferred: Size::new(total_width_pref, total_height_pref),
            minimum: Some(Size::new(total_width_min, total_height_min)),
            maximum: if total_width_max < f32::MAX && total_height_max < f32::MAX {
                Some(Size::new(total_width_max, total_height_max))
            } else {
                None
            },
        }
    }

    /// Distribute space among columns.
    fn distribute_column_space(&self, col_hints: &[(f32, f32, f32)], available: f32) -> Vec<f32> {
        let n = col_hints.len();
        if n == 0 {
            return Vec::new();
        }

        // Start with preferred widths
        let mut widths: Vec<f32> = col_hints.iter().map(|(pref, _, _)| *pref).collect();

        let total_pref: f32 = widths.iter().sum();
        let extra = available - total_pref;

        if extra > 0.0 {
            // Distribute extra space based on stretch factors
            let total_stretch: u32 = self.col_stretch.iter().map(|&s| s as u32).sum();

            if total_stretch == 0 {
                // No stretch factors - columns stay at preferred size
            } else {
                // Distribute by stretch factor
                for (col, width) in widths.iter_mut().enumerate() {
                    let stretch = self.col_stretch.get(col).copied().unwrap_or(0);
                    if stretch > 0 {
                        let share = extra * (stretch as f32 / total_stretch as f32);
                        let max = col_hints[col].2;
                        *width = (*width + share).min(max);
                    }
                }
            }
        } else if extra < 0.0 {
            // Need to shrink - shrink proportionally to shrink room
            let mut shrinkable: Vec<(usize, f32)> = Vec::new();
            for (col, (pref, min, _)) in col_hints.iter().enumerate() {
                let shrink_room = (pref - min).max(0.0);
                if shrink_room > 0.0 {
                    shrinkable.push((col, shrink_room));
                }
            }

            if !shrinkable.is_empty() {
                let total_shrink: f32 = shrinkable.iter().map(|(_, r)| *r).sum();
                let deficit = (-extra).min(total_shrink);

                for (col, shrink_room) in shrinkable {
                    let share = deficit * (shrink_room / total_shrink);
                    widths[col] = (widths[col] - share).max(col_hints[col].1);
                }
            }
        }

        // Ensure minimums
        for (col, width) in widths.iter_mut().enumerate() {
            *width = width.max(col_hints[col].1);
        }

        widths
    }

    /// Distribute space among rows.
    fn distribute_row_space(&self, row_hints: &[(f32, f32, f32)], available: f32) -> Vec<f32> {
        let n = row_hints.len();
        if n == 0 {
            return Vec::new();
        }

        // Start with preferred heights
        let mut heights: Vec<f32> = row_hints.iter().map(|(pref, _, _)| *pref).collect();

        let total_pref: f32 = heights.iter().sum();
        let extra = available - total_pref;

        if extra > 0.0 {
            let total_stretch: u32 = self.row_stretch.iter().map(|&s| s as u32).sum();

            if total_stretch == 0 {
                // No stretch factors - rows stay at preferred size
            } else {
                for (row, height) in heights.iter_mut().enumerate() {
                    let stretch = self.row_stretch.get(row).copied().unwrap_or(0);
                    if stretch > 0 {
                        let share = extra * (stretch as f32 / total_stretch as f32);
                        let max = row_hints[row].2;
                        *height = (*height + share).min(max);
                    }
                }
            }
        } else if extra < 0.0 {
            let mut shrinkable: Vec<(usize, f32)> = Vec::new();
            for (row, (pref, min, _)) in row_hints.iter().enumerate() {
                let shrink_room = (pref - min).max(0.0);
                if shrink_room > 0.0 {
                    shrinkable.push((row, shrink_room));
                }
            }

            if !shrinkable.is_empty() {
                let total_shrink: f32 = shrinkable.iter().map(|(_, r)| *r).sum();
                let deficit = (-extra).min(total_shrink);

                for (row, shrink_room) in shrinkable {
                    let share = deficit * (shrink_room / total_shrink);
                    heights[row] = (heights[row] - share).max(row_hints[row].1);
                }
            }
        }

        // Ensure minimums
        for (row, height) in heights.iter_mut().enumerate() {
            *height = height.max(row_hints[row].1);
        }

        heights
    }

    /// Find the next empty cell in the grid.
    fn find_next_empty_cell(&self) -> (usize, usize) {
        let mut row = 0;
        loop {
            for c in 0..self.col_count.max(1) {
                if self.item_at_position(row, c).is_none() {
                    return (row, c);
                }
            }
            row += 1;
            if row > 100 {
                // Safety limit - add to a new row
                return (row, 0);
            }
        }
    }

    /// Calculate positions from sizes.
    fn calculate_positions(sizes: &[f32], spacing: f32, start: f32) -> Vec<f32> {
        let mut positions = Vec::with_capacity(sizes.len());
        let mut pos = start;
        for (i, &size) in sizes.iter().enumerate() {
            positions.push(pos);
            pos += size;
            if i < sizes.len() - 1 {
                pos += spacing;
            }
        }
        positions
    }

    /// Apply alignment within a cell.
    fn apply_cell_alignment(
        &self,
        cell_rect: Rect,
        item_hint: SizeHint,
        alignment: CellAlignment,
    ) -> Rect {
        let pref = item_hint.preferred;

        // Calculate aligned position and size for horizontal
        let (x, width) = match alignment.horizontal {
            Alignment::Stretch => (cell_rect.origin.x, cell_rect.width()),
            Alignment::Start => {
                let w = pref.width.min(cell_rect.width());
                (cell_rect.origin.x, w)
            }
            Alignment::Center => {
                let w = pref.width.min(cell_rect.width());
                let x = cell_rect.origin.x + (cell_rect.width() - w) / 2.0;
                (x, w)
            }
            Alignment::End => {
                let w = pref.width.min(cell_rect.width());
                let x = cell_rect.origin.x + cell_rect.width() - w;
                (x, w)
            }
        };

        // Calculate aligned position and size for vertical
        let (y, height) = match alignment.vertical {
            Alignment::Stretch => (cell_rect.origin.y, cell_rect.height()),
            Alignment::Start => {
                let h = pref.height.min(cell_rect.height());
                (cell_rect.origin.y, h)
            }
            Alignment::Center => {
                let h = pref.height.min(cell_rect.height());
                let y = cell_rect.origin.y + (cell_rect.height() - h) / 2.0;
                (y, h)
            }
            Alignment::End => {
                let h = pref.height.min(cell_rect.height());
                let y = cell_rect.origin.y + cell_rect.height() - h;
                (y, h)
            }
        };

        Rect::new(x, y, width, height)
    }
}

impl Default for GridLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for GridLayout {
    // =========================================================================
    // Item Management
    // =========================================================================

    fn add_item(&mut self, item: LayoutItem) {
        // Add to the next available position (find first empty cell)
        // This is a fallback - prefer using add_item_at for grids
        let (row, col) = self.find_next_empty_cell();
        self.add_item_at(item, row, col);
    }

    fn insert_item(&mut self, _index: usize, item: LayoutItem) {
        // For grid layouts, index doesn't map directly to position
        // Just add the item
        self.add_item(item);
    }

    fn remove_item(&mut self, index: usize) -> Option<LayoutItem> {
        if index < self.cells.len() {
            let cell = self.cells.remove(index);
            self.base.remove_item(index);
            self.base.invalidate();
            Some(cell.item)
        } else {
            None
        }
    }

    fn remove_widget(&mut self, widget: ObjectId) -> bool {
        if let Some(index) = self.cells.iter().position(|cell| {
            matches!(&cell.item, LayoutItem::Widget(id) if *id == widget)
        }) {
            self.cells.remove(index);
            self.base.remove_widget(widget);
            self.base.invalidate();
            true
        } else {
            false
        }
    }

    fn item_count(&self) -> usize {
        self.cells.len()
    }

    fn item_at(&self, index: usize) -> Option<&LayoutItem> {
        self.cells.get(index).map(|c| &c.item)
    }

    fn item_at_mut(&mut self, index: usize) -> Option<&mut LayoutItem> {
        self.base.invalidate();
        self.cells.get_mut(index).map(|c| &mut c.item)
    }

    fn clear(&mut self) {
        self.cells.clear();
        self.base.clear();
        self.row_count = 0;
        self.col_count = 0;
        self.row_stretch.clear();
        self.col_stretch.clear();
        self.row_min_height.clear();
        self.col_min_width.clear();
        self.col_widths.clear();
        self.row_heights.clear();
        self.col_positions.clear();
        self.row_positions.clear();
    }

    // =========================================================================
    // Size Hints & Policies
    // =========================================================================

    fn size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        if let Some(cached) = self.base.cached_size_hint() {
            return cached;
        }
        self.calculate_size_hint(storage)
    }

    fn minimum_size<S: WidgetAccess>(&self, storage: &S) -> Size {
        if let Some(cached) = self.base.cached_minimum_size() {
            return cached;
        }
        self.size_hint(storage).effective_minimum()
    }

    fn size_policy(&self) -> SizePolicyPair {
        SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred)
    }

    // =========================================================================
    // Geometry & Margins
    // =========================================================================

    fn geometry(&self) -> Rect {
        self.base.geometry()
    }

    fn set_geometry(&mut self, rect: Rect) {
        self.base.set_geometry(rect);
    }

    fn content_margins(&self) -> ContentMargins {
        self.base.content_margins()
    }

    fn set_content_margins(&mut self, margins: ContentMargins) {
        self.base.set_content_margins(margins);
    }

    fn spacing(&self) -> f32 {
        // Return horizontal spacing as the "default" spacing
        self.horizontal_spacing
    }

    fn set_spacing(&mut self, spacing: f32) {
        // Set both horizontal and vertical spacing
        self.set_horizontal_spacing(spacing);
        self.set_vertical_spacing(spacing);
    }

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    fn calculate<S: WidgetAccess>(&mut self, storage: &S, _available: Size) -> Size {
        if self.row_count == 0 || self.col_count == 0 {
            self.base.mark_valid();
            return Size::ZERO;
        }

        let content_rect = self.base.content_rect();

        // Calculate spacing totals
        let h_spacing_total = self.horizontal_spacing * (self.col_count.saturating_sub(1)) as f32;
        let v_spacing_total = self.vertical_spacing * (self.row_count.saturating_sub(1)) as f32;

        let available_width = (content_rect.width() - h_spacing_total).max(0.0);
        let available_height = (content_rect.height() - v_spacing_total).max(0.0);

        // Calculate column and row hints
        let col_hints = self.calculate_column_hints(storage);
        let row_hints = self.calculate_row_hints(storage);

        // Distribute space
        self.col_widths = self.distribute_column_space(&col_hints, available_width);
        self.row_heights = self.distribute_row_space(&row_hints, available_height);

        // Calculate positions
        self.col_positions = Self::calculate_positions(
            &self.col_widths,
            self.horizontal_spacing,
            content_rect.origin.x,
        );
        self.row_positions = Self::calculate_positions(
            &self.row_heights,
            self.vertical_spacing,
            content_rect.origin.y,
        );

        // Calculate geometry for each cell item
        for (idx, cell) in self.cells.iter().enumerate() {
            if !self.base.is_item_visible(storage, &cell.item) {
                continue;
            }

            // Get the cell bounds (possibly spanning)
            if let Some(cell_rect) =
                self.cell_rect_spanning(cell.row, cell.col, cell.row_span, cell.col_span)
            {
                // Apply alignment
                let item_hint = self.base.get_item_size_hint(storage, &cell.item);
                let aligned_rect = self.apply_cell_alignment(cell_rect, item_hint, cell.alignment);
                self.base.set_item_geometry(idx, aligned_rect);
            }
        }

        // Cache size hint
        let size_hint = self.calculate_size_hint(storage);
        self.base.set_cached_size_hint(size_hint);
        self.base.set_cached_minimum_size(size_hint.effective_minimum());

        self.base.mark_valid();

        Size::new(
            content_rect.width() + self.base.content_margins().horizontal(),
            content_rect.height() + self.base.content_margins().vertical(),
        )
    }

    fn apply<S: WidgetAccess>(&self, storage: &mut S) {
        for (idx, cell) in self.cells.iter().enumerate() {
            if let Some(geometry) = self.base.item_geometry(idx) {
                LayoutBase::apply_item_geometry(storage, &cell.item, geometry);
            }
        }
    }

    // =========================================================================
    // Invalidation
    // =========================================================================

    fn invalidate(&mut self) {
        self.base.invalidate();
    }

    fn needs_recalculation(&self) -> bool {
        self.base.needs_recalculation()
    }

    // =========================================================================
    // Ownership
    // =========================================================================

    fn parent_widget(&self) -> Option<ObjectId> {
        self.base.parent_widget()
    }

    fn set_parent_widget(&mut self, parent: Option<ObjectId>) {
        self.base.set_parent_widget(parent);
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::base::WidgetBase;
    use crate::widget::traits::{PaintContext, Widget};
    use horizon_lattice_core::{init_global_registry, Object, ObjectId};
    use std::collections::HashMap;

    /// Mock widget for testing layouts.
    struct MockWidget {
        base: WidgetBase,
        mock_size_hint: SizeHint,
    }

    impl MockWidget {
        fn new(size_hint: SizeHint) -> Self {
            Self {
                base: WidgetBase::new::<Self>(),
                mock_size_hint: size_hint,
            }
        }
    }

    impl Object for MockWidget {
        fn object_id(&self) -> ObjectId {
            self.base.object_id()
        }
    }

    impl Widget for MockWidget {
        fn widget_base(&self) -> &WidgetBase {
            &self.base
        }

        fn widget_base_mut(&mut self) -> &mut WidgetBase {
            &mut self.base
        }

        fn size_hint(&self) -> SizeHint {
            self.mock_size_hint
        }

        fn paint(&self, _ctx: &mut PaintContext<'_>) {}
    }

    /// Mock widget storage for testing.
    struct MockStorage {
        widgets: HashMap<ObjectId, MockWidget>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                widgets: HashMap::new(),
            }
        }

        fn add(&mut self, widget: MockWidget) -> ObjectId {
            let id = widget.object_id();
            self.widgets.insert(id, widget);
            id
        }
    }

    impl WidgetAccess for MockStorage {
        fn get_widget(&self, id: ObjectId) -> Option<&dyn Widget> {
            self.widgets.get(&id).map(|w| w as &dyn Widget)
        }

        fn get_widget_mut(&mut self, id: ObjectId) -> Option<&mut dyn Widget> {
            self.widgets.get_mut(&id).map(|w| w as &mut dyn Widget)
        }
    }

    #[test]
    fn test_grid_layout_creation() {
        init_global_registry();

        let grid = GridLayout::new();
        assert_eq!(grid.row_count(), 0);
        assert_eq!(grid.column_count(), 0);
        assert_eq!(grid.item_count(), 0);
    }

    #[test]
    fn test_grid_layout_add_widgets() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 30.0))));
        let id3 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 40.0))));

        let mut grid = GridLayout::new();
        grid.add_widget_at(id1, 0, 0);
        grid.add_widget_at(id2, 0, 1);
        grid.add_widget_at(id3, 1, 0);

        assert_eq!(grid.row_count(), 2);
        assert_eq!(grid.column_count(), 2);
        assert_eq!(grid.item_count(), 3);
    }

    #[test]
    fn test_grid_layout_spanning() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(200.0, 30.0))));

        let mut grid = GridLayout::new();
        grid.add_widget_at(id1, 0, 0);
        grid.add_widget_spanning(id2, 1, 0, 1, 2); // Span 2 columns

        assert_eq!(grid.row_count(), 2);
        assert_eq!(grid.column_count(), 2);

        // id2 should occupy both (1,0) and (1,1)
        assert!(grid.item_at_position(1, 0).is_some());
        assert!(grid.item_at_position(1, 1).is_some());
    }

    #[test]
    fn test_grid_layout_stretch_factors() {
        init_global_registry();

        let mut grid = GridLayout::new();
        grid.set_row_stretch(0, 0);
        grid.set_row_stretch(1, 1);
        grid.set_column_stretch(0, 1);
        grid.set_column_stretch(1, 2);

        assert_eq!(grid.row_stretch(0), 0);
        assert_eq!(grid.row_stretch(1), 1);
        assert_eq!(grid.column_stretch(0), 1);
        assert_eq!(grid.column_stretch(1), 2);
    }

    #[test]
    fn test_grid_layout_minimum_sizes() {
        init_global_registry();

        let mut grid = GridLayout::new();
        grid.set_row_minimum_height(0, 50.0);
        grid.set_column_minimum_width(0, 100.0);

        assert_eq!(grid.row_minimum_height(0), 50.0);
        assert_eq!(grid.column_minimum_width(0), 100.0);
    }

    #[test]
    fn test_grid_layout_spacing() {
        init_global_registry();

        let mut grid = GridLayout::new();
        grid.set_horizontal_spacing(10.0);
        grid.set_vertical_spacing(5.0);

        assert_eq!(grid.horizontal_spacing(), 10.0);
        assert_eq!(grid.vertical_spacing(), 5.0);
    }

    #[test]
    fn test_grid_layout_calculate() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 30.0))));
        let id3 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 40.0))));
        let id4 = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 40.0))));

        let mut grid = GridLayout::new();
        grid.set_content_margins(ContentMargins::uniform(0.0));
        grid.set_horizontal_spacing(10.0);
        grid.set_vertical_spacing(5.0);

        grid.add_widget_at(id1, 0, 0);
        grid.add_widget_at(id2, 0, 1);
        grid.add_widget_at(id3, 1, 0);
        grid.add_widget_at(id4, 1, 1);

        // Set geometry and calculate
        grid.set_geometry(Rect::new(0.0, 0.0, 300.0, 100.0));
        grid.calculate(&storage, Size::new(300.0, 100.0));
        grid.apply(&mut storage);

        // Check that widgets have geometries set
        let w1 = storage.widgets.get(&id1).unwrap();
        let w3 = storage.widgets.get(&id3).unwrap();

        // First widget should be at origin
        assert_eq!(w1.geometry().origin.x, 0.0);
        assert_eq!(w1.geometry().origin.y, 0.0);

        // Third widget (row 1, col 0) should be below first widget + spacing
        assert_eq!(w3.geometry().origin.x, 0.0);
        assert!(w3.geometry().origin.y > 0.0);
    }

    #[test]
    fn test_grid_layout_size_hint() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 30.0))));

        let mut grid = GridLayout::new();
        grid.set_content_margins(ContentMargins::uniform(0.0));
        grid.set_horizontal_spacing(10.0);
        grid.set_vertical_spacing(0.0);

        grid.add_widget_at(id1, 0, 0);
        grid.add_widget_at(id2, 0, 1);

        let hint = grid.size_hint(&storage);

        // Width should be col0 (100) + spacing (10) + col1 (80) = 190
        assert_eq!(hint.preferred.width, 190.0);
        // Height should be max of row0 = 30
        assert_eq!(hint.preferred.height, 30.0);
    }

    #[test]
    fn test_grid_layout_cell_alignment() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 20.0))));

        let mut grid = GridLayout::new();
        grid.set_content_margins(ContentMargins::uniform(0.0));
        grid.set_horizontal_spacing(0.0);
        grid.set_vertical_spacing(0.0);
        grid.set_column_minimum_width(0, 100.0);
        grid.set_row_minimum_height(0, 100.0);

        grid.add_widget_aligned(id1, 0, 0, CellAlignment::center());

        grid.set_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
        grid.calculate(&storage, Size::new(100.0, 100.0));
        grid.apply(&mut storage);

        let w1 = storage.widgets.get(&id1).unwrap();

        // Widget should be centered: (100 - 50) / 2 = 25
        assert_eq!(w1.geometry().origin.x, 25.0);
        // (100 - 20) / 2 = 40
        assert_eq!(w1.geometry().origin.y, 40.0);
    }

    #[test]
    fn test_grid_layout_remove_widget() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 30.0))));

        let mut grid = GridLayout::new();
        grid.add_widget_at(id1, 0, 0);
        grid.add_widget_at(id2, 0, 1);

        assert_eq!(grid.item_count(), 2);

        assert!(grid.remove_widget(id1));
        assert_eq!(grid.item_count(), 1);

        assert!(!grid.remove_widget(id1)); // Already removed
    }

    #[test]
    fn test_cell_alignment_constructors() {
        assert_eq!(
            CellAlignment::fill(),
            CellAlignment::new(Alignment::Stretch, Alignment::Stretch)
        );
        assert_eq!(
            CellAlignment::center(),
            CellAlignment::new(Alignment::Center, Alignment::Center)
        );
        assert_eq!(
            CellAlignment::top_left(),
            CellAlignment::new(Alignment::Start, Alignment::Start)
        );
        assert_eq!(
            CellAlignment::bottom_right(),
            CellAlignment::new(Alignment::End, Alignment::End)
        );
    }
}
