//! Progress dialog implementation.
//!
//! This module provides [`ProgressDialog`], a modal dialog that displays the progress
//! of a long-running operation to the user.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::ProgressDialog;
//!
//! // Create a progress dialog
//! let mut dialog = ProgressDialog::new("Processing Files", "Processing...", 100);
//!
//! dialog.canceled.connect(|()| {
//!     println!("User canceled the operation");
//! });
//!
//! dialog.open();
//!
//! // Update progress from your operation
//! for i in 0..100 {
//!     dialog.set_value(i);
//!     dialog.set_label_text(&format!("Processing file {} of 100...", i + 1));
//! }
//! ```

use std::time::{Duration, Instant};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Rect, Renderer, RoundedRect};

use crate::widget::{Key, KeyPressEvent, PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent};

use super::dialog::{Dialog, DialogResult};
use super::dialog_button_box::StandardButton;

// ============================================================================
// ProgressDialog
// ============================================================================

/// A modal dialog displaying the progress of an operation.
///
/// ProgressDialog provides a standardized way to show progress feedback for
/// long-running operations. It supports:
///
/// - Determinate progress (known percentage)
/// - Indeterminate/busy progress (unknown duration)
/// - Label text describing the current operation
/// - Cancel button to allow user interruption
/// - Auto-close when progress reaches maximum
/// - Minimum duration before showing (to avoid flashing for quick operations)
/// - Auto-reset when dialog is shown again
///
/// # Progress Modes
///
/// ## Determinate Mode
///
/// When the total amount of work is known, use determinate mode by setting
/// a range with `set_range()` or by providing a maximum value to `new()`.
///
/// ```ignore
/// let mut dialog = ProgressDialog::new("Downloading", "Starting...", 100);
/// dialog.set_value(50); // 50% complete
/// ```
///
/// ## Indeterminate Mode
///
/// When the duration is unknown, use indeterminate mode by setting the
/// minimum equal to the maximum (typically both 0):
///
/// ```ignore
/// let mut dialog = ProgressDialog::new("Connecting", "Please wait...", 0);
/// dialog.set_range(0, 0); // Enables indeterminate mode
/// ```
///
/// # Minimum Duration
///
/// To prevent the dialog from appearing briefly for fast operations, set a
/// minimum duration with `set_minimum_duration()`. The dialog will only
/// appear if the operation takes longer than this duration.
///
/// ```ignore
/// dialog.set_minimum_duration(Duration::from_millis(500));
/// ```
///
/// # Signals
///
/// - `canceled()`: Emitted when the user clicks Cancel or presses Escape
/// - `value_changed(i32)`: Emitted when the progress value changes
pub struct ProgressDialog {
    /// The underlying dialog.
    dialog: Dialog,

    /// The progress bar (owned, state tracked internally).
    progress_minimum: i32,
    progress_maximum: i32,
    progress_value: i32,

    /// The label text describing the current operation.
    label_text: String,

    /// Whether to automatically close when progress reaches maximum.
    auto_close: bool,

    /// Whether to automatically reset when the dialog is shown.
    auto_reset: bool,

    /// Whether the Cancel button was clicked.
    was_canceled: bool,

    /// Minimum duration before showing the dialog.
    minimum_duration: Duration,

    /// Time when the operation started (for minimum duration check).
    operation_start: Option<Instant>,

    /// Whether the dialog should be shown after minimum duration elapses.
    pending_show: bool,

    // Visual styling
    /// Content padding.
    content_padding: f32,
    /// Spacing between label and progress bar.
    label_progress_spacing: f32,
    /// Progress bar height.
    progress_bar_height: f32,
    /// Progress bar background color.
    progress_background: Color,
    /// Progress bar fill color.
    progress_fill_color: Color,
    /// Progress bar border radius.
    progress_border_radius: f32,

    // Animation state for indeterminate mode
    animation_start: Instant,

    // Signals
    /// Signal emitted when the cancel button is clicked.
    pub canceled: Signal<()>,

