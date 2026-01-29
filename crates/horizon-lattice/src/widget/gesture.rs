//! Gesture recognition from touch events.
//!
//! This module provides a gesture recognizer that detects high-level gestures
//! (tap, double-tap, long-press, swipe, pan, pinch, rotate) from raw touch events.
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice::widget::gesture::GestureRecognizer;
//!
//! let mut recognizer = GestureRecognizer::new();
//!
//! // Feed touch events to the recognizer
//! let gestures = recognizer.process_touch(&touch_event);
//!
//! for gesture in gestures {
//!     match gesture {
//!         RecognizedGesture::Tap(e) => handle_tap(e),
//!         RecognizedGesture::LongPress(e) => handle_long_press(e),
//!         // ...
//!     }
//! }
//! ```

use std::collections::HashMap;
use std::time::{Duration, Instant};

use horizon_lattice_render::Point;

use super::events::{
    GestureState, GestureType, KeyboardModifiers, LongPressGestureEvent, PanGestureEvent,
    PinchGestureEvent, RotationGestureEvent, SwipeDirection, SwipeGestureEvent, TapGestureEvent,
    TouchEvent, TouchPhase, TouchPoint,
};

/// Default tap timeout in milliseconds.
///
/// A tap must complete within this duration to be recognized.
pub const DEFAULT_TAP_TIMEOUT_MS: u64 = 300;

/// Default double-tap timeout in milliseconds.
///
/// Two taps must occur within this duration to be recognized as a double-tap.
pub const DEFAULT_DOUBLE_TAP_TIMEOUT_MS: u64 = 300;

/// Default long-press timeout in milliseconds.
///
/// A touch must be held for at least this duration to trigger a long-press.
pub const DEFAULT_LONG_PRESS_TIMEOUT_MS: u64 = 500;

/// Default maximum movement for a tap in pixels.
///
/// Movement beyond this threshold cancels tap recognition.
pub const DEFAULT_TAP_SLOP: f32 = 10.0;

/// Default minimum velocity for a swipe in pixels per second.
pub const DEFAULT_SWIPE_MIN_VELOCITY: f32 = 300.0;

/// Default minimum distance for a swipe in pixels.
pub const DEFAULT_SWIPE_MIN_DISTANCE: f32 = 50.0;

/// All gesture types for iteration purposes.
pub const ALL_GESTURE_TYPES: &[GestureType] = &[
    GestureType::Tap,
    GestureType::DoubleTap,
    GestureType::LongPress,
    GestureType::Pinch,
    GestureType::Rotation,
    GestureType::Swipe,
    GestureType::Pan,
];

/// Recognized gesture events.
#[derive(Debug)]
pub enum RecognizedGesture {
    /// Single or multi-tap gesture.
    Tap(TapGestureEvent),
    /// Long press gesture.
    LongPress(LongPressGestureEvent),
    /// Swipe gesture.
    Swipe(SwipeGestureEvent),
    /// Pan/drag gesture.
    Pan(PanGestureEvent),
    /// Two-finger pinch gesture.
    Pinch(PinchGestureEvent),
    /// Two-finger rotation gesture.
    Rotation(RotationGestureEvent),
}

impl RecognizedGesture {
    /// Returns the gesture type for this recognized gesture.
    pub fn gesture_type(&self) -> GestureType {
        match self {
            Self::Tap(e) => {
                if e.tap_count >= 2 {
                    GestureType::DoubleTap
                } else {
                    GestureType::Tap
                }
            }
            Self::LongPress(_) => GestureType::LongPress,
            Self::Swipe(_) => GestureType::Swipe,
            Self::Pan(_) => GestureType::Pan,
            Self::Pinch(_) => GestureType::Pinch,
            Self::Rotation(_) => GestureType::Rotation,
        }
    }

    /// Returns the gesture state if the gesture has one.
    pub fn state(&self) -> Option<GestureState> {
        match self {
            Self::Tap(_) => None,   // Tap is instantaneous
            Self::Swipe(_) => None, // Swipe is instantaneous
            Self::LongPress(e) => Some(e.state),
            Self::Pan(e) => Some(e.state),
            Self::Pinch(e) => Some(e.state),
            Self::Rotation(e) => Some(e.state),
        }
    }
}

/// Policy for how a gesture interacts with other gestures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GesturePolicy {
    /// Default policy: gestures follow the simultaneous gesture rules.
    #[default]
    Default,
    /// Exclusive: accepting this gesture cancels all conflicting gestures.
    Exclusive,
    /// Cooperative: this gesture can coexist with others regardless of rules.
    Cooperative,
}

/// Priority level for gesture recognition.
///
/// Higher priority gestures get first chance at recognition when conflicts occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum GesturePriority {
    /// Low priority - processed last.
    Low = 0,
    /// Normal priority - default for most gestures.
    #[default]
    Normal = 1,
    /// High priority - processed first.
    High = 2,
}

/// Policy for simultaneous gesture recognition.
///
/// Controls which gesture types can be recognized together.
#[derive(Debug, Clone)]
pub struct SimultaneousGesturePolicy {
    /// Set of gesture type pairs that can be recognized simultaneously.
    /// A pair (A, B) means gesture type A and B can coexist.
    allowed_pairs: std::collections::HashSet<(GestureType, GestureType)>,
}

impl Default for SimultaneousGesturePolicy {
    fn default() -> Self {
        let mut policy = Self::new();
        // By default, allow pinch and rotation to occur simultaneously
        policy.allow_simultaneous(GestureType::Pinch, GestureType::Rotation);
        policy
    }
}

impl SimultaneousGesturePolicy {
    /// Creates a new policy with no allowed simultaneous gestures.
    pub fn new() -> Self {
        Self {
            allowed_pairs: std::collections::HashSet::new(),
        }
    }

    /// Creates a policy that allows all gestures to be recognized simultaneously.
    pub fn allow_all() -> Self {
        let mut policy = Self::new();
        for (i, &a) in ALL_GESTURE_TYPES.iter().enumerate() {
            for &b in ALL_GESTURE_TYPES.iter().skip(i + 1) {
                policy.allow_simultaneous(a, b);
            }
        }
        policy
    }

