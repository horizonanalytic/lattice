//! Model-less tree widget for simple hierarchical data displays.
//!
//! [`TreeWidget`] provides a convenient way to display tree data without
//! requiring explicit model creation. Items are managed directly through the widget.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{TreeWidget, TreeWidgetItem};
//!
//! let mut tree = TreeWidget::new();
//!
//! let mut root = TreeWidgetItem::new("Root");
//! root.add_child(TreeWidgetItem::new("Child 1"));
//! root.add_child(TreeWidgetItem::new("Child 2"));
//!
//! tree.add_top_level_item(root);
//!
//! // Connect to signals
//! tree.item_clicked.connect(|path| {
//!     println!("Clicked item at path {:?}", path);
//! });
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Icon, Point, Rect, Renderer, Stroke};
use parking_lot::RwLock;

use crate::model::selection::SelectionMode;
use crate::model::{
    CheckState, ItemData, ItemFlags, ItemModel, ItemRole, ModelIndex, ModelSignals,
};
use crate::widget::{
    FocusPolicy, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

/// Configuration for tree indentation style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TreeIndentationStyle {
    /// Simple indentation with no lines.
    #[default]
    Simple,
    /// Show dotted branch lines connecting items.
    DottedLines,
    /// Show solid branch lines connecting items.
    SolidLines,
}

/// An item in a [`TreeWidget`].
///
/// Stores all the data for a single tree item including text, icon, children, and custom data.
#[derive(Debug, Clone)]
pub struct TreeWidgetItem {
    text: String,
    icon: Option<Icon>,
    tooltip: Option<String>,
    check_state: Option<CheckState>,
    flags: ItemFlags,
    data: HashMap<u32, ItemData>,
    background: Option<Color>,
    foreground: Option<Color>,
    expanded: bool,
    hidden: bool,
    children: Vec<TreeWidgetItem>,
}

impl Default for TreeWidgetItem {
    fn default() -> Self {
        Self::new("")
    }
}

impl TreeWidgetItem {
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
            expanded: false,
            hidden: false,
            children: Vec::new(),
        }
    }

    /// Creates a new item with text and icon.
    pub fn with_icon(text: impl Into<String>, icon: Icon) -> Self {
        let mut item = Self::new(text);
        item.icon = Some(icon);
        item
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

    /// Returns whether the item is expanded.
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Sets whether the item is expanded.
    pub fn set_expanded(&mut self, expanded: bool) {
        self.expanded = expanded;
    }

    /// Returns whether the item is hidden.
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Sets whether the item is hidden.
    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }

    /// Returns the number of children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Returns whether the item has children.
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Gets a reference to a child at the given index.
    pub fn child(&self, index: usize) -> Option<&TreeWidgetItem> {
        self.children.get(index)
    }

    /// Gets a mutable reference to a child at the given index.
    pub fn child_mut(&mut self, index: usize) -> Option<&mut TreeWidgetItem> {
        self.children.get_mut(index)
    }

    /// Adds a child item.
    pub fn add_child(&mut self, child: TreeWidgetItem) {
        self.children.push(child);
    }

    /// Inserts a child at the given index.
    pub fn insert_child(&mut self, index: usize, child: TreeWidgetItem) {
        let index = index.min(self.children.len());
        self.children.insert(index, child);
    }

    /// Removes and returns the child at the given index.
    pub fn take_child(&mut self, index: usize) -> Option<TreeWidgetItem> {
        if index < self.children.len() {
            Some(self.children.remove(index))
        } else {
            None
        }
    }

    /// Returns an iterator over the children.
    pub fn children(&self) -> impl Iterator<Item = &TreeWidgetItem> {
        self.children.iter()
    }

    /// Returns a mutable iterator over the children.
    pub fn children_mut(&mut self) -> impl Iterator<Item = &mut TreeWidgetItem> {
        self.children.iter_mut()
    }
}

/// A flattened row for rendering.
#[derive(Debug, Clone)]
struct FlattenedRow {
    /// Path to this item (indices from root).
    path: Vec<usize>,
    /// The depth (indentation level).
    depth: usize,
    /// Whether this item has children.
    has_children: bool,
    /// Whether this item is expanded.
    is_expanded: bool,
    /// Whether this is the last child of its parent.
    is_last_child: bool,
}

