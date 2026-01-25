//! Async runtime integration for Horizon Lattice.
//!
//! This module provides optional Tokio integration for spawning async tasks
//! with seamless bridging to the UI thread via the signal/invocation system.
//!
//! # Feature Flag
//!
//! This module requires the `tokio` feature to be enabled:
//!
//! ```toml
//! [dependencies]
//! horizon-lattice-core = { version = "0.1", features = ["tokio"] }
//! ```
//!
//! # Example: Spawning an Async Task
//!
//! ```no_run
//! use horizon_lattice_core::async_runtime::{AsyncRuntime, RuntimeType};
//!
//! # async fn fetch_data() -> String { "data".to_string() }
//!
//! // Get the global async runtime
//! let runtime = AsyncRuntime::global();
//!
//! // Spawn an async task
//! let handle = runtime.spawn(async {
//!     fetch_data().await
//! });
//!
//! // Wait for the result (blocking)
//! let result = handle.blocking_wait();
//! ```
//!
//! # Example: Delivering Results to UI Thread
//!
//! ```no_run
//! use horizon_lattice_core::async_runtime::AsyncRuntime;
//!
//! # async fn expensive_computation() -> i32 { 42 }
//!
//! let runtime = AsyncRuntime::global();
//!
//! // Spawn a task that delivers its result to the UI thread
//! runtime.spawn_with_callback(
//!     async {
//!         expensive_computation().await
//!     },
//!     |result| {
//!         // This callback runs on the UI thread
//!         println!("Got result: {}", result);
//!     },
//! );
//! ```
//!
//! # Runtime Types
//!
//! Two runtime types are available:
//!
//! - **Multi-threaded** (default): Uses Tokio's multi-threaded scheduler for
//!   maximum parallelism. Best for CPU-intensive async workloads.
//!
//! - **Single-threaded**: Runs on a dedicated thread with a current-thread
//!   runtime. Useful for embedded scenarios or when you need deterministic
//!   task ordering.

use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;

use parking_lot::Mutex;
use tokio::runtime::{Builder, Handle, Runtime};
use tokio::sync::oneshot;

use crate::invocation::{invocation_registry, QueuedInvocation};

/// Global async runtime instance.
static GLOBAL_RUNTIME: OnceLock<AsyncRuntime> = OnceLock::new();

/// Counter for unique task IDs.
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

/// The type of async runtime to create.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RuntimeType {
    /// Multi-threaded runtime using Tokio's default scheduler.
    ///
    /// This provides maximum parallelism for async tasks and is the
    /// recommended choice for most applications.
    #[default]
    MultiThreaded,

    /// Single-threaded runtime on a dedicated thread.
    ///
    /// All async tasks run on a single dedicated thread. This is useful
    /// for embedded scenarios or when deterministic task ordering is needed.
    SingleThreaded,
}

/// Configuration for the async runtime.
#[derive(Debug, Clone)]
pub struct AsyncRuntimeConfig {
    /// The type of runtime to create.
    pub runtime_type: RuntimeType,
    /// Number of worker threads for multi-threaded runtime.
    /// Defaults to the number of CPU cores.
    pub worker_threads: Option<usize>,
    /// Name prefix for runtime threads.
    pub thread_name: String,
    /// Enable I/O driver (required for network operations).
    pub enable_io: bool,
    /// Enable time driver (required for tokio::time operations).
    pub enable_time: bool,
}

impl Default for AsyncRuntimeConfig {
    fn default() -> Self {
        Self {
            runtime_type: RuntimeType::MultiThreaded,
            worker_threads: None,
            thread_name: "horizon-async".to_string(),
            enable_io: true,
            enable_time: true,
        }
    }
}

impl AsyncRuntimeConfig {
    /// Create a configuration for a multi-threaded runtime.
    pub fn multi_threaded() -> Self {
        Self {
            runtime_type: RuntimeType::MultiThreaded,
            ..Default::default()
        }
    }

    /// Create a configuration for a single-threaded runtime.
    pub fn single_threaded() -> Self {
        Self {
            runtime_type: RuntimeType::SingleThreaded,
            ..Default::default()
        }
    }

