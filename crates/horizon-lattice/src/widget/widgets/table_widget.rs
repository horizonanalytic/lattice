//! Model-less table widget for simple tabular data displays.
//!
//! [`TableWidget`] provides a convenient way to display tabular data without
//! requiring explicit model creation. Cells are managed directly through the widget.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::TableWidget;
//!
//! let mut table = TableWidget::new(3, 4);
//! table.set_horizontal_header_labels(vec!["Name", "Age", "City", "Score"]);
//! table.set_item(0, 0, "Alice".into());
//! table.set_item(0, 1, "30".into());
//!
//! // Connect to signals
//! table.cell_clicked.connect(|(row, col)| {
//!     println!("Clicked cell ({}, {})", row, col);
//! });
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Icon, Point, Rect, Renderer, Stroke};
use parking_lot::RwLock;

use crate::model::selection::{SelectionBehavior, SelectionMode};
use crate::model::{
    CheckState, ItemData, ItemFlags, ItemModel, ItemRole, ModelIndex, ModelSignals, Orientation,
    TextAlignment,
};
use crate::widget::{
    FocusPolicy, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

/// An item in a [`TableWidget`] cell.
///
/// Stores all the data for a single table cell including text, icon, and custom data.
#[derive(Debug, Clone)]
pub struct TableWidgetItem {
    text: String,
    icon: Option<Icon>,
    tooltip: Option<String>,
    check_state: Option<CheckState>,
    flags: ItemFlags,
    data: HashMap<u32, ItemData>,
    background: Option<Color>,
    foreground: Option<Color>,
    text_alignment: TextAlignment,
}

impl Default for TableWidgetItem {
    fn default() -> Self {
        Self::new("")
    }
}

impl TableWidgetItem {
    /// Creates a new item with the given text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            tooltip: None,
            check_state: None,
            flags: ItemFlags::new(),
            data: HashMap::new(),
            background: None,
            foreground: None,
            text_alignment: TextAlignment::left(),
        }
    }

    /// Gets the item's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Sets the item's text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    /// Gets the item's icon.
    pub fn icon(&self) -> Option<&Icon> {
        self.icon.as_ref()
    }

    /// Sets the item's icon.
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon;
    }

    /// Gets the item's tooltip.
    pub fn tooltip(&self) -> Option<&str> {
        self.tooltip.as_deref()
    }

    /// Sets the item's tooltip.
    pub fn set_tooltip(&mut self, tooltip: Option<String>) {
        self.tooltip = tooltip;
    }

    /// Gets the item's check state.
    pub fn check_state(&self) -> Option<CheckState> {
        self.check_state
    }

    /// Sets the item's check state.
    pub fn set_check_state(&mut self, state: Option<CheckState>) {
        self.check_state = state;
    }

    /// Gets the item's flags.
    pub fn flags(&self) -> ItemFlags {
        self.flags
    }

    /// Sets the item's flags.
    pub fn set_flags(&mut self, flags: ItemFlags) {
        self.flags = flags;
    }

    /// Gets custom data for a role.
    pub fn data(&self, role: u32) -> Option<&ItemData> {
        self.data.get(&role)
    }

    /// Sets custom data for a role.
    pub fn set_data(&mut self, role: u32, data: ItemData) {
        self.data.insert(role, data);
    }

    /// Gets the background color.
    pub fn background(&self) -> Option<Color> {
        self.background
    }

    /// Sets the background color.
    pub fn set_background(&mut self, color: Option<Color>) {
        self.background = color;
    }

    /// Gets the foreground (text) color.
    pub fn foreground(&self) -> Option<Color> {
        self.foreground
    }

    /// Sets the foreground (text) color.
    pub fn set_foreground(&mut self, color: Option<Color>) {
        self.foreground = color;
    }

    /// Gets the text alignment.
    pub fn text_alignment(&self) -> TextAlignment {
        self.text_alignment
    }

    /// Sets the text alignment.
    pub fn set_text_alignment(&mut self, alignment: TextAlignment) {
        self.text_alignment = alignment;
    }
}

impl From<&str> for TableWidgetItem {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for TableWidgetItem {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

/// Internal model for TableWidget that implements ItemModel.
struct TableWidgetModel {
    cells: Arc<RwLock<Vec<Vec<Option<TableWidgetItem>>>>>,
    row_count: Arc<RwLock<usize>>,
    column_count: Arc<RwLock<usize>>,
    horizontal_headers: Arc<RwLock<Vec<String>>>,
    vertical_headers: Arc<RwLock<Vec<String>>>,
    signals: ModelSignals,
}

impl TableWidgetModel {
    fn new(
        cells: Arc<RwLock<Vec<Vec<Option<TableWidgetItem>>>>>,
        row_count: Arc<RwLock<usize>>,
        column_count: Arc<RwLock<usize>>,
        horizontal_headers: Arc<RwLock<Vec<String>>>,
        vertical_headers: Arc<RwLock<Vec<String>>>,
    ) -> Self {
        Self {
            cells,
            row_count,
            column_count,
            horizontal_headers,
            vertical_headers,
            signals: ModelSignals::new(),
        }
    }
}

impl ItemModel for TableWidgetModel {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        if parent.is_valid() {
            0
        } else {
            *self.row_count.read()
        }
    }

