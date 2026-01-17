//! ListView widget for displaying items from a model.
//!
//! This module provides [`ListView`], a view widget that displays items from
//! an [`ItemModel`] in either a vertical list or icon grid layout.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::model::{ListModel, ListItem, ItemData, DefaultItemDelegate};
//! use horizon_lattice::widget::widgets::{ListView, ListViewMode};
//! use std::sync::Arc;
//!
//! // Create a model
//! let items = vec!["Apple", "Banana", "Cherry"];
//! let model = Arc::new(ListModel::new(items.into_iter().map(String::from).collect()));
//!
//! // Create list view
//! let mut list_view = ListView::new()
//!     .with_model(model)
//!     .with_view_mode(ListViewMode::ListMode);
//!
//! // Connect to signals
//! list_view.clicked.connect(|index| {
//!     println!("Clicked row {}", index.row());
//! });
//! ```

use std::sync::Arc;
use std::time::Instant;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Size};

use crate::model::{
    DefaultItemDelegate, DelegatePaintContext, ItemDelegate, ItemModel, ItemRole,
    ModelIndex, StyleOptionViewItem, ViewItemFeatures, ViewItemState,
};
use crate::model::selection::{SelectionFlags, SelectionMode, SelectionModel};
use crate::widget::{
    ContextMenuEvent, FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent,
    MousePressEvent, MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair,
    WheelEvent, Widget, WidgetBase, WidgetEvent,
};
use crate::widget::drag_drop::{
    DragData, DragEnterEvent, DragMoveEvent, DragLeaveEvent, DropEvent, DropAction,
    DropIndicatorState, DropPosition,
};

use super::scroll_area::ScrollBarPolicy;

/// Display mode for ListView.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListViewMode {
    /// Vertical list with one item per row.
    #[default]
    ListMode,
    /// Icon/grid mode with items arranged in a grid.
    IconMode,
}

/// Flow direction for icon mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Flow {
    /// Items flow left to right, then wrap to next row.
    #[default]
    LeftToRight,
    /// Items flow top to bottom, then wrap to next column.
    TopToBottom,
}

/// Drag and drop mode for ListView.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DragDropMode {
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

/// A view widget that displays items from an ItemModel.
///
/// ListView provides:
/// - Vertical list mode (one item per row)
/// - Icon/grid mode (items arranged in a grid)
/// - Built-in scrolling with scrollbars
/// - Multiple selection modes
/// - Keyboard and mouse navigation
/// - Custom rendering via ItemDelegate
///
/// # Signals
///
/// - `clicked(ModelIndex)`: Emitted when an item is clicked
/// - `double_clicked(ModelIndex)`: Emitted when an item is double-clicked
/// - `activated(ModelIndex)`: Emitted when Enter is pressed or item is double-clicked
pub struct ListView {
    // Widget base
    base: WidgetBase,

    // Model/View
    model: Option<Arc<dyn ItemModel>>,
    selection_model: SelectionModel,
    delegate: Arc<dyn ItemDelegate>,

    // View mode
    view_mode: ListViewMode,
    flow: Flow,

    // Layout
    spacing: f32,
    grid_size: Option<Size>,
    uniform_item_sizes: bool,
    default_item_height: f32,

    // Scrolling
    scroll_x: i32,
    scroll_y: i32,
    content_size: Size,
    scrollbar_policy_h: ScrollBarPolicy,
    scrollbar_policy_v: ScrollBarPolicy,
    scrollbar_thickness: f32,

    // Item layout cache
    item_rects: Vec<Rect>,
    layout_dirty: bool,

    // Visual state
    hovered_row: Option<usize>,
    pressed_row: Option<usize>,
    last_click_time: Option<Instant>,
    last_click_row: Option<usize>,

    // Appearance
    background_color: Color,
    alternate_row_colors: bool,

    // Drag and drop
    drag_drop_mode: DragDropMode,
    drop_indicator_state: DropIndicatorState,
    drag_start_pos: Option<Point>,
    dragging_row: Option<usize>,

    // Signals
    /// Emitted when an item is clicked.
    pub clicked: Signal<ModelIndex>,
    /// Emitted when an item is double-clicked.
    pub double_clicked: Signal<ModelIndex>,
    /// Emitted when an item is activated (double-click or Enter).
    pub activated: Signal<ModelIndex>,
    /// Emitted when a context menu is requested.
    ///
    /// The tuple contains (index at position or invalid, position in widget coords).
    /// If the context menu was requested over an item, the index will be valid.
    /// If requested over empty space, the index will be invalid.
    pub context_menu_requested: Signal<(ModelIndex, Point)>,
}

impl Default for ListView {
    fn default() -> Self {
        Self::new()
    }
}

impl ListView {
    /// Creates a new empty ListView.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Expanding));