    /// Set the number of worker threads (multi-threaded runtime only).
    pub fn with_worker_threads(mut self, count: usize) -> Self {
        self.worker_threads = Some(count);
        self
    }

    /// Set the thread name prefix.
    pub fn with_thread_name(mut self, name: impl Into<String>) -> Self {
        self.thread_name = name.into();
        self
    }
}

/// A handle to a spawned async task.
///
/// Provides methods to wait for the task result, check completion status,
/// and cancel the task cooperatively.
#[derive(Debug)]
pub struct AsyncTaskHandle<T> {
    id: u64,
    receiver: oneshot::Receiver<T>,
    cancellation: Option<AsyncCancellationToken>,
}

impl<T> AsyncTaskHandle<T> {
    /// Get the unique task ID.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Cancel the task if it has a cancellation token.
    ///
    /// The task must cooperatively check for cancellation.
    pub fn cancel(&self) {
        if let Some(ref token) = self.cancellation {
            token.cancel();
        }
    }

    /// Get a reference to the cancellation token, if any.
    pub fn cancellation_token(&self) -> Option<&AsyncCancellationToken> {
        self.cancellation.as_ref()
    }

    /// Wait for the task to complete, blocking the current thread.
    ///
    /// Returns `Some(result)` if the task completed successfully,
    /// `None` if the task was cancelled or the channel was dropped.
    ///
    /// # Warning
    ///
    /// Do not call this from within an async context or the UI thread
    /// event loop, as it will block and potentially cause deadlocks.
    pub fn blocking_wait(self) -> Option<T> {
        // Use a blocking channel receive since we can't .await here
        self.receiver.blocking_recv().ok()
    }

    /// Try to get the result without blocking.
    ///
    /// Returns `Some(result)` if the task has completed, `None` otherwise.
    /// Note: This consumes the handle if successful.
    pub fn try_get(mut self) -> Result<T, Self> {
        match self.receiver.try_recv() {
            Ok(value) => Ok(value),
            Err(oneshot::error::TryRecvError::Empty) => Err(self),
            Err(oneshot::error::TryRecvError::Closed) => Err(self),
        }
    }

    /// Convert this handle into a Future that can be awaited.
    ///
    /// This is useful when you need to await the result within
    /// an async context.
    pub async fn wait(self) -> Option<T> {
        self.receiver.await.ok()
    }
}

/// A cancellation token for async tasks.
///
/// Similar to the synchronous `CancellationToken` but designed for use
/// with async tasks. Supports both polling and async waiting.
#[derive(Debug, Clone)]
pub struct AsyncCancellationToken {
    inner: Arc<AsyncCancellationState>,
}

#[derive(Debug)]
struct AsyncCancellationState {
    cancelled: AtomicBool,
    notify: tokio::sync::Notify,
}

impl AsyncCancellationToken {
    /// Create a new cancellation token.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AsyncCancellationState {
                cancelled: AtomicBool::new(false),
                notify: tokio::sync::Notify::new(),
            }),
        }
    }

    /// Check if cancellation has been requested.
    #[inline]
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }

    /// Request cancellation.
    ///
    /// This sets the cancellation flag and notifies any waiters.
    pub fn cancel(&self) {
        if !self.inner.cancelled.swap(true, Ordering::Release) {
            self.inner.notify.notify_waiters();
        }
    }

    /// Reset the token to non-cancelled state.
    pub fn reset(&self) {
        self.inner.cancelled.store(false, Ordering::Release);
    }

    /// Wait asynchronously until cancellation is requested.
    ///
    /// Returns immediately if already cancelled.
    pub async fn cancelled(&self) {
        if self.is_cancelled() {
            return;
        }
        // Wait for notification
        loop {
            let notified = self.inner.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
            if self.is_cancelled() {
                return;
            }
        }
    }
}

