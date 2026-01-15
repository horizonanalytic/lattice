//! Radio button widget implementation.
//!
//! This module provides [`RadioButton`], a widget for exclusive selection
//! among a group of options.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{RadioButton, ButtonGroup};
//! use std::sync::{Arc, RwLock};
//!
//! // Create a button group for exclusive selection
//! let group = Arc::new(RwLock::new(ButtonGroup::new()));
//!
//! // Create radio buttons
//! let mut rb1 = RadioButton::new("Option 1").with_group(group.clone());
//! let mut rb2 = RadioButton::new("Option 2").with_group(group.clone());
//! let mut rb3 = RadioButton::new("Option 3").with_group(group.clone());
//!
//! // Connect to signals
//! rb1.toggled().connect(|&checked| {
//!     if checked {
//!         println!("Option 1 selected");
//!     }
//! });
//! ```
//!
//! # Exclusive Selection
//!
//! Radio buttons support two modes of exclusive selection:
//!
//! 1. **Explicit ButtonGroup**: Create a `ButtonGroup` and assign it to radio buttons
//!    using `with_group()` or `set_group()`. The group handles exclusivity.
//!
//! 2. **Auto-exclusive** (default): When `auto_exclusive` is true and no explicit
//!    group is assigned, radio buttons attempt to find sibling radio buttons
//!    and coordinate exclusive selection through the `exclusive_toggle` signal.

use std::sync::{Arc, RwLock, Weak};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontSystem, Point, Renderer, Stroke, TextLayout, TextRenderer,
};

use super::abstract_button::AbstractButton;
use super::button_group::ButtonGroup;
use crate::widget::{PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent};

/// A radio button widget for exclusive selection among options.
///
/// RadioButton displays a circular indicator alongside a text label. When clicked,
/// it becomes checked and (in exclusive mode) unchecks other radio buttons in the
/// same group.
///
/// # Visual Appearance
///
/// The radio button renders a small circular indicator on the left with:
/// - A filled inner circle when checked
/// - An empty circle when unchecked
///
/// The label text appears to the right of the indicator.
///
/// # Signals
///
/// - `toggled(bool)`: Emitted when the checked state changes
/// - `clicked(bool)`: Emitted when clicked
/// - `pressed()`: Emitted when pressed down
/// - `released()`: Emitted when released
/// - `exclusive_toggle(ObjectId)`: Emitted to request siblings to uncheck
pub struct RadioButton {
    /// The underlying abstract button implementation.
    inner: AbstractButton,

    /// Optional button group for exclusive selection.
    group: Option<Weak<RwLock<ButtonGroup>>>,

    /// Whether auto-exclusive mode is enabled.
    auto_exclusive: bool,

    /// Size of the radio indicator circle.
    indicator_size: f32,

    /// Spacing between the indicator and label text.
    indicator_spacing: f32,

    /// Color of the indicator border.
    indicator_border_color: Color,

    /// Color of the indicator when checked.
    indicator_checked_color: Color,

    /// Signal emitted to request exclusive toggle from siblings.
    ///
    /// When a radio button is checked in auto-exclusive mode, it emits this
    /// signal with its own ObjectId. Other radio buttons connected to this
    /// signal should uncheck themselves if they are not the sender.
    pub exclusive_toggle: Signal<ObjectId>,
}

impl RadioButton {
    /// Create a new radio button with the specified label text.
    pub fn new(text: impl Into<String>) -> Self {
        let mut inner = AbstractButton::new(text);
        // Radio buttons are always checkable
        inner.set_checkable(true);
        // Default text color to near-black for readability
        inner.set_text_color(Some(Color::from_rgb8(33, 33, 33)));

        Self {
            inner,
            group: None,
            auto_exclusive: true,
            indicator_size: 18.0,
            indicator_spacing: 8.0,
            indicator_border_color: Color::from_rgb8(158, 158, 158),
            indicator_checked_color: Color::from_rgb8(33, 150, 243), // Material Blue
            exclusive_toggle: Signal::new(),
        }
    }

