//! Mouse input handling and conversion from platform events.
//!
//! This module provides conversion functions for translating platform-level
//! mouse events (from winit) into Horizon Lattice widget events.
//!
//! # Usage
//!
//! The main entry point is [`MouseInputHandler`], which manages mouse state
//! and converts raw mouse events into widget events.
//!
//! ```ignore
//! use horizon_lattice::widget::mouse::MouseInputHandler;
//!
//! let mut handler = MouseInputHandler::new();
//!
//! // When receiving a winit cursor moved event:
//! if let Some(event) = handler.handle_cursor_moved(position, modifiers) {
//!     // Dispatch event to the widget under cursor
//! }
//!
//! // When receiving a winit mouse input event:
//! if let Some(event) = handler.handle_mouse_input(state, button, modifiers) {
//!     // Dispatch event to the appropriate widget
//! }
//! ```

use std::time::{Duration, Instant};

use winit::event::{ElementState, MouseButton as WinitMouseButton, MouseScrollDelta};

use horizon_lattice_render::Point;

use super::events::{
    EnterEvent, KeyboardModifiers, LeaveEvent, MouseButton, MouseDoubleClickEvent, MouseMoveEvent,
    MousePressEvent, MouseReleaseEvent, WheelEvent,
};

/// Default double-click time threshold in milliseconds.
///
/// Two clicks must occur within this time to be considered a double-click.
pub const DEFAULT_DOUBLE_CLICK_TIME_MS: u64 = 500;

/// Default double-click distance threshold in pixels.
///
/// Two clicks must occur within this distance to be considered a double-click.
pub const DEFAULT_DOUBLE_CLICK_DISTANCE: f32 = 5.0;

/// Converts a winit mouse button to a Horizon Lattice MouseButton.
pub fn from_winit_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
    match button {
        WinitMouseButton::Left => Some(MouseButton::Left),
        WinitMouseButton::Right => Some(MouseButton::Right),
        WinitMouseButton::Middle => Some(MouseButton::Middle),
        WinitMouseButton::Back => Some(MouseButton::Button4),
        WinitMouseButton::Forward => Some(MouseButton::Button5),
        WinitMouseButton::Other(_) => None, // Unknown button
    }
}

/// State for tracking a potential double-click.
#[derive(Debug, Clone)]
struct ClickState {
    /// The button that was clicked.
    button: MouseButton,
    /// The position of the click.
    position: Point,
    /// The time of the click.
    time: Instant,
}

/// Mouse input event type returned by the handler.
#[derive(Debug)]
pub enum MouseEvent {
    /// Mouse button was pressed.
    Press(MousePressEvent),
    /// Mouse button was released.
    Release(MouseReleaseEvent),
    /// Mouse button was double-clicked.
    DoubleClick(MouseDoubleClickEvent),
    /// Mouse cursor moved.
    Move(MouseMoveEvent),
    /// Mouse wheel was scrolled.
    Wheel(WheelEvent),
    /// Mouse cursor entered the window.
    Enter(EnterEvent),
    /// Mouse cursor left the window.
    Leave(LeaveEvent),
}

/// Handler for mouse input that maintains mouse state.
///
/// This struct provides a stateful interface for converting winit mouse
/// events into widget events, tracking button state, position, and
/// detecting double-clicks.
#[derive(Debug)]
pub struct MouseInputHandler {
    /// Current mouse position in window coordinates.
    current_position: Point,
    /// Previous mouse position for delta calculations.
    previous_position: Point,
    /// Currently pressed buttons as a bitfield.
    pressed_buttons: u8,
    /// Current keyboard modifier state.
    modifiers: KeyboardModifiers,
    /// Last click for double-click detection.
    last_click: Option<ClickState>,
    /// Double-click time threshold.
    double_click_time: Duration,
    /// Double-click distance threshold.
    double_click_distance: f32,
    /// Whether the cursor is currently inside the window.
    cursor_in_window: bool,
}