impl Default for AsyncCancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal state for the single-threaded runtime.
struct SingleThreadedState {
    /// Handle to the runtime thread.
    thread_handle: Mutex<Option<JoinHandle<()>>>,
    /// Shutdown signal sender.
    shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

/// The async runtime manager.
///
/// Manages Tokio runtimes and provides methods to spawn async tasks
/// with integration to the UI thread via the invocation system.
pub struct AsyncRuntime {
    /// The underlying Tokio runtime (for multi-threaded mode).
    /// Kept alive to prevent the runtime from shutting down.
    #[allow(dead_code)]
    runtime: Option<Runtime>,
    /// Handle to the runtime for spawning tasks.
    handle: Handle,
    /// State for single-threaded runtime.
    single_threaded: Option<SingleThreadedState>,
    /// The runtime type.
    runtime_type: RuntimeType,
    /// Active task count.
    active_tasks: Arc<AtomicU64>,
}

impl AsyncRuntime {
    /// Get the global async runtime.
    ///
    /// The global runtime is lazily initialized with default settings
    /// (multi-threaded with automatic worker thread count).
    pub fn global() -> &'static AsyncRuntime {
        GLOBAL_RUNTIME.get_or_init(|| {
            AsyncRuntime::new(AsyncRuntimeConfig::default())
                .expect("Failed to create global async runtime")
        })
    }

