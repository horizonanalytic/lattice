//! SpinBox widget for integer input.
//!
//! The SpinBox widget provides a way to enter and modify integer values with:
//! - Increment/decrement buttons
//! - Direct text editing
//! - Range constraints (minimum, maximum)
//! - Step size configuration
//! - Optional prefix and suffix text
//! - Optional wrapping mode
//! - Keyboard and mouse wheel support
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::SpinBox;
//!
//! // Create a simple spinbox
//! let mut spinbox = SpinBox::new()
//!     .with_range(0, 100)
//!     .with_value(50)
//!     .with_single_step(5);
//!
//! // Create a spinbox with prefix and suffix
//! let mut price = SpinBox::new()
//!     .with_range(0, 1000)
//!     .with_prefix("$")
//!     .with_suffix(".00");
//!
//! // Connect to value changes
//! spinbox.value_changed.connect(|&value| {
//!     println!("Value: {}", value);
//! });
//! ```

use std::time::{Duration, Instant};

use horizon_lattice_core::{Object, ObjectId, Signal, TimerId};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, HorizontalAlign, Point, Rect, Renderer, RoundedRect,
    Stroke, TextLayout, TextLayoutOptions, TextRenderer, VerticalAlign,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, TimerEvent, WheelEvent,
    Widget, WidgetBase, WidgetEvent,
};

/// A widget for entering and modifying integer values.
///
/// SpinBox provides a text field showing the current value with small up/down
/// buttons to increment or decrement the value by a configurable step.
///
/// # Signals
///
/// - `value_changed(i32)`: Emitted when the value changes
/// - `editing_finished()`: Emitted when editing is completed (focus lost or Enter pressed)
pub struct SpinBox {
    /// Widget base.
    base: WidgetBase,

    /// Current value.
    value: i32,

    /// Minimum value.
    minimum: i32,

    /// Maximum value.
    maximum: i32,

    /// Step size for increment/decrement.
    single_step: i32,

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
    hover_part: SpinBoxPart,

    /// Which button is currently pressed.
    pressed_part: SpinBoxPart,

    /// Whether the text field is being edited.
    editing: bool,

    /// Text buffer for editing.
    edit_text: String,

    /// Cursor position in the edit text (byte offset).
    cursor_pos: usize,

    /// Selection start (byte offset), if any.
    selection_start: Option<usize>,

    /// Signal emitted when value changes.
    pub value_changed: Signal<i32>,

    /// Signal emitted when editing is finished.
    pub editing_finished: Signal<()>,

    // =========================================================================
    // Acceleration on Hold
    // =========================================================================
    /// Whether acceleration is enabled for button holds.
    acceleration_enabled: bool,

    /// Delay before acceleration starts (default: 300ms).
    acceleration_delay: Duration,

    /// Current repeat interval for acceleration.
    /// Starts at acceleration_delay, decreases with acceleration.
    current_repeat_interval: Duration,

    /// Minimum repeat interval (maximum speed, default: 20ms).
    min_repeat_interval: Duration,

    /// Speed multiplier for acceleration (default: 1.5).
    /// Each repeat, interval is divided by this factor.
    acceleration_multiplier: f32,

    /// Timer ID for the repeat timer, if active.
    repeat_timer_id: Option<TimerId>,

    /// When the button press started (for acceleration timing).
    press_start_time: Option<Instant>,

    /// Number of repeats since button press (for acceleration curve).
    repeat_count: u32,
}

/// Parts of the spinbox for hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SpinBoxPart {
    #[default]
    None,
    /// The up (increment) button.
    UpButton,
    /// The down (decrement) button.
    DownButton,
    /// The text field area.
    TextField,
}

