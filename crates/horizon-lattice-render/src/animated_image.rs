//! Animated image support for GIF and other animated formats.
//!
//! This module provides support for loading and playing animated images like GIFs.
//! It handles frame extraction, timing, and playback control.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_render::{AnimatedImage, AnimationController};
//! use std::time::Duration;
//!
//! // Load an animated GIF
//! let animated = AnimatedImage::from_file("animation.gif")?;
//!
//! // Create a playback controller
//! let mut controller = AnimationController::new(&animated);
//!
//! // In your render loop:
//! controller.update(delta_time);
//! let current_frame = controller.current_frame();
//! let rgba_data = animated.frame_data(current_frame);
//! ```

use std::io::{BufRead, BufReader, Cursor, Seek};
use std::path::Path;
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::{AnimationDecoder, ImageDecoder};

use crate::error::{RenderError, RenderResult};

/// A single frame in an animation.
#[derive(Clone)]
pub struct AnimationFrame {
    /// RGBA pixel data for this frame.
    rgba_data: Vec<u8>,
    /// Width of the frame in pixels.
    width: u32,
    /// Height of the frame in pixels.
    height: u32,
    /// Delay before showing the next frame.
    delay: Duration,
    /// X offset of this frame (for GIF disposal).
    left: u32,
    /// Y offset of this frame (for GIF disposal).
    top: u32,
}

impl AnimationFrame {
    /// Get the RGBA pixel data for this frame.
    #[inline]
    pub fn rgba_data(&self) -> &[u8] {
        &self.rgba_data
    }

    /// Get the width of this frame in pixels.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the height of this frame in pixels.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the delay before showing the next frame.
    #[inline]
    pub fn delay(&self) -> Duration {
        self.delay
    }

    /// Get the X offset of this frame.
    #[inline]
    pub fn left(&self) -> u32 {
        self.left
    }

    /// Get the Y offset of this frame.
    #[inline]
    pub fn top(&self) -> u32 {
        self.top
    }
}

impl std::fmt::Debug for AnimationFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationFrame")
            .field("dimensions", &format!("{}x{}", self.width, self.height))
            .field("offset", &format!("({}, {})", self.left, self.top))
            .field("delay", &self.delay)
            .field("data_size", &self.rgba_data.len())
            .finish()
    }
}

/// Loop behavior for animations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum LoopCount {
    /// Loop indefinitely.
    #[default]
    Infinite,
    /// Loop a specific number of times.
    Finite(u32),
}


/// An animated image containing multiple frames.
///
/// This struct holds all the frames of an animated image (like a GIF)
/// along with metadata about the animation.
#[derive(Clone)]
pub struct AnimatedImage {
    /// All frames in the animation.
    frames: Vec<AnimationFrame>,
    /// Overall width of the animation canvas.
    width: u32,
    /// Overall height of the animation canvas.
    height: u32,
    /// Number of times to loop the animation.
    loop_count: LoopCount,
    /// Total duration of one loop of the animation.
    total_duration: Duration,
}

impl AnimatedImage {
    /// Load an animated image from a file path.
    ///
    /// Currently supports GIF format. The file is read and all frames
    /// are decoded and stored in memory.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the animated image file
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The format is not supported
    /// - The image is corrupted
    ///
    /// # Example
    ///
    /// ```ignore
    /// let animated = AnimatedImage::from_file("loading.gif")?;
    /// println!("Loaded {} frames", animated.frame_count());
    /// ```
    pub fn from_file(path: impl AsRef<Path>) -> RenderResult<Self> {
        let file = std::fs::File::open(path.as_ref())
            .map_err(|e| RenderError::ImageLoad(format!("Failed to open file: {}", e)))?;
        let reader = BufReader::new(file);
        Self::from_reader(reader)
    }

    /// Load an animated image from bytes in memory.
    ///
    /// # Arguments
    ///
    /// * `data` - Raw file bytes (e.g., GIF file contents)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let gif_bytes = include_bytes!("../assets/spinner.gif");
    /// let animated = AnimatedImage::from_bytes(gif_bytes)?;
    /// ```
    pub fn from_bytes(data: &[u8]) -> RenderResult<Self> {
        let cursor = Cursor::new(data.to_vec());
        Self::from_reader(cursor)
    }

    /// Load an animated image from a reader.
    fn from_reader<R: BufRead + Seek + 'static>(reader: R) -> RenderResult<Self> {
        // Try to decode as GIF
        let decoder = GifDecoder::new(reader).map_err(|e| {
            RenderError::ImageLoad(format!("Failed to decode animated image: {}", e))
        })?;

