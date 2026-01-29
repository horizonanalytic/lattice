//! Color picker widget implementation.
//!
//! This module provides [`ColorPicker`], an inline widget for selecting colors
//! using an HSV (Hue-Saturation-Value) interface.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::ColorPicker;
//! use horizon_lattice_render::Color;
//!
//! // Create a color picker with initial red color
//! let mut picker = ColorPicker::new()
//!     .with_color(Color::RED);
//!
//! // Connect to color changed signal
//! picker.color_changed.connect(|&color| {
//!     println!("Color selected: {:?}", color);
//! });
//!
//! // Create a color picker with hex input
//! let mut picker = ColorPicker::new()
//!     .with_show_hex_input(true);
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, HorizontalAlign, Point, Rect, Renderer, RoundedRect,
    Stroke, TextLayout, TextLayoutOptions, TextRenderer, VerticalAlign,
};

use crate::widget::validator::{HexColorValidator, HexFormat, ValidationState, Validator};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

/// Identifies which part of the picker is being interacted with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    None,
    SaturationValue,
    Hue,
    Alpha,
    HexInput,
}

/// An inline color picker widget with HSV interface.
///
/// ColorPicker provides a saturation/value square, a hue bar, and optionally
/// an alpha slider for selecting colors. The selected color is represented
/// in HSV internally and converted to RGB for the color_changed signal.
///
/// # Layout
///
/// ```text
/// +------------------+---+---+
/// |                  |   |   |
/// |  Sat/Val Square  | H | A |
/// |                  | u | l |
/// |                  | e | p |
/// |                  |   | h |
/// +------------------+---+---+
/// | #RRGGBB          |       |  (optional hex input)
/// +------------------+-------+
/// ```
///
/// # Signals
///
/// - `color_changed(Color)`: Emitted whenever the color changes (during drag)
pub struct ColorPicker {
    /// Widget base.
    base: WidgetBase,

    /// Current hue (0-360 degrees).
    hue: f32,

    /// Current saturation (0-1).
    saturation: f32,

    /// Current value/brightness (0-1).
    value: f32,

    /// Current alpha (0-1).
    alpha: f32,

    /// Whether to show the alpha slider.
    show_alpha: bool,

    /// Current drag target.
    drag_target: DragTarget,

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

    // =========================================================================
    // Hex Input Fields
    // =========================================================================
    /// Whether to show the hex input field.
    show_hex_input: bool,

    /// The current hex text being edited.
    hex_text: String,

    /// Cursor position in the hex text (byte offset).
    hex_cursor_pos: usize,

    /// Selection anchor position (None if no selection).
    hex_selection_anchor: Option<usize>,

    /// Whether the hex input is currently focused.
    hex_focused: bool,

    /// Hex format options for display and validation.
    hex_format: HexFormat,

    /// Validator for hex input.
    hex_validator: HexColorValidator,

    /// Current validation state of the hex input.
    hex_validation_state: ValidationState,

    /// Height of the hex input field.
    hex_input_height: f32,

    /// Whether we're currently updating from the hex input (prevents recursion).
    updating_from_hex: bool,

    /// Font for hex input text.
    hex_font: Font,

    /// Signal emitted when the color changes.
    pub color_changed: Signal<Color>,
}

impl ColorPicker {
    /// Create a new color picker with default (white) color.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Fixed));

        let hex_format = HexFormat::new();
        let hex_validator = HexColorValidator::with_format(hex_format);