        Self {
            base,
            model: None,
            selection_model: SelectionModel::new(),
            delegate: Arc::new(DefaultItemDelegate::new()),
            view_mode: ListViewMode::ListMode,
            flow: Flow::LeftToRight,
            spacing: 2.0,
            grid_size: None,
            uniform_item_sizes: true,
            default_item_height: 24.0,
            scroll_x: 0,
            scroll_y: 0,
            content_size: Size::new(0.0, 0.0),
            scrollbar_policy_h: ScrollBarPolicy::AsNeeded,
            scrollbar_policy_v: ScrollBarPolicy::AsNeeded,
            scrollbar_thickness: 12.0,
            item_rects: Vec::new(),
            layout_dirty: true,
            hovered_row: None,
            pressed_row: None,
            last_click_time: None,
            last_click_row: None,
            background_color: Color::WHITE,
            alternate_row_colors: false,
            drag_drop_mode: DragDropMode::NoDragDrop,
            drop_indicator_state: DropIndicatorState::new(),
            drag_start_pos: None,
            dragging_row: None,
            clicked: Signal::new(),
            double_clicked: Signal::new(),
            activated: Signal::new(),
            context_menu_requested: Signal::new(),
        }
    }

    /// Creates a ListView with the given model.
    pub fn with_model(mut self, model: Arc<dyn ItemModel>) -> Self {
        self.set_model(Some(model));
        self
    }

    /// Sets the view mode using builder pattern.
    pub fn with_view_mode(mut self, mode: ListViewMode) -> Self {
        self.view_mode = mode;
        self.layout_dirty = true;
        self
    }

    /// Sets the selection mode using builder pattern.
    pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_model.set_selection_mode(mode);
        self
    }

    /// Sets the delegate using builder pattern.
    pub fn with_delegate(mut self, delegate: Arc<dyn ItemDelegate>) -> Self {
        self.delegate = delegate;
        self.layout_dirty = true;
        self
    }

    /// Sets the spacing using builder pattern.
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self.layout_dirty = true;
        self
    }

    /// Sets the grid size for icon mode using builder pattern.
    pub fn with_grid_size(mut self, size: Size) -> Self {
        self.grid_size = Some(size);
        self.layout_dirty = true;
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
        self.model = model;
        self.selection_model.reset();
        self.layout_dirty = true;
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

    /// Gets the selected indices.
    pub fn selected_indices(&self) -> &[ModelIndex] {
        self.selection_model.selected_indices()
    }

    /// Clears the selection.
    pub fn clear_selection(&mut self) {
        self.selection_model.clear_selection();
        self.base.update();
    }

    /// Selects all items.
    pub fn select_all(&mut self) {
        if let Some(model) = &self.model {
            let row_count = model.row_count(&ModelIndex::invalid());
            self.selection_model.select_all(row_count);
            self.base.update();
        }
    }

    // =========================================================================
    // View Mode
    // =========================================================================

    /// Gets the current view mode.
    pub fn view_mode(&self) -> ListViewMode {
        self.view_mode
    }

    /// Sets the view mode.
    pub fn set_view_mode(&mut self, mode: ListViewMode) {
        if self.view_mode != mode {
            self.view_mode = mode;
            self.layout_dirty = true;
            self.base.update();
        }
    }

    /// Gets the flow direction for icon mode.
    pub fn flow(&self) -> Flow {
        self.flow
    }

    /// Sets the flow direction for icon mode.
    pub fn set_flow(&mut self, flow: Flow) {
        if self.flow != flow {
            self.flow = flow;
            self.layout_dirty = true;
            self.base.update();
        }
    }

    // =========================================================================
    // Delegate
    // =========================================================================

    /// Gets the current delegate.
    pub fn delegate(&self) -> &Arc<dyn ItemDelegate> {
        &self.delegate
    }

    /// Sets the delegate.
    pub fn set_delegate(&mut self, delegate: Arc<dyn ItemDelegate>) {
        self.delegate = delegate;
        self.layout_dirty = true;
        self.base.update();
    }

    // =========================================================================
    // Layout
    // =========================================================================

    /// Gets the spacing between items.
    pub fn spacing(&self) -> f32 {
        self.spacing
    }

    /// Sets the spacing between items.
    pub fn set_spacing(&mut self, spacing: f32) {
        if (self.spacing - spacing).abs() > f32::EPSILON {
            self.spacing = spacing;
            self.layout_dirty = true;
            self.base.update();
        }
    }

    /// Gets the grid size for icon mode.
    pub fn grid_size(&self) -> Option<Size> {
        self.grid_size
    }

    /// Sets the grid size for icon mode.
    pub fn set_grid_size(&mut self, size: Option<Size>) {
        self.grid_size = size;
        self.layout_dirty = true;
        self.base.update();
    }

    /// Sets whether to use uniform item sizes (performance optimization).
    pub fn set_uniform_item_sizes(&mut self, uniform: bool) {
        self.uniform_item_sizes = uniform;
        self.layout_dirty = true;
        self.base.update();
    }

    // =========================================================================
    // Scrolling
    // =========================================================================

    /// Gets the horizontal scroll position.
    pub fn scroll_x(&self) -> i32 {
        self.scroll_x
    }

    /// Gets the vertical scroll position.
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
        if let Some(rect) = self.item_rects.get(row) {
            let viewport = self.viewport_rect();
            let item_top = rect.origin.y as i32;
            let item_bottom = (rect.origin.y + rect.height()) as i32;
            let viewport_top = self.scroll_y;
            let viewport_bottom = self.scroll_y + viewport.height() as i32;

            if item_top < viewport_top {
                self.scroll_y = item_top;
            } else if item_bottom > viewport_bottom {
                self.scroll_y = item_bottom - viewport.height() as i32;
            }

            self.scroll_y = self.scroll_y.clamp(0, self.max_scroll_y());
            self.base.update();
        }
    }

    /// Scrolls to the top.
    pub fn scroll_to_top(&mut self) {
        if self.scroll_y != 0 {
            self.scroll_y = 0;
            self.base.update();
        }
    }

    /// Scrolls to the bottom.
    pub fn scroll_to_bottom(&mut self) {
        let max = self.max_scroll_y();
        if self.scroll_y != max {
            self.scroll_y = max;
            self.base.update();
        }
    }

    /// Ensures the given index is visible.
    pub fn ensure_visible(&mut self, index: &ModelIndex) {
        self.scroll_to(index);
    }

    fn max_scroll_x(&self) -> i32 {
        let viewport = self.viewport_rect();
        (self.content_size.width - viewport.width()).max(0.0) as i32
    }

    fn max_scroll_y(&self) -> i32 {
        let viewport = self.viewport_rect();
        (self.content_size.height - viewport.height()).max(0.0) as i32
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Sets whether to use alternating row colors.
    pub fn set_alternate_row_colors(&mut self, enabled: bool) {
        if self.alternate_row_colors != enabled {
            self.alternate_row_colors = enabled;
            self.base.update();
        }
    }

    /// Sets the background color.
    pub fn set_background_color(&mut self, color: Color) {
        if self.background_color != color {
            self.background_color = color;
            self.base.update();
        }
    }

    // =========================================================================
    // Drag and Drop
    // =========================================================================

    /// Gets the current drag and drop mode.
    pub fn drag_drop_mode(&self) -> DragDropMode {
        self.drag_drop_mode
    }

    /// Sets the drag and drop mode.
    ///
    /// # Example
    ///
    /// ```ignore
    /// list_view.set_drag_drop_mode(DragDropMode::DragDrop);
    /// ```
    pub fn set_drag_drop_mode(&mut self, mode: DragDropMode) {
        if self.drag_drop_mode != mode {
            self.drag_drop_mode = mode;
            // Enable drops on the base widget if needed
            let accepts_drops = matches!(
                mode,
                DragDropMode::DropOnly | DragDropMode::DragDrop | DragDropMode::InternalMove
            );
            self.base.set_accepts_drops(accepts_drops);
        }
    }

    /// Returns whether dragging from this view is enabled.
    pub fn drag_enabled(&self) -> bool {
        matches!(
            self.drag_drop_mode,
            DragDropMode::DragOnly | DragDropMode::DragDrop | DragDropMode::InternalMove
        )
    }

    /// Returns whether dropping onto this view is enabled.
    pub fn drop_enabled(&self) -> bool {
        matches!(
            self.drag_drop_mode,
            DragDropMode::DropOnly | DragDropMode::DragDrop | DragDropMode::InternalMove
        )
    }

    /// Creates drag data from the selected items.
    ///
    /// Override this method in subclasses to customize the drag data.
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
                    model.data(index, ItemRole::Display).as_string().map(String::from)
                })
                .collect();

            if !texts.is_empty() {
                data.set_text(&texts.join("\n"));
            }
        }

        if data.is_empty() {
            None
        } else {
            Some(data)
        }
    }

    /// Returns the drop action for the given position.
    ///
    /// This determines where items will be inserted when dropped.
    fn drop_position_for_point(&self, point: Point) -> (Option<usize>, DropPosition) {
        let content_x = point.x + self.scroll_x as f32;
        let content_y = point.y + self.scroll_y as f32;
        let content_point = Point::new(content_x, content_y);

        for (row, rect) in self.item_rects.iter().enumerate() {
            if rect.contains(content_point) {
                // Determine if above or below midpoint
                let mid_y = rect.origin.y + rect.height() / 2.0;
                if content_y < mid_y {
                    return (Some(row), DropPosition::AboveItem);
                } else {
                    return (Some(row), DropPosition::BelowItem);
                }
            }
        }

        // After last item
        if let Some(_) = self.item_rects.last() {
            let row_count = self.item_rects.len();
            return (Some(row_count.saturating_sub(1)), DropPosition::BelowItem);
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
        let viewport = self.viewport_rect();
        if viewport.contains(event.local_pos) {
            // Get visible item rects for drop indicator calculation
            let (first, last) = self.visible_range();
            let item_rects: Vec<(usize, Rect)> = (first..=last)
                .filter_map(|row| {
                    self.item_rects.get(row).map(|r| {
                        (row, Rect::new(
                            r.origin.x - self.scroll_x as f32,
                            r.origin.y - self.scroll_y as f32,
                            r.width(),
                            r.height(),
                        ))
                    })
                })
                .collect();

            self.drop_indicator_state.update_for_vertical_list(
                event.local_pos,
                &item_rects,
                viewport.width(),
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

        let (_drop_row, _drop_position) = self.drop_position_for_point(event.local_pos);
        self.drop_indicator_state.clear();
        self.base.update();

        // Process the dropped data
        if event.data().has_text() || event.data().has_urls() {
            // For now, just accept the drop
            // In a real implementation, this would insert items into the model
            event.accept();
            return true;
        }

        false
    }

    /// Paints the drop indicator if active.
    fn paint_drop_indicator(&self, ctx: &mut PaintContext<'_>) {
        if let Some(indicator) = self.drop_indicator_state.indicator() {
            let style = self.drop_indicator_state.style();

            // For all indicator types, just fill with the line color
            // This is a simplified implementation - more sophisticated rendering
            // can be added later
            ctx.renderer().fill_rect(indicator.rect, style.line_color);
        }
    }

    // =========================================================================
    // Queries
    // =========================================================================

    /// Returns the index at the given point in widget coordinates.
    pub fn index_at(&self, point: Point) -> Option<ModelIndex> {
        if !self.viewport_rect().contains(point) {
            return None;
        }

        // Convert to content coordinates
        let content_x = point.x + self.scroll_x as f32;
        let content_y = point.y + self.scroll_y as f32;
        let content_point = Point::new(content_x, content_y);

        for (row, rect) in self.item_rects.iter().enumerate() {
            if rect.contains(content_point) {
                return Some(ModelIndex::new(row, 0, ModelIndex::invalid()));
            }
        }

        None
    }

    /// Returns the visual rectangle for an index in widget coordinates.
    pub fn visual_rect(&self, index: &ModelIndex) -> Option<Rect> {
        if !index.is_valid() {
            return None;
        }

        self.item_rects.get(index.row()).map(|r| {
            Rect::new(
                r.origin.x - self.scroll_x as f32,
                r.origin.y - self.scroll_y as f32,
                r.width(),
                r.height(),
            )
        })
    }

    /// Returns the viewport rectangle.
    pub fn viewport_rect(&self) -> Rect {
        let rect = self.base.rect();
        let h_visible = self.is_horizontal_scrollbar_visible();
        let v_visible = self.is_vertical_scrollbar_visible();

        let width = if v_visible {
            rect.width() - self.scrollbar_thickness
        } else {
            rect.width()
        };

        let height = if h_visible {
            rect.height() - self.scrollbar_thickness
        } else {
            rect.height()
        };

        Rect::new(0.0, 0.0, width.max(0.0), height.max(0.0))
    }

    /// Checks if an index is visible in the viewport.
    pub fn is_index_visible(&self, index: &ModelIndex) -> bool {
        if let Some(rect) = self.visual_rect(index) {
            let viewport = self.viewport_rect();
            rects_intersect(rect, viewport)
        } else {
            false
        }
    }

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    fn ensure_layout(&mut self) {
        if self.layout_dirty {
            self.update_layout();
        }
    }

    fn update_layout(&mut self) {
        match self.view_mode {
            ListViewMode::ListMode => self.layout_list_mode(),
            ListViewMode::IconMode => self.layout_icon_mode(),
        }
        self.layout_dirty = false;
    }

    fn layout_list_mode(&mut self) {
        let Some(model) = &self.model else {
            self.item_rects.clear();
            self.content_size = Size::new(0.0, 0.0);
            return;
        };

        let row_count = model.row_count(&ModelIndex::invalid());
        let viewport_width = self.viewport_rect().width();

        self.item_rects.clear();
        self.item_rects.reserve(row_count);

        let mut y = 0.0;

        for row in 0..row_count {
            let height = if self.uniform_item_sizes {
                self.default_item_height
            } else {
                self.calculate_item_height(row)
            };

            let rect = Rect::new(0.0, y, viewport_width, height);
            self.item_rects.push(rect);
            y += height + self.spacing;
        }

        self.content_size = Size::new(viewport_width, (y - self.spacing).max(0.0));
    }

    fn layout_icon_mode(&mut self) {
        let Some(model) = &self.model else {
            self.item_rects.clear();
            self.content_size = Size::new(0.0, 0.0);
            return;
        };

        let row_count = model.row_count(&ModelIndex::invalid());
        let viewport = self.viewport_rect();
        let viewport_width = viewport.width();
        let viewport_height = viewport.height();

        let cell_size = self.grid_size.unwrap_or(Size::new(80.0, 80.0));

        self.item_rects.clear();
        self.item_rects.reserve(row_count);

        match self.flow {
            Flow::LeftToRight => {
                let items_per_row = ((viewport_width + self.spacing)
                    / (cell_size.width + self.spacing))
                    .floor()
                    .max(1.0) as usize;

                for i in 0..row_count {
                    let col = i % items_per_row;
                    let row = i / items_per_row;
                    let x = col as f32 * (cell_size.width + self.spacing);
                    let y = row as f32 * (cell_size.height + self.spacing);
                    self.item_rects
                        .push(Rect::new(x, y, cell_size.width, cell_size.height));
                }

                let num_rows = (row_count + items_per_row - 1) / items_per_row;
                self.content_size = Size::new(
                    viewport_width,
                    (num_rows as f32 * (cell_size.height + self.spacing) - self.spacing).max(0.0),
                );
            }
            Flow::TopToBottom => {
                let items_per_col = ((viewport_height + self.spacing)
                    / (cell_size.height + self.spacing))
                    .floor()
                    .max(1.0) as usize;

                for i in 0..row_count {
                    let row = i % items_per_col;
                    let col = i / items_per_col;
                    let x = col as f32 * (cell_size.width + self.spacing);
                    let y = row as f32 * (cell_size.height + self.spacing);
                    self.item_rects
                        .push(Rect::new(x, y, cell_size.width, cell_size.height));
                }

                let num_cols = (row_count + items_per_col - 1) / items_per_col;
                self.content_size = Size::new(
                    (num_cols as f32 * (cell_size.width + self.spacing) - self.spacing).max(0.0),
                    viewport_height,
                );
            }
        }
    }

    fn calculate_item_height(&self, row: usize) -> f32 {
        if let Some(model) = &self.model {
            let index = model.index(row, 0, &ModelIndex::invalid());
            let option = self.build_style_option(row, Rect::new(0.0, 0.0, 200.0, 0.0));
            let (_width, height) = self.delegate.size_hint(&option);
            let _ = index; // Use index to query model if needed
            height
        } else {
            self.default_item_height
        }
    }

    fn visible_range(&self) -> (usize, usize) {
        if self.item_rects.is_empty() {
            return (0, 0);
        }

        let viewport_top = self.scroll_y as f32;
        let viewport_bottom = viewport_top + self.viewport_rect().height();

        let first = self
            .item_rects
            .iter()
            .position(|r| r.origin.y + r.height() >= viewport_top)
            .unwrap_or(0);

        let last = self
            .item_rects
            .iter()
            .rposition(|r| r.origin.y <= viewport_bottom)
            .unwrap_or(self.item_rects.len().saturating_sub(1));

        (first, last)
    }

    // =========================================================================
    // Scrollbar Helpers
    // =========================================================================

    fn is_horizontal_scrollbar_visible(&self) -> bool {
        match self.scrollbar_policy_h {
            ScrollBarPolicy::AlwaysOn => true,
            ScrollBarPolicy::AlwaysOff => false,
            ScrollBarPolicy::AsNeeded => {
                let rect = self.base.rect();
                self.content_size.width > rect.width()
            }
        }
    }

    fn is_vertical_scrollbar_visible(&self) -> bool {
        match self.scrollbar_policy_v {
            ScrollBarPolicy::AlwaysOn => true,
            ScrollBarPolicy::AlwaysOff => false,
            ScrollBarPolicy::AsNeeded => {
                let rect = self.base.rect();
                self.content_size.height > rect.height()
            }
        }
    }

    fn vertical_scrollbar_rect(&self) -> Option<Rect> {
        if !self.is_vertical_scrollbar_visible() {
            return None;
        }
        let rect = self.base.rect();
        let h_visible = self.is_horizontal_scrollbar_visible();
        let height = if h_visible {
            rect.height() - self.scrollbar_thickness
        } else {
            rect.height()
        };

        Some(Rect::new(
            rect.width() - self.scrollbar_thickness,
            0.0,
            self.scrollbar_thickness,
            height.max(0.0),
        ))
    }

    fn horizontal_scrollbar_rect(&self) -> Option<Rect> {
        if !self.is_horizontal_scrollbar_visible() {
            return None;
        }
        let rect = self.base.rect();
        let v_visible = self.is_vertical_scrollbar_visible();
        let width = if v_visible {
            rect.width() - self.scrollbar_thickness
        } else {
            rect.width()
        };

        Some(Rect::new(
            0.0,
            rect.height() - self.scrollbar_thickness,
            width.max(0.0),
            self.scrollbar_thickness,
        ))
    }

    // =========================================================================
    // Style Option Building
    // =========================================================================

    fn build_style_option(&self, row: usize, rect: Rect) -> StyleOptionViewItem {
        let index = ModelIndex::new(row, 0, ModelIndex::invalid());

        let mut text = None;
        let mut icon = None;
        let mut check_state = None;
        let mut flags = crate::model::ItemFlags::new();

        if let Some(model) = &self.model {
            let model_index = model.index(row, 0, &ModelIndex::invalid());
            text = model.data(&model_index, ItemRole::Display).as_string().map(|s| s.to_string());
            icon = model.data(&model_index, ItemRole::Decoration).as_icon().cloned();
            check_state = model.data(&model_index, ItemRole::CheckState).as_check_state();
            flags = model.flags(&model_index);
        }

        let is_current = self.selection_model.current_index().row() == row
            && self.selection_model.current_index().is_valid();

        StyleOptionViewItem {
            rect,
            index,
            state: ViewItemState::new()
                .with_selected(self.selection_model.is_row_selected(row))
                .with_focused(is_current && self.base.has_focus())
                .with_hovered(self.hovered_row == Some(row))
                .with_pressed(self.pressed_row == Some(row))
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
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Check scrollbar clicks
        if let Some(rect) = self.vertical_scrollbar_rect() {
            if rect.contains(event.local_pos) {
                return self.handle_scrollbar_click(event.local_pos, false);
            }
        }
        if let Some(rect) = self.horizontal_scrollbar_rect() {
            if rect.contains(event.local_pos) {
                return self.handle_scrollbar_click(event.local_pos, true);
            }
        }

        // Check item click
        self.ensure_layout();

        if let Some(index) = self.index_at(event.local_pos) {
            let row = index.row();
            self.pressed_row = Some(row);

            // Handle selection based on modifiers
            let mode = self.selection_model.selection_mode();
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
                        // Range selection from anchor
                        let anchor_row = self.selection_model.anchor_index().row();
                        self.selection_model
                            .select_range(anchor_row, row, SelectionFlags::CLEAR_AND_SELECT);
                        self.selection_model
                            .set_current_index(index, SelectionFlags::CURRENT);
                        self.base.update();
                        return true;
                    } else {
                        SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor()
                    }
                }
            };

            if flags.select || flags.toggle || flags.clear {
                self.selection_model.set_current_index(index, flags);
            }

            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pressed = self.pressed_row.take();
        self.base.update();

        // Emit clicked signal
        if let Some(row) = pressed {
            if let Some(click_index) = self.index_at(event.local_pos) {
                if click_index.row() == row {
                    let index = ModelIndex::new(row, 0, ModelIndex::invalid());
                    self.clicked.emit(index.clone());

                    // Check for double-click
                    let now = Instant::now();
                    if let (Some(last_time), Some(last_row)) =
                        (self.last_click_time, self.last_click_row)
                    {
                        if last_row == row && now.duration_since(last_time).as_millis() < 500 {
                            self.double_clicked.emit(index.clone());
                            self.activated.emit(index);
                            self.last_click_time = None;
                            self.last_click_row = None;
                            return true;
                        }
                    }

                    self.last_click_time = Some(now);
                    self.last_click_row = Some(row);
                }
            }
        }

        true
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let old_hovered = self.hovered_row;

        if self.viewport_rect().contains(event.local_pos) {
            self.hovered_row = self.index_at(event.local_pos).map(|idx| idx.row());
        } else {
            self.hovered_row = None;
        }

        if old_hovered != self.hovered_row {
            self.base.update();
        }

        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let scroll_amount = (event.delta_y * 0.5).round() as i32;
        let new_y = (self.scroll_y - scroll_amount).clamp(0, self.max_scroll_y());

        if self.scroll_y != new_y {
            self.scroll_y = new_y;
            self.base.update();
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let current_row = if self.selection_model.current_index().is_valid() {
            self.selection_model.current_index().row()
        } else {
            return false;
        };

        let row_count = self
            .model
            .as_ref()
            .map(|m| m.row_count(&ModelIndex::invalid()))
            .unwrap_or(0);

        if row_count == 0 {
            return false;
        }

        match event.key {
            Key::ArrowUp => {
                let new_row = current_row.saturating_sub(1);
                self.move_to_row(new_row, &event.modifiers);
                true
            }
            Key::ArrowDown => {
                let new_row = (current_row + 1).min(row_count - 1);
                self.move_to_row(new_row, &event.modifiers);
                true
            }
            Key::ArrowLeft => {
                if self.view_mode == ListViewMode::IconMode {
                    let new_row = current_row.saturating_sub(1);
                    self.move_to_row(new_row, &event.modifiers);
                    true
                } else {
                    false
                }
            }
            Key::ArrowRight => {
                if self.view_mode == ListViewMode::IconMode {
                    let new_row = (current_row + 1).min(row_count - 1);
                    self.move_to_row(new_row, &event.modifiers);
                    true
                } else {
                    false
                }
            }
            Key::PageUp => {
                let viewport_height = self.viewport_rect().height();
                let items_per_page =
                    (viewport_height / self.default_item_height).floor() as usize;
                let new_row = current_row.saturating_sub(items_per_page.max(1));
                self.move_to_row(new_row, &event.modifiers);
                true
            }
            Key::PageDown => {
                let viewport_height = self.viewport_rect().height();
                let items_per_page =
                    (viewport_height / self.default_item_height).floor() as usize;
                let new_row = (current_row + items_per_page.max(1)).min(row_count - 1);
                self.move_to_row(new_row, &event.modifiers);
                true
            }
            Key::Home => {
                self.move_to_row(0, &event.modifiers);
                true
            }
            Key::End => {
                self.move_to_row(row_count - 1, &event.modifiers);
                true
            }
            Key::Space => {
                // Toggle selection of current item
                let index = ModelIndex::new(current_row, 0, ModelIndex::invalid());
                self.selection_model.select(index, SelectionFlags::TOGGLE);
                self.base.update();
                true
            }
            Key::Enter | Key::NumpadEnter => {
                let index = ModelIndex::new(current_row, 0, ModelIndex::invalid());
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

    fn move_to_row(&mut self, row: usize, modifiers: &crate::widget::KeyboardModifiers) {
        let index = ModelIndex::new(row, 0, ModelIndex::invalid());
        let mode = self.selection_model.selection_mode();

        let flags = match mode {
            SelectionMode::NoSelection => SelectionFlags::CURRENT,
            SelectionMode::SingleSelection => SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor(),
            SelectionMode::MultiSelection | SelectionMode::ExtendedSelection => {
                if modifiers.shift {
                    let anchor_row = self.selection_model.anchor_index().row();
                    self.selection_model
                        .select_range(anchor_row, row, SelectionFlags::CLEAR_AND_SELECT);
                    SelectionFlags::CURRENT
                } else if modifiers.control {
                    SelectionFlags::CURRENT
                } else {
                    SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor()
                }
            }
        };

        self.selection_model.set_current_index(index.clone(), flags);
        self.scroll_to(&index);
        self.base.update();
    }

    fn handle_scrollbar_click(&mut self, pos: Point, horizontal: bool) -> bool {
        if horizontal {
            if let Some(rect) = self.horizontal_scrollbar_rect() {
                let viewport = self.viewport_rect();
                let thumb_ratio = viewport.width() / self.content_size.width.max(1.0);
                let thumb_width = (rect.width() * thumb_ratio).max(20.0).min(rect.width());
                let available_travel = rect.width() - thumb_width;
                let max_scroll = self.max_scroll_x() as f32;

                if available_travel > 0.0 && max_scroll > 0.0 {
                    let thumb_pos = (self.scroll_x as f32 / max_scroll) * available_travel;
                    let click_pos = pos.x - rect.origin.x;

                    if click_pos < thumb_pos {
                        self.set_scroll_position(
                            self.scroll_x - viewport.width() as i32,
                            self.scroll_y,
                        );
                    } else if click_pos > thumb_pos + thumb_width {
                        self.set_scroll_position(
                            self.scroll_x + viewport.width() as i32,
                            self.scroll_y,
                        );
                    }
                }
                return true;
            }
        } else {
            if let Some(rect) = self.vertical_scrollbar_rect() {
                let viewport = self.viewport_rect();
                let thumb_ratio = viewport.height() / self.content_size.height.max(1.0);
                let thumb_height = (rect.height() * thumb_ratio).max(20.0).min(rect.height());
                let available_travel = rect.height() - thumb_height;
                let max_scroll = self.max_scroll_y() as f32;

                if available_travel > 0.0 && max_scroll > 0.0 {
                    let thumb_pos = (self.scroll_y as f32 / max_scroll) * available_travel;
                    let click_pos = pos.y - rect.origin.y;

                    if click_pos < thumb_pos {
                        self.set_scroll_position(
                            self.scroll_x,
                            self.scroll_y - viewport.height() as i32,
                        );
                    } else if click_pos > thumb_pos + thumb_height {
                        self.set_scroll_position(
                            self.scroll_x,
                            self.scroll_y + viewport.height() as i32,
                        );
                    }
                }
                return true;
            }
        }
        false
    }

    fn handle_context_menu(&mut self, event: &ContextMenuEvent) -> bool {
        self.ensure_layout();

        // Find the index at the context menu position
        let index = self.index_at(event.local_pos).unwrap_or_else(ModelIndex::invalid);

        // Emit the context_menu_requested signal with the index and position
        self.context_menu_requested.emit((index, event.local_pos));

        true
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        ctx.renderer().fill_rect(self.base.rect(), self.background_color);
    }

    fn paint_items(&self, ctx: &mut PaintContext<'_>) {
        if self.model.is_none() || self.item_rects.is_empty() {
            return;
        }

        let (first_visible, last_visible) = self.visible_range();
        let viewport = self.viewport_rect();

        for row in first_visible..=last_visible {
            if let Some(content_rect) = self.item_rects.get(row) {
                // Transform to viewport coordinates
                let visual_rect = Rect::new(
                    content_rect.origin.x - self.scroll_x as f32,
                    content_rect.origin.y - self.scroll_y as f32,
                    content_rect.width(),
                    content_rect.height(),
                );

                // Clip check
                if !rects_intersect(visual_rect, viewport) {
                    continue;
                }

                let option = self.build_style_option(row, visual_rect);
                let mut delegate_ctx = DelegatePaintContext::new(ctx.renderer(), visual_rect);
                self.delegate.paint(&mut delegate_ctx, &option);
            }
        }
    }

    fn paint_scrollbars(&self, ctx: &mut PaintContext<'_>) {
        // Paint vertical scrollbar
        if let Some(rect) = self.vertical_scrollbar_rect() {
            self.paint_scrollbar(ctx, rect, false);
        }

        // Paint horizontal scrollbar
        if let Some(rect) = self.horizontal_scrollbar_rect() {
            self.paint_scrollbar(ctx, rect, true);
        }

        // Paint corner
        if self.is_horizontal_scrollbar_visible() && self.is_vertical_scrollbar_visible() {
            let base_rect = self.base.rect();
            let corner = Rect::new(
                base_rect.width() - self.scrollbar_thickness,
                base_rect.height() - self.scrollbar_thickness,
                self.scrollbar_thickness,
                self.scrollbar_thickness,
            );
            let corner_color = Color::from_rgb8(230, 230, 230);
            ctx.renderer().fill_rect(corner, corner_color);
        }
    }

    fn paint_scrollbar(&self, ctx: &mut PaintContext<'_>, rect: Rect, horizontal: bool) {
        // Track
        let track_color = Color::from_rgb8(240, 240, 240);
        ctx.renderer().fill_rect(rect, track_color);

        // Thumb
        let viewport = self.viewport_rect();
        let (content_length, viewport_length, scroll_pos, max_scroll) = if horizontal {
            (
                self.content_size.width,
                viewport.width(),
                self.scroll_x as f32,
                self.max_scroll_x() as f32,
            )
        } else {
            (
                self.content_size.height,
                viewport.height(),
                self.scroll_y as f32,
                self.max_scroll_y() as f32,
            )
        };

        if content_length <= 0.0 {
            return;
        }

        let thumb_ratio = (viewport_length / content_length).min(1.0);
        let (thumb_length, thumb_pos) = if horizontal {
            let track_length = rect.width();
            let thumb_length = (track_length * thumb_ratio).max(20.0).min(track_length);
            let available_travel = track_length - thumb_length;
            let thumb_pos = if max_scroll > 0.0 {
                rect.origin.x + (scroll_pos / max_scroll) * available_travel
            } else {
                rect.origin.x
            };
            (thumb_length, thumb_pos)
        } else {
            let track_length = rect.height();
            let thumb_length = (track_length * thumb_ratio).max(20.0).min(track_length);
            let available_travel = track_length - thumb_length;
            let thumb_pos = if max_scroll > 0.0 {
                rect.origin.y + (scroll_pos / max_scroll) * available_travel
            } else {
                rect.origin.y
            };
            (thumb_length, thumb_pos)
        };

        let thumb_rect = if horizontal {
            Rect::new(
                thumb_pos,
                rect.origin.y + 2.0,
                thumb_length,
                rect.height() - 4.0,
            )
        } else {
            Rect::new(
                rect.origin.x + 2.0,
                thumb_pos,
                rect.width() - 4.0,
                thumb_length,
            )
        };

        let thumb_color = Color::from_rgb8(180, 180, 180);
        let thumb_rrect = horizon_lattice_render::RoundedRect::new(thumb_rect, 4.0);
        ctx.renderer().fill_rounded_rect(thumb_rrect, thumb_color);
    }
}

fn rects_intersect(a: Rect, b: Rect) -> bool {
    let a_right = a.origin.x + a.width();
    let a_bottom = a.origin.y + a.height();
    let b_right = b.origin.x + b.width();
    let b_bottom = b.origin.y + b.height();

    a.origin.x < b_right && a_right > b.origin.x && a.origin.y < b_bottom && a_bottom > b.origin.y
}

impl Object for ListView {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ListView {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::from_dimensions(200.0, 150.0).with_minimum_dimensions(50.0, 50.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Const cast to call ensure_layout - normally we'd use interior mutability
        // For now, paint with potentially stale layout (will be correct after first interaction)
        self.paint_background(ctx);
        self.paint_items(ctx);
        self.paint_drop_indicator(ctx);
        self.paint_scrollbars(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Ensure layout is up to date before handling events
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
            WidgetEvent::Resize(_) => {
                self.layout_dirty = true;
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

// Ensure ListView is Send + Sync
static_assertions::assert_impl_all!(ListView: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_list_view_creation() {
        setup();
        let view = ListView::new();
        assert!(view.model().is_none());
        assert_eq!(view.view_mode(), ListViewMode::ListMode);
        assert!(!view.selection_model().has_selection());
    }

    #[test]
    fn test_list_view_builder() {
        setup();
        let view = ListView::new()
            .with_view_mode(ListViewMode::IconMode)
            .with_selection_mode(SelectionMode::ExtendedSelection)
            .with_spacing(8.0)
            .with_grid_size(Size::new(100.0, 120.0));

        assert_eq!(view.view_mode(), ListViewMode::IconMode);
        assert_eq!(
            view.selection_model().selection_mode(),
            SelectionMode::ExtendedSelection
        );
        assert_eq!(view.spacing(), 8.0);
        assert_eq!(view.grid_size(), Some(Size::new(100.0, 120.0)));
    }

    #[test]
    fn test_scroll_position() {
        setup();
        let mut view = ListView::new();
        view.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        // Initially at origin
        assert_eq!(view.scroll_x(), 0);
        assert_eq!(view.scroll_y(), 0);

        // Can't scroll without content
        view.set_scroll_position(100, 100);
        assert_eq!(view.scroll_x(), 0);
        assert_eq!(view.scroll_y(), 0);
    }

    #[test]
    fn test_viewport_rect() {
        setup();
        let mut view = ListView::new();
        view.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        let viewport = view.viewport_rect();
        assert_eq!(viewport.width(), 200.0);
        assert_eq!(viewport.height(), 200.0);
    }

    #[test]
    fn test_context_menu_signal() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        setup();

        let view = ListView::new();
        let signal_received = Arc::new(AtomicBool::new(false));
        let received_clone = signal_received.clone();

        // Connect to the context menu signal
        view.context_menu_requested.connect(move |_| {
            received_clone.store(true, Ordering::SeqCst);
        });

        // Emit a test signal (simulating what handle_context_menu does)
        view.context_menu_requested.emit((ModelIndex::invalid(), Point::new(10.0, 10.0)));

        assert!(signal_received.load(Ordering::SeqCst));
    }
}
