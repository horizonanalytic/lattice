//! Input dialog implementation.
//!
//! This module provides [`InputDialog`], a modal dialog for getting simple input
//! from the user with support for text, numbers, and item selection.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::InputDialog;
//!
//! // Get text input
//! let mut dialog = InputDialog::get_text("Enter Name", "Please enter your name:", "");
//! dialog.text_value_selected.connect(|text| {
//!     println!("Name: {}", text);
//! });
//! dialog.open();
//!
//! // Get integer input
//! let mut dialog = InputDialog::get_int("Enter Age", "Please enter your age:", 25, 0, 150, 1);
//! dialog.int_value_selected.connect(|&value| {
//!     println!("Age: {}", value);
//! });
//! dialog.open();
//!
//! // Get item selection
//! let items = vec!["Apple", "Banana", "Cherry"];
//! let mut dialog = InputDialog::get_item("Select Fruit", "Choose a fruit:", items, 0, false);
//! dialog.text_value_selected.connect(|text| {
//!     println!("Selected: {}", text);
//! });
//! dialog.open();
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, RoundedRect, Stroke, TextLayout,
    TextLayoutOptions, TextRenderer,
};

use crate::widget::{
    Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent, MouseReleaseEvent,
    PaintContext, SizeHint, WheelEvent, Widget, WidgetBase, WidgetEvent,
};

use super::dialog::{Dialog, DialogResult};
use super::dialog_button_box::StandardButton;

// ============================================================================
// Constants
// ============================================================================

/// Default dialog width.
const DEFAULT_WIDTH: f32 = 400.0;

/// Default dialog height for single-line inputs.
const DEFAULT_HEIGHT: f32 = 150.0;

/// Default dialog height for multiline inputs.
const MULTILINE_HEIGHT: f32 = 280.0;

/// Input field height for single-line modes.
const INPUT_HEIGHT: f32 = 32.0;

/// Button width for spinbox up/down.
const SPINBOX_BUTTON_WIDTH: f32 = 24.0;

/// List item height for combo dropdown.
const LIST_ITEM_HEIGHT: f32 = 28.0;

/// Maximum visible items in combo dropdown.
const MAX_DROPDOWN_ITEMS: usize = 8;

// ============================================================================
// InputMode
// ============================================================================

/// The input mode of an InputDialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// Single-line text input.
    #[default]
    Text,
    /// Multi-line text input.
    MultilineText,
    /// Integer input with spinbox.
    Int,
    /// Floating-point input with spinbox.
    Double,
    /// Item selection from a list.
    Item,
}

// ============================================================================
// InputDialogOptions
// ============================================================================

/// Echo mode for text input (for password fields).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputEchoMode {
    /// Display characters as entered.
    #[default]
    Normal,
    /// Do not display any characters.
    NoEcho,
    /// Display asterisks for each character.
    Password,
}

// ============================================================================
// HitPart
// ============================================================================

/// Identifies which part of the dialog is being interacted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum HitPart {
    #[default]
    None,
    /// The text input field.
    Input,
    /// The spinbox up button.
    SpinUp,
    /// The spinbox down button.
    SpinDown,
    /// The combo dropdown button.
    DropdownButton,
    /// An item in the dropdown list.
    DropdownItem(usize),
}

// ============================================================================
// InputDialog
// ============================================================================

/// A modal dialog for getting simple input from the user.
///
/// InputDialog provides convenient static methods for common input scenarios:
///
/// - [`InputDialog::get_text()`]: Single-line text input
/// - [`InputDialog::get_multiline_text()`]: Multi-line text input
/// - [`InputDialog::get_int()`]: Integer input with spinbox
/// - [`InputDialog::get_double()`]: Floating-point input with spinbox
/// - [`InputDialog::get_item()`]: Item selection from a list
///
/// # Signals
///
/// - `text_value_selected(String)`: Emitted when dialog is accepted with text/item value
/// - `int_value_selected(i32)`: Emitted when dialog is accepted with integer value
/// - `double_value_selected(f64)`: Emitted when dialog is accepted with double value
pub struct InputDialog {
    /// The underlying dialog.
    dialog: Dialog,

    /// Current input mode.
    mode: InputMode,

    /// Label text (prompt).
    label_text: String,

    // Text input state
    /// Current text value.
    text_value: String,
    /// Cursor position in text.
    text_cursor: usize,
    /// Text selection start, if any.
    text_selection: Option<usize>,
    /// Echo mode for text input.
    echo_mode: InputEchoMode,

    // Multiline text state
    /// Lines of text for multiline mode.
    lines: Vec<String>,
    /// Current line index.
    current_line: usize,
    /// Scroll offset for multiline.
    scroll_offset: usize,

    // Integer input state
    /// Current integer value.
    int_value: i32,
    /// Minimum integer value.
    int_min: i32,
    /// Maximum integer value.
    int_max: i32,
    /// Integer step size.
    int_step: i32,

    // Double input state
    /// Current double value.
    double_value: f64,
    /// Minimum double value.
    double_min: f64,
    /// Maximum double value.
    double_max: f64,
    /// Double step size.
    double_step: f64,
    /// Number of decimal places.
    decimals: u32,

    // Item selection state
    /// Items for selection.
    items: Vec<String>,
    /// Currently selected item index.
    selected_item: i32,
    /// Whether the dropdown is open.
    dropdown_open: bool,
    /// Scroll offset in dropdown.
    dropdown_scroll: usize,
    /// Whether the combo is editable.
    editable: bool,

    // Interaction state
    /// Current hover part.
    hover_part: HitPart,
    /// Currently pressed part.
    pressed_part: HitPart,
    /// Whether the input field is focused.
    input_focused: bool,

    // Visual styling
    /// Padding between elements.
    padding: f32,
    /// Border radius.
    border_radius: f32,
    /// Border color.
    border_color: Color,
    /// Focus border color.
    focus_color: Color,
    /// Selection background color.
    selection_color: Color,
    /// Hover background color.
    hover_color: Color,
    /// Input background color.
    input_background: Color,
    /// Button background color.
    button_color: Color,
    /// Button hover color.
    button_hover_color: Color,

    // Signals
    /// Signal emitted when dialog is accepted with a text value.
    pub text_value_selected: Signal<String>,

    /// Signal emitted when dialog is accepted with an integer value.
    pub int_value_selected: Signal<i32>,

    /// Signal emitted when dialog is accepted with a double value.
    pub double_value_selected: Signal<f64>,
}

impl InputDialog {
    /// Create a new input dialog with default settings.
    pub fn new() -> Self {
        let dialog = Dialog::new("Input")
            .with_size(DEFAULT_WIDTH, DEFAULT_HEIGHT)
            .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL);

