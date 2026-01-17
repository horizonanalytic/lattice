//! Touch input handling and conversion from platform events.
//!
//! This module provides conversion functions for translating platform-level
//! touch events (from winit) into Horizon Lattice widget events.
//!
//! # Usage
//!
//! The main entry point is [`TouchInputHandler`], which manages touch state
//! and converts raw touch events into widget events.
//!
//! ```ignore
//! use horizon_lattice::widget::touch::TouchInputHandler;
//!
//! let mut handler = TouchInputHandler::new();
//!
//! // When receiving a winit touch event:
//! if let Some(event) = handler.handle_touch(touch, modifiers) {
//!     // Dispatch event to the widget under touch
//! }
//! ```

use std::collections::HashMap;

use winit::event::{Force as WinitForce, Touch, TouchPhase as WinitTouchPhase};

use horizon_lattice_render::Point;

use super::events::{
    GestureState, KeyboardModifiers, PanGestureEvent, PinchGestureEvent, RotationGestureEvent,
    TouchEvent, TouchForce, TouchPhase, TouchPoint,
};

/// Converts a winit TouchPhase to a Horizon Lattice TouchPhase.
pub fn from_winit_touch_phase(phase: WinitTouchPhase) -> TouchPhase {
    match phase {
        WinitTouchPhase::Started => TouchPhase::Started,
        WinitTouchPhase::Moved => TouchPhase::Moved,
        WinitTouchPhase::Ended => TouchPhase::Ended,
        WinitTouchPhase::Cancelled => TouchPhase::Cancelled,
    }
}

/// Converts a winit Force to a Horizon Lattice TouchForce.
pub fn from_winit_force(force: WinitForce) -> TouchForce {
    match force {
        WinitForce::Calibrated {
            force,
            max_possible_force,
            altitude_angle,
        } => TouchForce::Calibrated {
            force,
            max_possible_force,
            altitude_angle,
        },
        WinitForce::Normalized(f) => TouchForce::Normalized(f),
    }
}

/// Information about an active touch.
#[derive(Debug, Clone, Copy)]
struct ActiveTouch {
    /// Touch ID.
    id: u64,
    /// Current position in window coordinates.
    position: Point,
    /// Start position in window coordinates.
    start_position: Point,
    /// Force information if available.
    force: Option<TouchForce>,
}

/// Handler for touch input that maintains touch state.
///
/// This struct provides a stateful interface for converting winit touch
/// events into widget events, tracking multiple simultaneous touch points.
#[derive(Debug)]
pub struct TouchInputHandler {
    /// Currently active touches, keyed by touch ID.
    active_touches: HashMap<u64, ActiveTouch>,
    /// Current keyboard modifier state.
    modifiers: KeyboardModifiers,
}

impl Default for TouchInputHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl TouchInputHandler {
    /// Creates a new touch input handler.
    pub fn new() -> Self {
        Self {
            active_touches: HashMap::new(),
            modifiers: KeyboardModifiers::NONE,
        }
    }

    /// Updates the keyboard modifier state.
    pub fn update_modifiers(&mut self, modifiers: KeyboardModifiers) {
        self.modifiers = modifiers;
    }

    /// Gets the current keyboard modifier state.
    pub fn modifiers(&self) -> KeyboardModifiers {
        self.modifiers
    }

    /// Gets the number of active touches.
    pub fn active_touch_count(&self) -> usize {
        self.active_touches.len()
    }