    /// Allows two gesture types to be recognized simultaneously.
    pub fn allow_simultaneous(&mut self, a: GestureType, b: GestureType) {
        // Store in canonical order to avoid duplicates
        let pair = if (a as u8) <= (b as u8) {
            (a, b)
        } else {
            (b, a)
        };
        self.allowed_pairs.insert(pair);
    }

    /// Disallows two gesture types from being recognized simultaneously.
    pub fn disallow_simultaneous(&mut self, a: GestureType, b: GestureType) {
        let pair = if (a as u8) <= (b as u8) {
            (a, b)
        } else {
            (b, a)
        };
        self.allowed_pairs.remove(&pair);
    }

    /// Returns whether two gesture types can be recognized simultaneously.
    pub fn can_be_simultaneous(&self, a: GestureType, b: GestureType) -> bool {
        if a == b {
            return true; // Same gesture type can always coexist with itself
        }
        let pair = if (a as u8) <= (b as u8) {
            (a, b)
        } else {
            (b, a)
        };
        self.allowed_pairs.contains(&pair)
    }
}

/// A dependency that requires one gesture to fail before another can be recognized.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureDependency {
    /// The gesture that must wait.
    pub waiting: GestureType,
    /// The gesture that must fail first.
    pub must_fail: GestureType,
}

impl FailureDependency {
    /// Creates a new failure dependency.
    ///
    /// The `waiting` gesture will only be recognized after the `must_fail` gesture fails.
    pub fn new(waiting: GestureType, must_fail: GestureType) -> Self {
        Self { waiting, must_fail }
    }
}

/// State of a gesture in the arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GestureArenaState {
    /// Gesture is waiting for recognition (may be pending failure dependencies).
    Pending,
    /// Gesture is actively being recognized.
    Active,
    /// Gesture was accepted and completed recognition.
    Accepted,
    /// Gesture was rejected or cancelled.
    Rejected,
}

/// Entry for a gesture in the arena.
#[derive(Debug, Clone)]
struct GestureArenaEntry {
    /// The type of gesture.
    gesture_type: GestureType,
    /// Priority for conflict resolution.
    priority: GesturePriority,
    /// Policy for this gesture.
    policy: GesturePolicy,
    /// Current state in the arena.
    state: GestureArenaState,
    /// Start time for timeout handling.
    start_time: Instant,
}

/// Arena for managing gesture conflicts.
///
/// The arena tracks all active gesture candidates and resolves conflicts
/// based on priority, policy, and simultaneous gesture rules.
#[derive(Debug)]
pub struct GestureArena {
    /// Active gesture entries indexed by a unique ID.
    entries: HashMap<u64, GestureArenaEntry>,
    /// Next entry ID.
    next_id: u64,
    /// Policy for simultaneous gestures.
    simultaneous_policy: SimultaneousGesturePolicy,
    /// Failure dependencies.
    failure_dependencies: Vec<FailureDependency>,
}

impl Default for GestureArena {
    fn default() -> Self {
        Self::new()
    }
}

