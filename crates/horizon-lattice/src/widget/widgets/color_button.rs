//! Color button widget implementation.
//!
//! This module provides [`ColorButton`], a widget that displays a color swatch
//! and emits signals when clicked. Optionally supports a dropdown mode for
//! showing a recent colors palette.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{ColorButton, ColorButtonPopupMode};
//! use horizon_lattice_render::Color;
//!
//! // Simple color button
//! let mut button = ColorButton::new()
//!     .with_color(Color::RED);
//!
//! button.clicked.connect(|&color| {
//!     println!("Button clicked, current color: {:?}", color);
//! });
//!
//! // Color button with dropdown arrow for recent colors palette
//! let mut button_with_dropdown = ColorButton::new()
//!     .with_color(Color::BLUE)
//!     .with_popup_mode(ColorButtonPopupMode::MenuButton);
//!
//! button_with_dropdown.dropdown_requested.connect(|()| {
//!     println!("Show recent colors palette");
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, RoundedRect, Stroke};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MousePressEvent, MouseReleaseEvent,
    PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

// ============================================================================
// ColorButtonPopupMode
// ============================================================================

/// Popup mode for color buttons.
///
/// Controls whether the color button shows a dropdown arrow for accessing
/// a recent colors palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorButtonPopupMode {
    /// No dropdown - clicking anywhere triggers the clicked signal.
    #[default]
    NoPopup,

    /// Show dropdown arrow - clicking the arrow shows the popup, clicking
    /// the main area triggers the clicked signal.
    MenuButton,

    /// Clicking anywhere shows the popup (dropdown_requested signal).
    InstantPopup,
}

// ============================================================================
// ColorButton
// ============================================================================

/// A button widget that displays a color swatch.
///
/// ColorButton shows the currently selected color and emits signals when clicked.
/// It is commonly used to open a color picker dialog or show a recent colors palette.
///
/// # Popup Modes
///
/// ColorButton supports different popup modes:
///
/// - **NoPopup** (default): Simple button, click emits `clicked` signal
/// - **MenuButton**: Split button with dropdown arrow, arrow shows popup
/// - **InstantPopup**: Any click shows the popup
///
/// # Signals
///
/// - `clicked(Color)`: Emitted when the button is clicked (not the dropdown)
/// - `color_changed(Color)`: Emitted when the color is changed programmatically
/// - `dropdown_requested()`: Emitted when the dropdown should be shown
pub struct ColorButton {
    /// Widget base.
    base: WidgetBase,

    /// Current color.
    color: Color,

    /// Whether alpha should be shown (checkerboard pattern behind color).
    show_alpha: bool,

    /// Border color.
    border_color: Color,

    /// Hover border color.
    hover_border_color: Color,

    /// Pressed state.
    pressed: bool,

    /// Hovered state.
    hovered: bool,

    /// Border radius for rounded corners.
    border_radius: f32,

    /// Popup mode.
    popup_mode: ColorButtonPopupMode,

    /// Width of the dropdown arrow area.
    arrow_width: f32,

    /// Whether the arrow area is hovered.
    arrow_hovered: bool,

    /// Whether the arrow area is pressed.
    arrow_pressed: bool,

    /// Signal emitted when the button is clicked.
    pub clicked: Signal<Color>,

    /// Signal emitted when the color changes.
    pub color_changed: Signal<Color>,

    /// Signal emitted when the dropdown should be shown.
    pub dropdown_requested: Signal<()>,
}

impl ColorButton {
    /// Create a new color button with default (white) color.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Fixed));