impl Default for MouseInputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl MouseInputHandler {
    /// Creates a new mouse input handler with default settings.
    pub fn new() -> Self {
        Self {
            current_position: Point::new(0.0, 0.0),
            previous_position: Point::new(0.0, 0.0),
            pressed_buttons: 0,
            modifiers: KeyboardModifiers::NONE,
            last_click: None,
            double_click_time: Duration::from_millis(DEFAULT_DOUBLE_CLICK_TIME_MS),
            double_click_distance: DEFAULT_DOUBLE_CLICK_DISTANCE,
            cursor_in_window: false,
        }
    }

    /// Sets the double-click time threshold.
    pub fn set_double_click_time(&mut self, duration: Duration) {
        self.double_click_time = duration;
    }

    /// Sets the double-click distance threshold.
    pub fn set_double_click_distance(&mut self, distance: f32) {
        self.double_click_distance = distance;
    }

    /// Gets the current mouse position in window coordinates.
    pub fn position(&self) -> Point {
        self.current_position
    }

    /// Gets the delta movement since the last position update.
    pub fn delta(&self) -> Point {
        Point::new(
            self.current_position.x - self.previous_position.x,
            self.current_position.y - self.previous_position.y,
        )
    }

    /// Gets the currently pressed buttons as a bitfield.
    pub fn pressed_buttons(&self) -> u8 {
        self.pressed_buttons
    }

    /// Checks if a specific button is currently pressed.
    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        (self.pressed_buttons & (1 << button as u8)) != 0
    }

    /// Checks if the cursor is currently inside the window.
    pub fn is_cursor_in_window(&self) -> bool {
        self.cursor_in_window
    }

    /// Updates the keyboard modifier state.
    pub fn update_modifiers(&mut self, modifiers: KeyboardModifiers) {
        self.modifiers = modifiers;
    }

    /// Gets the current keyboard modifier state.
    pub fn modifiers(&self) -> KeyboardModifiers {
        self.modifiers
    }

    /// Handles a cursor moved event from winit.
    ///
    /// Returns a `MouseMoveEvent` with the updated position information.
    ///
    /// # Arguments
    ///
    /// * `window_pos` - The cursor position in window coordinates (from winit PhysicalPosition)
    /// * `global_pos` - The cursor position in global screen coordinates (if available)
    pub fn handle_cursor_moved(
        &mut self,
        window_pos: Point,
        global_pos: Option<Point>,
    ) -> MouseMoveEvent {
        self.previous_position = self.current_position;
        self.current_position = window_pos;

        // If global position is not provided, use window position as fallback
        let global = global_pos.unwrap_or(window_pos);

        MouseMoveEvent::new(
            window_pos, // local_pos will be calculated during dispatch
            window_pos,
            global,
            self.pressed_buttons,
            self.modifiers,
        )
    }

    /// Handles a mouse input (button press/release) event from winit.
    ///
    /// Returns either a press, release, or double-click event depending on the
    /// button state and timing.
    ///
    /// # Arguments
    ///
    /// * `state` - The button state (pressed or released)
    /// * `button` - The winit mouse button that changed
    /// * `global_pos` - The cursor position in global screen coordinates (if available)
    pub fn handle_mouse_input(
        &mut self,
        state: ElementState,
        button: WinitMouseButton,
        global_pos: Option<Point>,
    ) -> Option<MouseEvent> {
        let button = from_winit_mouse_button(button)?;
        let global = global_pos.unwrap_or(self.current_position);

        match state {
            ElementState::Pressed => {
                // Update button state
                self.pressed_buttons |= 1 << button as u8;

                // Check for double-click
                let is_double_click = if let Some(ref last) = self.last_click {
                    last.button == button
                        && last.time.elapsed() < self.double_click_time
                        && self.distance_to(last.position) < self.double_click_distance
                } else {
                    false
                };

                if is_double_click {
                    // Clear last click state - don't allow triple-click to be double-double
                    self.last_click = None;

                    Some(MouseEvent::DoubleClick(MouseDoubleClickEvent::new(
                        button,
                        self.current_position, // local_pos calculated during dispatch
                        self.current_position,
                        global,
                        self.modifiers,
                    )))
                } else {
                    // Record this click for potential double-click
                    self.last_click = Some(ClickState {
                        button,
                        position: self.current_position,
                        time: Instant::now(),
                    });

                    Some(MouseEvent::Press(MousePressEvent::new(
                        button,
                        self.current_position, // local_pos calculated during dispatch
                        self.current_position,
                        global,
                        self.modifiers,
                    )))
                }
            }
            ElementState::Released => {
                // Update button state
                self.pressed_buttons &= !(1 << button as u8);

                Some(MouseEvent::Release(MouseReleaseEvent::new(
                    button,
                    self.current_position, // local_pos calculated during dispatch
                    self.current_position,
                    global,
                    self.modifiers,
                )))
            }
        }
    }

    /// Handles a mouse wheel event from winit.
    ///
    /// # Arguments
    ///
    /// * `delta` - The scroll delta from winit
    pub fn handle_mouse_wheel(&mut self, delta: MouseScrollDelta) -> WheelEvent {
        let (delta_x, delta_y) = match delta {
            MouseScrollDelta::LineDelta(x, y) => {
                // LineDelta is in "lines", typically -1 to 1
                // Scale to a reasonable pixel equivalent
                (x * 20.0, y * 20.0)
            }
            MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
        };

        WheelEvent::new(
            self.current_position, // local_pos calculated during dispatch
            self.current_position,
            delta_x,
            delta_y,
            self.modifiers,
        )
    }

    /// Handles a cursor entered event from winit.
    pub fn handle_cursor_entered(&mut self) -> EnterEvent {
        self.cursor_in_window = true;
        EnterEvent::new(self.current_position)
    }

    /// Handles a cursor left event from winit.
    pub fn handle_cursor_left(&mut self) -> LeaveEvent {
        self.cursor_in_window = false;
        // Reset last click state when cursor leaves
        self.last_click = None;
        LeaveEvent::new()
    }

    /// Calculates the distance from the current position to a point.
    fn distance_to(&self, point: Point) -> f32 {
        let dx = self.current_position.x - point.x;
        let dy = self.current_position.y - point.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Resets the handler state.
    ///
    /// This clears button state, position history, and double-click tracking.
    pub fn reset(&mut self) {
        self.current_position = Point::new(0.0, 0.0);
        self.previous_position = Point::new(0.0, 0.0);
        self.pressed_buttons = 0;
        self.last_click = None;
        self.cursor_in_window = false;
    }
}