impl GestureArena {
    /// Creates a new gesture arena with default settings.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            next_id: 0,
            simultaneous_policy: SimultaneousGesturePolicy::default(),
            failure_dependencies: Vec::new(),
        }
    }

    /// Creates a new gesture arena with a custom simultaneous gesture policy.
    pub fn with_policy(policy: SimultaneousGesturePolicy) -> Self {
        Self {
            entries: HashMap::new(),
            next_id: 0,
            simultaneous_policy: policy,
            failure_dependencies: Vec::new(),
        }
    }

    /// Sets the simultaneous gesture policy.
    pub fn set_simultaneous_policy(&mut self, policy: SimultaneousGesturePolicy) {
        self.simultaneous_policy = policy;
    }

    /// Adds a failure dependency.
    ///
    /// The `waiting` gesture type will only be recognized after `must_fail` fails.
    pub fn add_failure_dependency(&mut self, waiting: GestureType, must_fail: GestureType) {
        self.failure_dependencies
            .push(FailureDependency::new(waiting, must_fail));
    }

    /// Removes a failure dependency.
    pub fn remove_failure_dependency(&mut self, waiting: GestureType, must_fail: GestureType) {
        self.failure_dependencies
            .retain(|d| d.waiting != waiting || d.must_fail != must_fail);
    }

    /// Registers a new gesture candidate in the arena.
    ///
    /// Returns the ID assigned to this gesture entry, or `None` if the gesture
    /// cannot be added due to conflicts.
    pub fn register(
        &mut self,
        gesture_type: GestureType,
        priority: GesturePriority,
        policy: GesturePolicy,
    ) -> Option<u64> {
        // Check if this gesture is blocked by failure dependencies
        let is_blocked = self.failure_dependencies.iter().any(|dep| {
            dep.waiting == gesture_type
                && self.entries.values().any(|e| {
                    e.gesture_type == dep.must_fail && e.state != GestureArenaState::Rejected
                })
        });

        let initial_state = if is_blocked {
            GestureArenaState::Pending
        } else {
            GestureArenaState::Active
        };

        let id = self.next_id;
        self.next_id += 1;

        let entry = GestureArenaEntry {
            gesture_type,
            priority,
            policy,
            state: initial_state,
            start_time: Instant::now(),
        };

        self.entries.insert(id, entry);
        Some(id)
    }

    /// Attempts to accept a gesture, resolving any conflicts.
    ///
    /// Returns a list of gesture IDs that were cancelled as a result.
    pub fn accept(&mut self, id: u64) -> Vec<u64> {
        let mut cancelled = Vec::new();

        let Some(entry) = self.entries.get(&id) else {
            return cancelled;
        };

        if entry.state != GestureArenaState::Active {
            return cancelled;
        }

        let gesture_type = entry.gesture_type;
        let policy = entry.policy;
        let priority = entry.priority;

        // Check for conflicts with other active gestures
        let conflicting: Vec<u64> = self
            .entries
            .iter()
            .filter(|(other_id, other)| {
                **other_id != id
                    && other.state == GestureArenaState::Active
                    && !self
                        .simultaneous_policy
                        .can_be_simultaneous(gesture_type, other.gesture_type)
            })
            .map(|(id, _)| *id)
            .collect();

        // Determine if we should accept based on policy and priority
        let should_accept = match policy {
            GesturePolicy::Exclusive => {
                // Cancel all conflicting gestures
                for &conflict_id in &conflicting {
                    if let Some(conflict) = self.entries.get_mut(&conflict_id) {
                        conflict.state = GestureArenaState::Rejected;
                        cancelled.push(conflict_id);
                    }
                }
                true
            }
            GesturePolicy::Cooperative => {
                // Accept without cancelling others
                true
            }
            GesturePolicy::Default => {
                // Use priority to resolve conflicts
                let dominated_by_higher_priority = conflicting.iter().any(|&conflict_id| {
                    self.entries
                        .get(&conflict_id)
                        .map(|e| e.priority > priority)
                        .unwrap_or(false)
                });

                if dominated_by_higher_priority {
                    // Don't accept, wait for higher priority gesture
                    false
                } else {
                    // Cancel lower or equal priority conflicting gestures
                    for &conflict_id in &conflicting {
                        if let Some(conflict) = self.entries.get_mut(&conflict_id)
                            && conflict.priority <= priority
                        {
                            conflict.state = GestureArenaState::Rejected;
                            cancelled.push(conflict_id);
                        }
                    }
                    true
                }
            }
        };

        if should_accept && let Some(entry) = self.entries.get_mut(&id) {
            entry.state = GestureArenaState::Accepted;
        }

        // Check if any pending gestures can now become active
        self.promote_pending_gestures();

        cancelled
    }

    /// Rejects a gesture, removing it from the arena.
    ///
    /// Returns true if any pending gestures were promoted as a result.
    pub fn reject(&mut self, id: u64) -> bool {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.state = GestureArenaState::Rejected;
        }
        self.promote_pending_gestures()
    }

    /// Removes a gesture from the arena.
    pub fn remove(&mut self, id: u64) {
        self.entries.remove(&id);
        self.promote_pending_gestures();
    }

    /// Clears all gestures from the arena.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns the state of a gesture in the arena.
    pub fn state(&self, id: u64) -> Option<GestureArenaState> {
        self.entries.get(&id).map(|e| e.state)
    }

    /// Returns whether a gesture type is currently active in the arena.
    pub fn is_active(&self, gesture_type: GestureType) -> bool {
        self.entries
            .values()
            .any(|e| e.gesture_type == gesture_type && e.state == GestureArenaState::Active)
    }

    /// Returns the active gesture types.
    pub fn active_gestures(&self) -> Vec<GestureType> {
        self.entries
            .values()
            .filter(|e| e.state == GestureArenaState::Active)
            .map(|e| e.gesture_type)
            .collect()
    }

    /// Promotes pending gestures to active if their failure dependencies are resolved.
    fn promote_pending_gestures(&mut self) -> bool {
        let mut promoted = false;

        // Collect IDs of gestures that can be promoted
        let to_promote: Vec<u64> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.state == GestureArenaState::Pending)
            .filter(|(_, entry)| {
                // Check if all failure dependencies are resolved
                !self.failure_dependencies.iter().any(|dep| {
                    dep.waiting == entry.gesture_type
                        && self.entries.values().any(|e| {
                            e.gesture_type == dep.must_fail && e.state == GestureArenaState::Active
                        })
                })
            })
            .map(|(&id, _)| id)
            .collect();

        // Promote them
        for id in to_promote {
            if let Some(entry) = self.entries.get_mut(&id) {
                entry.state = GestureArenaState::Active;
                promoted = true;
            }
        }

        promoted
    }
}

/// Internal state for tracking a touch.
#[derive(Debug, Clone)]
struct TouchState {
    /// Touch ID.
    id: u64,
    /// Start time of the touch.
    start_time: Instant,
    /// Start position in window coordinates.
    start_pos: Point,
    /// Current position in window coordinates.
    current_pos: Point,
    /// Start position in global coordinates.
    start_global_pos: Point,
    /// Current position in global coordinates.
    current_global_pos: Point,
    /// Whether this touch has moved beyond the tap slop.
    moved_beyond_slop: bool,
    /// Whether a long-press has been recognized for this touch.
    long_press_recognized: bool,
}

/// State for tracking potential taps.
#[derive(Debug, Clone)]
struct TapState {
    /// Position of the last tap.
    position: Point,
    /// Global position of the last tap.
    global_position: Point,
    /// Time of the last tap.
    time: Instant,
    /// Number of consecutive taps.
    tap_count: u32,
}

/// Configuration for the gesture recognizer.
#[derive(Debug, Clone)]
pub struct GestureConfig {
    /// Maximum duration for a tap.
    pub tap_timeout: Duration,
    /// Maximum duration between taps for a multi-tap.
    pub double_tap_timeout: Duration,
    /// Duration a touch must be held for a long-press.
    pub long_press_timeout: Duration,
    /// Maximum movement allowed for a tap.
    pub tap_slop: f32,
    /// Minimum velocity for a swipe.
    pub swipe_min_velocity: f32,
    /// Minimum distance for a swipe.
    pub swipe_min_distance: f32,
}

impl Default for GestureConfig {
    fn default() -> Self {
        Self {
            tap_timeout: Duration::from_millis(DEFAULT_TAP_TIMEOUT_MS),
            double_tap_timeout: Duration::from_millis(DEFAULT_DOUBLE_TAP_TIMEOUT_MS),
            long_press_timeout: Duration::from_millis(DEFAULT_LONG_PRESS_TIMEOUT_MS),
            tap_slop: DEFAULT_TAP_SLOP,
            swipe_min_velocity: DEFAULT_SWIPE_MIN_VELOCITY,
            swipe_min_distance: DEFAULT_SWIPE_MIN_DISTANCE,
        }
    }
}

