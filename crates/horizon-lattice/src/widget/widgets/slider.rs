//! Slider widget implementation.
//!
//! This module provides [`Slider`], a widget for selecting a value from a range
//! by dragging a thumb along a track.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Slider, Orientation};
//!
//! // Create a horizontal slider
//! let mut slider = Slider::new(Orientation::Horizontal)
//!     .with_range(0, 100)
//!     .with_value(50);
//!
//! // Connect to value changes
//! slider.value_changed.connect(|&value| {
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

use super::Orientation;

/// Position of tick marks relative to the slider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TickPosition {
    /// No tick marks.
    #[default]
    NoTicks,
    /// Tick marks above (horizontal) or left of (vertical) the slider.
    TicksAbove,
    /// Tick marks below (horizontal) or right of (vertical) the slider.
    TicksBelow,
    /// Tick marks on both sides.
    TicksBothSides,
}

/// A slider widget for selecting a value from a range.
///
/// Slider provides a visual and interactive way to select a value by dragging
/// a thumb along a track. It supports both horizontal and vertical orientations,
/// tick marks, and keyboard navigation.
///
/// # Signals
///
/// - `value_changed(i32)`: Emitted when the value changes (including during drag)
/// - `slider_pressed()`: Emitted when the thumb is pressed
/// - `slider_released()`: Emitted when the thumb is released
/// - `slider_moved(i32)`: Emitted while the slider is being dragged
pub struct Slider {
    /// Widget base.
    base: WidgetBase,

    /// Slider orientation.
    orientation: Orientation,

    /// Minimum value.
    minimum: i32,

    /// Maximum value.
    maximum: i32,

    /// Current value.
    value: i32,

    /// Single step size (for arrow keys).
    single_step: i32,

    /// Page step size (for Page Up/Down and clicking track).
    page_step: i32,

    /// Whether the thumb is currently being dragged.
    dragging: bool,

    /// Drag start position (in widget coordinates).
    drag_start_pos: f32,

    /// Value when drag started.
    drag_start_value: i32,

    /// Track color.
    track_color: Color,

    /// Track fill color (portion of track before thumb).
    track_fill_color: Color,

    /// Thumb color.
    thumb_color: Color,

    /// Thumb hover color.
    thumb_hover_color: Color,

    /// Thumb pressed color.
    thumb_pressed_color: Color,

    /// Tick color.
    tick_color: Color,

    /// Thumb size (diameter).
    thumb_size: f32,

    /// Track height/width (thickness).
    track_thickness: f32,

    /// Border radius for track.
    border_radius: f32,

    /// Tick mark position.
    tick_position: TickPosition,

    /// Tick interval (0 = use single_step).
    tick_interval: i32,

    /// Which part is currently hovered.
    hover_thumb: bool,

    /// Signal emitted when value changes.
    pub value_changed: Signal<i32>,

    /// Signal emitted when thumb is pressed.
    pub slider_pressed: Signal<()>,

    /// Signal emitted when thumb is released.
    pub slider_released: Signal<()>,

    /// Signal emitted while slider is being dragged.
    pub slider_moved: Signal<i32>,

    /// Signal emitted when range changes.
    pub range_changed: Signal<(i32, i32)>,
}

