//! TreeView widget for displaying hierarchical data from a model.
//!
//! This module provides [`TreeView`], a view widget that displays items from
//! an [`ItemModel`] in a hierarchical tree structure with expand/collapse support.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::model::{TreeModel, TreeNodeData, ItemData, DefaultItemDelegate};
//! use horizon_lattice::widget::widgets::TreeView;
//! use std::sync::Arc;
//!
//! // Create a tree model
//! let model = Arc::new(TreeModel::<String>::new());
//! let root = model.add_root("Root".into());
//! model.add_child(root, "Child 1".into());
//! model.add_child(root, "Child 2".into());
//!
//! // Create tree view
//! let mut tree_view = TreeView::new()
//!     .with_model(model);
//!
//! // Connect to signals
//! tree_view.expanded.connect(|index| {
//!     println!("Expanded row {}", index.row());
//! });
//! ```

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Size, Stroke};

use crate::model::selection::{SelectionFlags, SelectionMode, SelectionModel};
use crate::model::{
    DefaultItemDelegate, DelegatePaintContext, ItemDelegate, ItemModel, ItemRole, ModelIndex,
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

use super::scroll_area::ScrollBarPolicy;

/// Configuration for tree indentation style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IndentationStyle {
    /// Simple indentation with no lines.
    #[default]
    Simple,
    /// Show dotted branch lines connecting items.
    DottedLines,
    /// Show solid branch lines connecting items.
    SolidLines,
}

/// Drag and drop mode for TreeView.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeDragDropMode {
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

/// A flattened row in the tree view.
#[derive(Debug, Clone)]
struct FlattenedRow {
    /// The model index for this row.
    index: ModelIndex,
    /// The depth (indentation level) of this row.
    depth: usize,
    /// Whether this row has children.
    has_children: bool,
    /// Whether this row is expanded (only meaningful if has_children).
    is_expanded: bool,
    /// Whether this row is the last child of its parent.
    is_last_child: bool,
    /// The visual rectangle for this row (in content coordinates).
    rect: Rect,
}

/// A tree view widget for displaying hierarchical data.
///
/// TreeView displays data from an ItemModel in a tree structure with:
/// - Expand/collapse indicators for items with children
/// - Indentation based on hierarchy depth
/// - Optional tree branch lines
/// - All standard view features (selection, scrolling, keyboard navigation)
///
/// # Signals
///
/// - `clicked(ModelIndex)`: Emitted when an item is clicked
/// - `double_clicked(ModelIndex)`: Emitted when an item is double-clicked
/// - `activated(ModelIndex)`: Emitted when Enter is pressed or item is double-clicked
/// - `expanded(ModelIndex)`: Emitted when an item is expanded
/// - `collapsed(ModelIndex)`: Emitted when an item is collapsed
pub struct TreeView {
    // Widget base
    base: WidgetBase,

    // Model/View
    model: Option<Arc<dyn ItemModel>>,
    selection_model: SelectionModel,
    delegate: Arc<dyn ItemDelegate>,

    // Tree structure
    /// Set of expanded indices (by internal_id for stable tracking).
    expanded_ids: HashSet<u64>,
    /// Flattened visible rows cache.
    flattened_rows: Vec<FlattenedRow>,

    // Layout
    indentation: f32,
    indentation_style: IndentationStyle,
    default_item_height: f32,
    uniform_item_heights: bool,
    expand_indicator_size: f32,
    layout_dirty: bool,

    // Scrolling
    scroll_x: i32,
    scroll_y: i32,
    content_size: Size,
    scrollbar_policy_h: ScrollBarPolicy,
    scrollbar_policy_v: ScrollBarPolicy,
    scrollbar_thickness: f32,

    // Visual state
    hovered_row: Option<usize>,
    pressed_row: Option<usize>,
    last_click_time: Option<Instant>,
    last_click_row: Option<usize>,

    // Appearance
    background_color: Color,
    alternate_row_colors: bool,
    branch_line_color: Color,
    expand_indicator_color: Color,

    // Behavior
    root_decorated: bool,
    items_expandable: bool,
    expand_on_double_click: bool,

    // Drag and drop
    drag_drop_mode: TreeDragDropMode,
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
    /// Emitted when an item is expanded.
    pub expanded: Signal<ModelIndex>,
    /// Emitted when an item is collapsed.
    pub collapsed: Signal<ModelIndex>,
    /// Emitted when a context menu is requested.
    ///
    /// The tuple contains (index at position or invalid, position in widget coords).
    /// If the context menu was requested over an item, the index will be valid.
    /// If requested over empty space, the index will be invalid.
    pub context_menu_requested: Signal<(ModelIndex, Point)>,
}

impl Default for TreeView {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeView {
    /// Creates a new empty TreeView.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));

