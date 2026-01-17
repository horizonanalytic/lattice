//! Color dialog implementation.
//!
//! This module provides [`ColorDialog`], a modal dialog for selecting colors
//! with HSV picker, color preview, and custom color palettes.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::ColorDialog;
//! use horizon_lattice_render::Color;
//!
//! // Using static helper
//! let mut dialog = ColorDialog::get_color(Some(Color::RED), "Select Color");
//! dialog.finished.connect(|result| {
//!     if result.is_accepted() {
//!         // Handle selected color
//!     }
//! });
//! dialog.open();
//!
//! // Using builder pattern
//! let mut dialog = ColorDialog::new()
//!     .with_color(Color::BLUE)
//!     .with_show_alpha(true)
//!     .with_title("Pick a Color");
//!
//! dialog.color_selected.connect(|&color| {
//!     println!("Selected color: {:?}", color);
//! });
//!
//! dialog.open();
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, RoundedRect, Stroke};

use crate::widget::{
    Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent, MouseReleaseEvent,
    PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent,
};

use super::dialog::{Dialog, DialogResult};
use super::dialog_button_box::StandardButton;
use super::native_dialogs::{self, NativeColorOptions};

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of custom colors that can be stored.
const MAX_CUSTOM_COLORS: usize = 16;

/// Maximum number of colors in the history.
const MAX_HISTORY_COLORS: usize = 8;

/// Size of color swatches in palettes.
const SWATCH_SIZE: f32 = 20.0;

/// Gap between swatches.
const SWATCH_GAP: f32 = 4.0;

// ============================================================================
// Drag Target
// ============================================================================

/// Identifies which part of the color dialog is being interacted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    None,
    SaturationValue,
    Hue,
    Alpha,
    CustomColor(usize),
    HistoryColor(usize),
}

// ============================================================================
// ColorDialog
// ============================================================================

/// A modal dialog for selecting colors.
///
/// ColorDialog provides a comprehensive color selection interface including:
///
/// - HSV picker (saturation/value square with hue bar)
/// - Optional alpha channel slider
/// - Color preview (current vs new)
/// - Custom colors palette (user-defined colors)
/// - Color history (recently used colors)
/// - Hex input field
///
/// # Static Helpers
///
/// For common use cases, use the static helper method:
///
/// - [`ColorDialog::get_color()`]: Show a dialog to select a color
///
/// # Signals
///
/// - `color_selected(Color)`: Emitted when dialog is accepted with the final color
/// - `color_changed(Color)`: Emitted during color selection (preview)
/// - `current_color_changed(Color)`: Emitted when the current/initial color changes
pub struct ColorDialog {
    /// The underlying dialog.
    dialog: Dialog,

    /// Current hue (0-360 degrees).
    hue: f32,

    /// Current saturation (0-1).
    saturation: f32,

    /// Current value/brightness (0-1).
    value: f32,

    /// Current alpha (0-1).
    alpha: f32,

    /// The initial/current color (for preview comparison).
    initial_color: Color,

    /// Whether to show the alpha slider.
    show_alpha: bool,

    /// Whether to show the hex input.
    show_hex_input: bool,

    /// The current hex text being edited.
    hex_text: String,

    /// Whether the hex input is focused.
    hex_focused: bool,

    /// Cursor position in hex text.
    hex_cursor_pos: usize,

    /// Custom colors palette.
    custom_colors: Vec<Color>,

    /// Color history (recently selected colors).
    history_colors: Vec<Color>,

    /// Currently selected custom color slot for editing.
    selected_custom_slot: Option<usize>,

    /// Current drag target.
    drag_target: DragTarget,

    // Layout constants
    /// Gap between components.
    gap: f32,
    /// Hue bar width.
    hue_bar_width: f32,
    /// Alpha bar width.
    alpha_bar_width: f32,
    /// Border radius for rounded corners.
    border_radius: f32,
    /// Border color.
    border_color: Color,
    /// Preview height.
    preview_height: f32,
    /// Palette section height.
    palette_height: f32,

    /// Whether to prefer native dialogs when available.
    use_native_dialog: bool,

    // Signals
    /// Signal emitted when the dialog is accepted with the selected color.
    pub color_selected: Signal<Color>,

    /// Signal emitted when the color changes during selection.
    pub color_changed: Signal<Color>,

    /// Signal emitted when the initial/current color changes.
    pub current_color_changed: Signal<Color>,
}

impl ColorDialog {
    /// Create a new color dialog with default settings.
    pub fn new() -> Self {
        let dialog = Dialog::new("Select Color")
            .with_size(420.0, 480.0)
            .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL);

        // Initialize with 16 custom color slots (default to white)
        let custom_colors = vec![Color::WHITE; MAX_CUSTOM_COLORS];