    /// Signal emitted when the progress value changes.
    pub value_changed: Signal<i32>,
}

impl ProgressDialog {
    /// Create a new progress dialog.
    ///
    /// # Arguments
    ///
    /// * `title` - The dialog window title
    /// * `label_text` - The text describing the operation
    /// * `maximum` - The maximum progress value (0 for indeterminate mode)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Determinate progress (0-100%)
    /// let dialog = ProgressDialog::new("Copying Files", "Copying...", 100);
    ///
    /// // Indeterminate progress
    /// let dialog = ProgressDialog::new("Connecting", "Please wait...", 0);
    /// ```
    pub fn new(title: impl Into<String>, label_text: impl Into<String>, maximum: i32) -> Self {
        let dialog = Dialog::new(title)
            .with_size(400.0, 140.0)
            .with_standard_buttons(StandardButton::CANCEL);

        Self {
            dialog,
            progress_minimum: 0,
            progress_maximum: maximum,
            progress_value: 0,
            label_text: label_text.into(),
            auto_close: true,
            auto_reset: true,
            was_canceled: false,
            minimum_duration: Duration::ZERO,
            operation_start: None,
            pending_show: false,
            content_padding: 20.0,
            label_progress_spacing: 12.0,
            progress_bar_height: 20.0,
            progress_background: Color::from_rgb8(224, 224, 224),
            progress_fill_color: Color::from_rgb8(66, 133, 244),
            progress_border_radius: 4.0,
            animation_start: Instant::now(),
            canceled: Signal::new(),
            value_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the dialog title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dialog.set_title(title);
        self
    }

    /// Set the label text using builder pattern.
    pub fn with_label_text(mut self, text: impl Into<String>) -> Self {
        self.label_text = text.into();
        self
    }

    /// Set the progress range using builder pattern.
    pub fn with_range(mut self, minimum: i32, maximum: i32) -> Self {
        self.set_range(minimum, maximum);
        self
    }

    /// Set the initial value using builder pattern.
    pub fn with_value(mut self, value: i32) -> Self {
        self.set_value(value);
        self
    }

    /// Set auto-close behavior using builder pattern.
    pub fn with_auto_close(mut self, auto_close: bool) -> Self {
        self.auto_close = auto_close;
        self
    }

    /// Set auto-reset behavior using builder pattern.
    pub fn with_auto_reset(mut self, auto_reset: bool) -> Self {
        self.auto_reset = auto_reset;
        self
    }

    /// Set minimum duration using builder pattern.
    pub fn with_minimum_duration(mut self, duration: Duration) -> Self {
        self.minimum_duration = duration;
        self
    }

    /// Set the dialog size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.dialog = std::mem::take(&mut self.dialog).with_size(width, height);
        self
    }

    /// Enable or disable the cancel button using builder pattern.
    pub fn with_cancel_button(mut self, enabled: bool) -> Self {
        if enabled {
            self.dialog.set_standard_buttons(StandardButton::CANCEL);
        } else {
            self.dialog.set_standard_buttons(StandardButton::NONE);
        }
        self
    }

    // =========================================================================
    // Progress Properties
    // =========================================================================

    /// Get the minimum progress value.
    pub fn minimum(&self) -> i32 {
        self.progress_minimum
    }

    /// Set the minimum progress value.
    pub fn set_minimum(&mut self, minimum: i32) {
        self.set_range(minimum, self.progress_maximum);
    }

    /// Get the maximum progress value.
    pub fn maximum(&self) -> i32 {
        self.progress_maximum
    }

    /// Set the maximum progress value.
    pub fn set_maximum(&mut self, maximum: i32) {
        self.set_range(self.progress_minimum, maximum);
    }

    /// Get the current progress value.
    pub fn value(&self) -> i32 {
        self.progress_value
    }

