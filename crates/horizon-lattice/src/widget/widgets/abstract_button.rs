//! Abstract button base implementation.
//!
//! This module provides [`AbstractButton`], the base implementation for all
//! button-like widgets (PushButton, CheckBox, RadioButton, ToolButton).
//!
//! # Overview
//!
//! AbstractButton provides common button functionality:
//! - Text label
//! - Checkable/toggle behavior
//! - Mouse and keyboard interaction
//! - Standard button signals (clicked, pressed, released, toggled)
//!
//! # Event Handling
//!
//! AbstractButton handles:
//! - Mouse press/release for click detection
//! - Keyboard activation (Space/Enter when focused)
//! - Auto-repeat when held (optional)

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Icon, IconMode, IconPosition, Size, TextLayout,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MousePressEvent, MouseReleaseEvent, SizeHint,
    WidgetBase,
};

/// Common functionality for all button widgets.
///
/// This struct encapsulates the shared behavior of buttons:
/// - Text and icon management
/// - Checkable state
/// - Signal emissions
/// - Keyboard activation
///
/// Concrete button types embed this and delegate common operations.
pub struct AbstractButton {
    /// Widget base for common widget functionality.
    base: WidgetBase,

    /// The button's text label.
    text: String,

    /// Whether the button is checkable (toggle button).
    checkable: bool,

    /// Whether the button is currently checked (only meaningful if checkable).
    checked: bool,

    /// Whether auto-repeat is enabled (emit clicked while held).
    auto_repeat: bool,

    /// Auto-repeat delay in milliseconds before repeating starts.
    auto_repeat_delay: u32,

    /// Auto-repeat interval in milliseconds between repeats.
    auto_repeat_interval: u32,

    /// The font to use for text rendering.
    font: Font,

    /// Text color.
    text_color: Color,

    /// Optional icon to display.
    icon: Option<Icon>,

    /// Position of the icon relative to text.
    icon_position: IconPosition,

    /// Icon display mode (icon+text, icon only, text only).
    icon_mode: IconMode,

    /// Spacing between icon and text in pixels.
    icon_spacing: f32,

    /// Signal emitted when the button is clicked.
    ///
    /// For checkable buttons, this is emitted after the checked state changes.
    /// The bool parameter indicates whether the button was checked (for checkable)
    /// or always false (for non-checkable buttons).
    pub clicked: Signal<bool>,

    /// Signal emitted when the button is pressed down.
    pub pressed: Signal<()>,

    /// Signal emitted when the button is released.
    pub released: Signal<()>,

    /// Signal emitted when the checked state changes (for checkable buttons).
    pub toggled: Signal<bool>,
}

impl AbstractButton {
    /// Create a new abstract button with the specified text.
    pub fn new(text: impl Into<String>) -> Self {
        let mut base = WidgetBase::new::<Self>();
        // Buttons should accept focus via both Tab and click
        base.set_focus_policy(FocusPolicy::StrongFocus);

        Self {
            base,
            text: text.into(),
            checkable: false,
            checked: false,
            auto_repeat: false,
            auto_repeat_delay: 300,
            auto_repeat_interval: 100,
            font: Font::new(FontFamily::SansSerif, 14.0),
            text_color: Color::BLACK,
            icon: None,
            icon_position: IconPosition::Left,
            icon_mode: IconMode::IconAndText,
            icon_spacing: 6.0,
            clicked: Signal::new(),
            pressed: Signal::new(),
            released: Signal::new(),
            toggled: Signal::new(),
        }
    }

    // =========================================================================
    // Text
    // =========================================================================

