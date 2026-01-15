//! Color button widget implementation.
//!
//! This module provides [`ColorButton`], a widget that displays a color swatch
//! and emits signals when clicked.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::ColorButton;
//! use horizon_lattice_render::Color;
//!
//! // Create a color button with red color
//! let mut button = ColorButton::new()
//!     .with_color(Color::RED);
//!
//! // Connect to clicked signal
//! button.clicked.connect(|&color| {
//!     println!("Button clicked, current color: {:?}", color);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Rect, Renderer, RoundedRect, Stroke};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MousePressEvent, MouseReleaseEvent,
    PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

/// A button widget that displays a color swatch.
///
/// ColorButton shows the currently selected color and emits signals when clicked.
/// It is commonly used to open a color picker dialog.
///
/// # Signals
///
/// - `clicked(Color)`: Emitted when the button is clicked, with the current color
/// - `color_changed(Color)`: Emitted when the color is changed programmatically
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

    /// Signal emitted when the button is clicked.
    pub clicked: Signal<Color>,

    /// Signal emitted when the color changes.
    pub color_changed: Signal<Color>,
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
            clicked: Signal::new(),
            color_changed: Signal::new(),
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
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button == MouseButton::Left {
            self.pressed = true;
            self.base.update();
            true
        } else {
            false
        }
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button == MouseButton::Left && self.pressed {
            self.pressed = false;
            self.base.update();

            // Check if release is within bounds
            let rect = self.base.rect();
            let local_pos = event.local_pos;
            if local_pos.x >= 0.0
                && local_pos.x < rect.width()
                && local_pos.y >= 0.0
                && local_pos.y < rect.height()
            {
                self.clicked.emit(self.color);
            }
            true
        } else {
            false
        }
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Space | Key::Enter => {
                self.clicked.emit(self.color);
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
        SizeHint::from_dimensions(32.0, 24.0).with_minimum_dimensions(24.0, 18.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let content_rect = rect.deflate(1.0);

        // Draw checkerboard if showing alpha and color has transparency
        if self.show_alpha && self.color.a < 1.0 {
            // Clip to rounded rect for checkerboard
            let rounded = RoundedRect::new(content_rect, self.border_radius.max(0.0));
            ctx.renderer().fill_rounded_rect(rounded, Color::WHITE);
            self.paint_checkerboard(ctx, content_rect);
        }

        // Draw the color swatch
        let rounded = RoundedRect::new(content_rect, self.border_radius.max(0.0));
        ctx.renderer().fill_rounded_rect(rounded, self.color);

        // Draw border
        let border_color = if self.hovered || self.pressed {
            self.hover_border_color
        } else {
            self.border_color
        };

        let stroke = Stroke::new(border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(RoundedRect::new(rect, self.border_radius), &stroke);

        // Draw pressed effect
        if self.pressed {
            let overlay = Color::from_rgba(0.0, 0.0, 0.0, 0.1);
            ctx.renderer().fill_rounded_rect(rounded, overlay);
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
            WidgetEvent::Enter(_) => {
                self.hovered = true;
                self.base.update();
            }
            WidgetEvent::Leave(_) => {
                self.hovered = false;
                self.pressed = false;
                self.base.update();
            }
            _ => {}
        }
        false
    }
}

// Thread safety assertion
static_assertions::assert_impl_all!(ColorButton: Send, Sync);
