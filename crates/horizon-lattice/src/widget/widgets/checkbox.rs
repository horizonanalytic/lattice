//! CheckBox widget implementation.
//!
//! This module provides [`CheckBox`], a widget that allows users to toggle
//! between checked and unchecked states, with optional tri-state support.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{CheckBox, CheckState};
//!
//! // Create a simple checkbox
//! let mut checkbox = CheckBox::new("Accept terms");
//!
//! // Connect to the toggled signal
//! checkbox.toggled().connect(|&checked| {
//!     println!("Checkbox is now: {}", if checked { "checked" } else { "unchecked" });
//! });
//!
//! // Create a tri-state checkbox
//! let mut tri_state = CheckBox::new("Select all")
//!     .with_tri_state(true);
//!
//! tri_state.state_changed().connect(|state| {
//!     println!("State changed to: {:?}", state);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontSystem, Path, Point, Rect, Renderer, RoundedRect, Stroke, TextLayout,
    TextRenderer,
};

use super::abstract_button::AbstractButton;
use crate::widget::{PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent};

/// The check state of a checkbox.
///
/// Checkboxes can be in one of three states:
/// - `Unchecked`: The checkbox is not selected
/// - `Checked`: The checkbox is fully selected
/// - `PartiallyChecked`: The checkbox is in an indeterminate state (tri-state mode only)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CheckState {
    /// The checkbox is not checked.
    #[default]
    Unchecked,
    /// The checkbox is checked.
    Checked,
    /// The checkbox is in a partially checked (indeterminate) state.
    ///
    /// This state is only available when tri-state mode is enabled.
    /// It's typically used to indicate that some, but not all, child items
    /// are selected in a hierarchical list.
    PartiallyChecked,
}

impl CheckState {
    /// Returns `true` if the state is `Checked`.
    pub fn is_checked(&self) -> bool {
        matches!(self, CheckState::Checked)
    }

    /// Returns `true` if the state is `Unchecked`.
    pub fn is_unchecked(&self) -> bool {
        matches!(self, CheckState::Unchecked)
    }

    /// Returns `true` if the state is `PartiallyChecked`.
    pub fn is_partially_checked(&self) -> bool {
        matches!(self, CheckState::PartiallyChecked)
    }

    /// Converts the state to a boolean.
    ///
    /// - `Checked` returns `true`
    /// - `Unchecked` and `PartiallyChecked` return `false`
    pub fn to_bool(&self) -> bool {
        matches!(self, CheckState::Checked)
    }
}

impl From<bool> for CheckState {
    fn from(checked: bool) -> Self {
        if checked {
            CheckState::Checked
        } else {
            CheckState::Unchecked
        }
    }
}

/// A checkbox widget for toggling boolean or tri-state values.
///
/// CheckBox displays a check indicator alongside a text label. Users can
/// click the checkbox or press Space when focused to toggle its state.
///
/// # States
///
/// In normal mode, the checkbox toggles between `Checked` and `Unchecked`.
/// When tri-state mode is enabled, clicking cycles through:
/// `Unchecked` → `Checked` → `PartiallyChecked` → `Unchecked`
///
/// # Visual Appearance
///
/// The checkbox renders a small box indicator on the left with:
/// - A checkmark (✓) when checked
/// - A horizontal dash (—) when partially checked
/// - Empty when unchecked
///
/// The label text appears to the right of the indicator.
///
/// # Signals
///
/// - `state_changed(CheckState)`: Emitted when the check state changes
/// - `toggled(bool)`: Emitted when the boolean checked state changes
/// - `clicked(bool)`: Emitted when clicked
/// - `pressed()`: Emitted when pressed down
/// - `released()`: Emitted when released
pub struct CheckBox {
    /// The underlying abstract button implementation.
    inner: AbstractButton,

    /// Whether tri-state mode is enabled.
    tri_state: bool,

    /// The current check state.
    check_state: CheckState,

    /// Size of the check indicator box.
    indicator_size: f32,

    /// Spacing between the indicator and label text.
    indicator_spacing: f32,

    /// Border radius of the indicator box.
    indicator_radius: f32,

    /// Color of the indicator box border.
    indicator_border_color: Color,

    /// Color of the indicator box background when checked.
    indicator_checked_color: Color,

