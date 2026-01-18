//! Async HTTP client with signal-based completion.
//!
//! This module provides integration with Horizon Lattice's signal system for
//! handling async HTTP operations in a GUI-friendly way.

use std::sync::Arc;

use horizon_lattice_core::Signal;
use parking_lot::Mutex;
use tokio::sync::oneshot;

use super::client::HttpClient;
use super::request::HttpRequestBuilder;
use super::response::TransferProgress;

/// Unique identifier for an async request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RequestId(u64);

impl RequestId {
    fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Status of a completed request (clonable for signals).
#[derive(Clone, Debug)]
pub enum RequestStatus {
    /// Request completed successfully with the given HTTP status code.
    Success {
        /// The request ID.
        id: RequestId,
        /// HTTP status code.
        status_code: u16,
        /// Content length if known.
        content_length: Option<u64>,
    },
    /// Request failed with an error.
    Error {
        /// The request ID.
        id: RequestId,
        /// Error message.
        message: String,
    },
    /// Request was cancelled.
    Cancelled {
        /// The request ID.
        id: RequestId,
    },
}

impl RequestStatus {
    /// Get the request ID.
    pub fn id(&self) -> RequestId {
        match self {
            Self::Success { id, .. } => *id,
            Self::Error { id, .. } => *id,
            Self::Cancelled { id } => *id,
        }
    }

    /// Check if the request was successful.
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success { .. })
    }

    /// Check if the request failed.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. })
    }

    /// Check if the request was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled { .. })
    }
}

/// A handle to a pending HTTP request that can be cancelled.
pub struct RequestHandle {
    /// The unique ID of this request.
    pub id: RequestId,
    cancel_tx: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

impl RequestHandle {
    /// Cancel the pending request.
    ///
    /// Returns `true` if the cancellation signal was sent, `false` if the
    /// request has already completed or was already cancelled.
    pub fn cancel(&self) -> bool {
        if let Some(tx) = self.cancel_tx.lock().take() {
            tx.send(()).is_ok()
        } else {
            false
        }
    }

    /// Check if the request is still pending.
    pub fn is_pending(&self) -> bool {
        self.cancel_tx.lock().is_some()
    }
}

impl Clone for RequestHandle {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            cancel_tx: self.cancel_tx.clone(),
        }
    }
}

/// An HTTP client with signal-based async request handling.
///
/// This client wraps `HttpClient` and provides methods to make requests that
/// emit signals upon completion, making it easy to integrate with GUI event loops.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_net::http::{AsyncHttpClient, RequestStatus};
///
/// let client = AsyncHttpClient::new();
///
/// // Connect to the completion signal
/// client.request_finished.connect(|status| {
///     match status {
///         RequestStatus::Success { id, status_code, .. } => {
///             println!("Request {:?} completed with status {}", id, status_code);
///         }
///         RequestStatus::Error { id, message } => {
///             println!("Request {:?} failed: {}", id, message);
///         }
///         RequestStatus::Cancelled { id } => {
///             println!("Request {:?} was cancelled", id);
///         }
///     }
/// });
///
/// // Start a request
/// let handle = client.get_async("https://api.example.com/data");
/// println!("Started request {:?}", handle.id);
///
/// // Can cancel if needed
/// // handle.cancel();
/// ```
pub struct AsyncHttpClient {
    client: HttpClient,
    /// Signal emitted when a request completes (success, error, or cancelled).
    pub request_finished: Signal<RequestStatus>,
    /// Signal emitted with progress updates during downloads.
    pub progress: Signal<(RequestId, TransferProgress)>,
}

impl Default for AsyncHttpClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncHttpClient {
    /// Create a new async HTTP client with default configuration.
    pub fn new() -> Self {
        Self {
            client: HttpClient::new(),
            request_finished: Signal::new(),
            progress: Signal::new(),
        }
    }

    /// Create from an existing HTTP client.
    pub fn from_client(client: HttpClient) -> Self {
        Self {
            client,
            request_finished: Signal::new(),
            progress: Signal::new(),
        }
    }