        Self {
            base,
            hue: 0.0,
            saturation: 0.0,
            value: 1.0,
            alpha: 1.0,
            show_alpha: true,
            drag_target: DragTarget::None,
            gap: 8.0,
            hue_bar_width: 20.0,
            alpha_bar_width: 20.0,
            border_radius: 4.0,
            border_color: Color::from_rgb8(180, 180, 180),
            // Hex input fields
            show_hex_input: false,
            hex_text: "#FFFFFF".to_string(),
            hex_cursor_pos: 0,
            hex_selection_anchor: None,
            hex_focused: false,
            hex_format,
            hex_validator,
            hex_validation_state: ValidationState::Acceptable,
            hex_input_height: 24.0,
            updating_from_hex: false,
            hex_font: Font::new(FontFamily::SansSerif, 13.0),
            color_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Color
    // =========================================================================

    /// Get the current color.
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
            // Update hex text from the new color (unless we're updating from hex input)
            if !self.updating_from_hex {
                self.update_hex_text_from_color();
            }
            self.base.update();
        }
    }

    /// Set the color using builder pattern.
    pub fn with_color(mut self, color: Color) -> Self {
        let (h, s, v, a) = color.to_hsva();
        self.hue = h;
        self.saturation = s;
        self.value = v;
        self.alpha = a;
        self
    }

    // =========================================================================
    // Alpha Display
    // =========================================================================

    /// Get whether alpha slider is shown.
    pub fn show_alpha(&self) -> bool {
        self.show_alpha
    }

    /// Set whether to show the alpha slider.
    pub fn set_show_alpha(&mut self, show: bool) {
        if self.show_alpha != show {
            self.show_alpha = show;
            self.base.update();
        }
    }

    /// Set show alpha using builder pattern.
    pub fn with_show_alpha(mut self, show: bool) -> Self {
        self.show_alpha = show;
        self
    }

    // =========================================================================
    // Hex Input
    // =========================================================================

    /// Get whether hex input field is shown.
    pub fn show_hex_input(&self) -> bool {
        self.show_hex_input
    }

    /// Set whether to show the hex input field.
    pub fn set_show_hex_input(&mut self, show: bool) {
        if self.show_hex_input != show {
            self.show_hex_input = show;
            if show {
                // Initialize hex text from current color
                self.update_hex_text_from_color();
            }
            self.base.update();
        }
    }

    /// Set show hex input using builder pattern.
    pub fn with_show_hex_input(mut self, show: bool) -> Self {
        self.show_hex_input = show;
        if show {
            self.update_hex_text_from_color();
        }
        self
    }

    /// Get the hex format options.
    pub fn hex_format(&self) -> &HexFormat {
        &self.hex_format
    }

    /// Set the hex format options.
    pub fn set_hex_format(&mut self, format: HexFormat) {
        self.hex_format = format;
        self.hex_validator = HexColorValidator::with_format(format);
        self.update_hex_text_from_color();
        self.base.update();
    }

    /// Set hex format using builder pattern.
    pub fn with_hex_format(mut self, format: HexFormat) -> Self {
        self.hex_format = format;
        self.hex_validator = HexColorValidator::with_format(format);
        self.update_hex_text_from_color();
        self
    }

    /// Get the current hex text.
    pub fn hex_text(&self) -> &str {
        &self.hex_text
    }

    /// Set the hex text directly (will update color if valid).
    pub fn set_hex_text(&mut self, text: &str) {
        self.hex_text = text.to_string();
        self.hex_cursor_pos = self.hex_text.len();
        self.hex_selection_anchor = None;
        self.validate_and_apply_hex();
        self.base.update();
    }

    /// Get the current validation state of the hex input.
    pub fn hex_validation_state(&self) -> ValidationState {
        self.hex_validation_state
    }

    /// Update hex text from the current color.
    fn update_hex_text_from_color(&mut self) {
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
        let a = (color.a * 255.0).round() as u8;

        self.hex_text = self.hex_format.format_color(r, g, b, a);
        self.hex_cursor_pos = self.hex_text.len();
        self.hex_selection_anchor = None;
        self.hex_validation_state = ValidationState::Acceptable;
    }

    /// Validate and apply the current hex text to update the color.
    fn validate_and_apply_hex(&mut self) {
        self.hex_validation_state = self.hex_validator.validate(&self.hex_text);

        if self.hex_validation_state == ValidationState::Acceptable
            && let Some((r, g, b, a)) = HexColorValidator::parse_hex(&self.hex_text)
        {
            let color = Color::from_rgba8(r, g, b, a);
            self.updating_from_hex = true;
            self.set_color(color);
            self.updating_from_hex = false;
            self.color_changed.emit(self.color());
        }
    }

    // =========================================================================
    // Layout Calculations
    // =========================================================================

    /// Calculate the height of the picker area (excluding hex input).
    fn picker_height(&self) -> f32 {
        let rect = self.base.rect();
        if self.show_hex_input {
            rect.height() - self.hex_input_height - self.gap
        } else {
            rect.height()
        }
    }

    /// Calculate the rectangle for the saturation/value square.
    fn sv_rect(&self) -> Rect {
        let rect = self.base.rect();
        let picker_height = self.picker_height();
        let alpha_width = if self.show_alpha {
            self.alpha_bar_width + self.gap
        } else {
            0.0
        };
        let sv_width = rect.width() - self.hue_bar_width - self.gap - alpha_width;
        Rect::new(rect.left(), rect.top(), sv_width, picker_height)
    }

    /// Calculate the rectangle for the hue bar.
    fn hue_rect(&self) -> Rect {
        let rect = self.base.rect();
        let picker_height = self.picker_height();
        let sv_rect = self.sv_rect();
        Rect::new(
            sv_rect.right() + self.gap,
            rect.top(),
            self.hue_bar_width,
            picker_height,
        )
    }

    /// Calculate the rectangle for the alpha bar.
    fn alpha_rect(&self) -> Rect {
        let rect = self.base.rect();
        let picker_height = self.picker_height();
        let hue_rect = self.hue_rect();
        Rect::new(
            hue_rect.right() + self.gap,
            rect.top(),
            self.alpha_bar_width,
            picker_height,
        )
    }

    /// Calculate the rectangle for the hex input field.
    fn hex_input_rect(&self) -> Rect {
        let rect = self.base.rect();
        let picker_height = self.picker_height();
        Rect::new(
            rect.left(),
            rect.top() + picker_height + self.gap,
            rect.width(),
            self.hex_input_height,
        )
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    fn hit_test(&self, pos: Point) -> DragTarget {
        // Check hex input first (if shown)
        if self.show_hex_input {
            let hex_rect = self.hex_input_rect();
            if hex_rect.contains(pos) {
                return DragTarget::HexInput;
            }
        }

        let sv_rect = self.sv_rect();
        if sv_rect.contains(pos) {
            return DragTarget::SaturationValue;
        }

        let hue_rect = self.hue_rect();
        if hue_rect.contains(pos) {
            return DragTarget::Hue;
        }

        if self.show_alpha {
            let alpha_rect = self.alpha_rect();
            if alpha_rect.contains(pos) {
                return DragTarget::Alpha;
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
        let target = self.hit_test(pos);

        // Handle hex input focus
        if target == DragTarget::HexInput {
            let was_focused = self.hex_focused;
            self.hex_focused = true;
            self.drag_target = target;

            // Calculate cursor position from click
            if self.show_hex_input {
                let hex_rect = self.hex_input_rect();
                let swatch_size = hex_rect.height() - 8.0;
                let text_x = hex_rect.left() + 4.0 + swatch_size + 8.0;
                let click_offset = pos.x - text_x;

                if click_offset >= 0.0 {
                    // Approximate cursor position (rough estimate based on monospace)
                    let char_width = 8.0;
                    let char_pos = (click_offset / char_width).round() as usize;
                    self.hex_cursor_pos = char_pos.min(self.hex_text.len());
                } else {
                    self.hex_cursor_pos = 0;
                }
            }

            if !was_focused {
                self.base.update();
            }
            return true;
        }

        // Clicking elsewhere unfocuses hex input
        if self.hex_focused {
            self.hex_focused = false;
            self.base.update();
        }

        if target != DragTarget::None {
            self.drag_target = target;
            self.update_from_mouse(pos);
            true
        } else {
            false
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
            DragTarget::HexInput | DragTarget::None => return,
        }
        // Update hex text when color changes from visual picker
        if self.show_hex_input && !self.hex_focused {
            self.update_hex_text_from_color();
        }
        self.base.update();
        self.color_changed.emit(self.color());
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Handle hex input keyboard events when focused
        if self.hex_focused {
            return self.handle_hex_key_press(event);
        }

        let step = if event.modifiers.shift { 10.0 } else { 1.0 };

        match event.key {
            Key::ArrowLeft => {
                self.saturation = (self.saturation - 0.01 * step).clamp(0.0, 1.0);
                self.base.update();
                self.color_changed.emit(self.color());
                true
            }
            Key::ArrowRight => {
                self.saturation = (self.saturation + 0.01 * step).clamp(0.0, 1.0);
                self.base.update();
                self.color_changed.emit(self.color());
                true
            }
            Key::ArrowUp => {
                self.value = (self.value + 0.01 * step).clamp(0.0, 1.0);
                self.base.update();
                self.color_changed.emit(self.color());
                true
            }
            Key::ArrowDown => {
                self.value = (self.value - 0.01 * step).clamp(0.0, 1.0);
                self.base.update();
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
                    self.base.update();
                }
                true
            }
            Key::ArrowRight => {
                if self.hex_cursor_pos < self.hex_text.len() {
                    self.hex_cursor_pos += 1;
                    self.base.update();
                }
                true
            }
            Key::Home => {
                self.hex_cursor_pos = 0;
                self.base.update();
                true
            }
            Key::End => {
                self.hex_cursor_pos = self.hex_text.len();
                self.base.update();
                true
            }
            Key::Backspace => {
                if self.hex_cursor_pos > 0 {
                    self.hex_text.remove(self.hex_cursor_pos - 1);
                    self.hex_cursor_pos -= 1;
                    self.validate_and_apply_hex();
                    self.base.update();
                }
                true
            }
            Key::Delete => {
                if self.hex_cursor_pos < self.hex_text.len() {
                    self.hex_text.remove(self.hex_cursor_pos);
                    self.validate_and_apply_hex();
                    self.base.update();
                }
                true
            }
            Key::Escape => {
                // Cancel editing and restore from current color
                self.update_hex_text_from_color();
                self.hex_focused = false;
                self.base.update();
                true
            }
            Key::Enter => {
                // Apply the value and unfocus
                if self.hex_validation_state == ValidationState::Acceptable {
                    self.hex_focused = false;
                } else {
                    // Try fixup
                    if let Some(fixed) = self.hex_validator.fixup(&self.hex_text) {
                        self.hex_text = fixed;
                        self.hex_cursor_pos = self.hex_text.len();
                        self.validate_and_apply_hex();
                        self.hex_focused = false;
                    }
                }
                self.base.update();
                true
            }
            _ => {
                // Handle character input from event.text
                if !event.text.is_empty() && !event.modifiers.control && !event.modifiers.alt {
                    let mut handled = false;
                    for c in event.text.chars() {
                        // Only allow valid hex characters and # prefix
                        if c == '#' || c.is_ascii_hexdigit() {
                            let c_final = if self.hex_format.uppercase {
                                c.to_ascii_uppercase()
                            } else {
                                c.to_ascii_lowercase()
                            };
                            self.hex_text.insert(self.hex_cursor_pos, c_final);
                            self.hex_cursor_pos += 1;
                            handled = true;
                        }
                    }
                    if handled {
                        self.validate_and_apply_hex();
                        self.base.update();
                        return true;
                    }
                }
                false
            }
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_sv_square(&self, ctx: &mut PaintContext<'_>) {
        let sv_rect = self.sv_rect();

        // Paint the saturation/value gradient
        // We approximate this by drawing vertical stripes
        let steps = (sv_rect.width() / 2.0).ceil() as i32;
        let step_width = sv_rect.width() / steps as f32;

        for i in 0..steps {
            let s = i as f32 / (steps - 1) as f32;
            let x = sv_rect.left() + i as f32 * step_width;

            // Draw vertical gradient from white (top) to hue color (bottom)
            // Actually, we need: top is white to hue (saturation gradient), value is vertical
            // At s=0, v=1: white; at s=1, v=1: pure hue; at s=0, v=0: black; at s=1, v=0: black
            let v_steps = (sv_rect.height() / 2.0).ceil() as i32;
            let v_step_height = sv_rect.height() / v_steps as f32;

            for j in 0..v_steps {
                let v = 1.0 - j as f32 / (v_steps - 1).max(1) as f32;
                let y = sv_rect.top() + j as f32 * v_step_height;
                let color = Color::from_hsv(self.hue, s, v);
                ctx.renderer().fill_rect(
                    Rect::new(x, y, step_width + 0.5, v_step_height + 0.5),
                    color,
                );
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

        // Paint hue gradient (vertical, from 0 at top to 360 at bottom)
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

        // Paint alpha gradient (vertical, from 1.0 at top to 0.0 at bottom)
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

        // Draw white background with black border
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

    fn paint_hex_input(&self, ctx: &mut PaintContext<'_>) {
        let hex_rect = self.hex_input_rect();

        // Draw background
        let bg_color = if self.hex_focused {
            Color::WHITE
        } else {
            Color::from_rgb8(250, 250, 250)
        };
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(hex_rect, self.border_radius), bg_color);

        // Draw border (color depends on validation state and focus)
        let border_color = match self.hex_validation_state {
            ValidationState::Invalid => Color::from_rgb8(220, 53, 69), // Red
            ValidationState::Intermediate => Color::from_rgb8(255, 193, 7), // Yellow
            ValidationState::Acceptable => {
                if self.hex_focused {
                    Color::from_rgb8(0, 123, 255) // Blue focus
                } else {
                    self.border_color
                }
            }
        };
        let stroke = Stroke::new(border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(hex_rect, self.border_radius), &stroke);

        // Draw the color swatch preview
        let swatch_size = hex_rect.height() - 8.0;
        let swatch_rect = Rect::new(
            hex_rect.left() + 4.0,
            hex_rect.top() + 4.0,
            swatch_size,
            swatch_size,
        );

        // Draw checkerboard for alpha visibility
        self.paint_checkerboard(ctx, swatch_rect);

        // Draw the color itself
        ctx.renderer()
            .fill_rounded_rect(RoundedRect::new(swatch_rect, 2.0), self.color());

        // Draw swatch border
        let swatch_stroke = Stroke::new(Color::from_rgb8(200, 200, 200), 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(swatch_rect, 2.0), &swatch_stroke);

        // Draw the hex text
        let text_x = swatch_rect.right() + 8.0;
        let text_y = hex_rect.top() + (hex_rect.height() - self.hex_font.size()) / 2.0;
        let text_color = Color::BLACK;

        // Create text layout for the hex string
        let mut font_system = FontSystem::new();
        let layout = TextLayout::with_options(
            &mut font_system,
            &self.hex_text,
            &self.hex_font,
            TextLayoutOptions::new()
                .horizontal_align(HorizontalAlign::Left)
                .vertical_align(VerticalAlign::Top),
        );

        let text_pos = Point::new(text_x, text_y);

        // Render text using TextRenderer
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(&mut font_system, &layout, text_pos, text_color);
            // Note: Actual glyph rendering requires integration with the
            // application's render pass system.
        }

        // Draw cursor if focused
        if self.hex_focused {
            // Calculate cursor position based on text layout
            let cursor_x = text_x
                + layout.width() * (self.hex_cursor_pos as f32 / self.hex_text.len().max(1) as f32);
            let cursor_y = hex_rect.top() + 4.0;
            let cursor_height = hex_rect.height() - 8.0;

            ctx.renderer().fill_rect(
                Rect::new(cursor_x, cursor_y, 1.0, cursor_height),
                Color::BLACK,
            );
        }
    }
}

impl Default for ColorPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ColorPicker {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ColorPicker {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let alpha_width = if self.show_alpha {
            self.alpha_bar_width + self.gap
        } else {
            0.0
        };
        let width = 200.0 + self.gap + self.hue_bar_width + alpha_width;
        let hex_height = if self.show_hex_input {
            self.hex_input_height + self.gap
        } else {
            0.0
        };
        let height = 200.0 + hex_height;

        let min_hex_height = if self.show_hex_input {
            self.hex_input_height + self.gap
        } else {
            0.0
        };
        SizeHint::from_dimensions(width, height)
            .with_minimum_dimensions(150.0, 150.0 + min_hex_height)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_sv_square(ctx);
        self.paint_hue_bar(ctx);
        if self.show_alpha {
            self.paint_alpha_bar(ctx);
        }
        if self.show_hex_input {
            self.paint_hex_input(ctx);
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                if self.handle_mouse_press(e) {
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
            WidgetEvent::MouseRelease(e) => {
                if self.handle_mouse_release(e) {
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
            WidgetEvent::Leave(_) => {
                if self.drag_target != DragTarget::None {
                    self.drag_target = DragTarget::None;
                    self.base.update();
                }
            }
            _ => {}
        }
        false
    }
}

// Thread safety assertion
static_assertions::assert_impl_all!(ColorPicker: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        let _ = init_global_registry();
    }

    #[test]
    fn test_color_picker_hex_input_defaults() {
        setup();
        let picker = ColorPicker::new();
        assert!(!picker.show_hex_input());
        assert_eq!(picker.hex_text(), "#FFFFFF");
    }

    #[test]
    fn test_color_picker_with_show_hex_input() {
        setup();
        let picker = ColorPicker::new().with_show_hex_input(true);
        assert!(picker.show_hex_input());
    }

    #[test]
    fn test_color_picker_set_hex_text() {
        setup();
        let mut picker = ColorPicker::new().with_show_hex_input(true);
        picker.set_hex_text("#FF0000");

        assert_eq!(picker.hex_text(), "#FF0000");
        // Color should be updated
        let color = picker.color();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!(color.g.abs() < 0.01);
        assert!(color.b.abs() < 0.01);
    }

    #[test]
    fn test_color_picker_set_color_updates_hex() {
        setup();
        let mut picker = ColorPicker::new().with_show_hex_input(true);
        picker.set_color(Color::from_rgb8(0, 255, 0));

        assert_eq!(picker.hex_text(), "#00FF00");
    }

    #[test]
    fn test_color_picker_hex_format() {
        setup();
        let format = HexFormat::new().lowercase().with_alpha();
        let picker = ColorPicker::new()
            .with_show_hex_input(true)
            .with_hex_format(format);

        assert!(!picker.hex_format().uppercase);
        assert!(picker.hex_format().include_alpha);
        // Hex text should reflect the format
        assert!(
            picker
                .hex_text()
                .chars()
                .all(|c| !c.is_ascii_uppercase() || c == '#')
        );
    }

    #[test]
    fn test_color_picker_hex_validation_state() {
        setup();
        let mut picker = ColorPicker::new().with_show_hex_input(true);

        // Valid hex
        picker.set_hex_text("#FF0000");
        assert_eq!(picker.hex_validation_state(), ValidationState::Acceptable);
    }
}