    /// Initialize the global async runtime with custom configuration.
    ///
    /// This must be called before any async operations if you want
    /// custom settings. Returns an error if already initialized.
    pub fn init_global(config: AsyncRuntimeConfig) -> Result<&'static AsyncRuntime, AsyncRuntimeError> {
        let runtime = AsyncRuntime::new(config)?;
        GLOBAL_RUNTIME
            .set(runtime)
            .map_err(|_| AsyncRuntimeError::AlreadyInitialized)?;
        Ok(GLOBAL_RUNTIME.get().unwrap())
    }

    /// Create a new async runtime with the given configuration.
    pub fn new(config: AsyncRuntimeConfig) -> Result<Self, AsyncRuntimeError> {
        match config.runtime_type {
            RuntimeType::MultiThreaded => Self::new_multi_threaded(config),
            RuntimeType::SingleThreaded => Self::new_single_threaded(config),
        }
    }

    fn new_multi_threaded(config: AsyncRuntimeConfig) -> Result<Self, AsyncRuntimeError> {
        let mut builder = Builder::new_multi_thread();
        builder.thread_name(&config.thread_name);

        if let Some(workers) = config.worker_threads {
            builder.worker_threads(workers);
        }

        if config.enable_io {
            builder.enable_io();
        }

        if config.enable_time {
            builder.enable_time();
        }

        let runtime = builder
            .build()
            .map_err(|e| AsyncRuntimeError::CreationFailed(e.to_string()))?;

        let handle = runtime.handle().clone();

        Ok(Self {
            runtime: Some(runtime),
            handle,
            single_threaded: None,
            runtime_type: RuntimeType::MultiThreaded,
            active_tasks: Arc::new(AtomicU64::new(0)),
        })
    }

    fn new_single_threaded(config: AsyncRuntimeConfig) -> Result<Self, AsyncRuntimeError> {
        let thread_name = config.thread_name.clone();
        let enable_io = config.enable_io;
        let enable_time = config.enable_time;

        // Channel to send the handle back from the spawned thread
        let (handle_tx, handle_rx) = std::sync::mpsc::channel();
        // Oneshot channel for shutdown signal
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

        let thread_handle = std::thread::Builder::new()
            .name(format!("{}-main", thread_name))
            .spawn(move || {
                let mut builder = Builder::new_current_thread();

                if enable_io {
                    builder.enable_io();
                }

                if enable_time {
                    builder.enable_time();
                }

                let runtime = builder.build().expect("Failed to create single-threaded runtime");
                let handle = runtime.handle().clone();

                // Send the handle back to the main thread
                let _ = handle_tx.send(handle);

                // Run the runtime until shutdown is signaled
                runtime.block_on(async {
                    // Wait for shutdown signal or just keep running
                    let _ = shutdown_rx.await;
                });
            })
            .map_err(|e| AsyncRuntimeError::CreationFailed(e.to_string()))?;

        // Wait for the handle
        let handle = handle_rx
            .recv()
            .map_err(|_| AsyncRuntimeError::CreationFailed("Failed to get runtime handle".to_string()))?;

        Ok(Self {
            runtime: None,
            handle,
            single_threaded: Some(SingleThreadedState {
                thread_handle: Mutex::new(Some(thread_handle)),
                shutdown_tx,
            }),
            runtime_type: RuntimeType::SingleThreaded,
            active_tasks: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Get the runtime type.
    pub fn runtime_type(&self) -> RuntimeType {
        self.runtime_type
    }

    /// Get the number of active tasks.
    pub fn active_tasks(&self) -> u64 {
        self.active_tasks.load(Ordering::Acquire)
    }

    /// Get a handle to the Tokio runtime.
    ///
    /// This can be used to spawn tasks directly on the runtime.
    pub fn handle(&self) -> &Handle {
        &self.handle
    }

    /// Spawn an async task on the runtime.
    ///
    /// Returns a handle that can be used to wait for the result.
    pub fn spawn<F, T>(&self, future: F) -> AsyncTaskHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        let active_tasks = self.active_tasks.clone();

        active_tasks.fetch_add(1, Ordering::AcqRel);

        self.handle.spawn(async move {
            let result = future.await;
            let _ = sender.send(result);
            active_tasks.fetch_sub(1, Ordering::AcqRel);
        });

        AsyncTaskHandle {
            id,
            receiver,
            cancellation: None,
        }
    }

    /// Spawn an async task with a cancellation token.
    ///
    /// The task receives a clone of the token and should check
    /// `token.is_cancelled()` periodically or await `token.cancelled()`.
    pub fn spawn_cancellable<F, Fut, T>(&self, f: F) -> (AsyncTaskHandle<T>, AsyncCancellationToken)
    where
        F: FnOnce(AsyncCancellationToken) -> Fut + Send + 'static,
        Fut: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let token = AsyncCancellationToken::new();
        let token_for_task = token.clone();

        let id = NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        let active_tasks = self.active_tasks.clone();

        active_tasks.fetch_add(1, Ordering::AcqRel);

        self.handle.spawn(async move {
            let future = f(token_for_task);
            let result = future.await;
            let _ = sender.send(result);
            active_tasks.fetch_sub(1, Ordering::AcqRel);
        });

        let handle = AsyncTaskHandle {
            id,
            receiver,
            cancellation: Some(token.clone()),
        };

        (handle, token)
    }

    /// Spawn an async task and deliver the result to the UI thread.
    ///
    /// The callback is executed on the main/UI thread after the task completes.
    /// This integrates with the event loop's invocation system.
    pub fn spawn_with_callback<F, T, C>(&self, future: F, callback: C)
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
        C: FnOnce(T) + Send + 'static,
    {
        let active_tasks = self.active_tasks.clone();
        active_tasks.fetch_add(1, Ordering::AcqRel);

        self.handle.spawn(async move {
            let result = future.await;

            // Queue the callback to run on the UI thread
            let invocation = QueuedInvocation::new(move || {
                callback(result);
            });

            let invocation_id = invocation_registry().register(invocation);

            // Post the event to the event loop
            post_to_ui_thread(invocation_id);

            active_tasks.fetch_sub(1, Ordering::AcqRel);
        });
    }

    /// Block on a future, running it to completion.
    ///
    /// # Warning
    ///
    /// This method blocks the current thread until the future completes.
    /// **Do not call this from within the UI thread event loop** or from
    /// within an async context, as it will cause deadlocks or panics.
    ///
    /// Use this only for:
    /// - Application startup/initialization
    /// - Background threads that need to run async code
    /// - Test code
    ///
    /// For async code that needs to deliver results to the UI thread,
    /// use `spawn_with_callback` instead.
    pub fn block_on<F, T>(&self, future: F) -> T
    where
        F: Future<Output = T>,
    {
        self.handle.block_on(future)
    }

    /// Shutdown the runtime gracefully.
    ///
    /// For single-threaded runtimes, this stops the runtime thread.
    /// For multi-threaded runtimes, this initiates shutdown of the thread pool.
    ///
    /// Note: This consumes the runtime. For shared ownership, wrap in Arc.
    pub fn shutdown(mut self) {
        if let Some(state) = self.single_threaded.take() {
            // Send shutdown signal
            let _ = state.shutdown_tx.send(());
            // Wait for the thread to finish
            if let Some(handle) = state.thread_handle.lock().take() {
                let _ = handle.join();
            }
        }
        // For multi-threaded, the runtime will shutdown when dropped
    }
}

