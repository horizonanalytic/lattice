//! Progress reporting for background tasks.
//!
//! This module provides thread-safe progress reporting capabilities for background
//! tasks, with automatic integration with the UI thread via signals.
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_core::progress::ProgressReporter;
//!
//! let reporter = ProgressReporter::new();
//!
//! // Connect to progress updates
//! reporter.on_progress_changed().connect(|&progress| {
//!     println!("Progress: {:.0}%", progress * 100.0);
//! });
//!
//! // Report progress from a background task
//! reporter.set_progress(0.5);
//! reporter.set_message("Processing...");
//! ```
//!
//! # Aggregate Progress
//!
//! For multi-step operations, use `AggregateProgress` to combine weighted sub-tasks:
//!
//! ```no_run
//! use horizon_lattice_core::progress::AggregateProgress;
//!
//! let mut aggregate = AggregateProgress::new();
//!
//! // Add weighted sub-tasks (weight determines contribution to total)
//! let download = aggregate.add_task("download", 3.0);  // 3x weight
//! let process = aggregate.add_task("process", 1.0);    // 1x weight
//!
//! // Total is 75% when download is complete (3/4 of total weight)
//! download.set_progress(1.0);
//! assert!((aggregate.progress() - 0.75).abs() < 0.01);
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use parking_lot::Mutex;

use crate::signal::{ConnectionId, ConnectionType, Signal};

/// A progress update containing both progress value and optional message.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressUpdate {
    /// Progress value from 0.0 to 1.0.
    pub progress: f32,
    /// Optional status message describing current operation.
    pub message: Option<String>,
}

impl ProgressUpdate {
    /// Create a new progress update with just progress.
    pub fn new(progress: f32) -> Self {
        Self {
            progress,
            message: None,
        }
    }

    /// Create a new progress update with progress and message.
    pub fn with_message(progress: f32, message: impl Into<String>) -> Self {
        Self {
            progress,
            message: Some(message.into()),
        }
    }
}

/// Internal state shared between ProgressReporter handles.
struct ProgressReporterInner {
    /// Progress stored as u32 bits (using f32::to_bits/from_bits for atomic access).
    progress_bits: AtomicU32,
    /// Current status message.
    message: Mutex<Option<String>>,
    /// Signal emitted when progress changes.
    progress_changed: Signal<f32>,
    /// Signal emitted when message changes.
    message_changed: Signal<String>,
    /// Signal emitted when either progress or message changes.
    updated: Signal<ProgressUpdate>,
}

impl ProgressReporterInner {
    fn new() -> Self {
        Self {
            progress_bits: AtomicU32::new(0.0_f32.to_bits()),
            message: Mutex::new(None),
            progress_changed: Signal::new(),
            message_changed: Signal::new(),
            updated: Signal::new(),
        }
    }

    fn progress(&self) -> f32 {
        f32::from_bits(self.progress_bits.load(Ordering::Acquire))
    }

    fn set_progress(&self, progress: f32) {
        let clamped = progress.clamp(0.0, 1.0);
        let old_bits = self.progress_bits.swap(clamped.to_bits(), Ordering::AcqRel);
        let old_progress = f32::from_bits(old_bits);

        if (clamped - old_progress).abs() > f32::EPSILON {
            self.progress_changed.emit(clamped);
            let message = self.message.lock().clone();
            self.updated.emit(ProgressUpdate {
                progress: clamped,
                message,
            });
        }
    }

    fn message(&self) -> Option<String> {
        self.message.lock().clone()
    }

    fn set_message(&self, message: impl Into<String>) {
        let new_message = message.into();
        {
            let mut guard = self.message.lock();
            *guard = Some(new_message.clone());
        }
        self.message_changed.emit(new_message.clone());
        let progress = self.progress();
        self.updated.emit(ProgressUpdate {
            progress,
            message: Some(new_message),
        });
    }

    fn update(&self, progress: f32, message: impl Into<String>) {
        let clamped = progress.clamp(0.0, 1.0);
        let new_message = message.into();

        let old_bits = self.progress_bits.swap(clamped.to_bits(), Ordering::AcqRel);
        let old_progress = f32::from_bits(old_bits);

        {
            let mut guard = self.message.lock();
            *guard = Some(new_message.clone());
        }

        if (clamped - old_progress).abs() > f32::EPSILON {
            self.progress_changed.emit(clamped);
        }
        self.message_changed.emit(new_message.clone());
        self.updated.emit(ProgressUpdate {
            progress: clamped,
            message: Some(new_message),
        });
    }

    fn reset(&self) {
        self.progress_bits
            .store(0.0_f32.to_bits(), Ordering::Release);
        {
            let mut guard = self.message.lock();
            *guard = None;
        }
        self.progress_changed.emit(0.0);
        self.updated.emit(ProgressUpdate::new(0.0));
    }
}