/// Internal model for TreeWidget that implements ItemModel.
struct TreeWidgetModel {
    items: Arc<RwLock<Vec<TreeWidgetItem>>>,
    signals: ModelSignals,
}

impl TreeWidgetModel {
    fn new(items: Arc<RwLock<Vec<TreeWidgetItem>>>) -> Self {
        Self {
            items,
            signals: ModelSignals::new(),
        }
    }

    fn get_item_at_path<'a>(
        items: &'a [TreeWidgetItem],
        path: &[usize],
    ) -> Option<&'a TreeWidgetItem> {
        if path.is_empty() {
            return None;
        }

        let mut current = items.get(path[0])?;
        for &index in &path[1..] {
            current = current.child(index)?;
        }
        Some(current)
    }
}

impl ItemModel for TreeWidgetModel {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        let items = self.items.read();

        if !parent.is_valid() {
            return items.len();
        }

        // Decode path from internal_id
        if let Some(path) = decode_path(parent.internal_id())
            && let Some(item) = TreeWidgetModel::get_item_at_path(&items, &path) {
                return item.child_count();
            }

        0
    }

    fn column_count(&self, _parent: &ModelIndex) -> usize {
        1
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        if !index.is_valid() {
            return ItemData::None;
        }

        let items = self.items.read();

        // Decode path from internal_id
        let Some(path) = decode_path(index.internal_id()) else {
            return ItemData::None;
        };

        let Some(item) = TreeWidgetModel::get_item_at_path(&items, &path) else {
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
            ItemRole::User(n) => item.data.get(&n).cloned().unwrap_or(ItemData::None),
            _ => ItemData::None,
        }
    }

    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex {
        if column > 0 {
            return ModelIndex::invalid();
        }

        let items = self.items.read();

        let path = if !parent.is_valid() {
            if row >= items.len() {
                return ModelIndex::invalid();
            }
            vec![row]
        } else {
            let Some(parent_path) = decode_path(parent.internal_id()) else {
                return ModelIndex::invalid();
            };

            let Some(parent_item) = TreeWidgetModel::get_item_at_path(&items, &parent_path) else {
                return ModelIndex::invalid();
            };

            if row >= parent_item.child_count() {
                return ModelIndex::invalid();
            }

            let mut path = parent_path;
            path.push(row);
            path
        };

        let id = encode_path(&path);
        ModelIndex::with_internal_id(row, column, parent.clone(), id)
    }

    fn parent(&self, index: &ModelIndex) -> ModelIndex {
        if !index.is_valid() {
            return ModelIndex::invalid();
        }

        let Some(path) = decode_path(index.internal_id()) else {
            return ModelIndex::invalid();
        };

        if path.len() <= 1 {
            return ModelIndex::invalid();
        }

        let parent_path: Vec<usize> = path[..path.len() - 1].to_vec();
        let parent_row = *parent_path.last().unwrap_or(&0);
        let grandparent_path = if parent_path.len() > 1 {
            parent_path[..parent_path.len() - 1].to_vec()
        } else {
            vec![]
        };

        let grandparent = if grandparent_path.is_empty() {
            ModelIndex::invalid()
        } else {
            let id = encode_path(&grandparent_path);
            let row = *grandparent_path.last().unwrap_or(&0);
            ModelIndex::with_internal_id(row, 0, ModelIndex::invalid(), id)
        };

        let id = encode_path(&parent_path);
        ModelIndex::with_internal_id(parent_row, 0, grandparent, id)
    }

    fn signals(&self) -> &ModelSignals {
        &self.signals
    }

    fn flags(&self, index: &ModelIndex) -> ItemFlags {
        if !index.is_valid() {
            return ItemFlags::disabled();
        }

        let items = self.items.read();

        let Some(path) = decode_path(index.internal_id()) else {
            return ItemFlags::disabled();
        };

        let Some(item) = TreeWidgetModel::get_item_at_path(&items, &path) else {
            return ItemFlags::disabled();
        };

        item.flags
    }

    fn has_children(&self, parent: &ModelIndex) -> bool {
        self.row_count(parent) > 0
    }
}

/// Encodes a path as a u64 for internal_id.
/// Supports paths up to 8 levels deep with indices up to 255.
fn encode_path(path: &[usize]) -> u64 {
    let mut id: u64 = 0;
    for (i, &index) in path.iter().enumerate() {
        if i >= 8 {
            break;
        }
        id |= ((index as u64) & 0xFF) << (i * 8);
    }
    // Use high bits to store path length
    id |= (path.len() as u64) << 56;
    id
}

