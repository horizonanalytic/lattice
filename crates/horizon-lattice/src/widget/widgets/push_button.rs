//! Push button widget implementation.
//!
//! This module provides [`PushButton`], the standard clickable button widget.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::PushButton;
//!
//! // Create a simple button
//! let mut button = PushButton::new("Click me!");
//!
//! // Connect to the clicked signal
//! button.clicked.connect(|checked| {
//!     println!("Button clicked! Checked: {}", checked);
//! });
//!
//! // Create a toggle button
//! let mut toggle = PushButton::new("Toggle")
//!     .with_checkable(true);
//!
//! toggle.toggled.connect(|checked| {
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

    /// Programmatically click the button.
    pub fn click(&mut self) {
        self.inner.click();
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
        if shows_text && !self.inner.text().is_empty() {
            let mut font_system = FontSystem::new();
            let layout = TextLayout::new(&mut font_system, self.inner.text(), self.inner.font());

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
        }

        // Draw focus indicator when focused
        if self.widget_base().has_focus() {
            let focus_rect = RoundedRect::new(
                rect.inflate(2.0),
                self.border_radius + 2.0,
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
}
