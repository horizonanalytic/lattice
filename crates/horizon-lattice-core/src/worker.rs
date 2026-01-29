//! Worker pattern for dedicated background thread processing.
//!
//! This module provides a `Worker` type that manages a dedicated thread with its own
//! task queue, enabling a producer-consumer pattern for background work. Unlike the
//! thread pool which distributes work across multiple threads, a Worker processes
//! tasks sequentially on a single dedicated thread.
//!
//! # Use Cases
//!
//! - Database connections that require single-threaded access
//! - Serial processing of ordered tasks
//! - Long-running background operations with controlled lifecycle
//! - Isolation of blocking or CPU-intensive work
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_core::worker::Worker;
//!
//! // Create a worker that produces String results
//! let worker = Worker::<String>::new();
//!
//! // Connect to the result signal
//! worker.on_result().connect(|result| {
//!     println!("Worker produced: {}", result);
//! });
//!
//! // Send tasks to the worker
//! worker.send(|| {
//!     std::thread::sleep(std::time::Duration::from_millis(100));
//!     "Hello from worker!".to_string()
//! });
//!
//! // Send task with direct callback (bypasses signal)
//! worker.send_with_callback(
//!     || "computed value".to_string(),
//!     |result| println!("Got: {}", result),
//! );
//!
//! // Graceful shutdown
//! worker.stop();
//! worker.join();
//! ```
//!
//! # Bidirectional Communication
//!
//! Workers support bidirectional communication through the result signal:
//!
//! ```no_run
//! use horizon_lattice_core::worker::Worker;
//! use horizon_lattice_core::signal::ConnectionType;
//!
//! let worker = Worker::<(String, i32)>::new();
//!
//! // Connect with queued delivery to ensure UI thread execution
//! worker.on_result().connect_with_type(
//!     |(msg, code)| {
//!         println!("Status: {} (code {})", msg, code);
//!     },
//!     ConnectionType::Queued,
//! );
//!
//! worker.send(|| {
//!     // Perform work...
//!     ("Success".to_string(), 0)
//! });
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};
use parking_lot::{Condvar, Mutex};

use crate::invocation::{QueuedInvocation, invocation_registry};
use crate::progress::ProgressReporter;
use crate::signal::Signal;
use crate::threadpool::CancellationToken;

/// Default capacity for the worker's task queue.
const DEFAULT_QUEUE_CAPACITY: usize = 256;

/// Configuration for creating a Worker.
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Name for the worker thread.
    pub name: String,
    /// Stack size for the worker thread in bytes. `None` uses the default.
    pub stack_size: Option<usize>,
    /// Capacity of the task queue.
    pub queue_capacity: usize,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            name: "horizon-worker".to_string(),
            stack_size: None,
            queue_capacity: DEFAULT_QUEUE_CAPACITY,
        }
    }
}

impl WorkerConfig {
    /// Create a new configuration with the given thread name.
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

/// Builder for creating Workers with custom configuration.
#[derive(Debug, Default)]
pub struct WorkerBuilder {
    config: WorkerConfig,
}

impl WorkerBuilder {
    /// Create a new WorkerBuilder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the thread name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    /// Set the stack size for the worker thread.
    pub fn stack_size(mut self, size: usize) -> Self {
        self.config.stack_size = Some(size);
        self
    }

    /// Set the task queue capacity.
    pub fn queue_capacity(mut self, capacity: usize) -> Self {
        self.config.queue_capacity = capacity;
        self
    }

    /// Build and start the worker.
    pub fn build<T: Clone + Send + 'static>(self) -> Worker<T> {
        Worker::with_config(self.config)
    }
}

/// Internal state shared between the Worker handle and worker thread.
struct WorkerState {
    /// Whether the worker is running.
    running: AtomicBool,
    /// Cancellation token for cooperative shutdown.
    cancellation: CancellationToken,
    /// Count of pending tasks in the queue.
    pending_tasks: AtomicUsize,
    /// Condvar for waiting on shutdown.
    shutdown_condvar: Condvar,
    /// Mutex for the condvar.
    shutdown_mutex: Mutex<()>,
}