    fn column_count(&self, parent: &ModelIndex) -> usize {
        if parent.is_valid() {
            0
        } else {
            *self.column_count.read()
        }
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        if !index.is_valid() {
            return ItemData::None;
        }

        let cells = self.cells.read();
        let row = index.row();
        let col = index.column();

        if row >= cells.len() || col >= cells.get(row).map(|r| r.len()).unwrap_or(0) {
            return ItemData::None;
        }

        let Some(item) = cells[row][col].as_ref() else {
            return ItemData::None;
        };

        match role {
            ItemRole::Display => ItemData::String(item.text.clone()),
            ItemRole::Decoration => item
                .icon
                .clone()
                .map(ItemData::Icon)
                .unwrap_or(ItemData::None),
            ItemRole::ToolTip => item
                .tooltip
                .clone()
                .map(ItemData::String)
                .unwrap_or(ItemData::None),
            ItemRole::CheckState => item
                .check_state
                .map(ItemData::CheckState)
                .unwrap_or(ItemData::None),
            ItemRole::BackgroundColor => item
                .background
                .map(ItemData::Color)
                .unwrap_or(ItemData::None),
            ItemRole::ForegroundColor => item
                .foreground
                .map(ItemData::Color)
                .unwrap_or(ItemData::None),
            ItemRole::TextAlignment => ItemData::TextAlignment(item.text_alignment),
            ItemRole::User(n) => item.data.get(&n).cloned().unwrap_or(ItemData::None),
            _ => ItemData::None,
        }
    }

    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex {
        if parent.is_valid() {
            return ModelIndex::invalid();
        }

        let row_count = *self.row_count.read();
        let col_count = *self.column_count.read();

        if row >= row_count || column >= col_count {
            return ModelIndex::invalid();
        }

        ModelIndex::new(row, column, ModelIndex::invalid())
    }

    fn parent(&self, _index: &ModelIndex) -> ModelIndex {
        ModelIndex::invalid()
    }

    fn signals(&self) -> &ModelSignals {
        &self.signals
    }

    fn flags(&self, index: &ModelIndex) -> ItemFlags {
        if !index.is_valid() {
            return ItemFlags::disabled();
        }

        let cells = self.cells.read();
        let row = index.row();
        let col = index.column();

        if row >= cells.len() || col >= cells.get(row).map(|r| r.len()).unwrap_or(0) {
            return ItemFlags::disabled();
        }

        cells[row][col]
            .as_ref()
            .map(|item| item.flags)
            .unwrap_or_default()
    }

    fn header_data(&self, section: usize, orientation: Orientation, role: ItemRole) -> ItemData {
        if role != ItemRole::Display {
            return ItemData::None;
        }

        match orientation {
            Orientation::Horizontal => {
                let headers = self.horizontal_headers.read();
                headers
                    .get(section)
                    .cloned()
                    .map(ItemData::String)
                    .unwrap_or(ItemData::None)
            }
            Orientation::Vertical => {
                let headers = self.vertical_headers.read();
                headers
                    .get(section)
                    .cloned()
                    .map(ItemData::String)
                    .unwrap_or(ItemData::None)
            }
        }
    }
}

const DEFAULT_ROW_HEIGHT: f32 = 24.0;
const DEFAULT_COLUMN_WIDTH: f32 = 100.0;
const HEADER_HEIGHT: f32 = 26.0;

/// A model-less table widget for simple tabular data displays.
///
/// `TableWidget` provides direct cell manipulation without requiring a separate model.
/// For complex scenarios (large datasets, custom data types), use `TableView`
/// with an explicit model instead.
pub struct TableWidget {
    base: WidgetBase,

    // Data storage
    cells: Arc<RwLock<Vec<Vec<Option<TableWidgetItem>>>>>,
    row_count: Arc<RwLock<usize>>,
    column_count: Arc<RwLock<usize>>,
    model: Arc<TableWidgetModel>,

    // Headers
    horizontal_headers: Arc<RwLock<Vec<String>>>,
    vertical_headers: Arc<RwLock<Vec<String>>>,
    show_horizontal_header: bool,
    show_vertical_header: bool,

    // Layout
    column_widths: Vec<f32>,
    row_heights: Vec<f32>,
    default_column_width: f32,
    default_row_height: f32,

    // Scrolling
    scroll_x: i32,
    scroll_y: i32,
    content_width: f32,
    content_height: f32,

    // Selection
    current_row: Option<usize>,
    current_column: Option<usize>,
    selection_mode: SelectionMode,
    selection_behavior: SelectionBehavior,
    selected_cells: Vec<(usize, usize)>,

    // Visual state
    hovered_cell: Option<(usize, usize)>,
    pressed_cell: Option<(usize, usize)>,