/// Decodes a path from an internal_id.
fn decode_path(id: u64) -> Option<Vec<usize>> {
    if id == 0 {
        return None;
    }

    let len = (id >> 56) as usize;
    if len == 0 || len > 8 {
        return None;
    }

    let mut path = Vec::with_capacity(len);
    for i in 0..len {
        let index = ((id >> (i * 8)) & 0xFF) as usize;
        path.push(index);
    }
    Some(path)
}

/// A model-less tree widget for simple hierarchical data displays.
///
/// `TreeWidget` provides direct item manipulation without requiring a separate model.
/// For complex scenarios (large datasets, custom data types), use [`TreeView`]
/// with an explicit model instead.
pub struct TreeWidget {
    base: WidgetBase,
    items: Arc<RwLock<Vec<TreeWidgetItem>>>,
    model: Arc<TreeWidgetModel>,

    // Cached flattened representation
    flattened_rows: Vec<FlattenedRow>,
    layout_dirty: bool,

    // Selection
    current_path: Option<Vec<usize>>,
    selection_mode: SelectionMode,
    selected_paths: Vec<Vec<usize>>,

    // Layout
    indentation: f32,
    indentation_style: TreeIndentationStyle,
    item_height: f32,
    expand_indicator_size: f32,

    // Scrolling
    scroll_y: i32,
    content_height: f32,

    // Visual state
    hovered_row: Option<usize>,
    pressed_row: Option<usize>,

    // Appearance
    background_color: Color,
    alternate_row_colors: bool,
    branch_line_color: Color,
    expand_indicator_color: Color,

    // Signals
    /// Emitted when an item is clicked. Parameter is the path to the item.
    pub item_clicked: Signal<Vec<usize>>,
    /// Emitted when an item is double-clicked. Parameter is the path to the item.
    pub item_double_clicked: Signal<Vec<usize>>,
    /// Emitted when an item is activated (Enter or double-click). Parameter is the path.
    pub item_activated: Signal<Vec<usize>>,
    /// Emitted when the current item changes.
    pub current_item_changed: Signal<(Option<Vec<usize>>, Option<Vec<usize>>)>,
    /// Emitted when an item is expanded. Parameter is the path to the item.
    pub item_expanded: Signal<Vec<usize>>,
    /// Emitted when an item is collapsed. Parameter is the path to the item.
    pub item_collapsed: Signal<Vec<usize>>,
    /// Emitted when an item's data changes. Parameter is the path to the item.
    pub item_changed: Signal<Vec<usize>>,
}

impl Default for TreeWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeWidget {
    /// Creates a new empty TreeWidget.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));

        let items = Arc::new(RwLock::new(Vec::new()));
        let model = Arc::new(TreeWidgetModel::new(items.clone()));

