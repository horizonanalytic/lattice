//! TableView widget for displaying tabular data.
//!
//! This module provides [`TableView`], a view widget that displays data from
//! an [`ItemModel`] in a 2D grid with support for headers, sorting, and
//! column management.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::model::{TableModel, SimpleTableModel};
//! use horizon_lattice::widget::widgets::{TableView, SelectionBehavior};
//! use std::sync::Arc;
//!
//! // Create a simple table model
//! let model = Arc::new(SimpleTableModel::new(3, 5));
//!
//! // Create table view
//! let mut table = TableView::new()
//!     .with_model(model)
//!     .with_sorting_enabled(true);
//!
//! // Connect to signals
//! table.clicked.connect(|index| {
//!     println!("Clicked cell ({}, {})", index.row(), index.column());
//! });
//! ```

use std::sync::Arc;
use std::time::Instant;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Stroke};

use crate::model::{
    DefaultItemDelegate, DelegatePaintContext, ItemDelegate, ItemModel, ItemRole, ModelIndex,
    Orientation, SelectionBehavior, SelectionFlags, SelectionMode, SelectionModel,
    StyleOptionViewItem, ViewItemFeatures, ViewItemState,
};
use crate::widget::drag_drop::{
    DragData, DragEnterEvent, DragLeaveEvent, DragMoveEvent, DropAction, DropEvent,
    DropIndicatorState, DropPosition,
};
use crate::widget::{
    ContextMenuEvent, FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent,
    MousePressEvent, MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair,
    WheelEvent, Widget, WidgetBase, WidgetEvent,
};

use super::header_view::{HeaderView, SortOrder};
use super::scroll_area::ScrollBarPolicy;

/// Grid line style for TableView.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridStyle {
    /// Show both horizontal and vertical grid lines.
    #[default]
    Both,
    /// Show only horizontal grid lines.
    Horizontal,
    /// Show only vertical grid lines.
    Vertical,
    /// No grid lines.
    None,
}

/// Drag and drop mode for TableView.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableDragDropMode {
    /// No drag or drop support (default).
    #[default]
    NoDragDrop,
    /// View can only drag items.
    DragOnly,
    /// View can only accept drops.
    DropOnly,
    /// Full drag and drop support.
    DragDrop,
    /// Internal move only (reorder items within the same view).
    InternalMove,
}

/// Location where a context menu was requested in a TableView.
#[derive(Debug, Clone, PartialEq)]
pub enum TableContextMenuLocation {
    /// Context menu requested on a cell.
    ///
    /// The ModelIndex is the cell that was clicked. If clicked on empty space
    /// within the cell area, the index may be invalid.
    Cell(ModelIndex),
    /// Context menu requested on a column header.
    ///
    /// The usize is the logical column index.
    ColumnHeader(usize),
    /// Context menu requested on a row header.
    ///
    /// The usize is the row index.
    RowHeader(usize),
    /// Context menu requested on the corner widget (between row and column headers).
    Corner,
    /// Context menu requested on empty space (outside data area).
    Empty,
}

const DEFAULT_ROW_HEIGHT: f32 = 24.0;
const DEFAULT_COLUMN_WIDTH: f32 = 100.0;
const SCROLLBAR_THICKNESS: f32 = 14.0;

/// A table view widget for displaying 2D tabular data.
///
/// TableView displays data from an ItemModel in a grid with rows and columns.
/// It supports:
/// - Column and row headers (via HeaderView)
/// - Column resizing, reordering, and hiding
/// - Row, column, or cell selection modes
/// - Frozen rows and columns
/// - Virtualized rendering for large datasets
/// - Sorting via header clicks
///
/// # Signals
///
/// - `clicked(ModelIndex)`: Emitted when a cell is clicked
/// - `double_clicked(ModelIndex)`: Emitted when a cell is double-clicked
/// - `activated(ModelIndex)`: Emitted when Enter is pressed or cell is double-clicked
/// - `header_clicked((Orientation, usize))`: Emitted when a header section is clicked
pub struct TableView {
    base: WidgetBase,

    // Model/View
    model: Option<Arc<dyn ItemModel>>,
    selection_model: SelectionModel,
    delegate: Arc<dyn ItemDelegate>,

    // Headers
    horizontal_header: HeaderView,
    vertical_header: HeaderView,
    show_horizontal_header: bool,
    show_vertical_header: bool,

    // Row management
    row_heights: Vec<f32>,
    row_positions: Vec<f32>,
    default_row_height: f32,
    uniform_row_heights: bool,

    // Scrolling
    scroll_x: i32,
    scroll_y: i32,
    content_width: f32,
    content_height: f32,
    scrollbar_policy_h: ScrollBarPolicy,
    scrollbar_policy_v: ScrollBarPolicy,
    scrollbar_thickness: f32,

    // Layout
    layout_dirty: bool,

    // Frozen sections
    frozen_row_count: usize,
    frozen_column_count: usize,

    // Grid
    show_grid: bool,
    grid_style: GridStyle,
    grid_color: Color,

    // Visual state
    hovered_cell: Option<(usize, usize)>,
    pressed_cell: Option<(usize, usize)>,
    last_click_time: Option<Instant>,
    last_click_cell: Option<(usize, usize)>,

    // Sorting
    sorting_enabled: bool,

    // Appearance
    background_color: Color,
    alternate_row_colors: bool,

    // Drag and drop
    drag_drop_mode: TableDragDropMode,
    drop_indicator_state: DropIndicatorState,
    drag_start_pos: Option<Point>,
    dragging_cell: Option<(usize, usize)>,

    // Signals
    /// Emitted when a cell is clicked.
    pub clicked: Signal<ModelIndex>,
    /// Emitted when a cell is double-clicked.
    pub double_clicked: Signal<ModelIndex>,
    /// Emitted when a cell is activated (Enter or double-click).
    pub activated: Signal<ModelIndex>,
    /// Emitted when a header is clicked. Args: (orientation, section).
    pub header_clicked: Signal<(Orientation, usize)>,
    /// Emitted when a context menu is requested.
    ///
    /// The tuple contains (location, position in widget coords).
    /// The location indicates what area of the table was clicked
    /// (cell, column header, row header, corner, or empty space).
    pub context_menu_requested: Signal<(TableContextMenuLocation, Point)>,
}

impl Default for TableView {
    fn default() -> Self {
        Self::new()
    }
}

impl TableView {
    /// Creates a new empty TableView.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));

        let mut selection_model = SelectionModel::new();
        selection_model.set_selection_behavior(SelectionBehavior::SelectRows);

