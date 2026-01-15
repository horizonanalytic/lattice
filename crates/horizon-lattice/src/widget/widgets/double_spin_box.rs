//! DoubleSpinBox widget for floating-point input.
//!
//! The DoubleSpinBox widget provides a way to enter and modify floating-point values with:
//! - Increment/decrement buttons
//! - Direct text editing
//! - Range constraints (minimum, maximum)
//! - Step size configuration
//! - Configurable decimal precision
//! - Optional prefix and suffix text
//! - Optional wrapping mode
//! - Keyboard and mouse wheel support
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::DoubleSpinBox;
//!
//! // Create a simple double spinbox
//! let mut spinbox = DoubleSpinBox::new()
//!     .with_range(0.0, 100.0)
//!     .with_value(50.0)
//!     .with_decimals(2)
//!     .with_single_step(0.5);
//!
//! // Create a spinbox with prefix and suffix
//! let mut temperature = DoubleSpinBox::new()
//!     .with_range(-40.0, 100.0)
//!     .with_suffix(" °C")
//!     .with_decimals(1);
//!
//! // Connect to value changes
//! spinbox.value_changed.connect(|&value| {
//!     println!("Value: {}", value);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, HorizontalAlign, Point, Rect, Renderer, RoundedRect,
    Stroke, TextLayout, TextLayoutOptions, TextRenderer, VerticalAlign,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, WheelEvent, Widget,
    WidgetBase, WidgetEvent,
};

/// A widget for entering and modifying floating-point values.
///
/// DoubleSpinBox provides a text field showing the current value with small up/down
/// buttons to increment or decrement the value by a configurable step.
///
/// # Signals
///
/// - `value_changed(f64)`: Emitted when the value changes
/// - `editing_finished()`: Emitted when editing is completed (focus lost or Enter pressed)
pub struct DoubleSpinBox {
    /// Widget base.
    base: WidgetBase,

    /// Current value.
    value: f64,

    /// Minimum value.
    minimum: f64,

    /// Maximum value.
    maximum: f64,

    /// Step size for increment/decrement.
    single_step: f64,

    /// Number of decimal places to display.
    decimals: u32,

    /// Prefix text displayed before the value.
    prefix: String,

    /// Suffix text displayed after the value.
    suffix: String,

    /// Whether values wrap around at boundaries.
    wrapping: bool,

    /// Whether the spinbox is read-only.
    read_only: bool,

    /// Special value text (shown when value equals minimum, if set).
    special_value_text: Option<String>,

    /// Background color.
    background_color: Color,

    /// Text color.
    text_color: Color,

    /// Border color.
    border_color: Color,

    /// Button color.
    button_color: Color,

    /// Button hover color.
    button_hover_color: Color,

    /// Button pressed color.
    button_pressed_color: Color,

    /// Font for text rendering.
    font: Font,

    /// Border radius.
    border_radius: f32,

    /// Button width.
    button_width: f32,

    /// Which part is currently hovered.
    hover_part: DoubleSpinBoxPart,

    /// Which button is currently pressed.
    pressed_part: DoubleSpinBoxPart,

    /// Whether the text field is being edited.
    editing: bool,

    /// Text buffer for editing.
    edit_text: String,

    /// Cursor position in the edit text (byte offset).
    cursor_pos: usize,

    /// Selection start (byte offset), if any.
    selection_start: Option<usize>,

    /// Signal emitted when value changes.
    pub value_changed: Signal<f64>,

    /// Signal emitted when editing is finished.
    pub editing_finished: Signal<()>,
}

/// Parts of the spinbox for hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DoubleSpinBoxPart {
    #[default]
    None,
    /// The up (increment) button.
    UpButton,
    /// The down (decrement) button.
    DownButton,
    /// The text field area.
    TextField,
}

