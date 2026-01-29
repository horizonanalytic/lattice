//! Progress bar widget for displaying progress of operations.
//!
//! The ProgressBar widget displays progress with support for:
//! - Determinate progress (0-100%)
//! - Indeterminate/busy mode (animated)
//! - Customizable text display with format strings
//! - Horizontal and vertical orientations
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{ProgressBar, Orientation};
//!
//! // Create a simple progress bar
//! let mut progress = ProgressBar::new();
//! progress.set_value(50);  // 50%
//!
//! // Create with custom range
//! let mut download = ProgressBar::new()
//!     .with_range(0, 1000)
//!     .with_format("%v/%m bytes");
//!
//! // Create indeterminate (busy) progress bar
//! let mut busy = ProgressBar::new()
//!     .with_range(0, 0);  // min == max == 0 enables indeterminate mode
//!
//! // Vertical progress bar
//! let mut vertical = ProgressBar::new()
//!     .with_orientation(Orientation::Vertical);
//! ```

use std::time::Instant;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, HorizontalAlign, Point, Renderer, RoundedRect, TextLayout,
    TextLayoutOptions, TextRenderer,
};

use crate::widget::{
    FocusPolicy, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
};

/// Orientation for the progress bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Orientation {
    /// Progress fills from left to right (or right to left if inverted).
    #[default]
    Horizontal,
    /// Progress fills from bottom to top (or top to bottom if inverted).
    Vertical,
}

/// A widget that displays progress of an operation.
///
/// ProgressBar shows visual feedback for the completion status of a task.
/// It supports both determinate progress (known completion percentage) and
/// indeterminate mode (unknown duration - animated busy indicator).
///
/// # Progress Range
///
/// By default, the progress bar ranges from 0 to 100. You can customize
/// this with `set_range()` or `with_range()`. The progress percentage
/// is calculated as: `(value - minimum) / (maximum - minimum) * 100`.
///
/// # Indeterminate Mode
///
/// When `minimum == maximum` (typically both 0), the progress bar enters
/// indeterminate mode. This displays an animated indicator suitable for
/// operations where progress cannot be determined.
///
/// # Text Format
///
/// The progress bar can display text using format placeholders:
/// - `%p` - Percentage complete (e.g., "50")
/// - `%v` - Current value
/// - `%m` - Maximum value
///
/// Default format is `"%p%"` which displays "50%" for half completion.
///
/// # Signals
///
/// - `value_changed(i32)`: Emitted when the value changes
pub struct ProgressBar {
    /// Widget base for common functionality.
    base: WidgetBase,

    /// Minimum value of the progress range.
    minimum: i32,

    /// Maximum value of the progress range.
    maximum: i32,

    /// Current progress value.
    value: i32,

    /// Progress bar orientation.
    orientation: Orientation,

    /// Whether to display the progress text.
    text_visible: bool,

    /// Format string for the progress text.
    /// Supports %p (percentage), %v (value), %m (maximum).
    format: String,

    /// Whether to invert the progress direction.
    inverted_appearance: bool,

    /// Background color of the progress bar track.
    background_color: Color,

    /// Fill color of the progress indicator.
    progress_color: Color,

    /// Border radius for rounded corners.
    border_radius: f32,

    /// Font for text rendering.
    font: Font,

    /// Text color.
    text_color: Color,

    /// Start time for indeterminate animation.
    animation_start: Instant,

    /// Signal emitted when the value changes.
    pub value_changed: Signal<i32>,
}

