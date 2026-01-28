//! Push button widget implementation.
//!
//! This module provides [`PushButton`], the standard clickable button widget.
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice::widget::widgets::PushButton;
//!
//! // Create a simple button
//! let button = PushButton::new("Click me!");
//!
//! // Connect to the clicked signal
//! button.clicked().connect(|&checked| {
//!     println!("Button clicked! Checked: {}", checked);
//! });
//!
//! // Create a toggle button
//! let toggle = PushButton::new("Toggle")
//!     .with_checkable(true);
//!
//! toggle.toggled().connect(|&checked| {
//!     println!("Toggled: {}", checked);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    icon_tint_for_state, Color, Font, FontSystem, Icon, IconMode, IconPosition, ImageScaleMode,
    Point, Rect, Renderer, RoundedRect, Stroke, TextLayout, TextRenderer,
};

use super::abstract_button::{AbstractButton, ButtonVariant};
use crate::widget::{PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent};

/// A standard push button widget.
///
/// PushButton is the most common button type, used for triggering actions.
/// It supports:
/// - Text labels
/// - Click handling with visual feedback
/// - Checkable/toggle mode
/// - Keyboard activation (Space/Enter when focused)
///
/// # Visual States
///
/// The button renders differently based on its state:
/// - **Normal**: Default appearance
/// - **Hovered**: Slightly lighter when mouse is over
/// - **Pressed**: Darker when clicked/pressed
/// - **Disabled**: Grayed out when not enabled
/// - **Checked**: Distinct appearance when in toggle mode and checked
///
/// # Signals
///
/// - `clicked`: Emitted when the button is clicked
/// - `pressed`: Emitted when mouse button is pressed down
/// - `released`: Emitted when mouse button is released
/// - `toggled`: Emitted when checked state changes (checkable buttons only)
pub struct PushButton {
    /// The underlying abstract button implementation.
    inner: AbstractButton,

    /// Border radius for rounded corners.
    border_radius: f32,
}