impl WorkerState {
    fn new() -> Self {
        Self {
            running: AtomicBool::new(true),
            cancellation: CancellationToken::new(),
            pending_tasks: AtomicUsize::new(0),
            shutdown_condvar: Condvar::new(),
            shutdown_mutex: Mutex::new(()),
        }
    }

    fn signal_shutdown(&self) {
        let _guard = self.shutdown_mutex.lock();
        self.shutdown_condvar.notify_all();
    }
}

/// A task sent to the worker.
enum WorkerTask<T> {
    /// Execute a task and emit the result via the signal.
    Execute(Box<dyn FnOnce() -> T + Send>),
    /// Execute a task and deliver result via callback on the UI thread.
    ExecuteWithCallback {
        task: Box<dyn FnOnce() -> T + Send>,
        callback: Box<dyn FnOnce(T) + Send>,
    },
    /// Shutdown signal.
    Shutdown,
}

/// A dedicated worker thread with its own task queue.
///
/// Worker provides a producer-consumer pattern where tasks are sent to a dedicated
/// thread for sequential processing. Results are delivered back via a signal or
/// direct callback.
///
/// # Type Parameter
///
/// - `T`: The result type produced by tasks. Must be `Send + 'static`.
///
/// # Thread Safety
///
/// `Worker<T>` is `Send + Sync` and can be safely shared between threads.
/// Multiple threads can send tasks concurrently.
pub struct Worker<T: Send + 'static> {
    /// Channel sender for submitting tasks.
    task_sender: Sender<WorkerTask<T>>,
    /// Thread handle for joining.
    handle: Mutex<Option<JoinHandle<()>>>,
    /// Shared state with the worker thread.
    state: Arc<WorkerState>,
    /// Signal emitted when a task produces a result.
    result_signal: Arc<Signal<T>>,
}

