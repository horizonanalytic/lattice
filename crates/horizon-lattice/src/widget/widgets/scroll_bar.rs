//! ScrollBar widget implementation.
//!
//! This module provides [`ScrollBar`], a standalone scrollbar widget that can be used
//! independently or as part of a [`ScrollArea`].
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{ScrollBar, Orientation};
//!
//! // Create a vertical scrollbar
//! let mut scrollbar = ScrollBar::new(Orientation::Vertical)
//!     .with_range(0, 1000)
//!     .with_page_step(100);
//!
//! // Connect to value changes
//! scrollbar.value_changed.connect(|&value| {
//!     println!("Scrolled to: {}", value);
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

/// A scrollbar widget for controlling scroll position.
///
/// ScrollBar provides a visual and interactive way to control scrolling.
/// It can be used standalone or as part of a ScrollArea.
///
/// # Components
///
/// - **Track**: The background area where the thumb moves
/// - **Thumb**: The draggable handle indicating current position and visible portion
/// - **Step buttons** (optional): Arrow buttons at each end for single-step scrolling
///
/// # Signals
///
/// - `value_changed(i32)`: Emitted when the scroll position changes
/// - `slider_pressed()`: Emitted when the thumb is pressed
/// - `slider_released()`: Emitted when the thumb is released
/// - `range_changed(i32, i32)`: Emitted when the range changes
pub struct ScrollBar {
    /// Widget base.
    base: WidgetBase,

    /// Scrollbar orientation.
    orientation: Orientation,

    /// Minimum value.
    minimum: i32,

    /// Maximum value.
    maximum: i32,

    /// Current value.
    value: i32,

    /// Page step size (used for Page Up/Down and clicking track).
    page_step: i32,

    /// Single step size (used for arrow keys and step buttons).
    single_step: i32,

    /// Whether to show step buttons at the ends.
    show_step_buttons: bool,

    /// Whether the thumb is currently being dragged.
    dragging: bool,

    /// Drag start position (in widget coordinates).
    drag_start_pos: f32,

    /// Value when drag started.
    drag_start_value: i32,

    /// Track color.
    track_color: Color,

    /// Thumb color.
    thumb_color: Color,

    /// Thumb hover color.
    thumb_hover_color: Color,

    /// Thumb pressed color.
    thumb_pressed_color: Color,

    /// Step button color.
    step_button_color: Color,

    /// Step button hover color.
    step_button_hover_color: Color,

    /// Step button size (width for vertical, height for horizontal).
    step_button_size: f32,

    /// Minimum thumb size.
    min_thumb_size: f32,

    /// Border radius for rounded corners.
    border_radius: f32,

    /// Which part is currently hovered (for visual feedback).
    hover_part: ScrollBarPart,

    /// Signal emitted when value changes.
    pub value_changed: Signal<i32>,

    /// Signal emitted when thumb is pressed.
    pub slider_pressed: Signal<()>,

    /// Signal emitted when thumb is released.
    pub slider_released: Signal<()>,

    /// Signal emitted when range changes.
    pub range_changed: Signal<(i32, i32)>,
}

/// Parts of the scrollbar for hit testing and hover feedback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ScrollBarPart {
    /// No part (outside scrollbar or no hover).
    #[default]
    None,
    /// The decrease step button (top/left arrow).
    StepButtonDecrease,
    /// The increase step button (bottom/right arrow).
    StepButtonIncrease,
    /// The track above/left of the thumb.
    TrackDecrease,
    /// The track below/right of the thumb.
    TrackIncrease,
    /// The thumb itself.
    Thumb,
}