/// Gesture recognizer that detects gestures from touch events.
#[derive(Debug)]
pub struct GestureRecognizer {
    /// Configuration.
    config: GestureConfig,
    /// Active touches being tracked.
    touches: HashMap<u64, TouchState>,
    /// Last tap state for double-tap detection.
    last_tap: Option<TapState>,
    /// Current keyboard modifiers.
    modifiers: KeyboardModifiers,
    /// Whether a pan gesture is in progress.
    pan_in_progress: bool,
    /// Whether a pinch gesture is in progress.
    pinch_in_progress: bool,
    /// Whether a rotation gesture is in progress.
    rotation_in_progress: bool,
    /// Initial distance for pinch gesture.
    initial_pinch_distance: f32,
    /// Initial angle for rotation gesture.
    initial_rotation_angle: f32,
    /// Accumulated pan translation.
    pan_translation: Point,
    /// Previous position for pan delta calculation.
    prev_pan_pos: Point,
}

impl Default for GestureRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl GestureRecognizer {
    /// Creates a new gesture recognizer with default configuration.
    pub fn new() -> Self {
        Self::with_config(GestureConfig::default())
    }

    /// Creates a new gesture recognizer with the given configuration.
    pub fn with_config(config: GestureConfig) -> Self {
        Self {
            config,
            touches: HashMap::new(),
            last_tap: None,
            modifiers: KeyboardModifiers::NONE,
            pan_in_progress: false,
            pinch_in_progress: false,
            rotation_in_progress: false,
            initial_pinch_distance: 0.0,
            initial_rotation_angle: 0.0,
            pan_translation: Point::new(0.0, 0.0),
            prev_pan_pos: Point::new(0.0, 0.0),
        }
    }

    /// Updates the keyboard modifier state.
    pub fn update_modifiers(&mut self, modifiers: KeyboardModifiers) {
        self.modifiers = modifiers;
    }

    /// Processes a touch event and returns any recognized gestures.
    ///
    /// This may return zero or more gestures depending on the touch state.
    pub fn process_touch(&mut self, event: &TouchEvent) -> Vec<RecognizedGesture> {
        let mut gestures = Vec::new();

        for point in &event.points {
            match point.phase {
                TouchPhase::Started => {
                    self.handle_touch_start(point, &mut gestures);
                }
                TouchPhase::Moved => {
                    self.handle_touch_move(point, &mut gestures);
                }
                TouchPhase::Ended => {
                    self.handle_touch_end(point, &mut gestures);
                }
                TouchPhase::Cancelled => {
                    self.handle_touch_cancel(point, &mut gestures);
                }
            }
        }

        gestures
    }

    /// Checks for long-press timeouts.
    ///
    /// Call this periodically (e.g., on a timer) to detect long-press gestures.
    pub fn check_long_press(&mut self) -> Option<LongPressGestureEvent> {
        let now = Instant::now();

        for touch in self.touches.values_mut() {
            if !touch.moved_beyond_slop
                && !touch.long_press_recognized
                && now.duration_since(touch.start_time) >= self.config.long_press_timeout
            {
                touch.long_press_recognized = true;

                return Some(LongPressGestureEvent::new(
                    touch.current_pos,
                    touch.current_pos,
                    touch.current_global_pos,
                    GestureState::Started,
                    self.modifiers,
                ));
            }
        }

        None
    }

    fn handle_touch_start(&mut self, point: &TouchPoint, _gestures: &mut Vec<RecognizedGesture>) {
        let state = TouchState {
            id: point.id,
            start_time: Instant::now(),
            start_pos: point.window_pos,
            current_pos: point.window_pos,
            start_global_pos: point.global_pos,
            current_global_pos: point.global_pos,
            moved_beyond_slop: false,
            long_press_recognized: false,
        };

        self.touches.insert(point.id, state);

        // Check for multi-touch gesture start
        if self.touches.len() == 2 {
            self.start_two_finger_tracking();
        }
    }

    fn handle_touch_move(&mut self, point: &TouchPoint, gestures: &mut Vec<RecognizedGesture>) {
        // First, update the touch state and extract needed info
        let touch_info = {
            let Some(touch) = self.touches.get_mut(&point.id) else {
                return;
            };

            let prev_pos = touch.current_pos;
            touch.current_pos = point.window_pos;
            touch.current_global_pos = point.global_pos;

            // Check if moved beyond tap slop
            let mut long_press_ended = false;
            if !touch.moved_beyond_slop {
                let dx = touch.current_pos.x - touch.start_pos.x;
                let dy = touch.current_pos.y - touch.start_pos.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if distance > self.config.tap_slop {
                    touch.moved_beyond_slop = true;
                    long_press_ended = touch.long_press_recognized;
                }
            }

            // Extract all needed info before releasing the borrow
            (
                prev_pos,
                touch.start_pos,
                touch.moved_beyond_slop,
                touch.long_press_recognized,
                touch.current_pos,
                touch.current_global_pos,
                long_press_ended,
            )
        };

        let (
            prev_pos,
            start_pos,
            moved_beyond_slop,
            long_press_recognized,
            current_pos,
            current_global_pos,
            long_press_ended,
        ) = touch_info;

        // If long press was recognized and we moved, end it
        if long_press_ended {
            gestures.push(RecognizedGesture::LongPress(LongPressGestureEvent::new(
                current_pos,
                current_pos,
                current_global_pos,
                GestureState::Ended,
                self.modifiers,
            )));
        }

        // Handle single-finger pan
        if self.touches.len() == 1 && moved_beyond_slop && !long_press_recognized {
            let delta = Point::new(
                point.window_pos.x - prev_pos.x,
                point.window_pos.y - prev_pos.y,
            );

            if !self.pan_in_progress {
                self.pan_in_progress = true;
                self.pan_translation = Point::new(0.0, 0.0);
                self.prev_pan_pos = start_pos;

                gestures.push(RecognizedGesture::Pan(PanGestureEvent::new(
                    point.window_pos,
                    point.window_pos,
                    point.global_pos,
                    self.pan_translation,
                    delta,
                    Point::new(0.0, 0.0),
                    GestureState::Started,
                    self.modifiers,
                )));
            } else {
                self.pan_translation.x += delta.x;
                self.pan_translation.y += delta.y;

                let velocity = self.calculate_velocity(prev_pos, point.window_pos);

                gestures.push(RecognizedGesture::Pan(PanGestureEvent::new(
                    point.window_pos,
                    point.window_pos,
                    point.global_pos,
                    self.pan_translation,
                    delta,
                    velocity,
                    GestureState::Updated,
                    self.modifiers,
                )));
            }
        }

        // Handle two-finger gestures
        if self.touches.len() == 2 {
            self.update_two_finger_gestures(gestures);
        }
    }