        Self {
            base,
            model: None,
            selection_model: SelectionModel::new(),
            delegate: Arc::new(DefaultItemDelegate::new()),
            expanded_ids: HashSet::new(),
            flattened_rows: Vec::new(),
            indentation: 20.0,
            indentation_style: IndentationStyle::Simple,
            default_item_height: 24.0,
            uniform_item_heights: true,
            expand_indicator_size: 16.0,
            layout_dirty: true,
            scroll_x: 0,
            scroll_y: 0,
            content_size: Size::new(0.0, 0.0),
            scrollbar_policy_h: ScrollBarPolicy::AsNeeded,
            scrollbar_policy_v: ScrollBarPolicy::AsNeeded,
            scrollbar_thickness: 12.0,
            hovered_row: None,
            pressed_row: None,
            last_click_time: None,
            last_click_row: None,
            background_color: Color::WHITE,
            alternate_row_colors: false,
            branch_line_color: Color::from_rgb8(180, 180, 180),
            expand_indicator_color: Color::from_rgb8(100, 100, 100),
            root_decorated: true,
            items_expandable: true,
            expand_on_double_click: true,
            drag_drop_mode: TreeDragDropMode::NoDragDrop,
            drop_indicator_state: DropIndicatorState::new(),
            drag_start_pos: None,
            dragging_row: None,
            clicked: Signal::new(),
            double_clicked: Signal::new(),
            activated: Signal::new(),
            expanded: Signal::new(),
            collapsed: Signal::new(),
            context_menu_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Methods
    // =========================================================================

    /// Creates a TreeView with the given model.
    pub fn with_model(mut self, model: Arc<dyn ItemModel>) -> Self {
        self.set_model(Some(model));
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

    /// Sets the indentation using builder pattern.
    pub fn with_indentation(mut self, indentation: f32) -> Self {
        self.indentation = indentation;
        self.layout_dirty = true;
        self
    }

    /// Sets the indentation style using builder pattern.
    pub fn with_indentation_style(mut self, style: IndentationStyle) -> Self {
        self.indentation_style = style;
        self
    }

    /// Sets whether root items are decorated using builder pattern.
    pub fn with_root_decorated(mut self, decorated: bool) -> Self {
        self.root_decorated = decorated;
        self.layout_dirty = true;
        self
    }

    /// Sets the drag and drop mode using builder pattern.
    pub fn with_drag_drop_mode(mut self, mode: TreeDragDropMode) -> Self {
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
        self.model = model;
        self.selection_model.reset();
        self.expanded_ids.clear();
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

    // =========================================================================
    // Expand/Collapse
    // =========================================================================

    /// Returns whether the item at the given index is expanded.
    pub fn is_expanded(&self, index: &ModelIndex) -> bool {
        if !index.is_valid() {
            return false;
        }
        self.expanded_ids.contains(&index.internal_id())
    }

    /// Expands the item at the given index.
    pub fn expand(&mut self, index: &ModelIndex) {
        if !index.is_valid() {
            return;
        }

        // Check if item has children - requires a model
        let Some(model) = &self.model else {
            return;
        };

        if !model.has_children(index) {
            return;
        }

        if self.expanded_ids.insert(index.internal_id()) {
            self.layout_dirty = true;
            self.expanded.emit(index.clone());
            self.base.update();
        }
    }

    /// Collapses the item at the given index.
    pub fn collapse(&mut self, index: &ModelIndex) {
        if !index.is_valid() {
            return;
        }

        if self.expanded_ids.remove(&index.internal_id()) {
            self.layout_dirty = true;
            self.collapsed.emit(index.clone());
            self.base.update();
        }
    }

    /// Toggles the expanded state of the item at the given index.
    pub fn toggle_expanded(&mut self, index: &ModelIndex) {
        if self.is_expanded(index) {
            self.collapse(index);
        } else {
            self.expand(index);
        }
    }

    /// Expands all items in the tree.
    pub fn expand_all(&mut self) {
        let Some(model) = self.model.clone() else {
            return;
        };
        self.expand_all_recursive_collect(&*model, &ModelIndex::invalid());
        self.layout_dirty = true;
        self.base.update();
    }

    fn expand_all_recursive_collect(&mut self, model: &dyn ItemModel, parent: &ModelIndex) {
        let row_count = model.row_count(parent);
        for row in 0..row_count {
            let index = model.index(row, 0, parent);
            if model.has_children(&index) {
                self.expanded_ids.insert(index.internal_id());
                self.expand_all_recursive_collect(model, &index);
            }
        }
    }

    /// Collapses all items in the tree.
    pub fn collapse_all(&mut self) {
        self.expanded_ids.clear();
        self.layout_dirty = true;
        self.base.update();
    }

    /// Expands the item and all its ancestors to make it visible.
    pub fn expand_to_index(&mut self, index: &ModelIndex) {
        if !index.is_valid() {
            return;
        }

        let Some(model) = self.model.clone() else {
            return;
        };

        // Collect all ancestor IDs first
        let mut ancestors = Vec::new();
        let mut current = model.parent(index);
        while current.is_valid() {
            ancestors.push(current.internal_id());
            current = model.parent(&current);
        }

        // Now insert them
        for id in ancestors {
            self.expanded_ids.insert(id);
        }

        self.layout_dirty = true;
        self.base.update();
    }

    // =========================================================================
    // Layout Properties
    // =========================================================================

    /// Gets the indentation amount per level.
    pub fn indentation(&self) -> f32 {
        self.indentation
    }

    /// Sets the indentation amount per level.
    pub fn set_indentation(&mut self, indentation: f32) {
        if (self.indentation - indentation).abs() > f32::EPSILON {
            self.indentation = indentation;
            self.layout_dirty = true;
            self.base.update();
        }
    }

    /// Gets the indentation style.
    pub fn indentation_style(&self) -> IndentationStyle {
        self.indentation_style
    }

    /// Sets the indentation style.
    pub fn set_indentation_style(&mut self, style: IndentationStyle) {
        if self.indentation_style != style {
            self.indentation_style = style;
            self.base.update();
        }
    }

    /// Gets whether root items are decorated with expand indicators.
    pub fn root_is_decorated(&self) -> bool {
        self.root_decorated
    }

    /// Sets whether root items are decorated with expand indicators.
    pub fn set_root_decorated(&mut self, decorated: bool) {
        if self.root_decorated != decorated {
            self.root_decorated = decorated;
            self.layout_dirty = true;
            self.base.update();
        }
    }

    /// Gets whether items can be expanded/collapsed.
    pub fn items_expandable(&self) -> bool {
        self.items_expandable
    }

    /// Sets whether items can be expanded/collapsed.
    pub fn set_items_expandable(&mut self, expandable: bool) {
        self.items_expandable = expandable;
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

    /// Returns the current drag and drop mode.
    pub fn drag_drop_mode(&self) -> TreeDragDropMode {
        self.drag_drop_mode
    }

    /// Sets the drag and drop mode.
    ///
    /// This controls whether the tree view supports dragging items from it,
    /// accepting drops onto it, or both.
    pub fn set_drag_drop_mode(&mut self, mode: TreeDragDropMode) {
        if self.drag_drop_mode != mode {
            self.drag_drop_mode = mode;
            // Configure whether we accept drops based on the mode
            let accepts_drops = matches!(
                mode,
                TreeDragDropMode::DropOnly
                    | TreeDragDropMode::DragDrop
                    | TreeDragDropMode::InternalMove
            );
            self.base.set_accepts_drops(accepts_drops);
        }
    }

    /// Returns whether dragging from this view is enabled.
    pub fn drag_enabled(&self) -> bool {
        matches!(
            self.drag_drop_mode,
            TreeDragDropMode::DragOnly
                | TreeDragDropMode::DragDrop
                | TreeDragDropMode::InternalMove
        )
    }

    /// Returns whether dropping onto this view is enabled.
    pub fn drop_enabled(&self) -> bool {
        matches!(
            self.drag_drop_mode,
            TreeDragDropMode::DropOnly
                | TreeDragDropMode::DragDrop
                | TreeDragDropMode::InternalMove
        )
    }

    /// Creates drag data from the selected items.
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
    fn drop_position_for_point(&self, point: Point) -> (Option<usize>, DropPosition) {
        if let Some(index) = self.index_at(point)
            && let Some(row_idx) = self.find_flattened_row(&index)
            && let Some(row) = self.flattened_rows.get(row_idx)
        {
            // Determine if in upper or lower half (for row-based drop)
            let viewport = self.viewport_rect();
            let visual_y = row.rect.origin.y - self.scroll_y as f32 + viewport.origin.y;
            let mid_y = visual_y + row.rect.height() / 2.0;
            if point.y < mid_y {
                return (Some(row_idx), DropPosition::AboveItem);
            } else {
                return (Some(row_idx), DropPosition::BelowItem);
            }
        }

        // After last row
        if !self.flattened_rows.is_empty() {
            return (Some(self.flattened_rows.len() - 1), DropPosition::BelowItem);
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
            // Calculate visible row rects for drop indicator
            let item_rects: Vec<(usize, Rect)> = self
                .flattened_rows
                .iter()
                .enumerate()
                .filter_map(|(idx, row)| {
                    // Convert content coordinates to visual coordinates
                    let visual_rect = Rect::new(
                        row.rect.origin.x - self.scroll_x as f32 + viewport.origin.x,
                        row.rect.origin.y - self.scroll_y as f32 + viewport.origin.y,
                        viewport.width(),
                        row.rect.height(),
                    );
                    // Only include visible rows
                    if visual_rect.origin.y + visual_rect.height() >= viewport.origin.y
                        && visual_rect.origin.y <= viewport.origin.y + viewport.height()
                    {
                        Some((idx, visual_rect))
                    } else {
                        None
                    }
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
            // Accept the drop
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
            ctx.renderer().fill_rect(indicator.rect, style.line_color);
        }
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

        // First expand ancestors to make the item visible
        self.expand_to_index(index);
        self.ensure_layout();

        // Find the row for this index and extract needed data
        let target_id = index.internal_id();
        let row_data = self
            .flattened_rows
            .iter()
            .find(|r| r.index.internal_id() == target_id)
            .map(|r| (r.rect.origin.y, r.rect.height()));

        if let Some((item_y, item_height)) = row_data {
            let viewport = self.viewport_rect();
            let item_top = item_y as i32;
            let item_bottom = item_top + item_height as i32;
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

    fn max_scroll_x(&self) -> i32 {
        let viewport = self.viewport_rect();
        (self.content_size.width - viewport.width()).max(0.0) as i32
    }

    fn max_scroll_y(&self) -> i32 {
        let viewport = self.viewport_rect();
        (self.content_size.height - viewport.height()).max(0.0) as i32
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
        let content_y = point.y + self.scroll_y as f32;

        for row in &self.flattened_rows {
            if content_y >= row.rect.origin.y && content_y < row.rect.origin.y + row.rect.height() {
                return Some(row.index.clone());
            }
        }

        None
    }

    /// Returns the visual rectangle for an index in widget coordinates.
    pub fn visual_rect(&self, index: &ModelIndex) -> Option<Rect> {
        if !index.is_valid() {
            return None;
        }

        self.find_flattened_row(index).map(|row_idx| {
            let row = &self.flattened_rows[row_idx];
            Rect::new(
                row.rect.origin.x - self.scroll_x as f32,
                row.rect.origin.y - self.scroll_y as f32,
                row.rect.width(),
                row.rect.height(),
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

    fn find_flattened_row(&self, index: &ModelIndex) -> Option<usize> {
        let target_id = index.internal_id();
        self.flattened_rows
            .iter()
            .position(|r| r.index.internal_id() == target_id)
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
        self.flattened_rows.clear();

        let Some(model) = self.model.clone() else {
            self.content_size = Size::new(0.0, 0.0);
            self.layout_dirty = false;
            return;
        };

        let viewport_width = self.viewport_rect().width();

        // Recursively flatten the tree
        self.flatten_tree(&*model, &ModelIndex::invalid(), 0);

        // Calculate positions - pre-compute indentation values to avoid borrow issues
        let indentation = self.indentation;
        let root_decorated = self.root_decorated;
        let expand_indicator_size = self.expand_indicator_size;
        let default_item_height = self.default_item_height;

        let calculate_indent = |depth: usize| -> f32 {
            let base_indent = if root_decorated {
                expand_indicator_size
            } else {
                0.0
            };
            base_indent + depth as f32 * indentation
        };

        let mut y = 0.0;
        let mut max_width: f32 = 0.0;

        for row in &mut self.flattened_rows {
            let indent = calculate_indent(row.depth);
            let height = default_item_height;
            let width = viewport_width.max(indent + 200.0); // Minimum width

            row.rect = Rect::new(0.0, y, width, height);
            max_width = max_width.max(width);
            y += height;
        }

        self.content_size = Size::new(max_width, y);
        self.layout_dirty = false;
    }

    fn flatten_tree(&mut self, model: &dyn ItemModel, parent: &ModelIndex, depth: usize) {
        let row_count = model.row_count(parent);

        for row in 0..row_count {
            let index = model.index(row, 0, parent);
            let has_children = model.has_children(&index);
            let is_expanded = self.expanded_ids.contains(&index.internal_id());
            let is_last_child = row == row_count - 1;

            self.flattened_rows.push(FlattenedRow {
                index: index.clone(),
                depth,
                has_children,
                is_expanded,
                is_last_child,
                rect: Rect::default(),
            });

            // Recurse into children if expanded
            if has_children && is_expanded {
                self.flatten_tree(model, &index, depth + 1);
            }
        }
    }

    fn calculate_indentation(&self, depth: usize) -> f32 {
        let base_indent = if self.root_decorated {
            self.expand_indicator_size
        } else {
            0.0
        };
        base_indent + depth as f32 * self.indentation
    }

    fn visible_range(&self) -> (usize, usize) {
        if self.flattened_rows.is_empty() {
            return (0, 0);
        }

        let viewport_top = self.scroll_y as f32;
        let viewport_bottom = viewport_top + self.viewport_rect().height();

        let first = self
            .flattened_rows
            .iter()
            .position(|r| r.rect.origin.y + r.rect.height() >= viewport_top)
            .unwrap_or(0);

        let last = self
            .flattened_rows
            .iter()
            .rposition(|r| r.rect.origin.y <= viewport_bottom)
            .unwrap_or(self.flattened_rows.len().saturating_sub(1));

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
    // Expand Indicator Helpers
    // =========================================================================

    fn expand_indicator_rect(&self, row_idx: usize) -> Option<Rect> {
        let row = self.flattened_rows.get(row_idx)?;

        if !row.has_children {
            return None;
        }

        // Don't show indicator for root items if root_decorated is false
        if row.depth == 0 && !self.root_decorated {
            return None;
        }

        let indent = if row.depth == 0 && self.root_decorated {
            0.0
        } else {
            (row.depth
                .saturating_sub(if self.root_decorated { 0 } else { 1 })) as f32
                * self.indentation
        };

        let visual_rect = Rect::new(
            row.rect.origin.x - self.scroll_x as f32,
            row.rect.origin.y - self.scroll_y as f32,
            row.rect.width(),
            row.rect.height(),
        );

        let indicator_y =
            visual_rect.origin.y + (visual_rect.height() - self.expand_indicator_size) / 2.0;

        Some(Rect::new(
            indent,
            indicator_y,
            self.expand_indicator_size,
            self.expand_indicator_size,
        ))
    }

    fn is_point_on_expand_indicator(&self, row_idx: usize, point: Point) -> bool {
        if let Some(indicator_rect) = self.expand_indicator_rect(row_idx) {
            indicator_rect.contains(point)
        } else {
            false
        }
    }

    // =========================================================================
    // Style Option Building
    // =========================================================================

    fn build_style_option(&self, row_idx: usize, rect: Rect) -> StyleOptionViewItem {
        let row = &self.flattened_rows[row_idx];
        let index = &row.index;

        let mut text = None;
        let mut icon = None;
        let mut check_state = None;
        let mut flags = crate::model::ItemFlags::new();

        if let Some(model) = &self.model {
            text = model
                .data(index, ItemRole::Display)
                .as_string()
                .map(|s| s.to_string());
            icon = model.data(index, ItemRole::Decoration).as_icon().cloned();
            check_state = model.data(index, ItemRole::CheckState).as_check_state();
            flags = model.flags(index);
        }

        let is_current = self.selection_model.current_index().internal_id() == index.internal_id()
            && self.selection_model.current_index().is_valid();

        StyleOptionViewItem {
            rect,
            index: index.clone(),
            state: ViewItemState::new()
                .with_selected(self.selection_model.is_selected(index))
                .with_focused(is_current && self.base.has_focus())
                .with_hovered(self.hovered_row == Some(row_idx))
                .with_pressed(self.pressed_row == Some(row_idx))
                .with_enabled(self.base.is_enabled())
                .with_alternate(self.alternate_row_colors && row_idx % 2 == 1)
                .with_expanded(row.is_expanded)
                .with_has_children(row.has_children),
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
        if let Some(rect) = self.vertical_scrollbar_rect()
            && rect.contains(event.local_pos)
        {
            return self.handle_scrollbar_click(event.local_pos, false);
        }
        if let Some(rect) = self.horizontal_scrollbar_rect()
            && rect.contains(event.local_pos)
        {
            return self.handle_scrollbar_click(event.local_pos, true);
        }

        // Check item click
        self.ensure_layout();

        // Find which row was clicked
        let content_y = event.local_pos.y + self.scroll_y as f32;
        let row_idx = self.flattened_rows.iter().position(|r| {
            content_y >= r.rect.origin.y && content_y < r.rect.origin.y + r.rect.height()
        });

        if let Some(row_idx) = row_idx {
            self.pressed_row = Some(row_idx);

            // Check if click is on expand indicator
            if self.items_expandable && self.is_point_on_expand_indicator(row_idx, event.local_pos)
            {
                let index = self.flattened_rows[row_idx].index.clone();
                self.toggle_expanded(&index);
                return true;
            }

            // Handle selection
            let index = self.flattened_rows[row_idx].index.clone();
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
                        // Range selection - find anchor row and select range
                        let anchor_id = self.selection_model.anchor_index().internal_id();
                        if let Some(anchor_row) = self
                            .flattened_rows
                            .iter()
                            .position(|r| r.index.internal_id() == anchor_id)
                        {
                            let start = anchor_row.min(row_idx);
                            let end = anchor_row.max(row_idx);
                            self.selection_model.clear_selection();
                            for i in start..=end {
                                let idx = self.flattened_rows[i].index.clone();
                                self.selection_model.select(idx, SelectionFlags::SELECT);
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
        if let Some(row_idx) = pressed {
            let content_y = event.local_pos.y + self.scroll_y as f32;
            if let Some(click_row) = self.flattened_rows.iter().position(|r| {
                content_y >= r.rect.origin.y && content_y < r.rect.origin.y + r.rect.height()
            }) && click_row == row_idx
            {
                let index = self.flattened_rows[row_idx].index.clone();
                self.clicked.emit(index.clone());

                // Check for double-click
                let now = Instant::now();
                if let (Some(last_time), Some(last_row)) =
                    (self.last_click_time, self.last_click_row)
                    && last_row == row_idx
                    && now.duration_since(last_time).as_millis() < 500
                {
                    self.double_clicked.emit(index.clone());

                    // Expand on double-click if enabled
                    if self.expand_on_double_click
                        && self.items_expandable
                        && self.flattened_rows[row_idx].has_children
                    {
                        self.toggle_expanded(&index);
                    } else {
                        self.activated.emit(index);
                    }

                    self.last_click_time = None;
                    self.last_click_row = None;
                    return true;
                }

                self.last_click_time = Some(now);
                self.last_click_row = Some(row_idx);
            }
        }

        true
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let old_hovered = self.hovered_row;

        if self.viewport_rect().contains(event.local_pos) {
            let content_y = event.local_pos.y + self.scroll_y as f32;
            self.hovered_row = self.flattened_rows.iter().position(|r| {
                content_y >= r.rect.origin.y && content_y < r.rect.origin.y + r.rect.height()
            });
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
        if self.flattened_rows.is_empty() {
            return false;
        }

        let current_id = self.selection_model.current_index().internal_id();
        let current_row = self
            .flattened_rows
            .iter()
            .position(|r| r.index.internal_id() == current_id);

        let row_count = self.flattened_rows.len();

        match event.key {
            Key::ArrowUp => {
                let new_row = current_row.map(|r| r.saturating_sub(1)).unwrap_or(0);
                self.move_to_row(new_row, &event.modifiers);
                true
            }
            Key::ArrowDown => {
                let new_row = current_row.map(|r| (r + 1).min(row_count - 1)).unwrap_or(0);
                self.move_to_row(new_row, &event.modifiers);
                true
            }
            Key::ArrowLeft => {
                if let Some(row_idx) = current_row {
                    let row = &self.flattened_rows[row_idx];
                    if row.has_children && row.is_expanded {
                        // Collapse
                        let index = row.index.clone();
                        self.collapse(&index);
                    } else if row.depth > 0 {
                        // Move to parent
                        if let Some(model) = &self.model {
                            let parent = model.parent(&row.index);
                            if parent.is_valid()
                                && let Some(parent_row) = self.find_flattened_row(&parent)
                            {
                                self.move_to_row(parent_row, &event.modifiers);
                            }
                        }
                    }
                }
                true
            }
            Key::ArrowRight => {
                if let Some(row_idx) = current_row {
                    let row = &self.flattened_rows[row_idx];
                    if row.has_children {
                        if !row.is_expanded {
                            // Expand
                            let index = row.index.clone();
                            self.expand(&index);
                        } else {
                            // Move to first child
                            if row_idx + 1 < row_count {
                                self.move_to_row(row_idx + 1, &event.modifiers);
                            }
                        }
                    }
                }
                true
            }
            Key::PageUp => {
                let viewport_height = self.viewport_rect().height();
                let items_per_page = (viewport_height / self.default_item_height).floor() as usize;
                let new_row = current_row
                    .map(|r| r.saturating_sub(items_per_page.max(1)))
                    .unwrap_or(0);
                self.move_to_row(new_row, &event.modifiers);
                true
            }
            Key::PageDown => {
                let viewport_height = self.viewport_rect().height();
                let items_per_page = (viewport_height / self.default_item_height).floor() as usize;
                let new_row = current_row
                    .map(|r| (r + items_per_page.max(1)).min(row_count - 1))
                    .unwrap_or(0);
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
                if let Some(row_idx) = current_row {
                    let index = self.flattened_rows[row_idx].index.clone();
                    self.selection_model.select(index, SelectionFlags::TOGGLE);
                    self.base.update();
                }
                true
            }
            Key::Enter | Key::NumpadEnter => {
                if let Some(row_idx) = current_row {
                    let row = &self.flattened_rows[row_idx];
                    if row.has_children {
                        let index = row.index.clone();
                        self.toggle_expanded(&index);
                    } else {
                        let index = row.index.clone();
                        self.activated.emit(index);
                    }
                }
                true
            }
            Key::A if event.modifiers.control => {
                // Select all visible items
                for row in &self.flattened_rows {
                    self.selection_model
                        .select(row.index.clone(), SelectionFlags::SELECT);
                }
                self.base.update();
                true
            }
            _ => false,
        }
    }

    fn move_to_row(&mut self, row_idx: usize, modifiers: &crate::widget::KeyboardModifiers) {
        if row_idx >= self.flattened_rows.len() {
            return;
        }

        let index = self.flattened_rows[row_idx].index.clone();
        let mode = self.selection_model.selection_mode();

        let flags = match mode {
            SelectionMode::NoSelection => SelectionFlags::CURRENT,
            SelectionMode::SingleSelection => SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor(),
            SelectionMode::MultiSelection | SelectionMode::ExtendedSelection => {
                if modifiers.shift {
                    // Range selection from anchor
                    let anchor_id = self.selection_model.anchor_index().internal_id();
                    if let Some(anchor_row) = self
                        .flattened_rows
                        .iter()
                        .position(|r| r.index.internal_id() == anchor_id)
                    {
                        let start = anchor_row.min(row_idx);
                        let end = anchor_row.max(row_idx);
                        self.selection_model.clear_selection();
                        for i in start..=end {
                            let idx = self.flattened_rows[i].index.clone();
                            self.selection_model.select(idx, SelectionFlags::SELECT);
                        }
                    }
                    SelectionFlags::CURRENT
                } else if modifiers.control {
                    SelectionFlags::CURRENT
                } else {
                    SelectionFlags::CLEAR_SELECT_CURRENT.with_anchor()
                }
            }
        };

        self.selection_model.set_current_index(index.clone(), flags);

        // Scroll to make visible
        if let Some(row) = self.flattened_rows.get(row_idx) {
            let viewport = self.viewport_rect();
            let item_top = row.rect.origin.y as i32;
            let item_bottom = item_top + row.rect.height() as i32;
            let viewport_top = self.scroll_y;
            let viewport_bottom = self.scroll_y + viewport.height() as i32;

            if item_top < viewport_top {
                self.scroll_y = item_top;
            } else if item_bottom > viewport_bottom {
                self.scroll_y = item_bottom - viewport.height() as i32;
            }

            self.scroll_y = self.scroll_y.clamp(0, self.max_scroll_y());
        }

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
        } else if let Some(rect) = self.vertical_scrollbar_rect() {
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
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        ctx.renderer()
            .fill_rect(self.base.rect(), self.background_color);
    }

    fn paint_items(&self, ctx: &mut PaintContext<'_>) {
        if self.model.is_none() || self.flattened_rows.is_empty() {
            return;
        }

        let (first_visible, last_visible) = self.visible_range();
        let viewport = self.viewport_rect();

        for row_idx in first_visible..=last_visible {
            if let Some(row) = self.flattened_rows.get(row_idx) {
                // Transform to viewport coordinates
                let visual_rect = Rect::new(
                    row.rect.origin.x - self.scroll_x as f32,
                    row.rect.origin.y - self.scroll_y as f32,
                    row.rect.width(),
                    row.rect.height(),
                );

                // Clip check
                if !rects_intersect(visual_rect, viewport) {
                    continue;
                }

                // Calculate content rect (after indentation)
                let indent = self.calculate_indentation(row.depth);
                let content_rect = Rect::new(
                    visual_rect.origin.x + indent + self.expand_indicator_size,
                    visual_rect.origin.y,
                    visual_rect.width() - indent - self.expand_indicator_size,
                    visual_rect.height(),
                );

                // Draw expand indicator if needed
                if row.has_children && (row.depth > 0 || self.root_decorated) {
                    self.paint_expand_indicator(ctx, row_idx, row.is_expanded);
                }

                // Draw branch lines if style requires
                if self.indentation_style != IndentationStyle::Simple {
                    self.paint_branch_lines(ctx, row_idx);
                }

                // Draw item content using delegate
                let option = self.build_style_option(row_idx, content_rect);
                let mut delegate_ctx = DelegatePaintContext::new(ctx.renderer(), content_rect);
                self.delegate.paint(&mut delegate_ctx, &option);
            }
        }
    }

    fn paint_expand_indicator(&self, ctx: &mut PaintContext<'_>, row_idx: usize, expanded: bool) {
        let Some(indicator_rect) = self.expand_indicator_rect(row_idx) else {
            return;
        };

        // Draw expand/collapse indicator using lines (triangle outline)
        let center_x = indicator_rect.origin.x + indicator_rect.width() / 2.0;
        let center_y = indicator_rect.origin.y + indicator_rect.height() / 2.0;
        let size = 4.0;
        let stroke = Stroke::new(self.expand_indicator_color, 1.5);

        if expanded {
            // Pointing down (expanded) - draw a "v" shape
            let left = Point::new(center_x - size, center_y - size / 2.0);
            let bottom = Point::new(center_x, center_y + size / 2.0);
            let right = Point::new(center_x + size, center_y - size / 2.0);
            ctx.renderer().draw_line(left, bottom, &stroke);
            ctx.renderer().draw_line(bottom, right, &stroke);
        } else {
            // Pointing right (collapsed) - draw a ">" shape
            let top = Point::new(center_x - size / 2.0, center_y - size);
            let right = Point::new(center_x + size / 2.0, center_y);
            let bottom = Point::new(center_x - size / 2.0, center_y + size);
            ctx.renderer().draw_line(top, right, &stroke);
            ctx.renderer().draw_line(right, bottom, &stroke);
        }
    }

    fn paint_branch_lines(&self, ctx: &mut PaintContext<'_>, row_idx: usize) {
        let row = &self.flattened_rows[row_idx];

        if row.depth == 0 {
            return;
        }

        let stroke = match self.indentation_style {
            IndentationStyle::DottedLines => Stroke::new(self.branch_line_color, 1.0),
            IndentationStyle::SolidLines => Stroke::new(self.branch_line_color, 1.0),
            IndentationStyle::Simple => return,
        };

        let visual_rect = Rect::new(
            row.rect.origin.x - self.scroll_x as f32,
            row.rect.origin.y - self.scroll_y as f32,
            row.rect.width(),
            row.rect.height(),
        );

        // Draw horizontal connector from branch to item
        let indent_x = if self.root_decorated {
            row.depth as f32 * self.indentation
        } else {
            (row.depth - 1) as f32 * self.indentation
        };

        let h_start = Point::new(indent_x + self.indentation / 2.0, visual_rect.center().y);
        let h_end = Point::new(indent_x + self.indentation, visual_rect.center().y);
        ctx.renderer().draw_line(h_start, h_end, &stroke);

        // Draw vertical line
        let v_top = visual_rect.origin.y;
        let v_bottom = if row.is_last_child {
            visual_rect.center().y
        } else {
            visual_rect.origin.y + visual_rect.height()
        };

        let v_start = Point::new(indent_x + self.indentation / 2.0, v_top);
        let v_end = Point::new(indent_x + self.indentation / 2.0, v_bottom);
        ctx.renderer().draw_line(v_start, v_end, &stroke);
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

    fn handle_context_menu(&mut self, event: &ContextMenuEvent) -> bool {
        self.ensure_layout();

        // Find the index at the context menu position
        let index = self
            .index_at(event.local_pos)
            .unwrap_or_else(ModelIndex::invalid);

        // Emit the context_menu_requested signal with the index and position
        self.context_menu_requested.emit((index, event.local_pos));

        true
    }
}

fn rects_intersect(a: Rect, b: Rect) -> bool {
    let a_right = a.origin.x + a.width();
    let a_bottom = a.origin.y + a.height();
    let b_right = b.origin.x + b.width();
    let b_bottom = b.origin.y + b.height();

    a.origin.x < b_right && a_right > b.origin.x && a.origin.y < b_bottom && a_bottom > b.origin.y
}

impl Object for TreeView {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for TreeView {
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

// Ensure TreeView is Send + Sync
static_assertions::assert_impl_all!(TreeView: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_tree_view_creation() {
        setup();
        let view = TreeView::new();
        assert!(view.model().is_none());
        assert!(!view.selection_model().has_selection());
        assert!(view.root_is_decorated());
        assert!(view.items_expandable());
    }

    #[test]
    fn test_tree_view_builder() {
        setup();
        let view = TreeView::new()
            .with_selection_mode(SelectionMode::ExtendedSelection)
            .with_indentation(30.0)
            .with_indentation_style(IndentationStyle::DottedLines)
            .with_root_decorated(false);

        assert_eq!(
            view.selection_model().selection_mode(),
            SelectionMode::ExtendedSelection
        );
        assert_eq!(view.indentation(), 30.0);
        assert_eq!(view.indentation_style(), IndentationStyle::DottedLines);
        assert!(!view.root_is_decorated());
    }

    #[test]
    fn test_expand_collapse() {
        setup();
        let mut view = TreeView::new();

        // Can't expand without a model
        let index = ModelIndex::with_internal_id(0, 0, ModelIndex::invalid(), 1);
        view.expand(&index);
        assert!(!view.is_expanded(&index));

        // Test expand/collapse tracking
        view.expanded_ids.insert(1);
        let index = ModelIndex::with_internal_id(0, 0, ModelIndex::invalid(), 1);
        assert!(view.is_expanded(&index));

        view.expanded_ids.remove(&1);
        assert!(!view.is_expanded(&index));
    }

    #[test]
    fn test_expand_all_collapse_all() {
        setup();
        let mut view = TreeView::new();

        // Manually add some expanded IDs
        view.expanded_ids.insert(1);
        view.expanded_ids.insert(2);
        view.expanded_ids.insert(3);

        assert!(!view.expanded_ids.is_empty());

        view.collapse_all();
        assert!(view.expanded_ids.is_empty());
    }

    #[test]
    fn test_viewport_rect() {
        setup();
        let mut view = TreeView::new();
        view.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        let viewport = view.viewport_rect();
        assert_eq!(viewport.width(), 200.0);
        assert_eq!(viewport.height(), 200.0);
    }

    #[test]
    fn test_scroll_position() {
        setup();
        let mut view = TreeView::new();
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
    fn test_indentation_calculation() {
        setup();
        let view = TreeView::new();

        // With root_decorated = true
        assert_eq!(view.calculate_indentation(0), view.expand_indicator_size);
        assert_eq!(
            view.calculate_indentation(1),
            view.expand_indicator_size + view.indentation
        );
        assert_eq!(
            view.calculate_indentation(2),
            view.expand_indicator_size + view.indentation * 2.0
        );
    }

    #[test]
    fn test_context_menu_signal() {
        use horizon_lattice_render::Point;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        setup();

        let view = TreeView::new();
        let signal_received = Arc::new(AtomicBool::new(false));
        let received_clone = signal_received.clone();

        // Connect to the context menu signal
        view.context_menu_requested.connect(move |_| {
            received_clone.store(true, Ordering::SeqCst);
        });

        // Emit a test signal (simulating what handle_context_menu does)
        view.context_menu_requested
            .emit((ModelIndex::invalid(), Point::new(10.0, 10.0)));

        assert!(signal_received.load(Ordering::SeqCst));
    }

    #[test]
    fn test_drag_drop_mode_default() {
        setup();
        let view = TreeView::new();
        assert_eq!(view.drag_drop_mode(), TreeDragDropMode::NoDragDrop);
        assert!(!view.drag_enabled());
        assert!(!view.drop_enabled());
    }

    #[test]
    fn test_drag_drop_mode_setter() {
        setup();
        let mut view = TreeView::new();

        view.set_drag_drop_mode(TreeDragDropMode::DragOnly);
        assert_eq!(view.drag_drop_mode(), TreeDragDropMode::DragOnly);
        assert!(view.drag_enabled());
        assert!(!view.drop_enabled());

        view.set_drag_drop_mode(TreeDragDropMode::DropOnly);
        assert_eq!(view.drag_drop_mode(), TreeDragDropMode::DropOnly);
        assert!(!view.drag_enabled());
        assert!(view.drop_enabled());

        view.set_drag_drop_mode(TreeDragDropMode::DragDrop);
        assert_eq!(view.drag_drop_mode(), TreeDragDropMode::DragDrop);
        assert!(view.drag_enabled());
        assert!(view.drop_enabled());

        view.set_drag_drop_mode(TreeDragDropMode::InternalMove);
        assert_eq!(view.drag_drop_mode(), TreeDragDropMode::InternalMove);
        assert!(view.drag_enabled());
        assert!(view.drop_enabled());
    }

    #[test]
    fn test_drag_drop_mode_builder() {
        setup();
        let view = TreeView::new().with_drag_drop_mode(TreeDragDropMode::DragDrop);
        assert_eq!(view.drag_drop_mode(), TreeDragDropMode::DragDrop);
        assert!(view.drag_enabled());
        assert!(view.drop_enabled());
    }

    #[test]
    fn test_drop_indicator_initial_state() {
        setup();
        let view = TreeView::new();
        // Drop indicator should not be active initially
        assert!(!view.drop_indicator_state.has_indicator());
    }
}