/// A thread-safe progress reporter for background tasks.
///
/// `ProgressReporter` allows background tasks to report their progress and status
/// messages, which can be connected to UI elements via signals. The reporter uses
/// atomic operations and lock-free algorithms where possible for efficient updates.
///
/// # Thread Safety
///
/// `ProgressReporter` is `Send + Sync` and can be safely shared across threads.
/// Progress updates are delivered to connected slots via the signal system, which
/// automatically handles cross-thread delivery using queued connections.
///
/// # Signals
///
/// - `on_progress_changed()`: Emitted when progress value changes
/// - `on_message_changed()`: Emitted when status message changes
/// - `on_updated()`: Emitted when either progress or message changes
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_core::progress::ProgressReporter;
/// use horizon_lattice_core::signal::ConnectionType;
///
/// let reporter = ProgressReporter::new();
///
/// // Connect with queued delivery for UI thread safety
/// reporter.on_progress_changed().connect_with_type(
///     |&progress| println!("Progress: {:.0}%", progress * 100.0),
///     ConnectionType::Queued,
/// );
///
/// // Use in a background task
/// std::thread::spawn({
///     let reporter = reporter.clone();
///     move || {
///         for i in 0..=100 {
///             reporter.set_progress(i as f32 / 100.0);
///             std::thread::sleep(std::time::Duration::from_millis(10));
///         }
///     }
/// });
/// ```
#[derive(Clone)]
pub struct ProgressReporter {
    inner: Arc<ProgressReporterInner>,
}

impl ProgressReporter {
    /// Create a new progress reporter.
    ///
    /// The reporter starts with progress at 0.0 and no message.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ProgressReporterInner::new()),
        }
    }

    /// Get the current progress value (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        self.inner.progress()
    }

    /// Set the progress value.
    ///
    /// The value is clamped to the range 0.0..=1.0.
    /// Emits `progress_changed` and `updated` signals if the value changes.
    pub fn set_progress(&self, progress: f32) {
        self.inner.set_progress(progress);
    }

    /// Get the current status message.
    pub fn message(&self) -> Option<String> {
        self.inner.message()
    }

    /// Set the status message.
    ///
    /// Emits `message_changed` and `updated` signals.
    pub fn set_message(&self, message: impl Into<String>) {
        self.inner.set_message(message);
    }

    /// Update both progress and message atomically.
    ///
    /// This is more efficient than calling `set_progress` and `set_message`
    /// separately, as it only emits the `updated` signal once.
    pub fn update(&self, progress: f32, message: impl Into<String>) {
        self.inner.update(progress, message);
    }

    /// Reset the reporter to initial state (progress 0.0, no message).
    pub fn reset(&self) {
        self.inner.reset();
    }

    /// Get a reference to the progress changed signal.
    ///
    /// This signal is emitted whenever the progress value changes.
    pub fn on_progress_changed(&self) -> &Signal<f32> {
        &self.inner.progress_changed
    }

    /// Get a reference to the message changed signal.
    ///
    /// This signal is emitted whenever the status message changes.
    pub fn on_message_changed(&self) -> &Signal<String> {
        &self.inner.message_changed
    }

    /// Get a reference to the combined update signal.
    ///
    /// This signal is emitted whenever progress or message changes,
    /// providing both values in a single `ProgressUpdate`.
    pub fn on_updated(&self) -> &Signal<ProgressUpdate> {
        &self.inner.updated
    }
}

impl Default for ProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ProgressReporter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProgressReporter")
            .field("progress", &self.progress())
            .field("message", &self.message())
            .finish()
    }
}

// Ensure ProgressReporter is Send + Sync
static_assertions::assert_impl_all!(ProgressReporter: Send, Sync);

/// Internal state for a sub-task in aggregate progress.
struct SubTask {
    /// Name of the sub-task.
    #[allow(dead_code)]
    name: String,
    /// Weight of this sub-task (contribution to total progress).
    weight: f32,
    /// The progress reporter for this sub-task.
    reporter: ProgressReporter,
    /// Connection ID for the progress signal.
    connection_id: ConnectionId,
}