        Self {
            base,
            model: None,
            selection_model,
            delegate: Arc::new(DefaultItemDelegate::new()),
            horizontal_header: HeaderView::new(Orientation::Horizontal),
            vertical_header: HeaderView::new(Orientation::Vertical),
            show_horizontal_header: true,
            show_vertical_header: false,
            row_heights: Vec::new(),
            row_positions: Vec::new(),
            default_row_height: DEFAULT_ROW_HEIGHT,
            uniform_row_heights: true,
            scroll_x: 0,
            scroll_y: 0,
            content_width: 0.0,
            content_height: 0.0,
            scrollbar_policy_h: ScrollBarPolicy::AsNeeded,
            scrollbar_policy_v: ScrollBarPolicy::AsNeeded,
            scrollbar_thickness: SCROLLBAR_THICKNESS,
            layout_dirty: true,
            frozen_row_count: 0,
            frozen_column_count: 0,
            show_grid: true,
            grid_style: GridStyle::Both,
            grid_color: Color::from_rgb8(220, 220, 220),
            hovered_cell: None,
            pressed_cell: None,
            last_click_time: None,
            last_click_cell: None,
            sorting_enabled: false,
            background_color: Color::WHITE,
            alternate_row_colors: true,
            drag_drop_mode: TableDragDropMode::NoDragDrop,
            drop_indicator_state: DropIndicatorState::new(),
            drag_start_pos: None,
            dragging_cell: None,
            clicked: Signal::new(),
            double_clicked: Signal::new(),
            activated: Signal::new(),
            header_clicked: Signal::new(),
            context_menu_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Methods
    // =========================================================================

    /// Sets the model using builder pattern.
    pub fn with_model(mut self, model: Arc<dyn ItemModel>) -> Self {
        self.set_model(Some(model));
        self
    }

    /// Sets the selection mode using builder pattern.
    pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_model.set_selection_mode(mode);
        self
    }

    /// Sets the selection behavior using builder pattern.
    pub fn with_selection_behavior(mut self, behavior: SelectionBehavior) -> Self {
        self.selection_model.set_selection_behavior(behavior);
        self
    }

    /// Sets the delegate using builder pattern.
    pub fn with_delegate(mut self, delegate: Arc<dyn ItemDelegate>) -> Self {
        self.delegate = delegate;
        self.layout_dirty = true;
        self
    }

    /// Enables or disables sorting using builder pattern.
    pub fn with_sorting_enabled(mut self, enabled: bool) -> Self {
        self.sorting_enabled = enabled;
        self
    }

    /// Sets alternate row colors using builder pattern.
    pub fn with_alternate_row_colors(mut self, enabled: bool) -> Self {
        self.alternate_row_colors = enabled;
        self
    }

    /// Sets the drag and drop mode using builder pattern.
    pub fn with_drag_drop_mode(mut self, mode: TableDragDropMode) -> Self {
        self.set_drag_drop_mode(mode);
        self
    }

    // =========================================================================
    // Model
    // =========================================================================

    /// Gets the current model.
    pub fn model(&self) -> Option<&Arc<dyn ItemModel>> {
        self.model.as_ref()
    }

    /// Sets the model.
    pub fn set_model(&mut self, model: Option<Arc<dyn ItemModel>>) {
        self.model = model.clone();
        self.selection_model.reset();
        self.layout_dirty = true;

        // Update header section counts
        if let Some(m) = &model {
            let parent = ModelIndex::invalid();
            let col_count = m.column_count(&parent);
            let row_count = m.row_count(&parent);

            self.horizontal_header.set_section_count(col_count);
            self.horizontal_header.set_model(model.clone());

            self.vertical_header.set_section_count(row_count);
            self.vertical_header.set_model(model);
        } else {
            self.horizontal_header.set_section_count(0);
            self.horizontal_header.set_model(None);
            self.vertical_header.set_section_count(0);
            self.vertical_header.set_model(None);
        }

        self.base.update();
    }

    // =========================================================================
    // Selection
    // =========================================================================

    /// Gets a reference to the selection model.
    pub fn selection_model(&self) -> &SelectionModel {
        &self.selection_model
    }

    /// Gets a mutable reference to the selection model.
    pub fn selection_model_mut(&mut self) -> &mut SelectionModel {
        &mut self.selection_model
    }

    /// Gets the current (focused) index.
    pub fn current_index(&self) -> &ModelIndex {
        self.selection_model.current_index()
    }

    /// Sets the current index.
    pub fn set_current_index(&mut self, index: ModelIndex) {
        self.selection_model
            .set_current_index(index, SelectionFlags::CLEAR_SELECT_CURRENT);
        self.base.update();
    }

    /// Clears the selection.
    pub fn clear_selection(&mut self) {
        self.selection_model.clear_selection();
        self.base.update();
    }

    /// Selects all cells.
    pub fn select_all(&mut self) {
        if let Some(model) = &self.model {
            let parent = ModelIndex::invalid();
            let row_count = model.row_count(&parent);
            let col_count = model.column_count(&parent);

            for row in 0..row_count {
                for col in 0..col_count {
                    let index = ModelIndex::new(row, col, parent.clone());
                    self.selection_model.select(index, SelectionFlags::SELECT);
                }
            }
            self.base.update();
        }
    }

    /// Selects an entire row.
    pub fn select_row(&mut self, row: usize) {
        if let Some(model) = &self.model {
            let parent = ModelIndex::invalid();
            let col_count = model.column_count(&parent);
            self.selection_model
                .select_row(row, col_count, SelectionFlags::CLEAR_AND_SELECT);
            self.base.update();
        }
    }

    /// Selects an entire column.
    pub fn select_column(&mut self, column: usize) {
        if let Some(model) = &self.model {
            let parent = ModelIndex::invalid();
            let row_count = model.row_count(&parent);
            self.selection_model
                .select_column(column, row_count, SelectionFlags::CLEAR_AND_SELECT);
            self.base.update();
        }
    }

    // =========================================================================
    // Headers
    // =========================================================================

    /// Gets a reference to the horizontal header.
    pub fn horizontal_header(&self) -> &HeaderView {
        &self.horizontal_header
    }

    /// Gets a mutable reference to the horizontal header.
    pub fn horizontal_header_mut(&mut self) -> &mut HeaderView {
        &mut self.horizontal_header
    }

    /// Gets a reference to the vertical header.
    pub fn vertical_header(&self) -> &HeaderView {
        &self.vertical_header
    }

    /// Gets a mutable reference to the vertical header.
    pub fn vertical_header_mut(&mut self) -> &mut HeaderView {
        &mut self.vertical_header
    }

    /// Sets horizontal header visibility.
    pub fn set_horizontal_header_visible(&mut self, visible: bool) {
        if self.show_horizontal_header != visible {
            self.show_horizontal_header = visible;
            self.layout_dirty = true;
            self.base.update();
        }
    }

    /// Sets vertical header visibility.
    pub fn set_vertical_header_visible(&mut self, visible: bool) {
        if self.show_vertical_header != visible {
            self.show_vertical_header = visible;
            self.layout_dirty = true;
            self.base.update();
        }
    }

    // =========================================================================
    // Column Management
    // =========================================================================

    /// Returns the number of columns.
    pub fn column_count(&self) -> usize {
        self.horizontal_header.section_count()
    }

    /// Returns the width of a column.
    pub fn column_width(&self, column: usize) -> f32 {
        self.horizontal_header.section_size(column)
    }

    /// Sets the width of a column.
    pub fn set_column_width(&mut self, column: usize, width: f32) {
        self.horizontal_header.set_section_size(column, width);
        self.layout_dirty = true;
        self.base.update();
    }

    /// Returns whether a column is hidden.
    pub fn is_column_hidden(&self, column: usize) -> bool {
        self.horizontal_header.is_section_hidden(column)
    }

    /// Sets whether a column is hidden.
    pub fn set_column_hidden(&mut self, column: usize, hidden: bool) {
        self.horizontal_header.set_section_hidden(column, hidden);
        self.layout_dirty = true;
        self.base.update();
    }