    // Appearance
    background_color: Color,
    alternate_row_colors: bool,
    show_grid: bool,
    grid_color: Color,

    // Signals
    /// Emitted when a cell is clicked. Parameters are (row, column).
    pub cell_clicked: Signal<(usize, usize)>,
    /// Emitted when a cell is double-clicked. Parameters are (row, column).
    pub cell_double_clicked: Signal<(usize, usize)>,
    /// Emitted when a cell is activated (Enter or double-click). Parameters are (row, column).
    pub cell_activated: Signal<(usize, usize)>,
    /// Emitted when the current cell changes.
    pub current_cell_changed: Signal<(Option<(usize, usize)>, Option<(usize, usize)>)>,
    /// Emitted when a cell's data changes. Parameters are (row, column).
    pub cell_changed: Signal<(usize, usize)>,
}

impl TableWidget {
    /// Creates a new TableWidget with the specified dimensions.
    pub fn new(rows: usize, columns: usize) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));

        // Initialize cells
        let cells: Vec<Vec<Option<TableWidgetItem>>> =
            (0..rows).map(|_| vec![None; columns]).collect();
        let cells = Arc::new(RwLock::new(cells));
        let row_count = Arc::new(RwLock::new(rows));
        let column_count = Arc::new(RwLock::new(columns));

        // Initialize headers
        let horizontal_headers = Arc::new(RwLock::new(Vec::new()));
        let vertical_headers = Arc::new(RwLock::new(Vec::new()));

        let model = Arc::new(TableWidgetModel::new(
            cells.clone(),
            row_count.clone(),
            column_count.clone(),
            horizontal_headers.clone(),
            vertical_headers.clone(),
        ));

        let mut widget = Self {
            base,
            cells,
            row_count,
            column_count,
            model,
            horizontal_headers,
            vertical_headers,
            show_horizontal_header: true,
            show_vertical_header: false,
            column_widths: vec![DEFAULT_COLUMN_WIDTH; columns],
            row_heights: vec![DEFAULT_ROW_HEIGHT; rows],
            default_column_width: DEFAULT_COLUMN_WIDTH,
            default_row_height: DEFAULT_ROW_HEIGHT,
            scroll_x: 0,
            scroll_y: 0,
            content_width: 0.0,
            content_height: 0.0,
            current_row: None,
            current_column: None,
            selection_mode: SelectionMode::SingleSelection,
            selection_behavior: SelectionBehavior::SelectItems,
            selected_cells: Vec::new(),
            hovered_cell: None,
            pressed_cell: None,
            background_color: Color::WHITE,
            alternate_row_colors: true,
            show_grid: true,
            grid_color: Color::from_rgb8(220, 220, 220),
            cell_clicked: Signal::new(),
            cell_double_clicked: Signal::new(),
            cell_activated: Signal::new(),
            current_cell_changed: Signal::new(),
            cell_changed: Signal::new(),
        };

        widget.update_content_size();
        widget
    }

    // =========================================================================
    // Dimensions
    // =========================================================================

    /// Returns the number of rows.
    pub fn row_count(&self) -> usize {
        *self.row_count.read()
    }

    /// Returns the number of columns.
    pub fn column_count(&self) -> usize {
        *self.column_count.read()
    }

    /// Sets the number of rows.
    pub fn set_row_count(&mut self, count: usize) {
        let old_count = *self.row_count.read();
        if count == old_count {
            return;
        }

        let columns = *self.column_count.read();

        if count > old_count {
            // Add rows
            self.model.signals.emit_rows_inserted(
                ModelIndex::invalid(),
                old_count,
                count - 1,
                || {
                    let mut cells = self.cells.write();
                    for _ in old_count..count {
                        cells.push(vec![None; columns]);
                    }
                    *self.row_count.write() = count;
                },
            );
            self.row_heights.resize(count, self.default_row_height);
        } else {
            // Remove rows
            self.model.signals.emit_rows_removed(
                ModelIndex::invalid(),
                count,
                old_count - 1,
                || {
                    let mut cells = self.cells.write();
                    cells.truncate(count);
                    *self.row_count.write() = count;
                },
            );
            self.row_heights.truncate(count);

            // Update selection
            if let Some(row) = self.current_row
                && row >= count
            {
                self.current_row = if count > 0 { Some(count - 1) } else { None };
            }
        }

        self.update_content_size();
        self.base.update();
    }

    /// Sets the number of columns.
    pub fn set_column_count(&mut self, count: usize) {
        let old_count = *self.column_count.read();
        if count == old_count {
            return;
        }

        {
            let mut cells = self.cells.write();
            for row in cells.iter_mut() {
                row.resize(count, None);
            }
            *self.column_count.write() = count;
        }

        self.column_widths.resize(count, self.default_column_width);

        // Update selection
        if let Some(col) = self.current_column
            && col >= count
        {
            self.current_column = if count > 0 { Some(count - 1) } else { None };
        }

        self.update_content_size();
        self.base.update();
    }

    // =========================================================================
    // Cell Management
    // =========================================================================

    /// Gets a reference to the item at the specified cell.
    pub fn item(&self, row: usize, column: usize) -> Option<TableWidgetItem> {
        let cells = self.cells.read();
        cells.get(row)?.get(column)?.clone()
    }

    /// Sets the item at the specified cell.
    pub fn set_item(&mut self, row: usize, column: usize, item: TableWidgetItem) {
        let rows = *self.row_count.read();
        let cols = *self.column_count.read();

        if row >= rows || column >= cols {
            return;
        }

        {
            let mut cells = self.cells.write();
            cells[row][column] = Some(item);
        }

        let index = ModelIndex::new(row, column, ModelIndex::invalid());
        self.model
            .signals
            .emit_data_changed_single(index, vec![ItemRole::Display]);
        self.cell_changed.emit((row, column));
        self.base.update();
    }

    /// Removes and returns the item at the specified cell.
    pub fn take_item(&mut self, row: usize, column: usize) -> Option<TableWidgetItem> {
        let rows = *self.row_count.read();
        let cols = *self.column_count.read();

        if row >= rows || column >= cols {
            return None;
        }

        let item = {
            let mut cells = self.cells.write();
            cells[row][column].take()
        };

        if item.is_some() {
            let index = ModelIndex::new(row, column, ModelIndex::invalid());
            self.model
                .signals
                .emit_data_changed_single(index, vec![ItemRole::Display]);
            self.cell_changed.emit((row, column));
            self.base.update();
        }

        item
    }

    /// Sets the text at the specified cell.
    pub fn set_cell_text(&mut self, row: usize, column: usize, text: impl Into<String>) {
        let text = text.into();
        self.set_item(row, column, TableWidgetItem::new(text));
    }

    /// Gets the text at the specified cell.
    pub fn cell_text(&self, row: usize, column: usize) -> Option<String> {
        self.item(row, column).map(|item| item.text.clone())
    }

    /// Clears all items but keeps the table structure.
    pub fn clear_contents(&mut self) {
        {
            let mut cells = self.cells.write();
            for row in cells.iter_mut() {
                for cell in row.iter_mut() {
                    *cell = None;
                }
            }
        }

        self.model.signals.model_reset.emit(());
        self.base.update();
    }

    /// Clears all items and resets dimensions to 0.
    pub fn clear(&mut self) {
        self.model.signals.emit_reset(|| {
            let mut cells = self.cells.write();
            cells.clear();
            *self.row_count.write() = 0;
            *self.column_count.write() = 0;
        });

        self.column_widths.clear();
        self.row_heights.clear();
        self.current_row = None;
        self.current_column = None;
        self.selected_cells.clear();
        self.update_content_size();
        self.base.update();
    }

    // =========================================================================
    // Row/Column Operations
    // =========================================================================

    /// Inserts a new row at the specified position.
    pub fn insert_row(&mut self, row: usize) {
        let rows = *self.row_count.read();
        let cols = *self.column_count.read();
        let row = row.min(rows);

        self.model
            .signals
            .emit_rows_inserted(ModelIndex::invalid(), row, row, || {
                let mut cells = self.cells.write();
                cells.insert(row, vec![None; cols]);
                *self.row_count.write() = rows + 1;
            });

        self.row_heights.insert(row, self.default_row_height);

        // Update current selection
        if let Some(current_row) = self.current_row
            && current_row >= row
        {
            self.current_row = Some(current_row + 1);
        }

        self.update_content_size();
        self.base.update();
    }

    /// Removes the row at the specified position.
    pub fn remove_row(&mut self, row: usize) {
        let rows = *self.row_count.read();
        if row >= rows {
            return;
        }

        self.model
            .signals
            .emit_rows_removed(ModelIndex::invalid(), row, row, || {
                let mut cells = self.cells.write();
                cells.remove(row);
                *self.row_count.write() = rows - 1;
            });

        self.row_heights.remove(row);

        // Update current selection
        if let Some(current_row) = self.current_row {
            if current_row == row {
                let new_rows = rows - 1;
                self.current_row = if new_rows > 0 {
                    Some(current_row.min(new_rows - 1))
                } else {
                    None
                };
            } else if current_row > row {
                self.current_row = Some(current_row - 1);
            }
        }

        self.update_content_size();
        self.base.update();
    }

    /// Inserts a new column at the specified position.
    pub fn insert_column(&mut self, column: usize) {
        let cols = *self.column_count.read();
        let column = column.min(cols);

        {
            let mut cells = self.cells.write();
            for row in cells.iter_mut() {
                row.insert(column, None);
            }
            *self.column_count.write() = cols + 1;
        }

        self.column_widths.insert(column, self.default_column_width);

        // Update current selection
        if let Some(current_col) = self.current_column
            && current_col >= column
        {
            self.current_column = Some(current_col + 1);
        }

        self.update_content_size();
        self.base.update();
    }

    /// Removes the column at the specified position.
    pub fn remove_column(&mut self, column: usize) {
        let cols = *self.column_count.read();
        if column >= cols {
            return;
        }

        {
            let mut cells = self.cells.write();
            for row in cells.iter_mut() {
                row.remove(column);
            }
            *self.column_count.write() = cols - 1;
        }

        self.column_widths.remove(column);

        // Update current selection
        if let Some(current_col) = self.current_column {
            if current_col == column {
                let new_cols = cols - 1;
                self.current_column = if new_cols > 0 {
                    Some(current_col.min(new_cols - 1))
                } else {
                    None
                };
            } else if current_col > column {
                self.current_column = Some(current_col - 1);
            }
        }

        self.update_content_size();
        self.base.update();
    }

    // =========================================================================
    // Headers
    // =========================================================================

    /// Sets the horizontal header labels.
    pub fn set_horizontal_header_labels(&mut self, labels: Vec<impl Into<String>>) {
        *self.horizontal_headers.write() = labels.into_iter().map(|l| l.into()).collect();
        self.base.update();
    }

    /// Sets the vertical header labels.
    pub fn set_vertical_header_labels(&mut self, labels: Vec<impl Into<String>>) {
        *self.vertical_headers.write() = labels.into_iter().map(|l| l.into()).collect();
        self.base.update();
    }

    /// Sets whether to show the horizontal header.
    pub fn set_horizontal_header_visible(&mut self, visible: bool) {
        self.show_horizontal_header = visible;
        self.base.update();
    }

    /// Sets whether to show the vertical header.
    pub fn set_vertical_header_visible(&mut self, visible: bool) {
        self.show_vertical_header = visible;
        self.base.update();
    }

    // =========================================================================
    // Column/Row Sizing
    // =========================================================================

    /// Sets the width of a column.
    pub fn set_column_width(&mut self, column: usize, width: f32) {
        if column < self.column_widths.len() {
            self.column_widths[column] = width;
            self.update_content_size();
            self.base.update();
        }
    }

    /// Gets the width of a column.
    pub fn column_width(&self, column: usize) -> f32 {
        self.column_widths
            .get(column)
            .copied()
            .unwrap_or(self.default_column_width)
    }

    /// Sets the height of a row.
    pub fn set_row_height(&mut self, row: usize, height: f32) {
        if row < self.row_heights.len() {
            self.row_heights[row] = height;
            self.update_content_size();
            self.base.update();
        }
    }

    /// Gets the height of a row.
    pub fn row_height(&self, row: usize) -> f32 {
        self.row_heights
            .get(row)
            .copied()
            .unwrap_or(self.default_row_height)
    }

    // =========================================================================
    // Selection
    // =========================================================================

    /// Gets the current cell (row, column).
    pub fn current_cell(&self) -> Option<(usize, usize)> {
        match (self.current_row, self.current_column) {
            (Some(row), Some(col)) => Some((row, col)),
            _ => None,
        }
    }

    /// Sets the current cell.
    pub fn set_current_cell(&mut self, row: usize, column: usize) {
        let rows = *self.row_count.read();
        let cols = *self.column_count.read();

        if row >= rows || column >= cols {
            return;
        }

        let old = self.current_cell();
        self.current_row = Some(row);
        self.current_column = Some(column);
        let new = self.current_cell();

        if old != new {
            // Update selection in single selection mode
            if self.selection_mode == SelectionMode::SingleSelection {
                self.selected_cells = vec![(row, column)];
            }

            self.current_cell_changed.emit((old, new));
            self.base.update();
        }
    }

    /// Gets the selection mode.
    pub fn selection_mode(&self) -> SelectionMode {
        self.selection_mode
    }

    /// Sets the selection mode.
    pub fn set_selection_mode(&mut self, mode: SelectionMode) {
        self.selection_mode = mode;
        if mode == SelectionMode::NoSelection {
            self.selected_cells.clear();
        }
        self.base.update();
    }

    /// Gets the selection behavior.
    pub fn selection_behavior(&self) -> SelectionBehavior {
        self.selection_behavior
    }

    /// Sets the selection behavior.
    pub fn set_selection_behavior(&mut self, behavior: SelectionBehavior) {
        self.selection_behavior = behavior;
        self.base.update();
    }

    /// Returns whether a cell is selected.
    pub fn is_cell_selected(&self, row: usize, column: usize) -> bool {
        match self.selection_behavior {
            SelectionBehavior::SelectItems => self.selected_cells.contains(&(row, column)),
            SelectionBehavior::SelectRows => self.selected_cells.iter().any(|(r, _)| *r == row),
            SelectionBehavior::SelectColumns => {
                self.selected_cells.iter().any(|(_, c)| *c == column)
            }
        }
    }

    /// Clears the selection.
    pub fn clear_selection(&mut self) {
        self.selected_cells.clear();
        self.base.update();
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Sets whether to use alternating row colors.
    pub fn set_alternate_row_colors(&mut self, enabled: bool) {
        self.alternate_row_colors = enabled;
        self.base.update();
    }

    /// Sets whether to show grid lines.
    pub fn set_show_grid(&mut self, show: bool) {
        self.show_grid = show;
        self.base.update();
    }

    /// Sets the grid line color.
    pub fn set_grid_color(&mut self, color: Color) {
        self.grid_color = color;
        self.base.update();
    }

    /// Sets the background color.
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
        self.base.update();
    }

    // =========================================================================
    // Internal
    // =========================================================================

    fn update_content_size(&mut self) {
        self.content_width = self.column_widths.iter().sum();
        self.content_height = self.row_heights.iter().sum();
    }

    fn header_height(&self) -> f32 {
        if self.show_horizontal_header {
            HEADER_HEIGHT
        } else {
            0.0
        }
    }

    fn viewport_rect(&self) -> Rect {
        let rect = self.base.rect();
        let header_h = self.header_height();
        Rect::new(0.0, header_h, rect.width(), rect.height() - header_h)
    }

    fn max_scroll_x(&self) -> i32 {
        let viewport = self.viewport_rect();
        (self.content_width - viewport.width()).max(0.0) as i32
    }

    fn max_scroll_y(&self) -> i32 {
        let viewport = self.viewport_rect();
        (self.content_height - viewport.height()).max(0.0) as i32
    }

    fn cell_at_pos(&self, pos: Point) -> Option<(usize, usize)> {
        let header_h = self.header_height();
        if pos.y < header_h {
            return None;
        }

        let content_x = pos.x + self.scroll_x as f32;
        let content_y = pos.y - header_h + self.scroll_y as f32;

        // Find column
        let mut x = 0.0;
        let mut col = None;
        for (i, &width) in self.column_widths.iter().enumerate() {
            if content_x >= x && content_x < x + width {
                col = Some(i);
                break;
            }
            x += width;
        }

        // Find row
        let mut y = 0.0;
        let mut row = None;
        for (i, &height) in self.row_heights.iter().enumerate() {
            if content_y >= y && content_y < y + height {
                row = Some(i);
                break;
            }
            y += height;
        }

        match (row, col) {
            (Some(r), Some(c)) => Some((r, c)),
            _ => None,
        }
    }

    fn cell_rect(&self, row: usize, column: usize) -> Rect {
        let header_h = self.header_height();

        let x: f32 = self.column_widths[..column].iter().sum();
        let y: f32 = self.row_heights[..row].iter().sum();

        Rect::new(
            x - self.scroll_x as f32,
            y - self.scroll_y as f32 + header_h,
            self.column_widths[column],
            self.row_heights[row],
        )
    }

    fn handle_click(
        &mut self,
        row: usize,
        col: usize,
        modifiers: &crate::widget::KeyboardModifiers,
    ) {
        match self.selection_mode {
            SelectionMode::NoSelection => {
                self.set_current_cell(row, col);
            }
            SelectionMode::SingleSelection => {
                self.set_current_cell(row, col);
            }
            SelectionMode::MultiSelection | SelectionMode::ExtendedSelection => {
                let cell = match self.selection_behavior {
                    SelectionBehavior::SelectItems => (row, col),
                    SelectionBehavior::SelectRows => (row, 0),
                    SelectionBehavior::SelectColumns => (0, col),
                };

                if modifiers.control {
                    // Toggle selection
                    if let Some(pos) = self.selected_cells.iter().position(|&c| c == cell) {
                        self.selected_cells.remove(pos);
                    } else {
                        self.selected_cells.push(cell);
                    }
                } else {
                    self.selected_cells = vec![cell];
                }

                let old = self.current_cell();
                self.current_row = Some(row);
                self.current_column = Some(col);
                let new = self.current_cell();

                if old != new {
                    self.current_cell_changed.emit((old, new));
                }
            }
        }

        self.cell_clicked.emit((row, col));
        self.base.update();
    }
}