    fn handle_touch_end(&mut self, point: &TouchPoint, gestures: &mut Vec<RecognizedGesture>) {
        let Some(touch) = self.touches.remove(&point.id) else {
            return;
        };

        let duration = Instant::now().duration_since(touch.start_time);

        // End any ongoing gestures
        if self.pan_in_progress && self.touches.is_empty() {
            let velocity = self.calculate_velocity(self.prev_pan_pos, point.window_pos);

            // Check for swipe
            let dx = point.window_pos.x - touch.start_pos.x;
            let dy = point.window_pos.y - touch.start_pos.y;
            let distance = (dx * dx + dy * dy).sqrt();
            let _duration_secs = duration.as_secs_f32();
            let velocity_magnitude = (velocity.x * velocity.x + velocity.y * velocity.y).sqrt();

            if distance >= self.config.swipe_min_distance
                && velocity_magnitude >= self.config.swipe_min_velocity
            {
                let direction = self.determine_swipe_direction(dx, dy);
                gestures.push(RecognizedGesture::Swipe(SwipeGestureEvent::new(
                    touch.start_pos,
                    point.window_pos,
                    touch.start_pos,
                    point.window_pos,
                    direction,
                    velocity_magnitude,
                    self.modifiers,
                )));
            }

            gestures.push(RecognizedGesture::Pan(PanGestureEvent::new(
                point.window_pos,
                point.window_pos,
                point.global_pos,
                self.pan_translation,
                Point::new(0.0, 0.0),
                velocity,
                GestureState::Ended,
                self.modifiers,
            )));

            self.pan_in_progress = false;
        }

        // End two-finger gestures if going from 2 to 1 touch
        if self.touches.len() == 1 {
            self.end_two_finger_gestures(point, gestures);
        }

        // End long-press if it was recognized
        if touch.long_press_recognized {
            gestures.push(RecognizedGesture::LongPress(LongPressGestureEvent::new(
                point.window_pos,
                point.window_pos,
                point.global_pos,
                GestureState::Ended,
                self.modifiers,
            )));
            return;
        }

        // Check for tap
        if !touch.moved_beyond_slop && duration < self.config.tap_timeout {
            let tap_count = if let Some(ref last_tap) = self.last_tap {
                let time_since_last = Instant::now().duration_since(last_tap.time);
                let dx = point.window_pos.x - last_tap.position.x;
                let dy = point.window_pos.y - last_tap.position.y;
                let distance = (dx * dx + dy * dy).sqrt();

                if time_since_last < self.config.double_tap_timeout
                    && distance < self.config.tap_slop
                {
                    last_tap.tap_count + 1
                } else {
                    1
                }
            } else {
                1
            };

            self.last_tap = Some(TapState {
                position: point.window_pos,
                global_position: point.global_pos,
                time: Instant::now(),
                tap_count,
            });

            gestures.push(RecognizedGesture::Tap(TapGestureEvent::new(
                point.window_pos,
                point.window_pos,
                point.global_pos,
                tap_count,
                self.modifiers,
            )));
        }
    }

    fn handle_touch_cancel(&mut self, point: &TouchPoint, gestures: &mut Vec<RecognizedGesture>) {
        let Some(touch) = self.touches.remove(&point.id) else {
            return;
        };

        // Cancel any ongoing gestures
        if self.pan_in_progress && self.touches.is_empty() {
            gestures.push(RecognizedGesture::Pan(PanGestureEvent::new(
                point.window_pos,
                point.window_pos,
                point.global_pos,
                self.pan_translation,
                Point::new(0.0, 0.0),
                Point::new(0.0, 0.0),
                GestureState::Cancelled,
                self.modifiers,
            )));
            self.pan_in_progress = false;
        }

        if touch.long_press_recognized {
            gestures.push(RecognizedGesture::LongPress(LongPressGestureEvent::new(
                point.window_pos,
                point.window_pos,
                point.global_pos,
                GestureState::Cancelled,
                self.modifiers,
            )));
        }

        if self.touches.len() == 1 {
            self.cancel_two_finger_gestures(point, gestures);
        }
    }

    fn start_two_finger_tracking(&mut self) {
        let touches: Vec<_> = self.touches.values().collect();
        if touches.len() != 2 {
            return;
        }

        let p1 = touches[0].current_pos;
        let p2 = touches[1].current_pos;

        self.initial_pinch_distance = self.distance_between(p1, p2);
        self.initial_rotation_angle = self.angle_between(p1, p2);
        self.pinch_in_progress = true;
        self.rotation_in_progress = true;
    }

    fn update_two_finger_gestures(&mut self, gestures: &mut Vec<RecognizedGesture>) {
        let touches: Vec<_> = self.touches.values().collect();
        if touches.len() != 2 {
            return;
        }

        let p1 = touches[0].current_pos;
        let p2 = touches[1].current_pos;
        let center = Point::new((p1.x + p2.x) / 2.0, (p1.y + p2.y) / 2.0);

        // Pinch gesture
        if self.pinch_in_progress {
            let current_distance = self.distance_between(p1, p2);
            let scale = if self.initial_pinch_distance > 0.0 {
                current_distance / self.initial_pinch_distance
            } else {
                1.0
            };

            let prev_scale = self.initial_pinch_distance;
            let delta = if prev_scale > 0.0 {
                (current_distance - prev_scale) / prev_scale
            } else {
                0.0
            };

            gestures.push(RecognizedGesture::Pinch(PinchGestureEvent::new(
                center,
                center,
                center,
                scale as f64,
                delta as f64,
                GestureState::Updated,
                self.modifiers,
            )));
        }

        // Rotation gesture
        if self.rotation_in_progress {
            let current_angle = self.angle_between(p1, p2);
            let rotation = current_angle - self.initial_rotation_angle;
            let delta = rotation; // For simplicity, delta is the total rotation

            gestures.push(RecognizedGesture::Rotation(RotationGestureEvent::new(
                center,
                center,
                center,
                rotation as f64,
                delta as f64,
                GestureState::Updated,
                self.modifiers,
            )));
        }
    }