// Methods that don't require Clone on T
impl<T: Send + 'static> Worker<T> {
    /// Check if the worker is still running.
    pub fn is_running(&self) -> bool {
        self.state.running.load(Ordering::Acquire)
    }

    /// Get the number of pending tasks in the queue.
    pub fn pending_tasks(&self) -> usize {
        self.state.pending_tasks.load(Ordering::Acquire)
    }

    /// Get a reference to the result signal.
    ///
    /// Connect to this signal to receive results from completed tasks.
    /// Results are emitted on the worker thread by default. Use
    /// `ConnectionType::Queued` to receive results on the UI thread.
    pub fn on_result(&self) -> &Signal<T> {
        &self.result_signal
    }

    /// Send a task with a callback for result delivery.
    ///
    /// Unlike `send()`, the result is delivered via the callback instead of
    /// the result signal. The callback is executed on the UI thread via the
    /// event loop's invocation system.
    ///
    /// Returns `true` if the task was queued successfully.
    pub fn send_with_callback<F, C>(&self, task: F, callback: C) -> bool
    where
        F: FnOnce() -> T + Send + 'static,
        C: FnOnce(T) + Send + 'static,
    {
        if !self.is_running() {
            return false;
        }

        self.state.pending_tasks.fetch_add(1, Ordering::AcqRel);

        let worker_task = WorkerTask::ExecuteWithCallback {
            task: Box::new(task),
            callback: Box::new(callback),
        };

        match self.task_sender.try_send(worker_task) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.state.pending_tasks.fetch_sub(1, Ordering::AcqRel);
                false
            }
        }
    }

    /// Send a task and block until it completes, returning the result.
    ///
    /// This is useful when you need to execute something on the worker thread
    /// but require the result immediately.
    ///
    /// Returns `None` if the worker has been stopped.
    pub fn send_sync<F>(&self, task: F) -> Option<T>
    where
        F: FnOnce() -> T + Send + 'static,
    {
        if !self.is_running() {
            return None;
        }

        let (result_sender, result_receiver) = bounded(1);

        self.state.pending_tasks.fetch_add(1, Ordering::AcqRel);

        // Use ExecuteWithCallback to send the result through the channel
        let worker_task = WorkerTask::ExecuteWithCallback {
            task: Box::new(task),
            callback: Box::new(move |result| {
                let _ = result_sender.send(result);
            }),
        };

        match self.task_sender.try_send(worker_task) {
            Ok(()) => result_receiver.recv().ok(),
            Err(_) => {
                self.state.pending_tasks.fetch_sub(1, Ordering::AcqRel);
                None
            }
        }
    }

    /// Request the worker to stop after processing remaining tasks.
    ///
    /// This is a non-blocking call. The worker will finish processing
    /// all pending tasks before shutting down. Use `join()` to wait
    /// for completion.
    ///
    /// After calling `stop()`, no new tasks will be accepted via `send()`
    /// or `send_with_callback()`.
    pub fn stop(&self) {
        // Mark as not running immediately so new sends are rejected
        self.state.running.store(false, Ordering::Release);
        self.state.cancellation.cancel();
        // Send shutdown signal (ignore errors if already disconnected)
        let _ = self.task_sender.try_send(WorkerTask::Shutdown);
    }

    /// Wait for the worker thread to finish.
    ///
    /// This blocks until the worker thread has processed all pending tasks
    /// and exited. Call `stop()` first to initiate shutdown.
    ///
    /// Returns `true` if the worker was joined successfully, `false` if
    /// already joined or the thread panicked.
    pub fn join(&self) -> bool {
        let mut handle = self.handle.lock();
        if let Some(h) = handle.take() {
            h.join().is_ok()
        } else {
            false
        }
    }

    /// Stop the worker and wait for it to finish.
    ///
    /// This is equivalent to calling `stop()` followed by `join()`.
    pub fn stop_and_join(&self) -> bool {
        self.stop();
        self.join()
    }

    /// Wait for the worker to finish with a timeout.
    ///
    /// Returns `true` if the worker finished within the timeout, `false`
    /// if the timeout elapsed.
    pub fn wait_timeout(&self, timeout: Duration) -> bool {
        if !self.is_running() {
            return true;
        }

        let guard = self.state.shutdown_mutex.lock();
        let result = self
            .state
            .shutdown_condvar
            .wait_for(&mut { guard }, timeout);
        !result.timed_out() || !self.is_running()
    }

    /// Get the cancellation token for this worker.
    ///
    /// Tasks can check this token to cooperatively respond to shutdown requests.
    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.state.cancellation
    }
}