    /// Shows a column.
    pub fn show_column(&mut self, column: usize) {
        self.set_column_hidden(column, false);
    }

    /// Hides a column.
    pub fn hide_column(&mut self, column: usize) {
        self.set_column_hidden(column, true);
    }

    // =========================================================================
    // Row Management
    // =========================================================================

    /// Returns the number of rows.
    pub fn row_count(&self) -> usize {
        self.model
            .as_ref()
            .map(|m| m.row_count(&ModelIndex::invalid()))
            .unwrap_or(0)
    }

    /// Returns the height of a row.
    pub fn row_height(&self, row: usize) -> f32 {
        self.row_heights
            .get(row)
            .copied()
            .unwrap_or(self.default_row_height)
    }

    /// Sets the height of a row.
    pub fn set_row_height(&mut self, row: usize, height: f32) {
        if row < self.row_heights.len() {
            self.row_heights[row] = height.max(1.0);
            self.layout_dirty = true;
            self.base.update();
        }
    }

    /// Sets the default row height.
    pub fn set_default_row_height(&mut self, height: f32) {
        self.default_row_height = height.max(1.0);
        self.layout_dirty = true;
        self.base.update();
    }

    // =========================================================================
    // Frozen Sections
    // =========================================================================

    /// Returns the number of frozen rows.
    pub fn frozen_row_count(&self) -> usize {
        self.frozen_row_count
    }

    /// Sets the number of frozen rows.
    pub fn set_frozen_row_count(&mut self, count: usize) {
        self.frozen_row_count = count;
        self.base.update();
    }

    /// Returns the number of frozen columns.
    pub fn frozen_column_count(&self) -> usize {
        self.frozen_column_count
    }

    /// Sets the number of frozen columns.
    pub fn set_frozen_column_count(&mut self, count: usize) {
        self.frozen_column_count = count;
        self.base.update();
    }

    // =========================================================================
    // Grid
    // =========================================================================

    /// Returns whether the grid is shown.
    pub fn show_grid(&self) -> bool {
        self.show_grid
    }

    /// Sets whether to show the grid.
    pub fn set_show_grid(&mut self, show: bool) {
        if self.show_grid != show {
            self.show_grid = show;
            self.base.update();
        }
    }

    /// Sets the grid style.
    pub fn set_grid_style(&mut self, style: GridStyle) {
        if self.grid_style != style {
            self.grid_style = style;
            self.base.update();
        }
    }

    /// Sets the grid color.
    pub fn set_grid_color(&mut self, color: Color) {
        if self.grid_color != color {
            self.grid_color = color;
            self.base.update();
        }
    }

    // =========================================================================
    // Drag and Drop
    // =========================================================================

    /// Returns the current drag and drop mode.
    pub fn drag_drop_mode(&self) -> TableDragDropMode {
        self.drag_drop_mode
    }

    /// Sets the drag and drop mode.
    ///
    /// This controls whether the table view supports dragging items from it,
    /// accepting drops onto it, or both.
    pub fn set_drag_drop_mode(&mut self, mode: TableDragDropMode) {
        if self.drag_drop_mode != mode {
            self.drag_drop_mode = mode;
            // Configure whether we accept drops based on the mode
            let accepts_drops = matches!(
                mode,
                TableDragDropMode::DropOnly
                    | TableDragDropMode::DragDrop
                    | TableDragDropMode::InternalMove
            );
            self.base.set_accepts_drops(accepts_drops);
        }
    }

    /// Returns whether dragging from this view is enabled.
    pub fn drag_enabled(&self) -> bool {
        matches!(
            self.drag_drop_mode,
            TableDragDropMode::DragOnly
                | TableDragDropMode::DragDrop
                | TableDragDropMode::InternalMove
        )
    }

    /// Returns whether dropping onto this view is enabled.
    pub fn drop_enabled(&self) -> bool {
        matches!(
            self.drag_drop_mode,
            TableDragDropMode::DropOnly
                | TableDragDropMode::DragDrop
                | TableDragDropMode::InternalMove
        )
    }

    /// Creates drag data from the selected cells.
    fn create_drag_data(&self, indices: &[ModelIndex]) -> Option<DragData> {
        if indices.is_empty() {
            return None;
        }

        let mut data = DragData::new();

        // Extract text from selected items
        if let Some(model) = &self.model {
            let texts: Vec<String> = indices
                .iter()
                .filter_map(|index| {
                    model
                        .data(index, ItemRole::Display)
                        .as_string()
                        .map(String::from)
                })
                .collect();

            if !texts.is_empty() {
                data.set_text(texts.join("\n"));
            }
        }

        if data.is_empty() { None } else { Some(data) }
    }

    /// Returns the drop position for the given point.
    ///
    /// This determines where items will be inserted when dropped.
    fn drop_position_for_point(&self, point: Point) -> (Option<(usize, usize)>, DropPosition) {
        if let Some(index) = self.index_at(point) {
            let row = index.row();
            let col = index.column();

            // Get the visual cell rect for position calculation
            if let Some(visual_rect) = self.row_rect_visual(row) {
                // Determine if in upper or lower half (for row-based drop)
                let mid_y = visual_rect.origin.y + visual_rect.height() / 2.0;
                if point.y < mid_y {
                    return (Some((row, col)), DropPosition::AboveItem);
                } else {
                    return (Some((row, col)), DropPosition::BelowItem);
                }
            }
        }

        // After last row
        if let Some(model) = &self.model {
            let row_count = model.row_count(&ModelIndex::invalid());
            if row_count > 0 {
                return (Some((row_count - 1, 0)), DropPosition::BelowItem);
            }
        }

        (None, DropPosition::OnItem)
    }

    /// Handles the DragEnter event.
    fn handle_drag_enter(&mut self, event: &mut DragEnterEvent) -> bool {
        if !self.drop_enabled() {
            return false;
        }

        // Accept the drag if it has text data
        if event.data().has_text() || event.data().has_urls() {
            event.accept_proposed_action();
            event.set_proposed_action(DropAction::COPY);
            return true;
        }

        false
    }

    /// Handles the DragMove event.
    fn handle_drag_move(&mut self, event: &mut DragMoveEvent) -> bool {
        if !self.drop_enabled() {
            return false;
        }

        // Update drop indicator
        let content_area = self.content_area_rect();
        if content_area.contains(event.local_pos) {
            // Calculate visible row rects for drop indicator
            let first_visible_row = self.first_visible_row();
            let last_visible_row = self.last_visible_row();

            let item_rects: Vec<(usize, Rect)> = (first_visible_row..=last_visible_row)
                .filter_map(|row| {
                    // Get the row rect (full width) in visual coordinates
                    let row_rect = self.row_rect_visual(row)?;
                    Some((row, row_rect))
                })
                .collect();

            self.drop_indicator_state.update_for_vertical_list(
                event.local_pos,
                &item_rects,
                content_area.width(),
            );
        } else {
            self.drop_indicator_state.clear();
        }

        event.accept();
        self.base.update();
        true
    }

    /// Handles the DragLeave event.
    fn handle_drag_leave(&mut self, _event: &mut DragLeaveEvent) -> bool {
        self.drop_indicator_state.clear();
        self.base.update();
        true
    }