        Self {
            dialog,
            mode: InputMode::Text,
            label_text: String::new(),
            text_value: String::new(),
            text_cursor: 0,
            text_selection: None,
            echo_mode: InputEchoMode::Normal,
            lines: vec![String::new()],
            current_line: 0,
            scroll_offset: 0,
            int_value: 0,
            int_min: i32::MIN,
            int_max: i32::MAX,
            int_step: 1,
            double_value: 0.0,
            double_min: f64::MIN,
            double_max: f64::MAX,
            double_step: 1.0,
            decimals: 2,
            items: Vec::new(),
            selected_item: -1,
            dropdown_open: false,
            dropdown_scroll: 0,
            editable: false,
            hover_part: HitPart::None,
            pressed_part: HitPart::None,
            input_focused: true,
            padding: 12.0,
            border_radius: 4.0,
            border_color: Color::from_rgb8(180, 180, 180),
            focus_color: Color::from_rgb8(51, 153, 255),
            selection_color: Color::from_rgba8(51, 153, 255, 255),
            hover_color: Color::from_rgba8(200, 200, 200, 100),
            input_background: Color::WHITE,
            button_color: Color::from_rgb8(240, 240, 240),
            button_hover_color: Color::from_rgb8(220, 220, 220),
            text_value_selected: Signal::new(),
            int_value_selected: Signal::new(),
            double_value_selected: Signal::new(),
        }
    }

    // =========================================================================
    // Static Helper Methods
    // =========================================================================

    /// Create a dialog to get a single line of text from the user.
    ///
    /// # Arguments
    ///
    /// * `title` - The dialog title
    /// * `label` - The prompt text
    /// * `text` - The default text value
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut dialog = InputDialog::get_text("Enter Name", "Your name:", "John");
    /// dialog.text_value_selected.connect(|name| {
    ///     println!("Name: {}", name);
    /// });
    /// dialog.open();
    /// ```
    pub fn get_text(
        title: impl Into<String>,
        label: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        Self::new()
            .with_title(title)
            .with_label(label)
            .with_text_value(text)
            .with_mode(InputMode::Text)
    }

    /// Create a dialog to get multi-line text from the user.
    ///
    /// # Arguments
    ///
    /// * `title` - The dialog title
    /// * `label` - The prompt text
    /// * `text` - The default text value
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut dialog = InputDialog::get_multiline_text("Enter Description", "Description:", "");
    /// dialog.text_value_selected.connect(|desc| {
    ///     println!("Description: {}", desc);
    /// });
    /// dialog.open();
    /// ```
    pub fn get_multiline_text(
        title: impl Into<String>,
        label: impl Into<String>,
        text: impl Into<String>,
    ) -> Self {
        let mut dialog = Self::new()
            .with_title(title)
            .with_label(label)
            .with_mode(InputMode::MultilineText);

        let text_str = text.into();
        dialog.lines = if text_str.is_empty() {
            vec![String::new()]
        } else {
            text_str.lines().map(String::from).collect()
        };
        dialog.dialog = std::mem::take(&mut dialog.dialog).with_size(DEFAULT_WIDTH, MULTILINE_HEIGHT);
        dialog
    }

    /// Create a dialog to get an integer value from the user.
    ///
    /// # Arguments
    ///
    /// * `title` - The dialog title
    /// * `label` - The prompt text
    /// * `value` - The default value
    /// * `min` - The minimum allowed value
    /// * `max` - The maximum allowed value
    /// * `step` - The step size for increment/decrement
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut dialog = InputDialog::get_int("Enter Age", "Your age:", 25, 0, 150, 1);
    /// dialog.int_value_selected.connect(|&age| {
    ///     println!("Age: {}", age);
    /// });
    /// dialog.open();
    /// ```
    pub fn get_int(
        title: impl Into<String>,
        label: impl Into<String>,
        value: i32,
        min: i32,
        max: i32,
        step: i32,
    ) -> Self {
        Self::new()
            .with_title(title)
            .with_label(label)
            .with_mode(InputMode::Int)
            .with_int_value(value)
            .with_int_range(min, max)
            .with_int_step(step)
    }

    /// Create a dialog to get a floating-point value from the user.
    ///
    /// # Arguments
    ///
    /// * `title` - The dialog title
    /// * `label` - The prompt text
    /// * `value` - The default value
    /// * `min` - The minimum allowed value
    /// * `max` - The maximum allowed value
    /// * `decimals` - Number of decimal places to display
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut dialog = InputDialog::get_double("Enter Temperature", "Temperature:", 20.0, -50.0, 50.0, 1);
    /// dialog.double_value_selected.connect(|&temp| {
    ///     println!("Temperature: {}", temp);
    /// });
    /// dialog.open();
    /// ```
    pub fn get_double(
        title: impl Into<String>,
        label: impl Into<String>,
        value: f64,
        min: f64,
        max: f64,
        decimals: u32,
    ) -> Self {
        Self::new()
            .with_title(title)
            .with_label(label)
            .with_mode(InputMode::Double)
            .with_double_value(value)
            .with_double_range(min, max)
            .with_decimals(decimals)
    }

    /// Create a dialog to select an item from a list.
    ///
    /// # Arguments
    ///
    /// * `title` - The dialog title
    /// * `label` - The prompt text
    /// * `items` - The list of items to choose from
    /// * `current` - The index of the initially selected item
    /// * `editable` - Whether the user can enter custom text
    ///
    /// # Example
    ///
    /// ```ignore
    /// let items = vec!["Apple", "Banana", "Cherry"];
    /// let mut dialog = InputDialog::get_item("Select Fruit", "Choose:", items, 0, false);
    /// dialog.text_value_selected.connect(|fruit| {
    ///     println!("Selected: {}", fruit);
    /// });
    /// dialog.open();
    /// ```
    pub fn get_item<S: Into<String>>(
        title: impl Into<String>,
        label: impl Into<String>,
        items: Vec<S>,
        current: i32,
        editable: bool,
    ) -> Self {
        let string_items: Vec<String> = items.into_iter().map(|s| s.into()).collect();
        let text_value = if current >= 0 && (current as usize) < string_items.len() {
            string_items[current as usize].clone()
        } else {
            String::new()
        };

        Self::new()
            .with_title(title)
            .with_label(label)
            .with_mode(InputMode::Item)
            .with_items(string_items)
            .with_selected_item(current)
            .with_editable(editable)
            .with_text_value(text_value)
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dialog.set_title(title);
        self
    }

    /// Set the label (prompt) text using builder pattern.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label_text = label.into();
        self
    }

    /// Set the input mode using builder pattern.
    pub fn with_mode(mut self, mode: InputMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the text value using builder pattern.
    pub fn with_text_value(mut self, text: impl Into<String>) -> Self {
        self.text_value = text.into();
        self.text_cursor = self.text_value.len();
        self
    }

    /// Set the echo mode using builder pattern.
    pub fn with_echo_mode(mut self, mode: InputEchoMode) -> Self {
        self.echo_mode = mode;
        self
    }

    /// Set the integer value using builder pattern.
    pub fn with_int_value(mut self, value: i32) -> Self {
        self.int_value = value.clamp(self.int_min, self.int_max);
        self
    }

    /// Set the integer range using builder pattern.
    pub fn with_int_range(mut self, min: i32, max: i32) -> Self {
        self.int_min = min;
        self.int_max = max;
        self.int_value = self.int_value.clamp(min, max);
        self
    }

    /// Set the integer step using builder pattern.
    pub fn with_int_step(mut self, step: i32) -> Self {
        self.int_step = step.max(1);
        self
    }

    /// Set the double value using builder pattern.
    pub fn with_double_value(mut self, value: f64) -> Self {
        self.double_value = value.clamp(self.double_min, self.double_max);
        self
    }

    /// Set the double range using builder pattern.
    pub fn with_double_range(mut self, min: f64, max: f64) -> Self {
        self.double_min = min;
        self.double_max = max;
        self.double_value = self.double_value.clamp(min, max);
        self
    }

    /// Set the double step using builder pattern.
    pub fn with_double_step(mut self, step: f64) -> Self {
        self.double_step = step;
        self
    }

    /// Set the decimal places using builder pattern.
    pub fn with_decimals(mut self, decimals: u32) -> Self {
        self.decimals = decimals;
        self
    }

    /// Set the items using builder pattern.
    pub fn with_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }

    /// Set the selected item index using builder pattern.
    pub fn with_selected_item(mut self, index: i32) -> Self {
        self.selected_item = index;
        self
    }

    /// Set whether the combo is editable using builder pattern.
    pub fn with_editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Set the dialog size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.dialog = std::mem::take(&mut self.dialog).with_size(width, height);
        self
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /// Get the current input mode.
    pub fn mode(&self) -> InputMode {
        self.mode
    }

    /// Set the input mode.
    pub fn set_mode(&mut self, mode: InputMode) {
        self.mode = mode;
        self.dialog.widget_base_mut().update();
    }

    /// Get the label text.
    pub fn label_text(&self) -> &str {
        &self.label_text
    }

    /// Set the label text.
    pub fn set_label_text(&mut self, text: impl Into<String>) {
        self.label_text = text.into();
        self.dialog.widget_base_mut().update();
    }

    /// Get the text value.
    pub fn text_value(&self) -> &str {
        &self.text_value
    }

    /// Set the text value.
    pub fn set_text_value(&mut self, text: impl Into<String>) {
        self.text_value = text.into();
        self.text_cursor = self.text_value.len();
        self.dialog.widget_base_mut().update();
    }

    /// Get the integer value.
    pub fn int_value(&self) -> i32 {
        self.int_value
    }

    /// Set the integer value.
    pub fn set_int_value(&mut self, value: i32) {
        self.int_value = value.clamp(self.int_min, self.int_max);
        self.dialog.widget_base_mut().update();
    }

    /// Get the double value.
    pub fn double_value(&self) -> f64 {
        self.double_value
    }

    /// Set the double value.
    pub fn set_double_value(&mut self, value: f64) {
        self.double_value = value.clamp(self.double_min, self.double_max);
        self.dialog.widget_base_mut().update();
    }

    /// Get the title.
    pub fn title(&self) -> &str {
        self.dialog.title()
    }

    /// Set the title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.dialog.set_title(title);
    }

    /// Get the dialog result.
    pub fn result(&self) -> DialogResult {
        self.dialog.result()
    }

    /// Check if the dialog is currently open.
    pub fn is_open(&self) -> bool {
        self.dialog.is_open()
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the input dialog (non-blocking modal).
    pub fn open(&mut self) {
        self.input_focused = true;
        self.dropdown_open = false;
        self.dialog.open();
    }

    /// Accept the dialog and emit the appropriate signal.
    pub fn accept(&mut self) {
        match self.mode {
            InputMode::Text => {
                self.text_value_selected.emit(self.text_value.clone());
            }
            InputMode::MultilineText => {
                let text = self.lines.join("\n");
                self.text_value_selected.emit(text);
            }
            InputMode::Int => {
                self.int_value_selected.emit(self.int_value);
            }
            InputMode::Double => {
                self.double_value_selected.emit(self.double_value);
            }
            InputMode::Item => {
                let text = if self.editable {
                    self.text_value.clone()
                } else if self.selected_item >= 0 && (self.selected_item as usize) < self.items.len()
                {
                    self.items[self.selected_item as usize].clone()
                } else {
                    String::new()
                };
                self.text_value_selected.emit(text);
            }
        }
        self.dialog.accept();
    }

    /// Reject the dialog.
    pub fn reject(&mut self) {
        self.dropdown_open = false;
        self.dialog.reject();
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.dialog.close();
    }

    // =========================================================================
    // Signal Access
    // =========================================================================

    /// Get a reference to the accepted signal.
    pub fn accepted(&self) -> &Signal<()> {
        &self.dialog.accepted
    }

    /// Get a reference to the rejected signal.
    pub fn rejected(&self) -> &Signal<()> {
        &self.dialog.rejected
    }

    /// Get a reference to the finished signal.
    pub fn finished(&self) -> &Signal<DialogResult> {
        &self.dialog.finished
    }

    // =========================================================================
    // Layout Calculations
    // =========================================================================

    fn content_rect(&self) -> Rect {
        self.dialog.content_rect()
    }

    fn label_rect(&self) -> Rect {
        let content = self.content_rect();
        Rect::new(content.left(), content.top(), content.width(), 20.0)
    }

    fn input_rect(&self) -> Rect {
        let label = self.label_rect();
        let content = self.content_rect();

        let height = match self.mode {
            InputMode::MultilineText => {
                content.bottom() - label.bottom() - self.padding
            }
            _ => INPUT_HEIGHT,
        };

        Rect::new(
            content.left(),
            label.bottom() + self.padding,
            content.width(),
            height,
        )
    }

    fn spinbox_up_rect(&self) -> Rect {
        let input = self.input_rect();
        Rect::new(
            input.right() - SPINBOX_BUTTON_WIDTH,
            input.top(),
            SPINBOX_BUTTON_WIDTH,
            input.height() / 2.0,
        )
    }

    fn spinbox_down_rect(&self) -> Rect {
        let input = self.input_rect();
        Rect::new(
            input.right() - SPINBOX_BUTTON_WIDTH,
            input.top() + input.height() / 2.0,
            SPINBOX_BUTTON_WIDTH,
            input.height() / 2.0,
        )
    }

    fn dropdown_button_rect(&self) -> Rect {
        let input = self.input_rect();
        Rect::new(
            input.right() - INPUT_HEIGHT,
            input.top(),
            INPUT_HEIGHT,
            input.height(),
        )
    }

    fn dropdown_list_rect(&self) -> Rect {
        let input = self.input_rect();
        let item_count = self.items.len().min(MAX_DROPDOWN_ITEMS);
        let height = item_count as f32 * LIST_ITEM_HEIGHT + 2.0;
        Rect::new(input.left(), input.bottom() + 2.0, input.width(), height)
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    fn hit_test(&self, pos: Point) -> HitPart {
        let input = self.input_rect();

        // Check mode-specific parts first
        match self.mode {
            InputMode::Int | InputMode::Double => {
                if self.spinbox_up_rect().contains(pos) {
                    return HitPart::SpinUp;
                }
                if self.spinbox_down_rect().contains(pos) {
                    return HitPart::SpinDown;
                }
            }
            InputMode::Item => {
                if self.dropdown_open {
                    let list_rect = self.dropdown_list_rect();
                    if list_rect.contains(pos) {
                        let local_y = pos.y - list_rect.top() - 1.0;
                        let idx = (local_y / LIST_ITEM_HEIGHT) as usize + self.dropdown_scroll;
                        if idx < self.items.len() {
                            return HitPart::DropdownItem(idx);
                        }
                    }
                }
                if self.dropdown_button_rect().contains(pos) {
                    return HitPart::DropdownButton;
                }
            }
            _ => {}
        }

        if input.contains(pos) {
            return HitPart::Input;
        }

        HitPart::None
    }

    // =========================================================================
    // Input Handling
    // =========================================================================

    fn increment_int(&mut self) {
        let new_value = self.int_value.saturating_add(self.int_step);
        self.int_value = new_value.clamp(self.int_min, self.int_max);
        self.dialog.widget_base_mut().update();
    }

    fn decrement_int(&mut self) {
        let new_value = self.int_value.saturating_sub(self.int_step);
        self.int_value = new_value.clamp(self.int_min, self.int_max);
        self.dialog.widget_base_mut().update();
    }

    fn increment_double(&mut self) {
        let new_value = self.double_value + self.double_step;
        self.double_value = new_value.clamp(self.double_min, self.double_max);
        self.dialog.widget_base_mut().update();
    }

    fn decrement_double(&mut self) {
        let new_value = self.double_value - self.double_step;
        self.double_value = new_value.clamp(self.double_min, self.double_max);
        self.dialog.widget_base_mut().update();
    }

    fn select_dropdown_item(&mut self, idx: usize) {
        if idx < self.items.len() {
            self.selected_item = idx as i32;
            self.text_value = self.items[idx].clone();
            self.text_cursor = self.text_value.len();
            self.dropdown_open = false;
            self.dialog.widget_base_mut().update();
        }
    }

    fn toggle_dropdown(&mut self) {
        self.dropdown_open = !self.dropdown_open;
        if self.dropdown_open {
            // Ensure selected item is visible
            if self.selected_item >= 0 {
                let idx = self.selected_item as usize;
                if idx >= self.dropdown_scroll + MAX_DROPDOWN_ITEMS {
                    self.dropdown_scroll = idx.saturating_sub(MAX_DROPDOWN_ITEMS - 1);
                } else if idx < self.dropdown_scroll {
                    self.dropdown_scroll = idx;
                }
            }
        }
        self.dialog.widget_base_mut().update();
    }

    // =========================================================================
    // Text Editing Helpers
    // =========================================================================

    fn insert_char(&mut self, c: char) {
        self.text_value.insert(self.text_cursor, c);
        self.text_cursor += c.len_utf8();
        self.dialog.widget_base_mut().update();
    }

    fn delete_char_backward(&mut self) {
        if self.text_cursor > 0 {
            let prev_char_len = self.text_value[..self.text_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.text_cursor -= prev_char_len;
            self.text_value.remove(self.text_cursor);
            self.dialog.widget_base_mut().update();
        }
    }

    fn delete_char_forward(&mut self) {
        if self.text_cursor < self.text_value.len() {
            self.text_value.remove(self.text_cursor);
            self.dialog.widget_base_mut().update();
        }
    }

    fn move_cursor_left(&mut self) {
        if self.text_cursor > 0 {
            let prev_char_len = self.text_value[..self.text_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.text_cursor -= prev_char_len;
            self.dialog.widget_base_mut().update();
        }
    }

    fn move_cursor_right(&mut self) {
        if self.text_cursor < self.text_value.len() {
            let next_char_len = self.text_value[self.text_cursor..]
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.text_cursor += next_char_len;
            self.dialog.widget_base_mut().update();
        }
    }

    // =========================================================================
    // Multiline Text Helpers
    // =========================================================================

    fn multiline_insert_char(&mut self, c: char) {
        if c == '\n' {
            // Split line at cursor
            let current = &self.lines[self.current_line];
            let rest = current[self.text_cursor..].to_string();
            self.lines[self.current_line].truncate(self.text_cursor);
            self.current_line += 1;
            self.lines.insert(self.current_line, rest);
            self.text_cursor = 0;
        } else {
            self.lines[self.current_line].insert(self.text_cursor, c);
            self.text_cursor += c.len_utf8();
        }
        self.dialog.widget_base_mut().update();
    }

    fn multiline_delete_backward(&mut self) {
        if self.text_cursor > 0 {
            let prev_char_len = self.lines[self.current_line][..self.text_cursor]
                .chars()
                .last()
                .map(|c| c.len_utf8())
                .unwrap_or(0);
            self.text_cursor -= prev_char_len;
            self.lines[self.current_line].remove(self.text_cursor);
        } else if self.current_line > 0 {
            // Merge with previous line
            let current_text = self.lines.remove(self.current_line);
            self.current_line -= 1;
            self.text_cursor = self.lines[self.current_line].len();
            self.lines[self.current_line].push_str(&current_text);
        }
        self.dialog.widget_base_mut().update();
    }

    fn multiline_move_up(&mut self) {
        if self.current_line > 0 {
            self.current_line -= 1;
            self.text_cursor = self.text_cursor.min(self.lines[self.current_line].len());
            self.ensure_line_visible();
            self.dialog.widget_base_mut().update();
        }
    }

    fn multiline_move_down(&mut self) {
        if self.current_line < self.lines.len() - 1 {
            self.current_line += 1;
            self.text_cursor = self.text_cursor.min(self.lines[self.current_line].len());
            self.ensure_line_visible();
            self.dialog.widget_base_mut().update();
        }
    }

    fn ensure_line_visible(&mut self) {
        let visible_lines = self.visible_line_count();
        if self.current_line < self.scroll_offset {
            self.scroll_offset = self.current_line;
        } else if self.current_line >= self.scroll_offset + visible_lines {
            self.scroll_offset = self.current_line.saturating_sub(visible_lines) + 1;
        }
    }

    fn visible_line_count(&self) -> usize {
        let input = self.input_rect();
        let line_height = 20.0;
        (input.height() / line_height) as usize
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;
        let part = self.hit_test(pos);

        self.pressed_part = part;

        match part {
            HitPart::Input => {
                self.input_focused = true;
                if self.mode == InputMode::Item && !self.editable {
                    self.toggle_dropdown();
                } else if self.dropdown_open {
                    self.dropdown_open = false;
                }
                self.dialog.widget_base_mut().update();
                true
            }
            HitPart::SpinUp => {
                match self.mode {
                    InputMode::Int => self.increment_int(),
                    InputMode::Double => self.increment_double(),
                    _ => {}
                }
                true
            }
            HitPart::SpinDown => {
                match self.mode {
                    InputMode::Int => self.decrement_int(),
                    InputMode::Double => self.decrement_double(),
                    _ => {}
                }
                true
            }
            HitPart::DropdownButton => {
                self.toggle_dropdown();
                true
            }
            HitPart::DropdownItem(idx) => {
                self.select_dropdown_item(idx);
                true
            }
            HitPart::None => {
                if self.dropdown_open {
                    self.dropdown_open = false;
                    self.dialog.widget_base_mut().update();
                }
                false
            }
        }
    }

    fn handle_mouse_release(&mut self, _event: &MouseReleaseEvent) -> bool {
        if self.pressed_part != HitPart::None {
            self.pressed_part = HitPart::None;
            self.dialog.widget_base_mut().update();
            return true;
        }
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let part = self.hit_test(event.local_pos);
        if part != self.hover_part {
            self.hover_part = part;
            self.dialog.widget_base_mut().update();
            return true;
        }
        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Handle Enter for accept
        if event.key == Key::Enter && !event.is_repeat {
            if self.mode != InputMode::MultilineText || event.modifiers.control {
                if self.dropdown_open {
                    if self.selected_item >= 0 {
                        self.select_dropdown_item(self.selected_item as usize);
                    } else {
                        self.dropdown_open = false;
                    }
                } else {
                    self.accept();
                }
                return true;
            }
        }

        // Handle Escape
        if event.key == Key::Escape {
            if self.dropdown_open {
                self.dropdown_open = false;
                self.dialog.widget_base_mut().update();
                return true;
            }
            // Let dialog handle escape for rejection
            return false;
        }

        // Handle mode-specific keys
        match self.mode {
            InputMode::Text => self.handle_text_key(event),
            InputMode::MultilineText => self.handle_multiline_key(event),
            InputMode::Int => self.handle_int_key(event),
            InputMode::Double => self.handle_double_key(event),
            InputMode::Item => self.handle_item_key(event),
        }
    }

    fn handle_text_key(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Backspace => {
                self.delete_char_backward();
                true
            }
            Key::Delete => {
                self.delete_char_forward();
                true
            }
            Key::ArrowLeft => {
                self.move_cursor_left();
                true
            }
            Key::ArrowRight => {
                self.move_cursor_right();
                true
            }
            Key::Home => {
                self.text_cursor = 0;
                self.dialog.widget_base_mut().update();
                true
            }
            Key::End => {
                self.text_cursor = self.text_value.len();
                self.dialog.widget_base_mut().update();
                true
            }
            _ => {
                if !event.text.is_empty() && !event.modifiers.control && !event.modifiers.alt {
                    for c in event.text.chars() {
                        if !c.is_control() {
                            self.insert_char(c);
                        }
                    }
                    return true;
                }
                false
            }
        }
    }

    fn handle_multiline_key(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Backspace => {
                self.multiline_delete_backward();
                true
            }
            Key::Delete => {
                if self.text_cursor < self.lines[self.current_line].len() {
                    self.lines[self.current_line].remove(self.text_cursor);
                    self.dialog.widget_base_mut().update();
                } else if self.current_line < self.lines.len() - 1 {
                    let next_line = self.lines.remove(self.current_line + 1);
                    self.lines[self.current_line].push_str(&next_line);
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            Key::ArrowLeft => {
                if self.text_cursor > 0 {
                    let prev_char_len = self.lines[self.current_line][..self.text_cursor]
                        .chars()
                        .last()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    self.text_cursor -= prev_char_len;
                } else if self.current_line > 0 {
                    self.current_line -= 1;
                    self.text_cursor = self.lines[self.current_line].len();
                }
                self.dialog.widget_base_mut().update();
                true
            }
            Key::ArrowRight => {
                if self.text_cursor < self.lines[self.current_line].len() {
                    let next_char_len = self.lines[self.current_line][self.text_cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                    self.text_cursor += next_char_len;
                } else if self.current_line < self.lines.len() - 1 {
                    self.current_line += 1;
                    self.text_cursor = 0;
                }
                self.dialog.widget_base_mut().update();
                true
            }
            Key::ArrowUp => {
                self.multiline_move_up();
                true
            }
            Key::ArrowDown => {
                self.multiline_move_down();
                true
            }
            Key::Home => {
                self.text_cursor = 0;
                self.dialog.widget_base_mut().update();
                true
            }
            Key::End => {
                self.text_cursor = self.lines[self.current_line].len();
                self.dialog.widget_base_mut().update();
                true
            }
            Key::Enter if !event.modifiers.control => {
                self.multiline_insert_char('\n');
                true
            }
            _ => {
                if !event.text.is_empty() && !event.modifiers.control && !event.modifiers.alt {
                    for c in event.text.chars() {
                        if !c.is_control() {
                            self.multiline_insert_char(c);
                        }
                    }
                    return true;
                }
                false
            }
        }
    }

    fn handle_int_key(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::ArrowUp => {
                self.increment_int();
                true
            }
            Key::ArrowDown => {
                self.decrement_int();
                true
            }
            _ => false,
        }
    }

    fn handle_double_key(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::ArrowUp => {
                self.increment_double();
                true
            }
            Key::ArrowDown => {
                self.decrement_double();
                true
            }
            _ => false,
        }
    }

    fn handle_item_key(&mut self, event: &KeyPressEvent) -> bool {
        if self.editable {
            // Handle text editing for editable combo
            match event.key {
                Key::ArrowUp if self.dropdown_open => {
                    if self.selected_item > 0 {
                        self.selected_item -= 1;
                        self.dialog.widget_base_mut().update();
                    }
                    true
                }
                Key::ArrowDown if self.dropdown_open => {
                    if (self.selected_item as usize) < self.items.len() - 1 {
                        self.selected_item += 1;
                        self.dialog.widget_base_mut().update();
                    }
                    true
                }
                Key::ArrowDown if !self.dropdown_open => {
                    self.toggle_dropdown();
                    true
                }
                _ => self.handle_text_key(event),
            }
        } else {
            // Non-editable combo - just arrow navigation
            match event.key {
                Key::ArrowUp => {
                    if self.dropdown_open {
                        if self.selected_item > 0 {
                            self.selected_item -= 1;
                            self.ensure_dropdown_item_visible(self.selected_item as usize);
                            self.dialog.widget_base_mut().update();
                        }
                    } else if self.selected_item > 0 {
                        self.selected_item -= 1;
                        self.text_value = self.items[self.selected_item as usize].clone();
                        self.dialog.widget_base_mut().update();
                    }
                    true
                }
                Key::ArrowDown => {
                    if self.dropdown_open {
                        if (self.selected_item as usize) < self.items.len() - 1 {
                            self.selected_item += 1;
                            self.ensure_dropdown_item_visible(self.selected_item as usize);
                            self.dialog.widget_base_mut().update();
                        }
                    } else if (self.selected_item as usize) < self.items.len() - 1 {
                        self.selected_item += 1;
                        self.text_value = self.items[self.selected_item as usize].clone();
                        self.dialog.widget_base_mut().update();
                    }
                    true
                }
                Key::Space => {
                    self.toggle_dropdown();
                    true
                }
                _ => false,
            }
        }
    }

    fn ensure_dropdown_item_visible(&mut self, idx: usize) {
        if idx >= self.dropdown_scroll + MAX_DROPDOWN_ITEMS {
            self.dropdown_scroll = idx.saturating_sub(MAX_DROPDOWN_ITEMS - 1);
        } else if idx < self.dropdown_scroll {
            self.dropdown_scroll = idx;
        }
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let delta = if event.delta_y > 0.0 { -1i32 } else { 1 };

        match self.mode {
            InputMode::Int => {
                if delta > 0 {
                    self.increment_int();
                } else {
                    self.decrement_int();
                }
                true
            }
            InputMode::Double => {
                if delta > 0 {
                    self.increment_double();
                } else {
                    self.decrement_double();
                }
                true
            }
            InputMode::MultilineText => {
                let new_scroll = (self.scroll_offset as i32 + delta)
                    .max(0)
                    .min(self.lines.len().saturating_sub(self.visible_line_count()) as i32)
                    as usize;
                if new_scroll != self.scroll_offset {
                    self.scroll_offset = new_scroll;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            InputMode::Item if self.dropdown_open => {
                let new_scroll = (self.dropdown_scroll as i32 + delta)
                    .max(0)
                    .min(self.items.len().saturating_sub(MAX_DROPDOWN_ITEMS) as i32)
                    as usize;
                if new_scroll != self.dropdown_scroll {
                    self.dropdown_scroll = new_scroll;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            _ => false,
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_label(&self, _ctx: &mut PaintContext<'_>) {
        if self.label_text.is_empty() {
            return;
        }

        let rect = self.label_rect();
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 13.0);
        let layout =
            TextLayout::with_options(&mut font_system, &self.label_text, &font, TextLayoutOptions::new());

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(rect.left(), rect.top()),
                Color::from_rgb8(40, 40, 40),
            );
        }
    }

    fn paint_input_background(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        let rounded = RoundedRect::new(rect, self.border_radius);
        ctx.renderer().fill_rounded_rect(rounded, self.input_background);

        let border_color = if self.input_focused {
            self.focus_color
        } else {
            self.border_color
        };
        let stroke = Stroke::new(border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(rounded, &stroke);
    }

    fn paint_text_input(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.input_rect();
        self.paint_input_background(ctx, rect);

        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 14.0);

        let display_text = match self.echo_mode {
            InputEchoMode::Normal => self.text_value.clone(),
            InputEchoMode::NoEcho => String::new(),
            InputEchoMode::Password => "●".repeat(self.text_value.chars().count()),
        };

        let layout =
            TextLayout::with_options(&mut font_system, &display_text, &font, TextLayoutOptions::new());

        let text_x = rect.left() + 8.0;
        let text_y = rect.top() + (rect.height() - layout.height()) / 2.0;

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                Color::BLACK,
            );
        }

        // Draw cursor if focused
        if self.input_focused {
            // Approximate cursor position
            let cursor_text = &display_text[..self.text_cursor.min(display_text.len())];
            let cursor_layout =
                TextLayout::with_options(&mut font_system, cursor_text, &font, TextLayoutOptions::new());
            let cursor_x = text_x + cursor_layout.width();

            ctx.renderer().fill_rect(
                Rect::new(cursor_x, text_y, 1.0, layout.height()),
                Color::BLACK,
            );
        }
    }

    fn paint_multiline_input(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.input_rect();
        self.paint_input_background(ctx, rect);

        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::Monospace, 13.0);
        let line_height = 20.0;
        let visible_count = self.visible_line_count();

        let text_x = rect.left() + 8.0;
        let mut text_y = rect.top() + 4.0;

        for i in 0..visible_count {
            let line_idx = self.scroll_offset + i;
            if line_idx >= self.lines.len() {
                break;
            }

            let line = &self.lines[line_idx];
            let layout =
                TextLayout::with_options(&mut font_system, line, &font, TextLayoutOptions::new());

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    Color::BLACK,
                );
            }

            // Draw cursor if this is the current line
            if self.input_focused && line_idx == self.current_line {
                let cursor_text = &line[..self.text_cursor.min(line.len())];
                let cursor_layout =
                    TextLayout::with_options(&mut font_system, cursor_text, &font, TextLayoutOptions::new());
                let cursor_x = text_x + cursor_layout.width();

                ctx.renderer().fill_rect(
                    Rect::new(cursor_x, text_y, 1.0, line_height - 4.0),
                    Color::BLACK,
                );
            }

            text_y += line_height;
        }
    }

    fn paint_int_input(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.input_rect();
        self.paint_input_background(ctx, rect);

        // Draw value
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 14.0);
        let value_text = self.int_value.to_string();
        let layout =
            TextLayout::with_options(&mut font_system, &value_text, &font, TextLayoutOptions::new());

        let text_x = rect.left() + 8.0;
        let text_y = rect.top() + (rect.height() - layout.height()) / 2.0;

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                Color::BLACK,
            );
        }

        // Draw spinbox buttons
        self.paint_spinbox_button(ctx, self.spinbox_up_rect(), true);
        self.paint_spinbox_button(ctx, self.spinbox_down_rect(), false);
    }

    fn paint_double_input(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.input_rect();
        self.paint_input_background(ctx, rect);

        // Draw value
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 14.0);
        let value_text = format!("{:.prec$}", self.double_value, prec = self.decimals as usize);
        let layout =
            TextLayout::with_options(&mut font_system, &value_text, &font, TextLayoutOptions::new());

        let text_x = rect.left() + 8.0;
        let text_y = rect.top() + (rect.height() - layout.height()) / 2.0;

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                Color::BLACK,
            );
        }

        // Draw spinbox buttons
        self.paint_spinbox_button(ctx, self.spinbox_up_rect(), true);
        self.paint_spinbox_button(ctx, self.spinbox_down_rect(), false);
    }

    fn paint_spinbox_button(&self, ctx: &mut PaintContext<'_>, rect: Rect, is_up: bool) {
        let part = if is_up { HitPart::SpinUp } else { HitPart::SpinDown };
        let is_hovered = self.hover_part == part;
        let is_pressed = self.pressed_part == part;

        let bg = if is_pressed {
            Color::from_rgb8(200, 200, 200)
        } else if is_hovered {
            self.button_hover_color
        } else {
            self.button_color
        };

        ctx.renderer().fill_rect(rect, bg);

        // Draw border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rect(rect, &stroke);

        // Draw arrow
        let arrow_size = 6.0;
        let cx = rect.left() + rect.width() / 2.0;
        let cy = rect.top() + rect.height() / 2.0;

        let arrow_stroke = Stroke::new(Color::from_rgb8(60, 60, 60), 1.5);

        if is_up {
            ctx.renderer().draw_line(
                Point::new(cx - arrow_size / 2.0, cy + arrow_size / 4.0),
                Point::new(cx, cy - arrow_size / 4.0),
                &arrow_stroke,
            );
            ctx.renderer().draw_line(
                Point::new(cx, cy - arrow_size / 4.0),
                Point::new(cx + arrow_size / 2.0, cy + arrow_size / 4.0),
                &arrow_stroke,
            );
        } else {
            ctx.renderer().draw_line(
                Point::new(cx - arrow_size / 2.0, cy - arrow_size / 4.0),
                Point::new(cx, cy + arrow_size / 4.0),
                &arrow_stroke,
            );
            ctx.renderer().draw_line(
                Point::new(cx, cy + arrow_size / 4.0),
                Point::new(cx + arrow_size / 2.0, cy - arrow_size / 4.0),
                &arrow_stroke,
            );
        }
    }

    fn paint_item_input(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.input_rect();
        self.paint_input_background(ctx, rect);

        // Draw current text
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 14.0);

        let display_text = if self.editable {
            &self.text_value
        } else if self.selected_item >= 0 && (self.selected_item as usize) < self.items.len() {
            &self.items[self.selected_item as usize]
        } else {
            &self.text_value
        };

        let layout =
            TextLayout::with_options(&mut font_system, display_text, &font, TextLayoutOptions::new());

        let text_x = rect.left() + 8.0;
        let text_y = rect.top() + (rect.height() - layout.height()) / 2.0;

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                Color::BLACK,
            );
        }

        // Draw cursor if editable and focused
        if self.editable && self.input_focused && !self.dropdown_open {
            let cursor_text = &display_text[..self.text_cursor.min(display_text.len())];
            let cursor_layout =
                TextLayout::with_options(&mut font_system, cursor_text, &font, TextLayoutOptions::new());
            let cursor_x = text_x + cursor_layout.width();

            ctx.renderer().fill_rect(
                Rect::new(cursor_x, text_y, 1.0, layout.height()),
                Color::BLACK,
            );
        }

        // Draw dropdown button
        self.paint_dropdown_button(ctx);

        // Draw dropdown list if open
        if self.dropdown_open {
            self.paint_dropdown_list(ctx);
        }
    }

    fn paint_dropdown_button(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.dropdown_button_rect();
        let is_hovered = self.hover_part == HitPart::DropdownButton;
        let is_pressed = self.pressed_part == HitPart::DropdownButton;

        let bg = if is_pressed {
            Color::from_rgb8(200, 200, 200)
        } else if is_hovered {
            self.button_hover_color
        } else {
            self.button_color
        };

        ctx.renderer().fill_rect(rect, bg);

        // Draw border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rect(rect, &stroke);

        // Draw arrow
        let arrow_size = 8.0;
        let cx = rect.left() + rect.width() / 2.0;
        let cy = rect.top() + rect.height() / 2.0;

        let arrow_stroke = Stroke::new(Color::from_rgb8(60, 60, 60), 1.5);

        ctx.renderer().draw_line(
            Point::new(cx - arrow_size / 2.0, cy - 2.0),
            Point::new(cx, cy + 3.0),
            &arrow_stroke,
        );
        ctx.renderer().draw_line(
            Point::new(cx, cy + 3.0),
            Point::new(cx + arrow_size / 2.0, cy - 2.0),
            &arrow_stroke,
        );
    }

    fn paint_dropdown_list(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.dropdown_list_rect();

        // Background with shadow
        let shadow_rect = Rect::new(rect.left() + 2.0, rect.top() + 2.0, rect.width(), rect.height());
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(shadow_rect, 4.0), Color::from_rgba8(0, 0, 0, 30));

        let rounded = RoundedRect::new(rect, 4.0);
        ctx.renderer().fill_rounded_rect(rounded, Color::WHITE);

        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(rounded, &stroke);

        // Draw items
        let mut font_system = FontSystem::new();
        let font = Font::new(FontFamily::SansSerif, 13.0);
        let visible_count = MAX_DROPDOWN_ITEMS.min(self.items.len() - self.dropdown_scroll);

        for i in 0..visible_count {
            let item_idx = self.dropdown_scroll + i;
            if item_idx >= self.items.len() {
                break;
            }

            let item_rect = Rect::new(
                rect.left() + 1.0,
                rect.top() + 1.0 + (i as f32) * LIST_ITEM_HEIGHT,
                rect.width() - 2.0,
                LIST_ITEM_HEIGHT,
            );

            let is_selected = item_idx as i32 == self.selected_item;
            let is_hovered = matches!(self.hover_part, HitPart::DropdownItem(h) if h == item_idx);

            if is_selected {
                ctx.renderer().fill_rect(item_rect, self.selection_color);
            } else if is_hovered {
                ctx.renderer().fill_rect(item_rect, self.hover_color);
            }

            let text_color = if is_selected {
                Color::WHITE
            } else {
                Color::BLACK
            };

            let layout = TextLayout::with_options(
                &mut font_system,
                &self.items[item_idx],
                &font,
                TextLayoutOptions::new(),
            );

            let text_x = item_rect.left() + 8.0;
            let text_y = item_rect.top() + (item_rect.height() - layout.height()) / 2.0;

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
}