    // =========================================================================
    // Text
    // =========================================================================

    /// Get the radio button's label text.
    pub fn text(&self) -> &str {
        self.inner.text()
    }

    /// Set the radio button's label text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.inner.set_text(text);
    }

    /// Set the text using builder pattern.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.inner.set_text(text);
        self
    }

    // =========================================================================
    // Checked State
    // =========================================================================

    /// Check if the radio button is currently checked.
    pub fn is_checked(&self) -> bool {
        self.inner.is_checked()
    }

    /// Set the checked state.
    ///
    /// Note: In exclusive mode, setting a radio button to checked will NOT
    /// automatically uncheck other radio buttons. Use `set_checked_exclusive()`
    /// for that behavior, or rely on click handling.
    pub fn set_checked(&mut self, checked: bool) {
        if self.inner.is_checked() != checked {
            self.inner.set_checked(checked);
        }
    }

    /// Set the checked state and handle exclusive behavior.
    ///
    /// If `checked` is true and the radio button is in a group or auto-exclusive,
    /// this will notify the group/siblings to uncheck.
    pub fn set_checked_exclusive(&mut self, checked: bool) {
        if checked && !self.inner.is_checked() {
            self.inner.set_checked(true);
            self.handle_exclusive_check();
        } else if !checked {
            // In exclusive mode, unchecking might be prevented
            if let Some(group_weak) = &self.group {
                if let Some(group) = group_weak.upgrade() {
                    if let Ok(group_guard) = group.read() {
                        if group_guard.should_prevent_uncheck(self.object_id()) {
                            return;
                        }
                    }
                }
            }
            self.inner.set_checked(false);
        }
    }

    /// Set checked state using builder pattern.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.inner.set_checked(checked);
        self
    }

    // =========================================================================
    // Button Group
    // =========================================================================

    /// Get the button group this radio button belongs to.
    pub fn group(&self) -> Option<Arc<RwLock<ButtonGroup>>> {
        self.group.as_ref().and_then(|w| w.upgrade())
    }

    /// Set the button group for exclusive selection.
    ///
    /// This adds the radio button to the group. Passing `None` removes the
    /// radio button from its current group.
    pub fn set_group(&mut self, group: Option<Arc<RwLock<ButtonGroup>>>) {
        // Remove from old group
        if let Some(old_weak) = &self.group {
            if let Some(old_group) = old_weak.upgrade() {
                if let Ok(mut old_guard) = old_group.write() {
                    old_guard.remove_button(self.object_id());
                }
            }
        }

        // Add to new group
        if let Some(new_group) = &group {
            if let Ok(mut new_guard) = new_group.write() {
                new_guard.add_button(self.object_id());
            }
        }

        self.group = group.map(|g| Arc::downgrade(&g));
    }

    /// Set the button group using builder pattern.
    pub fn with_group(mut self, group: Arc<RwLock<ButtonGroup>>) -> Self {
        self.set_group(Some(group));
        self
    }

    // =========================================================================
    // Auto-Exclusive
    // =========================================================================

    /// Check if auto-exclusive mode is enabled.
    ///
    /// When enabled, radio buttons without an explicit group will attempt
    /// to coordinate exclusive selection with sibling radio buttons.
    pub fn is_auto_exclusive(&self) -> bool {
        self.auto_exclusive
    }

    /// Set whether auto-exclusive mode is enabled.
    pub fn set_auto_exclusive(&mut self, enabled: bool) {
        self.auto_exclusive = enabled;
    }

    /// Set auto-exclusive using builder pattern.
    pub fn with_auto_exclusive(mut self, enabled: bool) -> Self {
        self.auto_exclusive = enabled;
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the indicator circle size.
    pub fn indicator_size(&self) -> f32 {
        self.indicator_size
    }

    /// Set the indicator circle size.
    pub fn set_indicator_size(&mut self, size: f32) {
        if self.indicator_size != size {
            self.indicator_size = size;
            self.inner.widget_base_mut().update();
        }
    }

    /// Set indicator size using builder pattern.
    pub fn with_indicator_size(mut self, size: f32) -> Self {
        self.indicator_size = size;
        self
    }

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
        self.inner.set_font(font);
        self
    }

    /// Get the effective text color.
    pub fn text_color(&self) -> Color {
        self.inner.effective_text_color()
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        self.inner.set_text_color(Some(color));
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.inner.set_text_color(Some(color));
        self
    }

    /// Get the checked indicator color.
    pub fn indicator_checked_color(&self) -> Color {
        self.indicator_checked_color
    }

    /// Set the checked indicator color.
    pub fn set_indicator_checked_color(&mut self, color: Color) {
        if self.indicator_checked_color != color {
            self.indicator_checked_color = color;
            self.inner.widget_base_mut().update();
        }
    }

    /// Set indicator checked color using builder pattern.
    pub fn with_indicator_checked_color(mut self, color: Color) -> Self {
        self.indicator_checked_color = color;
        self
    }

    // =========================================================================
    // Actions
    // =========================================================================

    /// Handle exclusive checking behavior.
    ///
    /// Called when this radio button is checked. Notifies the group or
    /// emits the exclusive_toggle signal for auto-exclusive behavior.
    fn handle_exclusive_check(&mut self) {
        let my_id = self.object_id();

        // If in a group, notify the group
        if let Some(group_weak) = &self.group {
            if let Some(group) = group_weak.upgrade() {
                if let Ok(mut group_guard) = group.write() {
                    let buttons_to_uncheck = group_guard.button_toggled(my_id, true);
                    // Note: The actual unchecking of other buttons needs to be handled
                    // by the application or a parent coordinator, since we don't have
                    // mutable access to sibling widgets.
                    drop(group_guard);

                    // Emit signals for each button that should be unchecked
                    // Other radio buttons should connect to this signal
                    for _ in buttons_to_uncheck {
                        self.exclusive_toggle.emit(my_id);
                    }
                }
            }
        } else if self.auto_exclusive {
            // Auto-exclusive: emit signal for siblings to uncheck
            self.exclusive_toggle.emit(my_id);
        }
    }

    /// Programmatically click the radio button.
    ///
    /// In exclusive mode, this will also handle unchecking other radio buttons.
    pub fn click(&mut self) {
        if !self.inner.widget_base().is_effectively_enabled() {
            return;
        }

        // Radio buttons can only be checked by clicking, not unchecked
        // (unless in non-exclusive mode)
        if self.inner.is_checked() {
            // Check if unchecking is prevented
            if let Some(group_weak) = &self.group {
                if let Some(group) = group_weak.upgrade() {
                    if let Ok(group_guard) = group.read() {
                        if group_guard.should_prevent_uncheck(self.object_id()) {
                            // Just emit clicked, don't change state
                            self.inner.clicked.emit(true);
                            return;
                        }
                    }
                }
            }
        }

        // Toggle state (usually check, since radio buttons are rarely unchecked directly)
        let new_checked = !self.inner.is_checked();
        self.inner.set_checked(new_checked);

        if new_checked {
            self.handle_exclusive_check();
        }

        self.inner.clicked.emit(new_checked);
        self.inner.widget_base_mut().update();
    }

    /// Handle an incoming exclusive toggle request.
    ///
    /// Call this when receiving an `exclusive_toggle` signal from a sibling.
    /// If the sender is not this button and this button is checked, uncheck it.
    pub fn handle_exclusive_toggle_request(&mut self, sender_id: ObjectId) {
        if sender_id != self.object_id() && self.inner.is_checked() {
            self.inner.set_checked(false);
            self.inner.widget_base_mut().update();
        }
    }

    // =========================================================================
    // Signal Access
    // =========================================================================

    /// Get the toggled signal.
    ///
    /// Emitted when the checked state changes.
    pub fn toggled(&self) -> &Signal<bool> {
        &self.inner.toggled
    }

    /// Get the clicked signal.
    ///
    /// Emitted when the radio button is clicked.
    pub fn clicked(&self) -> &Signal<bool> {
        &self.inner.clicked
    }

    /// Get the pressed signal.
    ///
    /// Emitted when the radio button is pressed down.
    pub fn pressed(&self) -> &Signal<()> {
        &self.inner.pressed
    }

    /// Get the released signal.
    ///
    /// Emitted when the radio button is released.
    pub fn released(&self) -> &Signal<()> {
        &self.inner.released
    }

    // =========================================================================
    // Rendering Helpers
    // =========================================================================

    /// Calculate the indicator color based on current state.
    fn indicator_fill_color(&self) -> Color {
        let base = self.inner.widget_base();

        if !base.is_effectively_enabled() {
            Color::from_rgb8(189, 189, 189) // Disabled gray
        } else if self.inner.is_checked() {
            if base.is_pressed() {
                darken_color(self.indicator_checked_color, 0.2)
            } else if base.is_hovered() {
                lighten_color(self.indicator_checked_color, 0.1)
            } else {
                self.indicator_checked_color
            }
        } else {
            // Unchecked - subtle hover/press feedback
            if base.is_pressed() {
                Color::from_rgba8(0, 0, 0, 20)
            } else if base.is_hovered() {
                Color::from_rgba8(0, 0, 0, 10)
            } else {
                Color::TRANSPARENT
            }
        }
    }

    /// Calculate the indicator border color based on current state.
    fn effective_border_color(&self) -> Color {
        let base = self.inner.widget_base();

        if !base.is_effectively_enabled() {
            Color::from_rgb8(189, 189, 189)
        } else if self.inner.is_checked() {
            self.indicator_checked_color
        } else if base.is_pressed() {
            Color::from_rgb8(97, 97, 97)
        } else if base.is_hovered() {
            Color::from_rgb8(117, 117, 117)
        } else {
            self.indicator_border_color
        }
    }

    /// Get the effective text color based on state.
    fn effective_text_color(&self) -> Color {
        self.inner.effective_text_color()
    }

    /// Draw the radio indicator (outer circle and inner dot when checked).
    fn draw_indicator(&self, ctx: &mut PaintContext<'_>, center: Point, radius: f32) {
        let border_color = self.effective_border_color();
        let fill_color = self.indicator_fill_color();

        // Draw outer circle border
        let border_stroke = Stroke::new(border_color, 1.5);
        ctx.renderer().stroke_circle(center, radius, &border_stroke);

        // Draw fill (for hover/press feedback when unchecked, or full fill when checked)
        if fill_color != Color::TRANSPARENT && !self.inner.is_checked() {
            ctx.renderer().fill_circle(center, radius - 1.0, fill_color);
        }

        // Draw inner dot when checked
        if self.inner.is_checked() {
            let inner_color = if self.inner.widget_base().is_effectively_enabled() {
                self.indicator_checked_color
            } else {
                Color::from_rgb8(158, 158, 158)
            };

            // Fill the outer circle
            ctx.renderer().fill_circle(center, radius - 1.0, inner_color);

            // Draw white inner circle (or filled dot style)
            let inner_radius = radius * 0.4;
            let dot_color = if self.inner.widget_base().is_effectively_enabled() {
                Color::WHITE
            } else {
                Color::from_rgb8(220, 220, 220)
            };
            ctx.renderer().fill_circle(center, inner_radius, dot_color);
        }
    }
}