    /// Signal emitted when the check state changes.
    pub state_changed: Signal<CheckState>,
}

impl CheckBox {
    /// Create a new checkbox with the specified label text.
    pub fn new(text: impl Into<String>) -> Self {
        let mut inner = AbstractButton::new(text);
        // Checkboxes are always checkable
        inner.set_checkable(true);
        // Default text color to near-black for readability
        inner.set_text_color(Some(Color::from_rgb8(33, 33, 33)));

        Self {
            inner,
            tri_state: false,
            check_state: CheckState::Unchecked,
            indicator_size: 18.0,
            indicator_spacing: 8.0,
            indicator_radius: 3.0,
            indicator_border_color: Color::from_rgb8(158, 158, 158),
            indicator_checked_color: Color::from_rgb8(33, 150, 243), // Material Blue
            state_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Text
    // =========================================================================

    /// Get the checkbox's label text.
    pub fn text(&self) -> &str {
        self.inner.text()
    }

    /// Set the checkbox's label text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.inner.set_text(text);
    }

    /// Set the text using builder pattern.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.inner.set_text(text);
        self
    }

    // =========================================================================
    // Check State
    // =========================================================================

    /// Get the current check state.
    pub fn check_state(&self) -> CheckState {
        self.check_state
    }

    /// Set the check state.
    ///
    /// If tri-state mode is disabled and `PartiallyChecked` is passed,
    /// it will be converted to `Unchecked`.
    pub fn set_check_state(&mut self, state: CheckState) {
        let effective_state = if !self.tri_state && state == CheckState::PartiallyChecked {
            CheckState::Unchecked
        } else {
            state
        };

        if self.check_state != effective_state {
            let old_checked = self.check_state.to_bool();
            self.check_state = effective_state;
            let new_checked = effective_state.to_bool();

            // Sync with AbstractButton's checked state
            if old_checked != new_checked {
                // Temporarily disable to avoid double signal emission
                self.inner.toggled.set_blocked(true);
                self.inner.set_checked(new_checked);
                self.inner.toggled.set_blocked(false);

                // Emit toggled signal
                self.inner.toggled.emit(new_checked);
            }

            // Always emit state_changed
            self.state_changed.emit(effective_state);
            self.inner.widget_base_mut().update();
        }
    }

    /// Set check state using builder pattern.
    pub fn with_check_state(mut self, state: CheckState) -> Self {
        self.set_check_state(state);
        self
    }

    /// Check if the checkbox is currently checked.
    ///
    /// Returns `true` only for `CheckState::Checked`.
    pub fn is_checked(&self) -> bool {
        self.check_state.is_checked()
    }

    /// Set the checked state (boolean).
    ///
    /// This is a convenience method that sets the state to `Checked` or `Unchecked`.
    pub fn set_checked(&mut self, checked: bool) {
        self.set_check_state(CheckState::from(checked));
    }