        let (width, height) = decoder.dimensions();

        // Get the frames
        let image_frames = decoder.into_frames();

        let mut frames = Vec::new();
        let mut total_duration = Duration::ZERO;

        for frame_result in image_frames {
            let frame = frame_result
                .map_err(|e| RenderError::ImageLoad(format!("Failed to decode frame: {}", e)))?;

            let delay = frame.delay();
            let (num, denom) = delay.numer_denom_ms();
            let delay_ms = if denom > 0 { num / denom } else { 100 };
            let delay_duration = Duration::from_millis(delay_ms as u64);

            // Get frame position and buffer
            let left = frame.left();
            let top = frame.top();
            let buffer = frame.into_buffer();
            let (frame_width, frame_height) = buffer.dimensions();

            frames.push(AnimationFrame {
                rgba_data: buffer.into_raw(),
                width: frame_width,
                height: frame_height,
                delay: delay_duration,
                left,
                top,
            });

            total_duration += delay_duration;
        }

        if frames.is_empty() {
            return Err(RenderError::ImageLoad(
                "Animated image contains no frames".to_string(),
            ));
        }

        // Default to infinite loop for GIFs
        let loop_count = LoopCount::Infinite;

        Ok(Self {
            frames,
            width,
            height,
            loop_count,
            total_duration,
        })
    }

    /// Get the number of frames in the animation.
    #[inline]
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// Get a specific frame by index.
    ///
    /// # Panics
    ///
    /// Panics if `index >= frame_count()`.
    #[inline]
    pub fn frame(&self, index: usize) -> &AnimationFrame {
        &self.frames[index]
    }

    /// Get a specific frame by index, returning None if out of bounds.
    #[inline]
    pub fn get_frame(&self, index: usize) -> Option<&AnimationFrame> {
        self.frames.get(index)
    }

    /// Get all frames as a slice.
    #[inline]
    pub fn frames(&self) -> &[AnimationFrame] {
        &self.frames
    }

    /// Get the RGBA pixel data for a specific frame.
    #[inline]
    pub fn frame_data(&self, index: usize) -> &[u8] {
        &self.frames[index].rgba_data
    }

    /// Get the delay for a specific frame.
    #[inline]
    pub fn frame_delay(&self, index: usize) -> Duration {
        self.frames[index].delay
    }

    /// Get the width of the animation canvas.
    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Get the height of the animation canvas.
    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Get the loop count for the animation.
    #[inline]
    pub fn loop_count(&self) -> LoopCount {
        self.loop_count
    }

    /// Get the total duration of one loop of the animation.
    #[inline]
    pub fn total_duration(&self) -> Duration {
        self.total_duration
    }

    /// Check if this is a single-frame "animation" (static image).
    #[inline]
    pub fn is_static(&self) -> bool {
        self.frames.len() == 1
    }

    /// Iterate over all frames with their indices.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &AnimationFrame)> {
        self.frames.iter().enumerate()
    }
}

impl std::fmt::Debug for AnimatedImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimatedImage")
            .field("dimensions", &format!("{}x{}", self.width, self.height))
            .field("frames", &self.frames.len())
            .field("loop_count", &self.loop_count)
            .field("total_duration", &self.total_duration)
            .finish()
    }
}

/// Playback state for an animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    /// Animation is playing.
    Playing,
    /// Animation is paused.
    Paused,
    /// Animation has stopped (reached the end and not looping).
    Stopped,
}

/// Controls playback of an animated image.
///
/// The controller tracks the current frame, elapsed time, and playback state.
/// Call `update()` each frame with the delta time to advance the animation.
///
/// # Example
///
/// ```ignore
/// let animated = AnimatedImage::from_file("spinner.gif")?;
/// let mut controller = AnimationController::new(&animated);
///
/// // In your render loop:
/// loop {
///     controller.update(delta_time);
///
///     let frame_idx = controller.current_frame();
///     let frame = animated.frame(frame_idx);
///     // Render the frame...
///
///     if controller.state() == PlaybackState::Stopped {
///         break;
///     }
/// }
/// ```
pub struct AnimationController {
    /// Current frame index.
    current_frame: usize,
    /// Time elapsed in the current frame.
    frame_elapsed: Duration,
    /// Current playback state.
    state: PlaybackState,
    /// Number of completed loops.
    loops_completed: u32,
    /// Total number of frames (cached).
    frame_count: usize,
    /// Frame delays (cached from AnimatedImage).
    frame_delays: Vec<Duration>,
    /// Loop behavior.
    loop_count: LoopCount,
    /// Playback speed multiplier (1.0 = normal speed).
    speed: f64,
}