/// Converts winit modifiers to Horizon Lattice KeyboardModifiers.
///
/// This is a convenience re-export from the keyboard module for use
/// when handling mouse events with modifier state.
pub use super::keyboard::from_winit_modifiers;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mouse_button_conversion() {
        assert_eq!(
            from_winit_mouse_button(WinitMouseButton::Left),
            Some(MouseButton::Left)
        );
        assert_eq!(
            from_winit_mouse_button(WinitMouseButton::Right),
            Some(MouseButton::Right)
        );
        assert_eq!(
            from_winit_mouse_button(WinitMouseButton::Middle),
            Some(MouseButton::Middle)
        );
        assert_eq!(
            from_winit_mouse_button(WinitMouseButton::Back),
            Some(MouseButton::Button4)
        );
        assert_eq!(
            from_winit_mouse_button(WinitMouseButton::Forward),
            Some(MouseButton::Button5)
        );
        assert_eq!(from_winit_mouse_button(WinitMouseButton::Other(99)), None);
    }

    #[test]
    fn test_button_state_tracking() {
        let mut handler = MouseInputHandler::new();

        assert!(!handler.is_button_pressed(MouseButton::Left));

        // Simulate left button press
        handler.pressed_buttons |= 1 << MouseButton::Left as u8;
        assert!(handler.is_button_pressed(MouseButton::Left));
        assert!(!handler.is_button_pressed(MouseButton::Right));

        // Simulate right button press
        handler.pressed_buttons |= 1 << MouseButton::Right as u8;
        assert!(handler.is_button_pressed(MouseButton::Left));
        assert!(handler.is_button_pressed(MouseButton::Right));

        // Simulate left button release
        handler.pressed_buttons &= !(1 << MouseButton::Left as u8);
        assert!(!handler.is_button_pressed(MouseButton::Left));
        assert!(handler.is_button_pressed(MouseButton::Right));
    }

    #[test]
    fn test_cursor_movement() {
        let mut handler = MouseInputHandler::new();

        let event1 = handler.handle_cursor_moved(Point::new(100.0, 200.0), None);
        assert_eq!(event1.window_pos, Point::new(100.0, 200.0));
        assert_eq!(handler.position(), Point::new(100.0, 200.0));

        let event2 = handler.handle_cursor_moved(Point::new(150.0, 250.0), None);
        assert_eq!(event2.window_pos, Point::new(150.0, 250.0));
        assert_eq!(handler.delta(), Point::new(50.0, 50.0));
    }

    #[test]
    fn test_double_click_detection() {
        let mut handler = MouseInputHandler::new();
        handler.handle_cursor_moved(Point::new(100.0, 100.0), None);

        // First click
        let event1 =
            handler.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Left, None);
        assert!(matches!(event1, Some(MouseEvent::Press(_))));

        // Release
        let _ = handler.handle_mouse_input(ElementState::Released, WinitMouseButton::Left, None);

        // Second click quickly should be double-click
        let event2 =
            handler.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Left, None);
        assert!(matches!(event2, Some(MouseEvent::DoubleClick(_))));
    }

    #[test]
    fn test_double_click_different_buttons() {
        let mut handler = MouseInputHandler::new();
        handler.handle_cursor_moved(Point::new(100.0, 100.0), None);

        // First click with left
        let _ = handler.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Left, None);
        let _ = handler.handle_mouse_input(ElementState::Released, WinitMouseButton::Left, None);

        // Second click with right - should NOT be double-click
        let event2 =
            handler.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Right, None);
        assert!(matches!(event2, Some(MouseEvent::Press(_))));
    }

    #[test]
    fn test_double_click_too_far() {
        let mut handler = MouseInputHandler::new();
        handler.set_double_click_distance(5.0);

        // First click
        handler.handle_cursor_moved(Point::new(100.0, 100.0), None);
        let _ = handler.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Left, None);
        let _ = handler.handle_mouse_input(ElementState::Released, WinitMouseButton::Left, None);

        // Move cursor far away
        handler.handle_cursor_moved(Point::new(200.0, 200.0), None);

        // Second click should NOT be double-click (too far)
        let event2 =
            handler.handle_mouse_input(ElementState::Pressed, WinitMouseButton::Left, None);
        assert!(matches!(event2, Some(MouseEvent::Press(_))));
    }

    #[test]
    fn test_wheel_event_line_delta() {
        let mut handler = MouseInputHandler::new();
        handler.handle_cursor_moved(Point::new(100.0, 100.0), None);

        let event = handler.handle_mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
        assert_eq!(event.delta_x, 0.0);
        assert_eq!(event.delta_y, -20.0); // Scaled by 20
    }

    #[test]
    fn test_wheel_event_pixel_delta() {
        let mut handler = MouseInputHandler::new();
        handler.handle_cursor_moved(Point::new(100.0, 100.0), None);

        let event = handler.handle_mouse_wheel(MouseScrollDelta::PixelDelta(
            winit::dpi::PhysicalPosition::new(10.0, -15.0),
        ));
        assert_eq!(event.delta_x, 10.0);
        assert_eq!(event.delta_y, -15.0);
    }

    #[test]
    fn test_cursor_enter_leave() {
        let mut handler = MouseInputHandler::new();

        assert!(!handler.is_cursor_in_window());

        handler.handle_cursor_entered();
        assert!(handler.is_cursor_in_window());

        handler.handle_cursor_left();
        assert!(!handler.is_cursor_in_window());
    }

    #[test]
    fn test_reset() {
        let mut handler = MouseInputHandler::new();
        handler.handle_cursor_moved(Point::new(100.0, 100.0), None);
        handler.pressed_buttons = 0b11;
        handler.cursor_in_window = true;

        handler.reset();

        assert_eq!(handler.position(), Point::new(0.0, 0.0));
        assert_eq!(handler.pressed_buttons(), 0);
        assert!(!handler.is_cursor_in_window());
    }
}
