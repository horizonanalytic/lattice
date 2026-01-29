//! ComboBox widget for dropdown selection.
//!
//! The ComboBox widget provides a dropdown selection with:
//! - Dropdown selection with popup list
//! - Editable mode with text filtering
//! - Model-based items with optional icons
//! - Custom item rendering via delegate
//! - Keyboard and mouse navigation
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{ComboBox, ComboBoxModel, StringListComboModel};
//!
//! // Create a simple combobox with string items
//! let items = vec!["Apple", "Banana", "Cherry", "Date"];
//! let mut combo = ComboBox::new()
//!     .with_model(Box::new(StringListComboModel::from(items)));
//!
//! // Create an editable combobox for filtering
//! let mut editable_combo = ComboBox::new()
//!     .with_model(Box::new(StringListComboModel::from(items)))
//!     .with_editable(true);
//!
//! // Connect to signals
//! combo.current_index_changed.connect(|&idx| {
//!     println!("Selected index: {}", idx);
//! });
//!
//! combo.current_text_changed.connect(|text| {
//!     println!("Selected text: {}", text);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Icon, ImageScaleMode, Point, Rect, Renderer, RoundedRect,
    Size, Stroke, TextLayout, TextLayoutOptions, TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, WheelEvent, Widget,
    WidgetBase, WidgetEvent,
};

// ============================================================================
// ComboBox Model Trait
// ============================================================================

/// An item in a ComboBox with text and optional icon.
#[derive(Debug)]
pub struct ComboBoxItem {
    /// The display text for this item.
    pub text: String,
    /// Optional icon for this item.
    pub icon: Option<Icon>,
}

impl ComboBoxItem {
    /// Create a new item with just text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            icon: None,
        }
    }

    /// Create a new item with text and icon.
    pub fn with_icon(text: impl Into<String>, icon: Icon) -> Self {
        Self {
            text: text.into(),
            icon: Some(icon),
        }
    }
}

impl Clone for ComboBoxItem {
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            icon: self.icon.clone(),
        }
    }
}

/// Trait for providing items to a ComboBox.
///
/// Implement this trait to provide custom data sources for the ComboBox.
pub trait ComboBoxModel: Send + Sync {
    /// Get the number of items in the model.
    fn row_count(&self) -> usize;

    /// Get the item at the given index.
    ///
    /// Returns `None` if the index is out of bounds.
    fn item(&self, index: usize) -> Option<ComboBoxItem>;

    /// Get just the text at the given index (for efficiency).
    fn text(&self, index: usize) -> Option<String> {
        self.item(index).map(|item| item.text)
    }

    /// Get just the icon at the given index (for efficiency).
    fn icon(&self, index: usize) -> Option<Icon> {
        self.item(index).and_then(|item| item.icon)
    }

    /// Find the index of an item by text.
    ///
    /// Returns the first matching index, or `None` if not found.
    fn find_text(&self, text: &str) -> Option<usize> {
        for i in 0..self.row_count() {
            if let Some(item_text) = self.text(i)
                && item_text == text
            {
                return Some(i);
            }
        }
        None
    }

    /// Get items matching a filter prefix (for editable mode).
    ///
    /// Returns indices of items that match the filter.
    fn filter(&self, prefix: &str, case_insensitive: bool) -> Vec<usize> {
        let prefix_cmp = if case_insensitive {
            prefix.to_lowercase()
        } else {
            prefix.to_string()
        };

        (0..self.row_count())
            .filter(|&i| {
                if let Some(text) = self.text(i) {
                    let text_cmp = if case_insensitive {
                        text.to_lowercase()
                    } else {
                        text
                    };
                    text_cmp.starts_with(&prefix_cmp)
                } else {
                    false
                }
            })
            .collect()
    }
}

// ============================================================================
// String List ComboBox Model
// ============================================================================

/// A simple ComboBox model backed by a list of strings.
#[derive(Debug, Clone, Default)]
pub struct StringListComboModel {
    items: Vec<String>,
}