    /// Gets an iterator over active touch IDs.
    pub fn active_touch_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.active_touches.keys().copied()
    }

    /// Gets the position of an active touch by ID.
    pub fn get_touch_position(&self, id: u64) -> Option<Point> {
        self.active_touches.get(&id).map(|t| t.position)
    }

    /// Handles a touch event from winit.
    ///
    /// Returns a `TouchEvent` with the updated touch point information.
    ///
    /// # Arguments
    ///
    /// * `touch` - The winit Touch event
    /// * `global_pos` - The touch position in global screen coordinates (if available)
    pub fn handle_touch(&mut self, touch: Touch, global_pos: Option<Point>) -> TouchEvent {
        let phase = from_winit_touch_phase(touch.phase);
        let window_pos = Point::new(touch.location.x as f32, touch.location.y as f32);
        let global = global_pos.unwrap_or(window_pos);
        let force = touch.force.map(from_winit_force);

        // Update internal tracking
        match phase {
            TouchPhase::Started => {
                self.active_touches.insert(
                    touch.id,
                    ActiveTouch {
                        id: touch.id,
                        position: window_pos,
                        start_position: window_pos,
                        force,
                    },
                );
            }
            TouchPhase::Moved => {
                if let Some(active) = self.active_touches.get_mut(&touch.id) {
                    active.position = window_pos;
                    active.force = force;
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.active_touches.remove(&touch.id);
            }
        }

        // Create the touch point
        let point = if let Some(f) = force {
            TouchPoint::with_force(
                touch.id,
                window_pos, // local_pos will be calculated during dispatch
                window_pos,
                global,
                phase,
                f,
            )
        } else {
            TouchPoint::new(
                touch.id,
                window_pos, // local_pos will be calculated during dispatch
                window_pos,
                global,
                phase,
            )
        };

        TouchEvent::new(point, self.modifiers)
    }

    /// Resets the handler state, clearing all active touches.
    pub fn reset(&mut self) {
        self.active_touches.clear();
    }
}

/// Converts winit TouchPhase to GestureState.
pub fn from_winit_touch_phase_to_gesture_state(phase: WinitTouchPhase) -> GestureState {
    match phase {
        WinitTouchPhase::Started => GestureState::Started,
        WinitTouchPhase::Moved => GestureState::Updated,
        WinitTouchPhase::Ended => GestureState::Ended,
        WinitTouchPhase::Cancelled => GestureState::Cancelled,
    }
}

/// Handler for platform gesture events (from winit).
///
/// This handles native gesture events from trackpad/touch devices,
/// such as PinchGesture, RotationGesture, and PanGesture.
#[derive(Debug)]
pub struct PlatformGestureHandler {
    /// Current keyboard modifier state.
    modifiers: KeyboardModifiers,
    /// Accumulated pinch scale.
    pinch_scale: f64,
    /// Accumulated rotation in radians.
    rotation: f64,
    /// Current gesture center position.
    gesture_position: Point,
    /// Accumulated pan translation.
    pan_translation: Point,
    /// Previous pan position for delta calculation.
    prev_pan_position: Option<Point>,
}

impl Default for PlatformGestureHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformGestureHandler {
    /// Creates a new platform gesture handler.
    pub fn new() -> Self {
        Self {
            modifiers: KeyboardModifiers::NONE,
            pinch_scale: 1.0,
            rotation: 0.0,
            gesture_position: Point::new(0.0, 0.0),
            pan_translation: Point::new(0.0, 0.0),
            prev_pan_position: None,
        }
    }

    /// Updates the keyboard modifier state.
    pub fn update_modifiers(&mut self, modifiers: KeyboardModifiers) {
        self.modifiers = modifiers;
    }

    /// Sets the gesture center position (typically cursor position).
    pub fn set_position(&mut self, position: Point) {
        self.gesture_position = position;
    }

    /// Handles a pinch gesture event from winit.
    ///
    /// # Arguments
    ///
    /// * `delta` - The pinch delta (positive for zoom in, negative for zoom out)
    /// * `phase` - The touch phase
    /// * `global_pos` - Global screen coordinates (if available)
    pub fn handle_pinch_gesture(
        &mut self,
        delta: f64,
        phase: WinitTouchPhase,
        global_pos: Option<Point>,
    ) -> PinchGestureEvent {
        let state = from_winit_touch_phase_to_gesture_state(phase);

        match state {
            GestureState::Started => {
                self.pinch_scale = 1.0;
            }
            GestureState::Updated | GestureState::Ended => {
                self.pinch_scale += delta;
            }
            GestureState::Cancelled => {
                self.pinch_scale = 1.0;
            }
        }

        let global = global_pos.unwrap_or(self.gesture_position);

        PinchGestureEvent::new(
            self.gesture_position,
            self.gesture_position,
            global,
            self.pinch_scale,
            delta,
            state,
            self.modifiers,
        )
    }