    /// Handles the Drop event.
    fn handle_drop(&mut self, event: &mut DropEvent) -> bool {
        if !self.drop_enabled() {
            return false;
        }

        let (_drop_cell, _drop_position) = self.drop_position_for_point(event.local_pos);
        self.drop_indicator_state.clear();
        self.base.update();

        // Process the dropped data
        if event.data().has_text() || event.data().has_urls() {
            // Accept the drop
            // In a real implementation, this would insert rows into the model
            event.accept();
            return true;
        }

        false
    }

    /// Paints the drop indicator if active.
    fn paint_drop_indicator(&self, ctx: &mut PaintContext<'_>) {
        if let Some(indicator) = self.drop_indicator_state.indicator() {
            let style = self.drop_indicator_state.style();
            ctx.renderer().fill_rect(indicator.rect, style.line_color);
        }
    }

    /// Returns the first visible row index.
    fn first_visible_row(&self) -> usize {
        if self.row_positions.is_empty() {
            return 0;
        }
        // Find the first row whose bottom is visible
        let scroll_y = self.scroll_y as f32;
        for (row, &pos) in self.row_positions.iter().enumerate() {
            let height = self
                .row_heights
                .get(row)
                .copied()
                .unwrap_or(self.default_row_height);
            if pos + height > scroll_y {
                return row;
            }
        }
        0
    }

    /// Returns the last visible row index.
    fn last_visible_row(&self) -> usize {
        if self.row_positions.is_empty() {
            return 0;
        }
        let content_area = self.content_area_rect();
        let scroll_y = self.scroll_y as f32;
        let visible_bottom = scroll_y + content_area.height();

        for (row, &pos) in self.row_positions.iter().enumerate().rev() {
            if pos < visible_bottom {
                return row;
            }
        }
        self.row_positions.len().saturating_sub(1)
    }

    /// Returns the rect for a row in visual (widget) coordinates.
    fn row_rect_visual(&self, row: usize) -> Option<Rect> {
        let content_area = self.content_area_rect();
        let y = self.row_positions.get(row)?;
        let height = self
            .row_heights
            .get(row)
            .copied()
            .unwrap_or(self.default_row_height);

        // Adjust for scrolling
        let visual_y = y - self.scroll_y as f32 + content_area.origin.y;

        Some(Rect::new(
            content_area.origin.x,
            visual_y,
            content_area.width(),
            height,
        ))
    }

    // =========================================================================
    // Sorting
    // =========================================================================

    /// Returns whether sorting is enabled.
    pub fn is_sorting_enabled(&self) -> bool {
        self.sorting_enabled
    }

    /// Sets whether sorting is enabled.
    pub fn set_sorting_enabled(&mut self, enabled: bool) {
        self.sorting_enabled = enabled;
    }

    /// Sets the sort indicator on a column.
    pub fn set_sort_indicator(&mut self, column: usize, order: SortOrder) {
        self.horizontal_header.set_sort_indicator(column, order);
    }

    // =========================================================================
    // Scrolling
    // =========================================================================

    /// Returns the horizontal scroll position.
    pub fn scroll_x(&self) -> i32 {
        self.scroll_x
    }

    /// Returns the vertical scroll position.
    pub fn scroll_y(&self) -> i32 {
        self.scroll_y
    }

    /// Sets the scroll position.
    pub fn set_scroll_position(&mut self, x: i32, y: i32) {
        let max_x = self.max_scroll_x();
        let max_y = self.max_scroll_y();
        let new_x = x.clamp(0, max_x);
        let new_y = y.clamp(0, max_y);

        if self.scroll_x != new_x || self.scroll_y != new_y {
            self.scroll_x = new_x;
            self.scroll_y = new_y;

            // Update header offsets
            self.horizontal_header.set_offset(new_x);
            self.vertical_header.set_offset(new_y);

            self.base.update();
        }
    }

    /// Scrolls to make an index visible.
    pub fn scroll_to(&mut self, index: &ModelIndex) {
        self.ensure_layout();
        if !index.is_valid() {
            return;
        }

        let row = index.row();
        let col = index.column();

        if let Some(cell_rect) = self.cell_rect(row, col) {
            let viewport = self.content_area_rect();

            // Scroll vertically
            let item_top = cell_rect.origin.y as i32;
            let item_bottom = (cell_rect.origin.y + cell_rect.height()) as i32;
            let viewport_top = self.scroll_y;
            let viewport_bottom = self.scroll_y + viewport.height() as i32;

            let new_scroll_y = if item_top < viewport_top {
                item_top
            } else if item_bottom > viewport_bottom {
                item_bottom - viewport.height() as i32
            } else {
                self.scroll_y
            };

            // Scroll horizontally
            let item_left = cell_rect.origin.x as i32;
            let item_right = (cell_rect.origin.x + cell_rect.width()) as i32;
            let viewport_left = self.scroll_x;
            let viewport_right = self.scroll_x + viewport.width() as i32;

            let new_scroll_x = if item_left < viewport_left {
                item_left
            } else if item_right > viewport_right {
                item_right - viewport.width() as i32
            } else {
                self.scroll_x
            };

            self.set_scroll_position(new_scroll_x, new_scroll_y);
        }
    }

    fn max_scroll_x(&self) -> i32 {
        let viewport = self.content_area_rect();
        (self.content_width - viewport.width()).max(0.0) as i32
    }

    fn max_scroll_y(&self) -> i32 {
        let viewport = self.content_area_rect();
        (self.content_height - viewport.height()).max(0.0) as i32
    }

    // =========================================================================
    // Queries
    // =========================================================================

    /// Returns the model index at a point in widget coordinates.
    pub fn index_at(&self, point: Point) -> Option<ModelIndex> {
        let content_area = self.content_area_rect();
        if !content_area.contains(point) {
            return None;
        }

        let content_x = point.x - content_area.origin.x + self.scroll_x as f32;
        let content_y = point.y - content_area.origin.y + self.scroll_y as f32;

        let row = self.row_at_content_y(content_y)?;
        let col = self.column_at_content_x(content_x)?;

        Some(ModelIndex::new(row, col, ModelIndex::invalid()))
    }

    fn row_at_content_y(&self, y: f32) -> Option<usize> {
        let row_count = self.row_count();
        if row_count == 0 {
            return None;
        }

        // Binary search in row_positions
        let mut lo = 0;
        let mut hi = row_count;

        while lo < hi {
            let mid = (lo + hi) / 2;
            let row_top = self.row_positions.get(mid).copied().unwrap_or(0.0);
            let row_height = self
                .row_heights
                .get(mid)
                .copied()
                .unwrap_or(self.default_row_height);

            if y < row_top {
                hi = mid;
            } else if y >= row_top + row_height {
                lo = mid + 1;
            } else {
                return Some(mid);
            }
        }

        None
    }

    fn column_at_content_x(&self, x: f32) -> Option<usize> {
        self.horizontal_header.section_at(x)
    }

    /// Returns whether an index is visible.
    pub fn is_index_visible(&self, index: &ModelIndex) -> bool {
        if !index.is_valid() {
            return false;
        }

        let (first_row, last_row) = self.visible_rows();
        let (first_col, last_col) = self.visible_columns();

        let row = index.row();
        let col = index.column();

        row >= first_row && row <= last_row && col >= first_col && col <= last_col
    }

    // =========================================================================
    // Layout
    // =========================================================================

    fn ensure_layout(&mut self) {
        if self.layout_dirty {
            self.update_layout();
            self.layout_dirty = false;
        }
    }