    /// Set checked using builder pattern.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.set_check_state(CheckState::from(checked));
        self
    }

    // =========================================================================
    // Tri-State
    // =========================================================================

    /// Check if tri-state mode is enabled.
    pub fn is_tri_state(&self) -> bool {
        self.tri_state
    }

    /// Enable or disable tri-state mode.
    ///
    /// When tri-state mode is disabled while in `PartiallyChecked` state,
    /// the checkbox will be set to `Unchecked`.
    pub fn set_tri_state(&mut self, enabled: bool) {
        if self.tri_state != enabled {
            self.tri_state = enabled;
            if !enabled && self.check_state == CheckState::PartiallyChecked {
                self.set_check_state(CheckState::Unchecked);
            }
        }
    }

    /// Set tri-state mode using builder pattern.
    pub fn with_tri_state(mut self, enabled: bool) -> Self {
        self.tri_state = enabled;
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the indicator box size.
    pub fn indicator_size(&self) -> f32 {
        self.indicator_size
    }

    /// Set the indicator box size.
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

    /// Toggle the checkbox state.
    ///
    /// In normal mode: toggles between Checked and Unchecked.
    /// In tri-state mode: cycles Unchecked → Checked → PartiallyChecked → Unchecked.
    pub fn toggle(&mut self) {
        let next_state = if self.tri_state {
            match self.check_state {
                CheckState::Unchecked => CheckState::Checked,
                CheckState::Checked => CheckState::PartiallyChecked,
                CheckState::PartiallyChecked => CheckState::Unchecked,
            }
        } else {
            match self.check_state {
                CheckState::Unchecked => CheckState::Checked,
                CheckState::Checked | CheckState::PartiallyChecked => CheckState::Unchecked,
            }
        };

        self.set_check_state(next_state);
    }

    /// Programmatically click the checkbox.
    ///
    /// This toggles the state and emits all relevant signals.
    pub fn click(&mut self) {
        if !self.inner.widget_base().is_effectively_enabled() {
            return;
        }

        self.toggle();
        self.inner.clicked.emit(self.is_checked());
        self.inner.widget_base_mut().update();
    }

    // =========================================================================
    // Signal Access
    // =========================================================================

    /// Get the state_changed signal.
    ///
    /// Emitted when the check state changes.
    pub fn state_changed(&self) -> &Signal<CheckState> {
        &self.state_changed
    }

    /// Get the toggled signal.
    ///
    /// Emitted when the boolean checked state changes.
    pub fn toggled(&self) -> &Signal<bool> {
        &self.inner.toggled
    }

    /// Get the clicked signal.
    ///
    /// Emitted when the checkbox is clicked.
    pub fn clicked(&self) -> &Signal<bool> {
        &self.inner.clicked
    }

    /// Get the pressed signal.
    ///
    /// Emitted when the checkbox is pressed down.
    pub fn pressed(&self) -> &Signal<()> {
        &self.inner.pressed
    }

    /// Get the released signal.
    ///
    /// Emitted when the checkbox is released.
    pub fn released(&self) -> &Signal<()> {
        &self.inner.released
    }

    // =========================================================================
    // Rendering Helpers
    // =========================================================================

    /// Calculate the indicator color based on current state.
    fn indicator_color(&self) -> Color {
        let base = self.inner.widget_base();

        if !base.is_effectively_enabled() {
            Color::from_rgb8(189, 189, 189) // Disabled gray
        } else if self.check_state != CheckState::Unchecked {
            // Checked or partially checked
            if base.is_pressed() {
                darken_color(self.indicator_checked_color, 0.2)
            } else if base.is_hovered() {
                lighten_color(self.indicator_checked_color, 0.1)
            } else {
                self.indicator_checked_color
            }
        } else {
            // Unchecked - use border color for pressed/hovered feedback
            if base.is_pressed() {
                Color::from_rgb8(97, 97, 97)
            } else if base.is_hovered() {
                Color::from_rgb8(117, 117, 117)
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
        } else if self.check_state != CheckState::Unchecked {
            // When checked, border matches fill
            self.indicator_color()
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

    /// Draw the check indicator (checkmark or dash).
    fn draw_indicator(&self, ctx: &mut PaintContext<'_>, indicator_rect: Rect) {
        let check_color = if self.inner.widget_base().is_effectively_enabled() {
            Color::WHITE
        } else {
            Color::from_rgb8(158, 158, 158)
        };

        match self.check_state {
            CheckState::Checked => {
                // Draw a checkmark
                self.draw_checkmark(ctx, indicator_rect, check_color);
            }
            CheckState::PartiallyChecked => {
                // Draw a horizontal dash
                self.draw_dash(ctx, indicator_rect, check_color);
            }
            CheckState::Unchecked => {
                // Nothing to draw
            }
        }
    }

    /// Draw a checkmark inside the indicator rect.
    fn draw_checkmark(&self, ctx: &mut PaintContext<'_>, rect: Rect, color: Color) {
        // Checkmark path: short leg down-right, long leg up-right
        let padding = rect.width() * 0.2;
        let inner_rect = Rect::new(
            rect.origin.x + padding,
            rect.origin.y + padding,
            rect.width() - padding * 2.0,
            rect.height() - padding * 2.0,
        );

        // Start point (left side, slightly below middle)
        let start = Point::new(inner_rect.origin.x, inner_rect.origin.y + inner_rect.height() * 0.5);

        // Middle point (bottom of the checkmark)
        let middle = Point::new(
            inner_rect.origin.x + inner_rect.width() * 0.35,
            inner_rect.origin.y + inner_rect.height() * 0.75,
        );

        // End point (top right)
        let end = Point::new(
            inner_rect.origin.x + inner_rect.width(),
            inner_rect.origin.y + inner_rect.height() * 0.15,
        );

        let mut path = Path::new();
        path.move_to(start);
        path.line_to(middle);
        path.line_to(end);

        let stroke = Stroke::new(color, 2.0);
        ctx.renderer().stroke_path(&path, &stroke);
    }

    /// Draw a horizontal dash inside the indicator rect.
    fn draw_dash(&self, ctx: &mut PaintContext<'_>, rect: Rect, color: Color) {
        let padding = rect.width() * 0.25;
        let y = rect.origin.y + rect.height() / 2.0;

        let start = Point::new(rect.origin.x + padding, y);
        let end = Point::new(rect.origin.x + rect.width() - padding, y);

        let stroke = Stroke::new(color, 2.0);
        ctx.renderer().draw_line(start, end, &stroke);
    }
}

impl Object for CheckBox {
    fn object_id(&self) -> ObjectId {
        self.inner.widget_base().object_id()
    }
}

impl Widget for CheckBox {
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
        let indicator_x = rect.origin.x + padding;
        let indicator_y = rect.origin.y + (rect.height() - self.indicator_size) / 2.0;
        let indicator_rect =
            Rect::new(indicator_x, indicator_y, self.indicator_size, self.indicator_size);

        // Draw indicator background
        let bg_color = self.indicator_color();
        let border_color = self.effective_border_color();

        let rrect = RoundedRect::new(indicator_rect, self.indicator_radius);

        // Fill background if checked or hovered
        if bg_color != Color::TRANSPARENT {
            ctx.renderer().fill_rounded_rect(rrect, bg_color);
        }

        // Draw border
        let border_stroke = Stroke::new(border_color, 1.5);
        ctx.renderer().stroke_rounded_rect(rrect, &border_stroke);

        // Draw check indicator (checkmark or dash)
        self.draw_indicator(ctx, indicator_rect);

        // Draw label text
        if !self.inner.text().is_empty() {
            let mut font_system = FontSystem::new();
            let layout = TextLayout::new(&mut font_system, self.inner.text(), self.inner.font());

            // Position text to the right of indicator, vertically centered
            let text_x = indicator_x + self.indicator_size + self.indicator_spacing;
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
            let focus_rect = RoundedRect::new(rect.inflate(1.0), 4.0);
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
                // Don't use AbstractButton's release handler since we have custom click behavior
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
                // Custom key release handling for our click behavior
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

// Ensure CheckBox is Send + Sync
static_assertions::assert_impl_all!(CheckBox: Send, Sync);

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
        atomic::{AtomicU32, Ordering},
        Arc,
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_checkbox_creation() {
        setup();
        let checkbox = CheckBox::new("Test Checkbox");
        assert_eq!(checkbox.text(), "Test Checkbox");
        assert_eq!(checkbox.check_state(), CheckState::Unchecked);
        assert!(!checkbox.is_checked());
        assert!(!checkbox.is_tri_state());
    }

    #[test]
    fn test_checkbox_builder_pattern() {
        setup();
        let checkbox = CheckBox::new("Test")
            .with_checked(true)
            .with_indicator_size(20.0)
            .with_text_color(Color::BLACK);

        assert!(checkbox.is_checked());
        assert_eq!(checkbox.check_state(), CheckState::Checked);
        assert_eq!(checkbox.indicator_size(), 20.0);
        assert_eq!(checkbox.text_color(), Color::BLACK);
    }

    #[test]
    fn test_checkbox_toggle() {
        setup();
        let mut checkbox = CheckBox::new("Toggle");

        assert_eq!(checkbox.check_state(), CheckState::Unchecked);

        checkbox.toggle();
        assert_eq!(checkbox.check_state(), CheckState::Checked);

        checkbox.toggle();
        assert_eq!(checkbox.check_state(), CheckState::Unchecked);
    }

    #[test]
    fn test_checkbox_tri_state_toggle() {
        setup();
        let mut checkbox = CheckBox::new("Tri-state").with_tri_state(true);

        assert_eq!(checkbox.check_state(), CheckState::Unchecked);

        checkbox.toggle();
        assert_eq!(checkbox.check_state(), CheckState::Checked);

        checkbox.toggle();
        assert_eq!(checkbox.check_state(), CheckState::PartiallyChecked);

        checkbox.toggle();
        assert_eq!(checkbox.check_state(), CheckState::Unchecked);
    }

    #[test]
    fn test_checkbox_signals() {
        setup();
        let mut checkbox = CheckBox::new("Signals");

        let state_change_count = Arc::new(AtomicU32::new(0));
        let toggle_count = Arc::new(AtomicU32::new(0));

        let state_clone = state_change_count.clone();
        checkbox.state_changed.connect(move |_| {
            state_clone.fetch_add(1, Ordering::SeqCst);
        });

        let toggle_clone = toggle_count.clone();
        checkbox.inner.toggled.connect(move |_| {
            toggle_clone.fetch_add(1, Ordering::SeqCst);
        });

        checkbox.toggle();
        assert_eq!(state_change_count.load(Ordering::SeqCst), 1);
        assert_eq!(toggle_count.load(Ordering::SeqCst), 1);

        checkbox.toggle();
        assert_eq!(state_change_count.load(Ordering::SeqCst), 2);
        assert_eq!(toggle_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_checkbox_tri_state_signals() {
        setup();
        let mut checkbox = CheckBox::new("Tri-state Signals").with_tri_state(true);

        let states = Arc::new(parking_lot::Mutex::new(Vec::new()));

        let states_clone = states.clone();
        checkbox.state_changed.connect(move |state| {
            states_clone.lock().push(*state);
        });

        checkbox.toggle(); // Unchecked -> Checked
        checkbox.toggle(); // Checked -> PartiallyChecked
        checkbox.toggle(); // PartiallyChecked -> Unchecked

        let received_states = states.lock().clone();
        assert_eq!(
            received_states,
            vec![
                CheckState::Checked,
                CheckState::PartiallyChecked,
                CheckState::Unchecked
            ]
        );
    }

    #[test]
    fn test_checkbox_set_check_state() {
        setup();
        let mut checkbox = CheckBox::new("State");

        checkbox.set_check_state(CheckState::Checked);
        assert_eq!(checkbox.check_state(), CheckState::Checked);

        // PartiallyChecked should become Unchecked when not tri-state
        checkbox.set_check_state(CheckState::PartiallyChecked);
        assert_eq!(checkbox.check_state(), CheckState::Unchecked);

        // Enable tri-state and try again
        checkbox.set_tri_state(true);
        checkbox.set_check_state(CheckState::PartiallyChecked);
        assert_eq!(checkbox.check_state(), CheckState::PartiallyChecked);
    }

    #[test]
    fn test_checkbox_disable_tri_state() {
        setup();
        let mut checkbox = CheckBox::new("Tri-state")
            .with_tri_state(true)
            .with_check_state(CheckState::PartiallyChecked);

        assert_eq!(checkbox.check_state(), CheckState::PartiallyChecked);

        // Disabling tri-state should convert PartiallyChecked to Unchecked
        checkbox.set_tri_state(false);
        assert_eq!(checkbox.check_state(), CheckState::Unchecked);
    }

    #[test]
    fn test_checkbox_size_hint() {
        setup();
        let checkbox = CheckBox::new("Test");
        let hint = checkbox.size_hint();

        // Should have reasonable preferred size
        assert!(hint.preferred.width >= checkbox.indicator_size());
        assert!(hint.preferred.height >= checkbox.indicator_size());
    }

    #[test]
    fn test_check_state_conversions() {
        assert!(CheckState::Checked.is_checked());
        assert!(!CheckState::Unchecked.is_checked());
        assert!(!CheckState::PartiallyChecked.is_checked());

        assert!(CheckState::Unchecked.is_unchecked());
        assert!(!CheckState::Checked.is_unchecked());

        assert!(CheckState::PartiallyChecked.is_partially_checked());

        assert!(CheckState::Checked.to_bool());
        assert!(!CheckState::Unchecked.to_bool());
        assert!(!CheckState::PartiallyChecked.to_bool());

        assert_eq!(CheckState::from(true), CheckState::Checked);
        assert_eq!(CheckState::from(false), CheckState::Unchecked);
    }
}