    /// Handles a rotation gesture event from winit.
    ///
    /// # Arguments
    ///
    /// * `delta` - The rotation delta in radians
    /// * `phase` - The touch phase
    /// * `global_pos` - Global screen coordinates (if available)
    pub fn handle_rotation_gesture(
        &mut self,
        delta: f32,
        phase: WinitTouchPhase,
        global_pos: Option<Point>,
    ) -> RotationGestureEvent {
        let state = from_winit_touch_phase_to_gesture_state(phase);

        match state {
            GestureState::Started => {
                self.rotation = 0.0;
            }
            GestureState::Updated | GestureState::Ended => {
                self.rotation += delta as f64;
            }
            GestureState::Cancelled => {
                self.rotation = 0.0;
            }
        }

        let global = global_pos.unwrap_or(self.gesture_position);

        RotationGestureEvent::new(
            self.gesture_position,
            self.gesture_position,
            global,
            self.rotation,
            delta as f64,
            state,
            self.modifiers,
        )
    }

    /// Handles a pan gesture event from winit.
    ///
    /// # Arguments
    ///
    /// * `delta` - The pan delta (x, y)
    /// * `phase` - The touch phase
    /// * `global_pos` - Global screen coordinates (if available)
    pub fn handle_pan_gesture(
        &mut self,
        delta: (f32, f32),
        phase: WinitTouchPhase,
        global_pos: Option<Point>,
    ) -> PanGestureEvent {
        let state = from_winit_touch_phase_to_gesture_state(phase);
        let delta_point = Point::new(delta.0, delta.1);

        match state {
            GestureState::Started => {
                self.pan_translation = Point::new(0.0, 0.0);
                self.prev_pan_position = Some(self.gesture_position);
            }
            GestureState::Updated | GestureState::Ended => {
                self.pan_translation.x += delta.0;
                self.pan_translation.y += delta.1;
            }
            GestureState::Cancelled => {
                self.pan_translation = Point::new(0.0, 0.0);
                self.prev_pan_position = None;
            }
        }

        let global = global_pos.unwrap_or(self.gesture_position);

        // Calculate velocity (simplified - would need time tracking for accurate velocity)
        let velocity = Point::new(delta.0 * 60.0, delta.1 * 60.0); // Approximate 60fps

        let event = PanGestureEvent::new(
            self.gesture_position,
            self.gesture_position,
            global,
            self.pan_translation,
            delta_point,
            velocity,
            state,
            self.modifiers,
        );

        if state == GestureState::Ended || state == GestureState::Cancelled {
            self.prev_pan_position = None;
        }

        event
    }