impl DoubleSpinBox {
    /// Create a new double spinbox with default settings.
    ///
    /// Default configuration:
    /// - Range: 0.0 to 99.99
    /// - Value: 0.0
    /// - Step: 1.0
    /// - Decimals: 2
    /// - No prefix or suffix
    /// - No wrapping
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Fixed));

        Self {
            base,
            value: 0.0,
            minimum: 0.0,
            maximum: 99.99,
            single_step: 1.0,
            decimals: 2,
            prefix: String::new(),
            suffix: String::new(),
            wrapping: false,
            read_only: false,
            special_value_text: None,
            background_color: Color::WHITE,
            text_color: Color::BLACK,
            border_color: Color::from_rgb8(180, 180, 180),
            button_color: Color::from_rgb8(240, 240, 240),
            button_hover_color: Color::from_rgb8(220, 220, 220),
            button_pressed_color: Color::from_rgb8(200, 200, 200),
            font: Font::new(FontFamily::SansSerif, 13.0),
            border_radius: 4.0,
            button_width: 20.0,
            hover_part: DoubleSpinBoxPart::None,
            pressed_part: DoubleSpinBoxPart::None,
            editing: false,
            edit_text: String::new(),
            cursor_pos: 0,
            selection_start: None,
            value_changed: Signal::new(),
            editing_finished: Signal::new(),
        }
    }

    // =========================================================================
    // Value and Range
    // =========================================================================

    /// Get the current value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Set the current value.
    ///
    /// The value is clamped to the valid range unless wrapping is enabled.
    pub fn set_value(&mut self, value: f64) {
        let new_value = if self.wrapping {
            self.wrap_value(value)
        } else {
            value.clamp(self.minimum, self.maximum)
        };

        // Use epsilon comparison for floats
        if (self.value - new_value).abs() > f64::EPSILON {
            self.value = new_value;
            self.base.update();
            self.value_changed.emit(new_value);
        }
    }

    /// Set value using builder pattern.
    pub fn with_value(mut self, value: f64) -> Self {
        self.set_value(value);
        self
    }

    /// Get the minimum value.
    pub fn minimum(&self) -> f64 {
        self.minimum
    }

    /// Set the minimum value.
    pub fn set_minimum(&mut self, minimum: f64) {
        self.set_range(minimum, self.maximum);
    }

    /// Set minimum using builder pattern.
    pub fn with_minimum(mut self, minimum: f64) -> Self {
        self.set_minimum(minimum);
        self
    }

    /// Get the maximum value.
    pub fn maximum(&self) -> f64 {
        self.maximum
    }

    /// Set the maximum value.
    pub fn set_maximum(&mut self, maximum: f64) {
        self.set_range(self.minimum, maximum);
    }

    /// Set maximum using builder pattern.
    pub fn with_maximum(mut self, maximum: f64) -> Self {
        self.set_maximum(maximum);
        self
    }

    /// Set the value range.
    pub fn set_range(&mut self, minimum: f64, maximum: f64) {
        let (min, max) = if minimum <= maximum {
            (minimum, maximum)
        } else {
            (maximum, minimum)
        };

        if (self.minimum - min).abs() > f64::EPSILON || (self.maximum - max).abs() > f64::EPSILON {
            self.minimum = min;
            self.maximum = max;
            // Clamp current value to new range
            let new_value = self.value.clamp(min, max);
            if (self.value - new_value).abs() > f64::EPSILON {
                self.value = new_value;
                self.value_changed.emit(new_value);
            }
            self.base.update();
        }
    }

    /// Set range using builder pattern.
    pub fn with_range(mut self, minimum: f64, maximum: f64) -> Self {
        self.set_range(minimum, maximum);
        self
    }

    // =========================================================================
    // Decimals
    // =========================================================================

    /// Get the number of decimal places.
    pub fn decimals(&self) -> u32 {
        self.decimals
    }

    /// Set the number of decimal places to display.
    pub fn set_decimals(&mut self, decimals: u32) {
        if self.decimals != decimals {
            self.decimals = decimals;
            self.base.update();
        }
    }

    /// Set decimals using builder pattern.
    pub fn with_decimals(mut self, decimals: u32) -> Self {
        self.decimals = decimals;
        self
    }

    // =========================================================================
    // Step Size
    // =========================================================================

    /// Get the step size.
    pub fn single_step(&self) -> f64 {
        self.single_step
    }

    /// Set the step size.
    pub fn set_single_step(&mut self, step: f64) {
        self.single_step = step.abs().max(f64::EPSILON);
    }

    /// Set step using builder pattern.
    pub fn with_single_step(mut self, step: f64) -> Self {
        self.set_single_step(step);
        self
    }

    // =========================================================================
    // Prefix and Suffix
    // =========================================================================

    /// Get the prefix text.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Set the prefix text displayed before the value.
    pub fn set_prefix(&mut self, prefix: impl Into<String>) {
        let new_prefix = prefix.into();
        if self.prefix != new_prefix {
            self.prefix = new_prefix;
            self.base.update();
        }
    }

    /// Set prefix using builder pattern.
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Get the suffix text.
    pub fn suffix(&self) -> &str {
        &self.suffix
    }

    /// Set the suffix text displayed after the value.
    pub fn set_suffix(&mut self, suffix: impl Into<String>) {
        let new_suffix = suffix.into();
        if self.suffix != new_suffix {
            self.suffix = new_suffix;
            self.base.update();
        }
    }

    /// Set suffix using builder pattern.
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    // =========================================================================
    // Special Value Text
    // =========================================================================

    /// Get the special value text.
    pub fn special_value_text(&self) -> Option<&str> {
        self.special_value_text.as_deref()
    }

    /// Set special text to display when value equals minimum.
    ///
    /// This is useful for displaying "Auto" or "Default" instead of a number.
    pub fn set_special_value_text(&mut self, text: Option<impl Into<String>>) {
        self.special_value_text = text.map(|t| t.into());
        self.base.update();
    }

    /// Set special value text using builder pattern.
    pub fn with_special_value_text(mut self, text: impl Into<String>) -> Self {
        self.special_value_text = Some(text.into());
        self
    }

    // =========================================================================
    // Wrapping and Read-Only
    // =========================================================================

    /// Check if wrapping is enabled.
    pub fn wrapping(&self) -> bool {
        self.wrapping
    }

    /// Set whether values wrap around at boundaries.
    ///
    /// When enabled, incrementing past maximum wraps to minimum and vice versa.
    pub fn set_wrapping(&mut self, wrapping: bool) {
        self.wrapping = wrapping;
    }

    /// Set wrapping using builder pattern.
    pub fn with_wrapping(mut self, wrapping: bool) -> Self {
        self.wrapping = wrapping;
        self
    }

    /// Check if the spinbox is read-only.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Set whether the spinbox is read-only.
    pub fn set_read_only(&mut self, read_only: bool) {
        if self.read_only != read_only {
            self.read_only = read_only;
            if read_only {
                self.finish_editing();
            }
            self.base.update();
        }
    }

    /// Set read-only using builder pattern.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the background color.
    pub fn background_color(&self) -> Color {
        self.background_color
    }

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Color) {
        if self.background_color != color {
            self.background_color = color;
            self.base.update();
        }
    }

    /// Set background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Get the text color.
    pub fn text_color(&self) -> Color {
        self.text_color
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        if self.text_color != color {
            self.text_color = color;
            self.base.update();
        }
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Get the border radius.
    pub fn border_radius(&self) -> f32 {
        self.border_radius
    }

    /// Set the border radius.
    pub fn set_border_radius(&mut self, radius: f32) {
        if (self.border_radius - radius).abs() > f32::EPSILON {
            self.border_radius = radius;
            self.base.update();
        }
    }

    /// Set border radius using builder pattern.
    pub fn with_border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    /// Get the font.
    pub fn font(&self) -> &Font {
        &self.font
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

    // =========================================================================
    // Actions
    // =========================================================================

    /// Increment the value by single_step.
    pub fn step_up(&mut self) {
        self.set_value(self.value + self.single_step);
    }

    /// Decrement the value by single_step.
    pub fn step_down(&mut self) {
        self.set_value(self.value - self.single_step);
    }

    /// Select all text (for editing mode).
    pub fn select_all(&mut self) {
        if !self.editing {
            self.start_editing();
        }
        self.selection_start = Some(0);
        self.cursor_pos = self.edit_text.len();
        self.base.update();
    }

    // =========================================================================
    // Helper Methods
    // =========================================================================

    /// Wrap value around range boundaries.
    fn wrap_value(&self, value: f64) -> f64 {
        let range = self.maximum - self.minimum;
        if range <= f64::EPSILON {
            return self.minimum;
        }

        if value < self.minimum {
            let excess = self.minimum - value;
            let wraps = (excess / range).ceil();
            self.minimum + (wraps * range - excess) % range
        } else if value > self.maximum {
            let excess = value - self.maximum;
            let wraps = (excess / range).ceil();
            self.maximum - (wraps * range - excess) % range
        } else {
            value
        }
    }

    /// Get the display text for the current value.
    fn display_text(&self) -> String {
        if (self.value - self.minimum).abs() < f64::EPSILON {
            if let Some(ref special) = self.special_value_text {
                return special.clone();
            }
        }
        format!(
            "{}{:.prec$}{}",
            self.prefix,
            self.value,
            self.suffix,
            prec = self.decimals as usize
        )
    }

    /// Get the text field rectangle.
    fn text_field_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            0.0,
            0.0,
            (rect.width() - self.button_width).max(0.0),
            rect.height(),
        )
    }

    /// Get the up button rectangle.
    fn up_button_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            rect.width() - self.button_width,
            0.0,
            self.button_width,
            rect.height() / 2.0,
        )
    }

    /// Get the down button rectangle.
    fn down_button_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            rect.width() - self.button_width,
            rect.height() / 2.0,
            self.button_width,
            rect.height() / 2.0,
        )
    }

    /// Hit test to determine which part is at a point.
    fn hit_test(&self, pos: Point) -> DoubleSpinBoxPart {
        if self.up_button_rect().contains(pos) {
            DoubleSpinBoxPart::UpButton
        } else if self.down_button_rect().contains(pos) {
            DoubleSpinBoxPart::DownButton
        } else if self.text_field_rect().contains(pos) {
            DoubleSpinBoxPart::TextField
        } else {
            DoubleSpinBoxPart::None
        }
    }

    /// Start editing the text field.
    fn start_editing(&mut self) {
        if self.read_only {
            return;
        }
        self.editing = true;
        // Initialize edit text with the formatted number (no prefix/suffix)
        self.edit_text = format!("{:.prec$}", self.value, prec = self.decimals as usize);
        self.cursor_pos = self.edit_text.len();
        self.selection_start = Some(0); // Select all initially
        self.base.update();
    }

    /// Finish editing and apply the value.
    fn finish_editing(&mut self) {
        if !self.editing {
            return;
        }
        self.editing = false;

        // Try to parse the edit text as a value
        if let Ok(parsed) = self.edit_text.trim().parse::<f64>() {
            if parsed.is_finite() {
                self.set_value(parsed);
            }
        }

        self.edit_text.clear();
        self.cursor_pos = 0;
        self.selection_start = None;
        self.base.update();
        self.editing_finished.emit(());
    }

    /// Cancel editing without applying changes.
    fn cancel_editing(&mut self) {
        if !self.editing {
            return;
        }
        self.editing = false;
        self.edit_text.clear();
        self.cursor_pos = 0;
        self.selection_start = None;
        self.base.update();
    }

    /// Delete selected text, if any.
    fn delete_selection(&mut self) -> bool {
        if let Some(sel_start) = self.selection_start {
            let (start, end) = if sel_start < self.cursor_pos {
                (sel_start, self.cursor_pos)
            } else {
                (self.cursor_pos, sel_start)
            };
            if start != end {
                self.edit_text.replace_range(start..end, "");
                self.cursor_pos = start;
                self.selection_start = None;
                return true;
            }
        }
        false
    }

    /// Insert text at the cursor position.
    fn insert_text(&mut self, text: &str) {
        self.delete_selection();
        self.edit_text.insert_str(self.cursor_pos, text);
        self.cursor_pos += text.len();
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let part = self.hit_test(event.local_pos);
        match part {
            DoubleSpinBoxPart::UpButton => {
                if !self.read_only {
                    self.pressed_part = DoubleSpinBoxPart::UpButton;
                    self.step_up();
                    self.base.update();
                    return true;
                }
            }
            DoubleSpinBoxPart::DownButton => {
                if !self.read_only {
                    self.pressed_part = DoubleSpinBoxPart::DownButton;
                    self.step_down();
                    self.base.update();
                    return true;
                }
            }
            DoubleSpinBoxPart::TextField => {
                if !self.editing {
                    self.start_editing();
                }
                return true;
            }
            DoubleSpinBoxPart::None => {}
        }
        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.pressed_part != DoubleSpinBoxPart::None {
            self.pressed_part = DoubleSpinBoxPart::None;
            self.base.update();
            return true;
        }
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let new_hover = self.hit_test(event.local_pos);
        if self.hover_part != new_hover {
            self.hover_part = new_hover;
            self.base.update();
        }
        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        if self.read_only {
            return false;
        }

        if event.delta_y.abs() > 0.0 {
            let steps = (event.delta_y / 120.0).round() as i32;
            if steps > 0 {
                for _ in 0..steps {
                    self.step_up();
                }
            } else {
                for _ in 0..(-steps) {
                    self.step_down();
                }
            }
            return true;
        }
        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        if self.editing {
            self.handle_edit_key(event)
        } else {
            self.handle_nav_key(event)
        }
    }

    fn handle_nav_key(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::ArrowUp => {
                if !self.read_only {
                    self.step_up();
                    return true;
                }
            }
            Key::ArrowDown => {
                if !self.read_only {
                    self.step_down();
                    return true;
                }
            }
            Key::PageUp => {
                if !self.read_only {
                    // Step by 10x single_step
                    self.set_value(self.value + self.single_step * 10.0);
                    return true;
                }
            }
            Key::PageDown => {
                if !self.read_only {
                    self.set_value(self.value - self.single_step * 10.0);
                    return true;
                }
            }
            Key::Home => {
                if !self.read_only {
                    self.set_value(self.minimum);
                    return true;
                }
            }
            Key::End => {
                if !self.read_only {
                    self.set_value(self.maximum);
                    return true;
                }
            }
            Key::Enter => {
                if !self.read_only {
                    self.start_editing();
                    return true;
                }
            }
            _ => {
                // Start editing if a digit, minus, or decimal point is pressed
                if !self.read_only {
                    if let Some(ch) = event.text.chars().next() {
                        if ch.is_ascii_digit() || ch == '-' || ch == '+' || ch == '.' {
                            self.start_editing();
                            self.edit_text.clear();
                            self.cursor_pos = 0;
                            self.selection_start = None;
                            return self.handle_edit_key(event);
                        }
                    }
                }
            }
        }
        false
    }

    fn handle_edit_key(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Escape => {
                self.cancel_editing();
                return true;
            }
            Key::Enter => {
                self.finish_editing();
                return true;
            }
            Key::ArrowLeft => {
                if self.cursor_pos > 0 {
                    let prev_pos = self.edit_text[..self.cursor_pos]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);

                    if event.modifiers.shift {
                        if self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_pos);
                        }
                    } else {
                        self.selection_start = None;
                    }
                    self.cursor_pos = prev_pos;
                    self.base.update();
                }
                return true;
            }
            Key::ArrowRight => {
                if self.cursor_pos < self.edit_text.len() {
                    let next_pos = self.edit_text[self.cursor_pos..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor_pos + i)
                        .unwrap_or(self.edit_text.len());

                    if event.modifiers.shift {
                        if self.selection_start.is_none() {
                            self.selection_start = Some(self.cursor_pos);
                        }
                    } else {
                        self.selection_start = None;
                    }
                    self.cursor_pos = next_pos;
                    self.base.update();
                }
                return true;
            }
            Key::Home => {
                if event.modifiers.shift {
                    if self.selection_start.is_none() {
                        self.selection_start = Some(self.cursor_pos);
                    }
                } else {
                    self.selection_start = None;
                }
                self.cursor_pos = 0;
                self.base.update();
                return true;
            }
            Key::End => {
                if event.modifiers.shift {
                    if self.selection_start.is_none() {
                        self.selection_start = Some(self.cursor_pos);
                    }
                } else {
                    self.selection_start = None;
                }
                self.cursor_pos = self.edit_text.len();
                self.base.update();
                return true;
            }
            Key::Backspace => {
                if !self.delete_selection() && self.cursor_pos > 0 {
                    let prev_pos = self.edit_text[..self.cursor_pos]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.edit_text.replace_range(prev_pos..self.cursor_pos, "");
                    self.cursor_pos = prev_pos;
                }
                self.base.update();
                return true;
            }
            Key::Delete => {
                if !self.delete_selection() && self.cursor_pos < self.edit_text.len() {
                    let next_pos = self.edit_text[self.cursor_pos..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| self.cursor_pos + i)
                        .unwrap_or(self.edit_text.len());
                    self.edit_text.replace_range(self.cursor_pos..next_pos, "");
                }
                self.base.update();
                return true;
            }
            Key::ArrowUp => {
                self.finish_editing();
                self.step_up();
                return true;
            }
            Key::ArrowDown => {
                self.finish_editing();
                self.step_down();
                return true;
            }
            _ => {
                // Handle text input
                if !event.text.is_empty() {
                    let ch = event.text.chars().next().unwrap();
                    // Allow digits, minus sign (at start), and decimal point (once)
                    let allow_char = ch.is_ascii_digit()
                        || (ch == '-' && self.cursor_pos == 0 && self.minimum < 0.0)
                        || (ch == '.' && !self.edit_text.contains('.'));

                    if allow_char {
                        self.insert_text(&event.text);
                        self.base.update();
                        return true;
                    }
                }
            }
        }
        false
    }

    fn handle_focus_out(&mut self) -> bool {
        self.finish_editing();
        false
    }

    fn handle_leave(&mut self) -> bool {
        if self.hover_part != DoubleSpinBoxPart::None {
            self.hover_part = DoubleSpinBoxPart::None;
            self.base.update();
        }
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Draw main background
        let bg_rrect = RoundedRect::new(rect, self.border_radius);
        ctx.renderer().fill_rounded_rect(bg_rrect, self.background_color);

        // Draw border
        let border_stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(bg_rrect, &border_stroke);
    }

    fn paint_text_field(&self, ctx: &mut PaintContext<'_>) {
        let text_rect = self.text_field_rect();
        let display = if self.editing {
            self.edit_text.clone()
        } else {
            self.display_text()
        };

        if display.is_empty() {
            return;
        }

        let mut font_system = FontSystem::new();
        let layout = TextLayout::with_options(
            &mut font_system,
            &display,
            &self.font,
            TextLayoutOptions::new()
                .horizontal_align(HorizontalAlign::Right)
                .vertical_align(VerticalAlign::Middle),
        );

        // Center text vertically, right-align with padding
        let padding = 6.0;
        let text_x = text_rect.origin.x + text_rect.width() - layout.width() - padding;
        let text_y = text_rect.origin.y + (text_rect.height() - layout.height()) / 2.0;
        let text_pos = Point::new(text_x, text_y);

        // Draw selection background if editing with selection
        if self.editing {
            if let Some(sel_start) = self.selection_start {
                let (start, end) = if sel_start < self.cursor_pos {
                    (sel_start, self.cursor_pos)
                } else {
                    (self.cursor_pos, sel_start)
                };
                if start != end {
                    let selection_color = Color::from_rgba8(66, 133, 244, 100);
                    let sel_rect = Rect::new(
                        text_x,
                        text_y,
                        layout.width(),
                        layout.height(),
                    );
                    ctx.renderer().fill_rect(sel_rect, selection_color);
                }
            }
        }

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                text_pos,
                self.text_color,
            );
        }

        // Draw cursor if editing
        if self.editing && self.widget_base().has_focus() {
            let cursor_x = text_x + layout.width();
            let cursor_y = text_y;
            let cursor_height = layout.height();
            let cursor_stroke = Stroke::new(self.text_color, 1.0);
            ctx.renderer().draw_line(
                Point::new(cursor_x, cursor_y),
                Point::new(cursor_x, cursor_y + cursor_height),
                &cursor_stroke,
            );
        }
    }

    fn paint_buttons(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        let sep_x = rect.width() - self.button_width;
        let sep_stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(sep_x, 0.0),
            Point::new(sep_x, rect.height()),
            &sep_stroke,
        );

        // Draw horizontal separator between buttons
        let mid_y = rect.height() / 2.0;
        ctx.renderer().draw_line(
            Point::new(sep_x, mid_y),
            Point::new(rect.width(), mid_y),
            &sep_stroke,
        );

        // Draw up button
        let up_rect = self.up_button_rect();
        let up_color = if self.pressed_part == DoubleSpinBoxPart::UpButton {
            self.button_pressed_color
        } else if self.hover_part == DoubleSpinBoxPart::UpButton {
            self.button_hover_color
        } else {
            self.button_color
        };
        ctx.renderer().fill_rect(up_rect, up_color);
        self.paint_arrow(ctx, up_rect, true);

        // Draw down button
        let down_rect = self.down_button_rect();
        let down_color = if self.pressed_part == DoubleSpinBoxPart::DownButton {
            self.button_pressed_color
        } else if self.hover_part == DoubleSpinBoxPart::DownButton {
            self.button_hover_color
        } else {
            self.button_color
        };
        ctx.renderer().fill_rect(down_rect, down_color);
        self.paint_arrow(ctx, down_rect, false);
    }

    fn paint_arrow(&self, ctx: &mut PaintContext<'_>, rect: Rect, up: bool) {
        let center_x = rect.origin.x + rect.width() / 2.0;
        let center_y = rect.origin.y + rect.height() / 2.0;
        let arrow_size = 4.0;
        let arrow_color = Color::from_rgb8(80, 80, 80);
        let stroke = Stroke::new(arrow_color, 1.5);

        if up {
            let p1 = Point::new(center_x - arrow_size, center_y + arrow_size / 2.0);
            let p2 = Point::new(center_x, center_y - arrow_size / 2.0);
            let p3 = Point::new(center_x + arrow_size, center_y + arrow_size / 2.0);
            ctx.renderer().draw_line(p1, p2, &stroke);
            ctx.renderer().draw_line(p2, p3, &stroke);
        } else {
            let p1 = Point::new(center_x - arrow_size, center_y - arrow_size / 2.0);
            let p2 = Point::new(center_x, center_y + arrow_size / 2.0);
            let p3 = Point::new(center_x + arrow_size, center_y - arrow_size / 2.0);
            ctx.renderer().draw_line(p1, p2, &stroke);
            ctx.renderer().draw_line(p2, p3, &stroke);
        }
    }

    fn paint_focus_indicator(&self, ctx: &mut PaintContext<'_>) {
        if !self.widget_base().has_focus() {
            return;
        }

        let rect = ctx.rect();
        let focus_color = Color::from_rgba8(66, 133, 244, 180);
        let focus_stroke = Stroke::new(focus_color, 2.0);
        let focus_rrect = RoundedRect::new(rect, self.border_radius);
        ctx.renderer().stroke_rounded_rect(focus_rrect, &focus_stroke);
    }
}