impl AnimationController {
    /// Create a new animation controller for the given animated image.
    ///
    /// The controller starts in the `Playing` state at frame 0.
    pub fn new(animated: &AnimatedImage) -> Self {
        Self {
            current_frame: 0,
            frame_elapsed: Duration::ZERO,
            state: PlaybackState::Playing,
            loops_completed: 0,
            frame_count: animated.frame_count(),
            frame_delays: animated.frames.iter().map(|f| f.delay).collect(),
            loop_count: animated.loop_count(),
            speed: 1.0,
        }
    }

    /// Create a new animation controller that starts paused.
    pub fn new_paused(animated: &AnimatedImage) -> Self {
        let mut controller = Self::new(animated);
        controller.state = PlaybackState::Paused;
        controller
    }

    /// Update the animation with elapsed time.
    ///
    /// Call this method each frame with the time elapsed since the last update.
    /// The controller will automatically advance frames based on their delays.
    ///
    /// # Returns
    ///
    /// `true` if the frame changed, `false` otherwise.
    pub fn update(&mut self, delta: Duration) -> bool {
        if self.state != PlaybackState::Playing {
            return false;
        }

        if self.frame_count == 0 {
            return false;
        }

        // Apply speed multiplier
        let adjusted_delta = Duration::from_secs_f64(delta.as_secs_f64() * self.speed);
        self.frame_elapsed += adjusted_delta;

        let mut frame_changed = false;

        // Advance frames as needed
        loop {
            let current_delay = self.frame_delays[self.current_frame];

            if self.frame_elapsed >= current_delay {
                self.frame_elapsed -= current_delay;
                frame_changed = true;

                // Move to next frame
                let next_frame = self.current_frame + 1;

                if next_frame >= self.frame_count {
                    // End of animation
                    self.loops_completed += 1;

                    match self.loop_count {
                        LoopCount::Infinite => {
                            // Loop back to start
                            self.current_frame = 0;
                        }
                        LoopCount::Finite(count) => {
                            if self.loops_completed >= count {
                                // Animation finished
                                self.current_frame = self.frame_count - 1;
                                self.state = PlaybackState::Stopped;
                                self.frame_elapsed = Duration::ZERO;
                                break;
                            } else {
                                // Loop back to start
                                self.current_frame = 0;
                            }
                        }
                    }
                } else {
                    self.current_frame = next_frame;
                }
            } else {
                break;
            }
        }

        frame_changed
    }

    /// Get the current frame index.
    #[inline]
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Get the current playback state.
    #[inline]
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    /// Check if the animation is playing.
    #[inline]
    pub fn is_playing(&self) -> bool {
        self.state == PlaybackState::Playing
    }

