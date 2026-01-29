//! ScrollArea widget implementation.
//!
//! This module provides [`ScrollArea`], a container widget that provides scrollable
//! views of larger content areas.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{ScrollArea, ScrollBarPolicy};
//!
//! // Create a scroll area with automatic scrollbars
//! let mut scroll_area = ScrollArea::new()
//!     .with_horizontal_policy(ScrollBarPolicy::AsNeeded)
//!     .with_vertical_policy(ScrollBarPolicy::AsNeeded);
//!
//! // Enable smooth kinetic scrolling
//! scroll_area.set_kinetic_scrolling(true);
//!
//! // Scroll programmatically
//! scroll_area.scroll_to(100, 200);
//! scroll_area.ensure_visible(50, 50, 100, 100);
//! ```

use std::time::Instant;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Size};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, WheelEvent, Widget,
    WidgetBase, WidgetEvent,
};

/// Policy for scrollbar visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollBarPolicy {
    /// Scrollbar is shown when content exceeds viewport (default).
    #[default]
    AsNeeded,
    /// Scrollbar is always shown.
    AlwaysOn,
    /// Scrollbar is never shown.
    AlwaysOff,
}

/// A scrollable container widget.
///
/// ScrollArea provides a scrollable view into content that may be larger
/// than the visible area. It includes optional horizontal and vertical
/// scrollbars that can be configured independently.
///
/// # Features
///
/// - **Viewport**: The visible area where content is displayed
/// - **Scrollbars**: Independent horizontal and vertical scrollbars
/// - **Scroll policies**: Control when scrollbars appear
/// - **Kinetic scrolling**: Momentum-based scrolling with physics simulation
/// - **Programmatic scrolling**: Scroll to specific positions or ensure visibility
///
/// # Signals
///
/// - `horizontal_scrolled(i32)`: Emitted when horizontal position changes
/// - `vertical_scrolled(i32)`: Emitted when vertical position changes
pub struct ScrollArea {
    /// Widget base.
    base: WidgetBase,

    /// Horizontal scroll position.
    scroll_x: i32,

    /// Vertical scroll position.
    scroll_y: i32,

    /// Content size (total scrollable area).
    content_size: Size,

    /// Horizontal scrollbar policy.
    horizontal_policy: ScrollBarPolicy,

    /// Vertical scrollbar policy.
    vertical_policy: ScrollBarPolicy,

    /// Whether kinetic scrolling is enabled.
    kinetic_scrolling: bool,

    /// Kinetic scroller state.
    scroller: KineticScroller,

    /// Scrollbar thickness.
    scrollbar_thickness: f32,

    /// Background color for the viewport.
    viewport_background: Color,

    /// Whether widget should resize to fill viewport when smaller.
    widget_resizable: bool,

    /// Signal for horizontal scroll changes.
    pub horizontal_scrolled: Signal<i32>,

    /// Signal for vertical scroll changes.
    pub vertical_scrolled: Signal<i32>,
}

/// State machine for kinetic (momentum-based) scrolling.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct KineticScroller {
    /// Current state.
    state: ScrollerState,

    /// Velocity in pixels per second.
    velocity: (f32, f32),

    /// Last update timestamp.
    last_update: Instant,

    /// Accumulated position for smooth animation.
    accumulated_x: f32,
    accumulated_y: f32,

    /// Deceleration factor (pixels/second^2).
    deceleration: f32,

    /// Minimum velocity to continue scrolling.
    min_velocity: f32,

    /// Maximum velocity cap.
    max_velocity: f32,

    /// Overshoot resistance factor (0-1).
    overshoot_resistance: f32,

    /// Maximum overshoot distance.
    max_overshoot: f32,

    /// Overshoot bounce-back duration in seconds.
    overshoot_time: f32,

    /// Position history for velocity calculation.
    position_history: Vec<(f32, f32, Instant)>,

    /// Drag start position.
    drag_start: Option<(f32, f32)>,

    /// Drag start scroll position.
    drag_start_scroll: (i32, i32),
}

/// States for the kinetic scroller.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ScrollerState {
    /// Not scrolling.
    Inactive,
    /// Touch/mouse pressed, waiting for movement.
    Pressed,
    /// Following finger/mouse movement.
    Dragging,
    /// Autonomous momentum scrolling after release.
    Scrolling,
    /// Bouncing back from overshoot.
    Overshooting,
}

impl Default for KineticScroller {
    fn default() -> Self {
        Self {
            state: ScrollerState::Inactive,
            velocity: (0.0, 0.0),
            last_update: Instant::now(),
            accumulated_x: 0.0,
            accumulated_y: 0.0,
            deceleration: 1500.0,
            min_velocity: 10.0,
            max_velocity: 5000.0,
            overshoot_resistance: 0.5,
            max_overshoot: 100.0,
            overshoot_time: 0.3,
            position_history: Vec::with_capacity(10),
            drag_start: None,
            drag_start_scroll: (0, 0),
        }
    }
}

impl KineticScroller {
    /// Reset the scroller to inactive state.
    fn reset(&mut self) {
        self.state = ScrollerState::Inactive;
        self.velocity = (0.0, 0.0);
        self.position_history.clear();
        self.drag_start = None;
    }

    /// Start a press at the given position.
    fn press(&mut self, x: f32, y: f32, scroll_x: i32, scroll_y: i32) {
        self.state = ScrollerState::Pressed;
        self.velocity = (0.0, 0.0);
        self.position_history.clear();
        self.drag_start = Some((x, y));
        self.drag_start_scroll = (scroll_x, scroll_y);
        self.last_update = Instant::now();
        self.record_position(x, y);
    }