impl StringListComboModel {
    /// Create a new model with the given items.
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }

    /// Create an empty model.
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Get a reference to the items.
    pub fn items(&self) -> &[String] {
        &self.items
    }

    /// Set the items.
    pub fn set_items(&mut self, items: Vec<String>) {
        self.items = items;
    }

    /// Add an item.
    pub fn add_item(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }

    /// Remove an item by index.
    pub fn remove_item(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
        }
    }

    /// Insert an item at a specific index.
    pub fn insert_item(&mut self, index: usize, item: impl Into<String>) {
        if index <= self.items.len() {
            self.items.insert(index, item.into());
        }
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl ComboBoxModel for StringListComboModel {
    fn row_count(&self) -> usize {
        self.items.len()
    }

    fn item(&self, index: usize) -> Option<ComboBoxItem> {
        self.items.get(index).map(ComboBoxItem::new)
    }

    fn text(&self, index: usize) -> Option<String> {
        self.items.get(index).cloned()
    }

    fn icon(&self, _index: usize) -> Option<Icon> {
        None
    }
}

impl From<Vec<String>> for StringListComboModel {
    fn from(items: Vec<String>) -> Self {
        Self::new(items)
    }
}

impl From<Vec<&str>> for StringListComboModel {
    fn from(items: Vec<&str>) -> Self {
        Self::new(items.into_iter().map(String::from).collect())
    }
}

impl<const N: usize> From<[&str; N]> for StringListComboModel {
    fn from(items: [&str; N]) -> Self {
        Self::new(items.into_iter().map(String::from).collect())
    }
}

// ============================================================================
// Icon List ComboBox Model
// ============================================================================

/// A ComboBox model with items that have both text and icons.
#[derive(Debug, Clone, Default)]
pub struct IconListComboModel {
    items: Vec<ComboBoxItem>,
}

impl IconListComboModel {
    /// Create a new model with the given items.
    pub fn new(items: Vec<ComboBoxItem>) -> Self {
        Self { items }
    }

    /// Create an empty model.
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Add an item with text only.
    pub fn add_text(&mut self, text: impl Into<String>) {
        self.items.push(ComboBoxItem::new(text));
    }

    /// Add an item with text and icon.
    pub fn add_item(&mut self, text: impl Into<String>, icon: Icon) {
        self.items.push(ComboBoxItem::with_icon(text, icon));
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.items.clear();
    }
}

impl ComboBoxModel for IconListComboModel {
    fn row_count(&self) -> usize {
        self.items.len()
    }

    fn item(&self, index: usize) -> Option<ComboBoxItem> {
        self.items.get(index).cloned()
    }

    fn text(&self, index: usize) -> Option<String> {
        self.items.get(index).map(|item| item.text.clone())
    }

    fn icon(&self, index: usize) -> Option<Icon> {
        self.items.get(index).and_then(|item| item.icon.clone())
    }
}

// ============================================================================
// Item Delegate Trait
// ============================================================================

/// Trait for custom rendering of ComboBox items.
///
/// Implement this to customize how items are displayed in the dropdown list.
pub trait ComboBoxItemDelegate: Send + Sync {
    /// Paint an item in the dropdown list.
    ///
    /// # Arguments
    /// * `ctx` - The paint context
    /// * `rect` - The rectangle to paint into
    /// * `item` - The item to paint
    /// * `index` - The item index
    /// * `selected` - Whether this item is currently selected/highlighted
    /// * `hovered` - Whether the mouse is over this item
    fn paint_item(
        &self,
        ctx: &mut PaintContext<'_>,
        rect: Rect,
        item: &ComboBoxItem,
        index: usize,
        selected: bool,
        hovered: bool,
    );

    /// Get the size hint for an item.
    ///
    /// Returns `None` to use the default item height.
    fn size_hint(&self, _item: &ComboBoxItem, _index: usize) -> Option<Size> {
        None
    }
}

/// Default item delegate that renders text and optional icon.
#[derive(Debug, Clone)]
pub struct DefaultComboBoxDelegate {
    /// Font for rendering text.
    pub font: Font,
    /// Text color.
    pub text_color: Color,
    /// Selected text color.
    pub selected_text_color: Color,
    /// Selection background color.
    pub selection_color: Color,
    /// Hover background color.
    pub hover_color: Color,
    /// Icon size.
    pub icon_size: f32,
    /// Padding.
    pub padding: f32,
}

impl Default for DefaultComboBoxDelegate {
    fn default() -> Self {
        Self {
            font: Font::new(FontFamily::SansSerif, 13.0),
            text_color: Color::BLACK,
            selected_text_color: Color::WHITE,
            selection_color: Color::from_rgba8(51, 153, 255, 255),
            hover_color: Color::from_rgba8(200, 200, 200, 100),
            icon_size: 16.0,
            padding: 4.0,
        }
    }
}

impl ComboBoxItemDelegate for DefaultComboBoxDelegate {
    fn paint_item(
        &self,
        ctx: &mut PaintContext<'_>,
        rect: Rect,
        item: &ComboBoxItem,
        _index: usize,
        selected: bool,
        hovered: bool,
    ) {
        // Draw background
        if selected {
            ctx.renderer().fill_rect(rect, self.selection_color);
        } else if hovered {
            ctx.renderer().fill_rect(rect, self.hover_color);
        }

        let text_color = if selected {
            self.selected_text_color
        } else {
            self.text_color
        };

        let mut text_x = rect.origin.x + self.padding;

        // Draw icon if present
        if let Some(icon) = &item.icon
            && let Some(image) = icon.image()
        {
            let icon_y = rect.origin.y + (rect.height() - self.icon_size) / 2.0;
            let icon_rect = Rect::new(text_x, icon_y, self.icon_size, self.icon_size);
            ctx.renderer()
                .draw_image(image, icon_rect, ImageScaleMode::Fit);
            text_x += self.icon_size + self.padding;
        }

        // Draw text
        let mut font_system = FontSystem::new();
        let layout = TextLayout::with_options(
            &mut font_system,
            &item.text,
            &self.font,
            TextLayoutOptions::new(),
        );

        let text_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                text_color,
            );
        }
    }
}

// ============================================================================
// ComboBox Parts
// ============================================================================

/// Parts of the ComboBox for hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ComboBoxPart {
    #[default]
    None,
    /// The text/display area.
    Display,
    /// The dropdown arrow button.
    Arrow,
    /// An item in the popup list.
    PopupItem(usize),
}

// ============================================================================
// ComboBox Widget
// ============================================================================

/// A dropdown selection widget.
///
/// ComboBox provides a compact way to present a list of choices. It shows the
/// currently selected item and opens a dropdown list when clicked.
///
/// # Features
///
/// - Non-editable mode: Click to show dropdown, select an item
/// - Editable mode: Type to filter items, select from filtered list
/// - Model-based: Supports any data source implementing [`ComboBoxModel`]
/// - Icons: Items can have optional icons
/// - Custom rendering: Use [`ComboBoxItemDelegate`] for custom item appearance
/// - Keyboard navigation: Arrow keys, Enter, Escape
///
/// # Signals
///
/// - `current_index_changed(i32)`: Emitted when the selected index changes
/// - `current_text_changed(String)`: Emitted when the selected text changes
/// - `activated(i32)`: Emitted when an item is activated (selected by user action)
/// - `editing_finished()`: Emitted when editing is finished (editable mode only)
pub struct ComboBox {
    /// Widget base.
    base: WidgetBase,

    /// The data model.
    model: Option<Box<dyn ComboBoxModel>>,

    /// Current selected index (-1 means no selection).
    current_index: i32,

    /// Whether the combobox is editable.
    editable: bool,

    /// Current edit text (for editable mode).
    edit_text: String,

    /// Cursor position in edit text.
    cursor_pos: usize,

    /// Selection start in edit text.
    selection_start: Option<usize>,

    /// Whether we're currently editing.
    is_editing: bool,

    /// Placeholder text for editable mode.
    placeholder: String,

    /// Whether the popup is currently visible.
    popup_visible: bool,