    /// Get the button's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the button's text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let new_text = text.into();
        if self.text != new_text {
            self.text = new_text;
            self.base.update();
        }
    }

    /// Set the text using builder pattern.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    // =========================================================================
    // Checkable State
    // =========================================================================

    /// Check if the button is checkable (toggle button).
    pub fn is_checkable(&self) -> bool {
        self.checkable
    }

    /// Set whether the button is checkable.
    ///
    /// When checkable, clicking the button toggles between checked and unchecked.
    pub fn set_checkable(&mut self, checkable: bool) {
        if self.checkable != checkable {
            self.checkable = checkable;
            if !checkable && self.checked {
                self.checked = false;
                self.toggled.emit(false);
            }
            self.base.update();
        }
    }

    /// Set checkable using builder pattern.
    pub fn with_checkable(mut self, checkable: bool) -> Self {
        self.checkable = checkable;
        self
    }

    /// Check if the button is currently checked.
    pub fn is_checked(&self) -> bool {
        self.checked
    }

    /// Set the checked state.
    ///
    /// Only has effect if the button is checkable.
    pub fn set_checked(&mut self, checked: bool) {
        if self.checkable && self.checked != checked {
            self.checked = checked;
            self.toggled.emit(checked);
            self.base.update();
        }
    }

    /// Set checked state using builder pattern.
    pub fn with_checked(mut self, checked: bool) -> Self {
        if self.checkable {
            self.checked = checked;
        }
        self
    }

    /// Toggle the checked state.
    ///
    /// Only has effect if the button is checkable.
    pub fn toggle(&mut self) {
        if self.checkable {
            self.set_checked(!self.checked);
        }
    }

    // =========================================================================
    // Auto-Repeat
    // =========================================================================

    /// Check if auto-repeat is enabled.
    pub fn auto_repeat(&self) -> bool {
        self.auto_repeat
    }

    /// Set whether auto-repeat is enabled.
    ///
    /// When enabled, the clicked signal is emitted repeatedly while the button
    /// is held down.
    pub fn set_auto_repeat(&mut self, enabled: bool) {
        self.auto_repeat = enabled;
    }

    /// Set auto-repeat using builder pattern.
    pub fn with_auto_repeat(mut self, enabled: bool) -> Self {
        self.auto_repeat = enabled;
        self
    }

    /// Get the auto-repeat delay in milliseconds.
    pub fn auto_repeat_delay(&self) -> u32 {
        self.auto_repeat_delay
    }

    /// Set the auto-repeat delay.
    ///
    /// This is the time in milliseconds before auto-repeat starts.
    pub fn set_auto_repeat_delay(&mut self, delay: u32) {
        self.auto_repeat_delay = delay;
    }

    /// Get the auto-repeat interval in milliseconds.
    pub fn auto_repeat_interval(&self) -> u32 {
        self.auto_repeat_interval
    }

    /// Set the auto-repeat interval.
    ///
    /// This is the time in milliseconds between auto-repeat clicks.
    pub fn set_auto_repeat_interval(&mut self, interval: u32) {
        self.auto_repeat_interval = interval;
    }

    // =========================================================================
    // Font and Text Color
    // =========================================================================

    /// Get the font.
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// Set the font for text rendering.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
        self.base.update();
    }

    /// Set font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
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

    // =========================================================================
    // Icon
    // =========================================================================

    /// Get the button's icon, if any.
    pub fn icon(&self) -> Option<&Icon> {
        self.icon.as_ref()
    }

    /// Set the button's icon.
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.icon = icon;
        self.base.update();
    }

    /// Set the icon using builder pattern.
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Get the icon position.
    pub fn icon_position(&self) -> IconPosition {
        self.icon_position
    }

    /// Set the position of the icon relative to text.
    pub fn set_icon_position(&mut self, position: IconPosition) {
        if self.icon_position != position {
            self.icon_position = position;
            self.base.update();
        }
    }

    /// Set icon position using builder pattern.
    pub fn with_icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// Get the icon display mode.
    pub fn icon_mode(&self) -> IconMode {
        self.icon_mode
    }

    /// Set the icon display mode.
    pub fn set_icon_mode(&mut self, mode: IconMode) {
        if self.icon_mode != mode {
            self.icon_mode = mode;
            self.base.update();
        }
    }

    /// Set icon mode using builder pattern.
    pub fn with_icon_mode(mut self, mode: IconMode) -> Self {
        self.icon_mode = mode;
        self
    }

    /// Get the spacing between icon and text.
    pub fn icon_spacing(&self) -> f32 {
        self.icon_spacing
    }

    /// Set the spacing between icon and text in pixels.
    pub fn set_icon_spacing(&mut self, spacing: f32) {
        if (self.icon_spacing - spacing).abs() > f32::EPSILON {
            self.icon_spacing = spacing;
            self.base.update();
        }
    }

    /// Set icon spacing using builder pattern.
    pub fn with_icon_spacing(mut self, spacing: f32) -> Self {
        self.icon_spacing = spacing;
        self
    }

    /// Check if this button should show an icon.
    pub fn shows_icon(&self) -> bool {
        self.icon.is_some() && self.icon_mode != IconMode::TextOnly
    }

    /// Check if this button should show text.
    pub fn shows_text(&self) -> bool {
        !self.text.is_empty() && self.icon_mode != IconMode::IconOnly
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    /// Handle a mouse press event.
    ///
    /// Returns `true` if the event was handled.
    pub fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if !self.base.is_effectively_enabled() {
            return false;
        }

        self.pressed.emit(());
        true
    }

    /// Handle a mouse release event.
    ///
    /// Returns `true` if the event was handled and a click occurred.
    pub fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if !self.base.is_effectively_enabled() {
            return false;
        }

        // Only click if we're still over the button
        let is_over = self.base.contains_point(event.local_pos);

        self.released.emit(());

        if is_over && self.base.is_pressed() {
            self.click();
            return true;
        }

        false
    }

    /// Handle a key press event.
    ///
    /// Returns `true` if the event was handled.
    pub fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        if !self.base.is_effectively_enabled() {
            return false;
        }

        // Space or Enter activates the button
        match event.key {
            Key::Space | Key::Enter => {
                if !event.is_repeat {
                    self.pressed.emit(());
                }
                true
            }
            _ => false,
        }
    }

    /// Handle a key release event.
    ///
    /// Returns `true` if the event was handled.
    pub fn handle_key_release(&mut self, key: Key) -> bool {
        if !self.base.is_effectively_enabled() {
            return false;
        }

        match key {
            Key::Space | Key::Enter => {
                self.released.emit(());
                self.click();
                true
            }
            _ => false,
        }
    }

    /// Programmatically click the button.
    ///
    /// This toggles the checked state (if checkable) and emits the clicked signal.
    pub fn click(&mut self) {
        if !self.base.is_effectively_enabled() {
            return;
        }

        if self.checkable {
            self.checked = !self.checked;
            self.toggled.emit(self.checked);
        }

        self.clicked.emit(self.checked);
        self.base.update();
    }

    // =========================================================================
    // Rendering Helpers
    // =========================================================================

    /// Calculate the size needed for the button text.
    pub fn text_size(&self) -> Size {
        if self.text.is_empty() || !self.shows_text() {
            return Size::new(0.0, self.font.size());
        }

        let mut font_system = FontSystem::new();
        let layout = TextLayout::new(&mut font_system, &self.text, &self.font);
        Size::new(layout.width(), layout.height())
    }

    /// Get the size of the icon for display.
    pub fn icon_size(&self) -> Size {
        if !self.shows_icon() {
            return Size::ZERO;
        }
        self.icon
            .as_ref()
            .map(|i| i.display_size())
            .unwrap_or(Size::ZERO)
    }

    /// Calculate the combined content size (icon + text + spacing).
    pub fn content_size(&self) -> Size {
        let text_size = self.text_size();
        let icon_size = self.icon_size();
        let shows_icon = self.shows_icon();
        let shows_text = self.shows_text();

        // Calculate total content size based on layout direction
        if shows_icon && shows_text {
            let spacing = self.icon_spacing;
            if self.icon_position.is_horizontal() {
                // Icon and text side by side
                Size::new(
                    icon_size.width + spacing + text_size.width,
                    icon_size.height.max(text_size.height),
                )
            } else {
                // Icon and text stacked
                Size::new(
                    icon_size.width.max(text_size.width),
                    icon_size.height + spacing + text_size.height,
                )
            }
        } else if shows_icon {
            icon_size
        } else {
            text_size
        }
    }

    /// Get the default size hint for the button.
    pub fn default_size_hint(&self) -> SizeHint {
        let content_size = self.content_size();
        // Add padding around the content
        let padding = 16.0; // 8px on each side
        let min_width = 64.0;
        let min_height = 24.0;

        let preferred = Size::new(
            (content_size.width + padding * 2.0).max(min_width),
            (content_size.height + padding).max(min_height),
        );

        SizeHint::new(preferred)
            .with_minimum_dimensions(min_width, min_height)
    }

    /// Get the color for the button background based on current state.
    pub fn background_color(&self, base_color: Color) -> Color {
        if !self.base.is_effectively_enabled() {
            // Disabled: muted gray
            Color::from_rgb8(200, 200, 200)
        } else if self.base.is_pressed() {
            // Pressed: darker
            darken_color(base_color, 0.2)
        } else if self.base.is_hovered() {
            // Hovered: lighter
            lighten_color(base_color, 0.1)
        } else if self.checked {
            // Checked: slightly darker to indicate active state
            darken_color(base_color, 0.1)
        } else {
            base_color
        }
    }

    /// Get the text color based on current state.
    pub fn effective_text_color(&self) -> Color {
        if !self.base.is_effectively_enabled() {
            Color::from_rgb8(128, 128, 128)
        } else {
            self.text_color
        }
    }

    // =========================================================================
    // WidgetBase Access
    // =========================================================================

    /// Get a reference to the widget base.
    pub fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    /// Get a mutable reference to the widget base.
    pub fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
}

impl Object for AbstractButton {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

// =========================================================================
// Color Helpers
// =========================================================================

/// Darken a color by a factor (0.0 = no change, 1.0 = black).
fn darken_color(color: Color, factor: f32) -> Color {
    let factor = 1.0 - factor.clamp(0.0, 1.0);
    Color::new(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a,
    )
}

/// Lighten a color by a factor (0.0 = no change, 1.0 = white).
fn lighten_color(color: Color, factor: f32) -> Color {
    let factor = factor.clamp(0.0, 1.0);
    Color::new(
        color.r + (color.a - color.r) * factor,
        color.g + (color.a - color.g) * factor,
        color.b + (color.a - color.b) * factor,
        color.a,
    )
}

// Ensure AbstractButton is Send + Sync
static_assertions::assert_impl_all!(AbstractButton: Send, Sync);