    fn update_layout(&mut self) {
        let row_count = self.row_count();

        // Update row heights and positions
        self.row_heights.clear();
        self.row_heights.reserve(row_count);
        self.row_positions.clear();
        self.row_positions.reserve(row_count);

        let mut y = 0.0;
        for _ in 0..row_count {
            self.row_positions.push(y);
            let height = self.default_row_height;
            self.row_heights.push(height);
            y += height;
        }

        self.content_height = y;
        self.content_width = self.horizontal_header.total_size();
    }

    fn visible_rows(&self) -> (usize, usize) {
        let row_count = self.row_count();
        if row_count == 0 {
            return (0, 0);
        }

        let viewport = self.content_area_rect();
        let viewport_top = self.scroll_y as f32;
        let viewport_bottom = viewport_top + viewport.height();

        // Find first visible row
        let first = self
            .row_positions
            .iter()
            .enumerate()
            .find(|(i, pos)| {
                let height = self
                    .row_heights
                    .get(*i)
                    .copied()
                    .unwrap_or(self.default_row_height);
                *pos + height >= viewport_top
            })
            .map(|(i, _)| i)
            .unwrap_or(0);

        // Find last visible row
        let last = self
            .row_positions
            .iter()
            .rposition(|pos| *pos <= viewport_bottom)
            .unwrap_or(row_count.saturating_sub(1));

        (first, last)
    }

    fn visible_columns(&self) -> (usize, usize) {
        let col_count = self.column_count();
        if col_count == 0 {
            return (0, 0);
        }

        let viewport = self.content_area_rect();
        let viewport_left = self.scroll_x as f32;
        let viewport_right = viewport_left + viewport.width();

        let mut first = 0;
        let mut last = 0;

        for col in 0..col_count {
            if self.horizontal_header.is_section_hidden(col) {
                continue;
            }

            let col_left = self.horizontal_header.section_position(col);
            let col_width = self.horizontal_header.section_size(col);

            if col_left + col_width >= viewport_left {
                first = col;
                break;
            }
        }

        for col in (0..col_count).rev() {
            if self.horizontal_header.is_section_hidden(col) {
                continue;
            }

            let col_left = self.horizontal_header.section_position(col);
            if col_left <= viewport_right {
                last = col;
                break;
            }
        }

        (first, last)
    }

    fn header_height(&self) -> f32 {
        if self.show_horizontal_header {
            self.horizontal_header.header_size()
        } else {
            0.0
        }
    }

    fn row_header_width(&self) -> f32 {
        if self.show_vertical_header {
            self.vertical_header.header_size()
        } else {
            0.0
        }
    }

    fn content_area_rect(&self) -> Rect {
        let rect = self.base.rect();
        let header_height = self.header_height();
        let row_header_width = self.row_header_width();

        // Account for scrollbars
        let has_v_scrollbar = self.scrollbar_policy_v != ScrollBarPolicy::AlwaysOff
            && self.content_height > rect.height() - header_height;
        let has_h_scrollbar = self.scrollbar_policy_h != ScrollBarPolicy::AlwaysOff
            && self.content_width > rect.width() - row_header_width;

        let scrollbar_width = if has_v_scrollbar {
            self.scrollbar_thickness
        } else {
            0.0
        };
        let scrollbar_height = if has_h_scrollbar {
            self.scrollbar_thickness
        } else {
            0.0
        };

        Rect::new(
            row_header_width,
            header_height,
            rect.width() - row_header_width - scrollbar_width,
            rect.height() - header_height - scrollbar_height,
        )
    }

    fn cell_rect(&self, row: usize, col: usize) -> Option<Rect> {
        if row >= self.row_count() || col >= self.column_count() {
            return None;
        }

        let x = self.horizontal_header.section_position(col);
        let y = self.row_positions.get(row).copied()?;
        let width = self.horizontal_header.section_size(col);
        let height = self
            .row_heights
            .get(row)
            .copied()
            .unwrap_or(self.default_row_height);

        Some(Rect::new(x, y, width, height))
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        ctx.renderer()
            .fill_rect(self.base.rect(), self.background_color);
    }

    fn paint_cells(&self, ctx: &mut PaintContext<'_>) {
        let Some(model) = &self.model else {
            return;
        };

        let (first_row, last_row) = self.visible_rows();
        let (first_col, last_col) = self.visible_columns();
        let content_area = self.content_area_rect();
        let parent = ModelIndex::invalid();

        for row in first_row..=last_row {
            for col in first_col..=last_col {
                if self.horizontal_header.is_section_hidden(col) {
                    continue;
                }

                let Some(cell_rect) = self.cell_rect(row, col) else {
                    continue;
                };

                // Transform to viewport coordinates
                let visual_rect = Rect::new(
                    cell_rect.origin.x - self.scroll_x as f32 + content_area.origin.x,
                    cell_rect.origin.y - self.scroll_y as f32 + content_area.origin.y,
                    cell_rect.width(),
                    cell_rect.height(),
                );

                // Skip if outside content area
                if visual_rect.origin.x + visual_rect.width() < content_area.origin.x
                    || visual_rect.origin.x > content_area.origin.x + content_area.width()
                    || visual_rect.origin.y + visual_rect.height() < content_area.origin.y
                    || visual_rect.origin.y > content_area.origin.y + content_area.height()
                {
                    continue;
                }

                let index = model.index(row, col, &parent);
                let option = self.build_style_option(row, col, visual_rect, &index, model.as_ref());

                let mut delegate_ctx = DelegatePaintContext::new(ctx.renderer(), visual_rect);
                self.delegate.paint(&mut delegate_ctx, &option);
            }
        }
    }

    fn paint_grid(&self, ctx: &mut PaintContext<'_>) {
        if !self.show_grid || self.grid_style == GridStyle::None {
            return;
        }

        let (first_row, last_row) = self.visible_rows();
        let (first_col, last_col) = self.visible_columns();
        let content_area = self.content_area_rect();
        let stroke = Stroke::new(self.grid_color, 1.0);

        // Horizontal lines
        if self.grid_style == GridStyle::Both || self.grid_style == GridStyle::Horizontal {
            for row in first_row..=last_row + 1 {
                let y = self
                    .row_positions
                    .get(row)
                    .copied()
                    .unwrap_or(self.content_height);
                let visual_y = y - self.scroll_y as f32 + content_area.origin.y;

                if visual_y >= content_area.origin.y
                    && visual_y <= content_area.origin.y + content_area.height()
                {
                    ctx.renderer().draw_line(
                        Point::new(content_area.origin.x, visual_y),
                        Point::new(content_area.origin.x + content_area.width(), visual_y),
                        &stroke,
                    );
                }
            }
        }

        // Vertical lines
        if self.grid_style == GridStyle::Both || self.grid_style == GridStyle::Vertical {
            for col in first_col..=last_col + 1 {
                let x = if col < self.column_count() {
                    self.horizontal_header.section_position(col)
                } else {
                    self.content_width
                };
                let visual_x = x - self.scroll_x as f32 + content_area.origin.x;

                if visual_x >= content_area.origin.x
                    && visual_x <= content_area.origin.x + content_area.width()
                {
                    ctx.renderer().draw_line(
                        Point::new(visual_x, content_area.origin.y),
                        Point::new(visual_x, content_area.origin.y + content_area.height()),
                        &stroke,
                    );
                }
            }
        }
    }