        Self {
            base,
            items,
            model,
            flattened_rows: Vec::new(),
            layout_dirty: true,
            current_path: None,
            selection_mode: SelectionMode::SingleSelection,
            selected_paths: Vec::new(),
            indentation: 20.0,
            indentation_style: TreeIndentationStyle::Simple,
            item_height: 24.0,
            expand_indicator_size: 16.0,
            scroll_y: 0,
            content_height: 0.0,
            hovered_row: None,
            pressed_row: None,
            background_color: Color::WHITE,
            alternate_row_colors: false,
            branch_line_color: Color::from_rgb8(180, 180, 180),
            expand_indicator_color: Color::from_rgb8(100, 100, 100),
            item_clicked: Signal::new(),
            item_double_clicked: Signal::new(),
            item_activated: Signal::new(),
            current_item_changed: Signal::new(),
            item_expanded: Signal::new(),
            item_collapsed: Signal::new(),
            item_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Item Management
    // =========================================================================

    /// Adds a top-level item.
    pub fn add_top_level_item(&mut self, item: TreeWidgetItem) {
        let row = self.items.read().len();
        self.model
            .signals
            .emit_rows_inserted(ModelIndex::invalid(), row, row, || {
                self.items.write().push(item);
            });
        self.layout_dirty = true;
        self.base.update();
    }

    /// Inserts a top-level item at the specified index.
    pub fn insert_top_level_item(&mut self, index: usize, item: TreeWidgetItem) {
        let len = self.items.read().len();
        let index = index.min(len);

        self.model
            .signals
            .emit_rows_inserted(ModelIndex::invalid(), index, index, || {
                self.items.write().insert(index, item);
            });

        self.layout_dirty = true;
        self.base.update();
    }

    /// Removes and returns the top-level item at the specified index.
    pub fn take_top_level_item(&mut self, index: usize) -> Option<TreeWidgetItem> {
        if index >= self.items.read().len() {
            return None;
        }

        let mut removed = None;
        self.model
            .signals
            .emit_rows_removed(ModelIndex::invalid(), index, index, || {
                removed = Some(self.items.write().remove(index));
            });

        self.layout_dirty = true;
        self.base.update();
        removed
    }

    /// Returns the number of top-level items.
    pub fn top_level_item_count(&self) -> usize {
        self.items.read().len()
    }

    /// Gets a reference to a top-level item.
    pub fn top_level_item(&self, index: usize) -> Option<TreeWidgetItem> {
        self.items.read().get(index).cloned()
    }

    /// Gets an item by path.
    pub fn item_at_path(&self, path: &[usize]) -> Option<TreeWidgetItem> {
        let items = self.items.read();
        TreeWidgetModel::get_item_at_path(&items, path).cloned()
    }

    /// Modifies an item at the given path.
    pub fn modify_item<F, R>(&mut self, path: &[usize], f: F) -> Option<R>
    where
        F: FnOnce(&mut TreeWidgetItem) -> R,
    {
        if path.is_empty() {
            return None;
        }

        let result = {
            let mut items = self.items.write();
            let mut current = items.get_mut(path[0])?;
            for &index in &path[1..] {
                current = current.child_mut(index)?;
            }
            f(current)
        };

        let index = self.model.index(path[0], 0, &ModelIndex::invalid());
        self.model
            .signals
            .emit_data_changed_single(index, vec![ItemRole::Display]);
        self.item_changed.emit(path.to_vec());
        self.layout_dirty = true;
        self.base.update();
        Some(result)
    }

    /// Removes all items.
    pub fn clear(&mut self) {
        self.model.signals.emit_reset(|| {
            self.items.write().clear();
        });
        self.current_path = None;
        self.selected_paths.clear();
        self.flattened_rows.clear();
        self.layout_dirty = true;
        self.base.update();
    }

    // =========================================================================
    // Expand/Collapse
    // =========================================================================

    /// Expands an item at the given path.
    pub fn expand_item(&mut self, path: &[usize]) {
        self.modify_item(path, |item| {
            if item.has_children() && !item.is_expanded() {
                item.set_expanded(true);
            }
        });
        self.item_expanded.emit(path.to_vec());
        self.layout_dirty = true;
        self.base.update();
    }

    /// Collapses an item at the given path.
    pub fn collapse_item(&mut self, path: &[usize]) {
        self.modify_item(path, |item| {
            if item.is_expanded() {
                item.set_expanded(false);
            }
        });
        self.item_collapsed.emit(path.to_vec());
        self.layout_dirty = true;
        self.base.update();
    }

    /// Toggles expand/collapse for an item.
    pub fn toggle_expand(&mut self, path: &[usize]) {
        let was_expanded = self
            .item_at_path(path)
            .map(|i| i.is_expanded())
            .unwrap_or(false);
        if was_expanded {
            self.collapse_item(path);
        } else {
            self.expand_item(path);
        }
    }

    /// Expands all items recursively.
    pub fn expand_all(&mut self) {
        fn expand_recursive(item: &mut TreeWidgetItem) {
            if item.has_children() {
                item.set_expanded(true);
                for child in item.children_mut() {
                    expand_recursive(child);
                }
            }
        }

        {
            let mut items = self.items.write();
            for item in items.iter_mut() {
                expand_recursive(item);
            }
        }

        self.layout_dirty = true;
        self.base.update();
    }

    /// Collapses all items recursively.
    pub fn collapse_all(&mut self) {
        fn collapse_recursive(item: &mut TreeWidgetItem) {
            item.set_expanded(false);
            for child in item.children_mut() {
                collapse_recursive(child);
            }
        }

        {
            let mut items = self.items.write();
            for item in items.iter_mut() {
                collapse_recursive(item);
            }
        }

        self.layout_dirty = true;
        self.base.update();
    }

    // =========================================================================
    // Selection
    // =========================================================================

    /// Gets the current item path.
    pub fn current_item(&self) -> Option<&Vec<usize>> {
        self.current_path.as_ref()
    }

    /// Sets the current item by path.
    pub fn set_current_item(&mut self, path: Option<Vec<usize>>) {
        if self.current_path != path {
            let old = self.current_path.clone();
            self.current_path = path.clone();

            // In single selection mode, also update selection
            if self.selection_mode == SelectionMode::SingleSelection {
                self.selected_paths = path.clone().into_iter().collect();
            }

            self.current_item_changed.emit((old, path));
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
            self.selected_paths.clear();
        }
        self.base.update();
    }

    /// Returns whether an item is selected.
    pub fn is_item_selected(&self, path: &[usize]) -> bool {
        self.selected_paths.iter().any(|p| p == path)
    }

    /// Clears the selection.
    pub fn clear_selection(&mut self) {
        self.selected_paths.clear();
        self.base.update();
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Sets the indentation amount.
    pub fn set_indentation(&mut self, indentation: f32) {
        self.indentation = indentation;
        self.layout_dirty = true;
        self.base.update();
    }

    /// Sets the indentation style.
    pub fn set_indentation_style(&mut self, style: TreeIndentationStyle) {
        self.indentation_style = style;
        self.base.update();
    }

    /// Sets whether to use alternating row colors.
    pub fn set_alternate_row_colors(&mut self, enabled: bool) {
        self.alternate_row_colors = enabled;
        self.base.update();
    }

    /// Sets the background color.
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
        self.base.update();
    }

    /// Sets the item height.
    pub fn set_item_height(&mut self, height: f32) {
        self.item_height = height;
        self.layout_dirty = true;
        self.base.update();
    }

    // =========================================================================
    // Internal
    // =========================================================================

    fn ensure_layout(&mut self) {
        if self.layout_dirty {
            self.update_layout();
        }
    }

    fn update_layout(&mut self) {
        self.flattened_rows.clear();

        fn flatten_item(
            items: &[TreeWidgetItem],
            index: usize,
            path: &mut Vec<usize>,
            depth: usize,
            is_last: bool,
            rows: &mut Vec<FlattenedRow>,
        ) {
            let item = &items[index];
            if item.hidden {
                return;
            }

            path.push(index);

            rows.push(FlattenedRow {
                path: path.clone(),
                depth,
                has_children: item.has_children(),
                is_expanded: item.is_expanded(),
                is_last_child: is_last,
            });

            if item.is_expanded() {
                let child_count = item.child_count();
                for (i, _child) in item.children().enumerate() {
                    flatten_item(
                        &item.children,
                        i,
                        path,
                        depth + 1,
                        i == child_count - 1,
                        rows,
                    );
                }
            }

            path.pop();
        }

        let items = self.items.read();
        let mut path = Vec::new();
        let count = items.len();

        for i in 0..count {
            flatten_item(
                &items,
                i,
                &mut path,
                0,
                i == count - 1,
                &mut self.flattened_rows,
            );
        }

        self.content_height = self.flattened_rows.len() as f32 * self.item_height;
        self.layout_dirty = false;
    }

    fn max_scroll_y(&self) -> i32 {
        let viewport_height = self.base.rect().height();
        (self.content_height - viewport_height).max(0.0) as i32
    }

    fn row_at_y(&self, y: f32) -> Option<usize> {
        let content_y = y + self.scroll_y as f32;
        if content_y < 0.0 {
            return None;
        }

        let row = (content_y / self.item_height) as usize;
        if row < self.flattened_rows.len() {
            Some(row)
        } else {
            None
        }
    }

    fn row_rect(&self, row: usize) -> Rect {
        let y = row as f32 * self.item_height - self.scroll_y as f32;
        Rect::new(0.0, y, self.base.rect().width(), self.item_height)
    }

    fn expand_indicator_rect(&self, row: usize) -> Rect {
        let row_data = &self.flattened_rows[row];
        let row_rect = self.row_rect(row);
        let x = row_data.depth as f32 * self.indentation;
        Rect::new(
            x,
            row_rect.origin.y + (self.item_height - self.expand_indicator_size) / 2.0,
            self.expand_indicator_size,
            self.expand_indicator_size,
        )
    }

    fn scroll_to_row(&mut self, row: usize) {
        let viewport_height = self.base.rect().height();
        let item_top = row as f32 * self.item_height;
        let item_bottom = item_top + self.item_height;

        let viewport_top = self.scroll_y as f32;
        let viewport_bottom = viewport_top + viewport_height;

        if item_top < viewport_top {
            self.scroll_y = item_top as i32;
        } else if item_bottom > viewport_bottom {
            self.scroll_y = (item_bottom - viewport_height) as i32;
        }

        self.scroll_y = self.scroll_y.max(0);
    }

    fn handle_click(
        &mut self,
        row: usize,
        pos: Point,
        modifiers: &crate::widget::KeyboardModifiers,
    ) {
        let row_data = self.flattened_rows[row].clone();

        // Check if clicked on expand indicator
        if row_data.has_children {
            let indicator_rect = self.expand_indicator_rect(row);
            if indicator_rect.contains(pos) {
                self.toggle_expand(&row_data.path);
                return;
            }
        }

        // Handle selection
        let path = row_data.path.clone();

        match self.selection_mode {
            SelectionMode::NoSelection => {
                self.set_current_item(Some(path.clone()));
            }
            SelectionMode::SingleSelection => {
                self.set_current_item(Some(path.clone()));
            }
            SelectionMode::MultiSelection | SelectionMode::ExtendedSelection => {
                if modifiers.control {
                    // Toggle selection
                    if let Some(pos) = self.selected_paths.iter().position(|p| *p == path) {
                        self.selected_paths.remove(pos);
                    } else {
                        self.selected_paths.push(path.clone());
                    }
                } else {
                    self.selected_paths = vec![path.clone()];
                }

                let old = self.current_path.clone();
                self.current_path = Some(path.clone());
                if old != self.current_path {
                    self.current_item_changed
                        .emit((old, self.current_path.clone()));
                }
            }
        }

        self.item_clicked.emit(path);
        self.base.update();
    }
}

impl Object for TreeWidget {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for TreeWidget {
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
        let widget_rect = self.base.rect();

