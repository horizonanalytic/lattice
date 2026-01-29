//! Dial widget implementation.
//!
//! This module provides [`Dial`], a rotary control widget for selecting a value
//! from a range by rotating around a center point.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::Dial;
//!
//! // Create a dial
//! let mut dial = Dial::new()
//!     .with_range(0, 100)
//!     .with_value(50);
//!
//! // Connect to value changes
//! dial.value_changed.connect(|&value| {
//!     println!("Value: {}", value);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, RoundedRect, Stroke};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, WheelEvent, Widget,
    WidgetBase, WidgetEvent,
};

/// Start angle in radians (7 o'clock position, 225 degrees from 3 o'clock).
const START_ANGLE: f32 = std::f32::consts::PI * 1.25;
/// End angle in radians (5 o'clock position, -45 degrees or 315 degrees from 3 o'clock).
const END_ANGLE: f32 = -std::f32::consts::PI * 0.25;
/// Total arc span (270 degrees in radians).
const ARC_SPAN: f32 = std::f32::consts::PI * 1.5;

/// A dial widget for selecting a value from a range.
///
/// Dial provides a visual and interactive rotary control to select a value.
/// The user can drag around the center of the dial to change the value, or
/// use keyboard navigation.
///
/// # Signals
///
/// - `value_changed(i32)`: Emitted when the value changes
pub struct Dial {
    /// Widget base.
    base: WidgetBase,

    /// Minimum value.
    minimum: i32,

    /// Maximum value.
    maximum: i32,

    /// Current value.
    value: i32,

    /// Single step size (for arrow keys).
    single_step: i32,

    /// Page step size (for Page Up/Down).
    page_step: i32,

    /// Whether wrapping is enabled.
    ///
    /// When true, dragging past the maximum wraps to minimum and vice versa.
    wrapping: bool,

    /// Whether notches (tick marks) are visible.
    notches_visible: bool,

    /// Target number of notches around the dial.
    ///
    /// The actual number of notches is adjusted based on the range and step.
    /// Set to 0 to use single_step for notch interval.
    notch_target: i32,

    /// Whether the dial is currently being dragged.
    dragging: bool,

    /// Last angle during drag (in radians).
    last_drag_angle: f32,

    /// Dial background color.
    dial_color: Color,

    /// Dial hover color.
    dial_hover_color: Color,

    /// Dial pressed color.
    dial_pressed_color: Color,

    /// Notch color.
    notch_color: Color,

    /// Indicator (needle) color.
    indicator_color: Color,

    /// Dial outer radius (relative to widget size).
    dial_radius_ratio: f32,

    /// Notch length (pixels from edge toward center).
    notch_length: f32,

    /// Whether the dial is currently hovered.
    hovered: bool,

    /// Signal emitted when value changes.
    pub value_changed: Signal<i32>,
}