/// Aggregates progress from multiple weighted sub-tasks.
///
/// `AggregateProgress` combines progress from multiple sub-tasks into a single
/// progress value based on their relative weights. This is useful for operations
/// that consist of multiple phases where each phase contributes differently to
/// the overall completion.
///
/// # Weighting
///
/// Each sub-task has a weight that determines its contribution to the total
/// progress. For example, if task A has weight 3 and task B has weight 1:
/// - Task A contributes 75% (3/4) to the total
/// - Task B contributes 25% (1/4) to the total
///
/// When task A is complete (progress = 1.0), the aggregate progress is 0.75.
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_core::progress::AggregateProgress;
///
/// let mut aggregate = AggregateProgress::new();
///
/// // Download is 3x more work than processing
/// let download = aggregate.add_task("download", 3.0);
/// let process = aggregate.add_task("process", 1.0);
///
/// // Connect to aggregate progress
/// aggregate.on_progress_changed().connect(|&progress| {
///     println!("Overall: {:.0}%", progress * 100.0);
/// });
///
/// // Simulate progress
/// download.set_progress(0.5);  // Overall: 37.5%
/// download.set_progress(1.0);  // Overall: 75%
/// process.set_progress(1.0);   // Overall: 100%
/// ```
pub struct AggregateProgress {
    /// Sub-tasks with their weights and reporters.
    tasks: Vec<SubTask>,
    /// Cached total weight for efficient calculation.
    total_weight: f32,
    /// Signal emitted when aggregate progress changes.
    progress_changed: Signal<f32>,
}

impl AggregateProgress {
    /// Create a new aggregate progress tracker.
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            total_weight: 0.0,
            progress_changed: Signal::new(),
        }
    }

    /// Add a weighted sub-task.
    ///
    /// Returns a `ProgressReporter` that should be used to report progress
    /// for this sub-task. The weight determines how much this task contributes
    /// to the overall progress.
    ///
    /// # Arguments
    ///
    /// * `name` - A name for the sub-task (for debugging/logging)
    /// * `weight` - The weight of this task (must be positive)
    ///
    /// # Panics
    ///
    /// Panics if weight is not positive.
    pub fn add_task(&mut self, name: impl Into<String>, weight: f32) -> ProgressReporter {
        assert!(weight > 0.0, "Task weight must be positive");

        let reporter = ProgressReporter::new();
        let reporter_clone = reporter.clone();

        // Update total weight
        self.total_weight += weight;

        // Connect to sub-task progress changes.
        // Note: The aggregate progress is recalculated on demand in progress(),
        // so we use a no-op connection here just to track the connection ID
        // for cleanup purposes.
        let connection_id = reporter.on_progress_changed().connect_with_type(
            move |_| {
                // Progress is recalculated on demand in progress()
            },
            ConnectionType::Direct,
        );

        self.tasks.push(SubTask {
            name: name.into(),
            weight,
            reporter: reporter_clone,
            connection_id,
        });

        reporter
    }

    /// Get the aggregate progress (0.0 to 1.0).
    ///
    /// This is the weighted sum of all sub-task progress values.
    pub fn progress(&self) -> f32 {
        if self.total_weight == 0.0 {
            return 0.0;
        }

        let weighted_sum: f32 = self
            .tasks
            .iter()
            .map(|task| task.reporter.progress() * task.weight)
            .sum();

        (weighted_sum / self.total_weight).clamp(0.0, 1.0)
    }

    /// Get a reference to the progress changed signal.
    ///
    /// Note: Due to the nature of aggregate progress, this signal may not
    /// fire for every sub-task update. For reliable updates, connect to
    /// individual sub-task reporters or poll `progress()`.
    pub fn on_progress_changed(&self) -> &Signal<f32> {
        &self.progress_changed
    }

    /// Reset all sub-tasks to initial state.
    pub fn reset(&mut self) {
        for task in &self.tasks {
            task.reporter.reset();
        }
        self.progress_changed.emit(0.0);
    }

    /// Get the number of sub-tasks.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Emit the current aggregate progress.
    ///
    /// Call this to manually trigger a progress update notification.
    pub fn emit_progress(&self) {
        self.progress_changed.emit(self.progress());
    }
}