    /// Resets the handler state.
    pub fn reset(&mut self) {
        self.pinch_scale = 1.0;
        self.rotation = 0.0;
        self.pan_translation = Point::new(0.0, 0.0);
        self.prev_pan_position = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use winit::dpi::PhysicalPosition;
    use winit::event::DeviceId;

    fn make_touch(id: u64, phase: WinitTouchPhase, x: f64, y: f64) -> Touch {
        Touch {
            device_id: DeviceId::dummy(),
            phase,
            location: PhysicalPosition::new(x, y),
            force: None,
            id,
        }
    }

    #[test]
    fn test_touch_phase_conversion() {
        assert_eq!(
            from_winit_touch_phase(WinitTouchPhase::Started),
            TouchPhase::Started
        );
        assert_eq!(
            from_winit_touch_phase(WinitTouchPhase::Moved),
            TouchPhase::Moved
        );
        assert_eq!(
            from_winit_touch_phase(WinitTouchPhase::Ended),
            TouchPhase::Ended
        );
        assert_eq!(
            from_winit_touch_phase(WinitTouchPhase::Cancelled),
            TouchPhase::Cancelled
        );
    }

    #[test]
    fn test_touch_tracking() {
        let mut handler = TouchInputHandler::new();

        // Start touch
        let touch = make_touch(1, WinitTouchPhase::Started, 100.0, 200.0);
        let event = handler.handle_touch(touch, None);

        assert_eq!(event.points.len(), 1);
        assert_eq!(event.points[0].id, 1);
        assert_eq!(event.points[0].phase, TouchPhase::Started);
        assert_eq!(handler.active_touch_count(), 1);

        // Move touch
        let touch = make_touch(1, WinitTouchPhase::Moved, 150.0, 250.0);
        let event = handler.handle_touch(touch, None);

        assert_eq!(event.points[0].phase, TouchPhase::Moved);
        assert_eq!(event.points[0].window_pos, Point::new(150.0, 250.0));
        assert_eq!(handler.active_touch_count(), 1);

        // End touch
        let touch = make_touch(1, WinitTouchPhase::Ended, 150.0, 250.0);
        let event = handler.handle_touch(touch, None);

        assert_eq!(event.points[0].phase, TouchPhase::Ended);
        assert_eq!(handler.active_touch_count(), 0);
    }

    #[test]
    fn test_multi_touch() {
        let mut handler = TouchInputHandler::new();

        // Start first touch
        let touch1 = make_touch(1, WinitTouchPhase::Started, 100.0, 100.0);
        handler.handle_touch(touch1, None);
        assert_eq!(handler.active_touch_count(), 1);

        // Start second touch
        let touch2 = make_touch(2, WinitTouchPhase::Started, 200.0, 200.0);
        handler.handle_touch(touch2, None);
        assert_eq!(handler.active_touch_count(), 2);

        // End first touch
        let touch1 = make_touch(1, WinitTouchPhase::Ended, 100.0, 100.0);
        handler.handle_touch(touch1, None);
        assert_eq!(handler.active_touch_count(), 1);

        // Cancel second touch
        let touch2 = make_touch(2, WinitTouchPhase::Cancelled, 200.0, 200.0);
        handler.handle_touch(touch2, None);
        assert_eq!(handler.active_touch_count(), 0);
    }

    #[test]
    fn test_pinch_gesture() {
        let mut handler = PlatformGestureHandler::new();
        handler.set_position(Point::new(100.0, 100.0));

        // Start gesture
        let event = handler.handle_pinch_gesture(0.0, WinitTouchPhase::Started, None);
        assert_eq!(event.state, GestureState::Started);
        assert!((event.scale - 1.0).abs() < f64::EPSILON);

        // Update gesture
        let event = handler.handle_pinch_gesture(0.5, WinitTouchPhase::Moved, None);
        assert_eq!(event.state, GestureState::Updated);
        assert!((event.scale - 1.5).abs() < f64::EPSILON);
        assert!((event.delta - 0.5).abs() < f64::EPSILON);

        // End gesture
        let event = handler.handle_pinch_gesture(0.2, WinitTouchPhase::Ended, None);
        assert_eq!(event.state, GestureState::Ended);
        assert!((event.scale - 1.7).abs() < f64::EPSILON);
    }

    #[test]
    fn test_rotation_gesture() {
        let mut handler = PlatformGestureHandler::new();
        handler.set_position(Point::new(100.0, 100.0));

        // Start gesture
        let event = handler.handle_rotation_gesture(0.0, WinitTouchPhase::Started, None);
        assert_eq!(event.state, GestureState::Started);
        assert!(event.rotation.abs() < 1e-6);

        // Update gesture
        let event = handler.handle_rotation_gesture(0.5, WinitTouchPhase::Moved, None);
        assert_eq!(event.state, GestureState::Updated);
        assert!((event.rotation - 0.5).abs() < 1e-6);

        // End gesture - note: 0.5 + (-0.2) in f32 arithmetic
        let event = handler.handle_rotation_gesture(-0.2, WinitTouchPhase::Ended, None);
        assert_eq!(event.state, GestureState::Ended);
        // Use a tolerance that accounts for f32 -> f64 conversion
        assert!((event.rotation - 0.3).abs() < 1e-5);
    }

    #[test]
    fn test_reset() {
        let mut handler = TouchInputHandler::new();

        // Start some touches
        let touch = make_touch(1, WinitTouchPhase::Started, 100.0, 100.0);
        handler.handle_touch(touch, None);
        let touch = make_touch(2, WinitTouchPhase::Started, 200.0, 200.0);
        handler.handle_touch(touch, None);

        assert_eq!(handler.active_touch_count(), 2);

        handler.reset();
        assert_eq!(handler.active_touch_count(), 0);
    }
}