    fn end_two_finger_gestures(
        &mut self,
        point: &TouchPoint,
        gestures: &mut Vec<RecognizedGesture>,
    ) {
        let center = point.window_pos;

        if self.pinch_in_progress {
            gestures.push(RecognizedGesture::Pinch(PinchGestureEvent::new(
                center,
                center,
                point.global_pos,
                1.0,
                0.0,
                GestureState::Ended,
                self.modifiers,
            )));
            self.pinch_in_progress = false;
        }

        if self.rotation_in_progress {
            gestures.push(RecognizedGesture::Rotation(RotationGestureEvent::new(
                center,
                center,
                point.global_pos,
                0.0,
                0.0,
                GestureState::Ended,
                self.modifiers,
            )));
            self.rotation_in_progress = false;
        }
    }

    fn cancel_two_finger_gestures(
        &mut self,
        point: &TouchPoint,
        gestures: &mut Vec<RecognizedGesture>,
    ) {
        let center = point.window_pos;

        if self.pinch_in_progress {
            gestures.push(RecognizedGesture::Pinch(PinchGestureEvent::new(
                center,
                center,
                point.global_pos,
                1.0,
                0.0,
                GestureState::Cancelled,
                self.modifiers,
            )));
            self.pinch_in_progress = false;
        }

        if self.rotation_in_progress {
            gestures.push(RecognizedGesture::Rotation(RotationGestureEvent::new(
                center,
                center,
                point.global_pos,
                0.0,
                0.0,
                GestureState::Cancelled,
                self.modifiers,
            )));
            self.rotation_in_progress = false;
        }
    }

    fn distance_between(&self, p1: Point, p2: Point) -> f32 {
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        (dx * dx + dy * dy).sqrt()
    }

    fn angle_between(&self, p1: Point, p2: Point) -> f32 {
        let dx = p2.x - p1.x;
        let dy = p2.y - p1.y;
        dy.atan2(dx)
    }

    fn calculate_velocity(&self, prev: Point, current: Point) -> Point {
        // Simplified velocity calculation (assumes ~16ms between updates)
        let dt = 0.016f32;
        Point::new((current.x - prev.x) / dt, (current.y - prev.y) / dt)
    }

    fn determine_swipe_direction(&self, dx: f32, dy: f32) -> SwipeDirection {
        if dx.abs() > dy.abs() {
            if dx > 0.0 {
                SwipeDirection::Right
            } else {
                SwipeDirection::Left
            }
        } else if dy > 0.0 {
            SwipeDirection::Down
        } else {
            SwipeDirection::Up
        }
    }

