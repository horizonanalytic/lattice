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
    Color, Font, FontSystem, Point, Renderer, RoundedRect, TextLayout, TextRenderer,
};

use super::abstract_button::AbstractButton;
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

    /// Base color for the button background.
    base_color: Color,

    /// Border radius for rounded corners.
    border_radius: f32,
}

impl PushButton {
    /// Create a new push button with the specified text.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            inner: AbstractButton::new(text),
            base_color: Color::from_rgb8(66, 133, 244), // Google Blue
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

    /// Get the text color.
    pub fn text_color(&self) -> Color {
        self.inner.text_color()
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        self.inner.set_text_color(color);
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.inner = self.inner.with_text_color(color);
        self
    }

    // =========================================================================
    // PushButton-Specific Methods
    // =========================================================================

    /// Get the base color.
    pub fn base_color(&self) -> Color {
        self.base_color
    }

    /// Set the base background color.
    pub fn set_base_color(&mut self, color: Color) {
        if self.base_color != color {
            self.base_color = color;
            self.inner.widget_base_mut().update();
        }
    }

    /// Set base color using builder pattern.
    pub fn with_base_color(mut self, color: Color) -> Self {
        self.base_color = color;
        self
    }

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

        // Calculate background color based on state
        let bg_color = self.inner.background_color(self.base_color);

        // Draw rounded rectangle background
        let rrect = RoundedRect::new(rect, self.border_radius);
        ctx.renderer().fill_rounded_rect(rrect, bg_color);

        // Draw text if present
        if !self.inner.text().is_empty() {
            let mut font_system = FontSystem::new();
            let layout = TextLayout::new(&mut font_system, self.inner.text(), self.inner.font());

            // Center the text in the button
            let text_x = rect.origin.x + (rect.width() - layout.width()) / 2.0;
            let text_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;
            let text_pos = Point::new(text_x, text_y);

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
        assert_eq!(button.text_color(), Color::WHITE);
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
}