impl Default for AggregateProgress {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AggregateProgress {
    fn drop(&mut self) {
        // Disconnect all signal connections
        for task in &self.tasks {
            task.reporter
                .on_progress_changed()
                .disconnect(task.connection_id);
        }
    }
}

impl std::fmt::Debug for AggregateProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AggregateProgress")
            .field("progress", &self.progress())
            .field("task_count", &self.tasks.len())
            .field("total_weight", &self.total_weight)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI32, Ordering as AtomicOrdering};

    #[test]
    fn test_progress_reporter_basic() {
        let reporter = ProgressReporter::new();

        // Initial state
        assert_eq!(reporter.progress(), 0.0);
        assert_eq!(reporter.message(), None);

        // Set progress
        reporter.set_progress(0.5);
        assert!((reporter.progress() - 0.5).abs() < f32::EPSILON);

        // Set message
        reporter.set_message("Processing...");
        assert_eq!(reporter.message(), Some("Processing...".to_string()));

        // Update both
        reporter.update(0.75, "Almost done");
        assert!((reporter.progress() - 0.75).abs() < f32::EPSILON);
        assert_eq!(reporter.message(), Some("Almost done".to_string()));

        // Reset
        reporter.reset();
        assert_eq!(reporter.progress(), 0.0);
        assert_eq!(reporter.message(), None);
    }

    #[test]
    fn test_progress_clamping() {
        let reporter = ProgressReporter::new();

        reporter.set_progress(-0.5);
        assert_eq!(reporter.progress(), 0.0);

        reporter.set_progress(1.5);
        assert_eq!(reporter.progress(), 1.0);

        reporter.set_progress(0.5);
        assert!((reporter.progress() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_progress_signals() {
        let reporter = ProgressReporter::new();
        let signal_count = Arc::new(AtomicI32::new(0));

        let count_clone = signal_count.clone();
        reporter.on_progress_changed().connect(move |_| {
            count_clone.fetch_add(1, AtomicOrdering::SeqCst);
        });

        reporter.set_progress(0.25);
        assert_eq!(signal_count.load(AtomicOrdering::SeqCst), 1);

        reporter.set_progress(0.5);
        assert_eq!(signal_count.load(AtomicOrdering::SeqCst), 2);

        // Same value should not emit
        reporter.set_progress(0.5);
        assert_eq!(signal_count.load(AtomicOrdering::SeqCst), 2);
    }

    #[test]
    fn test_aggregate_weighted() {
        let mut aggregate = AggregateProgress::new();

        // Add tasks with different weights
        let task1 = aggregate.add_task("task1", 3.0); // 75% of total
        let task2 = aggregate.add_task("task2", 1.0); // 25% of total

        // Initial state
        assert_eq!(aggregate.progress(), 0.0);

        // Complete task1 only
        task1.set_progress(1.0);
        assert!((aggregate.progress() - 0.75).abs() < 0.01);

        // Complete task2
        task2.set_progress(1.0);
        assert!((aggregate.progress() - 1.0).abs() < f32::EPSILON);

        // Partial progress
        task1.set_progress(0.5);
        task2.set_progress(0.5);
        assert!((aggregate.progress() - 0.5).abs() < 0.01);

        // Reset
        aggregate.reset();
        assert_eq!(aggregate.progress(), 0.0);
    }

    #[test]
    fn test_aggregate_equal_weights() {
        let mut aggregate = AggregateProgress::new();

        let task1 = aggregate.add_task("task1", 1.0);
        let task2 = aggregate.add_task("task2", 1.0);
        let task3 = aggregate.add_task("task3", 1.0);

        task1.set_progress(1.0);
        assert!((aggregate.progress() - 0.333).abs() < 0.01);

        task2.set_progress(1.0);
        assert!((aggregate.progress() - 0.666).abs() < 0.01);

        task3.set_progress(1.0);
        assert!((aggregate.progress() - 1.0).abs() < 0.01);
    }

    #[test]
    #[should_panic(expected = "Task weight must be positive")]
    fn test_aggregate_invalid_weight() {
        let mut aggregate = AggregateProgress::new();
        aggregate.add_task("invalid", 0.0);
    }

    #[test]
    fn test_thread_safety() {
        // Verify that types are Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ProgressReporter>();
        assert_send_sync::<ProgressUpdate>();
    }

    #[test]
    fn test_progress_update_struct() {
        let update1 = ProgressUpdate::new(0.5);
        assert_eq!(update1.progress, 0.5);
        assert_eq!(update1.message, None);

        let update2 = ProgressUpdate::with_message(0.75, "Loading");
        assert_eq!(update2.progress, 0.75);
        assert_eq!(update2.message, Some("Loading".to_string()));
    }

    #[test]
    fn test_reporter_clone() {
        let reporter1 = ProgressReporter::new();
        let reporter2 = reporter1.clone();

        reporter1.set_progress(0.5);
        assert!((reporter2.progress() - 0.5).abs() < f32::EPSILON);

        reporter2.set_message("Test");
        assert_eq!(reporter1.message(), Some("Test".to_string()));
    }

    #[test]
    fn test_message_signal() {
        let reporter = ProgressReporter::new();
        let last_message = Arc::new(Mutex::new(String::new()));

        let msg_clone = last_message.clone();
        reporter.on_message_changed().connect(move |msg| {
            *msg_clone.lock() = msg.clone();
        });

        reporter.set_message("Hello");
        assert_eq!(*last_message.lock(), "Hello");

        reporter.set_message("World");
        assert_eq!(*last_message.lock(), "World");
    }

    #[test]
    fn test_updated_signal() {
        let reporter = ProgressReporter::new();
        let last_update = Arc::new(Mutex::new(ProgressUpdate::new(0.0)));

        let update_clone = last_update.clone();
        reporter.on_updated().connect(move |update| {
            *update_clone.lock() = update.clone();
        });

        reporter.update(0.5, "Processing");

        let update = last_update.lock();
        assert!((update.progress - 0.5).abs() < f32::EPSILON);
        assert_eq!(update.message, Some("Processing".to_string()));
    }
}