    /// Highlighted index in popup (-1 means no highlight).
    highlighted_index: i32,

    /// Filtered indices for editable mode.
    filtered_indices: Vec<usize>,

    /// Whether filtering is active.
    filtering_active: bool,

    /// Case insensitive filtering.
    case_insensitive: bool,

    /// Maximum visible items in popup.
    max_visible_items: usize,

    /// Scroll offset in popup.
    scroll_offset: usize,

    /// Item height in popup.
    item_height: f32,

    /// Popup width (None = match combobox width).
    popup_width: Option<f32>,

    // Appearance
    /// Background color.
    background_color: Color,
    /// Text color.
    text_color: Color,
    /// Placeholder text color.
    placeholder_color: Color,
    /// Border color.
    border_color: Color,
    /// Focus border color.
    focus_border_color: Color,
    /// Arrow button color.
    arrow_color: Color,
    /// Arrow button hover color.
    arrow_hover_color: Color,
    /// Popup background color.
    popup_background_color: Color,
    /// Popup border color.
    popup_border_color: Color,

    /// Font for text.
    font: Font,
    /// Border radius.
    border_radius: f32,
    /// Arrow button width.
    arrow_width: f32,
    /// Padding inside the widget.
    padding: f32,

    /// Current hover part.
    hover_part: ComboBoxPart,
    /// Current pressed part.
    pressed_part: ComboBoxPart,

    /// Item delegate for custom rendering.
    delegate: Box<dyn ComboBoxItemDelegate>,

    // Signals
    /// Signal emitted when the current index changes.
    pub current_index_changed: Signal<i32>,
    /// Signal emitted when the current text changes.
    pub current_text_changed: Signal<String>,
    /// Signal emitted when an item is activated.
    pub activated: Signal<i32>,
    /// Signal emitted when editing is finished (editable mode).
    pub editing_finished: Signal<()>,
}