impl Default for InputDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for InputDialog {
    fn object_id(&self) -> ObjectId {
        self.dialog.object_id()
    }
}

impl Widget for InputDialog {
    fn widget_base(&self) -> &WidgetBase {
        self.dialog.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.dialog.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        self.dialog.size_hint()
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint dialog base
        self.dialog.paint(ctx);

        if !self.dialog.is_open() {
            return;
        }

        // Paint label
        self.paint_label(ctx);

        // Paint input based on mode
        match self.mode {
            InputMode::Text => self.paint_text_input(ctx),
            InputMode::MultilineText => self.paint_multiline_input(ctx),
            InputMode::Int => self.paint_int_input(ctx),
            InputMode::Double => self.paint_double_input(ctx),
            InputMode::Item => self.paint_item_input(ctx),
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle our own events first
        let handled = match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::Wheel(e) => self.handle_wheel(e),
            _ => false,
        };

        if handled {
            event.accept();
            return true;
        }

        // Delegate to dialog
        self.dialog.event(event)
    }
}

// Thread safety assertion
static_assertions::assert_impl_all!(InputDialog: Send, Sync);

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::Arc;

    fn setup() {
        let _ = init_global_registry();
    }

    #[test]
    fn test_input_dialog_creation() {
        setup();
        let dialog = InputDialog::new();
        assert!(!dialog.is_open());
        assert_eq!(dialog.mode(), InputMode::Text);
    }

    #[test]
    fn test_get_text() {
        setup();
        let dialog = InputDialog::get_text("Title", "Enter name:", "Default");
        assert_eq!(dialog.title(), "Title");
        assert_eq!(dialog.label_text(), "Enter name:");
        assert_eq!(dialog.text_value(), "Default");
        assert_eq!(dialog.mode(), InputMode::Text);
    }

    #[test]
    fn test_get_multiline_text() {
        setup();
        let dialog = InputDialog::get_multiline_text("Title", "Enter text:", "Line1\nLine2");
        assert_eq!(dialog.mode(), InputMode::MultilineText);
        assert_eq!(dialog.lines.len(), 2);
    }

    #[test]
    fn test_get_int() {
        setup();
        let dialog = InputDialog::get_int("Age", "Enter age:", 25, 0, 150, 1);
        assert_eq!(dialog.title(), "Age");
        assert_eq!(dialog.int_value(), 25);
        assert_eq!(dialog.mode(), InputMode::Int);
    }

    #[test]
    fn test_get_double() {
        setup();
        let dialog = InputDialog::get_double("Temp", "Temperature:", 20.0, -50.0, 50.0, 1);
        assert_eq!(dialog.mode(), InputMode::Double);
        assert!((dialog.double_value() - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_get_item() {
        setup();
        let items = vec!["Apple", "Banana", "Cherry"];
        let dialog = InputDialog::get_item("Select", "Choose:", items, 1, false);
        assert_eq!(dialog.mode(), InputMode::Item);
        assert_eq!(dialog.selected_item, 1);
        assert_eq!(dialog.text_value(), "Banana");
    }

    #[test]
    fn test_builder_pattern() {
        setup();
        let dialog = InputDialog::new()
            .with_title("Test")
            .with_label("Label:")
            .with_mode(InputMode::Int)
            .with_int_value(42)
            .with_int_range(0, 100)
            .with_int_step(5);

        assert_eq!(dialog.title(), "Test");
        assert_eq!(dialog.label_text(), "Label:");
        assert_eq!(dialog.int_value(), 42);
    }

    #[test]
    fn test_int_value_clamping() {
        setup();
        let mut dialog = InputDialog::get_int("", "", 50, 0, 100, 1);

        dialog.set_int_value(150);
        assert_eq!(dialog.int_value(), 100);

        dialog.set_int_value(-50);
        assert_eq!(dialog.int_value(), 0);
    }

    #[test]
    fn test_double_value_clamping() {
        setup();
        let mut dialog = InputDialog::get_double("", "", 50.0, 0.0, 100.0, 1);

        dialog.set_double_value(150.0);
        assert!((dialog.double_value() - 100.0).abs() < 0.01);

        dialog.set_double_value(-50.0);
        assert!((dialog.double_value() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_dialog_lifecycle() {
        setup();
        let mut dialog = InputDialog::new();
        assert!(!dialog.is_open());

        dialog.open();
        assert!(dialog.is_open());

        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    fn test_text_value_signal() {
        setup();
        let mut dialog = InputDialog::get_text("", "", "Test Value");

        let selected = Arc::new(std::sync::Mutex::new(String::new()));
        let selected_clone = selected.clone();

        dialog.text_value_selected.connect(move |text| {
            *selected_clone.lock().unwrap() = text.clone();
        });

        dialog.open();
        dialog.accept();

        let result = selected.lock().unwrap();
        assert_eq!(*result, "Test Value");
    }

    #[test]
    fn test_int_value_signal() {
        setup();
        let mut dialog = InputDialog::get_int("", "", 42, 0, 100, 1);

        let selected = Arc::new(std::sync::Mutex::new(0));
        let selected_clone = selected.clone();

        dialog.int_value_selected.connect(move |&value| {
            *selected_clone.lock().unwrap() = value;
        });

        dialog.open();
        dialog.accept();

        let result = *selected.lock().unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_echo_mode() {
        setup();
        let dialog = InputDialog::new()
            .with_mode(InputMode::Text)
            .with_echo_mode(InputEchoMode::Password)
            .with_text_value("secret");

        assert_eq!(dialog.echo_mode, InputEchoMode::Password);
    }
}