impl PushButton {
    /// Create a new push button with the specified text.
    ///
    /// By default, buttons use [`ButtonVariant::Primary`].
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            inner: AbstractButton::new(text),
            border_radius: 4.0,
        }
    }

    // =========================================================================
    // Delegated Text Methods
    // =========================================================================

    /// Get the button's text.
    pub fn text(&self) -> &str {
        self.inner.text()
    }

    /// Set the button's text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.inner.set_text(text);
    }

    /// Set the text using builder pattern.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.inner = self.inner.with_text(text);
        self
    }

    // =========================================================================
    // Delegated Checkable Methods
    // =========================================================================

    /// Check if the button is checkable (toggle button).
    pub fn is_checkable(&self) -> bool {
        self.inner.is_checkable()
    }

    /// Set whether the button is checkable.
    pub fn set_checkable(&mut self, checkable: bool) {
        self.inner.set_checkable(checkable);
    }

    /// Set checkable using builder pattern.
    pub fn with_checkable(mut self, checkable: bool) -> Self {
        self.inner = self.inner.with_checkable(checkable);
        self
    }

    /// Check if the button is currently checked.
    pub fn is_checked(&self) -> bool {
        self.inner.is_checked()
    }

    /// Set the checked state.
    pub fn set_checked(&mut self, checked: bool) {
        self.inner.set_checked(checked);
    }

    /// Set checked state using builder pattern.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.inner = self.inner.with_checked(checked);
        self
    }

    /// Toggle the checked state.
    pub fn toggle(&mut self) {
        self.inner.toggle();
    }

    // =========================================================================
    // Delegated Auto-Repeat Methods
    // =========================================================================

    /// Check if auto-repeat is enabled.
    pub fn auto_repeat(&self) -> bool {
        self.inner.auto_repeat()
    }

    /// Set whether auto-repeat is enabled.
    pub fn set_auto_repeat(&mut self, enabled: bool) {
        self.inner.set_auto_repeat(enabled);
    }

    /// Set auto-repeat using builder pattern.
    pub fn with_auto_repeat(mut self, enabled: bool) -> Self {
        self.inner = self.inner.with_auto_repeat(enabled);
        self
    }

    // =========================================================================
    // Delegated Font Methods
    // =========================================================================

    /// Get the font.
    pub fn font(&self) -> &Font {
        self.inner.font()
    }

    /// Set the font for text rendering.
    pub fn set_font(&mut self, font: Font) {
        self.inner.set_font(font);
    }

    /// Set font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.inner = self.inner.with_font(font);
        self
    }

    /// Get the explicit text color override, if set.
    pub fn text_color(&self) -> Option<Color> {
        self.inner.text_color()
    }

    /// Set an explicit text color override.
    ///
    /// If set, this color will be used instead of the variant's default text color.
    pub fn set_text_color(&mut self, color: Color) {
        self.inner.set_text_color(Some(color));
    }

    /// Clear the text color override, using the variant's default.
    pub fn clear_text_color(&mut self) {
        self.inner.set_text_color(None);
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.inner = self.inner.with_text_color(color);
        self
    }

    // =========================================================================
    // Delegated Icon Methods
    // =========================================================================

    /// Get the button's icon, if any.
    pub fn icon(&self) -> Option<&Icon> {
        self.inner.icon()
    }

    /// Set the button's icon.
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.inner.set_icon(icon);
    }

    /// Set the icon using builder pattern.
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.inner = self.inner.with_icon(icon);
        self
    }

    /// Get the icon position.
    pub fn icon_position(&self) -> IconPosition {
        self.inner.icon_position()
    }

    /// Set the position of the icon relative to text.
    pub fn set_icon_position(&mut self, position: IconPosition) {
        self.inner.set_icon_position(position);
    }

    /// Set icon position using builder pattern.
    pub fn with_icon_position(mut self, position: IconPosition) -> Self {
        self.inner = self.inner.with_icon_position(position);
        self
    }

    /// Get the icon display mode.
    pub fn icon_mode(&self) -> IconMode {
        self.inner.icon_mode()
    }

    /// Set the icon display mode.
    pub fn set_icon_mode(&mut self, mode: IconMode) {
        self.inner.set_icon_mode(mode);
    }

    /// Set icon mode using builder pattern.
    pub fn with_icon_mode(mut self, mode: IconMode) -> Self {
        self.inner = self.inner.with_icon_mode(mode);
        self
    }

    /// Get the spacing between icon and text.
    pub fn icon_spacing(&self) -> f32 {
        self.inner.icon_spacing()
    }

    /// Set the spacing between icon and text in pixels.
    pub fn set_icon_spacing(&mut self, spacing: f32) {
        self.inner.set_icon_spacing(spacing);
    }

    /// Set icon spacing using builder pattern.
    pub fn with_icon_spacing(mut self, spacing: f32) -> Self {
        self.inner = self.inner.with_icon_spacing(spacing);
        self
    }

    // =========================================================================
    // Variant Methods
    // =========================================================================

    /// Get the button's visual variant.
    pub fn variant(&self) -> ButtonVariant {
        self.inner.variant()
    }

    /// Set the button's visual variant.
    ///
    /// The variant determines the button's colors and visual style:
    /// - [`ButtonVariant::Primary`]: Filled with primary color (default)
    /// - [`ButtonVariant::Secondary`]: Outlined with primary color border
    /// - [`ButtonVariant::Danger`]: Filled with error/red color
    /// - [`ButtonVariant::Flat`]: Text only, no background
    /// - [`ButtonVariant::Outlined`]: Outlined with neutral border
    pub fn set_variant(&mut self, variant: ButtonVariant) {
        self.inner.set_variant(variant);
    }

    /// Set variant using builder pattern.
    pub fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.inner = self.inner.with_variant(variant);
        self
    }

    // =========================================================================
    // PushButton-Specific Methods
    // =========================================================================

    /// Get the border radius.
    pub fn border_radius(&self) -> f32 {
        self.border_radius
    }

    /// Set the border radius for rounded corners.
    pub fn set_border_radius(&mut self, radius: f32) {
        if self.border_radius != radius {
            self.border_radius = radius;
            self.inner.widget_base_mut().update();
        }
    }

    /// Set border radius using builder pattern.
    pub fn with_border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    // =========================================================================
    // Default Button
    // =========================================================================

    /// Check if this button is the default button.
    ///
    /// The default button is activated when Enter is pressed in a window/dialog,
    /// even if the button doesn't have keyboard focus. Default buttons have
    /// enhanced visual styling to indicate their special status.
    pub fn is_default(&self) -> bool {
        self.inner.is_default()
    }

    /// Set whether this button is the default button.
    ///
    /// Only one button in a window should typically be marked as default.
    /// Setting this to `true` enables:
    /// - Enhanced visual styling (prominent border ring)
    /// - Activation via Enter key at the window level
    ///
    /// # Example
    ///
    /// ```no_run
    /// use horizon_lattice::widget::widgets::PushButton;
    ///
    /// let ok_button = PushButton::new("OK")
    ///     .with_default(true);
    /// ```
    pub fn set_default(&mut self, is_default: bool) {
        self.inner.set_default(is_default);
    }

    /// Set default using builder pattern.
    pub fn with_default(mut self, is_default: bool) -> Self {
        self.inner = self.inner.with_default(is_default);
        self
    }

    // =========================================================================
    // Auto-Default Button
    // =========================================================================

    /// Check if this button has the auto-default property.
    ///
    /// An auto-default button becomes the default button when it receives
    /// keyboard focus via Tab navigation. When focus moves away, the original
    /// default button (if any) is restored.
    ///
    /// This is useful in dialogs where multiple buttons could be considered
    /// "default" depending on what the user is focused on.
    pub fn is_auto_default(&self) -> bool {
        self.inner.is_auto_default()
    }

    /// Set whether this button has the auto-default property.
    ///
    /// When `true`:
    /// - The button becomes the default button when focused via Tab
    /// - The original default is saved and restored when focus leaves
    /// - The button gets visual default styling when focused
    ///
    /// In dialogs, buttons are typically auto-default by default.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use horizon_lattice::widget::widgets::PushButton;
    ///
    /// let button = PushButton::new("OK")
    ///     .with_auto_default(true);
    /// ```
    pub fn set_auto_default(&mut self, auto_default: bool) {
        self.inner.set_auto_default(auto_default);
    }

    /// Set auto-default using builder pattern.
    pub fn with_auto_default(mut self, auto_default: bool) -> Self {
        self.inner = self.inner.with_auto_default(auto_default);
        self
    }

    /// Programmatically click the button.
    pub fn click(&mut self) {
        self.inner.click();
    }

    // =========================================================================
    // Keyboard Shortcut
    // =========================================================================

    /// Get the button's keyboard shortcut, if any.
    pub fn shortcut(&self) -> Option<&crate::widget::KeySequence> {
        self.inner.shortcut()
    }

    /// Set the button's keyboard shortcut.
    pub fn set_shortcut(&mut self, shortcut: Option<crate::widget::KeySequence>) {
        self.inner.set_shortcut(shortcut);
    }

    /// Set shortcut using builder pattern.
    pub fn with_shortcut(mut self, shortcut: crate::widget::KeySequence) -> Self {
        self.inner = self.inner.with_shortcut(shortcut);
        self
    }

    /// Set shortcut from a string using builder pattern.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use horizon_lattice::widget::widgets::PushButton;
    ///
    /// let button = PushButton::new("&Save")
    ///     .with_shortcut_str("Ctrl+S");
    /// ```
    pub fn with_shortcut_str(mut self, shortcut: &str) -> Self {
        self.inner = self.inner.with_shortcut_str(shortcut);
        self
    }

    /// Check if this button's shortcut matches the given key combination.
    pub fn matches_shortcut(
        &self,
        key: crate::widget::Key,
        modifiers: crate::widget::KeyboardModifiers,
    ) -> bool {
        self.inner.matches_shortcut(key, modifiers)
    }

    // =========================================================================
    // Mnemonic Support
    // =========================================================================

    /// Get the display text for the button (with mnemonic markers processed).
    pub fn display_text(&self) -> &str {
        self.inner.display_text()
    }

    /// Get the mnemonic character for this button, if any.
    pub fn mnemonic(&self) -> Option<char> {
        self.inner.mnemonic()
    }

    /// Get the index of the mnemonic character in the display text.
    pub fn mnemonic_index(&self) -> Option<usize> {
        self.inner.mnemonic_index()
    }

    /// Check if this button's mnemonic matches the given character.
    pub fn matches_mnemonic(&self, ch: char) -> bool {
        self.inner.matches_mnemonic(ch)
    }

    // =========================================================================
    // Private Rendering Helpers
    // =========================================================================

    /// Get background and border colors based on variant and state.
    ///
    /// Returns (background_color, Option<border_color>).
    fn variant_colors(
        &self,
        is_disabled: bool,
        is_pressed: bool,
        is_hovered: bool,
        is_checked: bool,
    ) -> (Color, Option<Color>) {
        // Palette colors (from light theme)
        // Primary blue
        let primary = Color::from_rgb8(0, 122, 255);
        let primary_light = Color::from_rgb8(77, 163, 255);
        let primary_dark = Color::from_rgb8(0, 86, 179);
        // Error red (for danger)
        let error = Color::from_rgb8(220, 53, 69);
        let error_light = Color::from_rgb8(235, 100, 113);
        let error_dark = Color::from_rgb8(176, 42, 55);
        // Neutral colors
        let border = Color::from_rgb8(222, 226, 230);
        let disabled_bg = Color::from_rgb8(200, 200, 200);
        let transparent = Color::from_rgba8(0, 0, 0, 0);

        // Disabled state overrides variant
        if is_disabled {
            return match self.inner.variant() {
                ButtonVariant::Primary | ButtonVariant::Danger => (disabled_bg, None),
                ButtonVariant::Secondary | ButtonVariant::Outlined => {
                    (transparent, Some(disabled_bg))
                }
                ButtonVariant::Flat => (transparent, None),
            };
        }

        match self.inner.variant() {
            ButtonVariant::Primary => {
                let bg = if is_pressed {
                    primary_dark
                } else if is_hovered {
                    primary_light
                } else if is_checked {
                    primary_dark
                } else {
                    primary
                };
                (bg, None)
            }

            ButtonVariant::Secondary => {
                let bg = if is_pressed {
                    Color::from_rgba8(0, 122, 255, 51) // 20% primary
                } else if is_hovered {
                    Color::from_rgba8(0, 122, 255, 26) // 10% primary
                } else if is_checked {
                    Color::from_rgba8(0, 122, 255, 38) // 15% primary
                } else {
                    transparent
                };
                (bg, Some(primary))
            }

            ButtonVariant::Danger => {
                let bg = if is_pressed {
                    error_dark
                } else if is_hovered {
                    error_light
                } else if is_checked {
                    error_dark
                } else {
                    error
                };
                (bg, None)
            }

            ButtonVariant::Flat => {
                let bg = if is_pressed {
                    Color::from_rgba8(0, 122, 255, 38) // 15% primary
                } else if is_hovered {
                    Color::from_rgba8(0, 122, 255, 20) // 8% primary
                } else if is_checked {
                    Color::from_rgba8(0, 122, 255, 26) // 10% primary
                } else {
                    transparent
                };
                (bg, None)
            }

            ButtonVariant::Outlined => {
                let bg = if is_pressed {
                    Color::from_rgba8(0, 0, 0, 26) // 10% black
                } else if is_hovered {
                    Color::from_rgba8(0, 0, 0, 13) // 5% black
                } else if is_checked {
                    Color::from_rgba8(0, 0, 0, 20) // 8% black
                } else {
                    transparent
                };
                (bg, Some(border))
            }
        }
    }

    // =========================================================================
    // Signal Access
    // =========================================================================

    /// Get the clicked signal.
    ///
    /// Emitted when the button is clicked. The bool parameter indicates
    /// whether the button is checked (for checkable buttons).
    pub fn clicked(&self) -> &Signal<bool> {
        &self.inner.clicked
    }

    /// Get the pressed signal.
    ///
    /// Emitted when the button is pressed down.
    pub fn pressed(&self) -> &Signal<()> {
        &self.inner.pressed
    }

    /// Get the released signal.
    ///
    /// Emitted when the button is released.
    pub fn released(&self) -> &Signal<()> {
        &self.inner.released
    }

    /// Get the toggled signal.
    ///
    /// Emitted when the checked state changes (checkable buttons only).
    pub fn toggled(&self) -> &Signal<bool> {
        &self.inner.toggled
    }
}