    /// Set the current progress value.
    ///
    /// The value is clamped to the range [minimum, maximum].
    /// If auto-close is enabled and value reaches maximum, the dialog closes.
    pub fn set_value(&mut self, value: i32) {
        let clamped = if self.progress_minimum <= self.progress_maximum {
            value.clamp(self.progress_minimum, self.progress_maximum)
        } else {
            value.clamp(self.progress_maximum, self.progress_minimum)
        };

        if self.progress_value != clamped {
            self.progress_value = clamped;
            self.dialog.widget_base_mut().update();
            self.value_changed.emit(clamped);

            // Check for auto-close
            if self.auto_close && !self.is_indeterminate() && clamped >= self.progress_maximum {
                self.dialog.accept();
            }
        }
    }

    /// Set the progress range.
    ///
    /// Setting `minimum == maximum` enables indeterminate mode.
    pub fn set_range(&mut self, minimum: i32, maximum: i32) {
        if self.progress_minimum != minimum || self.progress_maximum != maximum {
            self.progress_minimum = minimum;
            self.progress_maximum = maximum;

            // Clamp current value to new range
            self.progress_value = if minimum <= maximum {
                self.progress_value.clamp(minimum, maximum)
            } else {
                self.progress_value.clamp(maximum, minimum)
            };

            // Reset animation for indeterminate mode
            if self.is_indeterminate() {
                self.animation_start = Instant::now();
            }

            self.dialog.widget_base_mut().update();
        }
    }

    /// Check if the progress dialog is in indeterminate mode.
    pub fn is_indeterminate(&self) -> bool {
        self.progress_minimum == self.progress_maximum
    }

    /// Get the progress percentage (0.0 to 1.0).
    ///
    /// Returns 0.0 if in indeterminate mode.
    pub fn progress(&self) -> f32 {
        if self.is_indeterminate() {
            return 0.0;
        }
        let range = (self.progress_maximum - self.progress_minimum) as f32;
        if range == 0.0 {
            return 0.0;
        }
        ((self.progress_value - self.progress_minimum) as f32 / range).clamp(0.0, 1.0)
    }

    /// Reset the progress to minimum value.
    pub fn reset(&mut self) {
        self.was_canceled = false;
        self.progress_value = self.progress_minimum;
        self.animation_start = Instant::now();
        self.dialog.widget_base_mut().update();
    }

    // =========================================================================
    // Label Text
    // =========================================================================

    /// Get the label text.
    pub fn label_text(&self) -> &str {
        &self.label_text
    }

    /// Set the label text describing the current operation.
    pub fn set_label_text(&mut self, text: impl Into<String>) {
        let new_text = text.into();
        if self.label_text != new_text {
            self.label_text = new_text;
            self.dialog.widget_base_mut().update();
        }
    }

    // =========================================================================
    // Auto-Close and Auto-Reset
    // =========================================================================

    /// Check if auto-close is enabled.
    pub fn auto_close(&self) -> bool {
        self.auto_close
    }

    /// Set whether to automatically close when progress reaches maximum.
    pub fn set_auto_close(&mut self, auto_close: bool) {
        self.auto_close = auto_close;
    }

    /// Check if auto-reset is enabled.
    pub fn auto_reset(&self) -> bool {
        self.auto_reset
    }

    /// Set whether to automatically reset when the dialog is shown.
    pub fn set_auto_reset(&mut self, auto_reset: bool) {
        self.auto_reset = auto_reset;
    }

    // =========================================================================
    // Minimum Duration
    // =========================================================================

    /// Get the minimum duration before showing.
    pub fn minimum_duration(&self) -> Duration {
        self.minimum_duration
    }

    /// Set the minimum duration before showing.
    ///
    /// If the operation completes before this duration, the dialog is never shown.
    /// This prevents brief flashing for fast operations.
    pub fn set_minimum_duration(&mut self, duration: Duration) {
        self.minimum_duration = duration;
    }

    // =========================================================================
    // Cancel State
    // =========================================================================

    /// Check if the dialog was canceled.
    pub fn was_canceled(&self) -> bool {
        self.was_canceled
    }