impl Dial {
    /// Create a new dial.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Preferred,
        ));

        Self {
            base,
            minimum: 0,
            maximum: 100,
            value: 0,
            single_step: 1,
            page_step: 10,
            wrapping: false,
            notches_visible: true,
            notch_target: 11, // Default: 11 notches for 0-100 range
            dragging: false,
            last_drag_angle: 0.0,
            dial_color: Color::from_rgb8(60, 60, 60),
            dial_hover_color: Color::from_rgb8(70, 70, 70),
            dial_pressed_color: Color::from_rgb8(50, 50, 50),
            notch_color: Color::from_rgb8(150, 150, 150),
            indicator_color: Color::from_rgb8(66, 133, 244), // Blue accent
            dial_radius_ratio: 0.9,
            notch_length: 8.0,
            hovered: false,
            value_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Value and Range
    // =========================================================================

    /// Get the minimum value.
    pub fn minimum(&self) -> i32 {
        self.minimum
    }

    /// Set the minimum value.
    pub fn set_minimum(&mut self, minimum: i32) {
        self.set_range(minimum, self.maximum);
    }

    /// Get the maximum value.
    pub fn maximum(&self) -> i32 {
        self.maximum
    }

    /// Set the maximum value.
    pub fn set_maximum(&mut self, maximum: i32) {
        self.set_range(self.minimum, maximum);
    }

    /// Get the current value.
    pub fn value(&self) -> i32 {
        self.value
    }

    /// Set the current value.
    ///
    /// The value is clamped to the valid range [minimum, maximum].
    pub fn set_value(&mut self, value: i32) {
        let clamped = value.clamp(self.minimum, self.maximum);
        if self.value != clamped {
            self.value = clamped;
            self.base.update();
            self.value_changed.emit(clamped);
        }
    }

    /// Set value using builder pattern.
    pub fn with_value(mut self, value: i32) -> Self {
        self.set_value(value);
        self
    }

    /// Set the value range.
    pub fn set_range(&mut self, minimum: i32, maximum: i32) {
        let (min, max) = if minimum <= maximum {
            (minimum, maximum)
        } else {
            (maximum, minimum)
        };

        if self.minimum != min || self.maximum != max {
            self.minimum = min;
            self.maximum = max;
            // Clamp current value to new range
            let new_value = self.value.clamp(min, max);
            let value_changed = self.value != new_value;
            self.value = new_value;
            self.base.update();
            if value_changed {
                self.value_changed.emit(new_value);
            }
        }
    }

    /// Set range using builder pattern.
    pub fn with_range(mut self, minimum: i32, maximum: i32) -> Self {
        self.set_range(minimum, maximum);
        self
    }

    // =========================================================================
    // Step Sizes
    // =========================================================================

    /// Get the single step size.
    pub fn single_step(&self) -> i32 {
        self.single_step
    }

    /// Set the single step size.
    pub fn set_single_step(&mut self, step: i32) {
        self.single_step = step.max(1);
    }

    /// Set single step using builder pattern.
    pub fn with_single_step(mut self, step: i32) -> Self {
        self.set_single_step(step);
        self
    }

    /// Get the page step size.
    pub fn page_step(&self) -> i32 {
        self.page_step
    }

    /// Set the page step size.
    pub fn set_page_step(&mut self, step: i32) {
        self.page_step = step.max(1);
    }

    /// Set page step using builder pattern.
    pub fn with_page_step(mut self, step: i32) -> Self {
        self.set_page_step(step);
        self
    }

    // =========================================================================
    // Wrapping
    // =========================================================================

    /// Get whether wrapping is enabled.
    pub fn wrapping(&self) -> bool {
        self.wrapping
    }

    /// Set whether wrapping is enabled.
    ///
    /// When wrapping is enabled, dragging past the maximum value wraps to the
    /// minimum and vice versa, creating a continuous rotation.
    pub fn set_wrapping(&mut self, wrapping: bool) {
        self.wrapping = wrapping;
    }

    /// Set wrapping using builder pattern.
    pub fn with_wrapping(mut self, wrapping: bool) -> Self {
        self.set_wrapping(wrapping);
        self
    }

    // =========================================================================
    // Notches
    // =========================================================================

    /// Get whether notches are visible.
    pub fn notches_visible(&self) -> bool {
        self.notches_visible
    }

    /// Set whether notches are visible.
    pub fn set_notches_visible(&mut self, visible: bool) {
        if self.notches_visible != visible {
            self.notches_visible = visible;
            self.base.update();
        }
    }

    /// Set notches visible using builder pattern.
    pub fn with_notches_visible(mut self, visible: bool) -> Self {
        self.set_notches_visible(visible);
        self
    }

    /// Get the target number of notches.
    pub fn notch_target(&self) -> i32 {
        self.notch_target
    }

    /// Set the target number of notches.
    ///
    /// Set to 0 to use single_step for notch interval instead.
    pub fn set_notch_target(&mut self, target: i32) {
        if self.notch_target != target {
            self.notch_target = target.max(0);
            self.base.update();
        }
    }

    /// Set notch target using builder pattern.
    pub fn with_notch_target(mut self, target: i32) -> Self {
        self.set_notch_target(target);
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the dial color.
    pub fn dial_color(&self) -> Color {
        self.dial_color
    }

    /// Set the dial color.
    pub fn set_dial_color(&mut self, color: Color) {
        if self.dial_color != color {
            self.dial_color = color;
            self.base.update();
        }
    }

    /// Set dial color using builder pattern.
    pub fn with_dial_color(mut self, color: Color) -> Self {
        self.dial_color = color;
        self
    }

    /// Get the indicator color.
    pub fn indicator_color(&self) -> Color {
        self.indicator_color
    }

    /// Set the indicator color.
    pub fn set_indicator_color(&mut self, color: Color) {
        if self.indicator_color != color {
            self.indicator_color = color;
            self.base.update();
        }
    }

    /// Set indicator color using builder pattern.
    pub fn with_indicator_color(mut self, color: Color) -> Self {
        self.indicator_color = color;
        self
    }

    /// Get the notch color.
    pub fn notch_color(&self) -> Color {
        self.notch_color
    }

    /// Set the notch color.
    pub fn set_notch_color(&mut self, color: Color) {
        if self.notch_color != color {
            self.notch_color = color;
            self.base.update();
        }
    }

    /// Set notch color using builder pattern.
    pub fn with_notch_color(mut self, color: Color) -> Self {
        self.notch_color = color;
        self
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    /// Get the center point of the dial.
    fn dial_center(&self) -> Point {
        let rect = self.base.rect();
        Point::new(rect.width() / 2.0, rect.height() / 2.0)
    }

    /// Get the radius of the dial.
    fn dial_radius(&self) -> f32 {
        let rect = self.base.rect();
        let size = rect.width().min(rect.height());
        (size / 2.0) * self.dial_radius_ratio
    }

    /// Get the position ratio (0.0 to 1.0) for the current value.
    fn value_ratio(&self) -> f32 {
        let range = (self.maximum - self.minimum) as f32;
        if range <= 0.0 {
            return 0.0;
        }
        (self.value - self.minimum) as f32 / range
    }

    /// Convert value ratio to angle in radians.
    fn ratio_to_angle(&self, ratio: f32) -> f32 {
        // Map 0.0-1.0 to START_ANGLE to END_ANGLE
        // Start at 225° (7 o'clock) and go clockwise to 315° (5 o'clock)
        START_ANGLE - ratio * ARC_SPAN
    }

    /// Convert an angle in radians to value ratio.
    fn angle_to_ratio(&self, angle: f32) -> f32 {
        // Inverse of ratio_to_angle
        let ratio = (START_ANGLE - angle) / ARC_SPAN;
        ratio.clamp(0.0, 1.0)
    }

    /// Convert a point (relative to dial center) to angle in radians.
    fn point_to_angle(&self, center: Point, pos: Point) -> f32 {
        let dx = pos.x - center.x;
        let dy = pos.y - center.y;
        // atan2 returns angle from positive x-axis, counter-clockwise
        // We need to adjust for our coordinate system (y increases downward)
        (-dy).atan2(dx)
    }

    /// Get the angle for the current value.
    fn current_angle(&self) -> f32 {
        self.ratio_to_angle(self.value_ratio())
    }

    /// Check if a point is inside the dial face.
    fn hit_test_dial(&self, pos: Point) -> bool {
        let center = self.dial_center();
        let radius = self.dial_radius();
        let dx = pos.x - center.x;
        let dy = pos.y - center.y;
        let distance = (dx * dx + dy * dy).sqrt();
        distance <= radius + 5.0 // Small tolerance
    }

    /// Get the notch interval in value units.
    fn notch_interval(&self) -> i32 {
        if self.notch_target > 0 {
            let range = self.maximum - self.minimum;
            if range > 0 {
                ((range as f32) / (self.notch_target as f32 - 1.0).max(1.0)).ceil() as i32
            } else {
                1
            }
        } else {
            self.single_step
        }
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.hit_test_dial(event.local_pos) {
            self.dragging = true;
            let center = self.dial_center();
            self.last_drag_angle = self.point_to_angle(center, event.local_pos);
            self.base.update();
            true
        } else {
            false
        }
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.dragging {
            self.dragging = false;
            self.base.update();
            return true;
        }
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        // Update hover state
        let new_hover = self.hit_test_dial(event.local_pos);
        if self.hovered != new_hover {
            self.hovered = new_hover;
            self.base.update();
        }

        if self.dragging {
            let center = self.dial_center();
            let current_angle = self.point_to_angle(center, event.local_pos);

            // Calculate angle delta
            let mut angle_delta = current_angle - self.last_drag_angle;

            // Handle angle wrap-around (when crossing from +π to -π or vice versa)
            if angle_delta > std::f32::consts::PI {
                angle_delta -= 2.0 * std::f32::consts::PI;
            } else if angle_delta < -std::f32::consts::PI {
                angle_delta += 2.0 * std::f32::consts::PI;
            }

            // Convert angle delta to value delta
            let range = (self.maximum - self.minimum) as f32;
            // Negative because we rotate counter-clockwise for increasing value
            let value_delta = -(angle_delta / ARC_SPAN) * range;

            let new_value = if self.wrapping {
                // With wrapping, wrap around the range
                let raw_value = self.value as f32 + value_delta;
                let range_size = self.maximum - self.minimum + 1;
                let mut wrapped = ((raw_value - self.minimum as f32).rem_euclid(range_size as f32))
                    as i32
                    + self.minimum;
                if wrapped > self.maximum {
                    wrapped = self.minimum + (wrapped - self.maximum - 1);
                }
                wrapped
            } else {
                // Without wrapping, clamp to range
                (self.value as f32 + value_delta).round() as i32
            }
            .clamp(self.minimum, self.maximum);

            if new_value != self.value {
                self.value = new_value;
                self.base.update();
                self.value_changed.emit(new_value);
            }

            self.last_drag_angle = current_angle;
            return true;
        }

        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        // Use vertical scroll for value change
        let delta = -event.delta_y;

        if delta.abs() > 0.0 {
            let steps = (delta / 120.0).round() as i32;
            let new_value = if self.wrapping {
                let raw = self.value + steps * self.single_step;
                let range_size = self.maximum - self.minimum + 1;
                ((raw - self.minimum).rem_euclid(range_size)) + self.minimum
            } else {
                (self.value + steps * self.single_step).clamp(self.minimum, self.maximum)
            };
            if new_value != self.value {
                self.value = new_value;
                self.base.update();
                self.value_changed.emit(new_value);
            }
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let step_value = |current: i32, delta: i32, wrapping: bool, min: i32, max: i32| -> i32 {
            if wrapping {
                let range_size = max - min + 1;
                ((current - min + delta).rem_euclid(range_size)) + min
            } else {
                (current + delta).clamp(min, max)
            }
        };

        match event.key {
            Key::ArrowLeft | Key::ArrowDown => {
                let new_value = step_value(
                    self.value,
                    -self.single_step,
                    self.wrapping,
                    self.minimum,
                    self.maximum,
                );
                if new_value != self.value {
                    self.value = new_value;
                    self.base.update();
                    self.value_changed.emit(new_value);
                }
                true
            }
            Key::ArrowRight | Key::ArrowUp => {
                let new_value = step_value(
                    self.value,
                    self.single_step,
                    self.wrapping,
                    self.minimum,
                    self.maximum,
                );
                if new_value != self.value {
                    self.value = new_value;
                    self.base.update();
                    self.value_changed.emit(new_value);
                }
                true
            }
            Key::PageUp => {
                let new_value = step_value(
                    self.value,
                    self.page_step,
                    self.wrapping,
                    self.minimum,
                    self.maximum,
                );
                if new_value != self.value {
                    self.value = new_value;
                    self.base.update();
                    self.value_changed.emit(new_value);
                }
                true
            }
            Key::PageDown => {
                let new_value = step_value(
                    self.value,
                    -self.page_step,
                    self.wrapping,
                    self.minimum,
                    self.maximum,
                );
                if new_value != self.value {
                    self.value = new_value;
                    self.base.update();
                    self.value_changed.emit(new_value);
                }
                true
            }
            Key::Home if !self.wrapping => {
                if self.value != self.minimum {
                    self.value = self.minimum;
                    self.base.update();
                    self.value_changed.emit(self.minimum);
                }
                true
            }
            Key::End if !self.wrapping => {
                if self.value != self.maximum {
                    self.value = self.maximum;
                    self.base.update();
                    self.value_changed.emit(self.maximum);
                }
                true
            }
            _ => false,
        }
    }

    fn handle_leave(&mut self) -> bool {
        if self.hovered {
            self.hovered = false;
            self.base.update();
        }
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_dial_face(&self, ctx: &mut PaintContext<'_>) {
        let center = self.dial_center();
        let radius = self.dial_radius();

        // Choose color based on state
        let color = if self.dragging {
            self.dial_pressed_color
        } else if self.hovered {
            self.dial_hover_color
        } else {
            self.dial_color
        };

        // Draw outer shadow
        let shadow_color = Color::from_rgba8(0, 0, 0, 50);
        let shadow_rect = Rect::new(
            center.x - radius + 2.0,
            center.y - radius + 3.0,
            radius * 2.0,
            radius * 2.0,
        );
        let shadow_rrect = RoundedRect::new(shadow_rect, radius);
        ctx.renderer().fill_rounded_rect(shadow_rrect, shadow_color);

        // Draw dial face
        let dial_rect = Rect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        );
        let dial_rrect = RoundedRect::new(dial_rect, radius);
        ctx.renderer().fill_rounded_rect(dial_rrect, color);

        // Draw subtle border
        let border_color = Color::from_rgb8(80, 80, 80);
        let border_stroke = Stroke::new(border_color, 1.0);
        ctx.renderer()
            .stroke_rounded_rect(dial_rrect, &border_stroke);
    }

    fn paint_notches(&self, ctx: &mut PaintContext<'_>) {
        if !self.notches_visible {
            return;
        }

        let center = self.dial_center();
        let radius = self.dial_radius();
        let range = self.maximum - self.minimum;
        if range <= 0 {
            return;
        }

        let interval = self.notch_interval();
        if interval <= 0 {
            return;
        }

        let stroke = Stroke::new(self.notch_color, 1.5);
        let inner_radius = radius - self.notch_length;

        let mut value = self.minimum;
        while value <= self.maximum {
            let ratio = (value - self.minimum) as f32 / range as f32;
            let angle = self.ratio_to_angle(ratio);

            // Calculate line endpoints
            let cos_a = angle.cos();
            let sin_a = angle.sin();

            let outer_x = center.x + radius * cos_a;
            let outer_y = center.y - radius * sin_a; // Negative because y increases downward
            let inner_x = center.x + inner_radius * cos_a;
            let inner_y = center.y - inner_radius * sin_a;

            ctx.renderer().draw_line(
                Point::new(inner_x, inner_y),
                Point::new(outer_x, outer_y),
                &stroke,
            );

            value += interval;
        }
    }

    fn paint_indicator(&self, ctx: &mut PaintContext<'_>) {
        let center = self.dial_center();
        let radius = self.dial_radius();
        let angle = self.current_angle();

        // Calculate indicator position (from center toward edge)
        let cos_a = angle.cos();
        let sin_a = angle.sin();

        let inner_radius = radius * 0.25;
        let outer_radius = radius * 0.75;

        let inner_x = center.x + inner_radius * cos_a;
        let inner_y = center.y - inner_radius * sin_a;
        let outer_x = center.x + outer_radius * cos_a;
        let outer_y = center.y - outer_radius * sin_a;

        // Draw indicator line
        let stroke = Stroke::new(self.indicator_color, 3.0);
        ctx.renderer().draw_line(
            Point::new(inner_x, inner_y),
            Point::new(outer_x, outer_y),
            &stroke,
        );

        // Draw indicator dot at the end
        let dot_radius = 4.0;
        let dot_rect = Rect::new(
            outer_x - dot_radius,
            outer_y - dot_radius,
            dot_radius * 2.0,
            dot_radius * 2.0,
        );
        let dot_rrect = RoundedRect::new(dot_rect, dot_radius);
        ctx.renderer()
            .fill_rounded_rect(dot_rrect, self.indicator_color);
    }

    fn paint_center_cap(&self, ctx: &mut PaintContext<'_>) {
        let center = self.dial_center();
        let cap_radius = self.dial_radius() * 0.15;

        // Draw center cap
        let cap_rect = Rect::new(
            center.x - cap_radius,
            center.y - cap_radius,
            cap_radius * 2.0,
            cap_radius * 2.0,
        );
        let cap_rrect = RoundedRect::new(cap_rect, cap_radius);
        let cap_color = Color::from_rgb8(90, 90, 90);
        ctx.renderer().fill_rounded_rect(cap_rrect, cap_color);
    }

    fn paint_focus_indicator(&self, ctx: &mut PaintContext<'_>) {
        if !self.base.has_focus() {
            return;
        }

        let center = self.dial_center();
        let radius = self.dial_radius() + 4.0;

        let focus_rect = Rect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        );
        let focus_rrect = RoundedRect::new(focus_rect, radius);
        let focus_color = Color::from_rgba8(66, 133, 244, 100);
        let focus_stroke = Stroke::new(focus_color, 2.0);
        ctx.renderer()
            .stroke_rounded_rect(focus_rrect, &focus_stroke);
    }
}

impl Default for Dial {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for Dial {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Dial {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Dial is square, prefer 80x80 default size
        SizeHint::from_dimensions(80.0, 80.0).with_minimum_dimensions(40.0, 40.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_dial_face(ctx);
        self.paint_notches(ctx);
        self.paint_indicator(ctx);
        self.paint_center_cap(ctx);
        self.paint_focus_indicator(ctx);
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
            WidgetEvent::MouseMove(e) => {
                if self.handle_mouse_move(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::Wheel(e) => {
                if self.handle_wheel(e) {
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
                self.handle_leave();
            }
            _ => {}
        }
        false
    }
}

// Ensure Dial is Send + Sync
static_assertions::assert_impl_all!(Dial: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_dial_creation() {
        setup();
        let dial = Dial::new();
        assert_eq!(dial.minimum(), 0);
        assert_eq!(dial.maximum(), 100);
        assert_eq!(dial.value(), 0);
        assert_eq!(dial.single_step(), 1);
        assert_eq!(dial.page_step(), 10);
        assert!(!dial.wrapping());
        assert!(dial.notches_visible());
    }

    #[test]
    fn test_dial_builder_pattern() {
        setup();
        let dial = Dial::new()
            .with_range(0, 360)
            .with_value(180)
            .with_single_step(5)
            .with_page_step(45)
            .with_wrapping(true)
            .with_notches_visible(true)
            .with_notch_target(9);

        assert_eq!(dial.minimum(), 0);
        assert_eq!(dial.maximum(), 360);
        assert_eq!(dial.value(), 180);
        assert_eq!(dial.single_step(), 5);
        assert_eq!(dial.page_step(), 45);
        assert!(dial.wrapping());
        assert!(dial.notches_visible());
        assert_eq!(dial.notch_target(), 9);
    }

    #[test]
    fn test_value_clamping() {
        setup();
        let mut dial = Dial::new().with_range(0, 100);

        dial.set_value(-10);
        assert_eq!(dial.value(), 0);

        dial.set_value(150);
        assert_eq!(dial.value(), 100);
    }

    #[test]
    fn test_value_changed_signal() {
        setup();
        let mut dial = Dial::new();
        let last_value = Arc::new(AtomicI32::new(-1));
        let last_value_clone = last_value.clone();

        dial.value_changed.connect(move |&value| {
            last_value_clone.store(value, Ordering::SeqCst);
        });

        dial.set_value(42);
        assert_eq!(last_value.load(Ordering::SeqCst), 42);

        dial.set_value(75);
        assert_eq!(last_value.load(Ordering::SeqCst), 75);
    }

    #[test]
    fn test_no_signal_for_same_value() {
        setup();
        let mut dial = Dial::new().with_value(50);
        let signal_count = Arc::new(AtomicI32::new(0));
        let signal_count_clone = signal_count.clone();

        dial.value_changed.connect(move |_| {
            signal_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        dial.set_value(50);
        assert_eq!(signal_count.load(Ordering::SeqCst), 0);

        dial.set_value(51);
        assert_eq!(signal_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_range_change_clamps_value() {
        setup();
        let mut dial = Dial::new().with_range(0, 100).with_value(50);

        dial.set_range(0, 25);
        assert_eq!(dial.value(), 25); // Clamped to new max
    }

    #[test]
    fn test_size_hint() {
        setup();
        let dial = Dial::new();
        let hint = dial.size_hint();
        // Dial should be square
        assert_eq!(hint.preferred.width, hint.preferred.height);
        let min = hint.minimum.expect("minimum size should be set");
        assert!(min.width >= 40.0);
        assert!(min.height >= 40.0);
    }

    #[test]
    fn test_notch_interval() {
        setup();
        // With notch_target = 11 and range 0-100, interval should be ~10
        let dial = Dial::new().with_range(0, 100).with_notch_target(11);
        let interval = dial.notch_interval();
        assert_eq!(interval, 10);

        // With notch_target = 0, should use single_step
        let dial = Dial::new()
            .with_range(0, 100)
            .with_notch_target(0)
            .with_single_step(5);
        assert_eq!(dial.notch_interval(), 5);
    }

    #[test]
    fn test_angle_conversions() {
        setup();
        let dial = Dial::new().with_range(0, 100);

        // At minimum (ratio 0), angle should be START_ANGLE
        let angle_at_min = dial.ratio_to_angle(0.0);
        assert!((angle_at_min - START_ANGLE).abs() < 0.001);

        // At maximum (ratio 1), angle should be END_ANGLE
        let angle_at_max = dial.ratio_to_angle(1.0);
        assert!((angle_at_max - END_ANGLE).abs() < 0.001);

        // Round trip: ratio -> angle -> ratio
        let test_ratio = 0.5;
        let angle = dial.ratio_to_angle(test_ratio);
        let recovered_ratio = dial.angle_to_ratio(angle);
        assert!((recovered_ratio - test_ratio).abs() < 0.001);
    }
}