impl ComboBox {
    /// Create a new ComboBox with default settings.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Fixed,
        ));

        Self {
            base,
            model: None,
            current_index: -1,
            editable: false,
            edit_text: String::new(),
            cursor_pos: 0,
            selection_start: None,
            is_editing: false,
            placeholder: String::new(),
            popup_visible: false,
            highlighted_index: -1,
            filtered_indices: Vec::new(),
            filtering_active: false,
            case_insensitive: true,
            max_visible_items: 10,
            scroll_offset: 0,
            item_height: 24.0,
            popup_width: None,
            background_color: Color::WHITE,
            text_color: Color::BLACK,
            placeholder_color: Color::from_rgb8(160, 160, 160),
            border_color: Color::from_rgb8(180, 180, 180),
            focus_border_color: Color::from_rgb8(51, 153, 255),
            arrow_color: Color::from_rgb8(100, 100, 100),
            arrow_hover_color: Color::from_rgb8(51, 153, 255),
            popup_background_color: Color::WHITE,
            popup_border_color: Color::from_rgb8(180, 180, 180),
            font: Font::new(FontFamily::SansSerif, 13.0),
            border_radius: 4.0,
            arrow_width: 24.0,
            padding: 6.0,
            hover_part: ComboBoxPart::None,
            pressed_part: ComboBoxPart::None,
            delegate: Box::new(DefaultComboBoxDelegate::default()),
            current_index_changed: Signal::new(),
            current_text_changed: Signal::new(),
            activated: Signal::new(),
            editing_finished: Signal::new(),
        }
    }

    // =========================================================================
    // Model
    // =========================================================================

    /// Set the data model.
    pub fn set_model(&mut self, model: Box<dyn ComboBoxModel>) {
        self.model = Some(model);
        // Reset selection if current index is out of bounds
        if let Some(m) = &self.model
            && self.current_index >= m.row_count() as i32
        {
            self.set_current_index(-1);
        }
        self.base.update();
    }

    /// Set model using builder pattern.
    pub fn with_model(mut self, model: Box<dyn ComboBoxModel>) -> Self {
        self.set_model(model);
        self
    }

    /// Get the number of items.
    pub fn count(&self) -> usize {
        self.model.as_ref().map_or(0, |m| m.row_count())
    }

    /// Get the text at an index.
    pub fn item_text(&self, index: usize) -> Option<String> {
        self.model.as_ref().and_then(|m| m.text(index))
    }

    /// Get the icon at an index.
    pub fn item_icon(&self, index: usize) -> Option<Icon> {
        self.model.as_ref().and_then(|m| m.icon(index))
    }

    /// Find the index of an item by text.
    pub fn find_text(&self, text: &str) -> Option<usize> {
        self.model.as_ref().and_then(|m| m.find_text(text))
    }

    // =========================================================================
    // Convenience methods for adding items (requires mutable model)
    // =========================================================================

    /// Add a string item (only works with StringListComboModel).
    ///
    /// Note: This is a convenience method. For more control, modify the model directly.
    pub fn add_item(&mut self, text: impl Into<String>) {
        // We need to work around the fact that we can't downcast the boxed trait object
        // In practice, users should modify their model directly
        let text = text.into();
        if self.model.is_none() {
            self.model = Some(Box::new(StringListComboModel::new(vec![text])));
        } else {
            // Can't add to existing model through trait - user should modify model directly
        }
        self.base.update();
    }

    /// Add multiple string items.
    pub fn add_items(&mut self, texts: impl IntoIterator<Item = impl Into<String>>) {
        let items: Vec<String> = texts.into_iter().map(Into::into).collect();
        if self.model.is_none() {
            self.model = Some(Box::new(StringListComboModel::new(items)));
        }
        self.base.update();
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.model = None;
        self.set_current_index(-1);
        self.base.update();
    }

    // =========================================================================
    // Current Selection
    // =========================================================================

    /// Get the current selected index (-1 if no selection).
    pub fn current_index(&self) -> i32 {
        self.current_index
    }

    /// Set the current selected index.
    pub fn set_current_index(&mut self, index: i32) {
        let count = self.count() as i32;
        let new_index = if index < 0 || index >= count {
            -1
        } else {
            index
        };

        if self.current_index != new_index {
            self.current_index = new_index;

            // Update edit text for editable mode
            if self.editable
                && new_index >= 0
                && let Some(text) = self.item_text(new_index as usize)
            {
                self.edit_text = text.clone();
                self.cursor_pos = self.edit_text.len();
                self.selection_start = None;
            }

            self.base.update();
            self.current_index_changed.emit(new_index);

            // Emit text changed
            if new_index >= 0 {
                if let Some(text) = self.item_text(new_index as usize) {
                    self.current_text_changed.emit(text);
                }
            } else {
                self.current_text_changed.emit(String::new());
            }
        }
    }

    /// Set current index using builder pattern.
    pub fn with_current_index(mut self, index: i32) -> Self {
        self.set_current_index(index);
        self
    }

    /// Get the current selected text.
    pub fn current_text(&self) -> String {
        if self.editable {
            self.edit_text.clone()
        } else if self.current_index >= 0 {
            self.item_text(self.current_index as usize)
                .unwrap_or_default()
        } else {
            String::new()
        }
    }

    /// Set the current text (editable mode only).
    ///
    /// This sets the edit text and tries to find a matching item.
    pub fn set_current_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.editable {
            self.edit_text = text.clone();
            self.cursor_pos = self.edit_text.len();
            self.selection_start = None;
        }

        // Try to find matching item
        if let Some(idx) = self.find_text(&text) {
            self.set_current_index(idx as i32);
        } else if self.editable {
            // No match, but in editable mode we keep the text
            self.current_index = -1;
            self.current_text_changed.emit(text);
        }

        self.base.update();
    }

    /// Set current text using builder pattern.
    pub fn with_current_text(mut self, text: impl Into<String>) -> Self {
        self.set_current_text(text);
        self
    }

    // =========================================================================
    // Editable Mode
    // =========================================================================

    /// Check if the combobox is editable.
    pub fn is_editable(&self) -> bool {
        self.editable
    }

    /// Set whether the combobox is editable.
    pub fn set_editable(&mut self, editable: bool) {
        if self.editable != editable {
            self.editable = editable;

            // Initialize edit text from current selection
            if editable
                && self.current_index >= 0
                && let Some(text) = self.item_text(self.current_index as usize)
            {
                self.edit_text = text;
                self.cursor_pos = self.edit_text.len();
            }

            self.base.update();
        }
    }

    /// Set editable using builder pattern.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.set_editable(editable);
        self
    }

    /// Get the placeholder text.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Set placeholder text for editable mode.
    pub fn set_placeholder(&mut self, text: impl Into<String>) {
        self.placeholder = text.into();
        self.base.update();
    }

    /// Set placeholder using builder pattern.
    pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    // =========================================================================
    // Popup Configuration
    // =========================================================================

    /// Check if the popup is visible.
    pub fn is_popup_visible(&self) -> bool {
        self.popup_visible
    }

    /// Get the maximum number of visible items in the popup.
    pub fn max_visible_items(&self) -> usize {
        self.max_visible_items
    }

    /// Set the maximum number of visible items in the popup.
    pub fn set_max_visible_items(&mut self, count: usize) {
        self.max_visible_items = count.max(1);
    }

    /// Set max visible items using builder pattern.
    pub fn with_max_visible_items(mut self, count: usize) -> Self {
        self.max_visible_items = count.max(1);
        self
    }

    /// Get whether filtering is case insensitive.
    pub fn is_case_insensitive(&self) -> bool {
        self.case_insensitive
    }

    /// Set whether filtering is case insensitive.
    pub fn set_case_insensitive(&mut self, case_insensitive: bool) {
        self.case_insensitive = case_insensitive;
    }

    /// Set case insensitivity using builder pattern.
    pub fn with_case_insensitive(mut self, case_insensitive: bool) -> Self {
        self.case_insensitive = case_insensitive;
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
        self.base.update();
    }

    /// Set background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        self.text_color = color;
        self.base.update();
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the font.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
        self.base.update();
    }

    /// Set font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Set the border radius.
    pub fn set_border_radius(&mut self, radius: f32) {
        self.border_radius = radius;
        self.base.update();
    }

    /// Set border radius using builder pattern.
    pub fn with_border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    /// Set a custom item delegate.
    pub fn set_delegate(&mut self, delegate: Box<dyn ComboBoxItemDelegate>) {
        self.delegate = delegate;
        self.base.update();
    }

    /// Set delegate using builder pattern.
    pub fn with_delegate(mut self, delegate: Box<dyn ComboBoxItemDelegate>) -> Self {
        self.delegate = delegate;
        self
    }

    // =========================================================================
    // Popup Control
    // =========================================================================

    /// Show the dropdown popup.
    pub fn show_popup(&mut self) {
        if self.count() == 0 {
            return;
        }

        self.popup_visible = true;
        self.update_filtered_indices();

        // Set highlighted to current selection or first item
        if self.current_index >= 0 {
            self.highlighted_index = if self.filtering_active {
                // Find current index in filtered list
                self.filtered_indices
                    .iter()
                    .position(|&i| i == self.current_index as usize)
                    .map(|i| i as i32)
                    .unwrap_or(0)
            } else {
                self.current_index
            };
        } else {
            self.highlighted_index = 0;
        }

        self.ensure_highlighted_visible();
        self.base.update();
    }

    /// Hide the dropdown popup.
    pub fn hide_popup(&mut self) {
        if self.popup_visible {
            self.popup_visible = false;
            self.highlighted_index = -1;
            self.filtering_active = false;
            self.filtered_indices.clear();
            self.base.update();
        }
    }

    /// Toggle the popup visibility.
    pub fn toggle_popup(&mut self) {
        if self.popup_visible {
            self.hide_popup();
        } else {
            self.show_popup();
        }
    }

    fn update_filtered_indices(&mut self) {
        if self.editable && !self.edit_text.is_empty() {
            if let Some(model) = &self.model {
                self.filtered_indices = model.filter(&self.edit_text, self.case_insensitive);
                self.filtering_active = true;
            }
        } else {
            self.filtering_active = false;
            self.filtered_indices.clear();
        }
    }

    fn effective_item_count(&self) -> usize {
        if self.filtering_active {
            self.filtered_indices.len()
        } else {
            self.count()
        }
    }

    fn actual_index(&self, visual_index: usize) -> usize {
        if self.filtering_active {
            self.filtered_indices
                .get(visual_index)
                .copied()
                .unwrap_or(visual_index)
        } else {
            visual_index
        }
    }

    fn ensure_highlighted_visible(&mut self) {
        if self.highlighted_index < 0 {
            return;
        }

        let idx = self.highlighted_index as usize;
        if idx < self.scroll_offset {
            self.scroll_offset = idx;
        } else if idx >= self.scroll_offset + self.max_visible_items {
            self.scroll_offset = idx - self.max_visible_items + 1;
        }
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    fn display_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(0.0, 0.0, rect.width() - self.arrow_width, rect.height())
    }

    fn arrow_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            rect.width() - self.arrow_width,
            0.0,
            self.arrow_width,
            rect.height(),
        )
    }

    fn popup_rect(&self) -> Rect {
        let rect = self.base.rect();
        let item_count = self.effective_item_count();
        let visible_count = item_count.min(self.max_visible_items);
        let popup_height = visible_count as f32 * self.item_height + 2.0; // +2 for border
        let popup_width = self.popup_width.unwrap_or(rect.width());

        Rect::new(0.0, rect.height(), popup_width, popup_height)
    }

    fn hit_test(&self, pos: Point) -> ComboBoxPart {
        let rect = self.base.rect();
        let local_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());

        if !local_rect.contains(pos) {
            // Check popup
            if self.popup_visible {
                let popup_rect = self.popup_rect();
                if popup_rect.contains(pos) {
                    let local_y = pos.y - popup_rect.origin.y - 1.0; // -1 for border
                    let visual_idx = (local_y / self.item_height) as usize + self.scroll_offset;
                    if visual_idx < self.effective_item_count() {
                        return ComboBoxPart::PopupItem(visual_idx);
                    }
                }
            }
            return ComboBoxPart::None;
        }

        if self.arrow_rect().contains(pos) {
            ComboBoxPart::Arrow
        } else {
            ComboBoxPart::Display
        }
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let part = self.hit_test(event.local_pos);
        self.pressed_part = part;

        match part {
            ComboBoxPart::Arrow => {
                self.toggle_popup();
                self.base.update();
                true
            }
            ComboBoxPart::Display => {
                if self.editable {
                    self.is_editing = true;
                    // Set cursor position based on click
                    // For simplicity, just put cursor at end
                    self.cursor_pos = self.edit_text.len();
                    self.selection_start = None;
                } else {
                    self.toggle_popup();
                }
                self.base.update();
                true
            }
            ComboBoxPart::PopupItem(visual_idx) => {
                let actual_idx = self.actual_index(visual_idx);
                self.set_current_index(actual_idx as i32);
                self.activated.emit(actual_idx as i32);
                self.hide_popup();
                true
            }
            ComboBoxPart::None => {
                // Click outside - close popup
                if self.popup_visible {
                    self.hide_popup();
                    true
                } else {
                    false
                }
            }
        }
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        self.pressed_part = ComboBoxPart::None;
        self.base.update();
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let part = self.hit_test(event.local_pos);

        if part != self.hover_part {
            self.hover_part = part;

            // Update highlighted item in popup
            if let ComboBoxPart::PopupItem(visual_idx) = part {
                self.highlighted_index = visual_idx as i32;
            }

            self.base.update();
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let item_count = self.effective_item_count();

        match event.key {
            Key::Escape => {
                if self.popup_visible {
                    self.hide_popup();
                    return true;
                }
            }
            Key::Enter => {
                if self.popup_visible && self.highlighted_index >= 0 {
                    let actual_idx = self.actual_index(self.highlighted_index as usize);
                    self.set_current_index(actual_idx as i32);
                    self.activated.emit(actual_idx as i32);
                    self.hide_popup();
                    return true;
                } else if self.editable {
                    self.editing_finished.emit(());
                    return true;
                }
            }
            Key::ArrowDown => {
                if self.popup_visible {
                    if self.highlighted_index < item_count as i32 - 1 {
                        self.highlighted_index += 1;
                        self.ensure_highlighted_visible();
                        self.base.update();
                    }
                } else {
                    // Show popup or move to next item
                    if event.modifiers.alt {
                        self.show_popup();
                    } else if self.current_index < self.count() as i32 - 1 {
                        self.set_current_index(self.current_index + 1);
                        self.activated.emit(self.current_index);
                    }
                }
                return true;
            }
            Key::ArrowUp => {
                if self.popup_visible {
                    if self.highlighted_index > 0 {
                        self.highlighted_index -= 1;
                        self.ensure_highlighted_visible();
                        self.base.update();
                    }
                } else {
                    // Move to previous item
                    if self.current_index > 0 {
                        self.set_current_index(self.current_index - 1);
                        self.activated.emit(self.current_index);
                    }
                }
                return true;
            }
            Key::PageDown => {
                if self.popup_visible {
                    let new_idx = (self.highlighted_index + self.max_visible_items as i32)
                        .min(item_count as i32 - 1);
                    if new_idx != self.highlighted_index {
                        self.highlighted_index = new_idx;
                        self.ensure_highlighted_visible();
                        self.base.update();
                    }
                    return true;
                }
            }
            Key::PageUp => {
                if self.popup_visible {
                    let new_idx = (self.highlighted_index - self.max_visible_items as i32).max(0);
                    if new_idx != self.highlighted_index {
                        self.highlighted_index = new_idx;
                        self.ensure_highlighted_visible();
                        self.base.update();
                    }
                    return true;
                }
            }
            Key::Home => {
                if self.popup_visible && item_count > 0 {
                    self.highlighted_index = 0;
                    self.scroll_offset = 0;
                    self.base.update();
                    return true;
                }
            }
            Key::End => {
                if self.popup_visible && item_count > 0 {
                    self.highlighted_index = item_count as i32 - 1;
                    self.ensure_highlighted_visible();
                    self.base.update();
                    return true;
                }
            }
            Key::Space => {
                if !self.editable && !self.popup_visible {
                    self.show_popup();
                    return true;
                }
            }
            Key::Backspace => {
                if self.editable && !self.edit_text.is_empty() {
                    // Delete character before cursor
                    if self.cursor_pos > 0 {
                        // Find previous grapheme boundary
                        use unicode_segmentation::UnicodeSegmentation;
                        let graphemes: Vec<&str> = self.edit_text.graphemes(true).collect();
                        let mut byte_pos = 0;
                        let mut grapheme_idx = 0;

                        for (i, g) in graphemes.iter().enumerate() {
                            let next_byte_pos = byte_pos + g.len();
                            if next_byte_pos >= self.cursor_pos {
                                grapheme_idx = i;
                                break;
                            }
                            byte_pos = next_byte_pos;
                        }

                        if grapheme_idx > 0 {
                            let new_text: String = graphemes
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| *i != grapheme_idx - 1)
                                .map(|(_, g)| *g)
                                .collect();

                            let deleted_len = graphemes[grapheme_idx - 1].len();
                            self.cursor_pos -= deleted_len;
                            self.edit_text = new_text;
                        }

                        self.update_filtered_indices();
                        if self.popup_visible {
                            self.highlighted_index = 0;
                        }
                        self.base.update();
                        self.current_text_changed.emit(self.edit_text.clone());
                    }
                    return true;
                }
            }
            Key::Delete => {
                if self.editable && self.cursor_pos < self.edit_text.len() {
                    // Delete character after cursor
                    use unicode_segmentation::UnicodeSegmentation;
                    let graphemes: Vec<&str> = self.edit_text.graphemes(true).collect();
                    let mut byte_pos = 0;
                    let mut grapheme_idx = 0;

                    for (i, g) in graphemes.iter().enumerate() {
                        if byte_pos >= self.cursor_pos {
                            grapheme_idx = i;
                            break;
                        }
                        byte_pos += g.len();
                    }

                    if grapheme_idx < graphemes.len() {
                        let new_text: String = graphemes
                            .iter()
                            .enumerate()
                            .filter(|(i, _)| *i != grapheme_idx)
                            .map(|(_, g)| *g)
                            .collect();

                        self.edit_text = new_text;
                        self.update_filtered_indices();
                        if self.popup_visible {
                            self.highlighted_index = 0;
                        }
                        self.base.update();
                        self.current_text_changed.emit(self.edit_text.clone());
                    }
                    return true;
                }
            }
            _ => {}
        }

        // Handle character input for editable mode
        if self.editable
            && let Some(ch) = event.text.chars().next()
            && !ch.is_control()
        {
            // Insert character at cursor
            self.edit_text.insert(self.cursor_pos, ch);
            self.cursor_pos += ch.len_utf8();
            self.selection_start = None;

            self.update_filtered_indices();

            // Show popup if we have matches
            if !self.filtered_indices.is_empty() && !self.popup_visible {
                self.show_popup();
            } else if self.popup_visible {
                self.highlighted_index = 0;
                self.scroll_offset = 0;
            }

            self.base.update();
            self.current_text_changed.emit(self.edit_text.clone());
            return true;
        }

        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        if self.popup_visible {
            // Scroll the popup
            let delta = if event.delta_y > 0.0 { -1i32 } else { 1 };
            let item_count = self.effective_item_count();
            let max_scroll = item_count.saturating_sub(self.max_visible_items);

            let new_offset = (self.scroll_offset as i32 + delta)
                .max(0)
                .min(max_scroll as i32) as usize;

            if new_offset != self.scroll_offset {
                self.scroll_offset = new_offset;
                self.base.update();
            }
            return true;
        } else if !self.editable {
            // Change selection with wheel
            let delta = if event.delta_y > 0.0 { -1i32 } else { 1 };
            let new_index = (self.current_index + delta)
                .max(0)
                .min(self.count() as i32 - 1);

            if new_index != self.current_index && new_index >= 0 {
                self.set_current_index(new_index);
                self.activated.emit(new_index);
                return true;
            }
        }

        false
    }

    fn handle_focus_out(&mut self) -> bool {
        if self.popup_visible {
            self.hide_popup();
        }
        if self.editable && self.is_editing {
            self.is_editing = false;
            self.editing_finished.emit(());
        }
        self.base.update();
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_main(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let local_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());

        // Draw background
        let rounded = RoundedRect::new(local_rect, self.border_radius);
        ctx.renderer()
            .fill_rounded_rect(rounded, self.background_color);

        // Draw border
        let border_color = if self.base.has_focus() {
            self.focus_border_color
        } else {
            self.border_color
        };
        let stroke = Stroke::new(border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(rounded, &stroke);

        // Draw separator before arrow
        let arrow_x = rect.width() - self.arrow_width;
        let sep_stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(arrow_x, 2.0),
            Point::new(arrow_x, rect.height() - 2.0),
            &sep_stroke,
        );

        // Draw arrow button background on hover
        if matches!(self.hover_part, ComboBoxPart::Arrow) {
            let arrow_rect = Rect::new(
                arrow_x + 1.0,
                1.0,
                self.arrow_width - 2.0,
                rect.height() - 2.0,
            );
            ctx.renderer()
                .fill_rect(arrow_rect, Color::from_rgba8(200, 200, 200, 50));
        }

        // Draw dropdown arrow
        let arrow_color = if matches!(self.hover_part, ComboBoxPart::Arrow) {
            self.arrow_hover_color
        } else {
            self.arrow_color
        };
        self.paint_arrow(ctx, arrow_color);

        // Draw text or placeholder
        self.paint_text(ctx);
    }

    fn paint_arrow(&self, ctx: &mut PaintContext<'_>, color: Color) {
        let rect = self.base.rect();
        let arrow_x = rect.width() - self.arrow_width;
        let center_x = arrow_x + self.arrow_width / 2.0;
        let center_y = rect.height() / 2.0;

        // Draw a simple downward-pointing triangle
        let arrow_size = 5.0;
        let p1 = Point::new(center_x - arrow_size, center_y - arrow_size / 2.0);
        let p2 = Point::new(center_x + arrow_size, center_y - arrow_size / 2.0);
        let p3 = Point::new(center_x, center_y + arrow_size / 2.0);

        // Draw as three lines forming a triangle
        let stroke = Stroke::new(color, 2.0);
        ctx.renderer().draw_line(p1, p3, &stroke);
        ctx.renderer().draw_line(p2, p3, &stroke);
    }

    fn paint_text(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let _display_rect = self.display_rect();

        let mut font_system = FontSystem::new();

        // Determine what text to display
        let (text, color) = if self.editable {
            if self.edit_text.is_empty() && !self.placeholder.is_empty() {
                (&self.placeholder, self.placeholder_color)
            } else {
                (&self.edit_text, self.text_color)
            }
        } else if self.current_index >= 0 {
            if let Some(item_text) = self.item_text(self.current_index as usize) {
                // Return owned string, need to handle differently
                let layout = TextLayout::with_options(
                    &mut font_system,
                    &item_text,
                    &self.font,
                    TextLayoutOptions::new(),
                );

                let text_x = self.padding;

                // Draw icon if present
                let text_offset = if let Some(icon) = self.item_icon(self.current_index as usize) {
                    if let Some(image) = icon.image() {
                        let icon_size = 16.0;
                        let icon_y = (rect.height() - icon_size) / 2.0;
                        let icon_rect = Rect::new(text_x, icon_y, icon_size, icon_size);
                        ctx.renderer()
                            .draw_image(image, icon_rect, ImageScaleMode::Fit);
                        icon_size + self.padding
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                let text_y = (rect.height() - layout.height()) / 2.0;
                if let Ok(mut text_renderer) = TextRenderer::new() {
                    let _ = text_renderer.prepare_layout(
                        &mut font_system,
                        &layout,
                        Point::new(text_x + text_offset, text_y),
                        self.text_color,
                    );
                }
                return;
            } else {
                return;
            }
        } else if !self.placeholder.is_empty() {
            (&self.placeholder, self.placeholder_color)
        } else {
            return;
        };

        let layout =
            TextLayout::with_options(&mut font_system, text, &self.font, TextLayoutOptions::new());

        let text_x = self.padding;
        let text_y = (rect.height() - layout.height()) / 2.0;
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                color,
            );
        }

        // Draw cursor for editable mode when editing
        if self.editable && self.is_editing && self.base.has_focus() {
            // Simple cursor at end for now
            let cursor_x = text_x + layout.width();
            let cursor_stroke = Stroke::new(self.text_color, 1.0);
            ctx.renderer().draw_line(
                Point::new(cursor_x, text_y),
                Point::new(cursor_x, text_y + layout.height()),
                &cursor_stroke,
            );
        }
    }

    fn paint_popup(&self, ctx: &mut PaintContext<'_>) {
        if !self.popup_visible {
            return;
        }

        let popup_rect = self.popup_rect();

        // Draw popup background
        ctx.renderer()
            .fill_rect(popup_rect, self.popup_background_color);

        // Draw popup border
        let stroke = Stroke::new(self.popup_border_color, 1.0);
        ctx.renderer().stroke_rect(popup_rect, &stroke);

        // Draw items
        let item_count = self.effective_item_count();
        let visible_count = item_count.min(self.max_visible_items);

        for visual_idx in 0..visible_count {
            let list_idx = self.scroll_offset + visual_idx;
            if list_idx >= item_count {
                break;
            }

            let actual_idx = self.actual_index(list_idx);

            if let Some(model) = &self.model
                && let Some(item) = model.item(actual_idx)
            {
                let item_rect = Rect::new(
                    popup_rect.origin.x + 1.0,
                    popup_rect.origin.y + 1.0 + (visual_idx as f32) * self.item_height,
                    popup_rect.size.width - 2.0,
                    self.item_height,
                );

                let is_selected = list_idx as i32 == self.highlighted_index;
                let is_hovered =
                    matches!(self.hover_part, ComboBoxPart::PopupItem(idx) if idx == list_idx);

                self.delegate.paint_item(
                    ctx,
                    item_rect,
                    &item,
                    actual_idx,
                    is_selected,
                    is_hovered,
                );
            }
        }

        // Draw scroll indicators if needed
        if item_count > self.max_visible_items {
            self.paint_scroll_indicator(ctx, popup_rect, item_count);
        }
    }

    fn paint_scroll_indicator(
        &self,
        ctx: &mut PaintContext<'_>,
        popup_rect: Rect,
        item_count: usize,
    ) {
        let indicator_width = 4.0;
        let track_height = popup_rect.size.height - 2.0;
        let thumb_height = (self.max_visible_items as f32 / item_count as f32) * track_height;
        let max_scroll = item_count - self.max_visible_items;
        let thumb_y = if max_scroll > 0 {
            (self.scroll_offset as f32 / max_scroll as f32) * (track_height - thumb_height)
        } else {
            0.0
        };

        let track_rect = Rect::new(
            popup_rect.right() - indicator_width - 2.0,
            popup_rect.origin.y + 1.0,
            indicator_width,
            track_height,
        );

        let thumb_rect = Rect::new(
            track_rect.origin.x,
            track_rect.origin.y + thumb_y,
            indicator_width,
            thumb_height.max(10.0),
        );

        ctx.renderer()
            .fill_rect(track_rect, Color::from_rgb8(240, 240, 240));
        ctx.renderer()
            .fill_rect(thumb_rect, Color::from_rgb8(180, 180, 180));
    }
}