impl Object for RadioButton {
    fn object_id(&self) -> ObjectId {
        self.inner.widget_base().object_id()
    }
}

impl Widget for RadioButton {
    fn widget_base(&self) -> &WidgetBase {
        self.inner.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.inner.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        let text_size = if self.inner.text().is_empty() {
            horizon_lattice_render::Size::new(0.0, self.inner.font().size())
        } else {
            let mut font_system = FontSystem::new();
            let layout = TextLayout::new(&mut font_system, self.inner.text(), self.inner.font());
            horizon_lattice_render::Size::new(layout.width(), layout.height())
        };

        // Total width: indicator + spacing + text
        let width = self.indicator_size + self.indicator_spacing + text_size.width;
        // Height: max of indicator and text
        let height = self.indicator_size.max(text_size.height);

        // Add some padding
        let padding = 4.0;
        let preferred =
            horizon_lattice_render::Size::new(width + padding * 2.0, height + padding * 2.0);

        SizeHint::new(preferred)
            .with_minimum_dimensions(self.indicator_size + padding * 2.0, height + padding * 2.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        let padding = 4.0;

        // Calculate indicator position (vertically centered)
        let indicator_x = rect.origin.x + padding + self.indicator_size / 2.0;
        let indicator_y = rect.origin.y + rect.height() / 2.0;
        let indicator_center = Point::new(indicator_x, indicator_y);
        let indicator_radius = self.indicator_size / 2.0;

        // Draw the radio indicator
        self.draw_indicator(ctx, indicator_center, indicator_radius);

        // Draw label text
        if !self.inner.text().is_empty() {
            let mut font_system = FontSystem::new();
            let layout = TextLayout::new(&mut font_system, self.inner.text(), self.inner.font());

            // Position text to the right of indicator, vertically centered
            let text_x = rect.origin.x + padding + self.indicator_size + self.indicator_spacing;
            let text_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;
            let text_pos = Point::new(text_x, text_y);

            let text_color = self.effective_text_color();

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
            let focus_rect = horizon_lattice_render::RoundedRect::new(rect.inflate(1.0), 4.0);
            let focus_color = Color::from_rgba8(33, 150, 243, 64);
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
                if e.button != crate::widget::MouseButton::Left {
                    return false;
                }

                if !self.inner.widget_base().is_effectively_enabled() {
                    return false;
                }

                let is_over = self.inner.widget_base().contains_point(e.local_pos);
                self.inner.released.emit(());

                if is_over && self.inner.widget_base().is_pressed() {
                    self.click();
                    event.accept();
                    return true;
                }

                false
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
                if !self.inner.widget_base().is_effectively_enabled() {
                    return false;
                }

                match e.key {
                    crate::widget::Key::Space | crate::widget::Key::Enter => {
                        self.inner.released.emit(());
                        self.click();
                        event.accept();
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}

// Ensure RadioButton is Send + Sync
static_assertions::assert_impl_all!(RadioButton: Send, Sync);

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
    fn test_radio_button_creation() {
        setup();
        let rb = RadioButton::new("Test Option");
        assert_eq!(rb.text(), "Test Option");
        assert!(!rb.is_checked());
        assert!(rb.is_auto_exclusive());
    }

    #[test]
    fn test_radio_button_builder_pattern() {
        setup();
        let rb = RadioButton::new("Test")
            .with_checked(true)
            .with_auto_exclusive(false)
            .with_indicator_size(20.0)
            .with_text_color(Color::BLACK);

        assert!(rb.is_checked());
        assert!(!rb.is_auto_exclusive());
        assert_eq!(rb.indicator_size(), 20.0);
        assert_eq!(rb.text_color(), Color::BLACK);
    }

    #[test]
    fn test_radio_button_set_checked() {
        setup();
        let mut rb = RadioButton::new("Test");

        assert!(!rb.is_checked());
        rb.set_checked(true);
        assert!(rb.is_checked());
        rb.set_checked(false);
        assert!(!rb.is_checked());
    }

    #[test]
    fn test_radio_button_with_group() {
        setup();
        let group = Arc::new(RwLock::new(ButtonGroup::new()));
        let rb = RadioButton::new("Test").with_group(group.clone());

        assert!(rb.group().is_some());

        // Verify button was added to group
        let group_guard = group.read().unwrap();
        assert!(group_guard.contains(rb.object_id()));
    }

    #[test]
    fn test_radio_button_set_group() {
        setup();
        let group1 = Arc::new(RwLock::new(ButtonGroup::new()));
        let group2 = Arc::new(RwLock::new(ButtonGroup::new()));

        let mut rb = RadioButton::new("Test").with_group(group1.clone());

        // Should be in group1
        assert!(group1.read().unwrap().contains(rb.object_id()));
        assert!(!group2.read().unwrap().contains(rb.object_id()));

        // Move to group2
        rb.set_group(Some(group2.clone()));

        // Should now be in group2, not group1
        assert!(!group1.read().unwrap().contains(rb.object_id()));
        assert!(group2.read().unwrap().contains(rb.object_id()));
    }

    #[test]
    fn test_radio_button_exclusive_toggle_signal() {
        setup();
        let mut rb = RadioButton::new("Test");

        let received = Arc::new(AtomicBool::new(false));
        let received_clone = received.clone();

        rb.exclusive_toggle.connect(move |_| {
            received_clone.store(true, Ordering::SeqCst);
        });

        // When checked in auto-exclusive mode, should emit signal
        rb.set_checked(true);
        rb.handle_exclusive_check();

        assert!(received.load(Ordering::SeqCst));
    }

    #[test]
    fn test_radio_button_toggled_signal() {
        setup();
        let mut rb = RadioButton::new("Test");

        let toggle_count = Arc::new(AtomicU32::new(0));
        let toggle_clone = toggle_count.clone();

        rb.inner.toggled.connect(move |_| {
            toggle_clone.fetch_add(1, Ordering::SeqCst);
        });

        rb.set_checked(true);
        assert_eq!(toggle_count.load(Ordering::SeqCst), 1);

        rb.set_checked(false);
        assert_eq!(toggle_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_radio_button_group_exclusivity() {
        setup();
        let group = Arc::new(RwLock::new(ButtonGroup::new()));

        let mut rb1 = RadioButton::new("Option 1").with_group(group.clone());
        let mut rb2 = RadioButton::new("Option 2").with_group(group.clone());

        // Check rb1
        rb1.set_checked(true);
        {
            let mut g = group.write().unwrap();
            g.button_toggled(rb1.object_id(), true);
        }
        assert!(rb1.is_checked());

        // Check rb2 - rb1 should be in the uncheck list
        rb2.set_checked(true);
        let buttons_to_uncheck = {
            let mut g = group.write().unwrap();
            g.button_toggled(rb2.object_id(), true)
        };

        assert_eq!(buttons_to_uncheck.len(), 1);
        assert_eq!(buttons_to_uncheck[0], rb1.object_id());

        // Verify group state
        let g = group.read().unwrap();
        assert_eq!(g.checked_button(), Some(rb2.object_id()));
    }

    #[test]
    fn test_radio_button_size_hint() {
        setup();
        let rb = RadioButton::new("Test");
        let hint = rb.size_hint();

        // Should have reasonable preferred size
        assert!(hint.preferred.width >= rb.indicator_size());
        assert!(hint.preferred.height >= rb.indicator_size());
    }

    #[test]
    fn test_handle_exclusive_toggle_request() {
        setup();
        let mut rb1 = RadioButton::new("Option 1");
        let rb2 = RadioButton::new("Option 2");

        // Check rb1
        rb1.set_checked(true);
        assert!(rb1.is_checked());

        // Simulate exclusive toggle request from rb2
        rb1.handle_exclusive_toggle_request(rb2.object_id());

        // rb1 should now be unchecked
        assert!(!rb1.is_checked());
    }

    #[test]
    fn test_handle_exclusive_toggle_request_ignores_self() {
        setup();
        let mut rb = RadioButton::new("Option");

        rb.set_checked(true);
        assert!(rb.is_checked());

        // Request from self should be ignored
        rb.handle_exclusive_toggle_request(rb.object_id());

        // Still checked
        assert!(rb.is_checked());
    }
}