    /// Check if the animation is paused.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.state == PlaybackState::Paused
    }

    /// Check if the animation has stopped (finished).
    #[inline]
    pub fn is_stopped(&self) -> bool {
        self.state == PlaybackState::Stopped
    }

    /// Get the number of completed loops.
    #[inline]
    pub fn loops_completed(&self) -> u32 {
        self.loops_completed
    }

    /// Get the playback speed multiplier.
    #[inline]
    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// Set the playback speed multiplier.
    ///
    /// Values greater than 1.0 speed up the animation, values less than 1.0
    /// slow it down. Must be positive.
    pub fn set_speed(&mut self, speed: f64) {
        self.speed = speed.max(0.01);
    }

    /// Start or resume playback.
    pub fn play(&mut self) {
        if self.state == PlaybackState::Stopped {
            // Restart from beginning
            self.reset();
        }
        self.state = PlaybackState::Playing;
    }

    /// Pause playback.
    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.state = PlaybackState::Paused;
        }
    }

    /// Toggle between playing and paused states.
    pub fn toggle(&mut self) {
        match self.state {
            PlaybackState::Playing => self.pause(),
            PlaybackState::Paused => self.play(),
            PlaybackState::Stopped => self.play(),
        }
    }

    /// Stop the animation (sets state to Stopped).
    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
    }

    /// Reset the animation to the beginning.
    pub fn reset(&mut self) {
        self.current_frame = 0;
        self.frame_elapsed = Duration::ZERO;
        self.loops_completed = 0;
        if self.state == PlaybackState::Stopped {
            self.state = PlaybackState::Paused;
        }
    }

    /// Jump to a specific frame.
    ///
    /// If the frame index is out of bounds, it will be clamped to valid range.
    pub fn goto_frame(&mut self, frame: usize) {
        self.current_frame = frame.min(self.frame_count.saturating_sub(1));
        self.frame_elapsed = Duration::ZERO;
    }

    /// Advance to the next frame manually.
    ///
    /// Useful for step-by-step playback while paused.
    pub fn next_frame(&mut self) {
        if self.frame_count == 0 {
            return;
        }

        let next = self.current_frame + 1;
        if next >= self.frame_count {
            self.current_frame = 0;
        } else {
            self.current_frame = next;
        }
        self.frame_elapsed = Duration::ZERO;
    }

    /// Go back to the previous frame.
    ///
    /// Useful for step-by-step playback while paused.
    pub fn prev_frame(&mut self) {
        if self.frame_count == 0 {
            return;
        }

        if self.current_frame == 0 {
            self.current_frame = self.frame_count - 1;
        } else {
            self.current_frame -= 1;
        }
        self.frame_elapsed = Duration::ZERO;
    }

    /// Get the time remaining until the next frame change.
    pub fn time_until_next_frame(&self) -> Duration {
        if self.frame_count == 0 || self.state != PlaybackState::Playing {
            return Duration::ZERO;
        }

        let current_delay = self.frame_delays[self.current_frame];
        current_delay.saturating_sub(self.frame_elapsed)
    }

    /// Get progress within the current frame (0.0 to 1.0).
    pub fn frame_progress(&self) -> f64 {
        if self.frame_count == 0 {
            return 0.0;
        }

        let current_delay = self.frame_delays[self.current_frame];
        if current_delay.is_zero() {
            return 1.0;
        }

        self.frame_elapsed.as_secs_f64() / current_delay.as_secs_f64()
    }
}