// Methods that require Clone on T (for signal emission)
impl<T: Clone + Send + 'static> Worker<T> {
    /// Create a new worker with default configuration.
    ///
    /// The worker thread starts immediately and begins processing tasks.
    pub fn new() -> Self {
        Self::with_config(WorkerConfig::default())
    }

    /// Create a new worker with custom configuration.
    pub fn with_config(config: WorkerConfig) -> Self {
        let (sender, receiver) = bounded(config.queue_capacity);
        let state = Arc::new(WorkerState::new());
        let result_signal = Arc::new(Signal::new());

        let thread_state = state.clone();
        let thread_signal = result_signal.clone();

        let mut builder = thread::Builder::new().name(config.name);
        if let Some(stack_size) = config.stack_size {
            builder = builder.stack_size(stack_size);
        }

        let handle = builder
            .spawn(move || {
                worker_loop(receiver, thread_state.clone(), thread_signal);
                thread_state.running.store(false, Ordering::Release);
                thread_state.signal_shutdown();
            })
            .expect("Failed to spawn worker thread");

        Self {
            task_sender: sender,
            handle: Mutex::new(Some(handle)),
            state,
            result_signal,
        }
    }

    /// Send a task to the worker for execution.
    ///
    /// The task will be queued and executed on the worker thread.
    /// When the task completes, the result is emitted via the result signal.
    ///
    /// Returns `true` if the task was queued successfully, `false` if the
    /// worker has been stopped or the queue is full.
    pub fn send<F>(&self, task: F) -> bool
    where
        F: FnOnce() -> T + Send + 'static,
    {
        if !self.is_running() {
            return false;
        }

        self.state.pending_tasks.fetch_add(1, Ordering::AcqRel);

        match self
            .task_sender
            .try_send(WorkerTask::Execute(Box::new(task)))
        {
            Ok(()) => true,
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.state.pending_tasks.fetch_sub(1, Ordering::AcqRel);
                false
            }
        }
    }

    /// Send a task with progress reporting.
    ///
    /// Similar to `send()`, but the task receives a `ProgressReporter` that can
    /// be used to report progress updates. The reporter's signals can be connected
    /// to UI elements like `ProgressBar`.
    ///
    /// Returns a `ProgressReporter` if the task was queued successfully,
    /// or `None` if the worker has been stopped or the queue is full.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use horizon_lattice_core::worker::Worker;
    /// use horizon_lattice_core::signal::ConnectionType;
    ///
    /// let worker = Worker::<String>::new();
    ///
    /// if let Some(reporter) = worker.send_with_progress(|progress| {
    ///     for i in 0..100 {
    ///         progress.update(i as f32 / 100.0, format!("Step {}", i));
    ///         std::thread::sleep(std::time::Duration::from_millis(10));
    ///     }
    ///     progress.set_progress(1.0);
    ///     "Done!".to_string()
    /// }) {
    ///     // Connect to progress updates
    ///     reporter.on_progress_changed().connect_with_type(
    ///         |&p| println!("Progress: {:.0}%", p * 100.0),
    ///         ConnectionType::Queued,
    ///     );
    /// }
    /// ```
    pub fn send_with_progress<F>(&self, task: F) -> Option<ProgressReporter>
    where
        F: FnOnce(ProgressReporter) -> T + Send + 'static,
    {
        if !self.is_running() {
            return None;
        }

        let reporter = ProgressReporter::new();
        let reporter_for_task = reporter.clone();

        self.state.pending_tasks.fetch_add(1, Ordering::AcqRel);

        match self
            .task_sender
            .try_send(WorkerTask::Execute(Box::new(move || {
                task(reporter_for_task)
            }))) {
            Ok(()) => Some(reporter),
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.state.pending_tasks.fetch_sub(1, Ordering::AcqRel);
                None
            }
        }
    }
}

impl<T: Clone + Send + 'static> Default for Worker<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Send + 'static> Drop for Worker<T> {
    fn drop(&mut self) {
        self.stop();
        // Don't block in drop - just request shutdown
    }
}

// Worker is Send + Sync when T is Send
unsafe impl<T: Send + 'static> Send for Worker<T> {}
unsafe impl<T: Send + 'static> Sync for Worker<T> {}