impl SpinBox {
    /// Create a new spinbox with default settings.
    ///
    /// Default configuration:
    /// - Range: 0 to 99
    /// - Value: 0
    /// - Step: 1
    /// - No prefix or suffix
    /// - No wrapping
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Fixed,
        ));

        Self {
            base,
            value: 0,
            minimum: 0,
            maximum: 99,
            single_step: 1,
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
            hover_part: SpinBoxPart::None,
            pressed_part: SpinBoxPart::None,
            editing: false,
            edit_text: String::new(),
            cursor_pos: 0,
            selection_start: None,
            value_changed: Signal::new(),
            editing_finished: Signal::new(),
            // Acceleration defaults
            acceleration_enabled: true,
            acceleration_delay: Duration::from_millis(300),
            current_repeat_interval: Duration::from_millis(300),
            min_repeat_interval: Duration::from_millis(20),
            acceleration_multiplier: 1.5,
            repeat_timer_id: None,
            press_start_time: None,
            repeat_count: 0,
        }
    }

    // =========================================================================
    // Value and Range
    // =========================================================================

    /// Get the current value.
    pub fn value(&self) -> i32 {
        self.value
    }

    /// Set the current value.
    ///
    /// The value is clamped to the valid range unless wrapping is enabled.
    pub fn set_value(&mut self, value: i32) {
        let new_value = if self.wrapping {
            self.wrap_value(value)
        } else {
            value.clamp(self.minimum, self.maximum)
        };

        if self.value != new_value {
            self.value = new_value;
            self.base.update();
            self.value_changed.emit(new_value);
        }
    }

    /// Set value using builder pattern.
    pub fn with_value(mut self, value: i32) -> Self {
        self.set_value(value);
        self
    }

    /// Get the minimum value.
    pub fn minimum(&self) -> i32 {
        self.minimum
    }

    /// Set the minimum value.
    pub fn set_minimum(&mut self, minimum: i32) {
        self.set_range(minimum, self.maximum);
    }

    /// Set minimum using builder pattern.
    pub fn with_minimum(mut self, minimum: i32) -> Self {
        self.set_minimum(minimum);
        self
    }

    /// Get the maximum value.
    pub fn maximum(&self) -> i32 {
        self.maximum
    }

    /// Set the maximum value.
    pub fn set_maximum(&mut self, maximum: i32) {
        self.set_range(self.minimum, maximum);
    }

    /// Set maximum using builder pattern.
    pub fn with_maximum(mut self, maximum: i32) -> Self {
        self.set_maximum(maximum);
        self
    }

    /// Set the value range.
    pub fn set_range(&mut self, minimum: i32, maximum: i32) {
        let (min, max) = if minimum <= maximum {
            (minimum, maximum)
        } else {
            (maximum, minimum)
        };

        if self.minimum != min || self.maximum != max {
            self.minimum = min;
            self.maximum = max;
            // Clamp current value to new range
            let new_value = self.value.clamp(min, max);
            if self.value != new_value {
                self.value = new_value;
                self.value_changed.emit(new_value);
            }
            self.base.update();
        }
    }

    /// Set range using builder pattern.
    pub fn with_range(mut self, minimum: i32, maximum: i32) -> Self {
        self.set_range(minimum, maximum);
        self
    }

    // =========================================================================
    // Step Size
    // =========================================================================

    /// Get the step size.
    pub fn single_step(&self) -> i32 {
        self.single_step
    }

    /// Set the step size.
    pub fn set_single_step(&mut self, step: i32) {
        self.single_step = step.max(1);
    }

    /// Set step using builder pattern.
    pub fn with_single_step(mut self, step: i32) -> Self {
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
    // Acceleration on Hold
    // =========================================================================

    /// Check if acceleration is enabled.
    ///
    /// When enabled, holding the up/down buttons will repeat the action
    /// with increasing speed.
    pub fn acceleration(&self) -> bool {
        self.acceleration_enabled
    }

    /// Set whether acceleration is enabled.
    ///
    /// When enabled, holding the up/down buttons will repeat the action
    /// with increasing speed. Default is `true`.
    pub fn set_acceleration(&mut self, enabled: bool) {
        self.acceleration_enabled = enabled;
    }

    /// Set acceleration using builder pattern.
    pub fn with_acceleration(mut self, enabled: bool) -> Self {
        self.acceleration_enabled = enabled;
        self
    }

    /// Get the acceleration delay (initial delay before repeating starts).
    pub fn acceleration_delay(&self) -> Duration {
        self.acceleration_delay
    }

    /// Set the acceleration delay.
    ///
    /// This is the initial delay before the button action starts repeating
    /// when held down. Default is 300ms.
    pub fn set_acceleration_delay(&mut self, delay: Duration) {
        self.acceleration_delay = delay;
    }

    /// Set acceleration delay using builder pattern.
    pub fn with_acceleration_delay(mut self, delay: Duration) -> Self {
        self.acceleration_delay = delay;
        self
    }

    /// Get the acceleration multiplier.
    pub fn acceleration_multiplier(&self) -> f32 {
        self.acceleration_multiplier
    }

    /// Set the acceleration multiplier.
    ///
    /// Each time the timer fires, the repeat interval is divided by this factor,
    /// causing the repeat rate to increase. Default is 1.5.
    ///
    /// Higher values = faster acceleration. A value of 1.0 disables acceleration
    /// (constant repeat rate). Values less than 1.0 would slow down (not recommended).
    pub fn set_acceleration_multiplier(&mut self, multiplier: f32) {
        self.acceleration_multiplier = multiplier.max(1.0);
    }

    /// Set acceleration multiplier using builder pattern.
    pub fn with_acceleration_multiplier(mut self, multiplier: f32) -> Self {
        self.acceleration_multiplier = multiplier.max(1.0);
        self
    }

    // =========================================================================
    // Actions
    // =========================================================================

    /// Increment the value by single_step.
    pub fn step_up(&mut self) {
        self.set_value(self.value.saturating_add(self.single_step));
    }

    /// Decrement the value by single_step.
    pub fn step_down(&mut self) {
        self.set_value(self.value.saturating_sub(self.single_step));
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
    fn wrap_value(&self, value: i32) -> i32 {
        let range = self.maximum - self.minimum + 1;
        if range <= 0 {
            return self.minimum;
        }

        if value < self.minimum {
            self.maximum - ((self.minimum - value - 1) % range)
        } else if value > self.maximum {
            self.minimum + ((value - self.maximum - 1) % range)
        } else {
            value
        }
    }

    /// Get the display text for the current value.
    fn display_text(&self) -> String {
        if self.value == self.minimum
            && let Some(ref special) = self.special_value_text
        {
            return special.clone();
        }
        format!("{}{}{}", self.prefix, self.value, self.suffix)
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
    fn hit_test(&self, pos: Point) -> SpinBoxPart {
        if self.up_button_rect().contains(pos) {
            SpinBoxPart::UpButton
        } else if self.down_button_rect().contains(pos) {
            SpinBoxPart::DownButton
        } else if self.text_field_rect().contains(pos) {
            SpinBoxPart::TextField
        } else {
            SpinBoxPart::None
        }
    }

    /// Start editing the text field.
    fn start_editing(&mut self) {
        if self.read_only {
            return;
        }
        self.editing = true;
        // Initialize edit text with just the number (no prefix/suffix)
        self.edit_text = self.value.to_string();
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
        if let Ok(parsed) = self.edit_text.trim().parse::<i32>() {
            self.set_value(parsed);
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
    // Repeat Timer (for acceleration)
    // =========================================================================

    /// Start the repeat timer for button hold acceleration.
    fn start_repeat_timer(&mut self) {
        if !self.acceleration_enabled {
            return;
        }

        // Reset acceleration state
        self.current_repeat_interval = self.acceleration_delay;
        self.press_start_time = Some(Instant::now());
        self.repeat_count = 0;

        // Start the timer with the initial delay
        let timer_id = self.base.start_timer(self.acceleration_delay);
        self.repeat_timer_id = Some(timer_id);
    }

    /// Stop the repeat timer.
    fn stop_repeat_timer(&mut self) {
        if let Some(timer_id) = self.repeat_timer_id.take() {
            self.base.stop_timer(timer_id);
        }
        self.press_start_time = None;
        self.repeat_count = 0;
        self.current_repeat_interval = self.acceleration_delay;
    }

    /// Handle a timer event for button repeat.
    fn handle_timer(&mut self, event: &TimerEvent) -> bool {
        // Check if this is our repeat timer
        if self.repeat_timer_id != Some(event.id) {
            return false;
        }

        // Perform the repeat action based on which button is pressed
        match self.pressed_part {
            SpinBoxPart::UpButton => {
                self.step_up();
            }
            SpinBoxPart::DownButton => {
                self.step_down();
            }
            _ => {
                // Button was released, stop the timer
                self.stop_repeat_timer();
                return true;
            }
        }

        self.repeat_count += 1;

        // Calculate the next interval with acceleration
        if self.acceleration_multiplier > 1.0 {
            let new_interval = Duration::from_secs_f64(
                self.current_repeat_interval.as_secs_f64() / self.acceleration_multiplier as f64,
            );
            self.current_repeat_interval = new_interval.max(self.min_repeat_interval);
        }

        // Stop the old timer and start a new one with the updated interval
        if let Some(timer_id) = self.repeat_timer_id.take() {
            self.base.stop_timer(timer_id);
        }
        let timer_id = self.base.start_timer(self.current_repeat_interval);
        self.repeat_timer_id = Some(timer_id);

        self.base.update();
        true
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
            SpinBoxPart::UpButton => {
                if !self.read_only {
                    self.pressed_part = SpinBoxPart::UpButton;
                    self.step_up();
                    self.start_repeat_timer();
                    self.base.update();
                    return true;
                }
            }
            SpinBoxPart::DownButton => {
                if !self.read_only {
                    self.pressed_part = SpinBoxPart::DownButton;
                    self.step_down();
                    self.start_repeat_timer();
                    self.base.update();
                    return true;
                }
            }
            SpinBoxPart::TextField => {
                if !self.editing {
                    self.start_editing();
                }
                return true;
            }
            SpinBoxPart::None => {}
        }
        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.pressed_part != SpinBoxPart::None {
            self.pressed_part = SpinBoxPart::None;
            self.stop_repeat_timer();
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
                    self.set_value(self.value.saturating_add(self.single_step * 10));
                    return true;
                }
            }
            Key::PageDown => {
                if !self.read_only {
                    self.set_value(self.value.saturating_sub(self.single_step * 10));
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
                // Start editing if a digit or minus is pressed
                if !self.read_only
                    && let Some(ch) = event.text.chars().next()
                    && (ch.is_ascii_digit() || ch == '-' || ch == '+')
                {
                    self.start_editing();
                    self.edit_text.clear();
                    self.cursor_pos = 0;
                    self.selection_start = None;
                    return self.handle_edit_key(event);
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
                    // Find previous grapheme boundary
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
                    // Find next grapheme boundary
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
                    // Delete character before cursor
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
                    // Delete character after cursor
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
                // Step up while editing
                self.finish_editing();
                self.step_up();
                return true;
            }
            Key::ArrowDown => {
                // Step down while editing
                self.finish_editing();
                self.step_down();
                return true;
            }
            _ => {
                // Handle text input
                if !event.text.is_empty() {
                    let ch = event.text.chars().next().unwrap();
                    // Only allow digits and minus sign
                    if ch.is_ascii_digit()
                        || (ch == '-' && self.cursor_pos == 0 && self.minimum < 0)
                    {
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
        if self.hover_part != SpinBoxPart::None {
            self.hover_part = SpinBoxPart::None;
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
        ctx.renderer()
            .fill_rounded_rect(bg_rrect, self.background_color);

        // Draw border
        let border_stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(bg_rrect, &border_stroke);
    }

    fn paint_text_field(&self, ctx: &mut PaintContext<'_>) {
        let text_rect = self.text_field_rect();
        let display = if self.editing {
            &self.edit_text
        } else {
            &self.display_text()
        };

        if display.is_empty() {
            return;
        }

        let mut font_system = FontSystem::new();
        let layout = TextLayout::with_options(
            &mut font_system,
            display,
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
        if self.editing
            && let Some(sel_start) = self.selection_start
        {
            let (start, end) = if sel_start < self.cursor_pos {
                (sel_start, self.cursor_pos)
            } else {
                (self.cursor_pos, sel_start)
            };
            if start != end {
                // Draw selection highlight
                let selection_color = Color::from_rgba8(66, 133, 244, 100);
                // Approximate selection rect (simplified)
                let sel_rect = Rect::new(text_x, text_y, layout.width(), layout.height());
                ctx.renderer().fill_rect(sel_rect, selection_color);
            }
        }

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ =
                text_renderer.prepare_layout(&mut font_system, &layout, text_pos, self.text_color);
        }

        // Draw cursor if editing
        if self.editing && self.widget_base().has_focus() {
            let cursor_x = text_x + layout.width(); // Simplified - cursor at end
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
        // Draw button separator
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
        let up_color = if self.pressed_part == SpinBoxPart::UpButton {
            self.button_pressed_color
        } else if self.hover_part == SpinBoxPart::UpButton {
            self.button_hover_color
        } else {
            self.button_color
        };
        ctx.renderer().fill_rect(up_rect, up_color);
        self.paint_arrow(ctx, up_rect, true);

        // Draw down button
        let down_rect = self.down_button_rect();
        let down_color = if self.pressed_part == SpinBoxPart::DownButton {
            self.button_pressed_color
        } else if self.hover_part == SpinBoxPart::DownButton {
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
            // Up arrow (triangle pointing up)
            let p1 = Point::new(center_x - arrow_size, center_y + arrow_size / 2.0);
            let p2 = Point::new(center_x, center_y - arrow_size / 2.0);
            let p3 = Point::new(center_x + arrow_size, center_y + arrow_size / 2.0);
            ctx.renderer().draw_line(p1, p2, &stroke);
            ctx.renderer().draw_line(p2, p3, &stroke);
        } else {
            // Down arrow (triangle pointing down)
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
        ctx.renderer()
            .stroke_rounded_rect(focus_rrect, &focus_stroke);
    }
}

impl Default for SpinBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for SpinBox {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for SpinBox {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Calculate based on max value digits + prefix + suffix
        let max_digits = self
            .maximum
            .abs()
            .to_string()
            .len()
            .max(self.minimum.abs().to_string().len());
        let sign_width = if self.minimum < 0 { 1 } else { 0 };
        let text_width =
            (max_digits + sign_width + self.prefix.len() + self.suffix.len()) as f32 * 10.0;

        let width = (text_width + self.button_width + 16.0).max(80.0);
        let height = 28.0;

        SizeHint::from_dimensions(width, height).with_minimum_dimensions(60.0, 22.0)
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
            WidgetEvent::Timer(e) => {
                if self.handle_timer(e) {
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

// Ensure SpinBox is Send + Sync
static_assertions::assert_impl_all!(SpinBox: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_spinbox_creation() {
        setup();
        let spinbox = SpinBox::new();
        assert_eq!(spinbox.value(), 0);
        assert_eq!(spinbox.minimum(), 0);
        assert_eq!(spinbox.maximum(), 99);
        assert_eq!(spinbox.single_step(), 1);
        assert!(!spinbox.wrapping());
    }

    #[test]
    fn test_spinbox_builder_pattern() {
        setup();
        let spinbox = SpinBox::new()
            .with_range(-100, 100)
            .with_value(50)
            .with_single_step(5)
            .with_prefix("$")
            .with_suffix(".00")
            .with_wrapping(true);

        assert_eq!(spinbox.minimum(), -100);
        assert_eq!(spinbox.maximum(), 100);
        assert_eq!(spinbox.value(), 50);
        assert_eq!(spinbox.single_step(), 5);
        assert_eq!(spinbox.prefix(), "$");
        assert_eq!(spinbox.suffix(), ".00");
        assert!(spinbox.wrapping());
    }

    #[test]
    fn test_value_clamping() {
        setup();
        let mut spinbox = SpinBox::new().with_range(0, 100);

        spinbox.set_value(-10);
        assert_eq!(spinbox.value(), 0);

        spinbox.set_value(150);
        assert_eq!(spinbox.value(), 100);
    }

    #[test]
    fn test_value_wrapping() {
        setup();
        let mut spinbox = SpinBox::new().with_range(0, 10).with_wrapping(true);

        spinbox.set_value(11);
        assert_eq!(spinbox.value(), 0);

        spinbox.set_value(-1);
        assert_eq!(spinbox.value(), 10);
    }

    #[test]
    fn test_step_up_down() {
        setup();
        let mut spinbox = SpinBox::new()
            .with_range(0, 100)
            .with_value(50)
            .with_single_step(5);

        spinbox.step_up();
        assert_eq!(spinbox.value(), 55);

        spinbox.step_down();
        assert_eq!(spinbox.value(), 50);
    }

    #[test]
    fn test_value_changed_signal() {
        setup();
        let mut spinbox = SpinBox::new();
        let last_value = Arc::new(AtomicI32::new(-1));
        let last_value_clone = last_value.clone();

        spinbox.value_changed.connect(move |&value| {
            last_value_clone.store(value, Ordering::SeqCst);
        });

        spinbox.set_value(42);
        assert_eq!(last_value.load(Ordering::SeqCst), 42);

        spinbox.set_value(75);
        assert_eq!(last_value.load(Ordering::SeqCst), 75);
    }

    #[test]
    fn test_no_signal_for_same_value() {
        setup();
        let mut spinbox = SpinBox::new().with_value(50);
        let signal_count = Arc::new(AtomicI32::new(0));
        let signal_count_clone = signal_count.clone();

        spinbox.value_changed.connect(move |_| {
            signal_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        spinbox.set_value(50);
        assert_eq!(signal_count.load(Ordering::SeqCst), 0);

        spinbox.set_value(51);
        assert_eq!(signal_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_range_change_clamps_value() {
        setup();
        let mut spinbox = SpinBox::new().with_range(0, 100).with_value(75);

        spinbox.set_range(0, 50);
        assert_eq!(spinbox.value(), 50);
    }

    #[test]
    fn test_display_text() {
        setup();
        let spinbox = SpinBox::new()
            .with_value(42)
            .with_prefix("$")
            .with_suffix(".00");

        assert_eq!(spinbox.display_text(), "$42.00");
    }

    #[test]
    fn test_special_value_text() {
        setup();
        let spinbox = SpinBox::new()
            .with_range(0, 100)
            .with_value(0)
            .with_special_value_text("Auto");

        assert_eq!(spinbox.display_text(), "Auto");
    }

    #[test]
    fn test_size_hint() {
        setup();
        let spinbox = SpinBox::new();
        let hint = spinbox.size_hint();

        assert!(hint.preferred.width >= 60.0);
        assert!(hint.preferred.height >= 22.0);
    }

    #[test]
    fn test_acceleration_defaults() {
        setup();
        let spinbox = SpinBox::new();

        // Acceleration should be enabled by default
        assert!(spinbox.acceleration());
        assert_eq!(spinbox.acceleration_delay(), Duration::from_millis(300));
        assert_eq!(spinbox.acceleration_multiplier(), 1.5);
    }

    #[test]
    fn test_acceleration_api() {
        setup();
        let mut spinbox = SpinBox::new();

        // Test set_acceleration
        spinbox.set_acceleration(false);
        assert!(!spinbox.acceleration());
        spinbox.set_acceleration(true);
        assert!(spinbox.acceleration());

        // Test set_acceleration_delay
        spinbox.set_acceleration_delay(Duration::from_millis(500));
        assert_eq!(spinbox.acceleration_delay(), Duration::from_millis(500));

        // Test set_acceleration_multiplier
        spinbox.set_acceleration_multiplier(2.0);
        assert_eq!(spinbox.acceleration_multiplier(), 2.0);

        // Test multiplier clamping (should not go below 1.0)
        spinbox.set_acceleration_multiplier(0.5);
        assert_eq!(spinbox.acceleration_multiplier(), 1.0);
    }

    #[test]
    fn test_acceleration_builder_pattern() {
        setup();
        let spinbox = SpinBox::new()
            .with_acceleration(false)
            .with_acceleration_delay(Duration::from_millis(200))
            .with_acceleration_multiplier(2.5);

        assert!(!spinbox.acceleration());
        assert_eq!(spinbox.acceleration_delay(), Duration::from_millis(200));
        assert_eq!(spinbox.acceleration_multiplier(), 2.5);
    }
}