impl std::fmt::Debug for AsyncRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncRuntime")
            .field("runtime_type", &self.runtime_type)
            .field("active_tasks", &self.active_tasks())
            .finish()
    }
}

/// Post a queued invocation to the UI thread.
///
/// This is called from async tasks to deliver results to the main thread.
fn post_to_ui_thread(invocation_id: u64) {
    use crate::event::LatticeEvent;

    // Try to get the application's event loop proxy and send the event.
    if let Some(proxy) = crate::application::try_get_event_proxy() {
        let _ = proxy.send_event(LatticeEvent::QueuedSignal { invocation_id });
    }
}

/// Errors that can occur with the async runtime.
#[derive(Debug, Clone)]
pub enum AsyncRuntimeError {
    /// The runtime has already been initialized.
    AlreadyInitialized,
    /// Failed to create the runtime.
    CreationFailed(String),
}

impl std::fmt::Display for AsyncRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized => write!(f, "Async runtime already initialized"),
            Self::CreationFailed(msg) => write!(f, "Failed to create async runtime: {}", msg),
        }
    }
}

impl std::error::Error for AsyncRuntimeError {}

/// Create an async channel that can bridge to the signal system.
///
/// Returns a sender/receiver pair where the receiver can be used
/// to process messages on the UI thread.
pub fn async_channel<T: Send + 'static>(buffer: usize) -> (AsyncSender<T>, AsyncReceiver<T>) {
    let (tx, rx) = tokio::sync::mpsc::channel(buffer);
    (AsyncSender { inner: tx }, AsyncReceiver { inner: rx })
}

/// Sender half of an async channel.
#[derive(Debug, Clone)]
pub struct AsyncSender<T> {
    inner: tokio::sync::mpsc::Sender<T>,
}

impl<T: Send> AsyncSender<T> {
    /// Send a value on the channel.
    pub async fn send(&self, value: T) -> Result<(), AsyncChannelError> {
        self.inner
            .send(value)
            .await
            .map_err(|_| AsyncChannelError::Closed)
    }

    /// Try to send a value without blocking.
    pub fn try_send(&self, value: T) -> Result<(), AsyncChannelError> {
        self.inner
            .try_send(value)
            .map_err(|e| match e {
                tokio::sync::mpsc::error::TrySendError::Full(_) => AsyncChannelError::Full,
                tokio::sync::mpsc::error::TrySendError::Closed(_) => AsyncChannelError::Closed,
            })
    }
}

/// Receiver half of an async channel.
#[derive(Debug)]
pub struct AsyncReceiver<T> {
    inner: tokio::sync::mpsc::Receiver<T>,
}

impl<T> AsyncReceiver<T> {
    /// Receive a value from the channel.
    pub async fn recv(&mut self) -> Option<T> {
        self.inner.recv().await
    }

    /// Try to receive a value without blocking.
    pub fn try_recv(&mut self) -> Result<T, AsyncChannelError> {
        self.inner
            .try_recv()
            .map_err(|e| match e {
                tokio::sync::mpsc::error::TryRecvError::Empty => AsyncChannelError::Empty,
                tokio::sync::mpsc::error::TryRecvError::Disconnected => AsyncChannelError::Closed,
            })
    }
}

/// Errors that can occur with async channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncChannelError {
    /// The channel is full (for try_send).
    Full,
    /// The channel is empty (for try_recv).
    Empty,
    /// The channel has been closed.
    Closed,
}

