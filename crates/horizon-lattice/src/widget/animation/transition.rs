//! Transition types and state management.
//!
//! Transitions define how animated changes occur between states.

use std::time::{Duration, Instant};

use super::easing::{Easing, ease};

/// Type of transition effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransitionType {
    /// No transition, instant switch.
    #[default]
    None,
    /// Fade between states using opacity.
    Fade,
    /// Slide horizontally (left to right for forward, right to left for backward).
    SlideHorizontal,
    /// Slide vertically (top to bottom for forward, bottom to top for backward).
    SlideVertical,
}

/// Current state of a transition.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TransitionState {
    /// No transition in progress.
    #[default]
    Idle,
    /// Transition is running.
    Running {
        /// Progress from 0.0 to 1.0.
        progress: f32,
        /// Index transitioning from.
        from_index: usize,
        /// Index transitioning to.
        to_index: usize,
    },
}

impl TransitionState {
    /// Check if a transition is currently in progress.
    pub fn is_running(&self) -> bool {
        matches!(self, TransitionState::Running { .. })
    }

    /// Get the current progress if running.
    pub fn progress(&self) -> Option<f32> {
        match self {
            TransitionState::Running { progress, .. } => Some(*progress),
            TransitionState::Idle => None,
        }
    }
}

/// A transition animation controller.
///
/// Manages the timing and progress of a transition between two states.
#[derive(Debug, Clone)]
pub struct Transition {
    /// The type of transition effect.
    transition_type: TransitionType,
    /// Easing function for the transition.
    easing: Easing,
    /// Duration of the transition.
    duration: Duration,
    /// When the transition started (if running).
    start_time: Option<Instant>,
    /// Index transitioning from.
    from_index: usize,
    /// Index transitioning to.
    to_index: usize,
    /// Whether the transition is currently running.
    running: bool,
}

impl Transition {
    /// Create a new transition with default settings.
    pub fn new() -> Self {
        Self {
            transition_type: TransitionType::None,
            easing: Easing::EaseInOut,
            duration: Duration::from_millis(250),
            start_time: None,
            from_index: 0,
            to_index: 0,
            running: false,
        }
    }

    /// Create a transition with a specific type.
    pub fn with_type(transition_type: TransitionType) -> Self {
        Self {
            transition_type,
            ..Self::new()
        }
    }

    /// Get the transition type.
    #[inline]
    pub fn transition_type(&self) -> TransitionType {
        self.transition_type
    }

    /// Set the transition type.
    pub fn set_transition_type(&mut self, transition_type: TransitionType) {
        self.transition_type = transition_type;
    }

    /// Get the easing function.
    #[inline]
    pub fn easing(&self) -> Easing {
        self.easing
    }

    /// Set the easing function.
    pub fn set_easing(&mut self, easing: Easing) {
        self.easing = easing;
    }

    /// Get the transition duration.
    #[inline]
    pub fn duration(&self) -> Duration {
        self.duration
    }

    /// Set the transition duration.
    pub fn set_duration(&mut self, duration: Duration) {
        self.duration = duration;
    }

    /// Check if a transition is currently running.
    #[inline]
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Start a transition from one index to another.
    ///
    /// Returns `true` if the transition was started, `false` if no transition
    /// is needed (e.g., same index or transition type is None).
    pub fn start(&mut self, from_index: usize, to_index: usize) -> bool {
        if from_index == to_index || self.transition_type == TransitionType::None {
            return false;
        }

        self.from_index = from_index;
        self.to_index = to_index;
        self.start_time = Some(Instant::now());
        self.running = true;
        true
    }

    /// Stop the current transition immediately.
    pub fn stop(&mut self) {
        self.running = false;
        self.start_time = None;
    }

