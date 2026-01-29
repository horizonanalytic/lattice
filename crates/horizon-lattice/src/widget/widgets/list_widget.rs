//! Model-less list widget for simple list displays.
//!
//! [`ListWidget`] provides a convenient way to display a list of items without
//! requiring explicit model creation. Items are managed directly through the widget.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::ListWidget;
//!
//! let mut list = ListWidget::new();
//! list.add_item("Apple");
//! list.add_item("Banana");
//! list.add_item("Cherry");
//!
//! // Connect to signals
//! list.item_clicked.connect(|row| {
//!     println!("Clicked row {}", row);
//! });
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Icon, Rect, Renderer, Stroke};
use parking_lot::RwLock;

use crate::model::selection::SelectionMode;
use crate::model::{
    CheckState, ItemData, ItemFlags, ItemModel, ItemRole, ModelIndex, ModelSignals, TextAlignment,
};
use crate::widget::{
    FocusPolicy, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

/// An item in a [`ListWidget`].
///
/// Stores all the data for a single list item including text, icon, and custom data.
#[derive(Debug, Clone)]
pub struct ListWidgetItem {
    text: String,
    icon: Option<Icon>,
    tooltip: Option<String>,
    check_state: Option<CheckState>,
    flags: ItemFlags,
    data: HashMap<u32, ItemData>,
    background: Option<Color>,
    foreground: Option<Color>,
    text_alignment: TextAlignment,
    hidden: bool,
}

impl Default for ListWidgetItem {
    fn default() -> Self {
        Self::new("")
    }
}

impl ListWidgetItem {
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
            hidden: false,
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

    /// Makes the item checkable.
    pub fn set_checkable(&mut self, checkable: bool) {
        self.flags.checkable = checkable;
        if checkable && self.check_state.is_none() {
            self.check_state = Some(CheckState::Unchecked);
        }
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

    /// Returns whether the item is hidden.
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Sets whether the item is hidden.
    pub fn set_hidden(&mut self, hidden: bool) {
        self.hidden = hidden;
    }
}

/// Internal model for ListWidget that implements ItemModel.
struct ListWidgetModel {
    items: Arc<RwLock<Vec<ListWidgetItem>>>,
    signals: ModelSignals,
}

impl ListWidgetModel {
    fn new(items: Arc<RwLock<Vec<ListWidgetItem>>>) -> Self {
        Self {
            items,
            signals: ModelSignals::new(),
        }
    }
}

impl ItemModel for ListWidgetModel {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        if parent.is_valid() {
            0
        } else {
            self.items.read().len()
        }
    }

    fn column_count(&self, _parent: &ModelIndex) -> usize {
        1
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        if !index.is_valid() {
            return ItemData::None;
        }

        let items = self.items.read();
        let row = index.row();

        if row >= items.len() {
            return ItemData::None;
        }

        let item = &items[row];

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
        if parent.is_valid() || column > 0 {
            return ModelIndex::invalid();
        }

        if row >= self.items.read().len() {
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

        let items = self.items.read();
        if index.row() >= items.len() {
            return ItemFlags::disabled();
        }

        items[index.row()].flags
    }
}

/// Flags for matching items in searches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MatchFlags {
    /// Case-sensitive matching.
    pub case_sensitive: bool,
    /// Match items that contain the text.
    pub contains: bool,
    /// Match items that start with the text.
    pub starts_with: bool,
    /// Match items that end with the text.
    pub ends_with: bool,
}

impl MatchFlags {
    /// Default: case-insensitive exact match.
    pub fn exact() -> Self {
        Self::default()
    }

    /// Case-insensitive contains match.
    pub fn contains() -> Self {
        Self {
            contains: true,
            ..Default::default()
        }
    }
}

/// A model-less list widget for simple list displays.
///
/// `ListWidget` wraps a [`ListView`] and provides direct item manipulation
/// without requiring a separate model. This is convenient for simple use cases.
///
/// For more complex scenarios (large datasets, custom data types), use
/// [`ListView`] with an explicit model instead.
pub struct ListWidget {
    base: WidgetBase,
    items: Arc<RwLock<Vec<ListWidgetItem>>>,
    model: Arc<ListWidgetModel>,

    // Selection
    current_row: Option<usize>,
    selection_mode: SelectionMode,
    selected_rows: Vec<usize>,

    // Layout
    item_height: f32,
    spacing: f32,
    scroll_y: i32,
    content_height: f32,

    // Appearance
    background_color: Color,
    alternate_row_colors: bool,

    // Visual state
    hovered_row: Option<usize>,
    pressed_row: Option<usize>,

    // Signals
    /// Emitted when an item is clicked. Parameter is the row index.
    pub item_clicked: Signal<usize>,
    /// Emitted when an item is double-clicked. Parameter is the row index.
    pub item_double_clicked: Signal<usize>,
    /// Emitted when an item is activated (Enter or double-click). Parameter is the row index.
    pub item_activated: Signal<usize>,
    /// Emitted when the current row changes. Parameters are (old, new).
    pub current_row_changed: Signal<(Option<usize>, Option<usize>)>,
    /// Emitted when an item's data changes. Parameter is the row index.
    pub item_changed: Signal<usize>,
}

impl Default for ListWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl ListWidget {
    /// Creates a new empty ListWidget.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));

        let items = Arc::new(RwLock::new(Vec::new()));
        let model = Arc::new(ListWidgetModel::new(items.clone()));

        Self {
            base,
            items,
            model,
            current_row: None,
            selection_mode: SelectionMode::SingleSelection,
            selected_rows: Vec::new(),
            item_height: 24.0,
            spacing: 2.0,
            scroll_y: 0,
            content_height: 0.0,
            background_color: Color::WHITE,
            alternate_row_colors: false,
            hovered_row: None,
            pressed_row: None,
            item_clicked: Signal::new(),
            item_double_clicked: Signal::new(),
            item_activated: Signal::new(),
            current_row_changed: Signal::new(),
            item_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Item Management
    // =========================================================================

    /// Adds an item with the given text.
    pub fn add_item(&mut self, text: impl Into<String>) {
        let item = ListWidgetItem::new(text);
        self.add_item_object(item);
    }

    /// Adds an item with text and icon.
    pub fn add_item_with_icon(&mut self, text: impl Into<String>, icon: Icon) {
        let item = ListWidgetItem::with_icon(text, icon);
        self.add_item_object(item);
    }

    /// Adds a pre-configured item.
    pub fn add_item_object(&mut self, item: ListWidgetItem) {
        let row = self.items.read().len();
        self.model
            .signals
            .emit_rows_inserted(ModelIndex::invalid(), row, row, || {
                self.items.write().push(item);
            });
        self.update_content_height();
        self.base.update();
    }

    /// Inserts an item at the specified row.
    pub fn insert_item(&mut self, row: usize, text: impl Into<String>) {
        self.insert_item_object(row, ListWidgetItem::new(text));
    }

    /// Inserts a pre-configured item at the specified row.
    pub fn insert_item_object(&mut self, row: usize, item: ListWidgetItem) {
        let len = self.items.read().len();
        let row = row.min(len);

        self.model
            .signals
            .emit_rows_inserted(ModelIndex::invalid(), row, row, || {
                self.items.write().insert(row, item);
            });

        // Update current row if needed
        if let Some(current) = self.current_row
            && current >= row
        {
            self.current_row = Some(current + 1);
        }

        self.update_content_height();
        self.base.update();
    }

    /// Removes and returns the item at the specified row.
    pub fn take_item(&mut self, row: usize) -> Option<ListWidgetItem> {
        if row >= self.items.read().len() {
            return None;
        }

        let mut removed = None;
        self.model
            .signals
            .emit_rows_removed(ModelIndex::invalid(), row, row, || {
                removed = Some(self.items.write().remove(row));
            });

        // Update current row if needed
        if let Some(current) = self.current_row {
            if current == row {
                let len = self.items.read().len();
                self.current_row = if len > 0 {
                    Some(current.min(len - 1))
                } else {
                    None
                };
            } else if current > row {
                self.current_row = Some(current - 1);
            }
        }

        self.update_content_height();
        self.base.update();
        removed
    }

    /// Removes all items.
    pub fn clear(&mut self) {
        self.model.signals.emit_reset(|| {
            self.items.write().clear();
        });
        self.current_row = None;
        self.selected_rows.clear();
        self.update_content_height();
        self.base.update();
    }

    /// Returns the number of items.
    pub fn count(&self) -> usize {
        self.items.read().len()
    }

    /// Returns a reference to the item at the given row.
    pub fn item(&self, row: usize) -> Option<ListWidgetItem> {
        self.items.read().get(row).cloned()
    }

    /// Modifies an item at the given row.
    pub fn modify_item<F, R>(&mut self, row: usize, f: F) -> Option<R>
    where
        F: FnOnce(&mut ListWidgetItem) -> R,
    {
        let mut items = self.items.write();
        if row >= items.len() {
            return None;
        }

        let result = f(&mut items[row]);
        drop(items);

        let index = ModelIndex::new(row, 0, ModelIndex::invalid());
        self.model
            .signals
            .emit_data_changed_single(index, vec![ItemRole::Display]);
        self.item_changed.emit(row);
        self.base.update();
        Some(result)
    }

    /// Sets the text of an item.
    pub fn set_item_text(&mut self, row: usize, text: impl Into<String>) {
        self.modify_item(row, |item| item.set_text(text));
    }

    /// Sets the icon of an item.
    pub fn set_item_icon(&mut self, row: usize, icon: Option<Icon>) {
        self.modify_item(row, |item| item.set_icon(icon));
    }

    /// Finds items matching the given text.
    pub fn find_items(&self, text: &str, flags: MatchFlags) -> Vec<usize> {
        let items = self.items.read();
        let mut results = Vec::new();

        let search_text = if flags.case_sensitive {
            text.to_string()
        } else {
            text.to_lowercase()
        };

        for (i, item) in items.iter().enumerate() {
            let item_text = if flags.case_sensitive {
                item.text.clone()
            } else {
                item.text.to_lowercase()
            };

            let matches = if flags.contains {
                item_text.contains(&search_text)
            } else if flags.starts_with {
                item_text.starts_with(&search_text)
            } else if flags.ends_with {
                item_text.ends_with(&search_text)
            } else {
                item_text == search_text
            };

            if matches {
                results.push(i);
            }
        }

        results
    }

    // =========================================================================
    // Selection
    // =========================================================================

    /// Gets the current row (focused item).
    pub fn current_row(&self) -> Option<usize> {
        self.current_row
    }

    /// Sets the current row (focused item).
    pub fn set_current_row(&mut self, row: Option<usize>) {
        let row = row.and_then(|r| {
            if r < self.items.read().len() {
                Some(r)
            } else {
                None
            }
        });

        if self.current_row != row {
            let old = self.current_row;
            self.current_row = row;

            // In single selection mode, also update selection
            if self.selection_mode == SelectionMode::SingleSelection {
                self.selected_rows = row.into_iter().collect();
            }

            self.current_row_changed.emit((old, row));
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
            self.selected_rows.clear();
        } else if mode == SelectionMode::SingleSelection && self.selected_rows.len() > 1 {
            self.selected_rows = self.current_row.into_iter().collect();
        }
        self.base.update();
    }

    /// Returns the selected row indices.
    pub fn selected_rows(&self) -> &[usize] {
        &self.selected_rows
    }

    /// Selects all items.
    pub fn select_all(&mut self) {
        if self.selection_mode == SelectionMode::NoSelection
            || self.selection_mode == SelectionMode::SingleSelection
        {
            return;
        }

        let count = self.items.read().len();
        self.selected_rows = (0..count).collect();
        self.base.update();
    }

    /// Clears the selection.
    pub fn clear_selection(&mut self) {
        self.selected_rows.clear();
        self.base.update();
    }

    /// Returns whether a row is selected.
    pub fn is_row_selected(&self, row: usize) -> bool {
        self.selected_rows.contains(&row)
    }

    // =========================================================================
    // Appearance
    // =========================================================================

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
        self.update_content_height();
        self.base.update();
    }

    /// Sets the spacing between items.
    pub fn set_spacing(&mut self, spacing: f32) {
        self.spacing = spacing;
        self.update_content_height();
        self.base.update();
    }

    // =========================================================================
    // Scrolling
    // =========================================================================

    /// Scrolls to make a row visible.
    pub fn scroll_to_item(&mut self, row: usize) {
        let viewport_height = self.base.rect().height();
        let item_top = row as f32 * (self.item_height + self.spacing);
        let item_bottom = item_top + self.item_height;

        let viewport_top = self.scroll_y as f32;
        let viewport_bottom = viewport_top + viewport_height;

        if item_top < viewport_top {
            self.scroll_y = item_top as i32;
        } else if item_bottom > viewport_bottom {
            self.scroll_y = (item_bottom - viewport_height) as i32;
        }

        self.scroll_y = self.scroll_y.max(0);
        self.base.update();
    }

    // =========================================================================
    // Internal
    // =========================================================================

    fn update_content_height(&mut self) {
        let count = self.items.read().len();
        if count == 0 {
            self.content_height = 0.0;
        } else {
            self.content_height =
                count as f32 * self.item_height + (count - 1) as f32 * self.spacing;
        }
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

        let row = (content_y / (self.item_height + self.spacing)) as usize;
        let count = self.items.read().len();

        if row < count { Some(row) } else { None }
    }

    fn row_rect(&self, row: usize) -> Rect {
        let y = row as f32 * (self.item_height + self.spacing) - self.scroll_y as f32;
        Rect::new(0.0, y, self.base.rect().width(), self.item_height)
    }

    fn handle_click(&mut self, row: usize, modifiers: &crate::widget::KeyboardModifiers) {
        match self.selection_mode {
            SelectionMode::NoSelection => {
                self.set_current_row(Some(row));
            }
            SelectionMode::SingleSelection => {
                self.set_current_row(Some(row));
            }
            SelectionMode::MultiSelection | SelectionMode::ExtendedSelection => {
                if modifiers.control {
                    // Toggle selection
                    if let Some(pos) = self.selected_rows.iter().position(|&r| r == row) {
                        self.selected_rows.remove(pos);
                    } else {
                        self.selected_rows.push(row);
                    }
                } else if modifiers.shift {
                    // Range selection
                    if let Some(anchor) = self.current_row {
                        let (start, end) = if anchor <= row {
                            (anchor, row)
                        } else {
                            (row, anchor)
                        };
                        self.selected_rows = (start..=end).collect();
                    } else {
                        self.selected_rows = vec![row];
                    }
                } else {
                    self.selected_rows = vec![row];
                }
                let old = self.current_row;
                self.current_row = Some(row);
                if old != self.current_row {
                    self.current_row_changed.emit((old, self.current_row));
                }
            }
        }

        self.item_clicked.emit(row);
        self.base.update();
    }
}

impl Object for ListWidget {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ListWidget {
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
        let rect = self.base.rect();

        // Background
        ctx.renderer().fill_rect(rect, self.background_color);

        // Items
        let items = self.items.read();
        let viewport_height = rect.height();
        let first_visible = (self.scroll_y as f32 / (self.item_height + self.spacing)) as usize;
        let visible_count =
            (viewport_height / (self.item_height + self.spacing)).ceil() as usize + 1;

        for row in first_visible..(first_visible + visible_count).min(items.len()) {
            let item = &items[row];
            if item.hidden {
                continue;
            }

            let item_rect = self.row_rect(row);
            if item_rect.origin.y > viewport_height || item_rect.origin.y + item_rect.height() < 0.0
            {
                continue;
            }

            // Background
            let bg_color = if self.is_row_selected(row) {
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

            ctx.renderer().fill_rect(item_rect, bg_color);

            // Text - for now just draw a placeholder line showing where text would be
            // Proper text rendering requires FontSystem integration
            let text_color = if self.is_row_selected(row) {
                Color::WHITE
            } else if let Some(fg) = item.foreground {
                fg
            } else {
                Color::BLACK
            };

            // Draw a simple text placeholder (horizontal line)
            let text_x = item_rect.origin.x + 8.0;
            let text_y = item_rect.origin.y + item_rect.height() / 2.0;
            let text_width = (item.text.len() as f32 * 7.0).min(item_rect.width() - 16.0);
            if text_width > 0.0 {
                ctx.renderer()
                    .fill_rect(Rect::new(text_x, text_y - 1.0, text_width, 2.0), text_color);
            }

            // Focus indicator
            if self.current_row == Some(row) && self.base.has_focus() {
                let focus_rect = Rect::new(
                    item_rect.origin.x + 1.0,
                    item_rect.origin.y + 1.0,
                    item_rect.width() - 2.0,
                    item_rect.height() - 2.0,
                );
                let stroke = Stroke::new(Color::from_rgb8(100, 100, 100), 1.0);
                ctx.renderer().stroke_rect(focus_rect, &stroke);
            }
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                if e.button == crate::widget::MouseButton::Left
                    && let Some(row) = self.row_at_y(e.local_pos.y)
                {
                    self.pressed_row = Some(row);
                    self.handle_click(row, &e.modifiers);
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if e.button == crate::widget::MouseButton::Left {
                    if let Some(pressed) = self.pressed_row.take()
                        && let Some(row) = self.row_at_y(e.local_pos.y)
                        && row == pressed
                    {
                        // Check for double-click would go here
                    }
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
                let count = self.items.read().len();
                if count == 0 {
                    return false;
                }

                match e.key {
                    crate::widget::Key::ArrowUp => {
                        if let Some(current) = self.current_row {
                            if current > 0 {
                                self.set_current_row(Some(current - 1));
                                self.scroll_to_item(current - 1);
                            }
                        } else {
                            self.set_current_row(Some(0));
                        }
                        return true;
                    }
                    crate::widget::Key::ArrowDown => {
                        if let Some(current) = self.current_row {
                            if current + 1 < count {
                                self.set_current_row(Some(current + 1));
                                self.scroll_to_item(current + 1);
                            }
                        } else {
                            self.set_current_row(Some(0));
                        }
                        return true;
                    }
                    crate::widget::Key::Home => {
                        self.set_current_row(Some(0));
                        self.scroll_to_item(0);
                        return true;
                    }
                    crate::widget::Key::End => {
                        self.set_current_row(Some(count - 1));
                        self.scroll_to_item(count - 1);
                        return true;
                    }
                    crate::widget::Key::PageUp => {
                        let page_size = (self.base.rect().height()
                            / (self.item_height + self.spacing))
                            as usize;
                        if let Some(current) = self.current_row {
                            let new_row = current.saturating_sub(page_size.max(1));
                            self.set_current_row(Some(new_row));
                            self.scroll_to_item(new_row);
                        }
                        return true;
                    }
                    crate::widget::Key::PageDown => {
                        let page_size = (self.base.rect().height()
                            / (self.item_height + self.spacing))
                            as usize;
                        if let Some(current) = self.current_row {
                            let new_row = (current + page_size.max(1)).min(count - 1);
                            self.set_current_row(Some(new_row));
                            self.scroll_to_item(new_row);
                        }
                        return true;
                    }
                    crate::widget::Key::Enter | crate::widget::Key::NumpadEnter => {
                        if let Some(row) = self.current_row {
                            self.item_activated.emit(row);
                        }
                        return true;
                    }
                    crate::widget::Key::A if e.modifiers.control => {
                        self.select_all();
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
                // Clamp scroll position
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
    fn test_list_widget_creation() {
        setup();
        let widget = ListWidget::new();
        assert_eq!(widget.count(), 0);
        assert!(widget.current_row().is_none());
    }

    #[test]
    fn test_add_items() {
        setup();
        let mut widget = ListWidget::new();
        widget.add_item("First");
        widget.add_item("Second");
        widget.add_item("Third");

        assert_eq!(widget.count(), 3);
        assert_eq!(widget.item(0).unwrap().text(), "First");
        assert_eq!(widget.item(1).unwrap().text(), "Second");
        assert_eq!(widget.item(2).unwrap().text(), "Third");
    }

    #[test]
    fn test_insert_item() {
        setup();
        let mut widget = ListWidget::new();
        widget.add_item("First");
        widget.add_item("Third");
        widget.insert_item(1, "Second");

        assert_eq!(widget.count(), 3);
        assert_eq!(widget.item(0).unwrap().text(), "First");
        assert_eq!(widget.item(1).unwrap().text(), "Second");
        assert_eq!(widget.item(2).unwrap().text(), "Third");
    }

    #[test]
    fn test_take_item() {
        setup();
        let mut widget = ListWidget::new();
        widget.add_item("First");
        widget.add_item("Second");
        widget.add_item("Third");

        let removed = widget.take_item(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().text(), "Second");
        assert_eq!(widget.count(), 2);
        assert_eq!(widget.item(1).unwrap().text(), "Third");
    }

    #[test]
    fn test_clear() {
        setup();
        let mut widget = ListWidget::new();
        widget.add_item("First");
        widget.add_item("Second");
        widget.set_current_row(Some(0));

        widget.clear();

        assert_eq!(widget.count(), 0);
        assert!(widget.current_row().is_none());
    }

    #[test]
    fn test_selection() {
        setup();
        let mut widget = ListWidget::new();
        widget.add_item("First");
        widget.add_item("Second");

        widget.set_current_row(Some(1));
        assert_eq!(widget.current_row(), Some(1));
        assert!(widget.is_row_selected(1));
    }

    #[test]
    fn test_find_items() {
        setup();
        let mut widget = ListWidget::new();
        widget.add_item("Apple");
        widget.add_item("Banana");
        widget.add_item("Apricot");

        let results = widget.find_items("ap", MatchFlags::contains());
        assert_eq!(results.len(), 2);
        assert!(results.contains(&0));
        assert!(results.contains(&2));
    }
}