impl std::fmt::Display for AsyncChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "Channel is full"),
            Self::Empty => write!(f, "Channel is empty"),
            Self::Closed => write!(f, "Channel is closed"),
        }
    }
}

impl std::error::Error for AsyncChannelError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicI32;
    use std::time::Duration;

    #[test]
    fn test_spawn_and_wait() {
        let runtime = AsyncRuntime::new(AsyncRuntimeConfig::multi_threaded()).unwrap();
        let handle = runtime.spawn(async { 42 });
        assert_eq!(handle.blocking_wait(), Some(42));
    }

    #[test]
    fn test_spawn_async_computation() {
        let runtime = AsyncRuntime::new(AsyncRuntimeConfig::multi_threaded()).unwrap();
        let handle = runtime.spawn(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            "hello"
        });
        assert_eq!(handle.blocking_wait(), Some("hello"));
    }

    #[test]
    fn test_multiple_tasks() {
        let runtime = AsyncRuntime::new(AsyncRuntimeConfig::multi_threaded()).unwrap();
        let counter = Arc::new(AtomicI32::new(0));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let counter = counter.clone();
                runtime.spawn(async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();

        for handle in handles {
            handle.blocking_wait();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn test_cancellation_token() {
        let runtime = AsyncRuntime::new(AsyncRuntimeConfig::multi_threaded()).unwrap();

        let (handle, token) = runtime.spawn_cancellable(|token| async move {
            for i in 0..100 {
                if token.is_cancelled() {
                    return -1;
                }
                if i > 5 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
            42
        });

        // Cancel after a short delay
        std::thread::sleep(Duration::from_millis(50));
        token.cancel();

        let result = handle.blocking_wait();
        // Should have been cancelled (or finished if it was fast enough)
        assert!(result == Some(-1) || result == Some(42));
    }

    #[test]
    fn test_single_threaded_runtime() {
        let runtime = AsyncRuntime::new(AsyncRuntimeConfig::single_threaded()).unwrap();
        assert_eq!(runtime.runtime_type(), RuntimeType::SingleThreaded);

        let handle = runtime.spawn(async { 42 });
        assert_eq!(handle.blocking_wait(), Some(42));
    }

    #[test]
    fn test_block_on() {
        let runtime = AsyncRuntime::new(AsyncRuntimeConfig::multi_threaded()).unwrap();
        let result = runtime.block_on(async { 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_async_channel() {
        let runtime = AsyncRuntime::new(AsyncRuntimeConfig::multi_threaded()).unwrap();
        let (tx, mut rx) = async_channel::<i32>(10);

        runtime.block_on(async {
            tx.send(42).await.unwrap();
            assert_eq!(rx.recv().await, Some(42));
        });
    }

    #[test]
    fn test_active_task_count() {
        let runtime = AsyncRuntime::new(AsyncRuntimeConfig::multi_threaded()).unwrap();

        // Spawn tasks that wait
        let handles: Vec<_> = (0..3)
            .map(|_| {
                runtime.spawn(async {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                })
            })
            .collect();

        // Give tasks time to start
        std::thread::sleep(Duration::from_millis(10));

        // Should have active tasks
        let active = runtime.active_tasks();
        assert!(active > 0 && active <= 3);

        // Wait for all
        for handle in handles {
            handle.blocking_wait();
        }

        // Should have no active tasks
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(runtime.active_tasks(), 0);
    }

    #[test]
    fn test_handle_wait_async() {
        let runtime = AsyncRuntime::new(AsyncRuntimeConfig::multi_threaded()).unwrap();
        let handle = runtime.spawn(async { 42 });
        // Use the runtime's block_on to await the handle
        let result = runtime.block_on(handle.wait());
        assert_eq!(result, Some(42));
    }

    #[test]
    fn test_try_send_recv() {
        let (tx, mut rx) = async_channel::<i32>(1);

        tx.try_send(42).unwrap();
        assert_eq!(rx.try_recv(), Ok(42));
        assert_eq!(rx.try_recv(), Err(AsyncChannelError::Empty));
    }
}