impl Widget for ComboBox {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let preferred = Size::new(120.0, 28.0);
        SizeHint::new(preferred).with_minimum(Size::new(60.0, 24.0))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_main(ctx);
        self.paint_popup(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::Wheel(e) => self.handle_wheel(e),
            WidgetEvent::FocusOut(_) => self.handle_focus_out(),
            WidgetEvent::Leave(_) => {
                self.hover_part = ComboBoxPart::None;
                self.base.update();
                false
            }
            _ => false,
        }
    }
}

impl Object for ComboBox {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Default for ComboBox {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        let _ = init_global_registry();
    }

    #[test]
    fn test_combo_box_creation() {
        setup();
        let combo = ComboBox::new();
        assert_eq!(combo.count(), 0);
        assert_eq!(combo.current_index(), -1);
        assert!(!combo.is_editable());
        assert!(!combo.is_popup_visible());
    }

    #[test]
    fn test_combo_box_with_model() {
        setup();
        let model = StringListComboModel::from(["Apple", "Banana", "Cherry"]);
        let combo = ComboBox::new().with_model(Box::new(model));

        assert_eq!(combo.count(), 3);
        assert_eq!(combo.item_text(0), Some("Apple".to_string()));
        assert_eq!(combo.item_text(1), Some("Banana".to_string()));
        assert_eq!(combo.item_text(2), Some("Cherry".to_string()));
    }