impl Object for PushButton {
    fn object_id(&self) -> ObjectId {
        self.inner.widget_base().object_id()
    }
}

impl Widget for PushButton {
    fn widget_base(&self) -> &WidgetBase {
        self.inner.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.inner.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.default_size_hint()
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Get state info
        let is_disabled = !self.inner.widget_base().is_effectively_enabled();
        let is_pressed = self.inner.widget_base().is_pressed();
        let is_hovered = self.inner.widget_base().is_hovered();
        let is_checked = self.inner.is_checked();

        // Get variant-specific colors
        let (bg_color, border_color) =
            self.variant_colors(is_disabled, is_pressed, is_hovered, is_checked);

        // Draw rounded rectangle background and/or border based on variant
        let rrect = RoundedRect::new(rect, self.border_radius);

        // Fill background if not transparent
        if bg_color.a > 0.0 {
            ctx.renderer().fill_rounded_rect(rrect, bg_color);
        }

        // Draw border if color is specified (non-transparent)
        if let Some(border) = border_color {
            let stroke = Stroke::new(border, 1.0);
            ctx.renderer().stroke_rounded_rect(rrect, &stroke);
        }

        // Calculate content sizes
        let shows_icon = self.inner.shows_icon();
        let shows_text = self.inner.shows_text();
        let icon_size = self.inner.icon_size();
        let content_size = self.inner.content_size();
        let spacing = self.inner.icon_spacing();

        // Center the content in the button
        let content_x = rect.origin.x + (rect.width() - content_size.width) / 2.0;
        let content_y = rect.origin.y + (rect.height() - content_size.height) / 2.0;

        // Calculate icon and text positions based on icon position
        let (icon_pos, text_offset) = if shows_icon && shows_text {
            match self.inner.icon_position() {
                IconPosition::Left => {
                    let icon_y = content_y + (content_size.height - icon_size.height) / 2.0;
                    (
                        Point::new(content_x, icon_y),
                        Point::new(icon_size.width + spacing, 0.0),
                    )
                }
                IconPosition::Right => {
                    let text_size = self.inner.text_size();
                    let icon_y = content_y + (content_size.height - icon_size.height) / 2.0;
                    (
                        Point::new(content_x + text_size.width + spacing, icon_y),
                        Point::new(0.0, 0.0),
                    )
                }
                IconPosition::Top => {
                    let icon_x = content_x + (content_size.width - icon_size.width) / 2.0;
                    (
                        Point::new(icon_x, content_y),
                        Point::new(0.0, icon_size.height + spacing),
                    )
                }
                IconPosition::Bottom => {
                    let text_size = self.inner.text_size();
                    let icon_x = content_x + (content_size.width - icon_size.width) / 2.0;
                    (
                        Point::new(icon_x, content_y + text_size.height + spacing),
                        Point::new(0.0, 0.0),
                    )
                }
            }
        } else if shows_icon {
            // Icon only - center it
            let icon_x = content_x + (content_size.width - icon_size.width) / 2.0;
            let icon_y = content_y + (content_size.height - icon_size.height) / 2.0;
            (Point::new(icon_x, icon_y), Point::new(0.0, 0.0))
        } else {
            // Text only - no icon position needed
            (Point::new(0.0, 0.0), Point::new(0.0, 0.0))
        };

        // Draw icon if present and loaded
        if shows_icon {
            if let Some(icon) = self.inner.icon() {
                // Get the appropriate image based on state
                let image = if is_disabled {
                    icon.disabled_image()
                } else {
                    icon.image()
                };

                if let Some(img) = image {
                    let icon_rect = Rect::new(
                        icon_pos.x,
                        icon_pos.y,
                        icon_size.width,
                        icon_size.height,
                    );

                    // Apply tint for state feedback (only if not using dedicated disabled image)
                    let _tint = icon_tint_for_state(
                        Color::WHITE,
                        is_disabled && icon.disabled_image().is_none(),
                        is_pressed,
                        is_hovered,
                    );

                    // Draw the icon image
                    ctx.renderer().draw_image(img, icon_rect, ImageScaleMode::Fit);
                }
            }
        }

        // Draw text if present
        if shows_text && !self.inner.display_text().is_empty() {
            let mut font_system = FontSystem::new();
            let display_text = self.inner.display_text();
            let layout = TextLayout::new(&mut font_system, display_text, self.inner.font());

            // Calculate text position (centered within text area, offset by icon if present)
            let text_area_x = if shows_icon {
                content_x + text_offset.x
            } else {
                rect.origin.x + (rect.width() - layout.width()) / 2.0
            };
            let text_area_y = if shows_icon {
                content_y + text_offset.y + (content_size.height - text_offset.y - layout.height()) / 2.0
            } else {
                rect.origin.y + (rect.height() - layout.height()) / 2.0
            };

            let text_pos = Point::new(text_area_x, text_area_y);
            let text_color = self.inner.effective_text_color();

            // Render text
            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    text_pos,
                    text_color,
                );
            }