    /// Record a position for velocity calculation.
    fn record_position(&mut self, x: f32, y: f32) {
        let now = Instant::now();

        // Keep only recent history (last 100ms)
        let cutoff = now - std::time::Duration::from_millis(100);
        self.position_history.retain(|(_, _, t)| *t > cutoff);

        self.position_history.push((x, y, now));
    }

    /// Move during a drag.
    fn drag(&mut self, x: f32, y: f32) -> Option<(i32, i32)> {
        if self.state == ScrollerState::Pressed {
            // Check if we've moved enough to start dragging
            if let Some((start_x, start_y)) = self.drag_start {
                let dx = (x - start_x).abs();
                let dy = (y - start_y).abs();
                if dx > 5.0 || dy > 5.0 {
                    self.state = ScrollerState::Dragging;
                }
            }
        }

        if self.state == ScrollerState::Dragging {
            self.record_position(x, y);

            if let Some((start_x, start_y)) = self.drag_start {
                let dx = start_x - x;
                let dy = start_y - y;
                let new_x = self.drag_start_scroll.0 + dx as i32;
                let new_y = self.drag_start_scroll.1 + dy as i32;
                return Some((new_x, new_y));
            }
        }

        None
    }

    /// Release after a drag, calculating velocity for momentum.
    fn release(&mut self) -> bool {
        if self.state != ScrollerState::Dragging {
            self.reset();
            return false;
        }

        // Calculate velocity from position history
        if self.position_history.len() >= 2 {
            let (x1, y1, t1) = self.position_history.first().copied().unwrap();
            let (x2, y2, t2) = self.position_history.last().copied().unwrap();
            let dt = t2.duration_since(t1).as_secs_f32();

            if dt > 0.001 {
                let vx = -(x2 - x1) / dt; // Negative because drag direction is opposite to scroll
                let vy = -(y2 - y1) / dt;

                // Apply velocity caps
                self.velocity.0 = vx.clamp(-self.max_velocity, self.max_velocity);
                self.velocity.1 = vy.clamp(-self.max_velocity, self.max_velocity);

                let speed = (self.velocity.0.powi(2) + self.velocity.1.powi(2)).sqrt();
                if speed > self.min_velocity {
                    self.state = ScrollerState::Scrolling;
                    self.last_update = Instant::now();
                    self.accumulated_x = 0.0;
                    self.accumulated_y = 0.0;
                    return true;
                }
            }
        }

        self.reset();
        false
    }

    /// Update momentum scrolling, returning scroll delta if still animating.
    fn update(&mut self) -> Option<(i32, i32)> {
        if self.state != ScrollerState::Scrolling && self.state != ScrollerState::Overshooting {
            return None;
        }

        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        self.last_update = now;

        if dt <= 0.0 {
            return None;
        }

        // Apply deceleration
        let speed = (self.velocity.0.powi(2) + self.velocity.1.powi(2)).sqrt();
        if speed > self.min_velocity {
            let decel = self.deceleration * dt;
            let decel_factor = ((speed - decel) / speed).max(0.0);
            self.velocity.0 *= decel_factor;
            self.velocity.1 *= decel_factor;

            // Calculate scroll delta
            self.accumulated_x += self.velocity.0 * dt;
            self.accumulated_y += self.velocity.1 * dt;

            let dx = self.accumulated_x.round() as i32;
            let dy = self.accumulated_y.round() as i32;

            if dx != 0 || dy != 0 {
                self.accumulated_x -= dx as f32;
                self.accumulated_y -= dy as f32;
                return Some((dx, dy));
            }

            // Continue animation even if no visible delta
            return Some((0, 0));
        }

        self.reset();
        None
    }

    /// Check if currently scrolling (needs animation updates).
    fn is_animating(&self) -> bool {
        matches!(
            self.state,
            ScrollerState::Scrolling | ScrollerState::Overshooting
        )
    }

    /// Check if currently dragging.
    fn is_dragging(&self) -> bool {
        matches!(self.state, ScrollerState::Pressed | ScrollerState::Dragging)
    }
}

impl ScrollArea {
    /// Create a new scroll area with default settings.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));