impl Slider {
    /// Create a new slider with the given orientation.
    pub fn new(orientation: Orientation) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);

        // Set size policy based on orientation
        let policy = match orientation {
            Orientation::Horizontal => {
                SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Fixed)
            }
            Orientation::Vertical => {
                SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Expanding)
            }
        };
        base.set_size_policy(policy);

        Self {
            base,
            orientation,
            minimum: 0,
            maximum: 100,
            value: 0,
            single_step: 1,
            page_step: 10,
            dragging: false,
            drag_start_pos: 0.0,
            drag_start_value: 0,
            track_color: Color::from_rgb8(200, 200, 200),
            track_fill_color: Color::from_rgb8(66, 133, 244), // Blue accent
            thumb_color: Color::from_rgb8(255, 255, 255),
            thumb_hover_color: Color::from_rgb8(245, 245, 245),
            thumb_pressed_color: Color::from_rgb8(230, 230, 230),
            tick_color: Color::from_rgb8(150, 150, 150),
            thumb_size: 18.0,
            track_thickness: 4.0,
            border_radius: 2.0,
            tick_position: TickPosition::NoTicks,
            tick_interval: 0,
            hover_thumb: false,
            value_changed: Signal::new(),
            slider_pressed: Signal::new(),
            slider_released: Signal::new(),
            slider_moved: Signal::new(),
            range_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Orientation
    // =========================================================================

    /// Get the orientation.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Set the orientation.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        if self.orientation != orientation {
            self.orientation = orientation;
            let policy = match orientation {
                Orientation::Horizontal => {
                    SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Fixed)
                }
                Orientation::Vertical => {
                    SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Expanding)
                }
            };
            self.base.set_size_policy(policy);
            self.base.update();
        }
    }

    /// Set orientation using builder pattern.
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.set_orientation(orientation);
        self
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
            self.range_changed.emit((min, max));
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
    // Tick Marks
    // =========================================================================

    /// Get the tick position.
    pub fn tick_position(&self) -> TickPosition {
        self.tick_position
    }

    /// Set the tick position.
    pub fn set_tick_position(&mut self, position: TickPosition) {
        if self.tick_position != position {
            self.tick_position = position;
            self.base.update();
        }
    }

    /// Set tick position using builder pattern.
    pub fn with_tick_position(mut self, position: TickPosition) -> Self {
        self.set_tick_position(position);
        self
    }

    /// Get the tick interval.
    pub fn tick_interval(&self) -> i32 {
        self.tick_interval
    }

    /// Set the tick interval.
    ///
    /// If set to 0, the single_step value is used.
    pub fn set_tick_interval(&mut self, interval: i32) {
        let new_interval = interval.max(0);
        if self.tick_interval != new_interval {
            self.tick_interval = new_interval;
            self.base.update();
        }
    }

    /// Set tick interval using builder pattern.
    pub fn with_tick_interval(mut self, interval: i32) -> Self {
        self.set_tick_interval(interval);
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the track color.
    pub fn track_color(&self) -> Color {
        self.track_color
    }

    /// Set the track color.
    pub fn set_track_color(&mut self, color: Color) {
        if self.track_color != color {
            self.track_color = color;
            self.base.update();
        }
    }

    /// Set track color using builder pattern.
    pub fn with_track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self
    }

    /// Get the track fill color.
    pub fn track_fill_color(&self) -> Color {
        self.track_fill_color
    }

    /// Set the track fill color (portion before thumb).
    pub fn set_track_fill_color(&mut self, color: Color) {
        if self.track_fill_color != color {
            self.track_fill_color = color;
            self.base.update();
        }
    }

    /// Set track fill color using builder pattern.
    pub fn with_track_fill_color(mut self, color: Color) -> Self {
        self.track_fill_color = color;
        self
    }

    /// Get the thumb color.
    pub fn thumb_color(&self) -> Color {
        self.thumb_color
    }

    /// Set the thumb color.
    pub fn set_thumb_color(&mut self, color: Color) {
        if self.thumb_color != color {
            self.thumb_color = color;
            self.base.update();
        }
    }

    /// Set thumb color using builder pattern.
    pub fn with_thumb_color(mut self, color: Color) -> Self {
        self.thumb_color = color;
        self
    }

    /// Get the thumb size.
    pub fn thumb_size(&self) -> f32 {
        self.thumb_size
    }

    /// Set the thumb size (diameter).
    pub fn set_thumb_size(&mut self, size: f32) {
        let new_size = size.max(8.0);
        if (self.thumb_size - new_size).abs() > f32::EPSILON {
            self.thumb_size = new_size;
            self.base.update();
        }
    }

    /// Set thumb size using builder pattern.
    pub fn with_thumb_size(mut self, size: f32) -> Self {
        self.set_thumb_size(size);
        self
    }

    /// Get the track thickness.
    pub fn track_thickness(&self) -> f32 {
        self.track_thickness
    }

    /// Set the track thickness.
    pub fn set_track_thickness(&mut self, thickness: f32) {
        let new_thickness = thickness.max(1.0);
        if (self.track_thickness - new_thickness).abs() > f32::EPSILON {
            self.track_thickness = new_thickness;
            self.base.update();
        }
    }

    /// Set track thickness using builder pattern.
    pub fn with_track_thickness(mut self, thickness: f32) -> Self {
        self.set_track_thickness(thickness);
        self
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    /// Get the track rectangle.
    fn track_rect(&self) -> Rect {
        let rect = self.base.rect();
        let half_thumb = self.thumb_size / 2.0;

        match self.orientation {
            Orientation::Horizontal => {
                let center_y = rect.height() / 2.0;
                Rect::new(
                    half_thumb,
                    center_y - self.track_thickness / 2.0,
                    rect.width() - self.thumb_size,
                    self.track_thickness,
                )
            }
            Orientation::Vertical => {
                let center_x = rect.width() / 2.0;
                Rect::new(
                    center_x - self.track_thickness / 2.0,
                    half_thumb,
                    self.track_thickness,
                    rect.height() - self.thumb_size,
                )
            }
        }
    }

    /// Get the position ratio (0.0 to 1.0) for the current value.
    fn value_ratio(&self) -> f32 {
        let range = (self.maximum - self.minimum) as f32;
        if range <= 0.0 {
            return 0.0;
        }
        (self.value - self.minimum) as f32 / range
    }

    /// Get the thumb center position.
    fn thumb_center(&self) -> Point {
        let track = self.track_rect();
        let ratio = self.value_ratio();

        match self.orientation {
            Orientation::Horizontal => {
                let x = track.origin.x + ratio * track.width();
                let y = track.origin.y + track.height() / 2.0;
                Point::new(x, y)
            }
            Orientation::Vertical => {
                let x = track.origin.x + track.width() / 2.0;
                let y = track.origin.y + ratio * track.height();
                Point::new(x, y)
            }
        }
    }

    /// Get the thumb rectangle.
    fn thumb_rect(&self) -> Rect {
        let center = self.thumb_center();
        let half_size = self.thumb_size / 2.0;
        Rect::new(
            center.x - half_size,
            center.y - half_size,
            self.thumb_size,
            self.thumb_size,
        )
    }

    /// Check if a point is inside the thumb.
    fn hit_test_thumb(&self, pos: Point) -> bool {
        let center = self.thumb_center();
        let dx = pos.x - center.x;
        let dy = pos.y - center.y;
        let distance_sq = dx * dx + dy * dy;
        let radius = self.thumb_size / 2.0 + 4.0; // Add tolerance
        distance_sq <= radius * radius
    }

    /// Convert a position to a value.
    fn position_to_value(&self, pos: Point) -> i32 {
        let track = self.track_rect();
        let range = (self.maximum - self.minimum) as f32;

        if range <= 0.0 {
            return self.minimum;
        }

        let ratio = match self.orientation {
            Orientation::Horizontal => {
                let relative = pos.x - track.origin.x;
                (relative / track.width()).clamp(0.0, 1.0)
            }
            Orientation::Vertical => {
                let relative = pos.y - track.origin.y;
                (relative / track.height()).clamp(0.0, 1.0)
            }
        };

        self.minimum + (ratio * range).round() as i32
    }

    /// Get the effective tick interval.
    fn effective_tick_interval(&self) -> i32 {
        if self.tick_interval > 0 {
            self.tick_interval
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

        if self.hit_test_thumb(event.local_pos) {
            // Start dragging the thumb
            self.dragging = true;
            self.drag_start_pos = match self.orientation {
                Orientation::Horizontal => event.local_pos.x,
                Orientation::Vertical => event.local_pos.y,
            };
            self.drag_start_value = self.value;
            self.slider_pressed.emit(());
            self.base.update();
            true
        } else {
            // Click on track - jump to position or page step
            let track = self.track_rect();
            if track.contains(event.local_pos) {
                let thumb_center = self.thumb_center();
                let click_before = match self.orientation {
                    Orientation::Horizontal => event.local_pos.x < thumb_center.x,
                    Orientation::Vertical => event.local_pos.y < thumb_center.y,
                };

                // Page step towards click position
                if click_before {
                    self.set_value(self.value - self.page_step);
                } else {
                    self.set_value(self.value + self.page_step);
                }
                true
            } else {
                false
            }
        }
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.dragging {
            self.dragging = false;
            self.slider_released.emit(());
            self.base.update();
            return true;
        }
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        // Update hover state
        let new_hover = self.hit_test_thumb(event.local_pos);
        if self.hover_thumb != new_hover {
            self.hover_thumb = new_hover;
            self.base.update();
        }

        // Handle dragging
        if self.dragging {
            let current_pos = match self.orientation {
                Orientation::Horizontal => event.local_pos.x,
                Orientation::Vertical => event.local_pos.y,
            };

            let track = self.track_rect();
            let range = (self.maximum - self.minimum) as f32;

            let track_length = match self.orientation {
                Orientation::Horizontal => track.width(),
                Orientation::Vertical => track.height(),
            };

            if track_length > 0.0 && range > 0.0 {
                let delta_pos = current_pos - self.drag_start_pos;
                let delta_value = (delta_pos / track_length * range).round() as i32;
                let new_value = (self.drag_start_value + delta_value).clamp(self.minimum, self.maximum);

                if new_value != self.value {
                    self.value = new_value;
                    self.base.update();
                    self.value_changed.emit(new_value);
                    self.slider_moved.emit(new_value);
                }
            }
            return true;
        }

        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let delta = match self.orientation {
            Orientation::Horizontal => {
                if event.delta_x.abs() > event.delta_y.abs() {
                    event.delta_x
                } else {
                    -event.delta_y
                }
            }
            Orientation::Vertical => -event.delta_y,
        };

        if delta.abs() > 0.0 {
            let steps = (delta / 120.0).round() as i32;
            self.set_value(self.value + steps * self.single_step);
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let (decrease_key, increase_key) = match self.orientation {
            Orientation::Horizontal => (Key::ArrowLeft, Key::ArrowRight),
            Orientation::Vertical => (Key::ArrowUp, Key::ArrowDown),
        };

        match event.key {
            key if key == decrease_key || key == Key::ArrowDown || key == Key::ArrowLeft => {
                self.set_value(self.value - self.single_step);
                true
            }
            key if key == increase_key || key == Key::ArrowUp || key == Key::ArrowRight => {
                self.set_value(self.value + self.single_step);
                true
            }
            Key::PageUp => {
                self.set_value(self.value - self.page_step);
                true
            }
            Key::PageDown => {
                self.set_value(self.value + self.page_step);
                true
            }
            Key::Home => {
                self.set_value(self.minimum);
                true
            }
            Key::End => {
                self.set_value(self.maximum);
                true
            }
            _ => false,
        }
    }

    fn handle_leave(&mut self) -> bool {
        if self.hover_thumb {
            self.hover_thumb = false;
            self.base.update();
        }
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_track(&self, ctx: &mut PaintContext<'_>) {
        let track = self.track_rect();

        // Paint background track
        let track_rrect = RoundedRect::new(track, self.border_radius);
        ctx.renderer().fill_rounded_rect(track_rrect, self.track_color);

        // Paint filled portion (before thumb)
        let ratio = self.value_ratio();
        if ratio > 0.0 {
            let fill_rect = match self.orientation {
                Orientation::Horizontal => Rect::new(
                    track.origin.x,
                    track.origin.y,
                    track.width() * ratio,
                    track.height(),
                ),
                Orientation::Vertical => Rect::new(
                    track.origin.x,
                    track.origin.y,
                    track.width(),
                    track.height() * ratio,
                ),
            };
            let fill_rrect = RoundedRect::new(fill_rect, self.border_radius);
            ctx.renderer().fill_rounded_rect(fill_rrect, self.track_fill_color);
        }
    }

    fn paint_ticks(&self, ctx: &mut PaintContext<'_>) {
        if self.tick_position == TickPosition::NoTicks {
            return;
        }

        let track = self.track_rect();
        let range = self.maximum - self.minimum;
        if range <= 0 {
            return;
        }

        let interval = self.effective_tick_interval();
        if interval <= 0 {
            return;
        }

        let tick_length = 6.0;
        let tick_offset = self.thumb_size / 2.0 + 2.0;
        let stroke = Stroke::new(self.tick_color, 1.0);

        let mut value = self.minimum;
        while value <= self.maximum {
            let ratio = (value - self.minimum) as f32 / range as f32;

            match self.orientation {
                Orientation::Horizontal => {
                    let x = track.origin.x + ratio * track.width();
                    let center_y = track.origin.y + track.height() / 2.0;

                    // Above ticks
                    if matches!(self.tick_position, TickPosition::TicksAbove | TickPosition::TicksBothSides) {
                        let y1 = center_y - tick_offset;
                        let y2 = y1 - tick_length;
                        ctx.renderer().draw_line(
                            Point::new(x, y1),
                            Point::new(x, y2),
                            &stroke,
                        );
                    }

                    // Below ticks
                    if matches!(self.tick_position, TickPosition::TicksBelow | TickPosition::TicksBothSides) {
                        let y1 = center_y + tick_offset;
                        let y2 = y1 + tick_length;
                        ctx.renderer().draw_line(
                            Point::new(x, y1),
                            Point::new(x, y2),
                            &stroke,
                        );
                    }
                }
                Orientation::Vertical => {
                    let y = track.origin.y + ratio * track.height();
                    let center_x = track.origin.x + track.width() / 2.0;

                    // Left ticks (above in vertical orientation)
                    if matches!(self.tick_position, TickPosition::TicksAbove | TickPosition::TicksBothSides) {
                        let x1 = center_x - tick_offset;
                        let x2 = x1 - tick_length;
                        ctx.renderer().draw_line(
                            Point::new(x1, y),
                            Point::new(x2, y),
                            &stroke,
                        );
                    }

                    // Right ticks (below in vertical orientation)
                    if matches!(self.tick_position, TickPosition::TicksBelow | TickPosition::TicksBothSides) {
                        let x1 = center_x + tick_offset;
                        let x2 = x1 + tick_length;
                        ctx.renderer().draw_line(
                            Point::new(x1, y),
                            Point::new(x2, y),
                            &stroke,
                        );
                    }
                }
            }

            value += interval;
        }
    }

    fn paint_thumb(&self, ctx: &mut PaintContext<'_>) {
        let center = self.thumb_center();
        let radius = self.thumb_size / 2.0;

        // Choose color based on state
        let color = if self.dragging {
            self.thumb_pressed_color
        } else if self.hover_thumb {
            self.thumb_hover_color
        } else {
            self.thumb_color
        };

        // Draw thumb shadow
        let shadow_color = Color::from_rgba8(0, 0, 0, 40);
        let shadow_rect = Rect::new(
            center.x - radius + 1.0,
            center.y - radius + 2.0,
            self.thumb_size,
            self.thumb_size,
        );
        let shadow_rrect = RoundedRect::new(shadow_rect, radius);
        ctx.renderer().fill_rounded_rect(shadow_rrect, shadow_color);

        // Draw thumb
        let thumb_rect = Rect::new(
            center.x - radius,
            center.y - radius,
            self.thumb_size,
            self.thumb_size,
        );
        let thumb_rrect = RoundedRect::new(thumb_rect, radius);
        ctx.renderer().fill_rounded_rect(thumb_rrect, color);

        // Draw thumb border
        let border_color = Color::from_rgb8(180, 180, 180);
        let border_stroke = Stroke::new(border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(thumb_rrect, &border_stroke);
    }

    fn paint_focus_indicator(&self, ctx: &mut PaintContext<'_>) {
        if !self.base.has_focus() {
            return;
        }

        let center = self.thumb_center();
        let radius = self.thumb_size / 2.0 + 3.0;

        let focus_rect = Rect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        );
        let focus_rrect = RoundedRect::new(focus_rect, radius);
        let focus_color = Color::from_rgba8(66, 133, 244, 100);
        let focus_stroke = Stroke::new(focus_color, 2.0);
        ctx.renderer().stroke_rounded_rect(focus_rrect, &focus_stroke);
    }
}

impl Default for Slider {
    fn default() -> Self {
        Self::new(Orientation::Horizontal)
    }
}

impl Object for Slider {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Slider {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Calculate height based on tick position
        let base_height = self.thumb_size;
        let tick_height = if self.tick_position == TickPosition::NoTicks {
            0.0
        } else if self.tick_position == TickPosition::TicksBothSides {
            20.0
        } else {
            10.0
        };
        let total_height = base_height + tick_height;

        match self.orientation {
            Orientation::Horizontal => {
                SizeHint::from_dimensions(100.0, total_height)
                    .with_minimum_dimensions(40.0, total_height)
            }
            Orientation::Vertical => {
                SizeHint::from_dimensions(total_height, 100.0)
                    .with_minimum_dimensions(total_height, 40.0)
            }
        }
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_track(ctx);
        self.paint_ticks(ctx);
        self.paint_thumb(ctx);
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

// Ensure Slider is Send + Sync
static_assertions::assert_impl_all!(Slider: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        atomic::{AtomicI32, Ordering},
        Arc,
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_slider_creation() {
        setup();
        let slider = Slider::new(Orientation::Horizontal);
        assert_eq!(slider.orientation(), Orientation::Horizontal);
        assert_eq!(slider.minimum(), 0);
        assert_eq!(slider.maximum(), 100);
        assert_eq!(slider.value(), 0);
        assert_eq!(slider.single_step(), 1);
        assert_eq!(slider.page_step(), 10);
        assert_eq!(slider.tick_position(), TickPosition::NoTicks);
    }

    #[test]
    fn test_slider_builder_pattern() {
        setup();
        let slider = Slider::new(Orientation::Vertical)
            .with_range(0, 1000)
            .with_value(500)
            .with_single_step(10)
            .with_page_step(100)
            .with_tick_position(TickPosition::TicksBothSides)
            .with_tick_interval(50);

        assert_eq!(slider.orientation(), Orientation::Vertical);
        assert_eq!(slider.minimum(), 0);
        assert_eq!(slider.maximum(), 1000);
        assert_eq!(slider.value(), 500);
        assert_eq!(slider.single_step(), 10);
        assert_eq!(slider.page_step(), 100);
        assert_eq!(slider.tick_position(), TickPosition::TicksBothSides);
        assert_eq!(slider.tick_interval(), 50);
    }

    #[test]
    fn test_value_clamping() {
        setup();
        let mut slider = Slider::new(Orientation::Horizontal).with_range(0, 100);

        slider.set_value(-10);
        assert_eq!(slider.value(), 0);

        slider.set_value(150);
        assert_eq!(slider.value(), 100);
    }

    #[test]
    fn test_value_changed_signal() {
        setup();
        let mut slider = Slider::new(Orientation::Horizontal);
        let last_value = Arc::new(AtomicI32::new(-1));
        let last_value_clone = last_value.clone();

        slider.value_changed.connect(move |&value| {
            last_value_clone.store(value, Ordering::SeqCst);
        });

        slider.set_value(42);
        assert_eq!(last_value.load(Ordering::SeqCst), 42);

        slider.set_value(75);
        assert_eq!(last_value.load(Ordering::SeqCst), 75);
    }

    #[test]
    fn test_no_signal_for_same_value() {
        setup();
        let mut slider = Slider::new(Orientation::Horizontal).with_value(50);
        let signal_count = Arc::new(AtomicI32::new(0));
        let signal_count_clone = signal_count.clone();

        slider.value_changed.connect(move |_| {
            signal_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        slider.set_value(50);
        assert_eq!(signal_count.load(Ordering::SeqCst), 0);

        slider.set_value(51);
        assert_eq!(signal_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_range_change_clamps_value() {
        setup();
        let mut slider = Slider::new(Orientation::Horizontal)
            .with_range(0, 100)
            .with_value(50);

        slider.set_range(0, 25);
        assert_eq!(slider.value(), 25); // Clamped to new max
    }

    #[test]
    fn test_size_hint() {
        setup();
        let horizontal = Slider::new(Orientation::Horizontal);
        let hint = horizontal.size_hint();
        assert!(hint.preferred.width > hint.preferred.height);

        let vertical = Slider::new(Orientation::Vertical);
        let hint = vertical.size_hint();
        assert!(hint.preferred.height > hint.preferred.width);
    }

    #[test]
    fn test_tick_interval() {
        setup();
        let slider = Slider::new(Orientation::Horizontal)
            .with_single_step(5)
            .with_tick_interval(0);

        // When tick_interval is 0, effective_tick_interval uses single_step
        assert_eq!(slider.effective_tick_interval(), 5);

        let slider = Slider::new(Orientation::Horizontal)
            .with_single_step(5)
            .with_tick_interval(10);

        assert_eq!(slider.effective_tick_interval(), 10);
    }

    #[test]
    fn test_orientation_change() {
        setup();
        let mut slider = Slider::new(Orientation::Horizontal);
        assert_eq!(slider.orientation(), Orientation::Horizontal);

        slider.set_orientation(Orientation::Vertical);
        assert_eq!(slider.orientation(), Orientation::Vertical);
    }
}