    /// Update the transition and get its current state.
    ///
    /// Should be called each frame while the transition is running.
    ///
    /// # Returns
    ///
    /// The current transition state, or `Idle` if not running.
    pub fn update(&mut self) -> TransitionState {
        if !self.running {
            return TransitionState::Idle;
        }

        let Some(start_time) = self.start_time else {
            return TransitionState::Idle;
        };

        let elapsed = start_time.elapsed();
        let raw_progress = if self.duration.is_zero() {
            1.0
        } else {
            (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
        };

        // Apply easing
        let progress = ease(self.easing, raw_progress);

        if raw_progress >= 1.0 {
            // Transition complete
            self.running = false;
            self.start_time = None;
            return TransitionState::Idle;
        }

        TransitionState::Running {
            progress,
            from_index: self.from_index,
            to_index: self.to_index,
        }
    }

    /// Get the indices involved in the current transition.
    ///
    /// Returns `None` if no transition is running.
    pub fn indices(&self) -> Option<(usize, usize)> {
        if self.running {
            Some((self.from_index, self.to_index))
        } else {
            None
        }
    }

    /// Check if a specific index is visible during the transition.
    ///
    /// During transitions, both the from and to indices may be visible.
    pub fn is_index_visible(&self, index: usize, current_index: usize) -> bool {
        if self.running {
            index == self.from_index || index == self.to_index
        } else {
            index == current_index
        }
    }

    /// Calculate the opacity for a given index during a fade transition.
    ///
    /// # Arguments
    ///
    /// * `index` - The index to calculate opacity for
    /// * `progress` - The transition progress (0.0 to 1.0)
    ///
    /// # Returns
    ///
    /// Opacity value from 0.0 to 1.0.
    pub fn fade_opacity(&self, index: usize, progress: f32) -> f32 {
        if !self.running {
            return 1.0;
        }

        if index == self.from_index {
            1.0 - progress
        } else if index == self.to_index {
            progress
        } else {
            0.0
        }
    }

    /// Calculate the x-offset for a given index during a horizontal slide transition.
    ///
    /// # Arguments
    ///
    /// * `index` - The index to calculate offset for
    /// * `progress` - The transition progress (0.0 to 1.0)
    /// * `width` - The width of the layout
    ///
    /// # Returns
    ///
    /// X offset in pixels.
    pub fn slide_horizontal_offset(&self, index: usize, progress: f32, width: f32) -> f32 {
        if !self.running {
            return 0.0;
        }

        // Determine direction: forward (to > from) slides left, backward slides right
        let forward = self.to_index > self.from_index;

        if index == self.from_index {
            // Outgoing: slide out in the opposite direction of travel
            if forward {
                -width * progress // Slide left (out of view)
            } else {
                width * progress // Slide right (out of view)
            }
        } else if index == self.to_index {
            // Incoming: slide in from the direction of travel
            if forward {
                width * (1.0 - progress) // Start from right, slide to center
            } else {
                -width * (1.0 - progress) // Start from left, slide to center
            }
        } else {
            0.0
        }
    }

    /// Calculate the y-offset for a given index during a vertical slide transition.
    ///
    /// # Arguments
    ///
    /// * `index` - The index to calculate offset for
    /// * `progress` - The transition progress (0.0 to 1.0)
    /// * `height` - The height of the layout
    ///
    /// # Returns
    ///
    /// Y offset in pixels.
    pub fn slide_vertical_offset(&self, index: usize, progress: f32, height: f32) -> f32 {
        if !self.running {
            return 0.0;
        }

        // Determine direction: forward (to > from) slides up, backward slides down
        let forward = self.to_index > self.from_index;

        if index == self.from_index {
            if forward {
                -height * progress // Slide up (out of view)
            } else {
                height * progress // Slide down (out of view)
            }
        } else if index == self.to_index {
            if forward {
                height * (1.0 - progress) // Start from bottom, slide to center
            } else {
                -height * (1.0 - progress) // Start from top, slide to center
            }
        } else {
            0.0
        }
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_creation() {
        let t = Transition::new();
        assert_eq!(t.transition_type(), TransitionType::None);
        assert!(!t.is_running());
    }

    #[test]
    fn test_transition_start_same_index() {
        let mut t = Transition::with_type(TransitionType::Fade);
        // Same index should not start a transition
        assert!(!t.start(0, 0));
        assert!(!t.is_running());
    }

    #[test]
    fn test_transition_start_different_index() {
        let mut t = Transition::with_type(TransitionType::Fade);
        assert!(t.start(0, 1));
        assert!(t.is_running());
        assert_eq!(t.indices(), Some((0, 1)));
    }

    #[test]
    fn test_transition_none_type() {
        let mut t = Transition::new();
        // TransitionType::None should not start
        assert!(!t.start(0, 1));
    }

    #[test]
    fn test_transition_state() {
        let state = TransitionState::Running {
            progress: 0.5,
            from_index: 0,
            to_index: 1,
        };
        assert!(state.is_running());
        assert_eq!(state.progress(), Some(0.5));

        let idle = TransitionState::Idle;
        assert!(!idle.is_running());
        assert_eq!(idle.progress(), None);
    }

    #[test]
    fn test_fade_opacity() {
        let mut t = Transition::with_type(TransitionType::Fade);
        t.start(0, 1);

        // At progress 0.0: from is fully visible, to is invisible
        assert_eq!(t.fade_opacity(0, 0.0), 1.0);
        assert_eq!(t.fade_opacity(1, 0.0), 0.0);

        // At progress 0.5: both are half visible
        assert_eq!(t.fade_opacity(0, 0.5), 0.5);
        assert_eq!(t.fade_opacity(1, 0.5), 0.5);

        // At progress 1.0: from is invisible, to is fully visible
        assert_eq!(t.fade_opacity(0, 1.0), 0.0);
        assert_eq!(t.fade_opacity(1, 1.0), 1.0);

        // Other indices are always invisible
        assert_eq!(t.fade_opacity(2, 0.5), 0.0);
    }

    #[test]
    fn test_slide_horizontal_forward() {
        let mut t = Transition::with_type(TransitionType::SlideHorizontal);
        t.start(0, 1); // Forward: 0 -> 1

        let width = 100.0;

        // At start: from at center, to off to the right
        assert_eq!(t.slide_horizontal_offset(0, 0.0, width), 0.0);
        assert_eq!(t.slide_horizontal_offset(1, 0.0, width), 100.0);

        // At end: from off to the left, to at center
        assert_eq!(t.slide_horizontal_offset(0, 1.0, width), -100.0);
        assert_eq!(t.slide_horizontal_offset(1, 1.0, width), 0.0);
    }

    #[test]
    fn test_slide_horizontal_backward() {
        let mut t = Transition::with_type(TransitionType::SlideHorizontal);
        t.start(1, 0); // Backward: 1 -> 0

        let width = 100.0;

        // At start: from at center, to off to the left
        assert_eq!(t.slide_horizontal_offset(1, 0.0, width), 0.0);
        assert_eq!(t.slide_horizontal_offset(0, 0.0, width), -100.0);

        // At end: from off to the right, to at center
        assert_eq!(t.slide_horizontal_offset(1, 1.0, width), 100.0);
        assert_eq!(t.slide_horizontal_offset(0, 1.0, width), 0.0);
    }

    #[test]
    fn test_is_index_visible() {
        let mut t = Transition::with_type(TransitionType::Fade);
        t.start(0, 2);

        // During transition, indices 0 and 2 are visible
        assert!(t.is_index_visible(0, 0));
        assert!(!t.is_index_visible(1, 0)); // Index 1 not part of transition
        assert!(t.is_index_visible(2, 0));

        // When not running, only current index is visible
        t.stop();
        assert!(t.is_index_visible(1, 1));
        assert!(!t.is_index_visible(0, 1));
    }
}