impl ProgressBar {
    /// Create a new progress bar with default settings.
    ///
    /// The progress bar is created with:
    /// - Range: 0 to 100
    /// - Value: 0
    /// - Horizontal orientation
    /// - Text visible with "%p%" format
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        // Progress bars don't receive focus
        base.set_focus_policy(FocusPolicy::NoFocus);
        // Set appropriate size policy
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Fixed,
        ));

        Self {
            base,
            minimum: 0,
            maximum: 100,
            value: 0,
            orientation: Orientation::Horizontal,
            text_visible: true,
            format: "%p%".to_string(),
            inverted_appearance: false,
            background_color: Color::from_rgb8(224, 224, 224),
            progress_color: Color::from_rgb8(66, 133, 244), // Google Blue
            border_radius: 4.0,
            font: Font::new(FontFamily::SansSerif, 12.0),
            text_color: Color::BLACK,
            animation_start: Instant::now(),
            value_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Value and Range Methods
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

    /// Set the current progress value.
    ///
    /// The value is clamped to the range [minimum, maximum].
    pub fn set_value(&mut self, value: i32) {
        let clamped = if self.minimum <= self.maximum {
            value.clamp(self.minimum, self.maximum)
        } else {
            value.clamp(self.maximum, self.minimum)
        };

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

    /// Set the progress range.
    ///
    /// Setting `minimum == maximum` (typically both 0) enables indeterminate mode.
    pub fn set_range(&mut self, minimum: i32, maximum: i32) {
        if self.minimum != minimum || self.maximum != maximum {
            self.minimum = minimum;
            self.maximum = maximum;
            // Clamp current value to new range
            self.value = if minimum <= maximum {
                self.value.clamp(minimum, maximum)
            } else {
                self.value.clamp(maximum, minimum)
            };
            // Reset animation for indeterminate mode
            if self.is_indeterminate() {
                self.animation_start = Instant::now();
            }
            self.base.update();
        }
    }

    /// Set range using builder pattern.
    pub fn with_range(mut self, minimum: i32, maximum: i32) -> Self {
        self.set_range(minimum, maximum);
        self
    }

    /// Check if the progress bar is in indeterminate mode.
    ///
    /// Indeterminate mode is active when `minimum == maximum`.
    pub fn is_indeterminate(&self) -> bool {
        self.minimum == self.maximum
    }

    /// Get the progress percentage (0.0 to 1.0).
    ///
    /// Returns 0.0 if in indeterminate mode or if range is invalid.
    pub fn progress(&self) -> f32 {
        if self.is_indeterminate() {
            return 0.0;
        }
        let range = (self.maximum - self.minimum) as f32;
        if range == 0.0 {
            return 0.0;
        }
        ((self.value - self.minimum) as f32 / range).clamp(0.0, 1.0)
    }

    /// Reset the progress bar to minimum value.
    pub fn reset(&mut self) {
        self.set_value(self.minimum);
    }

    // =========================================================================
    // Orientation Methods
    // =========================================================================

    /// Get the orientation.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Set the orientation.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        if self.orientation != orientation {
            self.orientation = orientation;
            // Swap size policy for vertical orientation
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
    // Text Display Methods
    // =========================================================================

    /// Check if text is visible.
    pub fn text_visible(&self) -> bool {
        self.text_visible
    }

    /// Set whether to display progress text.
    pub fn set_text_visible(&mut self, visible: bool) {
        if self.text_visible != visible {
            self.text_visible = visible;
            self.base.update();
        }
    }

    /// Set text visibility using builder pattern.
    pub fn with_text_visible(mut self, visible: bool) -> Self {
        self.text_visible = visible;
        self
    }

    /// Get the format string.
    pub fn format(&self) -> &str {
        &self.format
    }

    /// Set the format string for progress text.
    ///
    /// Supported placeholders:
    /// - `%p` - Percentage complete (0-100)
    /// - `%v` - Current value
    /// - `%m` - Maximum value
    pub fn set_format(&mut self, format: impl Into<String>) {
        let new_format = format.into();
        if self.format != new_format {
            self.format = new_format;
            self.base.update();
        }
    }

    /// Set format using builder pattern.
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = format.into();
        self
    }

    /// Get the formatted progress text.
    pub fn text(&self) -> String {
        if self.is_indeterminate() {
            return String::new();
        }

        let percentage = (self.progress() * 100.0).round() as i32;
        self.format
            .replace("%p", &percentage.to_string())
            .replace("%v", &self.value.to_string())
            .replace("%m", &self.maximum.to_string())
    }

    // =========================================================================
    // Appearance Methods
    // =========================================================================

    /// Check if appearance is inverted.
    pub fn inverted_appearance(&self) -> bool {
        self.inverted_appearance
    }

    /// Set whether to invert the progress direction.
    ///
    /// When inverted:
    /// - Horizontal: fills right to left
    /// - Vertical: fills top to bottom
    pub fn set_inverted_appearance(&mut self, inverted: bool) {
        if self.inverted_appearance != inverted {
            self.inverted_appearance = inverted;
            self.base.update();
        }
    }

    /// Set inverted appearance using builder pattern.
    pub fn with_inverted_appearance(mut self, inverted: bool) -> Self {
        self.inverted_appearance = inverted;
        self
    }

    /// Get the background color.
    pub fn background_color(&self) -> Color {
        self.background_color
    }

    /// Set the background (track) color.
    pub fn set_background_color(&mut self, color: Color) {
        if self.background_color != color {
            self.background_color = color;
            self.base.update();
        }
    }

    /// Set background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Get the progress fill color.
    pub fn progress_color(&self) -> Color {
        self.progress_color
    }

    /// Set the progress fill color.
    pub fn set_progress_color(&mut self, color: Color) {
        if self.progress_color != color {
            self.progress_color = color;
            self.base.update();
        }
    }

    /// Set progress color using builder pattern.
    pub fn with_progress_color(mut self, color: Color) -> Self {
        self.progress_color = color;
        self
    }

    /// Get the border radius.
    pub fn border_radius(&self) -> f32 {
        self.border_radius
    }

    /// Set the border radius for rounded corners.
    pub fn set_border_radius(&mut self, radius: f32) {
        if self.border_radius != radius {
            self.border_radius = radius;
            self.base.update();
        }
    }

    /// Set border radius using builder pattern.
    pub fn with_border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    /// Get the font.
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// Set the font for text rendering.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
        self.base.update();
    }

    /// Set font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Get the text color.
    pub fn text_color(&self) -> Color {
        self.text_color
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        if self.text_color != color {
            self.text_color = color;
            self.base.update();
        }
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    // =========================================================================
    // Animation Methods
    // =========================================================================

    /// Reset the animation timer.
    ///
    /// Call this when starting a new indeterminate operation.
    pub fn reset_animation(&mut self) {
        self.animation_start = Instant::now();
    }

    /// Get the animation progress for indeterminate mode (0.0 to 1.0, cycling).
    fn animation_progress(&self) -> f32 {
        let elapsed = self.animation_start.elapsed().as_secs_f32();
        // Complete cycle every 2 seconds
        (elapsed % 2.0) / 2.0
    }
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ProgressBar {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ProgressBar {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        match self.orientation {
            Orientation::Horizontal => {
                SizeHint::from_dimensions(200.0, 24.0).with_minimum_dimensions(40.0, 16.0)
            }
            Orientation::Vertical => {
                SizeHint::from_dimensions(24.0, 200.0).with_minimum_dimensions(16.0, 40.0)
            }
        }
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Draw background track
        let track_rrect = RoundedRect::new(rect, self.border_radius);
        ctx.renderer()
            .fill_rounded_rect(track_rrect, self.background_color);

        if self.is_indeterminate() {
            // Draw indeterminate animated indicator
            self.paint_indeterminate(ctx);
        } else {
            // Draw determinate progress fill
            self.paint_determinate(ctx);
        }

        // Draw progress text
        if self.text_visible && !self.is_indeterminate() {
            self.paint_text(ctx);
        }
    }
}

impl ProgressBar {
    /// Paint the determinate progress fill.
    fn paint_determinate(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        let progress = self.progress();

        if progress <= 0.0 {
            return;
        }

        let fill_rect = match self.orientation {
            Orientation::Horizontal => {
                let fill_width = rect.width() * progress;
                if self.inverted_appearance {
                    // Fill from right
                    horizon_lattice_render::Rect::new(
                        rect.origin.x + rect.width() - fill_width,
                        rect.origin.y,
                        fill_width,
                        rect.height(),
                    )
                } else {
                    // Fill from left
                    horizon_lattice_render::Rect::new(
                        rect.origin.x,
                        rect.origin.y,
                        fill_width,
                        rect.height(),
                    )
                }
            }
            Orientation::Vertical => {
                let fill_height = rect.height() * progress;
                if self.inverted_appearance {
                    // Fill from top
                    horizon_lattice_render::Rect::new(
                        rect.origin.x,
                        rect.origin.y,
                        rect.width(),
                        fill_height,
                    )
                } else {
                    // Fill from bottom
                    horizon_lattice_render::Rect::new(
                        rect.origin.x,
                        rect.origin.y + rect.height() - fill_height,
                        rect.width(),
                        fill_height,
                    )
                }
            }
        };

        let fill_rrect = RoundedRect::new(fill_rect, self.border_radius);
        ctx.renderer()
            .fill_rounded_rect(fill_rrect, self.progress_color);
    }

    /// Paint the indeterminate (busy) indicator.
    fn paint_indeterminate(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        let anim_progress = self.animation_progress();

        // Sliding indicator that moves back and forth
        let indicator_size_ratio = 0.3; // 30% of the bar length

        match self.orientation {
            Orientation::Horizontal => {
                let indicator_width = rect.width() * indicator_size_ratio;
                let travel_distance = rect.width() - indicator_width;

                // Ping-pong animation
                let position = if anim_progress < 0.5 {
                    anim_progress * 2.0
                } else {
                    (1.0 - anim_progress) * 2.0
                };

                let x_offset = travel_distance * position;
                let indicator_rect = horizon_lattice_render::Rect::new(
                    rect.origin.x + x_offset,
                    rect.origin.y,
                    indicator_width,
                    rect.height(),
                );

                let indicator_rrect = RoundedRect::new(indicator_rect, self.border_radius);
                ctx.renderer()
                    .fill_rounded_rect(indicator_rrect, self.progress_color);
            }
            Orientation::Vertical => {
                let indicator_height = rect.height() * indicator_size_ratio;
                let travel_distance = rect.height() - indicator_height;

                // Ping-pong animation
                let position = if anim_progress < 0.5 {
                    anim_progress * 2.0
                } else {
                    (1.0 - anim_progress) * 2.0
                };

                let y_offset = travel_distance * position;
                let indicator_rect = horizon_lattice_render::Rect::new(
                    rect.origin.x,
                    rect.origin.y + y_offset,
                    rect.width(),
                    indicator_height,
                );

                let indicator_rrect = RoundedRect::new(indicator_rect, self.border_radius);
                ctx.renderer()
                    .fill_rounded_rect(indicator_rrect, self.progress_color);
            }
        }
    }

    /// Paint the progress text.
    fn paint_text(&self, ctx: &mut PaintContext<'_>) {
        let text = self.text();
        if text.is_empty() {
            return;
        }

        let rect = ctx.rect();
        let mut font_system = FontSystem::new();

        let layout = TextLayout::with_options(
            &mut font_system,
            &text,
            &self.font,
            TextLayoutOptions::new().horizontal_align(HorizontalAlign::Center),
        );

        // Center the text in the progress bar
        let text_x = rect.origin.x + (rect.width() - layout.width()) / 2.0;
        let text_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;
        let text_pos = Point::new(text_x, text_y);

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ =
                text_renderer.prepare_layout(&mut font_system, &layout, text_pos, self.text_color);
        }
    }
}