        Self {
            base,
            scroll_x: 0,
            scroll_y: 0,
            content_size: Size::new(0.0, 0.0),
            horizontal_policy: ScrollBarPolicy::AsNeeded,
            vertical_policy: ScrollBarPolicy::AsNeeded,
            kinetic_scrolling: true,
            scroller: KineticScroller::default(),
            scrollbar_thickness: 12.0,
            viewport_background: Color::WHITE,
            widget_resizable: false,
            horizontal_scrolled: Signal::new(),
            vertical_scrolled: Signal::new(),
        }
    }

    // =========================================================================
    // Scroll Position
    // =========================================================================

    /// Get the horizontal scroll position.
    pub fn scroll_x(&self) -> i32 {
        self.scroll_x
    }

    /// Get the vertical scroll position.
    pub fn scroll_y(&self) -> i32 {
        self.scroll_y
    }

    /// Set the horizontal scroll position.
    pub fn set_scroll_x(&mut self, x: i32) {
        let max_x = self.max_scroll_x();
        let clamped = x.clamp(0, max_x);
        if self.scroll_x != clamped {
            self.scroll_x = clamped;
            self.base.update();
            self.horizontal_scrolled.emit(clamped);
        }
    }

    /// Set the vertical scroll position.
    pub fn set_scroll_y(&mut self, y: i32) {
        let max_y = self.max_scroll_y();
        let clamped = y.clamp(0, max_y);
        if self.scroll_y != clamped {
            self.scroll_y = clamped;
            self.base.update();
            self.vertical_scrolled.emit(clamped);
        }
    }

    /// Scroll to a specific position.
    pub fn scroll_to(&mut self, x: i32, y: i32) {
        self.set_scroll_x(x);
        self.set_scroll_y(y);
    }

    /// Scroll by a relative amount.
    pub fn scroll_by(&mut self, dx: i32, dy: i32) {
        self.set_scroll_x(self.scroll_x + dx);
        self.set_scroll_y(self.scroll_y + dy);
    }

    /// Get the maximum horizontal scroll position.
    pub fn max_scroll_x(&self) -> i32 {
        let viewport = self.viewport_rect();
        (self.content_size.width - viewport.width()).max(0.0) as i32
    }

    /// Get the maximum vertical scroll position.
    pub fn max_scroll_y(&self) -> i32 {
        let viewport = self.viewport_rect();
        (self.content_size.height - viewport.height()).max(0.0) as i32
    }

    // =========================================================================
    // Content Size
    // =========================================================================

    /// Get the content size.
    pub fn content_size(&self) -> Size {
        self.content_size
    }

    /// Set the content size.
    ///
    /// This determines how much can be scrolled.
    pub fn set_content_size(&mut self, size: Size) {
        if (self.content_size.width - size.width).abs() > f32::EPSILON
            || (self.content_size.height - size.height).abs() > f32::EPSILON
        {
            self.content_size = size;
            // Clamp scroll position to new bounds
            let max_x = self.max_scroll_x();
            let max_y = self.max_scroll_y();
            if self.scroll_x > max_x {
                self.scroll_x = max_x;
            }
            if self.scroll_y > max_y {
                self.scroll_y = max_y;
            }
            self.base.update();
        }
    }

    /// Set content size using builder pattern.
    pub fn with_content_size(mut self, size: Size) -> Self {
        self.set_content_size(size);
        self
    }

    // =========================================================================
    // Scroll Policies
    // =========================================================================

    /// Get the horizontal scrollbar policy.
    pub fn horizontal_policy(&self) -> ScrollBarPolicy {
        self.horizontal_policy
    }

    /// Set the horizontal scrollbar policy.
    pub fn set_horizontal_policy(&mut self, policy: ScrollBarPolicy) {
        if self.horizontal_policy != policy {
            self.horizontal_policy = policy;
            self.base.update();
        }
    }

    /// Set horizontal policy using builder pattern.
    pub fn with_horizontal_policy(mut self, policy: ScrollBarPolicy) -> Self {
        self.horizontal_policy = policy;
        self
    }

    /// Get the vertical scrollbar policy.
    pub fn vertical_policy(&self) -> ScrollBarPolicy {
        self.vertical_policy
    }

    /// Set the vertical scrollbar policy.
    pub fn set_vertical_policy(&mut self, policy: ScrollBarPolicy) {
        if self.vertical_policy != policy {
            self.vertical_policy = policy;
            self.base.update();
        }
    }

    /// Set vertical policy using builder pattern.
    pub fn with_vertical_policy(mut self, policy: ScrollBarPolicy) -> Self {
        self.vertical_policy = policy;
        self
    }

    // =========================================================================
    // Kinetic Scrolling
    // =========================================================================

    /// Check if kinetic scrolling is enabled.
    pub fn kinetic_scrolling(&self) -> bool {
        self.kinetic_scrolling
    }

    /// Enable or disable kinetic scrolling.
    pub fn set_kinetic_scrolling(&mut self, enabled: bool) {
        self.kinetic_scrolling = enabled;
        if !enabled {
            self.scroller.reset();
        }
    }

    /// Set kinetic scrolling using builder pattern.
    pub fn with_kinetic_scrolling(mut self, enabled: bool) -> Self {
        self.kinetic_scrolling = enabled;
        self
    }

    /// Get the deceleration factor for kinetic scrolling.
    pub fn deceleration(&self) -> f32 {
        self.scroller.deceleration
    }

    /// Set the deceleration factor for kinetic scrolling.
    pub fn set_deceleration(&mut self, deceleration: f32) {
        self.scroller.deceleration = deceleration.max(100.0);
    }

    /// Set deceleration using builder pattern.
    pub fn with_deceleration(mut self, deceleration: f32) -> Self {
        self.set_deceleration(deceleration);
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the scrollbar thickness.
    pub fn scrollbar_thickness(&self) -> f32 {
        self.scrollbar_thickness
    }

    /// Set the scrollbar thickness.
    pub fn set_scrollbar_thickness(&mut self, thickness: f32) {
        if (self.scrollbar_thickness - thickness).abs() > f32::EPSILON {
            self.scrollbar_thickness = thickness.max(6.0);
            self.base.update();
        }
    }

    /// Set scrollbar thickness using builder pattern.
    pub fn with_scrollbar_thickness(mut self, thickness: f32) -> Self {
        self.set_scrollbar_thickness(thickness);
        self
    }

    /// Get the viewport background color.
    pub fn viewport_background(&self) -> Color {
        self.viewport_background
    }

    /// Set the viewport background color.
    pub fn set_viewport_background(&mut self, color: Color) {
        if self.viewport_background != color {
            self.viewport_background = color;
            self.base.update();
        }
    }

    /// Set viewport background using builder pattern.
    pub fn with_viewport_background(mut self, color: Color) -> Self {
        self.viewport_background = color;
        self
    }

    /// Check if the content widget should resize to fill the viewport.
    pub fn widget_resizable(&self) -> bool {
        self.widget_resizable
    }

    /// Set whether the content widget should resize to fill the viewport.
    pub fn set_widget_resizable(&mut self, resizable: bool) {
        if self.widget_resizable != resizable {
            self.widget_resizable = resizable;
            self.base.update();
        }
    }

    /// Set widget resizable using builder pattern.
    pub fn with_widget_resizable(mut self, resizable: bool) -> Self {
        self.widget_resizable = resizable;
        self
    }

    // =========================================================================
    // Scroll Features
    // =========================================================================

    /// Ensure a point is visible in the viewport.
    ///
    /// Scrolls the minimum amount needed to make the point visible.
    pub fn ensure_visible_point(&mut self, x: i32, y: i32, x_margin: i32, y_margin: i32) {
        let viewport = self.viewport_rect();
        let vw = viewport.width() as i32;
        let vh = viewport.height() as i32;

        // Horizontal
        let left = self.scroll_x + x_margin;
        let right = self.scroll_x + vw - x_margin;
        if x < left {
            self.set_scroll_x(x - x_margin);
        } else if x > right {
            self.set_scroll_x(x - vw + x_margin);
        }

        // Vertical
        let top = self.scroll_y + y_margin;
        let bottom = self.scroll_y + vh - y_margin;
        if y < top {
            self.set_scroll_y(y - y_margin);
        } else if y > bottom {
            self.set_scroll_y(y - vh + y_margin);
        }
    }

    /// Ensure a rectangle is visible in the viewport.
    ///
    /// Scrolls the minimum amount needed to make the rectangle visible.
    pub fn ensure_visible_rect(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        x_margin: i32,
        y_margin: i32,
    ) {
        let viewport = self.viewport_rect();
        let vw = viewport.width() as i32;
        let vh = viewport.height() as i32;

        // Horizontal
        let rect_left = x;
        let rect_right = x + width;
        let view_left = self.scroll_x;
        let view_right = self.scroll_x + vw;

        if rect_left < view_left + x_margin {
            self.set_scroll_x(rect_left - x_margin);
        } else if rect_right > view_right - x_margin {
            // Try to show the whole rect if possible
            if width <= vw - 2 * x_margin {
                self.set_scroll_x(rect_right - vw + x_margin);
            } else {
                // Rect is too wide, show left edge
                self.set_scroll_x(rect_left - x_margin);
            }
        }

        // Vertical
        let rect_top = y;
        let rect_bottom = y + height;
        let view_top = self.scroll_y;
        let view_bottom = self.scroll_y + vh;

        if rect_top < view_top + y_margin {
            self.set_scroll_y(rect_top - y_margin);
        } else if rect_bottom > view_bottom - y_margin {
            if height <= vh - 2 * y_margin {
                self.set_scroll_y(rect_bottom - vh + y_margin);
            } else {
                self.set_scroll_y(rect_top - y_margin);
            }
        }
    }

    /// Scroll to the top.
    pub fn scroll_to_top(&mut self) {
        self.set_scroll_y(0);
    }

    /// Scroll to the bottom.
    pub fn scroll_to_bottom(&mut self) {
        self.set_scroll_y(self.max_scroll_y());
    }

    /// Scroll to the left.
    pub fn scroll_to_left(&mut self) {
        self.set_scroll_x(0);
    }

    /// Scroll to the right.
    pub fn scroll_to_right(&mut self) {
        self.set_scroll_x(self.max_scroll_x());
    }

    // =========================================================================
    // Viewport Culling
    // =========================================================================

    /// Get the visible content rectangle in content coordinates.
    ///
    /// This is the portion of the content that's currently visible in the viewport.
    /// Use this to optimize rendering by only drawing content that's visible.
    pub fn visible_content_rect(&self) -> Rect {
        let viewport = self.viewport_rect();
        Rect::new(
            self.scroll_x as f32,
            self.scroll_y as f32,
            viewport.width(),
            viewport.height(),
        )
    }

    /// Check if a content rectangle is visible (intersects the viewport).
    ///
    /// Use this for viewport culling to skip rendering items that aren't visible.
    pub fn is_content_visible(&self, content_rect: Rect) -> bool {
        let visible = self.visible_content_rect();
        rects_intersect(visible, content_rect)
    }

    /// Check if a content point is visible in the viewport.
    pub fn is_point_visible(&self, x: f32, y: f32) -> bool {
        let visible = self.visible_content_rect();
        x >= visible.origin.x
            && x < visible.origin.x + visible.width()
            && y >= visible.origin.y
            && y < visible.origin.y + visible.height()
    }

    /// Transform a content coordinate to viewport coordinate.
    ///
    /// Returns the position within the viewport where the content point would appear.
    pub fn content_to_viewport(&self, content_x: f32, content_y: f32) -> Point {
        Point::new(
            content_x - self.scroll_x as f32,
            content_y - self.scroll_y as f32,
        )
    }

    /// Transform a viewport coordinate to content coordinate.
    ///
    /// Returns the content position that corresponds to the viewport point.
    pub fn viewport_to_content(&self, viewport_x: f32, viewport_y: f32) -> Point {
        Point::new(
            viewport_x + self.scroll_x as f32,
            viewport_y + self.scroll_y as f32,
        )
    }

    /// Get the first visible row index for a list with fixed row heights.
    ///
    /// Useful for virtual scrolling in lists.
    pub fn first_visible_row(&self, row_height: f32) -> usize {
        if row_height <= 0.0 {
            return 0;
        }
        (self.scroll_y as f32 / row_height).floor() as usize
    }

    /// Get the last visible row index for a list with fixed row heights.
    ///
    /// Useful for virtual scrolling in lists.
    pub fn last_visible_row(&self, row_height: f32, total_rows: usize) -> usize {
        if row_height <= 0.0 {
            return 0;
        }
        let viewport = self.viewport_rect();
        let last = ((self.scroll_y as f32 + viewport.height()) / row_height).ceil() as usize;
        last.min(total_rows.saturating_sub(1))
    }

    /// Get the range of visible row indices for virtual scrolling.
    ///
    /// Returns (first_visible_row, last_visible_row).
    pub fn visible_row_range(&self, row_height: f32, total_rows: usize) -> (usize, usize) {
        (
            self.first_visible_row(row_height),
            self.last_visible_row(row_height, total_rows),
        )
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    /// Get the viewport rectangle (visible content area).
    pub fn viewport_rect(&self) -> Rect {
        let rect = self.base.rect();
        let h_visible = self.is_horizontal_scrollbar_visible();
        let v_visible = self.is_vertical_scrollbar_visible();

        let width = if v_visible {
            rect.width() - self.scrollbar_thickness
        } else {
            rect.width()
        };

        let height = if h_visible {
            rect.height() - self.scrollbar_thickness
        } else {
            rect.height()
        };

        Rect::new(0.0, 0.0, width.max(0.0), height.max(0.0))
    }

    /// Check if the horizontal scrollbar should be visible.
    fn is_horizontal_scrollbar_visible(&self) -> bool {
        match self.horizontal_policy {
            ScrollBarPolicy::AlwaysOn => true,
            ScrollBarPolicy::AlwaysOff => false,
            ScrollBarPolicy::AsNeeded => {
                // Need to calculate without considering this scrollbar's space
                let rect = self.base.rect();
                let v_space = if self.is_vertical_scrollbar_needed_basic() {
                    self.scrollbar_thickness
                } else {
                    0.0
                };
                let available_width = rect.width() - v_space;
                self.content_size.width > available_width
            }
        }
    }

    /// Check if the vertical scrollbar should be visible.
    fn is_vertical_scrollbar_visible(&self) -> bool {
        match self.vertical_policy {
            ScrollBarPolicy::AlwaysOn => true,
            ScrollBarPolicy::AlwaysOff => false,
            ScrollBarPolicy::AsNeeded => {
                let rect = self.base.rect();
                let h_space = if self.is_horizontal_scrollbar_needed_basic() {
                    self.scrollbar_thickness
                } else {
                    0.0
                };
                let available_height = rect.height() - h_space;
                self.content_size.height > available_height
            }
        }
    }

    /// Basic check for horizontal scrollbar need (ignores other scrollbar).
    fn is_horizontal_scrollbar_needed_basic(&self) -> bool {
        match self.horizontal_policy {
            ScrollBarPolicy::AlwaysOn => true,
            ScrollBarPolicy::AlwaysOff => false,
            ScrollBarPolicy::AsNeeded => {
                let rect = self.base.rect();
                self.content_size.width > rect.width()
            }
        }
    }

    /// Basic check for vertical scrollbar need (ignores other scrollbar).
    fn is_vertical_scrollbar_needed_basic(&self) -> bool {
        match self.vertical_policy {
            ScrollBarPolicy::AlwaysOn => true,
            ScrollBarPolicy::AlwaysOff => false,
            ScrollBarPolicy::AsNeeded => {
                let rect = self.base.rect();
                self.content_size.height > rect.height()
            }
        }
    }

    /// Get the horizontal scrollbar rectangle.
    fn horizontal_scrollbar_rect(&self) -> Option<Rect> {
        if !self.is_horizontal_scrollbar_visible() {
            return None;
        }
        let rect = self.base.rect();
        let v_visible = self.is_vertical_scrollbar_visible();
        let width = if v_visible {
            rect.width() - self.scrollbar_thickness
        } else {
            rect.width()
        };

        Some(Rect::new(
            0.0,
            rect.height() - self.scrollbar_thickness,
            width.max(0.0),
            self.scrollbar_thickness,
        ))
    }

    /// Get the vertical scrollbar rectangle.
    fn vertical_scrollbar_rect(&self) -> Option<Rect> {
        if !self.is_vertical_scrollbar_visible() {
            return None;
        }
        let rect = self.base.rect();
        let h_visible = self.is_horizontal_scrollbar_visible();
        let height = if h_visible {
            rect.height() - self.scrollbar_thickness
        } else {
            rect.height()
        };

        Some(Rect::new(
            rect.width() - self.scrollbar_thickness,
            0.0,
            self.scrollbar_thickness,
            height.max(0.0),
        ))
    }

    /// Get the corner rectangle (between scrollbars).
    fn corner_rect(&self) -> Option<Rect> {
        if self.is_horizontal_scrollbar_visible() && self.is_vertical_scrollbar_visible() {
            let rect = self.base.rect();
            Some(Rect::new(
                rect.width() - self.scrollbar_thickness,
                rect.height() - self.scrollbar_thickness,
                self.scrollbar_thickness,
                self.scrollbar_thickness,
            ))
        } else {
            None
        }
    }

    /// Check if a point is in the viewport area.
    fn is_in_viewport(&self, pos: Point) -> bool {
        self.viewport_rect().contains(pos)
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Check if in scrollbar area
        if let Some(h_rect) = self.horizontal_scrollbar_rect()
            && h_rect.contains(event.local_pos)
        {
            // Let scrollbar handle it (would need embedded scrollbar)
            return self.handle_scrollbar_click(event.local_pos, true);
        }
        if let Some(v_rect) = self.vertical_scrollbar_rect()
            && v_rect.contains(event.local_pos)
        {
            return self.handle_scrollbar_click(event.local_pos, false);
        }

        // In viewport - start kinetic scrolling if enabled
        if self.kinetic_scrolling && self.is_in_viewport(event.local_pos) {
            self.scroller.press(
                event.local_pos.x,
                event.local_pos.y,
                self.scroll_x,
                self.scroll_y,
            );
            return true;
        }

        false
    }

    fn handle_scrollbar_click(&mut self, pos: Point, horizontal: bool) -> bool {
        if horizontal {
            if let Some(rect) = self.horizontal_scrollbar_rect() {
                let viewport = self.viewport_rect();
                let thumb_ratio = viewport.width() / self.content_size.width.max(1.0);
                let thumb_width = (rect.width() * thumb_ratio).max(20.0).min(rect.width());
                let available_travel = rect.width() - thumb_width;
                let max_scroll = self.max_scroll_x() as f32;

                if available_travel > 0.0 && max_scroll > 0.0 {
                    let thumb_pos = (self.scroll_x as f32 / max_scroll) * available_travel;
                    let click_pos = pos.x - rect.origin.x;

                    if click_pos < thumb_pos {
                        // Page left
                        self.set_scroll_x(self.scroll_x - viewport.width() as i32);
                    } else if click_pos > thumb_pos + thumb_width {
                        // Page right
                        self.set_scroll_x(self.scroll_x + viewport.width() as i32);
                    }
                }
                return true;
            }
        } else if let Some(rect) = self.vertical_scrollbar_rect() {
            let viewport = self.viewport_rect();
            let thumb_ratio = viewport.height() / self.content_size.height.max(1.0);
            let thumb_height = (rect.height() * thumb_ratio).max(20.0).min(rect.height());
            let available_travel = rect.height() - thumb_height;
            let max_scroll = self.max_scroll_y() as f32;

            if available_travel > 0.0 && max_scroll > 0.0 {
                let thumb_pos = (self.scroll_y as f32 / max_scroll) * available_travel;
                let click_pos = pos.y - rect.origin.y;

                if click_pos < thumb_pos {
                    // Page up
                    self.set_scroll_y(self.scroll_y - viewport.height() as i32);
                } else if click_pos > thumb_pos + thumb_height {
                    // Page down
                    self.set_scroll_y(self.scroll_y + viewport.height() as i32);
                }
            }
            return true;
        }
        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.scroller.is_dragging() {
            if self.scroller.release() {
                // Start momentum animation - would need timer/animation system
                self.base.update();
            }
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        if self.scroller.is_dragging()
            && let Some((new_x, new_y)) = self.scroller.drag(event.local_pos.x, event.local_pos.y)
        {
            self.scroll_to(new_x, new_y);
            return true;
        }
        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let mut handled = false;

        // Horizontal scroll with shift
        if event.modifiers.shift || event.delta_x.abs() > event.delta_y.abs() {
            let delta = if event.modifiers.shift {
                event.delta_y
            } else {
                event.delta_x
            };
            if delta.abs() > 0.0 {
                let scroll_amount = (delta * 0.5).round() as i32;
                self.set_scroll_x(self.scroll_x - scroll_amount);
                handled = true;
            }
        } else {
            // Vertical scroll
            if event.delta_y.abs() > 0.0 {
                let scroll_amount = (event.delta_y * 0.5).round() as i32;
                self.set_scroll_y(self.scroll_y - scroll_amount);
                handled = true;
            }
        }

        handled
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let viewport = self.viewport_rect();
        let _page_x = viewport.width() as i32;
        let page_y = viewport.height() as i32;
        let step = 40; // Single step in pixels

        match event.key {
            Key::ArrowUp => {
                self.set_scroll_y(self.scroll_y - step);
                true
            }
            Key::ArrowDown => {
                self.set_scroll_y(self.scroll_y + step);
                true
            }
            Key::ArrowLeft => {
                self.set_scroll_x(self.scroll_x - step);
                true
            }
            Key::ArrowRight => {
                self.set_scroll_x(self.scroll_x + step);
                true
            }
            Key::PageUp => {
                self.set_scroll_y(self.scroll_y - page_y);
                true
            }
            Key::PageDown => {
                self.set_scroll_y(self.scroll_y + page_y);
                true
            }
            Key::Home => {
                if event.modifiers.control {
                    self.scroll_to(0, 0);
                } else {
                    self.scroll_to_top();
                }
                true
            }
            Key::End => {
                if event.modifiers.control {
                    self.scroll_to(self.max_scroll_x(), self.max_scroll_y());
                } else {
                    self.scroll_to_bottom();
                }
                true
            }
            _ => false,
        }
    }

    /// Update kinetic scrolling animation.
    ///
    /// Returns true if animation should continue.
    pub fn update_animation(&mut self) -> bool {
        if !self.scroller.is_animating() {
            return false;
        }

        if let Some((dx, dy)) = self.scroller.update() {
            if dx != 0 || dy != 0 {
                self.scroll_by(dx, dy);
            }
            self.base.update();
            return self.scroller.is_animating();
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_viewport(&self, ctx: &mut PaintContext<'_>) {
        let viewport = self.viewport_rect();
        ctx.renderer().fill_rect(viewport, self.viewport_background);
    }

    fn paint_scrollbars(&self, ctx: &mut PaintContext<'_>) {
        // Paint horizontal scrollbar
        if let Some(rect) = self.horizontal_scrollbar_rect() {
            self.paint_scrollbar(ctx, rect, true);
        }

        // Paint vertical scrollbar
        if let Some(rect) = self.vertical_scrollbar_rect() {
            self.paint_scrollbar(ctx, rect, false);
        }

        // Paint corner
        if let Some(corner) = self.corner_rect() {
            let corner_color = Color::from_rgb8(230, 230, 230);
            ctx.renderer().fill_rect(corner, corner_color);
        }
    }

    fn paint_scrollbar(&self, ctx: &mut PaintContext<'_>, rect: Rect, horizontal: bool) {
        // Track
        let track_color = Color::from_rgb8(240, 240, 240);
        ctx.renderer().fill_rect(rect, track_color);

        // Thumb
        let viewport = self.viewport_rect();
        let (content_length, viewport_length, scroll_pos, max_scroll) = if horizontal {
            (
                self.content_size.width,
                viewport.width(),
                self.scroll_x as f32,
                self.max_scroll_x() as f32,
            )
        } else {
            (
                self.content_size.height,
                viewport.height(),
                self.scroll_y as f32,
                self.max_scroll_y() as f32,
            )
        };

        if content_length <= 0.0 {
            return;
        }

        let thumb_ratio = (viewport_length / content_length).min(1.0);
        let (thumb_length, thumb_pos) = if horizontal {
            let track_length = rect.width();
            let thumb_length = (track_length * thumb_ratio).max(20.0).min(track_length);
            let available_travel = track_length - thumb_length;
            let thumb_pos = if max_scroll > 0.0 {
                rect.origin.x + (scroll_pos / max_scroll) * available_travel
            } else {
                rect.origin.x
            };
            (thumb_length, thumb_pos)
        } else {
            let track_length = rect.height();
            let thumb_length = (track_length * thumb_ratio).max(20.0).min(track_length);
            let available_travel = track_length - thumb_length;
            let thumb_pos = if max_scroll > 0.0 {
                rect.origin.y + (scroll_pos / max_scroll) * available_travel
            } else {
                rect.origin.y
            };
            (thumb_length, thumb_pos)
        };

        let thumb_rect = if horizontal {
            Rect::new(
                thumb_pos,
                rect.origin.y + 2.0,
                thumb_length,
                rect.height() - 4.0,
            )
        } else {
            Rect::new(
                rect.origin.x + 2.0,
                thumb_pos,
                rect.width() - 4.0,
                thumb_length,
            )
        };

        let thumb_color = Color::from_rgb8(180, 180, 180);
        let thumb_rrect = horizon_lattice_render::RoundedRect::new(thumb_rect, 4.0);
        ctx.renderer().fill_rounded_rect(thumb_rrect, thumb_color);
    }
}

/// Check if two rectangles intersect.
fn rects_intersect(a: Rect, b: Rect) -> bool {
    let a_right = a.origin.x + a.width();
    let a_bottom = a.origin.y + a.height();
    let b_right = b.origin.x + b.width();
    let b_bottom = b.origin.y + b.height();

    a.origin.x < b_right && a_right > b.origin.x && a.origin.y < b_bottom && a_bottom > b.origin.y
}

impl Default for ScrollArea {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ScrollArea {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ScrollArea {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::from_dimensions(200.0, 200.0).with_minimum_dimensions(50.0, 50.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_viewport(ctx);
        self.paint_scrollbars(ctx);
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
            _ => {}
        }
        false
    }
}

// Ensure ScrollArea is Send + Sync
static_assertions::assert_impl_all!(ScrollArea: Send, Sync);

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
    fn test_scroll_area_creation() {
        setup();
        let area = ScrollArea::new();
        assert_eq!(area.scroll_x(), 0);
        assert_eq!(area.scroll_y(), 0);
        assert_eq!(area.horizontal_policy(), ScrollBarPolicy::AsNeeded);
        assert_eq!(area.vertical_policy(), ScrollBarPolicy::AsNeeded);
        assert!(area.kinetic_scrolling());
    }

    #[test]
    fn test_scroll_area_builder_pattern() {
        setup();
        let area = ScrollArea::new()
            .with_content_size(Size::new(1000.0, 2000.0))
            .with_horizontal_policy(ScrollBarPolicy::AlwaysOn)
            .with_vertical_policy(ScrollBarPolicy::AlwaysOff)
            .with_kinetic_scrolling(false);

        assert_eq!(area.content_size().width, 1000.0);
        assert_eq!(area.content_size().height, 2000.0);
        assert_eq!(area.horizontal_policy(), ScrollBarPolicy::AlwaysOn);
        assert_eq!(area.vertical_policy(), ScrollBarPolicy::AlwaysOff);
        assert!(!area.kinetic_scrolling());
    }

    #[test]
    fn test_scroll_position_clamping() {
        setup();
        let mut area = ScrollArea::new().with_content_size(Size::new(1000.0, 1000.0));

        // Set geometry for viewport calculation
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        area.set_scroll_x(-100);
        assert_eq!(area.scroll_x(), 0);

        area.set_scroll_y(-100);
        assert_eq!(area.scroll_y(), 0);
    }

    #[test]
    fn test_scroll_signals() {
        setup();
        let mut area = ScrollArea::new().with_content_size(Size::new(1000.0, 1000.0));
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        let last_x = Arc::new(AtomicI32::new(-1));
        let last_y = Arc::new(AtomicI32::new(-1));

        let last_x_clone = last_x.clone();
        area.horizontal_scrolled.connect(move |&x| {
            last_x_clone.store(x, Ordering::SeqCst);
        });

        let last_y_clone = last_y.clone();
        area.vertical_scrolled.connect(move |&y| {
            last_y_clone.store(y, Ordering::SeqCst);
        });

        area.set_scroll_x(100);
        assert_eq!(last_x.load(Ordering::SeqCst), 100);

        area.set_scroll_y(200);
        assert_eq!(last_y.load(Ordering::SeqCst), 200);
    }

    #[test]
    fn test_scroll_to() {
        setup();
        let mut area = ScrollArea::new().with_content_size(Size::new(1000.0, 1000.0));
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        area.scroll_to(50, 100);
        assert_eq!(area.scroll_x(), 50);
        assert_eq!(area.scroll_y(), 100);
    }

    #[test]
    fn test_scroll_by() {
        setup();
        let mut area = ScrollArea::new().with_content_size(Size::new(1000.0, 1000.0));
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        area.scroll_by(10, 20);
        assert_eq!(area.scroll_x(), 10);
        assert_eq!(area.scroll_y(), 20);

        area.scroll_by(5, 10);
        assert_eq!(area.scroll_x(), 15);
        assert_eq!(area.scroll_y(), 30);
    }

    #[test]
    fn test_max_scroll() {
        setup();
        // Use AlwaysOff to avoid scrollbar affecting viewport size
        let mut area = ScrollArea::new()
            .with_content_size(Size::new(500.0, 800.0))
            .with_horizontal_policy(ScrollBarPolicy::AlwaysOff)
            .with_vertical_policy(ScrollBarPolicy::AlwaysOff);
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        // Max scroll should be content - viewport
        assert_eq!(area.max_scroll_x(), 300);
        assert_eq!(area.max_scroll_y(), 600);
    }

    #[test]
    fn test_scroll_policies() {
        setup();
        let mut area = ScrollArea::new();
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        // With small content, AsNeeded should hide scrollbars
        area.set_content_size(Size::new(100.0, 100.0));
        assert!(!area.is_horizontal_scrollbar_visible());
        assert!(!area.is_vertical_scrollbar_visible());

        // AlwaysOn should show scrollbars even with small content
        area.set_horizontal_policy(ScrollBarPolicy::AlwaysOn);
        area.set_vertical_policy(ScrollBarPolicy::AlwaysOn);
        assert!(area.is_horizontal_scrollbar_visible());
        assert!(area.is_vertical_scrollbar_visible());

        // AlwaysOff should hide scrollbars even with large content
        area.set_content_size(Size::new(1000.0, 1000.0));
        area.set_horizontal_policy(ScrollBarPolicy::AlwaysOff);
        area.set_vertical_policy(ScrollBarPolicy::AlwaysOff);
        assert!(!area.is_horizontal_scrollbar_visible());
        assert!(!area.is_vertical_scrollbar_visible());
    }

    #[test]
    fn test_kinetic_scroller_states() {
        let mut scroller = KineticScroller::default();

        // Initial state
        assert_eq!(scroller.state, ScrollerState::Inactive);
        assert!(!scroller.is_animating());
        assert!(!scroller.is_dragging());

        // Press
        scroller.press(100.0, 100.0, 0, 0);
        assert_eq!(scroller.state, ScrollerState::Pressed);
        assert!(scroller.is_dragging());

        // Drag
        let _ = scroller.drag(120.0, 130.0); // Small movement
        assert_eq!(scroller.state, ScrollerState::Dragging);

        // Reset
        scroller.reset();
        assert_eq!(scroller.state, ScrollerState::Inactive);
    }

    #[test]
    fn test_visible_content_rect() {
        setup();
        let mut area = ScrollArea::new()
            .with_content_size(Size::new(1000.0, 1000.0))
            .with_horizontal_policy(ScrollBarPolicy::AlwaysOff)
            .with_vertical_policy(ScrollBarPolicy::AlwaysOff);
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        // Initially at origin
        let visible = area.visible_content_rect();
        assert_eq!(visible.origin.x, 0.0);
        assert_eq!(visible.origin.y, 0.0);
        assert_eq!(visible.width(), 200.0);
        assert_eq!(visible.height(), 200.0);

        // After scrolling
        area.scroll_to(100, 150);
        let visible = area.visible_content_rect();
        assert_eq!(visible.origin.x, 100.0);
        assert_eq!(visible.origin.y, 150.0);
    }

    #[test]
    fn test_is_content_visible() {
        setup();
        let mut area = ScrollArea::new()
            .with_content_size(Size::new(1000.0, 1000.0))
            .with_horizontal_policy(ScrollBarPolicy::AlwaysOff)
            .with_vertical_policy(ScrollBarPolicy::AlwaysOff);
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        // Rectangle at origin should be visible
        assert!(area.is_content_visible(Rect::new(0.0, 0.0, 50.0, 50.0)));

        // Rectangle far away should not be visible
        assert!(!area.is_content_visible(Rect::new(500.0, 500.0, 50.0, 50.0)));

        // Rectangle partially in view should be visible
        assert!(area.is_content_visible(Rect::new(180.0, 180.0, 50.0, 50.0)));

        // After scrolling, first rect should no longer be visible
        area.scroll_to(300, 300);
        assert!(!area.is_content_visible(Rect::new(0.0, 0.0, 50.0, 50.0)));
        assert!(area.is_content_visible(Rect::new(350.0, 350.0, 50.0, 50.0)));
    }

    #[test]
    fn test_visible_row_range() {
        setup();
        let mut area = ScrollArea::new()
            .with_content_size(Size::new(200.0, 2000.0)) // 100 rows of 20px each
            .with_horizontal_policy(ScrollBarPolicy::AlwaysOff)
            .with_vertical_policy(ScrollBarPolicy::AlwaysOff);
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        // Initially showing rows 0-10 (200px viewport / 20px row height, using ceil for partial visibility)
        let (first, last) = area.visible_row_range(20.0, 100);
        assert_eq!(first, 0);
        assert_eq!(last, 10);

        // After scrolling 100px, showing rows 5-15
        area.scroll_to(0, 100);
        let (first, last) = area.visible_row_range(20.0, 100);
        assert_eq!(first, 5);
        assert_eq!(last, 15);
    }

    #[test]
    fn test_coordinate_transforms() {
        setup();
        let mut area = ScrollArea::new()
            .with_content_size(Size::new(1000.0, 1000.0))
            .with_horizontal_policy(ScrollBarPolicy::AlwaysOff)
            .with_vertical_policy(ScrollBarPolicy::AlwaysOff);
        area.widget_base_mut()
            .set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        // At origin, coordinates are the same
        let vp = area.content_to_viewport(50.0, 50.0);
        assert_eq!(vp.x, 50.0);
        assert_eq!(vp.y, 50.0);

        // After scrolling
        area.scroll_to(100, 100);
        let vp = area.content_to_viewport(150.0, 150.0);
        assert_eq!(vp.x, 50.0); // 150 - 100 = 50
        assert_eq!(vp.y, 50.0);

        // Reverse transform
        let content = area.viewport_to_content(50.0, 50.0);
        assert_eq!(content.x, 150.0); // 50 + 100 = 150
        assert_eq!(content.y, 150.0);
    }
}
