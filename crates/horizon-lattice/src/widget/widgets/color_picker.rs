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
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, RoundedRect, Stroke};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

/// Identifies which part of the picker is being dragged.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragTarget {
    None,
    SaturationValue,
    Hue,
    Alpha,
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

    /// Signal emitted when the color changes.
    pub color_changed: Signal<Color>,
}

impl ColorPicker {
    /// Create a new color picker with default (white) color.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Fixed));

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
    // Layout Calculations
    // =========================================================================

    /// Calculate the rectangle for the saturation/value square.
    fn sv_rect(&self) -> Rect {
        let rect = self.base.rect();
        let alpha_width = if self.show_alpha {
            self.alpha_bar_width + self.gap
        } else {
            0.0
        };
        let sv_width = rect.width() - self.hue_bar_width - self.gap - alpha_width;
        Rect::new(rect.left(), rect.top(), sv_width, rect.height())
    }

    /// Calculate the rectangle for the hue bar.
    fn hue_rect(&self) -> Rect {
        let rect = self.base.rect();
        let sv_rect = self.sv_rect();
        Rect::new(
            sv_rect.right() + self.gap,
            rect.top(),
            self.hue_bar_width,
            rect.height(),
        )
    }

    /// Calculate the rectangle for the alpha bar.
    fn alpha_rect(&self) -> Rect {
        let rect = self.base.rect();
        let hue_rect = self.hue_rect();
        Rect::new(
            hue_rect.right() + self.gap,
            rect.top(),
            self.alpha_bar_width,
            rect.height(),
        )
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    fn hit_test(&self, pos: Point) -> DragTarget {
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
            DragTarget::None => return,
        }
        self.base.update();
        self.color_changed.emit(self.color());
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
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
        ctx.renderer()
            .fill_circle(pos, radius, Color::WHITE);

        // Inner circle (black)
        ctx.renderer()
            .fill_circle(pos, radius - 2.0, Color::BLACK);

        // Current color circle
        ctx.renderer()
            .fill_circle(pos, radius - 3.0, self.color());
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
        let height = 200.0;

        SizeHint::from_dimensions(width, height).with_minimum_dimensions(150.0, 150.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_sv_square(ctx);
        self.paint_hue_bar(ctx);
        if self.show_alpha {
            self.paint_alpha_bar(ctx);
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