    /// Cancel the dialog programmatically.
    pub fn cancel(&mut self) {
        if !self.was_canceled {
            self.was_canceled = true;
            self.canceled.emit(());
            self.dialog.reject();
        }
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the progress dialog (non-blocking modal).
    ///
    /// If a minimum duration is set, the dialog may not appear immediately.
    /// Call `set_value()` to update progress and the dialog will show if
    /// the minimum duration has elapsed.
    pub fn open(&mut self) {
        // Auto-reset if enabled
        if self.auto_reset {
            self.reset();
        }

        self.was_canceled = false;
        self.operation_start = Some(Instant::now());

        // Check minimum duration
        if self.minimum_duration.is_zero() {
            // Show immediately
            self.dialog.open();
            self.pending_show = false;
        } else {
            // Defer showing
            self.pending_show = true;
            // The dialog will be shown in set_value() if minimum duration elapses
        }
    }

    /// Check if enough time has passed to show the dialog.
    fn check_minimum_duration(&mut self) {
        if self.pending_show
            && let Some(start) = self.operation_start
            && start.elapsed() >= self.minimum_duration
        {
            self.pending_show = false;
            self.dialog.open();
        }
    }

    /// Check if the dialog is currently open.
    pub fn is_open(&self) -> bool {
        self.dialog.is_open() || self.pending_show
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.pending_show = false;
        self.dialog.close();
    }

    /// Get the dialog result.
    pub fn result(&self) -> DialogResult {
        self.dialog.result()
    }

    // =========================================================================
    // Signal Access
    // =========================================================================

    /// Get a reference to the accepted signal.
    pub fn accepted(&self) -> &Signal<()> {
        &self.dialog.accepted
    }

    /// Get a reference to the rejected signal.
    pub fn rejected(&self) -> &Signal<()> {
        &self.dialog.rejected
    }

    /// Get a reference to the finished signal.
    pub fn finished(&self) -> &Signal<DialogResult> {
        &self.dialog.finished
    }

    // =========================================================================
    // Geometry
    // =========================================================================

    /// Get the label rectangle.
    fn label_rect(&self) -> Rect {
        let title_bar_height = 28.0;
        let rect = self.dialog.widget_base().rect();

        Rect::new(
            self.content_padding,
            title_bar_height + self.content_padding,
            rect.width() - self.content_padding * 2.0,
            20.0, // Approximate label height
        )
    }

    /// Get the progress bar rectangle.
    fn progress_bar_rect(&self) -> Rect {
        let label_rect = self.label_rect();
        let rect = self.dialog.widget_base().rect();

        Rect::new(
            self.content_padding,
            label_rect.origin.y + label_rect.height() + self.label_progress_spacing,
            rect.width() - self.content_padding * 2.0,
            self.progress_bar_height,
        )
    }

    /// Get the animation progress for indeterminate mode (0.0 to 1.0, cycling).
    fn animation_progress(&self) -> f32 {
        let elapsed = self.animation_start.elapsed().as_secs_f32();
        // Complete cycle every 2 seconds
        (elapsed % 2.0) / 2.0
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Escape to cancel
        if event.key == Key::Escape {
            self.cancel();
            return true;
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_progress_bar(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.progress_bar_rect();

        // Draw background track
        let track_rrect = RoundedRect::new(rect, self.progress_border_radius);
        ctx.renderer()
            .fill_rounded_rect(track_rrect, self.progress_background);

        if self.is_indeterminate() {
            // Draw indeterminate animated indicator
            self.paint_indeterminate_progress(ctx, rect);
        } else {
            // Draw determinate progress fill
            self.paint_determinate_progress(ctx, rect);
        }
    }

    fn paint_determinate_progress(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        let progress = self.progress();
        if progress <= 0.0 {
            return;
        }

        let fill_width = rect.width() * progress;
        let fill_rect = Rect::new(rect.origin.x, rect.origin.y, fill_width, rect.height());
        let fill_rrect = RoundedRect::new(fill_rect, self.progress_border_radius);
        ctx.renderer()
            .fill_rounded_rect(fill_rrect, self.progress_fill_color);
    }

    fn paint_indeterminate_progress(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        let anim_progress = self.animation_progress();

        // Sliding indicator that moves back and forth
        let indicator_size_ratio = 0.3; // 30% of the bar length
        let indicator_width = rect.width() * indicator_size_ratio;
        let travel_distance = rect.width() - indicator_width;

        // Ping-pong animation
        let position = if anim_progress < 0.5 {
            anim_progress * 2.0
        } else {
            (1.0 - anim_progress) * 2.0
        };

        let x_offset = travel_distance * position;
        let indicator_rect = Rect::new(
            rect.origin.x + x_offset,
            rect.origin.y,
            indicator_width,
            rect.height(),
        );

        let indicator_rrect = RoundedRect::new(indicator_rect, self.progress_border_radius);
        ctx.renderer()
            .fill_rounded_rect(indicator_rrect, self.progress_fill_color);
    }
}

impl Object for ProgressDialog {
    fn object_id(&self) -> ObjectId {
        self.dialog.object_id()
    }
}

impl Widget for ProgressDialog {
    fn widget_base(&self) -> &WidgetBase {
        self.dialog.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.dialog.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        self.dialog.size_hint()
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint the dialog base
        self.dialog.paint(ctx);

        if !self.dialog.is_open() {
            return;
        }

        // Paint progress bar
        self.paint_progress_bar(ctx);

        // Note: Label text would be rendered by the text rendering system
        // The label_text field is stored and would be rendered by the
        // actual rendering implementation
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Check minimum duration on any event to potentially show the dialog
        self.check_minimum_duration();

        // Handle our own events first
        let handled = match event {
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            _ => false,
        };

        if handled {
            return true;
        }

        // Delegate to dialog
        let result = self.dialog.event(event);

        // Check if dialog was rejected (canceled via button)
        if !self.dialog.is_open()
            && self.dialog.result() == DialogResult::Rejected
            && !self.was_canceled
        {
            self.was_canceled = true;
            self.canceled.emit(());
        }

        result
    }
}

impl Default for ProgressDialog {
    fn default() -> Self {
        Self::new("Progress", "", 100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, AtomicI32, Ordering},
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_progress_dialog_creation() {
        setup();
        let dialog = ProgressDialog::new("Test", "Processing...", 100);

        assert_eq!(dialog.minimum(), 0);
        assert_eq!(dialog.maximum(), 100);
        assert_eq!(dialog.value(), 0);
        assert_eq!(dialog.label_text(), "Processing...");
        assert!(!dialog.is_open());
        assert!(!dialog.was_canceled());
    }

    #[test]
    fn test_builder_pattern() {
        setup();
        let dialog = ProgressDialog::new("Test", "Initial", 100)
            .with_title("New Title")
            .with_label_text("Updated label")
            .with_range(10, 200)
            .with_value(50)
            .with_auto_close(false)
            .with_auto_reset(false)
            .with_minimum_duration(Duration::from_millis(500));

        assert_eq!(dialog.minimum(), 10);
        assert_eq!(dialog.maximum(), 200);
        assert_eq!(dialog.value(), 50);
        assert_eq!(dialog.label_text(), "Updated label");
        assert!(!dialog.auto_close());
        assert!(!dialog.auto_reset());
        assert_eq!(dialog.minimum_duration(), Duration::from_millis(500));
    }

    #[test]
    fn test_progress_value_clamping() {
        setup();
        let mut dialog = ProgressDialog::new("Test", "Label", 100);

        dialog.set_value(-10);
        assert_eq!(dialog.value(), 0);

        dialog.set_value(150);
        assert_eq!(dialog.value(), 100);

        dialog.set_value(50);
        assert_eq!(dialog.value(), 50);
    }

    #[test]
    fn test_progress_percentage() {
        setup();
        let mut dialog = ProgressDialog::new("Test", "Label", 100);

        dialog.set_value(0);
        assert!((dialog.progress() - 0.0).abs() < 0.001);

        dialog.set_value(50);
        assert!((dialog.progress() - 0.5).abs() < 0.001);

        dialog.set_value(100);
        assert!((dialog.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_custom_range() {
        setup();
        let mut dialog = ProgressDialog::new("Test", "Label", 0).with_range(10, 20);

        dialog.set_value(10);
        assert!((dialog.progress() - 0.0).abs() < 0.001);

        dialog.set_value(15);
        assert!((dialog.progress() - 0.5).abs() < 0.001);

        dialog.set_value(20);
        assert!((dialog.progress() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_indeterminate_mode() {
        setup();
        let dialog = ProgressDialog::new("Test", "Label", 0).with_range(0, 0);

        assert!(dialog.is_indeterminate());
        assert_eq!(dialog.progress(), 0.0);
    }

    #[test]
    fn test_value_changed_signal() {
        setup();
        let mut dialog = ProgressDialog::new("Test", "Label", 100);
        let last_value = Arc::new(AtomicI32::new(-1));
        let last_value_clone = last_value.clone();

        dialog.value_changed.connect(move |&value| {
            last_value_clone.store(value, Ordering::SeqCst);
        });

        dialog.set_value(42);
        assert_eq!(last_value.load(Ordering::SeqCst), 42);

        dialog.set_value(75);
        assert_eq!(last_value.load(Ordering::SeqCst), 75);
    }

    #[test]
    fn test_cancel_signal() {
        setup();
        let mut dialog = ProgressDialog::new("Test", "Label", 100);
        let was_canceled = Arc::new(AtomicBool::new(false));
        let was_canceled_clone = was_canceled.clone();

        dialog.canceled.connect(move |()| {
            was_canceled_clone.store(true, Ordering::SeqCst);
        });

        dialog.cancel();
        assert!(dialog.was_canceled());
        assert!(was_canceled.load(Ordering::SeqCst));
    }

    #[test]
    fn test_reset() {
        setup();
        let mut dialog = ProgressDialog::new("Test", "Label", 100);
        dialog.set_value(50);
        assert_eq!(dialog.value(), 50);

        dialog.reset();
        assert_eq!(dialog.value(), 0);
        assert!(!dialog.was_canceled());
    }

    #[test]
    fn test_label_text_update() {
        setup();
        let mut dialog = ProgressDialog::new("Test", "Initial", 100);
        assert_eq!(dialog.label_text(), "Initial");

        dialog.set_label_text("Updated");
        assert_eq!(dialog.label_text(), "Updated");
    }

    #[test]
    fn test_no_signal_for_same_value() {
        setup();
        let mut dialog = ProgressDialog::new("Test", "Label", 100);
        dialog.set_value(50);

        let signal_count = Arc::new(AtomicI32::new(0));
        let signal_count_clone = signal_count.clone();

        dialog.value_changed.connect(move |_| {
            signal_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Setting same value should not emit signal
        dialog.set_value(50);
        assert_eq!(signal_count.load(Ordering::SeqCst), 0);

        // Setting different value should emit
        dialog.set_value(51);
        assert_eq!(signal_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_minimum_duration_zero() {
        setup();
        let dialog = ProgressDialog::new("Test", "Label", 100);
        assert_eq!(dialog.minimum_duration(), Duration::ZERO);
    }

    #[test]
    fn test_auto_close_default() {
        setup();
        let dialog = ProgressDialog::new("Test", "Label", 100);
        assert!(dialog.auto_close());
    }

    #[test]
    fn test_auto_reset_default() {
        setup();
        let dialog = ProgressDialog::new("Test", "Label", 100);
        assert!(dialog.auto_reset());
    }

    #[test]
    fn test_disable_cancel_button() {
        setup();
        let dialog = ProgressDialog::new("Test", "Label", 100).with_cancel_button(false);
        assert!(dialog.dialog.standard_buttons().is_empty());
    }
}