/// The main worker loop that processes tasks.
fn worker_loop<T: Clone + Send + 'static>(
    receiver: Receiver<WorkerTask<T>>,
    state: Arc<WorkerState>,
    result_signal: Arc<Signal<T>>,
) {
    while !state.cancellation.is_cancelled() || state.pending_tasks.load(Ordering::Acquire) > 0 {
        // Use a timeout so we can check cancellation periodically
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(WorkerTask::Execute(task)) => {
                let result = task();
                result_signal.emit(result);
                state.pending_tasks.fetch_sub(1, Ordering::AcqRel);
            }
            Ok(WorkerTask::ExecuteWithCallback { task, callback }) => {
                let result = task();
                // Deliver callback via UI thread
                post_callback_to_ui(callback, result);
                state.pending_tasks.fetch_sub(1, Ordering::AcqRel);
            }
            Ok(WorkerTask::Shutdown) => {
                // Process remaining tasks before exiting
                while let Ok(task) = receiver.try_recv() {
                    match task {
                        WorkerTask::Execute(t) => {
                            let result = t();
                            result_signal.emit(result);
                            state.pending_tasks.fetch_sub(1, Ordering::AcqRel);
                        }
                        WorkerTask::ExecuteWithCallback {
                            task: t,
                            callback: c,
                        } => {
                            let result = t();
                            post_callback_to_ui(c, result);
                            state.pending_tasks.fetch_sub(1, Ordering::AcqRel);
                        }
                        WorkerTask::Shutdown => continue,
                    }
                }
                break;
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                // Check if we should exit
                if state.cancellation.is_cancelled()
                    && state.pending_tasks.load(Ordering::Acquire) == 0
                {
                    break;
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }
}

/// Post a callback to the UI thread.
fn post_callback_to_ui<T: Send + 'static>(callback: Box<dyn FnOnce(T) + Send>, result: T) {
    let invocation = QueuedInvocation::new(move || {
        callback(result);
    });

    let invocation_id = invocation_registry().register(invocation);

    // Try to post to event loop
    if let Some(proxy) = crate::application::try_get_event_proxy() {
        let _ = proxy.send_event(crate::event::LatticeEvent::QueuedSignal { invocation_id });
    } else {
        // No event loop - execute immediately as fallback
        if let Some(inv) = invocation_registry().take(invocation_id) {
            inv.execute();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI32;
    use std::time::Duration;

    #[test]
    fn test_worker_creation() {
        let worker = Worker::<i32>::new();
        assert!(worker.is_running());
        assert_eq!(worker.pending_tasks(), 0);
        worker.stop_and_join();
    }

    #[test]
    fn test_worker_with_config() {
        let worker = WorkerBuilder::new()
            .name("test-worker")
            .queue_capacity(64)
            .build::<i32>();

        assert!(worker.is_running());
        worker.stop_and_join();
    }

    #[test]
    fn test_send_and_receive() {
        let worker = Worker::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        let received_clone = received.clone();
        worker.on_result().connect(move |&value| {
            received_clone.lock().push(value);
        });

        worker.send(|| 42);
        worker.send(|| 100);

        // Wait for processing
        thread::sleep(Duration::from_millis(100));

        let values = received.lock();
        assert!(values.contains(&42));
        assert!(values.contains(&100));

        worker.stop_and_join();
    }

    #[test]
    fn test_send_with_callback() {
        let worker = Worker::<String>::new();
        let received = Arc::new(Mutex::new(None));

        let received_clone = received.clone();
        worker.send_with_callback(
            || "hello".to_string(),
            move |result| {
                *received_clone.lock() = Some(result);
            },
        );

        // Wait for processing and callback
        thread::sleep(Duration::from_millis(100));

        assert_eq!(*received.lock(), Some("hello".to_string()));

        worker.stop_and_join();
    }

    #[test]
    fn test_graceful_shutdown() {
        let worker = Worker::<i32>::new();
        let counter = Arc::new(AtomicI32::new(0));

        // Queue several tasks
        for _ in 0..5 {
            let counter_clone = counter.clone();
            worker.send(move || {
                thread::sleep(Duration::from_millis(10));
                counter_clone.fetch_add(1, Ordering::SeqCst);
                1
            });
        }

        // Stop and wait
        worker.stop();
        worker.join();

        // All tasks should have completed
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn test_pending_tasks_count() {
        let worker = Worker::<i32>::new();

        // Send tasks with artificial delay
        for _ in 0..3 {
            worker.send(|| {
                thread::sleep(Duration::from_millis(50));
                1
            });
        }

        // Should have some pending (might have started one already)
        let pending = worker.pending_tasks();
        assert!(pending <= 3);

        // Wait for completion
        thread::sleep(Duration::from_millis(200));
        assert_eq!(worker.pending_tasks(), 0);

        worker.stop_and_join();
    }

    #[test]
    fn test_multiple_senders() {
        let worker = Arc::new(Worker::<i32>::new());
        let counter = Arc::new(AtomicI32::new(0));

        let mut handles = vec![];
        for _ in 0..5 {
            let w = worker.clone();
            let c = counter.clone();
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    let c2 = c.clone();
                    w.send(move || {
                        c2.fetch_add(1, Ordering::SeqCst);
                        1
                    });
                }
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        // Wait for processing
        thread::sleep(Duration::from_millis(200));

        assert_eq!(counter.load(Ordering::SeqCst), 50);

        worker.stop_and_join();
    }

    #[test]
    fn test_send_after_stop() {
        let worker = Worker::<i32>::new();
        worker.stop();

        // Should fail to send after stop
        let result = worker.send(|| 42);
        assert!(!result);

        worker.join();
    }

    #[test]
    fn test_wait_timeout() {
        let worker = Worker::<i32>::new();

        // Worker should not finish on its own
        assert!(!worker.wait_timeout(Duration::from_millis(50)));

        // Now stop it
        worker.stop();

        // Should finish quickly
        assert!(worker.wait_timeout(Duration::from_millis(500)));
    }

    #[test]
    fn test_cancellation_token() {
        let worker = Worker::<i32>::new();

        assert!(!worker.cancellation_token().is_cancelled());

        worker.stop();

        assert!(worker.cancellation_token().is_cancelled());

        worker.join();
    }

    #[test]
    fn test_cooperative_cancellation() {
        let worker = Worker::<String>::new();
        let iterations = Arc::new(AtomicI32::new(0));

        // Send a long-running task that checks cancellation
        let worker_token = worker.cancellation_token().clone();
        let iter_clone = iterations.clone();
        worker.send(move || {
            for i in 0..100 {
                if worker_token.is_cancelled() {
                    return format!("cancelled at {}", i);
                }
                iter_clone.fetch_add(1, Ordering::SeqCst);
                thread::sleep(Duration::from_millis(10));
            }
            "completed".to_string()
        });

        // Let it run a bit
        thread::sleep(Duration::from_millis(50));

        // Stop the worker
        worker.stop();
        worker.join();

        // Should have been cancelled before completing all iterations
        assert!(iterations.load(Ordering::SeqCst) < 100);
    }

    #[test]
    fn test_sequential_processing() {
        let worker = Worker::<i32>::new();
        let order = Arc::new(Mutex::new(Vec::new()));

        // Tasks should be processed in order
        for i in 0..10 {
            let order_clone = order.clone();
            worker.send(move || {
                order_clone.lock().push(i);
                i
            });
        }

        // Wait for processing
        thread::sleep(Duration::from_millis(100));

        let processed = order.lock();
        assert_eq!(*processed, (0..10).collect::<Vec<_>>());

        worker.stop_and_join();
    }

    #[test]
    fn test_send_sync() {
        let worker = Worker::<i32>::new();

        // Synchronous execution should block and return result
        let result = worker.send_sync(|| {
            thread::sleep(Duration::from_millis(10));
            42
        });

        assert_eq!(result, Some(42));

        worker.stop_and_join();
    }

    #[test]
    fn test_send_sync_after_stop() {
        let worker = Worker::<i32>::new();
        worker.stop();

        // Should return None after stop
        let result = worker.send_sync(|| 42);
        assert!(result.is_none());

        worker.join();
    }

    #[test]
    fn test_send_with_progress() {
        let worker = Worker::<String>::new();
        let progress_values = Arc::new(Mutex::new(Vec::new()));
        let progress_clone = progress_values.clone();

        let reporter = worker.send_with_progress(|progress| {
            for i in 0..=10 {
                progress.set_progress(i as f32 / 10.0);
                thread::sleep(Duration::from_millis(5));
            }
            "done".to_string()
        });

        assert!(reporter.is_some());
        let reporter = reporter.unwrap();

        // Connect to progress updates
        reporter.on_progress_changed().connect(move |&p| {
            progress_clone.lock().push(p);
        });

        // Wait for processing
        thread::sleep(Duration::from_millis(200));

        // Verify progress was reported
        let values = progress_values.lock();
        assert!(!values.is_empty());
        // Final progress should be 1.0
        assert!((reporter.progress() - 1.0).abs() < f32::EPSILON);

        worker.stop_and_join();
    }

    #[test]
    fn test_send_with_progress_after_stop() {
        let worker = Worker::<String>::new();
        worker.stop();

        // Should return None after stop
        let reporter = worker.send_with_progress(|progress| {
            progress.set_progress(1.0);
            "done".to_string()
        });

        assert!(reporter.is_none());

        worker.join();
    }
}