impl Default for DoubleSpinBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for DoubleSpinBox {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for DoubleSpinBox {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Calculate based on max value digits + decimals + prefix + suffix
        let max_int = self.maximum.abs().max(self.minimum.abs()) as i64;
        let int_digits = if max_int == 0 { 1 } else { (max_int as f64).log10().floor() as usize + 1 };
        let sign_width = if self.minimum < 0.0 { 1 } else { 0 };
        let decimal_part = if self.decimals > 0 { self.decimals as usize + 1 } else { 0 };
        let total_chars = int_digits + sign_width + decimal_part + self.prefix.len() + self.suffix.len();
        let text_width = total_chars as f32 * 10.0;

        let width = (text_width + self.button_width + 16.0).max(100.0);
        let height = 28.0;

        SizeHint::from_dimensions(width, height)
            .with_minimum_dimensions(70.0, 22.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);
        self.paint_text_field(ctx);
        self.paint_buttons(ctx);
        self.paint_focus_indicator(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
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
            WidgetEvent::FocusOut(_) => {
                self.handle_focus_out();
            }
            WidgetEvent::Leave(_) => {
                self.handle_leave();
            }
            _ => {}
        }
        false
    }
}

// Ensure DoubleSpinBox is Send + Sync
static_assertions::assert_impl_all!(DoubleSpinBox: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use parking_lot::Mutex;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_double_spinbox_creation() {
        setup();
        let spinbox = DoubleSpinBox::new();
        assert!((spinbox.value() - 0.0).abs() < f64::EPSILON);
        assert!((spinbox.minimum() - 0.0).abs() < f64::EPSILON);
        assert!((spinbox.maximum() - 99.99).abs() < 0.001);
        assert!((spinbox.single_step() - 1.0).abs() < f64::EPSILON);
        assert_eq!(spinbox.decimals(), 2);
        assert!(!spinbox.wrapping());
    }

    #[test]
    fn test_double_spinbox_builder_pattern() {
        setup();
        let spinbox = DoubleSpinBox::new()
            .with_range(-100.0, 100.0)
            .with_value(50.5)
            .with_single_step(0.5)
            .with_decimals(1)
            .with_prefix("$")
            .with_suffix(" USD")
            .with_wrapping(true);

        assert!((spinbox.minimum() - (-100.0)).abs() < f64::EPSILON);
        assert!((spinbox.maximum() - 100.0).abs() < f64::EPSILON);
        assert!((spinbox.value() - 50.5).abs() < f64::EPSILON);
        assert!((spinbox.single_step() - 0.5).abs() < f64::EPSILON);
        assert_eq!(spinbox.decimals(), 1);
        assert_eq!(spinbox.prefix(), "$");
        assert_eq!(spinbox.suffix(), " USD");
        assert!(spinbox.wrapping());
    }

    #[test]
    fn test_value_clamping() {
        setup();
        let mut spinbox = DoubleSpinBox::new().with_range(0.0, 100.0);

        spinbox.set_value(-10.0);
        assert!((spinbox.value() - 0.0).abs() < f64::EPSILON);

        spinbox.set_value(150.0);
        assert!((spinbox.value() - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_step_up_down() {
        setup();
        let mut spinbox = DoubleSpinBox::new()
            .with_range(0.0, 100.0)
            .with_value(50.0)
            .with_single_step(0.5);

        spinbox.step_up();
        assert!((spinbox.value() - 50.5).abs() < f64::EPSILON);

        spinbox.step_down();
        assert!((spinbox.value() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_value_changed_signal() {
        setup();
        let mut spinbox = DoubleSpinBox::new();
        let last_value = Arc::new(Mutex::new(-1.0f64));
        let last_value_clone = last_value.clone();

        spinbox.value_changed.connect(move |&value| {
            *last_value_clone.lock() = value;
        });

        spinbox.set_value(42.5);
        assert!((*last_value.lock() - 42.5).abs() < f64::EPSILON);

        spinbox.set_value(75.0);
        assert!((*last_value.lock() - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_no_signal_for_same_value() {
        setup();
        let mut spinbox = DoubleSpinBox::new().with_value(50.0);
        let signal_fired = Arc::new(AtomicBool::new(false));
        let signal_fired_clone = signal_fired.clone();

        spinbox.value_changed.connect(move |_| {
            signal_fired_clone.store(true, Ordering::SeqCst);
        });

        spinbox.set_value(50.0);
        assert!(!signal_fired.load(Ordering::SeqCst));

        spinbox.set_value(51.0);
        assert!(signal_fired.load(Ordering::SeqCst));
    }

    #[test]
    fn test_range_change_clamps_value() {
        setup();
        let mut spinbox = DoubleSpinBox::new()
            .with_range(0.0, 100.0)
            .with_value(75.0);

        spinbox.set_range(0.0, 50.0);
        assert!((spinbox.value() - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_display_text() {
        setup();
        let spinbox = DoubleSpinBox::new()
            .with_value(42.5)
            .with_decimals(2)
            .with_prefix("$")
            .with_suffix(" USD");

        assert_eq!(spinbox.display_text(), "$42.50 USD");
    }

    #[test]
    fn test_special_value_text() {
        setup();
        let spinbox = DoubleSpinBox::new()
            .with_range(0.0, 100.0)
            .with_value(0.0)
            .with_special_value_text("Auto");

        assert_eq!(spinbox.display_text(), "Auto");
    }

    #[test]
    fn test_size_hint() {
        setup();
        let spinbox = DoubleSpinBox::new();
        let hint = spinbox.size_hint();

        assert!(hint.preferred.width >= 70.0);
        assert!(hint.preferred.height >= 22.0);
    }
}