impl ScrollBar {
    /// Create a new scrollbar with the given orientation.
    pub fn new(orientation: Orientation) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::TabFocus);

        // Set size policy based on orientation
        let policy = match orientation {
            Orientation::Horizontal => {
                SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Fixed)
            }
            Orientation::Vertical => SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Expanding),
        };
        base.set_size_policy(policy);

        Self {
            base,
            orientation,
            minimum: 0,
            maximum: 100,
            value: 0,
            page_step: 10,
            single_step: 1,
            show_step_buttons: false,
            dragging: false,
            drag_start_pos: 0.0,
            drag_start_value: 0,
            track_color: Color::from_rgb8(230, 230, 230),
            thumb_color: Color::from_rgb8(180, 180, 180),
            thumb_hover_color: Color::from_rgb8(160, 160, 160),
            thumb_pressed_color: Color::from_rgb8(140, 140, 140),
            step_button_color: Color::from_rgb8(200, 200, 200),
            step_button_hover_color: Color::from_rgb8(180, 180, 180),
            step_button_size: 16.0,
            min_thumb_size: 20.0,
            border_radius: 4.0,
            hover_part: ScrollBarPart::None,
            value_changed: Signal::new(),
            slider_pressed: Signal::new(),
            slider_released: Signal::new(),
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

    // =========================================================================
    // Step Buttons
    // =========================================================================

    /// Check if step buttons are shown.
    pub fn show_step_buttons(&self) -> bool {
        self.show_step_buttons
    }

    /// Set whether to show step buttons.
    pub fn set_show_step_buttons(&mut self, show: bool) {
        if self.show_step_buttons != show {
            self.show_step_buttons = show;
            self.base.update();
        }
    }

    /// Set step buttons visibility using builder pattern.
    pub fn with_step_buttons(mut self, show: bool) -> Self {
        self.show_step_buttons = show;
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

    /// Get the border radius.
    pub fn border_radius(&self) -> f32 {
        self.border_radius
    }

    /// Set the border radius.
    pub fn set_border_radius(&mut self, radius: f32) {
        if (self.border_radius - radius).abs() > f32::EPSILON {
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
    // Geometry Helpers
    // =========================================================================

    /// Get the track rectangle (excludes step buttons).
    fn track_rect(&self) -> Rect {
        let rect = self.base.rect();
        let button_offset = if self.show_step_buttons {
            self.step_button_size
        } else {
            0.0
        };

        match self.orientation {
            Orientation::Horizontal => Rect::new(
                button_offset,
                0.0,
                (rect.width() - 2.0 * button_offset).max(0.0),
                rect.height(),
            ),
            Orientation::Vertical => Rect::new(
                0.0,
                button_offset,
                rect.width(),
                (rect.height() - 2.0 * button_offset).max(0.0),
            ),
        }
    }

    /// Get the thumb rectangle.
    fn thumb_rect(&self) -> Rect {
        let track = self.track_rect();
        let range = (self.maximum - self.minimum) as f32;

        if range <= 0.0 {
            return track;
        }

        // Calculate thumb size proportional to page step vs range
        let thumb_ratio = (self.page_step as f32 / (range + self.page_step as f32)).min(1.0);

        match self.orientation {
            Orientation::Horizontal => {
                let track_width = track.width();
                let thumb_width = (track_width * thumb_ratio)
                    .max(self.min_thumb_size)
                    .min(track_width);
                let available_travel = track_width - thumb_width;
                let position = if range > 0.0 {
                    (self.value - self.minimum) as f32 / range * available_travel
                } else {
                    0.0
                };
                Rect::new(
                    track.origin.x + position,
                    track.origin.y,
                    thumb_width,
                    track.height(),
                )
            }
            Orientation::Vertical => {
                let track_height = track.height();
                let thumb_height = (track_height * thumb_ratio)
                    .max(self.min_thumb_size)
                    .min(track_height);
                let available_travel = track_height - thumb_height;
                let position = if range > 0.0 {
                    (self.value - self.minimum) as f32 / range * available_travel
                } else {
                    0.0
                };
                Rect::new(
                    track.origin.x,
                    track.origin.y + position,
                    track.width(),
                    thumb_height,
                )
            }
        }
    }

    /// Get the decrease step button rectangle (top/left).
    fn decrease_button_rect(&self) -> Option<Rect> {
        if !self.show_step_buttons {
            return None;
        }
        let rect = self.base.rect();
        Some(match self.orientation {
            Orientation::Horizontal => Rect::new(0.0, 0.0, self.step_button_size, rect.height()),
            Orientation::Vertical => Rect::new(0.0, 0.0, rect.width(), self.step_button_size),
        })
    }

    /// Get the increase step button rectangle (bottom/right).
    fn increase_button_rect(&self) -> Option<Rect> {
        if !self.show_step_buttons {
            return None;
        }
        let rect = self.base.rect();
        Some(match self.orientation {
            Orientation::Horizontal => Rect::new(
                rect.width() - self.step_button_size,
                0.0,
                self.step_button_size,
                rect.height(),
            ),
            Orientation::Vertical => Rect::new(
                0.0,
                rect.height() - self.step_button_size,
                rect.width(),
                self.step_button_size,
            ),
        })
    }

    /// Hit test to determine which part of the scrollbar is at a point.
    fn hit_test(&self, pos: Point) -> ScrollBarPart {
        // Check step buttons first
        if let Some(rect) = self.decrease_button_rect()
            && rect.contains(pos) {
                return ScrollBarPart::StepButtonDecrease;
            }
        if let Some(rect) = self.increase_button_rect()
            && rect.contains(pos) {
                return ScrollBarPart::StepButtonIncrease;
            }

        // Check thumb
        let thumb = self.thumb_rect();
        if thumb.contains(pos) {
            return ScrollBarPart::Thumb;
        }

        // Check track regions
        let track = self.track_rect();
        if track.contains(pos) {
            match self.orientation {
                Orientation::Horizontal => {
                    if pos.x < thumb.origin.x {
                        return ScrollBarPart::TrackDecrease;
                    } else {
                        return ScrollBarPart::TrackIncrease;
                    }
                }
                Orientation::Vertical => {
                    if pos.y < thumb.origin.y {
                        return ScrollBarPart::TrackDecrease;
                    } else {
                        return ScrollBarPart::TrackIncrease;
                    }
                }
            }
        }

        ScrollBarPart::None
    }

    /// Convert a position in track space to a scroll value.
    #[allow(dead_code)]
    fn position_to_value(&self, pos: f32) -> i32 {
        let track = self.track_rect();
        let thumb = self.thumb_rect();
        let range = (self.maximum - self.minimum) as f32;

        if range <= 0.0 {
            return self.minimum;
        }

        let (track_length, thumb_length) = match self.orientation {
            Orientation::Horizontal => (track.width(), thumb.width()),
            Orientation::Vertical => (track.height(), thumb.height()),
        };

        let available_travel = track_length - thumb_length;
        if available_travel <= 0.0 {
            return self.minimum;
        }

        let track_start = match self.orientation {
            Orientation::Horizontal => track.origin.x,
            Orientation::Vertical => track.origin.y,
        };

        let relative_pos = (pos - track_start - thumb_length / 2.0).clamp(0.0, available_travel);
        let ratio = relative_pos / available_travel;

        self.minimum + (ratio * range).round() as i32
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let part = self.hit_test(event.local_pos);
        match part {
            ScrollBarPart::Thumb => {
                self.dragging = true;
                self.drag_start_pos = match self.orientation {
                    Orientation::Horizontal => event.local_pos.x,
                    Orientation::Vertical => event.local_pos.y,
                };
                self.drag_start_value = self.value;
                self.slider_pressed.emit(());
                self.base.update();
                true
            }
            ScrollBarPart::TrackDecrease => {
                self.set_value(self.value - self.page_step);
                true
            }
            ScrollBarPart::TrackIncrease => {
                self.set_value(self.value + self.page_step);
                true
            }
            ScrollBarPart::StepButtonDecrease => {
                self.set_value(self.value - self.single_step);
                true
            }
            ScrollBarPart::StepButtonIncrease => {
                self.set_value(self.value + self.single_step);
                true
            }
            ScrollBarPart::None => false,
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
        let new_hover = self.hit_test(event.local_pos);
        if self.hover_part != new_hover {
            self.hover_part = new_hover;
            self.base.update();
        }

        // Handle dragging
        if self.dragging {
            let current_pos = match self.orientation {
                Orientation::Horizontal => event.local_pos.x,
                Orientation::Vertical => event.local_pos.y,
            };

            let track = self.track_rect();
            let thumb = self.thumb_rect();
            let range = (self.maximum - self.minimum) as f32;

            let (track_length, thumb_length) = match self.orientation {
                Orientation::Horizontal => (track.width(), thumb.width()),
                Orientation::Vertical => (track.height(), thumb.height()),
            };

            let available_travel = track_length - thumb_length;
            if available_travel > 0.0 && range > 0.0 {
                let delta_pos = current_pos - self.drag_start_pos;
                let delta_value = (delta_pos / available_travel * range).round() as i32;
                let new_value =
                    (self.drag_start_value + delta_value).clamp(self.minimum, self.maximum);
                self.set_value(new_value);
            }
            return true;
        }

        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        // Scroll by single steps based on wheel delta
        let delta = if event.modifiers.shift {
            // Horizontal wheel with shift
            event.delta_x
        } else {
            event.delta_y
        };

        if delta.abs() > 0.0 {
            let steps = (delta / 120.0 * 3.0).round() as i32;
            self.set_value(self.value - steps * self.single_step);
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let (decrease_key, increase_key, page_decrease_key, page_increase_key) =
            match self.orientation {
                Orientation::Horizontal => (Key::ArrowLeft, Key::ArrowRight, Key::Home, Key::End),
                Orientation::Vertical => (Key::ArrowUp, Key::ArrowDown, Key::PageUp, Key::PageDown),
            };

        match event.key {
            key if key == decrease_key => {
                self.set_value(self.value - self.single_step);
                true
            }
            key if key == increase_key => {
                self.set_value(self.value + self.single_step);
                true
            }
            key if key == page_decrease_key || key == Key::PageUp => {
                self.set_value(self.value - self.page_step);
                true
            }
            key if key == page_increase_key || key == Key::PageDown => {
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
        if self.hover_part != ScrollBarPart::None {
            self.hover_part = ScrollBarPart::None;
            self.base.update();
        }
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_track(&self, ctx: &mut PaintContext<'_>) {
        let track = self.track_rect();
        let track_rrect = RoundedRect::new(track, self.border_radius);
        ctx.renderer()
            .fill_rounded_rect(track_rrect, self.track_color);
    }

    fn paint_thumb(&self, ctx: &mut PaintContext<'_>) {
        let thumb = self.thumb_rect();

        // Choose color based on state
        let color = if self.dragging {
            self.thumb_pressed_color
        } else if self.hover_part == ScrollBarPart::Thumb {
            self.thumb_hover_color
        } else {
            self.thumb_color
        };

        // Add small padding to thumb
        let padding = 2.0;
        let padded_thumb = match self.orientation {
            Orientation::Horizontal => Rect::new(
                thumb.origin.x + padding,
                thumb.origin.y + padding,
                thumb.width() - 2.0 * padding,
                thumb.height() - 2.0 * padding,
            ),
            Orientation::Vertical => Rect::new(
                thumb.origin.x + padding,
                thumb.origin.y + padding,
                thumb.width() - 2.0 * padding,
                thumb.height() - 2.0 * padding,
            ),
        };

        let thumb_rrect = RoundedRect::new(padded_thumb, self.border_radius);
        ctx.renderer().fill_rounded_rect(thumb_rrect, color);
    }

    fn paint_step_buttons(&self, ctx: &mut PaintContext<'_>) {
        if !self.show_step_buttons {
            return;
        }

        // Paint decrease button
        if let Some(rect) = self.decrease_button_rect() {
            let color = if self.hover_part == ScrollBarPart::StepButtonDecrease {
                self.step_button_hover_color
            } else {
                self.step_button_color
            };
            ctx.renderer().fill_rect(rect, color);

            // Draw arrow
            self.paint_arrow(ctx, rect, false);
        }

        // Paint increase button
        if let Some(rect) = self.increase_button_rect() {
            let color = if self.hover_part == ScrollBarPart::StepButtonIncrease {
                self.step_button_hover_color
            } else {
                self.step_button_color
            };
            ctx.renderer().fill_rect(rect, color);

            // Draw arrow
            self.paint_arrow(ctx, rect, true);
        }
    }

    fn paint_arrow(&self, ctx: &mut PaintContext<'_>, rect: Rect, increase: bool) {
        let center_x = rect.origin.x + rect.width() / 2.0;
        let center_y = rect.origin.y + rect.height() / 2.0;
        let arrow_size = 4.0;
        let arrow_color = Color::from_rgb8(100, 100, 100);
        let stroke = Stroke::new(arrow_color, 1.5);

        match (self.orientation, increase) {
            (Orientation::Horizontal, false) => {
                // Left arrow
                let p1 = Point::new(center_x + arrow_size / 2.0, center_y - arrow_size);
                let p2 = Point::new(center_x - arrow_size / 2.0, center_y);
                let p3 = Point::new(center_x + arrow_size / 2.0, center_y + arrow_size);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
            (Orientation::Horizontal, true) => {
                // Right arrow
                let p1 = Point::new(center_x - arrow_size / 2.0, center_y - arrow_size);
                let p2 = Point::new(center_x + arrow_size / 2.0, center_y);
                let p3 = Point::new(center_x - arrow_size / 2.0, center_y + arrow_size);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
            (Orientation::Vertical, false) => {
                // Up arrow
                let p1 = Point::new(center_x - arrow_size, center_y + arrow_size / 2.0);
                let p2 = Point::new(center_x, center_y - arrow_size / 2.0);
                let p3 = Point::new(center_x + arrow_size, center_y + arrow_size / 2.0);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
            (Orientation::Vertical, true) => {
                // Down arrow
                let p1 = Point::new(center_x - arrow_size, center_y - arrow_size / 2.0);
                let p2 = Point::new(center_x, center_y + arrow_size / 2.0);
                let p3 = Point::new(center_x + arrow_size, center_y - arrow_size / 2.0);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
        }
    }
}

impl Default for ScrollBar {
    fn default() -> Self {
        Self::new(Orientation::Vertical)
    }
}

impl Object for ScrollBar {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ScrollBar {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let thickness = 16.0;
        let length = 100.0;

        match self.orientation {
            Orientation::Horizontal => SizeHint::from_dimensions(length, thickness)
                .with_minimum_dimensions(40.0, thickness),
            Orientation::Vertical => SizeHint::from_dimensions(thickness, length)
                .with_minimum_dimensions(thickness, 40.0),
        }
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_track(ctx);
        self.paint_step_buttons(ctx);
        self.paint_thumb(ctx);
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

// Ensure ScrollBar is Send + Sync
static_assertions::assert_impl_all!(ScrollBar: Send, Sync);

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
    fn test_scrollbar_creation() {
        setup();
        let bar = ScrollBar::new(Orientation::Vertical);
        assert_eq!(bar.orientation(), Orientation::Vertical);
        assert_eq!(bar.minimum(), 0);
        assert_eq!(bar.maximum(), 100);
        assert_eq!(bar.value(), 0);
        assert!(!bar.show_step_buttons());
    }

    #[test]
    fn test_scrollbar_builder_pattern() {
        setup();
        let bar = ScrollBar::new(Orientation::Horizontal)
            .with_range(0, 1000)
            .with_value(500)
            .with_page_step(100)
            .with_single_step(10)
            .with_step_buttons(true);

        assert_eq!(bar.orientation(), Orientation::Horizontal);
        assert_eq!(bar.minimum(), 0);
        assert_eq!(bar.maximum(), 1000);
        assert_eq!(bar.value(), 500);
        assert_eq!(bar.page_step(), 100);
        assert_eq!(bar.single_step(), 10);
        assert!(bar.show_step_buttons());
    }

    #[test]
    fn test_value_clamping() {
        setup();
        let mut bar = ScrollBar::new(Orientation::Vertical).with_range(0, 100);

        bar.set_value(-10);
        assert_eq!(bar.value(), 0);

        bar.set_value(150);
        assert_eq!(bar.value(), 100);
    }

    #[test]
    fn test_value_changed_signal() {
        setup();
        let mut bar = ScrollBar::new(Orientation::Vertical);
        let last_value = Arc::new(AtomicI32::new(-1));
        let last_value_clone = last_value.clone();

        bar.value_changed.connect(move |&value| {
            last_value_clone.store(value, Ordering::SeqCst);
        });

        bar.set_value(42);
        assert_eq!(last_value.load(Ordering::SeqCst), 42);

        bar.set_value(75);
        assert_eq!(last_value.load(Ordering::SeqCst), 75);
    }

    #[test]
    fn test_no_signal_for_same_value() {
        setup();
        let mut bar = ScrollBar::new(Orientation::Vertical).with_value(50);
        let signal_count = Arc::new(AtomicI32::new(0));
        let signal_count_clone = signal_count.clone();

        bar.value_changed.connect(move |_| {
            signal_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        bar.set_value(50);
        assert_eq!(signal_count.load(Ordering::SeqCst), 0);

        bar.set_value(51);
        assert_eq!(signal_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_range_change() {
        setup();
        let mut bar = ScrollBar::new(Orientation::Vertical)
            .with_range(0, 100)
            .with_value(50);

        bar.set_range(0, 25);
        assert_eq!(bar.value(), 25); // Clamped to new max
    }

    #[test]
    fn test_size_hint() {
        setup();
        let vertical = ScrollBar::new(Orientation::Vertical);
        let hint = vertical.size_hint();
        assert!(hint.preferred.height > hint.preferred.width);

        let horizontal = ScrollBar::new(Orientation::Horizontal);
        let hint = horizontal.size_hint();
        assert!(hint.preferred.width > hint.preferred.height);
    }
}