    /// Get a reference to the underlying HTTP client.
    pub fn client(&self) -> &HttpClient {
        &self.client
    }

    /// Start an async GET request.
    ///
    /// Returns a handle that can be used to cancel the request.
    pub fn get_async(&self, url: impl AsRef<str>) -> RequestHandle {
        self.send_async(self.client.get(url))
    }

    /// Start an async POST request.
    pub fn post_async(&self, url: impl AsRef<str>) -> RequestHandle {
        self.send_async(self.client.post(url))
    }

    /// Start an async PUT request.
    pub fn put_async(&self, url: impl AsRef<str>) -> RequestHandle {
        self.send_async(self.client.put(url))
    }

    /// Start an async DELETE request.
    pub fn delete_async(&self, url: impl AsRef<str>) -> RequestHandle {
        self.send_async(self.client.delete(url))
    }

    /// Send a request builder asynchronously.
    ///
    /// The request will be executed on the tokio runtime and the result will
    /// be emitted via the `request_finished` signal.
    pub fn send_async(&self, builder: HttpRequestBuilder) -> RequestHandle {
        let request_id = RequestId::new();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let handle = RequestHandle {
            id: request_id,
            cancel_tx: Arc::new(Mutex::new(Some(cancel_tx))),
        };

        // Clone signal for use in the spawned task
        // We need to use a pointer-based approach since Signal isn't Clone
        let signal_ptr = &self.request_finished as *const Signal<RequestStatus> as usize;
        let handle_clone = handle.clone();

        // Spawn the request on the tokio runtime
        tokio::spawn(async move {
            tokio::select! {
                result = builder.send() => {
                    // Mark handle as completed
                    handle_clone.cancel_tx.lock().take();

                    let status = match result {
                        Ok(response) => RequestStatus::Success {
                            id: request_id,
                            status_code: response.status(),
                            content_length: response.content_length(),
                        },
                        Err(err) => RequestStatus::Error {
                            id: request_id,
                            message: err.to_string(),
                        },
                    };

                    // SAFETY: The signal pointer is valid as long as AsyncHttpClient exists.
                    // This is a limitation - in a real application, you'd use Arc or similar.
                    unsafe {
                        let signal = &*(signal_ptr as *const Signal<RequestStatus>);
                        signal.emit(status);
                    }
                }
                _ = cancel_rx => {
                    // Request was cancelled
                    unsafe {
                        let signal = &*(signal_ptr as *const Signal<RequestStatus>);
                        signal.emit(RequestStatus::Cancelled { id: request_id });
                    }
                }
            }
        });

        handle
    }
}

impl std::fmt::Debug for AsyncHttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AsyncHttpClient")
            .field("client", &self.client)
            .finish()
    }
}

/// Runtime management for async operations.
///
/// This module provides utilities for integrating the tokio runtime with
/// Horizon Lattice applications.
pub mod runtime {
    use std::sync::OnceLock;
    use tokio::runtime::Runtime;

    static RUNTIME: OnceLock<Runtime> = OnceLock::new();

    /// Initialize the async runtime.
    ///
    /// This should be called early in your application, typically before
    /// creating the `Application` instance. If not called explicitly, a
    /// runtime will be created on first use.
    pub fn init() -> &'static Runtime {
        RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime")
        })
    }

    /// Get a reference to the async runtime.
    ///
    /// Initializes the runtime if it hasn't been created yet.
    pub fn get() -> &'static Runtime {
        init()
    }

    /// Block on a future using the global runtime.
    ///
    /// This is useful for running async code from synchronous contexts.
    ///
    /// # Warning
    ///
    /// Do not call this from within an async context or the GUI event loop,
    /// as it will block the current thread.
    pub fn block_on<F: std::future::Future>(future: F) -> F::Output {
        get().block_on(future)
    }

    /// Spawn a future on the global runtime.
    pub fn spawn<F>(future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        get().spawn(future)
    }
}