        // Background
        ctx.renderer().fill_rect(widget_rect, self.background_color);

        let viewport_height = widget_rect.height();
        let first_visible = (self.scroll_y as f32 / self.item_height) as usize;
        let visible_count = (viewport_height / self.item_height).ceil() as usize + 1;

        let items = self.items.read();

        for visible_idx in 0..visible_count {
            let row = first_visible + visible_idx;
            if row >= self.flattened_rows.len() {
                break;
            }

            let row_data = &self.flattened_rows[row];
            let row_rect = self.row_rect(row);

            if row_rect.origin.y > viewport_height || row_rect.origin.y + row_rect.height() < 0.0 {
                continue;
            }

            // Get item
            let Some(item) = TreeWidgetModel::get_item_at_path(&items, &row_data.path) else {
                continue;
            };

            // Row background
            let is_selected = self.is_item_selected(&row_data.path);
            let bg_color = if is_selected {
                Color::from_rgb8(51, 153, 255)
            } else if self.hovered_row == Some(row) {
                Color::from_rgb8(229, 243, 255)
            } else if self.alternate_row_colors && row % 2 == 1 {
                Color::from_rgb8(245, 245, 245)
            } else if let Some(bg) = item.background {
                bg
            } else {
                self.background_color
            };

            ctx.renderer().fill_rect(row_rect, bg_color);

            // Branch lines (if enabled)
            if self.indentation_style != TreeIndentationStyle::Simple && row_data.depth > 0 {
                // Draw branch lines would go here
            }

            // Expand indicator
            let content_x =
                row_data.depth as f32 * self.indentation + self.expand_indicator_size + 4.0;

            if row_data.has_children {
                let indicator_rect = self.expand_indicator_rect(row);
                let center_x = indicator_rect.origin.x + indicator_rect.width() / 2.0;
                let center_y = indicator_rect.origin.y + indicator_rect.height() / 2.0;
                let size = 5.0;

                let indicator_stroke = Stroke::new(self.expand_indicator_color, 1.5);
                if row_data.is_expanded {
                    // Draw down arrow
                    ctx.renderer().draw_line(
                        Point::new(center_x - size, center_y - size / 2.0),
                        Point::new(center_x, center_y + size / 2.0),
                        &indicator_stroke,
                    );
                    ctx.renderer().draw_line(
                        Point::new(center_x, center_y + size / 2.0),
                        Point::new(center_x + size, center_y - size / 2.0),
                        &indicator_stroke,
                    );
                } else {
                    // Draw right arrow
                    ctx.renderer().draw_line(
                        Point::new(center_x - size / 2.0, center_y - size),
                        Point::new(center_x + size / 2.0, center_y),
                        &indicator_stroke,
                    );
                    ctx.renderer().draw_line(
                        Point::new(center_x + size / 2.0, center_y),
                        Point::new(center_x - size / 2.0, center_y + size),
                        &indicator_stroke,
                    );
                }
            }

            // Item text
            let text_color = if is_selected {
                Color::WHITE
            } else if let Some(fg) = item.foreground {
                fg
            } else {
                Color::BLACK
            };

            let text_x = content_x + 4.0;
            let text_y = row_rect.origin.y + (row_rect.height() - 14.0) / 2.0;
            // Text rendering placeholder - actual text rendering requires FontSystem/TextLayout
            let text_width = item.text.len() as f32 * 7.0; // Approximate width
            ctx.renderer().fill_rect(
                Rect::new(
                    text_x,
                    text_y,
                    text_width.min(row_rect.width() - text_x + row_rect.origin.x),
                    14.0,
                ),
                text_color.with_alpha(0.8),
            );

            // Focus indicator
            if self.current_path.as_ref() == Some(&row_data.path) && self.base.has_focus() {
                let focus_rect = Rect::new(
                    row_rect.origin.x + 1.0,
                    row_rect.origin.y + 1.0,
                    row_rect.width() - 2.0,
                    row_rect.height() - 2.0,
                );
                let focus_stroke = Stroke::new(Color::from_rgb8(100, 100, 100), 1.0);
                ctx.renderer().stroke_rect(focus_rect, &focus_stroke);
            }
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        self.ensure_layout();

        match event {
            WidgetEvent::MousePress(e) => {
                if e.button == crate::widget::MouseButton::Left
                    && let Some(row) = self.row_at_y(e.local_pos.y) {
                        self.pressed_row = Some(row);
                        self.handle_click(row, e.local_pos, &e.modifiers);
                        event.accept();
                        return true;
                    }
            }
            WidgetEvent::MouseRelease(e) => {
                if e.button == crate::widget::MouseButton::Left {
                    self.pressed_row = None;
                    self.base.update();
                }
            }
            WidgetEvent::MouseMove(e) => {
                let old_hovered = self.hovered_row;
                self.hovered_row = self.row_at_y(e.local_pos.y);
                if old_hovered != self.hovered_row {
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
                if self.flattened_rows.is_empty() {
                    return false;
                }

                // Find current row index
                let current_row = self
                    .current_path
                    .as_ref()
                    .and_then(|path| self.flattened_rows.iter().position(|r| &r.path == path));

                match e.key {
                    crate::widget::Key::ArrowUp => {
                        if let Some(row) = current_row {
                            if row > 0 {
                                let path = self.flattened_rows[row - 1].path.clone();
                                self.set_current_item(Some(path));
                                self.scroll_to_row(row - 1);
                            }
                        } else if !self.flattened_rows.is_empty() {
                            let path = self.flattened_rows[0].path.clone();
                            self.set_current_item(Some(path));
                        }
                        return true;
                    }
                    crate::widget::Key::ArrowDown => {
                        if let Some(row) = current_row {
                            if row + 1 < self.flattened_rows.len() {
                                let path = self.flattened_rows[row + 1].path.clone();
                                self.set_current_item(Some(path));
                                self.scroll_to_row(row + 1);
                            }
                        } else if !self.flattened_rows.is_empty() {
                            let path = self.flattened_rows[0].path.clone();
                            self.set_current_item(Some(path));
                        }
                        return true;
                    }
                    crate::widget::Key::ArrowLeft => {
                        if let Some(path) = self.current_path.clone() {
                            let item = self.item_at_path(&path);
                            if item.as_ref().map(|i| i.is_expanded()).unwrap_or(false) {
                                self.collapse_item(&path);
                            } else if path.len() > 1 {
                                // Go to parent
                                let parent_path: Vec<usize> = path[..path.len() - 1].to_vec();
                                self.set_current_item(Some(parent_path));
                            }
                        }
                        return true;
                    }
                    crate::widget::Key::ArrowRight => {
                        if let Some(path) = self.current_path.clone() {
                            let item = self.item_at_path(&path);
                            let has_children =
                                item.as_ref().map(|i| i.has_children()).unwrap_or(false);
                            let is_expanded =
                                item.as_ref().map(|i| i.is_expanded()).unwrap_or(false);
                            if has_children {
                                if is_expanded {
                                    // Go to first child
                                    let mut child_path = path.clone();
                                    child_path.push(0);
                                    self.set_current_item(Some(child_path));
                                } else {
                                    self.expand_item(&path);
                                }
                            }
                        }
                        return true;
                    }
                    crate::widget::Key::Home => {
                        if !self.flattened_rows.is_empty() {
                            let path = self.flattened_rows[0].path.clone();
                            self.set_current_item(Some(path));
                            self.scroll_to_row(0);
                        }
                        return true;
                    }
                    crate::widget::Key::End => {
                        if !self.flattened_rows.is_empty() {
                            let last = self.flattened_rows.len() - 1;
                            let path = self.flattened_rows[last].path.clone();
                            self.set_current_item(Some(path));
                            self.scroll_to_row(last);
                        }
                        return true;
                    }
                    crate::widget::Key::Enter | crate::widget::Key::NumpadEnter => {
                        if let Some(path) = &self.current_path {
                            self.item_activated.emit(path.clone());
                        }
                        return true;
                    }
                    crate::widget::Key::Space => {
                        if let Some(path) = self.current_path.clone() {
                            let item = self.item_at_path(&path);
                            if item.as_ref().map(|i| i.has_children()).unwrap_or(false) {
                                self.toggle_expand(&path);
                            }
                        }
                        return true;
                    }
                    _ => {}
                }
            }
            WidgetEvent::Leave(_) => {
                self.hovered_row = None;
                self.base.update();
            }
            WidgetEvent::Resize(_) => {
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
    fn test_tree_widget_creation() {
        setup();
        let widget = TreeWidget::new();
        assert_eq!(widget.top_level_item_count(), 0);
    }

    #[test]
    fn test_add_top_level_items() {
        setup();
        let mut widget = TreeWidget::new();
        widget.add_top_level_item(TreeWidgetItem::new("First"));
        widget.add_top_level_item(TreeWidgetItem::new("Second"));

        assert_eq!(widget.top_level_item_count(), 2);
        assert_eq!(widget.top_level_item(0).unwrap().text(), "First");
        assert_eq!(widget.top_level_item(1).unwrap().text(), "Second");
    }

    #[test]
    fn test_tree_item_children() {
        setup();
        let mut widget = TreeWidget::new();

        let mut root = TreeWidgetItem::new("Root");
        root.add_child(TreeWidgetItem::new("Child 1"));
        root.add_child(TreeWidgetItem::new("Child 2"));

        widget.add_top_level_item(root);

        let item = widget.top_level_item(0).unwrap();
        assert_eq!(item.child_count(), 2);
        assert_eq!(item.child(0).unwrap().text(), "Child 1");
        assert_eq!(item.child(1).unwrap().text(), "Child 2");
    }

    #[test]
    fn test_expand_collapse() {
        setup();
        let mut widget = TreeWidget::new();

        let mut root = TreeWidgetItem::new("Root");
        root.add_child(TreeWidgetItem::new("Child"));
        widget.add_top_level_item(root);

        assert!(!widget.top_level_item(0).unwrap().is_expanded());

        widget.expand_item(&[0]);
        assert!(widget.top_level_item(0).unwrap().is_expanded());

        widget.collapse_item(&[0]);
        assert!(!widget.top_level_item(0).unwrap().is_expanded());
    }

    #[test]
    fn test_clear() {
        setup();
        let mut widget = TreeWidget::new();
        widget.add_top_level_item(TreeWidgetItem::new("Item"));
        widget.set_current_item(Some(vec![0]));

        widget.clear();

        assert_eq!(widget.top_level_item_count(), 0);
        assert!(widget.current_item().is_none());
    }

    #[test]
    fn test_path_encoding() {
        let path = vec![1, 2, 3, 4];
        let encoded = encode_path(&path);
        let decoded = decode_path(encoded);
        assert_eq!(decoded, Some(path));
    }
}