            // Draw mnemonic underline if there's a mnemonic character
            if let Some(mnemonic_idx) = self.inner.mnemonic_index() {
                // Calculate underline position by measuring text segments
                let text_before = &display_text[..mnemonic_idx];
                let mnemonic_char = display_text
                    .chars()
                    .nth(text_before.chars().count())
                    .map(|c| c.to_string())
                    .unwrap_or_default();

                // Measure width of text before mnemonic
                let before_width = if text_before.is_empty() {
                    0.0
                } else {
                    let before_layout =
                        TextLayout::new(&mut font_system, text_before, self.inner.font());
                    before_layout.width()
                };

                // Measure width of mnemonic character
                let mnemonic_width = if mnemonic_char.is_empty() {
                    0.0
                } else {
                    let mnemonic_layout =
                        TextLayout::new(&mut font_system, &mnemonic_char, self.inner.font());
                    mnemonic_layout.width()
                };

                // Draw underline beneath the mnemonic character
                if mnemonic_width > 0.0 {
                    let underline_y = text_pos.y + layout.height() + 1.0;
                    let underline_start = Point::new(text_pos.x + before_width, underline_y);
                    let underline_end =
                        Point::new(text_pos.x + before_width + mnemonic_width, underline_y);
                    let stroke = Stroke::new(text_color, 1.0);
                    ctx.renderer().draw_line(underline_start, underline_end, &stroke);
                }
            }
        }

        // Draw default button indicator (prominent ring)
        if self.inner.is_default() {
            let default_rect = RoundedRect::new(
                rect.inflate(2.0),
                self.border_radius + 2.0,
            );
            // Primary blue color for default button ring
            let default_color = Color::from_rgb8(0, 122, 255);
            let stroke = Stroke::new(default_color, 2.0);
            ctx.renderer().stroke_rounded_rect(default_rect, &stroke);
        }

        // Draw focus indicator when focused (stacks on top of default indicator)
        if self.widget_base().has_focus() {
            let focus_rect = RoundedRect::new(
                rect.inflate(4.0),
                self.border_radius + 4.0,
            );
            let focus_color = Color::from_rgba8(66, 133, 244, 128);
            ctx.renderer().fill_rounded_rect(focus_rect, focus_color);
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                if self.inner.handle_mouse_press(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if self.inner.handle_mouse_release(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::KeyPress(e) => {
                if self.inner.handle_key_press(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::KeyRelease(e) => {
                if self.inner.handle_key_release(e.key) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

// Ensure PushButton is Send + Sync
static_assertions::assert_impl_all!(PushButton: Send, Sync);

// =========================================================================
// Accessibility
// =========================================================================

#[cfg(feature = "accessibility")]
impl crate::widget::accessibility::Accessible for PushButton {
    fn accessible_role(&self) -> crate::widget::accessibility::AccessibleRole {
        crate::widget::accessibility::AccessibleRole::Button
    }

    fn accessible_name(&self) -> Option<String> {
        // Use custom accessible name if set, otherwise use the button text
        self.widget_base()
            .accessible_name()
            .map(String::from)
            .or_else(|| {
                let text = self.text();
                if text.is_empty() {
                    None
                } else {
                    Some(text.to_string())
                }
            })
    }

    fn accessible_description(&self) -> Option<String> {
        self.widget_base().accessible_description().map(String::from)
    }

    fn is_accessible_checked(&self) -> Option<bool> {
        // Only return checked state if the button is checkable
        if self.is_checkable() {
            Some(self.is_checked())
        } else {
            None
        }
    }

    fn accessible_actions(&self) -> Vec<accesskit::Action> {
        let mut actions = vec![accesskit::Action::Focus];
        if self.widget_base().is_effectively_enabled() {
            actions.push(accesskit::Action::Click);
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        Arc,
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_push_button_creation() {
        setup();
        let button = PushButton::new("Test Button");
        assert_eq!(button.text(), "Test Button");
        assert!(!button.is_checkable());
        assert!(!button.is_checked());
    }

    #[test]
    fn test_push_button_builder_pattern() {
        setup();
        let button = PushButton::new("Test")
            .with_checkable(true)
            .with_checked(true)
            .with_border_radius(8.0)
            .with_text_color(Color::WHITE);

        assert!(button.is_checkable());
        assert!(button.is_checked());
        assert_eq!(button.border_radius(), 8.0);
        assert_eq!(button.text_color(), Some(Color::WHITE));
    }

    #[test]
    fn test_push_button_click_signal() {
        setup();
        let mut button = PushButton::new("Test");
        let clicked = Arc::new(AtomicBool::new(false));
        let clicked_clone = clicked.clone();

        button.inner.clicked.connect(move |_| {
            clicked_clone.store(true, Ordering::SeqCst);
        });

        button.click();
        assert!(clicked.load(Ordering::SeqCst));
    }

    #[test]
    fn test_push_button_toggle() {
        setup();
        let mut button = PushButton::new("Toggle").with_checkable(true);

        let toggle_count = Arc::new(AtomicU32::new(0));
        let toggle_count_clone = toggle_count.clone();

        button.inner.toggled.connect(move |_| {
            toggle_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        assert!(!button.is_checked());
        button.toggle();
        assert!(button.is_checked());
        button.toggle();
        assert!(!button.is_checked());

        assert_eq!(toggle_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_push_button_size_hint() {
        setup();
        let button = PushButton::new("Test");
        let hint = button.size_hint();

        // Should have reasonable preferred size
        assert!(hint.preferred.width >= 64.0);
        assert!(hint.preferred.height >= 24.0);
    }

    #[test]
    fn test_non_checkable_toggle_no_effect() {
        setup();
        let mut button = PushButton::new("Not Checkable");
        assert!(!button.is_checkable());
        assert!(!button.is_checked());

        button.toggle();
        assert!(!button.is_checked()); // No effect since not checkable
    }

    #[test]
    fn test_icon_position_default() {
        setup();
        let button = PushButton::new("Test");
        assert_eq!(button.icon_position(), IconPosition::Left);
    }

    #[test]
    fn test_icon_position_builder() {
        setup();
        let button = PushButton::new("Test")
            .with_icon_position(IconPosition::Right);
        assert_eq!(button.icon_position(), IconPosition::Right);
    }

    #[test]
    fn test_icon_mode_default() {
        setup();
        let button = PushButton::new("Test");
        assert_eq!(button.icon_mode(), IconMode::IconAndText);
    }

    #[test]
    fn test_icon_mode_builder() {
        setup();
        let button = PushButton::new("Test")
            .with_icon_mode(IconMode::IconOnly);
        assert_eq!(button.icon_mode(), IconMode::IconOnly);
    }

    #[test]
    fn test_icon_spacing_default() {
        setup();
        let button = PushButton::new("Test");
        assert_eq!(button.icon_spacing(), 6.0);
    }

    #[test]
    fn test_icon_spacing_builder() {
        setup();
        let button = PushButton::new("Test")
            .with_icon_spacing(12.0);
        assert_eq!(button.icon_spacing(), 12.0);
    }

    #[test]
    fn test_no_icon_by_default() {
        setup();
        let button = PushButton::new("Test");
        assert!(button.icon().is_none());
    }

    #[test]
    fn test_icon_from_path() {
        setup();
        let icon = Icon::from_path("test/icon.png");
        let button = PushButton::new("Test").with_icon(icon);
        assert!(button.icon().is_some());
    }

    // =========================================================================
    // Button Variant Tests
    // =========================================================================

    #[test]
    fn test_default_variant_is_primary() {
        setup();
        let button = PushButton::new("Test");
        assert_eq!(button.variant(), ButtonVariant::Primary);
    }

    #[test]
    fn test_variant_builder() {
        setup();
        let button = PushButton::new("Delete")
            .with_variant(ButtonVariant::Danger);
        assert_eq!(button.variant(), ButtonVariant::Danger);
    }

    #[test]
    fn test_set_variant() {
        setup();
        let mut button = PushButton::new("Action");
        assert_eq!(button.variant(), ButtonVariant::Primary);

        button.set_variant(ButtonVariant::Secondary);
        assert_eq!(button.variant(), ButtonVariant::Secondary);

        button.set_variant(ButtonVariant::Flat);
        assert_eq!(button.variant(), ButtonVariant::Flat);

        button.set_variant(ButtonVariant::Outlined);
        assert_eq!(button.variant(), ButtonVariant::Outlined);
    }

    #[test]
    fn test_all_variants_accessible() {
        setup();
        // Ensure all variants can be set via builder
        let _primary = PushButton::new("Primary").with_variant(ButtonVariant::Primary);
        let _secondary = PushButton::new("Secondary").with_variant(ButtonVariant::Secondary);
        let _danger = PushButton::new("Danger").with_variant(ButtonVariant::Danger);
        let _flat = PushButton::new("Flat").with_variant(ButtonVariant::Flat);
        let _outlined = PushButton::new("Outlined").with_variant(ButtonVariant::Outlined);
    }

    // =========================================================================
    // Default Button Tests
    // =========================================================================

    #[test]
    fn test_default_button_false_by_default() {
        setup();
        let button = PushButton::new("Test");
        assert!(!button.is_default());
    }

    #[test]
    fn test_default_button_builder() {
        setup();
        let button = PushButton::new("OK").with_default(true);
        assert!(button.is_default());
    }

    #[test]
    fn test_set_default() {
        setup();
        let mut button = PushButton::new("OK");
        assert!(!button.is_default());

        button.set_default(true);
        assert!(button.is_default());

        button.set_default(false);
        assert!(!button.is_default());
    }

    // =========================================================================
    // Mnemonic Tests
    // =========================================================================

    #[test]
    fn test_mnemonic_from_text() {
        setup();
        let button = PushButton::new("&Open");
        assert_eq!(button.display_text(), "Open");
        assert_eq!(button.mnemonic(), Some('o'));
        assert_eq!(button.mnemonic_index(), Some(0));
    }

    #[test]
    fn test_mnemonic_middle_of_text() {
        setup();
        let button = PushButton::new("Save &As");
        assert_eq!(button.display_text(), "Save As");
        assert_eq!(button.mnemonic(), Some('a'));
        assert_eq!(button.mnemonic_index(), Some(5));
    }

    #[test]
    fn test_escaped_ampersand() {
        setup();
        let button = PushButton::new("Fish && Chips");
        assert_eq!(button.display_text(), "Fish & Chips");
        assert_eq!(button.mnemonic(), None);
    }

    #[test]
    fn test_no_mnemonic() {
        setup();
        let button = PushButton::new("Plain Text");
        assert_eq!(button.display_text(), "Plain Text");
        assert_eq!(button.mnemonic(), None);
    }

    #[test]
    fn test_matches_mnemonic() {
        setup();
        let button = PushButton::new("&Open");
        assert!(button.matches_mnemonic('o'));
        assert!(button.matches_mnemonic('O')); // Case insensitive
        assert!(!button.matches_mnemonic('x'));
    }

    // =========================================================================
    // Shortcut Tests
    // =========================================================================

    #[test]
    fn test_no_shortcut_by_default() {
        setup();
        let button = PushButton::new("Test");
        assert!(button.shortcut().is_none());
    }

    #[test]
    fn test_shortcut_builder() {
        setup();
        use crate::widget::{Key, KeySequence};

        let button = PushButton::new("Save").with_shortcut(KeySequence::ctrl(Key::S));
        assert!(button.shortcut().is_some());
        let shortcut = button.shortcut().unwrap();
        assert_eq!(shortcut.key(), Key::S);
        assert!(shortcut.modifiers().control);
    }

    #[test]
    fn test_shortcut_from_string() {
        setup();
        use crate::widget::Key;

        let button = PushButton::new("Save").with_shortcut_str("Ctrl+S");
        assert!(button.shortcut().is_some());
        let shortcut = button.shortcut().unwrap();
        assert_eq!(shortcut.key(), Key::S);
        assert!(shortcut.modifiers().control);
    }

    #[test]
    fn test_matches_shortcut() {
        setup();
        use crate::widget::{Key, KeySequence, KeyboardModifiers};

        let button = PushButton::new("Save").with_shortcut(KeySequence::ctrl(Key::S));

        let ctrl_s = KeyboardModifiers {
            control: true,
            ..Default::default()
        };
        let none = KeyboardModifiers::default();

        assert!(button.matches_shortcut(Key::S, ctrl_s));
        assert!(!button.matches_shortcut(Key::S, none));
        assert!(!button.matches_shortcut(Key::A, ctrl_s));
    }
}