    fn paint_headers(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();

        // Horizontal header
        if self.show_horizontal_header {
            let header_rect = Rect::new(
                self.row_header_width(),
                0.0,
                rect.width() - self.row_header_width(),
                self.horizontal_header.header_size(),
            );
            // Note: In a full implementation, we'd render the header here
            // For now, draw a simple background
            ctx.renderer()
                .fill_rect(header_rect, Color::from_rgb8(240, 240, 240));

            // Draw bottom border
            ctx.renderer().fill_rect(
                Rect::new(
                    header_rect.origin.x,
                    header_rect.origin.y + header_rect.height() - 1.0,
                    header_rect.width(),
                    1.0,
                ),
                Color::from_rgb8(200, 200, 200),
            );
        }

        // Vertical header
        if self.show_vertical_header {
            let header_rect = Rect::new(
                0.0,
                self.header_height(),
                self.vertical_header.header_size(),
                rect.height() - self.header_height(),
            );
            ctx.renderer()
                .fill_rect(header_rect, Color::from_rgb8(240, 240, 240));

            // Draw right border
            ctx.renderer().fill_rect(
                Rect::new(
                    header_rect.origin.x + header_rect.width() - 1.0,
                    header_rect.origin.y,
                    1.0,
                    header_rect.height(),
                ),
                Color::from_rgb8(200, 200, 200),
            );
        }

        // Corner widget (if both headers visible)
        if self.show_horizontal_header && self.show_vertical_header {
            let corner_rect = Rect::new(0.0, 0.0, self.row_header_width(), self.header_height());
            ctx.renderer()
                .fill_rect(corner_rect, Color::from_rgb8(230, 230, 230));
        }
    }

    fn paint_scrollbars(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let content_area = self.content_area_rect();

        // Vertical scrollbar
        if self.content_height > content_area.height() {
            let track_rect = Rect::new(
                rect.width() - self.scrollbar_thickness,
                self.header_height(),
                self.scrollbar_thickness,
                rect.height() - self.header_height(),
            );
            ctx.renderer()
                .fill_rect(track_rect, Color::from_rgb8(240, 240, 240));

            // Thumb
            let visible_ratio = content_area.height() / self.content_height;
            let thumb_height = (track_rect.height() * visible_ratio).max(20.0);
            let scroll_ratio = if self.max_scroll_y() > 0 {
                self.scroll_y as f32 / self.max_scroll_y() as f32
            } else {
                0.0
            };
            let thumb_y = track_rect.origin.y + scroll_ratio * (track_rect.height() - thumb_height);

            let thumb_rect = Rect::new(
                track_rect.origin.x + 2.0,
                thumb_y,
                track_rect.width() - 4.0,
                thumb_height,
            );
            ctx.renderer()
                .fill_rect(thumb_rect, Color::from_rgb8(180, 180, 180));
        }

        // Horizontal scrollbar
        if self.content_width > content_area.width() {
            let track_rect = Rect::new(
                self.row_header_width(),
                rect.height() - self.scrollbar_thickness,
                rect.width() - self.row_header_width(),
                self.scrollbar_thickness,
            );
            ctx.renderer()
                .fill_rect(track_rect, Color::from_rgb8(240, 240, 240));

            // Thumb
            let visible_ratio = content_area.width() / self.content_width;
            let thumb_width = (track_rect.width() * visible_ratio).max(20.0);
            let scroll_ratio = if self.max_scroll_x() > 0 {
                self.scroll_x as f32 / self.max_scroll_x() as f32
            } else {
                0.0
            };
            let thumb_x = track_rect.origin.x + scroll_ratio * (track_rect.width() - thumb_width);

            let thumb_rect = Rect::new(
                thumb_x,
                track_rect.origin.y + 2.0,
                thumb_width,
                track_rect.height() - 4.0,
            );
            ctx.renderer()
                .fill_rect(thumb_rect, Color::from_rgb8(180, 180, 180));
        }
    }