impl std::fmt::Debug for AnimationController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationController")
            .field("current_frame", &self.current_frame)
            .field("state", &self.state)
            .field("speed", &self.speed)
            .field("loops_completed", &self.loops_completed)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_count_default() {
        assert_eq!(LoopCount::default(), LoopCount::Infinite);
    }

    #[test]
    fn test_animation_controller_play_pause() {
        // Create a minimal animated image struct for testing
        let frames = vec![
            AnimationFrame {
                rgba_data: vec![255, 0, 0, 255],
                width: 1,
                height: 1,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
            AnimationFrame {
                rgba_data: vec![0, 255, 0, 255],
                width: 1,
                height: 1,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
        ];

        let animated = AnimatedImage {
            frames,
            width: 1,
            height: 1,
            loop_count: LoopCount::Infinite,
            total_duration: Duration::from_millis(200),
        };

        let mut controller = AnimationController::new(&animated);

        assert_eq!(controller.state(), PlaybackState::Playing);
        assert!(controller.is_playing());

        controller.pause();
        assert_eq!(controller.state(), PlaybackState::Paused);
        assert!(controller.is_paused());

        controller.play();
        assert!(controller.is_playing());
    }

    #[test]
    fn test_animation_controller_update() {
        let frames = vec![
            AnimationFrame {
                rgba_data: vec![255, 0, 0, 255],
                width: 1,
                height: 1,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
            AnimationFrame {
                rgba_data: vec![0, 255, 0, 255],
                width: 1,
                height: 1,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
        ];

        let animated = AnimatedImage {
            frames,
            width: 1,
            height: 1,
            loop_count: LoopCount::Infinite,
            total_duration: Duration::from_millis(200),
        };

        let mut controller = AnimationController::new(&animated);

        assert_eq!(controller.current_frame(), 0);

        // Update with less than frame delay - should stay on frame 0
        let changed = controller.update(Duration::from_millis(50));
        assert!(!changed);
        assert_eq!(controller.current_frame(), 0);

        // Update past frame delay - should advance to frame 1
        let changed = controller.update(Duration::from_millis(60));
        assert!(changed);
        assert_eq!(controller.current_frame(), 1);

        // Update past second frame - should loop back to 0
        let changed = controller.update(Duration::from_millis(100));
        assert!(changed);
        assert_eq!(controller.current_frame(), 0);
        assert_eq!(controller.loops_completed(), 1);
    }

    #[test]
    fn test_animation_controller_finite_loop() {
        let frames = vec![AnimationFrame {
            rgba_data: vec![255, 0, 0, 255],
            width: 1,
            height: 1,
            delay: Duration::from_millis(50),
            left: 0,
            top: 0,
        }];

        let animated = AnimatedImage {
            frames,
            width: 1,
            height: 1,
            loop_count: LoopCount::Finite(2),
            total_duration: Duration::from_millis(50),
        };

        let mut controller = AnimationController::new(&animated);

        // First loop
        controller.update(Duration::from_millis(60));
        assert_eq!(controller.loops_completed(), 1);
        assert!(controller.is_playing());

        // Second loop - should stop
        controller.update(Duration::from_millis(60));
        assert_eq!(controller.loops_completed(), 2);
        assert!(controller.is_stopped());
    }

    #[test]
    fn test_animation_controller_speed() {
        let frames = vec![
            AnimationFrame {
                rgba_data: vec![255, 0, 0, 255],
                width: 1,
                height: 1,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
            AnimationFrame {
                rgba_data: vec![0, 255, 0, 255],
                width: 1,
                height: 1,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
        ];

        let animated = AnimatedImage {
            frames,
            width: 1,
            height: 1,
            loop_count: LoopCount::Infinite,
            total_duration: Duration::from_millis(200),
        };

        let mut controller = AnimationController::new(&animated);
        controller.set_speed(2.0); // 2x speed

        // At 2x speed, 50ms real time = 100ms animation time
        let changed = controller.update(Duration::from_millis(50));
        assert!(changed);
        assert_eq!(controller.current_frame(), 1);
    }

    #[test]
    fn test_animation_controller_navigation() {
        let frames = vec![
            AnimationFrame {
                rgba_data: vec![255, 0, 0, 255],
                width: 1,
                height: 1,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
            AnimationFrame {
                rgba_data: vec![0, 255, 0, 255],
                width: 1,
                height: 1,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
            AnimationFrame {
                rgba_data: vec![0, 0, 255, 255],
                width: 1,
                height: 1,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
        ];

        let animated = AnimatedImage {
            frames,
            width: 1,
            height: 1,
            loop_count: LoopCount::Infinite,
            total_duration: Duration::from_millis(300),
        };

        let mut controller = AnimationController::new(&animated);

        assert_eq!(controller.current_frame(), 0);

        controller.next_frame();
        assert_eq!(controller.current_frame(), 1);

        controller.next_frame();
        assert_eq!(controller.current_frame(), 2);

        controller.next_frame(); // Wraps around
        assert_eq!(controller.current_frame(), 0);

        controller.prev_frame(); // Wraps to end
        assert_eq!(controller.current_frame(), 2);

        controller.goto_frame(1);
        assert_eq!(controller.current_frame(), 1);

        controller.reset();
        assert_eq!(controller.current_frame(), 0);
    }

    #[test]
    fn test_animated_image_accessors() {
        let frames = vec![
            AnimationFrame {
                rgba_data: vec![
                    255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
                ],
                width: 2,
                height: 2,
                delay: Duration::from_millis(100),
                left: 0,
                top: 0,
            },
            AnimationFrame {
                rgba_data: vec![
                    0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255,
                ],
                width: 2,
                height: 2,
                delay: Duration::from_millis(150),
                left: 0,
                top: 0,
            },
        ];

        let animated = AnimatedImage {
            frames,
            width: 2,
            height: 2,
            loop_count: LoopCount::Finite(3),
            total_duration: Duration::from_millis(250),
        };

        assert_eq!(animated.frame_count(), 2);
        assert_eq!(animated.width(), 2);
        assert_eq!(animated.height(), 2);
        assert_eq!(animated.loop_count(), LoopCount::Finite(3));
        assert_eq!(animated.total_duration(), Duration::from_millis(250));
        assert!(!animated.is_static());

        assert_eq!(animated.frame(0).width(), 2);
        assert_eq!(animated.frame_delay(0), Duration::from_millis(100));
        assert_eq!(animated.frame_delay(1), Duration::from_millis(150));
    }

    #[test]
    fn test_frame_progress() {
        let frames = vec![AnimationFrame {
            rgba_data: vec![255, 0, 0, 255],
            width: 1,
            height: 1,
            delay: Duration::from_millis(100),
            left: 0,
            top: 0,
        }];

        let animated = AnimatedImage {
            frames,
            width: 1,
            height: 1,
            loop_count: LoopCount::Infinite,
            total_duration: Duration::from_millis(100),
        };

        let mut controller = AnimationController::new(&animated);

        assert!((controller.frame_progress() - 0.0).abs() < 0.01);

        controller.update(Duration::from_millis(50));
        assert!((controller.frame_progress() - 0.5).abs() < 0.01);

        assert!(controller.time_until_next_frame() <= Duration::from_millis(50));
    }
}