// Ensure ProgressBar is Send + Sync
static_assertions::assert_impl_all!(ProgressBar: Send, Sync);

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
    fn test_progress_bar_creation() {
        setup();
        let bar = ProgressBar::new();
        assert_eq!(bar.minimum(), 0);
        assert_eq!(bar.maximum(), 100);
        assert_eq!(bar.value(), 0);
        assert_eq!(bar.orientation(), Orientation::Horizontal);
        assert!(bar.text_visible());
        assert_eq!(bar.format(), "%p%");
    }

    #[test]
    fn test_progress_bar_builder_pattern() {
        setup();
        let bar = ProgressBar::new()
            .with_range(0, 1000)
            .with_value(500)
            .with_format("%v/%m")
            .with_orientation(Orientation::Vertical)
            .with_text_visible(false)
            .with_inverted_appearance(true);

        assert_eq!(bar.minimum(), 0);
        assert_eq!(bar.maximum(), 1000);
        assert_eq!(bar.value(), 500);
        assert_eq!(bar.orientation(), Orientation::Vertical);
        assert!(!bar.text_visible());
        assert!(bar.inverted_appearance());
    }

    #[test]
    fn test_progress_percentage() {
        setup();
        let mut bar = ProgressBar::new();

        bar.set_value(0);
        assert!((bar.progress() - 0.0).abs() < 0.001);

        bar.set_value(50);
        assert!((bar.progress() - 0.5).abs() < 0.001);

        bar.set_value(100);
        assert!((bar.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_progress_custom_range() {
        setup();
        let mut bar = ProgressBar::new().with_range(10, 20);

        bar.set_value(10);
        assert!((bar.progress() - 0.0).abs() < 0.001);

        bar.set_value(15);
        assert!((bar.progress() - 0.5).abs() < 0.001);

        bar.set_value(20);
        assert!((bar.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_value_clamping() {
        setup();
        let mut bar = ProgressBar::new().with_range(0, 100);

        bar.set_value(-10);
        assert_eq!(bar.value(), 0);

        bar.set_value(150);
        assert_eq!(bar.value(), 100);
    }

    #[test]
    fn test_indeterminate_mode() {
        setup();
        let bar = ProgressBar::new().with_range(0, 0);
        assert!(bar.is_indeterminate());
        assert_eq!(bar.progress(), 0.0);
    }

    #[test]
    fn test_text_formatting() {
        setup();
        let bar = ProgressBar::new()
            .with_range(0, 200)
            .with_value(100)
            .with_format("%p% (%v of %m)");

        assert_eq!(bar.text(), "50% (100 of 200)");
    }

    #[test]
    fn test_value_changed_signal() {
        setup();
        let mut bar = ProgressBar::new();
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
    fn test_reset() {
        setup();
        let mut bar = ProgressBar::new().with_value(50);
        assert_eq!(bar.value(), 50);

        bar.reset();
        assert_eq!(bar.value(), 0);
    }

    #[test]
    fn test_size_hint_horizontal() {
        setup();
        let bar = ProgressBar::new();
        let hint = bar.size_hint();

        assert!(hint.preferred.width >= 100.0);
        assert!(hint.preferred.height >= 16.0);
    }

    #[test]
    fn test_size_hint_vertical() {
        setup();
        let bar = ProgressBar::new().with_orientation(Orientation::Vertical);
        let hint = bar.size_hint();

        assert!(hint.preferred.height >= 100.0);
        assert!(hint.preferred.width >= 16.0);
    }

    #[test]
    fn test_orientation_default() {
        setup();
        assert_eq!(Orientation::default(), Orientation::Horizontal);
    }

    #[test]
    fn test_no_signal_for_same_value() {
        setup();
        let mut bar = ProgressBar::new().with_value(50);
        let signal_count = Arc::new(AtomicI32::new(0));
        let signal_count_clone = signal_count.clone();

        bar.value_changed.connect(move |_| {
            signal_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Setting same value should not emit signal
        bar.set_value(50);
        assert_eq!(signal_count.load(Ordering::SeqCst), 0);

        // Setting different value should emit
        bar.set_value(51);
        assert_eq!(signal_count.load(Ordering::SeqCst), 1);
    }
}