    fn build_style_option(
        &self,
        row: usize,
        col: usize,
        rect: Rect,
        index: &ModelIndex,
        model: &dyn ItemModel,
    ) -> StyleOptionViewItem {
        let text = model
            .data(index, ItemRole::Display)
            .as_string()
            .map(|s| s.to_string());
        let icon = model.data(index, ItemRole::Decoration).as_icon().cloned();
        let check_state = model.data(index, ItemRole::CheckState).as_check_state();
        let flags = model.flags(index);

        let is_current = self.selection_model.current_index().row() == row
            && self.selection_model.current_index().column() == col
            && self.selection_model.current_index().is_valid();

        let is_selected = match self.selection_model.selection_behavior() {
            SelectionBehavior::SelectItems => self.selection_model.is_cell_selected(row, col),
            SelectionBehavior::SelectRows => self.selection_model.is_row_selected(row),
            SelectionBehavior::SelectColumns => self.selection_model.is_column_selected(col),
        };

        StyleOptionViewItem {
            rect,
            index: index.clone(),
            state: ViewItemState::new()
                .with_selected(is_selected)
                .with_focused(is_current && self.base.has_focus())
                .with_hovered(self.hovered_cell == Some((row, col)))
                .with_pressed(self.pressed_cell == Some((row, col)))
                .with_enabled(self.base.is_enabled())
                .with_alternate(self.alternate_row_colors && row % 2 == 1),
            features: ViewItemFeatures::default_for_view(),
            flags,
            text,
            icon,
            check_state,
            ..Default::default()
        }
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Check if in content area
        if let Some(index) = self.index_at(event.local_pos) {
            let row = index.row();
            let col = index.column();
            self.pressed_cell = Some((row, col));

            // Selection handling based on behavior
            let mode = self.selection_model.selection_mode();
            let behavior = self.selection_model.selection_behavior();

            let flags = match mode {
                SelectionMode::NoSelection => SelectionFlags::NONE,
                SelectionMode::SingleSelection => {
                    SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor()
                }
                SelectionMode::MultiSelection => {
                    if event.modifiers.control {
                        SelectionFlags::TOGGLE.with_current()
                    } else {
                        SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor()
                    }
                }
                SelectionMode::ExtendedSelection => {
                    if event.modifiers.control {
                        SelectionFlags::TOGGLE.with_current()
                    } else if event.modifiers.shift {
                        // Range selection
                        let anchor = self.selection_model.anchor_index();
                        if anchor.is_valid() {
                            match behavior {
                                SelectionBehavior::SelectItems => {
                                    self.selection_model.select_range_2d(
                                        anchor.row(),
                                        anchor.column(),
                                        row,
                                        col,
                                        SelectionFlags::CLEAR_AND_SELECT,
                                    );
                                }
                                SelectionBehavior::SelectRows => {
                                    self.selection_model.select_range(
                                        anchor.row(),
                                        row,
                                        SelectionFlags::CLEAR_AND_SELECT,
                                    );
                                }
                                SelectionBehavior::SelectColumns => {
                                    // For column selection, we need custom handling
                                    let row_count = self.row_count();
                                    let start_col = anchor.column().min(col);
                                    let end_col = anchor.column().max(col);
                                    self.selection_model.clear_selection();
                                    for c in start_col..=end_col {
                                        self.selection_model.select_column(
                                            c,
                                            row_count,
                                            SelectionFlags::SELECT,
                                        );
                                    }
                                }
                            }
                            self.selection_model
                                .set_current_index(index.clone(), SelectionFlags::CURRENT);
                            self.base.update();
                            return true;
                        }
                        SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor()
                    } else {
                        SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor()
                    }
                }
            };

            // Apply selection based on behavior
            match behavior {
                SelectionBehavior::SelectItems => {
                    self.selection_model.set_current_index(index, flags);
                }
                SelectionBehavior::SelectRows => {
                    let col_count = self.column_count();
                    if flags.clear {
                        self.selection_model.clear_selection();
                    }
                    self.selection_model
                        .select_row(row, col_count, SelectionFlags::SELECT);
                    self.selection_model.set_current_index(
                        ModelIndex::new(row, 0, ModelIndex::invalid()),
                        SelectionFlags::CURRENT.with_anchor(),
                    );
                }
                SelectionBehavior::SelectColumns => {
                    let row_count = self.row_count();
                    if flags.clear {
                        self.selection_model.clear_selection();
                    }
                    self.selection_model
                        .select_column(col, row_count, SelectionFlags::SELECT);
                    self.selection_model.set_current_index(
                        ModelIndex::new(0, col, ModelIndex::invalid()),
                        SelectionFlags::CURRENT.with_anchor(),
                    );
                }
            }

            self.base.update();
            return true;
        }

        // Check for header clicks
        let header_height = self.header_height();
        if self.show_horizontal_header
            && event.local_pos.y < header_height
            && let Some(col) = self.column_at_content_x(
                event.local_pos.x - self.row_header_width() + self.scroll_x as f32,
            )
        {
            self.header_clicked.emit((Orientation::Horizontal, col));

            if self.sorting_enabled {
                // Toggle sort order
                let current_order = self.horizontal_header.sort_indicator_order();
                let new_order = if self.horizontal_header.sort_indicator_section() == Some(col) {
                    match current_order {
                        SortOrder::Ascending => SortOrder::Descending,
                        SortOrder::Descending => SortOrder::Ascending,
                    }
                } else {
                    SortOrder::Ascending
                };
                self.horizontal_header.set_sort_indicator(col, new_order);
            }
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pressed = self.pressed_cell.take();
        self.base.update();

        if let Some((row, col)) = pressed
            && let Some(index) = self.index_at(event.local_pos)
            && index.row() == row
            && index.column() == col
        {
            let emit_index = ModelIndex::new(row, col, ModelIndex::invalid());
            self.clicked.emit(emit_index.clone());

            // Check for double-click
            let now = Instant::now();
            if let (Some(last_time), Some(last_cell)) = (self.last_click_time, self.last_click_cell)
                && last_cell == (row, col)
                && now.duration_since(last_time).as_millis() < 500
            {
                self.double_clicked.emit(emit_index.clone());
                self.activated.emit(emit_index);
                self.last_click_time = None;
                self.last_click_cell = None;
                return true;
            }

            self.last_click_time = Some(now);
            self.last_click_cell = Some((row, col));
        }

        true
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let old_hovered = self.hovered_cell;

        if let Some(index) = self.index_at(event.local_pos) {
            self.hovered_cell = Some((index.row(), index.column()));
        } else {
            self.hovered_cell = None;
        }

        if old_hovered != self.hovered_cell {
            self.base.update();
        }

        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let scroll_amount = (event.delta_y * 0.5).round() as i32;
        let new_y = (self.scroll_y - scroll_amount).clamp(0, self.max_scroll_y());

        if self.scroll_y != new_y {
            self.set_scroll_position(self.scroll_x, new_y);
            return true;
        }
        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let current = self.selection_model.current_index();
        if !current.is_valid() {
            return false;
        }

        let row_count = self.row_count();
        let col_count = self.column_count();

        if row_count == 0 || col_count == 0 {
            return false;
        }

        let current_row = current.row();
        let current_col = current.column();

        match event.key {
            Key::ArrowUp => {
                let new_row = current_row.saturating_sub(1);
                self.move_to_cell(new_row, current_col, &event.modifiers);
                true
            }
            Key::ArrowDown => {
                let new_row = (current_row + 1).min(row_count - 1);
                self.move_to_cell(new_row, current_col, &event.modifiers);
                true
            }
            Key::ArrowLeft => {
                let new_col = current_col.saturating_sub(1);
                self.move_to_cell(current_row, new_col, &event.modifiers);
                true
            }
            Key::ArrowRight => {
                let new_col = (current_col + 1).min(col_count - 1);
                self.move_to_cell(current_row, new_col, &event.modifiers);
                true
            }
            Key::PageUp => {
                let viewport = self.content_area_rect();
                let items_per_page = (viewport.height() / self.default_row_height).floor() as usize;
                let new_row = current_row.saturating_sub(items_per_page.max(1));
                self.move_to_cell(new_row, current_col, &event.modifiers);
                true
            }
            Key::PageDown => {
                let viewport = self.content_area_rect();
                let items_per_page = (viewport.height() / self.default_row_height).floor() as usize;
                let new_row = (current_row + items_per_page.max(1)).min(row_count - 1);
                self.move_to_cell(new_row, current_col, &event.modifiers);
                true
            }
            Key::Home => {
                if event.modifiers.control {
                    self.move_to_cell(0, 0, &event.modifiers);
                } else {
                    self.move_to_cell(current_row, 0, &event.modifiers);
                }
                true
            }
            Key::End => {
                if event.modifiers.control {
                    self.move_to_cell(row_count - 1, col_count - 1, &event.modifiers);
                } else {
                    self.move_to_cell(current_row, col_count - 1, &event.modifiers);
                }
                true
            }
            Key::Enter | Key::NumpadEnter => {
                let index = ModelIndex::new(current_row, current_col, ModelIndex::invalid());
                self.activated.emit(index);
                true
            }
            Key::A if event.modifiers.control => {
                self.select_all();
                true
            }
            _ => false,
        }
    }

    fn move_to_cell(
        &mut self,
        row: usize,
        col: usize,
        modifiers: &crate::widget::KeyboardModifiers,
    ) {
        let index = ModelIndex::new(row, col, ModelIndex::invalid());
        let behavior = self.selection_model.selection_behavior();

        let flags = if modifiers.shift {
            // Extend selection from anchor
            let anchor = self.selection_model.anchor_index();
            if anchor.is_valid() {
                match behavior {
                    SelectionBehavior::SelectItems => {
                        self.selection_model.select_range_2d(
                            anchor.row(),
                            anchor.column(),
                            row,
                            col,
                            SelectionFlags::CLEAR_AND_SELECT,
                        );
                    }
                    SelectionBehavior::SelectRows => {
                        self.selection_model.select_range(
                            anchor.row(),
                            row,
                            SelectionFlags::CLEAR_AND_SELECT,
                        );
                    }
                    SelectionBehavior::SelectColumns => {
                        let row_count = self.row_count();
                        let start_col = anchor.column().min(col);
                        let end_col = anchor.column().max(col);
                        self.selection_model.clear_selection();
                        for c in start_col..=end_col {
                            self.selection_model.select_column(
                                c,
                                row_count,
                                SelectionFlags::SELECT,
                            );
                        }
                    }
                }
            }
            SelectionFlags::CURRENT
        } else if modifiers.control {
            SelectionFlags::CURRENT
        } else {
            SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor()
        };

        self.selection_model.set_current_index(index.clone(), flags);
        self.scroll_to(&index);
        self.base.update();
    }

    fn handle_context_menu(&mut self, event: &ContextMenuEvent) -> bool {
        self.ensure_layout();

        let pos = event.local_pos;
        let header_height = self.header_height();
        let row_header_width = self.row_header_width();

        // Determine which area was clicked
        let location = if self.show_horizontal_header
            && self.show_vertical_header
            && pos.x < row_header_width
            && pos.y < header_height
        {
            // Corner widget area
            TableContextMenuLocation::Corner
        } else if self.show_horizontal_header && pos.y < header_height {
            // Column header area
            let content_x = pos.x - row_header_width + self.scroll_x as f32;
            if let Some(col) = self.column_at_content_x(content_x) {
                TableContextMenuLocation::ColumnHeader(col)
            } else {
                TableContextMenuLocation::Empty
            }
        } else if self.show_vertical_header && pos.x < row_header_width {
            // Row header area
            let content_y = pos.y - header_height + self.scroll_y as f32;
            if let Some(row) = self.row_at_content_y(content_y) {
                TableContextMenuLocation::RowHeader(row)
            } else {
                TableContextMenuLocation::Empty
            }
        } else {
            // Cell area
            if let Some(index) = self.index_at(pos) {
                TableContextMenuLocation::Cell(index)
            } else {
                TableContextMenuLocation::Cell(ModelIndex::invalid())
            }
        };

        // Emit the context_menu_requested signal with the location and position
        self.context_menu_requested.emit((location, pos));

        true
    }
}

impl Object for TableView {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for TableView {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::from_dimensions(400.0, 300.0).with_minimum_dimensions(100.0, 100.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Note: In a real implementation, we'd call ensure_layout() here
        // but that requires &mut self. For now, layout must be up-to-date.
        self.paint_background(ctx);
        self.paint_cells(ctx);
        self.paint_grid(ctx);
        self.paint_drop_indicator(ctx);
        self.paint_headers(ctx);
        self.paint_scrollbars(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        self.ensure_layout();

        match event {
            WidgetEvent::MousePress(e) => {
                if self.handle_mouse_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if self.handle_mouse_release(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseMove(e) => {
                if self.handle_mouse_move(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::Wheel(e) => {
                if self.handle_wheel(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::KeyPress(e) => {
                if self.handle_key_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::ContextMenu(e) => {
                return self.handle_context_menu(e);
            }
            WidgetEvent::DragEnter(e) => {
                if self.handle_drag_enter(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::DragMove(e) => {
                if self.handle_drag_move(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::DragLeave(e) => {
                if self.handle_drag_leave(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::Drop(e) => {
                if self.handle_drop(e) {
                    event.accept();
                    return true;
                }
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
    fn test_table_view_creation() {
        setup();
        let table = TableView::new();
        assert!(table.model().is_none());
        assert_eq!(table.row_count(), 0);
        assert_eq!(table.column_count(), 0);
        assert!(table.show_grid());
    }

    #[test]
    fn test_frozen_sections() {
        setup();
        let mut table = TableView::new();
        assert_eq!(table.frozen_row_count(), 0);
        assert_eq!(table.frozen_column_count(), 0);

        table.set_frozen_row_count(2);
        table.set_frozen_column_count(1);

        assert_eq!(table.frozen_row_count(), 2);
        assert_eq!(table.frozen_column_count(), 1);
    }

    #[test]
    fn test_grid_style() {
        setup();
        let mut table = TableView::new();
        assert_eq!(table.grid_style, GridStyle::Both);

        table.set_grid_style(GridStyle::Horizontal);
        assert_eq!(table.grid_style, GridStyle::Horizontal);

        table.set_show_grid(false);
        assert!(!table.show_grid());
    }

    #[test]
    fn test_selection_behavior() {
        setup();
        let table = TableView::new();
        assert_eq!(
            table.selection_model().selection_behavior(),
            SelectionBehavior::SelectRows
        );
    }

    #[test]
    fn test_context_menu_signal() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        setup();
        let table = TableView::new();
        let signal_received = Arc::new(AtomicBool::new(false));
        let received_clone = signal_received.clone();

        // Connect to the context menu signal
        table.context_menu_requested.connect(move |_| {
            received_clone.store(true, Ordering::SeqCst);
        });

        // Emit a test signal for a cell location
        table.context_menu_requested.emit((
            TableContextMenuLocation::Cell(ModelIndex::invalid()),
            Point::new(10.0, 10.0),
        ));

        assert!(signal_received.load(Ordering::SeqCst));
    }

    #[test]
    fn test_context_menu_location_variants() {
        // Test that all location variants can be created
        let cell = TableContextMenuLocation::Cell(ModelIndex::invalid());
        let col = TableContextMenuLocation::ColumnHeader(0);
        let row = TableContextMenuLocation::RowHeader(0);
        let corner = TableContextMenuLocation::Corner;
        let empty = TableContextMenuLocation::Empty;

        // Basic equality checks
        assert_eq!(cell, TableContextMenuLocation::Cell(ModelIndex::invalid()));
        assert_eq!(col, TableContextMenuLocation::ColumnHeader(0));
        assert_eq!(row, TableContextMenuLocation::RowHeader(0));
        assert_eq!(corner, TableContextMenuLocation::Corner);
        assert_eq!(empty, TableContextMenuLocation::Empty);
    }

    #[test]
    fn test_drag_drop_mode_default() {
        setup();
        let table = TableView::new();
        assert_eq!(table.drag_drop_mode(), TableDragDropMode::NoDragDrop);
        assert!(!table.drag_enabled());
        assert!(!table.drop_enabled());
    }

    #[test]
    fn test_drag_drop_mode_setter() {
        setup();
        let mut table = TableView::new();

        table.set_drag_drop_mode(TableDragDropMode::DragOnly);
        assert_eq!(table.drag_drop_mode(), TableDragDropMode::DragOnly);
        assert!(table.drag_enabled());
        assert!(!table.drop_enabled());

        table.set_drag_drop_mode(TableDragDropMode::DropOnly);
        assert_eq!(table.drag_drop_mode(), TableDragDropMode::DropOnly);
        assert!(!table.drag_enabled());
        assert!(table.drop_enabled());

        table.set_drag_drop_mode(TableDragDropMode::DragDrop);
        assert_eq!(table.drag_drop_mode(), TableDragDropMode::DragDrop);
        assert!(table.drag_enabled());
        assert!(table.drop_enabled());

        table.set_drag_drop_mode(TableDragDropMode::InternalMove);
        assert_eq!(table.drag_drop_mode(), TableDragDropMode::InternalMove);
        assert!(table.drag_enabled());
        assert!(table.drop_enabled());
    }

    #[test]
    fn test_drag_drop_mode_builder() {
        setup();
        let table = TableView::new().with_drag_drop_mode(TableDragDropMode::DragDrop);
        assert_eq!(table.drag_drop_mode(), TableDragDropMode::DragDrop);
        assert!(table.drag_enabled());
        assert!(table.drop_enabled());
    }

    #[test]
    fn test_drop_indicator_initial_state() {
        setup();
        let table = TableView::new();
        // Drop indicator should not be active initially
        assert!(!table.drop_indicator_state.has_indicator());
    }
}