impl Object for TableWidget {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for TableWidget {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::from_dimensions(300.0, 200.0).with_minimum_dimensions(100.0, 50.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let widget_rect = self.base.rect();
        let header_h = self.header_height();
        let grid_stroke = Stroke::new(self.grid_color, 1.0);

        // Background
        ctx.renderer().fill_rect(widget_rect, self.background_color);

        let rows = *self.row_count.read();
        let cols = *self.column_count.read();

        // Draw header
        if self.show_horizontal_header {
            let header_rect = Rect::new(0.0, 0.0, widget_rect.width(), header_h);
            ctx.renderer()
                .fill_rect(header_rect, Color::from_rgb8(240, 240, 240));

            let headers = self.horizontal_headers.read();
            let mut x = -self.scroll_x as f32;

            for (col, width) in self.column_widths.iter().enumerate() {
                if x + width > 0.0 && x < widget_rect.width() {
                    let label = headers
                        .get(col)
                        .cloned()
                        .unwrap_or_else(|| (col + 1).to_string());
                    // Draw text placeholder (horizontal line representing text)
                    let text_x = x + 8.0;
                    let text_y = header_h / 2.0;
                    let text_width = (label.len() as f32 * 7.0).min(width - 16.0);
                    if text_width > 0.0 {
                        ctx.renderer().fill_rect(
                            Rect::new(text_x, text_y - 1.0, text_width, 2.0),
                            Color::BLACK,
                        );
                    }

                    // Header separator
                    ctx.renderer().draw_line(
                        Point::new(x + width, 0.0),
                        Point::new(x + width, header_h),
                        &grid_stroke,
                    );
                }
                x += width;
            }

            // Header bottom line
            ctx.renderer().draw_line(
                Point::new(0.0, header_h),
                Point::new(widget_rect.width(), header_h),
                &grid_stroke,
            );
        }

        // Draw cells
        let cells = self.cells.read();
        let viewport = self.viewport_rect();

        for row in 0..rows {
            let cell_rect = self.cell_rect(row, 0);
            if cell_rect.origin.y > viewport.origin.y + viewport.height() {
                break; // Below viewport
            }
            if cell_rect.origin.y + self.row_heights[row] < viewport.origin.y {
                continue; // Above viewport
            }

            for col in 0..cols {
                let cell_rect = self.cell_rect(row, col);
                if cell_rect.origin.x > viewport.width() {
                    break; // Right of viewport
                }
                if cell_rect.origin.x + self.column_widths[col] < 0.0 {
                    continue; // Left of viewport
                }

                // Cell background
                let bg_color = if self.is_cell_selected(row, col) {
                    Color::from_rgb8(51, 153, 255)
                } else if self.hovered_cell == Some((row, col)) {
                    Color::from_rgb8(229, 243, 255)
                } else if self.alternate_row_colors && row % 2 == 1 {
                    Color::from_rgb8(245, 245, 245)
                } else if let Some(item) = cells
                    .get(row)
                    .and_then(|r| r.get(col))
                    .and_then(|c| c.as_ref())
                {
                    item.background.unwrap_or(self.background_color)
                } else {
                    self.background_color
                };

                ctx.renderer().fill_rect(cell_rect, bg_color);

                // Cell text placeholder
                if let Some(item) = cells
                    .get(row)
                    .and_then(|r| r.get(col))
                    .and_then(|c| c.as_ref())
                {
                    let text_color = if self.is_cell_selected(row, col) {
                        Color::WHITE
                    } else {
                        item.foreground.unwrap_or(Color::BLACK)
                    };

                    let text_x = cell_rect.origin.x + 8.0;
                    let text_y = cell_rect.origin.y + cell_rect.height() / 2.0;
                    let text_width = (item.text.len() as f32 * 7.0).min(cell_rect.width() - 16.0);
                    if text_width > 0.0 {
                        ctx.renderer().fill_rect(
                            Rect::new(text_x, text_y - 1.0, text_width, 2.0),
                            text_color,
                        );
                    }
                }

                // Grid lines
                if self.show_grid {
                    // Right border
                    ctx.renderer().draw_line(
                        Point::new(cell_rect.origin.x + cell_rect.width(), cell_rect.origin.y),
                        Point::new(
                            cell_rect.origin.x + cell_rect.width(),
                            cell_rect.origin.y + cell_rect.height(),
                        ),
                        &grid_stroke,
                    );
                    // Bottom border
                    ctx.renderer().draw_line(
                        Point::new(cell_rect.origin.x, cell_rect.origin.y + cell_rect.height()),
                        Point::new(
                            cell_rect.origin.x + cell_rect.width(),
                            cell_rect.origin.y + cell_rect.height(),
                        ),
                        &grid_stroke,
                    );
                }

                // Focus indicator
                if self.current_cell() == Some((row, col)) && self.base.has_focus() {
                    let focus_rect = Rect::new(
                        cell_rect.origin.x + 1.0,
                        cell_rect.origin.y + 1.0,
                        cell_rect.width() - 2.0,
                        cell_rect.height() - 2.0,
                    );
                    let focus_stroke = Stroke::new(Color::from_rgb8(100, 100, 100), 1.0);
                    ctx.renderer().stroke_rect(focus_rect, &focus_stroke);
                }
            }
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                if e.button == crate::widget::MouseButton::Left
                    && let Some((row, col)) = self.cell_at_pos(e.local_pos)
                {
                    self.pressed_cell = Some((row, col));
                    self.handle_click(row, col, &e.modifiers);
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if e.button == crate::widget::MouseButton::Left {
                    self.pressed_cell = None;
                    self.base.update();
                }
            }
            WidgetEvent::MouseMove(e) => {
                let old_hovered = self.hovered_cell;
                self.hovered_cell = self.cell_at_pos(e.local_pos);
                if old_hovered != self.hovered_cell {
                    self.base.update();
                }
            }
            WidgetEvent::Wheel(e) => {
                let scroll_amount = (e.delta_y * 0.5).round() as i32;
                let new_y = (self.scroll_y - scroll_amount).clamp(0, self.max_scroll_y());
                if self.scroll_y != new_y {
                    self.scroll_y = new_y;
                    self.base.update();
                    return true;
                }
            }
            WidgetEvent::KeyPress(e) => {
                let rows = *self.row_count.read();
                let cols = *self.column_count.read();

                if rows == 0 || cols == 0 {
                    return false;
                }

                match e.key {
                    crate::widget::Key::ArrowUp => {
                        if let Some(row) = self.current_row
                            && row > 0
                        {
                            let col = self.current_column.unwrap_or(0);
                            self.set_current_cell(row - 1, col);
                        }
                        return true;
                    }
                    crate::widget::Key::ArrowDown => {
                        if let Some(row) = self.current_row {
                            if row + 1 < rows {
                                let col = self.current_column.unwrap_or(0);
                                self.set_current_cell(row + 1, col);
                            }
                        } else {
                            self.set_current_cell(0, 0);
                        }
                        return true;
                    }
                    crate::widget::Key::ArrowLeft => {
                        if let Some(col) = self.current_column
                            && col > 0
                        {
                            let row = self.current_row.unwrap_or(0);
                            self.set_current_cell(row, col - 1);
                        }
                        return true;
                    }
                    crate::widget::Key::ArrowRight => {
                        if let Some(col) = self.current_column {
                            if col + 1 < cols {
                                let row = self.current_row.unwrap_or(0);
                                self.set_current_cell(row, col + 1);
                            }
                        } else {
                            self.set_current_cell(0, 0);
                        }
                        return true;
                    }
                    crate::widget::Key::Home => {
                        let row = self.current_row.unwrap_or(0);
                        self.set_current_cell(row, 0);
                        return true;
                    }
                    crate::widget::Key::End => {
                        let row = self.current_row.unwrap_or(0);
                        self.set_current_cell(row, cols - 1);
                        return true;
                    }
                    crate::widget::Key::Enter | crate::widget::Key::NumpadEnter => {
                        if let Some((row, col)) = self.current_cell() {
                            self.cell_activated.emit((row, col));
                        }
                        return true;
                    }
                    _ => {}
                }
            }
            WidgetEvent::Leave(_) => {
                self.hovered_cell = None;
                self.base.update();
            }
            WidgetEvent::Resize(_) => {
                self.scroll_x = self.scroll_x.min(self.max_scroll_x());
                self.scroll_y = self.scroll_y.min(self.max_scroll_y());
            }
            _ => {}
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_table_widget_creation() {
        setup();
        let widget = TableWidget::new(3, 4);
        assert_eq!(widget.row_count(), 3);
        assert_eq!(widget.column_count(), 4);
    }

    #[test]
    fn test_set_item() {
        setup();
        let mut widget = TableWidget::new(3, 3);
        widget.set_item(1, 2, TableWidgetItem::new("Hello"));

        let item = widget.item(1, 2);
        assert!(item.is_some());
        assert_eq!(item.unwrap().text(), "Hello");
    }

    #[test]
    fn test_set_cell_text() {
        setup();
        let mut widget = TableWidget::new(2, 2);
        widget.set_cell_text(0, 0, "Test");

        assert_eq!(widget.cell_text(0, 0), Some("Test".to_string()));
    }

    #[test]
    fn test_dimensions() {
        setup();
        let mut widget = TableWidget::new(2, 2);

        widget.set_row_count(5);
        assert_eq!(widget.row_count(), 5);

        widget.set_column_count(3);
        assert_eq!(widget.column_count(), 3);
    }

    #[test]
    fn test_insert_remove_row() {
        setup();
        let mut widget = TableWidget::new(2, 2);
        widget.set_cell_text(0, 0, "A");
        widget.set_cell_text(1, 0, "B");

        widget.insert_row(1);
        assert_eq!(widget.row_count(), 3);
        assert_eq!(widget.cell_text(0, 0), Some("A".to_string()));
        assert_eq!(widget.cell_text(1, 0), None);
        assert_eq!(widget.cell_text(2, 0), Some("B".to_string()));

        widget.remove_row(1);
        assert_eq!(widget.row_count(), 2);
        assert_eq!(widget.cell_text(0, 0), Some("A".to_string()));
        assert_eq!(widget.cell_text(1, 0), Some("B".to_string()));
    }

    #[test]
    fn test_clear() {
        setup();
        let mut widget = TableWidget::new(3, 3);
        widget.set_cell_text(0, 0, "Test");

        widget.clear_contents();
        assert_eq!(widget.row_count(), 3);
        assert!(widget.cell_text(0, 0).is_none());

        widget.set_cell_text(0, 0, "Test");
        widget.clear();
        assert_eq!(widget.row_count(), 0);
        assert_eq!(widget.column_count(), 0);
    }

    #[test]
    fn test_headers() {
        setup();
        let mut widget = TableWidget::new(2, 3);
        widget.set_horizontal_header_labels(vec!["A", "B", "C"]);

        // Headers are stored internally, can be verified through model
        assert_eq!(widget.column_count(), 3);
    }
}