    #[test]
    fn test_combo_box_current_index() {
        setup();
        let model = StringListComboModel::from(["A", "B", "C"]);
        let mut combo = ComboBox::new().with_model(Box::new(model));

        assert_eq!(combo.current_index(), -1);

        combo.set_current_index(1);
        assert_eq!(combo.current_index(), 1);
        assert_eq!(combo.current_text(), "B");

        // Out of bounds should reset to -1
        combo.set_current_index(10);
        assert_eq!(combo.current_index(), -1);

        // Negative should be -1
        combo.set_current_index(-5);
        assert_eq!(combo.current_index(), -1);
    }

    #[test]
    fn test_combo_box_editable() {
        setup();
        let model = StringListComboModel::from(["Apple", "Banana", "Cherry"]);
        let mut combo = ComboBox::new()
            .with_model(Box::new(model))
            .with_editable(true);

        assert!(combo.is_editable());

        combo.set_current_text("Banana");
        assert_eq!(combo.current_index(), 1);
        assert_eq!(combo.current_text(), "Banana");

        // Custom text that doesn't match
        combo.set_current_text("Orange");
        assert_eq!(combo.current_index(), -1);
        assert_eq!(combo.current_text(), "Orange");
    }

    #[test]
    fn test_combo_box_find_text() {
        setup();
        let model = StringListComboModel::from(["Alpha", "Beta", "Gamma"]);
        let combo = ComboBox::new().with_model(Box::new(model));

        assert_eq!(combo.find_text("Beta"), Some(1));
        assert_eq!(combo.find_text("Delta"), None);
    }