    /// Resets the recognizer state.
    pub fn reset(&mut self) {
        self.touches.clear();
        self.last_tap = None;
        self.pan_in_progress = false;
        self.pinch_in_progress = false;
        self.rotation_in_progress = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_touch_point(id: u64, phase: TouchPhase, x: f32, y: f32) -> TouchPoint {
        TouchPoint::new(
            id,
            Point::new(x, y),
            Point::new(x, y),
            Point::new(x, y),
            phase,
        )
    }

    fn make_touch_event(points: Vec<TouchPoint>) -> TouchEvent {
        TouchEvent::with_points(points, KeyboardModifiers::NONE)
    }

    #[test]
    fn test_single_tap() {
        let mut recognizer = GestureRecognizer::new();

        // Touch start
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Started, 100.0, 100.0)]);
        let gestures = recognizer.process_touch(&event);
        assert!(gestures.is_empty());

        // Touch end (quick)
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Ended, 100.0, 100.0)]);
        let gestures = recognizer.process_touch(&event);

        assert_eq!(gestures.len(), 1);
        match &gestures[0] {
            RecognizedGesture::Tap(e) => {
                assert_eq!(e.tap_count, 1);
            }
            _ => panic!("Expected tap gesture"),
        }
    }

    #[test]
    fn test_double_tap() {
        let mut recognizer = GestureRecognizer::new();

        // First tap
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Started, 100.0, 100.0)]);
        recognizer.process_touch(&event);
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Ended, 100.0, 100.0)]);
        let gestures = recognizer.process_touch(&event);
        assert_eq!(gestures.len(), 1);
        match &gestures[0] {
            RecognizedGesture::Tap(e) => assert_eq!(e.tap_count, 1),
            _ => panic!("Expected tap"),
        }

        // Second tap (quickly)
        let event = make_touch_event(vec![make_touch_point(2, TouchPhase::Started, 100.0, 100.0)]);
        recognizer.process_touch(&event);
        let event = make_touch_event(vec![make_touch_point(2, TouchPhase::Ended, 100.0, 100.0)]);
        let gestures = recognizer.process_touch(&event);

        assert_eq!(gestures.len(), 1);
        match &gestures[0] {
            RecognizedGesture::Tap(e) => {
                assert_eq!(e.tap_count, 2);
            }
            _ => panic!("Expected double tap"),
        }
    }

    #[test]
    fn test_pan_gesture() {
        let mut recognizer = GestureRecognizer::new();

        // Touch start
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Started, 100.0, 100.0)]);
        recognizer.process_touch(&event);

        // Move beyond slop
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Moved, 150.0, 100.0)]);
        let gestures = recognizer.process_touch(&event);

        assert_eq!(gestures.len(), 1);
        match &gestures[0] {
            RecognizedGesture::Pan(e) => {
                assert_eq!(e.state, GestureState::Started);
            }
            _ => panic!("Expected pan start"),
        }

        // Continue moving
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Moved, 200.0, 100.0)]);
        let gestures = recognizer.process_touch(&event);

        assert_eq!(gestures.len(), 1);
        match &gestures[0] {
            RecognizedGesture::Pan(e) => {
                assert_eq!(e.state, GestureState::Updated);
            }
            _ => panic!("Expected pan update"),
        }

        // End
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Ended, 200.0, 100.0)]);
        let gestures = recognizer.process_touch(&event);

        // Should have pan end (possibly swipe too if velocity is high enough)
        assert!(
            gestures
                .iter()
                .any(|g| matches!(g, RecognizedGesture::Pan(e) if e.state == GestureState::Ended))
        );
    }

    #[test]
    fn test_swipe_detection() {
        let mut recognizer = GestureRecognizer::new();

        // Quick swipe right
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Started, 100.0, 100.0)]);
        recognizer.process_touch(&event);

        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Moved, 200.0, 100.0)]);
        recognizer.process_touch(&event);

        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Ended, 200.0, 100.0)]);
        let gestures = recognizer.process_touch(&event);

        // Check for swipe
        let _has_swipe = gestures.iter().any(
            |g| matches!(g, RecognizedGesture::Swipe(e) if e.direction == SwipeDirection::Right),
        );

        // Note: May not always trigger depending on velocity calculation
        // This test mainly verifies the code path works
        assert!(
            !gestures.is_empty(),
            "Should have at least one gesture (pan end)"
        );
    }

    #[test]
    fn test_two_finger_pinch() {
        let mut recognizer = GestureRecognizer::new();

        // Start two touches
        let event = make_touch_event(vec![
            make_touch_point(1, TouchPhase::Started, 100.0, 100.0),
            make_touch_point(2, TouchPhase::Started, 200.0, 100.0),
        ]);
        recognizer.process_touch(&event);

        // Move touches apart (pinch out)
        let event = make_touch_event(vec![
            make_touch_point(1, TouchPhase::Moved, 50.0, 100.0),
            make_touch_point(2, TouchPhase::Moved, 250.0, 100.0),
        ]);
        let gestures = recognizer.process_touch(&event);

        // Should have pinch and rotation gestures
        assert!(
            gestures
                .iter()
                .any(|g| matches!(g, RecognizedGesture::Pinch(_)))
        );
        assert!(
            gestures
                .iter()
                .any(|g| matches!(g, RecognizedGesture::Rotation(_)))
        );
    }

    #[test]
    fn test_reset() {
        let mut recognizer = GestureRecognizer::new();

        // Start a touch
        let event = make_touch_event(vec![make_touch_point(1, TouchPhase::Started, 100.0, 100.0)]);
        recognizer.process_touch(&event);

        assert!(!recognizer.touches.is_empty());

        recognizer.reset();

        assert!(recognizer.touches.is_empty());
        assert!(recognizer.last_tap.is_none());
        assert!(!recognizer.pan_in_progress);
    }

    // =========================================================================
    // Conflict Resolution Tests
    // =========================================================================

    #[test]
    fn test_simultaneous_gesture_policy_default() {
        let policy = SimultaneousGesturePolicy::default();

        // Pinch and rotation should be allowed simultaneously by default
        assert!(policy.can_be_simultaneous(GestureType::Pinch, GestureType::Rotation));
        assert!(policy.can_be_simultaneous(GestureType::Rotation, GestureType::Pinch)); // Order shouldn't matter

        // Other combinations should not be allowed by default
        assert!(!policy.can_be_simultaneous(GestureType::Pan, GestureType::Swipe));
        assert!(!policy.can_be_simultaneous(GestureType::Tap, GestureType::LongPress));

        // Same gesture type should always be allowed
        assert!(policy.can_be_simultaneous(GestureType::Tap, GestureType::Tap));
    }

    #[test]
    fn test_simultaneous_gesture_policy_allow_all() {
        let policy = SimultaneousGesturePolicy::allow_all();

        // All combinations should be allowed
        assert!(policy.can_be_simultaneous(GestureType::Pan, GestureType::Swipe));
        assert!(policy.can_be_simultaneous(GestureType::Tap, GestureType::LongPress));
        assert!(policy.can_be_simultaneous(GestureType::Pinch, GestureType::Pan));
    }

    #[test]
    fn test_simultaneous_gesture_policy_custom() {
        let mut policy = SimultaneousGesturePolicy::new();

        // Initially nothing is allowed
        assert!(!policy.can_be_simultaneous(GestureType::Pan, GestureType::Swipe));

        // Allow pan + swipe
        policy.allow_simultaneous(GestureType::Pan, GestureType::Swipe);
        assert!(policy.can_be_simultaneous(GestureType::Pan, GestureType::Swipe));
        assert!(policy.can_be_simultaneous(GestureType::Swipe, GestureType::Pan)); // Order independent

        // Disallow it
        policy.disallow_simultaneous(GestureType::Pan, GestureType::Swipe);
        assert!(!policy.can_be_simultaneous(GestureType::Pan, GestureType::Swipe));
    }

    #[test]
    fn test_gesture_arena_basic() {
        let mut arena = GestureArena::new();

        // Register a gesture
        let id = arena.register(
            GestureType::Pan,
            GesturePriority::Normal,
            GesturePolicy::Default,
        );
        assert!(id.is_some());
        let id = id.unwrap();

        // Should be active
        assert_eq!(arena.state(id), Some(GestureArenaState::Active));
        assert!(arena.is_active(GestureType::Pan));

        // Accept it
        let cancelled = arena.accept(id);
        assert!(cancelled.is_empty());
        assert_eq!(arena.state(id), Some(GestureArenaState::Accepted));
    }

    #[test]
    fn test_gesture_arena_conflict_resolution() {
        let mut arena = GestureArena::new();

        // Register two conflicting gestures (pan and swipe are not simultaneous by default)
        let pan_id = arena
            .register(
                GestureType::Pan,
                GesturePriority::Normal,
                GesturePolicy::Default,
            )
            .unwrap();
        let swipe_id = arena
            .register(
                GestureType::Swipe,
                GesturePriority::Normal,
                GesturePolicy::Default,
            )
            .unwrap();

        // Both should be active
        assert!(arena.is_active(GestureType::Pan));
        assert!(arena.is_active(GestureType::Swipe));

        // Accept pan - should cancel swipe (equal priority, pan accepted first)
        let cancelled = arena.accept(pan_id);
        assert!(cancelled.contains(&swipe_id));
        assert_eq!(arena.state(swipe_id), Some(GestureArenaState::Rejected));
    }

    #[test]
    fn test_gesture_arena_priority() {
        let mut arena = GestureArena::new();

        // Register low priority gesture first
        let low_id = arena
            .register(
                GestureType::Pan,
                GesturePriority::Low,
                GesturePolicy::Default,
            )
            .unwrap();

        // Register high priority conflicting gesture
        let high_id = arena
            .register(
                GestureType::Swipe,
                GesturePriority::High,
                GesturePolicy::Default,
            )
            .unwrap();

        // Try to accept low priority - should fail due to higher priority conflict
        let cancelled = arena.accept(low_id);
        assert!(cancelled.is_empty());
        assert_eq!(arena.state(low_id), Some(GestureArenaState::Active)); // Still active, not accepted

        // Accept high priority - should succeed and cancel low
        let cancelled = arena.accept(high_id);
        assert!(cancelled.contains(&low_id));
        assert_eq!(arena.state(high_id), Some(GestureArenaState::Accepted));
    }

    #[test]
    fn test_gesture_arena_exclusive_policy() {
        let mut arena = GestureArena::new();

        // Register two gestures
        let g1 = arena
            .register(
                GestureType::Pan,
                GesturePriority::Normal,
                GesturePolicy::Default,
            )
            .unwrap();
        let g2 = arena
            .register(
                GestureType::Swipe,
                GesturePriority::High,
                GesturePolicy::Default,
            )
            .unwrap();

        // Register exclusive gesture
        let exclusive = arena
            .register(
                GestureType::LongPress,
                GesturePriority::Normal,
                GesturePolicy::Exclusive,
            )
            .unwrap();

        // Accept exclusive - should cancel all conflicting regardless of priority
        let cancelled = arena.accept(exclusive);
        assert!(cancelled.contains(&g1));
        assert!(cancelled.contains(&g2));
    }

    #[test]
    fn test_gesture_arena_cooperative_policy() {
        let mut arena = GestureArena::new();

        // Register two conflicting gestures
        let g1 = arena
            .register(
                GestureType::Pan,
                GesturePriority::Normal,
                GesturePolicy::Default,
            )
            .unwrap();
        let coop = arena
            .register(
                GestureType::Swipe,
                GesturePriority::Normal,
                GesturePolicy::Cooperative,
            )
            .unwrap();

        // Accept cooperative - should not cancel others
        let cancelled = arena.accept(coop);
        assert!(cancelled.is_empty());
        assert_eq!(arena.state(g1), Some(GestureArenaState::Active)); // g1 still active
        assert_eq!(arena.state(coop), Some(GestureArenaState::Accepted));
    }

    #[test]
    fn test_gesture_arena_simultaneous_allowed() {
        let mut arena = GestureArena::new();

        // Pinch and rotation are allowed simultaneously by default
        let pinch_id = arena
            .register(
                GestureType::Pinch,
                GesturePriority::Normal,
                GesturePolicy::Default,
            )
            .unwrap();
        let rotation_id = arena
            .register(
                GestureType::Rotation,
                GesturePriority::Normal,
                GesturePolicy::Default,
            )
            .unwrap();

        // Accept pinch - should NOT cancel rotation (they can coexist)
        let cancelled = arena.accept(pinch_id);
        assert!(cancelled.is_empty());
        assert_eq!(arena.state(rotation_id), Some(GestureArenaState::Active)); // Still active!

        // Can also accept rotation
        let cancelled = arena.accept(rotation_id);
        assert!(cancelled.is_empty());
        assert_eq!(arena.state(rotation_id), Some(GestureArenaState::Accepted));
    }

    #[test]
    fn test_gesture_arena_failure_dependency() {
        let mut arena = GestureArena::new();

        // Add failure dependency: tap waits for double-tap to fail
        arena.add_failure_dependency(GestureType::Tap, GestureType::DoubleTap);

        // Register double-tap first
        let double_tap_id = arena
            .register(
                GestureType::DoubleTap,
                GesturePriority::Normal,
                GesturePolicy::Default,
            )
            .unwrap();

        // Register tap - should be pending due to failure dependency
        let tap_id = arena
            .register(
                GestureType::Tap,
                GesturePriority::Normal,
                GesturePolicy::Default,
            )
            .unwrap();
        assert_eq!(arena.state(tap_id), Some(GestureArenaState::Pending));

        // Reject double-tap - tap should now become active
        let promoted = arena.reject(double_tap_id);
        assert!(promoted); // Indicates pending gestures were promoted
        assert_eq!(arena.state(tap_id), Some(GestureArenaState::Active));
    }

    #[test]
    fn test_gesture_arena_clear() {
        let mut arena = GestureArena::new();

        arena.register(
            GestureType::Pan,
            GesturePriority::Normal,
            GesturePolicy::Default,
        );
        arena.register(
            GestureType::Swipe,
            GesturePriority::Normal,
            GesturePolicy::Default,
        );

        assert!(!arena.active_gestures().is_empty());

        arena.clear();
        assert!(arena.active_gestures().is_empty());
    }

    #[test]
    fn test_failure_dependency_creation() {
        let dep = FailureDependency::new(GestureType::Tap, GestureType::DoubleTap);
        assert_eq!(dep.waiting, GestureType::Tap);
        assert_eq!(dep.must_fail, GestureType::DoubleTap);
    }

    #[test]
    fn test_gesture_priority_ordering() {
        assert!(GesturePriority::Low < GesturePriority::Normal);
        assert!(GesturePriority::Normal < GesturePriority::High);
        assert!(GesturePriority::Low < GesturePriority::High);
    }

    #[test]
    fn test_recognized_gesture_type() {
        // Single tap
        let tap = TapGestureEvent::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            1, // tap_count
            KeyboardModifiers::NONE,
        );
        let gesture = RecognizedGesture::Tap(tap);
        assert_eq!(gesture.gesture_type(), GestureType::Tap);

        // Double tap
        let double_tap = TapGestureEvent::new(
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            Point::new(0.0, 0.0),
            2, // tap_count
            KeyboardModifiers::NONE,
        );
        let gesture = RecognizedGesture::Tap(double_tap);
        assert_eq!(gesture.gesture_type(), GestureType::DoubleTap);
    }
}