        Self {
            dialog,
            hue: 0.0,
            saturation: 0.0,
            value: 1.0,
            alpha: 1.0,
            initial_color: Color::WHITE,
            show_alpha: true,
            show_hex_input: true,
            hex_text: "#FFFFFF".to_string(),
            hex_focused: false,
            hex_cursor_pos: 0,
            custom_colors,
            history_colors: Vec::new(),
            selected_custom_slot: None,
            drag_target: DragTarget::None,
            gap: 8.0,
            hue_bar_width: 20.0,
            alpha_bar_width: 20.0,
            border_radius: 4.0,
            border_color: Color::from_rgb8(180, 180, 180),
            preview_height: 50.0,
            palette_height: 60.0,
            use_native_dialog: false,
            color_selected: Signal::new(),
            color_changed: Signal::new(),
            current_color_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Static Helper Methods
    // =========================================================================

    /// Create a color dialog to select a color.
    ///
    /// # Arguments
    ///
    /// * `initial` - The initial color (None for white)
    /// * `title` - The dialog title
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut dialog = ColorDialog::get_color(Some(Color::RED), "Choose Color");
    /// dialog.color_selected.connect(|&color| {
    ///     println!("Selected: {:?}", color);
    /// });
    /// dialog.open();
    /// ```
    pub fn get_color(initial: Option<Color>, title: impl Into<String>) -> Self {
        let mut dialog = Self::new().with_title(title);
        if let Some(color) = initial {
            dialog.set_color(color);
            dialog.initial_color = color;
        }
        dialog
    }

    /// Create a color dialog with alpha channel support.
    pub fn get_color_with_alpha(
        initial: Option<Color>,
        title: impl Into<String>,
        show_alpha: bool,
    ) -> Self {
        Self::get_color(initial, title).with_show_alpha(show_alpha)
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dialog.set_title(title);
        self
    }

    /// Set whether to prefer native dialogs using builder pattern.
    ///
    /// When enabled, the color dialog will use native system color pickers
    /// (ChooseColor on Windows) if available.
    pub fn with_native_dialog(mut self, use_native: bool) -> Self {
        self.use_native_dialog = use_native;
        self
    }

    /// Set the initial color using builder pattern.
    pub fn with_color(mut self, color: Color) -> Self {
        let (h, s, v, a) = color.to_hsva();
        self.hue = h;
        self.saturation = s;
        self.value = v;
        self.alpha = a;
        self.initial_color = color;
        self.update_hex_text();
        self
    }

    /// Set whether to show alpha slider using builder pattern.
    pub fn with_show_alpha(mut self, show: bool) -> Self {
        self.show_alpha = show;
        self
    }

    /// Set whether to show hex input using builder pattern.
    pub fn with_show_hex_input(mut self, show: bool) -> Self {
        self.show_hex_input = show;
        self
    }

    /// Set custom colors using builder pattern.
    pub fn with_custom_colors(mut self, colors: Vec<Color>) -> Self {
        self.custom_colors = colors;
        // Ensure we have exactly MAX_CUSTOM_COLORS slots
        self.custom_colors.resize(MAX_CUSTOM_COLORS, Color::WHITE);
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

    /// Get the current selected color.
    pub fn color(&self) -> Color {
        Color::from_hsva(self.hue, self.saturation, self.value, self.alpha)
    }

    /// Set the current color.
    pub fn set_color(&mut self, color: Color) {
        let (h, s, v, a) = color.to_hsva();
        let changed = (self.hue - h).abs() > 0.001
            || (self.saturation - s).abs() > 0.001
            || (self.value - v).abs() > 0.001
            || (self.alpha - a).abs() > 0.001;

        if changed {
            self.hue = h;
            self.saturation = s;
            self.value = v;
            self.alpha = a;
            self.update_hex_text();
            self.dialog.widget_base_mut().update();
            self.color_changed.emit(self.color());
        }
    }

    /// Get the initial/current color for preview.
    pub fn current_color(&self) -> Color {
        self.initial_color
    }

    /// Set the initial/current color for preview.
    pub fn set_current_color(&mut self, color: Color) {
        if self.initial_color != color {
            self.initial_color = color;
            self.dialog.widget_base_mut().update();
            self.current_color_changed.emit(color);
        }
    }

    /// Get whether alpha slider is shown.
    pub fn show_alpha(&self) -> bool {
        self.show_alpha
    }

    /// Set whether to show the alpha slider.
    pub fn set_show_alpha(&mut self, show: bool) {
        if self.show_alpha != show {
            self.show_alpha = show;
            self.dialog.widget_base_mut().update();
        }
    }

    /// Get the title.
    pub fn title(&self) -> &str {
        self.dialog.title()
    }

    /// Set the title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.dialog.set_title(title);
    }

    /// Get the custom colors.
    pub fn custom_colors(&self) -> &[Color] {
        &self.custom_colors
    }

    /// Set a custom color at the specified index.
    pub fn set_custom_color(&mut self, index: usize, color: Color) {
        if index < self.custom_colors.len() {
            self.custom_colors[index] = color;
            self.dialog.widget_base_mut().update();
        }
    }

    /// Get the color history.
    pub fn history_colors(&self) -> &[Color] {
        &self.history_colors
    }

    /// Add a color to the history.
    pub fn add_to_history(&mut self, color: Color) {
        // Remove if already exists
        self.history_colors.retain(|&c| c != color);
        // Add to front
        self.history_colors.insert(0, color);
        // Limit size
        if self.history_colors.len() > MAX_HISTORY_COLORS {
            self.history_colors.pop();
        }
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
    // Hex Input
    // =========================================================================

    /// Get the current hex text.
    pub fn hex_text(&self) -> &str {
        &self.hex_text
    }

    /// Update hex text from current color.
    fn update_hex_text(&mut self) {
        let color = self.color();
        // Unpremultiply alpha to get actual RGB values
        let (r, g, b) = if color.a > 0.0 {
            (
                (color.r / color.a * 255.0).round() as u8,
                (color.g / color.a * 255.0).round() as u8,
                (color.b / color.a * 255.0).round() as u8,
            )
        } else {
            (0, 0, 0)
        };

        if self.show_alpha {
            let a = (color.a * 255.0).round() as u8;
            self.hex_text = format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a);
        } else {
            self.hex_text = format!("#{:02X}{:02X}{:02X}", r, g, b);
        }
        self.hex_cursor_pos = self.hex_text.len();
    }

    /// Parse hex text and update color if valid.
    fn apply_hex_text(&mut self) {
        let text = self.hex_text.trim();
        if let Some(color) = Self::parse_hex(text) {
            let (h, s, v, a) = color.to_hsva();
            self.hue = h;
            self.saturation = s;
            self.value = v;
            self.alpha = a;
            self.dialog.widget_base_mut().update();
            self.color_changed.emit(self.color());
        }
    }

    /// Parse a hex color string.
    fn parse_hex(text: &str) -> Option<Color> {
        let text = text.trim_start_matches('#');
        match text.len() {
            6 => {
                let r = u8::from_str_radix(&text[0..2], 16).ok()?;
                let g = u8::from_str_radix(&text[2..4], 16).ok()?;
                let b = u8::from_str_radix(&text[4..6], 16).ok()?;
                Some(Color::from_rgb8(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&text[0..2], 16).ok()?;
                let g = u8::from_str_radix(&text[2..4], 16).ok()?;
                let b = u8::from_str_radix(&text[4..6], 16).ok()?;
                let a = u8::from_str_radix(&text[6..8], 16).ok()?;
                Some(Color::from_rgba8(r, g, b, a))
            }
            _ => None,
        }
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the color dialog (non-blocking modal).
    ///
    /// If `use_native_dialog` is enabled and native color pickers are available,
    /// a native system color dialog will be shown instead.
    pub fn open(&mut self) {
        // Try native dialog if preferred and available
        if self.use_native_dialog && native_dialogs::is_available() {
            let options = NativeColorOptions::new()
                .initial_color(self.color())
                .show_alpha(self.show_alpha)
                .title(self.dialog.title());

            if let Some(color) = native_dialogs::pick_color(options) {
                // Set the color from native picker
                let (h, s, v, a) = color.to_hsva();
                self.hue = h;
                self.saturation = s;
                self.value = v;
                self.alpha = a;
                self.update_hex_text();

                self.add_to_history(color);
                self.color_selected.emit(color);
                return;
            }
            // Native dialog cancelled or not available - don't fall through
            return;
        }

        // Use custom dialog
        self.dialog.open();
    }

    /// Accept the dialog and emit selected color.
    pub fn accept(&mut self) {
        let color = self.color();
        self.add_to_history(color);
        self.color_selected.emit(color);
        self.dialog.accept();
    }

    /// Reject the dialog.
    pub fn reject(&mut self) {
        self.dialog.reject();
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.dialog.close();
    }

    // =========================================================================
    // Signal Access (delegated from dialog)
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

    /// Get the content rect (inside dialog, below title bar).
    fn content_rect(&self) -> Rect {
        self.dialog.content_rect()
    }

    /// Get the picker area rectangle (HSV picker).
    fn picker_rect(&self) -> Rect {
        let content = self.content_rect();
        let picker_height = 200.0;
        Rect::new(content.left(), content.top(), content.width(), picker_height)
    }

    /// Get the saturation/value square rectangle.
    fn sv_rect(&self) -> Rect {
        let picker = self.picker_rect();
        let alpha_width = if self.show_alpha {
            self.alpha_bar_width + self.gap
        } else {
            0.0
        };
        let sv_width = picker.width() - self.hue_bar_width - self.gap - alpha_width;
        Rect::new(picker.left(), picker.top(), sv_width, picker.height())
    }

    /// Get the hue bar rectangle.
    fn hue_rect(&self) -> Rect {
        let picker = self.picker_rect();
        let sv_rect = self.sv_rect();
        Rect::new(
            sv_rect.right() + self.gap,
            picker.top(),
            self.hue_bar_width,
            picker.height(),
        )
    }

    /// Get the alpha bar rectangle.
    fn alpha_rect(&self) -> Rect {
        let picker = self.picker_rect();
        let hue_rect = self.hue_rect();
        Rect::new(
            hue_rect.right() + self.gap,
            picker.top(),
            self.alpha_bar_width,
            picker.height(),
        )
    }

    /// Get the preview area rectangle.
    fn preview_rect(&self) -> Rect {
        let picker = self.picker_rect();
        let content = self.content_rect();
        Rect::new(
            content.left(),
            picker.bottom() + self.gap,
            content.width(),
            self.preview_height,
        )
    }

    /// Get the hex input rectangle.
    fn hex_input_rect(&self) -> Rect {
        let preview = self.preview_rect();
        let content = self.content_rect();
        Rect::new(
            content.left(),
            preview.bottom() + self.gap,
            content.width() * 0.4,
            24.0,
        )
    }

    /// Get the custom colors palette rectangle.
    fn custom_colors_rect(&self) -> Rect {
        let hex_rect = self.hex_input_rect();
        let content = self.content_rect();
        Rect::new(
            content.left(),
            hex_rect.bottom() + self.gap * 2.0,
            content.width(),
            self.palette_height,
        )
    }

    /// Get the history colors palette rectangle.
    fn history_rect(&self) -> Rect {
        let custom_rect = self.custom_colors_rect();
        let content = self.content_rect();
        Rect::new(
            content.left(),
            custom_rect.bottom() + self.gap,
            content.width(),
            SWATCH_SIZE + 8.0,
        )
    }

    /// Get a custom color swatch rectangle.
    fn custom_swatch_rect(&self, index: usize) -> Rect {
        let base = self.custom_colors_rect();
        let swatches_per_row = 8;
        let row = index / swatches_per_row;
        let col = index % swatches_per_row;
        let x = base.left() + col as f32 * (SWATCH_SIZE + SWATCH_GAP);
        let y = base.top() + 16.0 + row as f32 * (SWATCH_SIZE + SWATCH_GAP);
        Rect::new(x, y, SWATCH_SIZE, SWATCH_SIZE)
    }

    /// Get a history color swatch rectangle.
    fn history_swatch_rect(&self, index: usize) -> Rect {
        let base = self.history_rect();
        let x = base.left() + index as f32 * (SWATCH_SIZE + SWATCH_GAP);
        let y = base.top() + 16.0;
        Rect::new(x, y, SWATCH_SIZE, SWATCH_SIZE)
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    fn hit_test(&self, pos: Point) -> DragTarget {
        // Check hex input
        if self.show_hex_input && self.hex_input_rect().contains(pos) {
            return DragTarget::None; // Handle separately
        }

        // Check SV square
        let sv_rect = self.sv_rect();
        if sv_rect.contains(pos) {
            return DragTarget::SaturationValue;
        }

        // Check hue bar
        let hue_rect = self.hue_rect();
        if hue_rect.contains(pos) {
            return DragTarget::Hue;
        }

        // Check alpha bar
        if self.show_alpha {
            let alpha_rect = self.alpha_rect();
            if alpha_rect.contains(pos) {
                return DragTarget::Alpha;
            }
        }

        // Check custom colors
        for i in 0..self.custom_colors.len() {
            if self.custom_swatch_rect(i).contains(pos) {
                return DragTarget::CustomColor(i);
            }
        }

        // Check history colors
        for i in 0..self.history_colors.len() {
            if self.history_swatch_rect(i).contains(pos) {
                return DragTarget::HistoryColor(i);
            }
        }

        DragTarget::None
    }

    // =========================================================================
    // Value Calculations
    // =========================================================================

    fn update_sv_from_pos(&mut self, pos: Point) {
        let sv_rect = self.sv_rect();
        let s = ((pos.x - sv_rect.left()) / sv_rect.width()).clamp(0.0, 1.0);
        let v = 1.0 - ((pos.y - sv_rect.top()) / sv_rect.height()).clamp(0.0, 1.0);
        self.saturation = s;
        self.value = v;
    }

    fn update_hue_from_pos(&mut self, pos: Point) {
        let hue_rect = self.hue_rect();
        let t = ((pos.y - hue_rect.top()) / hue_rect.height()).clamp(0.0, 1.0);
        self.hue = t * 360.0;
    }

    fn update_alpha_from_pos(&mut self, pos: Point) {
        let alpha_rect = self.alpha_rect();
        let t = ((pos.y - alpha_rect.top()) / alpha_rect.height()).clamp(0.0, 1.0);
        self.alpha = 1.0 - t;
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check hex input click
        if self.show_hex_input && self.hex_input_rect().contains(pos) {
            self.hex_focused = true;
            self.dialog.widget_base_mut().update();
            return true;
        }

        // Unfocus hex input when clicking elsewhere
        if self.hex_focused {
            self.hex_focused = false;
            self.dialog.widget_base_mut().update();
        }

        let target = self.hit_test(pos);
        match target {
            DragTarget::SaturationValue | DragTarget::Hue | DragTarget::Alpha => {
                self.drag_target = target;
                self.update_from_mouse(pos);
                true
            }
            DragTarget::CustomColor(index) => {
                // Double-click would set custom color, single click selects it
                let color = self.custom_colors[index];
                self.set_color(color);
                self.selected_custom_slot = Some(index);
                true
            }
            DragTarget::HistoryColor(index) => {
                let color = self.history_colors[index];
                self.set_color(color);
                true
            }
            DragTarget::None => false,
        }
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        if self.drag_target != DragTarget::None {
            self.update_from_mouse(event.local_pos);
            true
        } else {
            false
        }
    }

    fn handle_mouse_release(&mut self, _event: &MouseReleaseEvent) -> bool {
        if self.drag_target != DragTarget::None {
            self.drag_target = DragTarget::None;
            true
        } else {
            false
        }
    }

    fn update_from_mouse(&mut self, pos: Point) {
        match self.drag_target {
            DragTarget::SaturationValue => self.update_sv_from_pos(pos),
            DragTarget::Hue => self.update_hue_from_pos(pos),
            DragTarget::Alpha => self.update_alpha_from_pos(pos),
            _ => return,
        }
        self.update_hex_text();
        self.dialog.widget_base_mut().update();
        self.color_changed.emit(self.color());
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Handle hex input
        if self.hex_focused {
            return self.handle_hex_key_press(event);
        }

        // Enter to accept
        if event.key == Key::Enter && !event.is_repeat {
            self.accept();
            return true;
        }

        // Escape to reject (handled by dialog)
        // Arrow keys for fine adjustment
        let step = if event.modifiers.shift { 10.0 } else { 1.0 };
        match event.key {
            Key::ArrowLeft => {
                self.saturation = (self.saturation - 0.01 * step).clamp(0.0, 1.0);
                self.update_hex_text();
                self.dialog.widget_base_mut().update();
                self.color_changed.emit(self.color());
                true
            }
            Key::ArrowRight => {
                self.saturation = (self.saturation + 0.01 * step).clamp(0.0, 1.0);
                self.update_hex_text();
                self.dialog.widget_base_mut().update();
                self.color_changed.emit(self.color());
                true
            }
            Key::ArrowUp => {
                self.value = (self.value + 0.01 * step).clamp(0.0, 1.0);
                self.update_hex_text();
                self.dialog.widget_base_mut().update();
                self.color_changed.emit(self.color());
                true
            }
            Key::ArrowDown => {
                self.value = (self.value - 0.01 * step).clamp(0.0, 1.0);
                self.update_hex_text();
                self.dialog.widget_base_mut().update();
                self.color_changed.emit(self.color());
                true
            }
            _ => false,
        }
    }

    fn handle_hex_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::ArrowLeft => {
                if self.hex_cursor_pos > 0 {
                    self.hex_cursor_pos -= 1;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            Key::ArrowRight => {
                if self.hex_cursor_pos < self.hex_text.len() {
                    self.hex_cursor_pos += 1;
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            Key::Home => {
                self.hex_cursor_pos = 0;
                self.dialog.widget_base_mut().update();
                true
            }
            Key::End => {
                self.hex_cursor_pos = self.hex_text.len();
                self.dialog.widget_base_mut().update();
                true
            }
            Key::Backspace => {
                if self.hex_cursor_pos > 0 {
                    self.hex_text.remove(self.hex_cursor_pos - 1);
                    self.hex_cursor_pos -= 1;
                    self.apply_hex_text();
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            Key::Delete => {
                if self.hex_cursor_pos < self.hex_text.len() {
                    self.hex_text.remove(self.hex_cursor_pos);
                    self.apply_hex_text();
                    self.dialog.widget_base_mut().update();
                }
                true
            }
            Key::Escape => {
                self.update_hex_text();
                self.hex_focused = false;
                self.dialog.widget_base_mut().update();
                true
            }
            Key::Enter => {
                self.apply_hex_text();
                self.hex_focused = false;
                self.dialog.widget_base_mut().update();
                true
            }
            _ => {
                // Handle character input
                if !event.text.is_empty() && !event.modifiers.control && !event.modifiers.alt {
                    for c in event.text.chars() {
                        if c == '#' || c.is_ascii_hexdigit() {
                            self.hex_text.insert(self.hex_cursor_pos, c.to_ascii_uppercase());
                            self.hex_cursor_pos += 1;
                        }
                    }
                    self.apply_hex_text();
                    self.dialog.widget_base_mut().update();
                    return true;
                }
                false
            }
        }
    }

    // =========================================================================
    // Set Custom Color
    // =========================================================================

    /// Set the currently selected color to a custom color slot.
    pub fn set_color_to_custom_slot(&mut self, index: usize) {
        if index < self.custom_colors.len() {
            self.custom_colors[index] = self.color();
            self.dialog.widget_base_mut().update();
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_sv_square(&self, ctx: &mut PaintContext<'_>) {
        let sv_rect = self.sv_rect();

        // Paint the saturation/value gradient
        let steps = (sv_rect.width() / 2.0).ceil() as i32;
        let step_width = sv_rect.width() / steps as f32;

        for i in 0..steps {
            let s = i as f32 / (steps - 1).max(1) as f32;
            let x = sv_rect.left() + i as f32 * step_width;

            let v_steps = (sv_rect.height() / 2.0).ceil() as i32;
            let v_step_height = sv_rect.height() / v_steps as f32;

            for j in 0..v_steps {
                let v = 1.0 - j as f32 / (v_steps - 1).max(1) as f32;
                let y = sv_rect.top() + j as f32 * v_step_height;
                let color = Color::from_hsv(self.hue, s, v);
                ctx.renderer()
                    .fill_rect(Rect::new(x, y, step_width + 0.5, v_step_height + 0.5), color);
            }
        }

        // Draw border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(sv_rect, self.border_radius), &stroke);

        // Draw selection cursor
        let cursor_x = sv_rect.left() + self.saturation * sv_rect.width();
        let cursor_y = sv_rect.top() + (1.0 - self.value) * sv_rect.height();
        self.paint_cursor(ctx, Point::new(cursor_x, cursor_y));
    }

    fn paint_hue_bar(&self, ctx: &mut PaintContext<'_>) {
        let hue_rect = self.hue_rect();

        // Paint hue gradient
        let steps = (hue_rect.height() / 2.0).ceil() as i32;
        let step_height = hue_rect.height() / steps as f32;

        for i in 0..steps {
            let h = (i as f32 / (steps - 1).max(1) as f32) * 360.0;
            let y = hue_rect.top() + i as f32 * step_height;
            let color = Color::from_hsv(h, 1.0, 1.0);
            ctx.renderer().fill_rect(
                Rect::new(hue_rect.left(), y, hue_rect.width(), step_height + 0.5),
                color,
            );
        }

        // Draw border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(hue_rect, self.border_radius), &stroke);

        // Draw selection indicator
        let indicator_y = hue_rect.top() + (self.hue / 360.0) * hue_rect.height();
        self.paint_bar_indicator(ctx, hue_rect, indicator_y);
    }

    fn paint_alpha_bar(&self, ctx: &mut PaintContext<'_>) {
        let alpha_rect = self.alpha_rect();

        // Paint checkerboard background
        self.paint_checkerboard(ctx, alpha_rect);

        // Paint alpha gradient
        let steps = (alpha_rect.height() / 2.0).ceil() as i32;
        let step_height = alpha_rect.height() / steps as f32;
        let base_color = Color::from_hsv(self.hue, self.saturation, self.value);

        for i in 0..steps {
            let a = 1.0 - i as f32 / (steps - 1).max(1) as f32;
            let y = alpha_rect.top() + i as f32 * step_height;
            let color = base_color.with_alpha(a);
            ctx.renderer().fill_rect(
                Rect::new(alpha_rect.left(), y, alpha_rect.width(), step_height + 0.5),
                color,
            );
        }

        // Draw border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(alpha_rect, self.border_radius), &stroke);

        // Draw selection indicator
        let indicator_y = alpha_rect.top() + (1.0 - self.alpha) * alpha_rect.height();
        self.paint_bar_indicator(ctx, alpha_rect, indicator_y);
    }

    fn paint_cursor(&self, ctx: &mut PaintContext<'_>, pos: Point) {
        let radius = 6.0;

        // Outer circle (white)
        ctx.renderer().fill_circle(pos, radius, Color::WHITE);

        // Inner circle (black)
        ctx.renderer().fill_circle(pos, radius - 2.0, Color::BLACK);

        // Current color circle
        ctx.renderer().fill_circle(pos, radius - 3.0, self.color());
    }

    fn paint_bar_indicator(&self, ctx: &mut PaintContext<'_>, bar_rect: Rect, y: f32) {
        let indicator_height = 4.0;
        let indicator_rect = Rect::new(
            bar_rect.left() - 2.0,
            y - indicator_height / 2.0,
            bar_rect.width() + 4.0,
            indicator_height,
        );

        let rounded = RoundedRect::new(indicator_rect, 2.0);
        ctx.renderer().fill_rounded_rect(rounded, Color::WHITE);

        let stroke = Stroke::new(Color::BLACK, 1.0);
        ctx.renderer().stroke_rounded_rect(rounded, &stroke);
    }

    fn paint_checkerboard(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        let checker_size = 4.0;
        let light = Color::from_rgb8(255, 255, 255);
        let dark = Color::from_rgb8(200, 200, 200);

        let cols = (rect.width() / checker_size).ceil() as i32;
        let rows = (rect.height() / checker_size).ceil() as i32;

        for row in 0..rows {
            for col in 0..cols {
                let color = if (row + col) % 2 == 0 { light } else { dark };
                let x = rect.left() + col as f32 * checker_size;
                let y = rect.top() + row as f32 * checker_size;
                let w = checker_size.min(rect.right() - x);
                let h = checker_size.min(rect.bottom() - y);
                ctx.renderer().fill_rect(Rect::new(x, y, w, h), color);
            }
        }
    }

    fn paint_preview(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.preview_rect();

        // Draw label backgrounds
        let half_width = rect.width() / 2.0 - self.gap / 2.0;

        // "Current" color (left side)
        let current_rect = Rect::new(rect.left(), rect.top() + 16.0, half_width, rect.height() - 16.0);
        self.paint_checkerboard(ctx, current_rect);
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(current_rect, self.border_radius), self.initial_color);
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(current_rect, self.border_radius), &stroke);

        // "New" color (right side)
        let new_rect = Rect::new(
            rect.left() + half_width + self.gap,
            rect.top() + 16.0,
            half_width,
            rect.height() - 16.0,
        );
        self.paint_checkerboard(ctx, new_rect);
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(new_rect, self.border_radius), self.color());
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(new_rect, self.border_radius), &stroke);
    }

    fn paint_hex_input(&self, ctx: &mut PaintContext<'_>) {
        if !self.show_hex_input {
            return;
        }

        let rect = self.hex_input_rect();

        // Background
        let bg_color = if self.hex_focused {
            Color::WHITE
        } else {
            Color::from_rgb8(250, 250, 250)
        };
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(rect, self.border_radius), bg_color);

        // Border
        let border_color = if self.hex_focused {
            Color::from_rgb8(0, 123, 255)
        } else {
            self.border_color
        };
        let stroke = Stroke::new(border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(rect, self.border_radius), &stroke);

        // Draw cursor if focused
        if self.hex_focused {
            let char_width = 8.0;
            let cursor_x = rect.left() + 8.0 + self.hex_cursor_pos as f32 * char_width;
            let cursor_y = rect.top() + 4.0;
            let cursor_height = rect.height() - 8.0;
            ctx.renderer().fill_rect(
                Rect::new(cursor_x, cursor_y, 1.0, cursor_height),
                Color::BLACK,
            );
        }
    }

    fn paint_custom_colors(&self, ctx: &mut PaintContext<'_>) {
        // Draw each custom color swatch
        for (i, &color) in self.custom_colors.iter().enumerate() {
            let rect = self.custom_swatch_rect(i);

            // Checkerboard for alpha
            self.paint_checkerboard(ctx, rect);

            // Color
            ctx.renderer()
                .fill_rounded_rect(RoundedRect::new(rect, 2.0), color);

            // Border (highlight selected)
            let border_color = if self.selected_custom_slot == Some(i) {
                Color::from_rgb8(0, 123, 255)
            } else {
                self.border_color
            };
            let stroke = Stroke::new(border_color, 1.0);
            ctx.renderer()
                .stroke_rounded_rect(RoundedRect::new(rect, 2.0), &stroke);
        }
    }

    fn paint_history_colors(&self, ctx: &mut PaintContext<'_>) {
        // Draw each history color swatch
        for (i, &color) in self.history_colors.iter().enumerate() {
            let rect = self.history_swatch_rect(i);

            // Checkerboard for alpha
            self.paint_checkerboard(ctx, rect);

            // Color
            ctx.renderer()
                .fill_rounded_rect(RoundedRect::new(rect, 2.0), color);

            // Border
            let stroke = Stroke::new(self.border_color, 1.0);
            ctx.renderer()
                .stroke_rounded_rect(RoundedRect::new(rect, 2.0), &stroke);
        }
    }
}

impl Default for ColorDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ColorDialog {
    fn object_id(&self) -> ObjectId {
        self.dialog.object_id()
    }
}

impl Widget for ColorDialog {
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
        // Paint the dialog base
        self.dialog.paint(ctx);

        if !self.dialog.is_open() {
            return;
        }

        // Paint ColorDialog-specific content
        self.paint_sv_square(ctx);
        self.paint_hue_bar(ctx);
        if self.show_alpha {
            self.paint_alpha_bar(ctx);
        }
        self.paint_preview(ctx);
        self.paint_hex_input(ctx);
        self.paint_custom_colors(ctx);
        self.paint_history_colors(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle our own events first
        let handled = match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
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
static_assertions::assert_impl_all!(ColorDialog: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::Arc;

    fn setup() {
        let _ = init_global_registry();
    }

    #[test]
    fn test_color_dialog_creation() {
        setup();
        let dialog = ColorDialog::new();
        assert!(!dialog.is_open());
        assert!(dialog.show_alpha());
        // Default color is white
        let color = dialog.color();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 1.0).abs() < 0.01);
        assert!((color.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_dialog_builder() {
        setup();
        let dialog = ColorDialog::new()
            .with_title("Test Color")
            .with_color(Color::RED)
            .with_show_alpha(false);

        assert_eq!(dialog.title(), "Test Color");
        assert!(!dialog.show_alpha());
        // Color should be red
        let color = dialog.color();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!(color.g.abs() < 0.01);
        assert!(color.b.abs() < 0.01);
    }

    #[test]
    fn test_get_color_helper() {
        setup();
        let dialog = ColorDialog::get_color(Some(Color::BLUE), "Select Blue");
        assert_eq!(dialog.title(), "Select Blue");
        let color = dialog.color();
        assert!(color.r.abs() < 0.01);
        assert!(color.g.abs() < 0.01);
        assert!((color.b - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_dialog_set_color() {
        setup();
        let mut dialog = ColorDialog::new();
        dialog.set_color(Color::from_rgb8(128, 64, 192));
        let color = dialog.color();
        // Check that color was set (accounting for HSV conversion)
        assert!(color.r > 0.0);
        assert!(color.g > 0.0);
        assert!(color.b > 0.0);
    }

    #[test]
    fn test_custom_colors() {
        setup();
        let mut dialog = ColorDialog::new();
        assert_eq!(dialog.custom_colors().len(), MAX_CUSTOM_COLORS);

        dialog.set_custom_color(0, Color::RED);
        assert_eq!(dialog.custom_colors()[0], Color::RED);
    }

    #[test]
    fn test_color_history() {
        setup();
        let mut dialog = ColorDialog::new();
        assert!(dialog.history_colors().is_empty());

        dialog.add_to_history(Color::RED);
        assert_eq!(dialog.history_colors().len(), 1);

        dialog.add_to_history(Color::GREEN);
        assert_eq!(dialog.history_colors().len(), 2);
        // Most recent should be first
        assert_eq!(dialog.history_colors()[0], Color::GREEN);

        // Adding same color again should move it to front
        dialog.add_to_history(Color::RED);
        assert_eq!(dialog.history_colors().len(), 2);
        assert_eq!(dialog.history_colors()[0], Color::RED);
    }

    #[test]
    fn test_hex_parsing() {
        // Test 6-digit hex
        let color = ColorDialog::parse_hex("#FF0000").unwrap();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!(color.g.abs() < 0.01);
        assert!(color.b.abs() < 0.01);

        // Test 8-digit hex with alpha
        let color = ColorDialog::parse_hex("#FF000080").unwrap();
        // Note: Color uses premultiplied alpha
        assert!((color.a - 0.5).abs() < 0.02);

        // Test without #
        let color = ColorDialog::parse_hex("00FF00").unwrap();
        assert!(color.r.abs() < 0.01);
        assert!((color.g - 1.0).abs() < 0.01);
        assert!(color.b.abs() < 0.01);

        // Test invalid
        assert!(ColorDialog::parse_hex("invalid").is_none());
        assert!(ColorDialog::parse_hex("#GGG").is_none());
    }

    #[test]
    fn test_dialog_lifecycle() {
        setup();
        let mut dialog = ColorDialog::new();
        assert!(!dialog.is_open());

        dialog.open();
        assert!(dialog.is_open());

        dialog.close();
        assert!(!dialog.is_open());
    }

    #[test]
    fn test_color_selected_signal() {
        setup();
        let mut dialog = ColorDialog::new().with_color(Color::RED);

        let selected = Arc::new(std::sync::Mutex::new(Color::TRANSPARENT));
        let selected_clone = selected.clone();

        dialog.color_selected.connect(move |color| {
            *selected_clone.lock().unwrap() = *color;
        });

        dialog.open();
        dialog.accept();

        let result = *selected.lock().unwrap();
        // Should have emitted red
        assert!((result.r - 1.0).abs() < 0.01);
    }
}