    #[test]
    fn test_string_list_combo_model() {
        let mut model = StringListComboModel::new(vec!["A".to_string(), "B".to_string()]);

        assert_eq!(model.row_count(), 2);
        assert_eq!(model.text(0), Some("A".to_string()));
        assert_eq!(model.text(1), Some("B".to_string()));
        assert_eq!(model.text(2), None);

        model.add_item("C");
        assert_eq!(model.row_count(), 3);

        model.remove_item(1);
        assert_eq!(model.row_count(), 2);
        assert_eq!(model.text(1), Some("C".to_string()));

        model.clear();
        assert_eq!(model.row_count(), 0);
    }

    #[test]
    fn test_combo_box_model_filter() {
        let model = StringListComboModel::from(["Apple", "Application", "Banana", "Cherry"]);

        // Case insensitive filter
        let filtered = model.filter("app", true);
        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&0)); // Apple
        assert!(filtered.contains(&1)); // Application

        // Case sensitive filter
        let filtered = model.filter("App", false);
        assert_eq!(filtered.len(), 2);

        let filtered = model.filter("app", false);
        assert_eq!(filtered.len(), 0);

        // No match
        let filtered = model.filter("xyz", true);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_combo_box_builder_pattern() {
        setup();
        let model = StringListComboModel::from(["A", "B", "C"]);
        let combo = ComboBox::new()
            .with_model(Box::new(model))
            .with_current_index(1)
            .with_editable(true)
            .with_placeholder("Select...")
            .with_max_visible_items(5)
            .with_case_insensitive(false);

        assert_eq!(combo.current_index(), 1);
        assert!(combo.is_editable());
        assert_eq!(combo.placeholder(), "Select...");
        assert_eq!(combo.max_visible_items(), 5);
        assert!(!combo.is_case_insensitive());
    }

    #[test]
    fn test_combo_box_popup_control() {
        setup();
        let model = StringListComboModel::from(["A", "B", "C"]);
        let mut combo = ComboBox::new().with_model(Box::new(model));

        assert!(!combo.is_popup_visible());

        combo.show_popup();
        assert!(combo.is_popup_visible());

        combo.hide_popup();
        assert!(!combo.is_popup_visible());

        combo.toggle_popup();
        assert!(combo.is_popup_visible());

        combo.toggle_popup();
        assert!(!combo.is_popup_visible());
    }

    #[test]
    fn test_combo_box_empty_model_popup() {
        setup();
        let mut combo = ComboBox::new();

        // Can't show popup with no items
        combo.show_popup();
        assert!(!combo.is_popup_visible());
    }

    #[test]
    fn test_icon_list_combo_model() {
        let mut model = IconListComboModel::empty();

        model.add_text("Item 1");
        model.add_text("Item 2");

        assert_eq!(model.row_count(), 2);
        assert_eq!(model.text(0), Some("Item 1".to_string()));
        assert!(model.icon(0).is_none());

        model.clear();
        assert_eq!(model.row_count(), 0);
    }

    #[test]
    fn test_combo_box_item() {
        let item = ComboBoxItem::new("Test");
        assert_eq!(item.text, "Test");
        assert!(item.icon.is_none());

        // Test clone
        let cloned = item.clone();
        assert_eq!(cloned.text, "Test");
        assert!(cloned.icon.is_none());
    }
}