        Self {
            base,
            color: Color::WHITE,
            show_alpha: true,
            border_color: Color::from_rgb8(180, 180, 180),
            hover_border_color: Color::from_rgb8(100, 100, 100),
            pressed: false,
            hovered: false,
            border_radius: 4.0,
            popup_mode: ColorButtonPopupMode::NoPopup,
            arrow_width: 14.0,
            arrow_hovered: false,
            arrow_pressed: false,
            clicked: Signal::new(),
            color_changed: Signal::new(),
            dropdown_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Color
    // =========================================================================

    /// Get the current color.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Set the current color.
    pub fn set_color(&mut self, color: Color) {
        if self.color != color {
            self.color = color;
            self.color_changed.emit(color);
            self.base.update();
        }
    }

    /// Set the color using builder pattern.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    // =========================================================================
    // Alpha Display
    // =========================================================================

    /// Get whether alpha is shown.
    pub fn show_alpha(&self) -> bool {
        self.show_alpha
    }

    /// Set whether to show alpha (checkerboard pattern).
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
    // Appearance
    // =========================================================================

    /// Get the border radius.
    pub fn border_radius(&self) -> f32 {
        self.border_radius
    }

    /// Set the border radius.
    pub fn set_border_radius(&mut self, radius: f32) {
        if self.border_radius != radius {
            self.border_radius = radius;
            self.base.update();
        }
    }

    /// Set border radius using builder pattern.
    pub fn with_border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    // =========================================================================
    // Popup Mode
    // =========================================================================

    /// Get the popup mode.
    pub fn popup_mode(&self) -> ColorButtonPopupMode {
        self.popup_mode
    }

    /// Set the popup mode.
    ///
    /// - `NoPopup`: Simple button, no dropdown arrow
    /// - `MenuButton`: Shows dropdown arrow, split button behavior
    /// - `InstantPopup`: Any click shows the popup
    pub fn set_popup_mode(&mut self, mode: ColorButtonPopupMode) {
        if self.popup_mode != mode {
            self.popup_mode = mode;
            self.base.update();
        }
    }

    /// Set popup mode using builder pattern.
    pub fn with_popup_mode(mut self, mode: ColorButtonPopupMode) -> Self {
        self.popup_mode = mode;
        self
    }

    /// Get the arrow area width.
    pub fn arrow_width(&self) -> f32 {
        self.arrow_width
    }

    /// Set the arrow area width.
    pub fn set_arrow_width(&mut self, width: f32) {
        if self.arrow_width != width {
            self.arrow_width = width;
            self.base.update();
        }
    }

    // =========================================================================
    // Geometry
    // =========================================================================

    /// Check if the button has a dropdown arrow.
    fn has_dropdown(&self) -> bool {
        self.popup_mode == ColorButtonPopupMode::MenuButton
    }

    /// Get the main color swatch area rectangle.
    fn swatch_rect(&self) -> Rect {
        let rect = self.base.rect();
        if self.has_dropdown() {
            Rect::new(0.0, 0.0, rect.width() - self.arrow_width, rect.height())
        } else {
            Rect::new(0.0, 0.0, rect.width(), rect.height())
        }
    }

    /// Get the dropdown arrow area rectangle.
    fn arrow_rect(&self) -> Option<Rect> {
        if !self.has_dropdown() {
            return None;
        }
        let rect = self.base.rect();
        Some(Rect::new(
            rect.width() - self.arrow_width,
            0.0,
            self.arrow_width,
            rect.height(),
        ))
    }

    /// Check if a point is in the arrow area.
    fn is_in_arrow_area(&self, pos: Point) -> bool {
        self.arrow_rect().map(|r| r.contains(pos)).unwrap_or(false)
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // InstantPopup mode: any click shows popup
        if self.popup_mode == ColorButtonPopupMode::InstantPopup {
            self.dropdown_requested.emit(());
            return true;
        }

        // MenuButton mode: check if click is on arrow
        if self.has_dropdown() && self.is_in_arrow_area(event.local_pos) {
            self.arrow_pressed = true;
            self.base.update();
            return true;
        }

        // Normal click on swatch area
        self.pressed = true;
        self.base.update();
        true
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Handle arrow release
        if self.arrow_pressed {
            self.arrow_pressed = false;
            if self.is_in_arrow_area(event.local_pos) {
                self.dropdown_requested.emit(());
            }
            self.base.update();
            return true;
        }

        // Handle swatch release
        if self.pressed {
            self.pressed = false;
            self.base.update();

            // Check if release is within swatch bounds
            let swatch = self.swatch_rect();
            if swatch.contains(event.local_pos) {
                self.clicked.emit(self.color);
            }
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Space | Key::Enter => {
                if self.popup_mode == ColorButtonPopupMode::InstantPopup {
                    self.dropdown_requested.emit(());
                } else {
                    self.clicked.emit(self.color);
                }
                true
            }
            Key::ArrowDown if self.has_dropdown() => {
                // Down arrow opens dropdown
                self.dropdown_requested.emit(());
                true
            }
            _ => false,
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_checkerboard(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        let checker_size = 6.0;
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

    fn paint_dropdown_arrow(&self, ctx: &mut PaintContext<'_>) {
        let Some(arrow_rect) = self.arrow_rect() else {
            return;
        };

        // Background for arrow area on hover/press
        if self.arrow_pressed {
            ctx.renderer()
                .fill_rect(arrow_rect, Color::from_rgba8(0, 0, 0, 30));
        } else if self.arrow_hovered {
            ctx.renderer()
                .fill_rect(arrow_rect, Color::from_rgba8(0, 0, 0, 15));
        }

        // Draw separator line
        let sep_x = arrow_rect.left();
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(sep_x, arrow_rect.top() + 3.0),
            Point::new(sep_x, arrow_rect.bottom() - 3.0),
            &stroke,
        );

        // Draw chevron arrow
        let arrow_color = Color::from_rgb8(80, 80, 80);
        let center_x = arrow_rect.left() + arrow_rect.width() / 2.0;
        let center_y = arrow_rect.top() + arrow_rect.height() / 2.0;
        let arrow_size = 3.0;

        let points = [
            Point::new(center_x - arrow_size, center_y - arrow_size / 2.0),
            Point::new(center_x, center_y + arrow_size / 2.0),
            Point::new(center_x + arrow_size, center_y - arrow_size / 2.0),
        ];

        let arrow_stroke = Stroke::new(arrow_color, 1.5);
        ctx.renderer().draw_polyline(&points, &arrow_stroke);
    }
}

impl Default for ColorButton {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ColorButton {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ColorButton {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let base_width = 32.0;
        let width = if self.has_dropdown() {
            base_width + self.arrow_width
        } else {
            base_width
        };
        SizeHint::from_dimensions(width, 24.0).with_minimum_dimensions(24.0, 18.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        let swatch_rect = self.swatch_rect();
        let content_rect = swatch_rect.deflate(1.0);

        // Draw checkerboard if showing alpha and color has transparency
        if self.show_alpha && self.color.a < 1.0 {
            let rounded = RoundedRect::new(content_rect, self.border_radius.max(0.0));
            ctx.renderer().fill_rounded_rect(rounded, Color::WHITE);
            self.paint_checkerboard(ctx, content_rect);
        }

        // Draw the color swatch
        let rounded = RoundedRect::new(content_rect, self.border_radius.max(0.0));
        ctx.renderer().fill_rounded_rect(rounded, self.color);

        // Draw border around swatch
        let is_swatch_active = self.hovered && !self.arrow_hovered || self.pressed;
        let swatch_border_color = if is_swatch_active {
            self.hover_border_color
        } else {
            self.border_color
        };

        let stroke = Stroke::new(swatch_border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(swatch_rect, self.border_radius), &stroke);

        // Draw pressed effect on swatch
        if self.pressed {
            let overlay = Color::from_rgba(0.0, 0.0, 0.0, 0.1);
            ctx.renderer().fill_rounded_rect(rounded, overlay);
        }

        // Draw dropdown arrow if applicable
        if self.has_dropdown() {
            self.paint_dropdown_arrow(ctx);

            // Draw outer border around entire button
            let outer_stroke = Stroke::new(self.border_color, 1.0);
            ctx.renderer()
                .stroke_rounded_rect(RoundedRect::new(rect, self.border_radius), &outer_stroke);
        }

        // Draw focus indicator
        if self.base.has_focus() {
            let focus_rect = rect.inflate(2.0);
            let focus_stroke = Stroke::new(Color::from_rgba8(66, 133, 244, 128), 2.0);
            ctx.renderer().stroke_rounded_rect(
                RoundedRect::new(focus_rect, self.border_radius + 2.0),
                &focus_stroke,
            );
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
            WidgetEvent::MouseMove(e) => {
                // Track arrow hover state
                if self.has_dropdown() {
                    let new_arrow_hover = self.is_in_arrow_area(e.local_pos);
                    if new_arrow_hover != self.arrow_hovered {
                        self.arrow_hovered = new_arrow_hover;
                        self.base.update();
                    }
                }
            }
            WidgetEvent::Enter(_) => {
                self.hovered = true;
                self.base.update();
            }
            WidgetEvent::Leave(_) => {
                self.hovered = false;
                self.pressed = false;
                self.arrow_hovered = false;
                self.arrow_pressed = false;
                self.base.update();
            }
            _ => {}
        }
        false
    }
}

// Thread safety assertion
static_assertions::assert_impl_all!(ColorButton: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    };

    fn setup() {
        let _ = init_global_registry();
    }

    #[test]
    fn test_color_button_creation() {
        setup();
        let button = ColorButton::new();
        assert_eq!(button.color(), Color::WHITE);
        assert!(button.show_alpha());
        assert_eq!(button.popup_mode(), ColorButtonPopupMode::NoPopup);
    }

    #[test]
    fn test_color_button_builder() {
        setup();
        let button = ColorButton::new()
            .with_color(Color::RED)
            .with_show_alpha(false)
            .with_popup_mode(ColorButtonPopupMode::MenuButton);

        assert_eq!(button.color(), Color::RED);
        assert!(!button.show_alpha());
        assert_eq!(button.popup_mode(), ColorButtonPopupMode::MenuButton);
    }

    #[test]
    fn test_color_button_set_color() {
        setup();
        let mut button = ColorButton::new();

        let changed = Arc::new(Mutex::new(Color::TRANSPARENT));
        let changed_clone = changed.clone();

        button.color_changed.connect(move |color| {
            *changed_clone.lock().unwrap() = *color;
        });

        button.set_color(Color::BLUE);
        assert_eq!(button.color(), Color::BLUE);
        assert_eq!(*changed.lock().unwrap(), Color::BLUE);
    }

    #[test]
    fn test_color_button_clicked_signal() {
        setup();
        let button = ColorButton::new().with_color(Color::GREEN);

        let clicked_color = Arc::new(Mutex::new(Color::TRANSPARENT));
        let clicked_clone = clicked_color.clone();

        button.clicked.connect(move |color| {
            *clicked_clone.lock().unwrap() = *color;
        });

        button.clicked.emit(Color::GREEN);
        assert_eq!(*clicked_color.lock().unwrap(), Color::GREEN);
    }

    #[test]
    fn test_popup_mode_changes_size_hint() {
        setup();
        let button_no_popup = ColorButton::new();
        let button_menu = ColorButton::new().with_popup_mode(ColorButtonPopupMode::MenuButton);

        let hint_no_popup = button_no_popup.size_hint();
        let hint_menu = button_menu.size_hint();

        // MenuButton mode should be wider
        assert!(hint_menu.preferred.width > hint_no_popup.preferred.width);
    }

    #[test]
    fn test_dropdown_requested_signal() {
        setup();
        let button = ColorButton::new().with_popup_mode(ColorButtonPopupMode::MenuButton);

        let dropdown_called = Arc::new(AtomicBool::new(false));
        let dropdown_clone = dropdown_called.clone();

        button.dropdown_requested.connect(move |()| {
            dropdown_clone.store(true, Ordering::SeqCst);
        });

        button.dropdown_requested.emit(());
        assert!(dropdown_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_instant_popup_mode() {
        setup();
        let button = ColorButton::new().with_popup_mode(ColorButtonPopupMode::InstantPopup);
        assert_eq!(button.popup_mode(), ColorButtonPopupMode::InstantPopup);
        // In InstantPopup mode, clicking anywhere should request dropdown
        // (tested via signal emission in integration tests)
    }
}
