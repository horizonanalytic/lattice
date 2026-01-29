//! Download manager with pause/resume support.
//!
//! This module provides a download manager that supports:
//! - Background downloads
//! - Progress tracking
//! - Pause and resume using HTTP Range headers
//! - Automatic retry on failure
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::http::{DownloadManager, DownloadEvent};
//!
//! let manager = DownloadManager::new();
//!
//! // Connect to events
//! manager.event.connect(|event| {
//!     match event {
//!         DownloadEvent::Progress { id, bytes_downloaded, total_bytes } => {
//!             println!("Download {:?}: {}/{:?} bytes", id, bytes_downloaded, total_bytes);
//!         }
//!         DownloadEvent::Finished { id } => {
//!             println!("Download {:?} completed!", id);
//!         }
//!         DownloadEvent::Error { id, message } => {
//!             println!("Download {:?} failed: {}", id, message);
//!         }
//!         _ => {}
//!     }
//! });
//!
//! // Start a download
//! let id = manager.download("https://example.com/file.zip", "/tmp/file.zip")?;
//!
//! // Pause if needed
//! manager.pause(id);
//!
//! // Resume later
//! manager.resume(id);
//! ```

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use horizon_lattice_core::Signal;
use parking_lot::Mutex;
use tokio::sync::oneshot;

use super::client::HttpClient;
use crate::error::{NetworkError, Result};

/// Unique identifier for a download.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DownloadId(u64);

impl DownloadId {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Current state of a download.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DownloadState {
    /// Download is queued but not started.
    Pending,
    /// Download is actively transferring data.
    Downloading,
    /// Download is paused and can be resumed.
    Paused,
    /// Download completed successfully.
    Completed,
    /// Download failed with an error.
    Failed,
    /// Download was cancelled by the user.
    Cancelled,
}

/// Events emitted by the download manager.
#[derive(Clone, Debug)]
pub enum DownloadEvent {
    /// Download started.
    Started {
        /// The download ID.
        id: DownloadId,
    },
    /// Progress update during download.
    Progress {
        /// The download ID.
        id: DownloadId,
        /// Bytes downloaded so far.
        bytes_downloaded: u64,
        /// Total bytes to download, if known.
        total_bytes: Option<u64>,
    },
    /// Download completed successfully.
    Finished {
        /// The download ID.
        id: DownloadId,
        /// Path to the downloaded file.
        path: PathBuf,
    },
    /// Download was paused.
    Paused {
        /// The download ID.
        id: DownloadId,
    },
    /// Download was resumed.
    Resumed {
        /// The download ID.
        id: DownloadId,
    },
    /// Download failed with an error.
    Error {
        /// The download ID.
        id: DownloadId,
        /// Error message.
        message: String,
    },
    /// Download was cancelled.
    Cancelled {
        /// The download ID.
        id: DownloadId,
    },
}

impl DownloadEvent {
    /// Get the download ID associated with this event.
    pub fn id(&self) -> DownloadId {
        match self {
            Self::Started { id } => *id,
            Self::Progress { id, .. } => *id,
            Self::Finished { id, .. } => *id,
            Self::Paused { id } => *id,
            Self::Resumed { id } => *id,
            Self::Error { id, .. } => *id,
            Self::Cancelled { id } => *id,
        }
    }
}

/// Configuration for retry behavior.
#[derive(Clone, Debug)]
pub struct RetryConfig {
    /// Maximum number of retry attempts.
    pub max_retries: u32,
    /// Initial delay between retries in milliseconds.
    pub initial_delay_ms: u64,
    /// Maximum delay between retries in milliseconds.
    pub max_delay_ms: u64,
    /// Multiplier for exponential backoff.
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 30000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Internal download task state.
struct DownloadTask {
    url: String,
    path: PathBuf,
    state: DownloadState,
    bytes_downloaded: u64,
    total_bytes: Option<u64>,
    supports_resume: bool,
    retry_count: u32,
    cancel_tx: Option<oneshot::Sender<()>>,
}

/// A download manager that handles multiple concurrent downloads with pause/resume support.
///
/// Downloads can be paused and resumed using HTTP Range headers when the server supports them.
/// The manager automatically detects server support for partial content and handles resumption
/// transparently.
pub struct DownloadManager {
    client: HttpClient,
    downloads: Arc<Mutex<HashMap<DownloadId, DownloadTask>>>,
    retry_config: RetryConfig,
    /// Signal emitted for download events (progress, completion, errors).
    pub event: Signal<DownloadEvent>,
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadManager {
    /// Create a new download manager with default configuration.
    pub fn new() -> Self {
        Self {
            client: HttpClient::new(),
            downloads: Arc::new(Mutex::new(HashMap::new())),
            retry_config: RetryConfig::default(),
            event: Signal::new(),
        }
    }

    /// Create a download manager with a custom HTTP client.
    pub fn with_client(client: HttpClient) -> Self {
        Self {
            client,
            downloads: Arc::new(Mutex::new(HashMap::new())),
            retry_config: RetryConfig::default(),
            event: Signal::new(),
        }
    }

    /// Set the retry configuration.
    pub fn set_retry_config(&mut self, config: RetryConfig) {
        self.retry_config = config;
    }

    /// Get the retry configuration.
    pub fn retry_config(&self) -> &RetryConfig {
        &self.retry_config
    }

    /// Start a new download.
    ///
    /// Returns a `DownloadId` that can be used to pause, resume, or cancel the download.
    pub fn download(&self, url: impl Into<String>, path: impl AsRef<Path>) -> Result<DownloadId> {
        let url = url.into();
        let path = path.as_ref().to_path_buf();
        let id = DownloadId::new();

        let (cancel_tx, cancel_rx) = oneshot::channel();

        let task = DownloadTask {
            url: url.clone(),
            path: path.clone(),
            state: DownloadState::Pending,
            bytes_downloaded: 0,
            total_bytes: None,
            supports_resume: false,
            retry_count: 0,
            cancel_tx: Some(cancel_tx),
        };

        self.downloads.lock().insert(id, task);

        // Start the download task
        self.spawn_download_task(id, url, path, 0, cancel_rx);

        Ok(id)
    }

    /// Pause a download.
    ///
    /// Returns `true` if the download was paused, `false` if it was not in a pausable state.
    pub fn pause(&self, id: DownloadId) -> bool {
        let mut downloads = self.downloads.lock();
        if let Some(task) = downloads.get_mut(&id)
            && task.state == DownloadState::Downloading {
                // Send cancel signal to stop the current download
                if let Some(tx) = task.cancel_tx.take() {
                    let _ = tx.send(());
                }
                task.state = DownloadState::Paused;
                drop(downloads);
                self.event.emit(DownloadEvent::Paused { id });
                return true;
            }
        false
    }

    /// Resume a paused download.
    ///
    /// Returns `true` if the download was resumed, `false` if it was not in a resumable state
    /// or the server doesn't support range requests.
    pub fn resume(&self, id: DownloadId) -> bool {
        let mut downloads = self.downloads.lock();
        if let Some(task) = downloads.get_mut(&id)
            && task.state == DownloadState::Paused {
                if !task.supports_resume && task.bytes_downloaded > 0 {
                    // Server doesn't support resume, need to restart
                    task.bytes_downloaded = 0;
                }

                let (cancel_tx, cancel_rx) = oneshot::channel();
                task.cancel_tx = Some(cancel_tx);
                task.state = DownloadState::Pending;

                let url = task.url.clone();
                let path = task.path.clone();
                let offset = task.bytes_downloaded;

                drop(downloads);

                self.event.emit(DownloadEvent::Resumed { id });
                self.spawn_download_task(id, url, path, offset, cancel_rx);
                return true;
            }
        false
    }

    /// Cancel a download.
    ///
    /// Returns `true` if the download was cancelled.
    pub fn cancel(&self, id: DownloadId) -> bool {
        let mut downloads = self.downloads.lock();
        if let Some(task) = downloads.get_mut(&id)
            && matches!(
                task.state,
                DownloadState::Pending | DownloadState::Downloading | DownloadState::Paused
            ) {
                // Send cancel signal
                if let Some(tx) = task.cancel_tx.take() {
                    let _ = tx.send(());
                }
                task.state = DownloadState::Cancelled;
                drop(downloads);
                self.event.emit(DownloadEvent::Cancelled { id });
                return true;
            }
        false
    }

    /// Get the current state of a download.
    pub fn state(&self, id: DownloadId) -> Option<DownloadState> {
        self.downloads.lock().get(&id).map(|t| t.state)
    }

    /// Get progress information for a download.
    pub fn progress(&self, id: DownloadId) -> Option<(u64, Option<u64>)> {
        self.downloads
            .lock()
            .get(&id)
            .map(|t| (t.bytes_downloaded, t.total_bytes))
    }

    /// Remove a completed, failed, or cancelled download from the manager.
    pub fn remove(&self, id: DownloadId) -> bool {
        let mut downloads = self.downloads.lock();
        if let Some(task) = downloads.get(&id)
            && matches!(
                task.state,
                DownloadState::Completed | DownloadState::Failed | DownloadState::Cancelled
            ) {
                downloads.remove(&id);
                return true;
            }
        false
    }

    /// Spawn the async download task.
    fn spawn_download_task(
        &self,
        id: DownloadId,
        url: String,
        path: PathBuf,
        offset: u64,
        cancel_rx: oneshot::Receiver<()>,
    ) {
        let client = self.client.clone();
        let downloads = self.downloads.clone();
        let event_ptr = &self.event as *const Signal<DownloadEvent> as usize;
        let retry_config = self.retry_config.clone();

        tokio::spawn(async move {
            // SAFETY: Signal pointer is valid as long as DownloadManager exists
            let emit_event = |event: DownloadEvent| unsafe {
                let signal = &*(event_ptr as *const Signal<DownloadEvent>);
                signal.emit(event);
            };

            // Update state to downloading
            {
                let mut downloads = downloads.lock();
                if let Some(task) = downloads.get_mut(&id) {
                    task.state = DownloadState::Downloading;
                }
            }
            emit_event(DownloadEvent::Started { id });

            // Execute download with cancellation support
            let result = tokio::select! {
                result = Self::execute_download(&client, &downloads, id, &url, &path, offset, event_ptr) => result,
                _ = cancel_rx => {
                    // Cancelled - state already updated by pause/cancel
                    return;
                }
            };

            // Handle result
            match result {
                Ok(()) => {
                    let mut downloads = downloads.lock();
                    if let Some(task) = downloads.get_mut(&id) {
                        task.state = DownloadState::Completed;
                    }
                    drop(downloads);
                    emit_event(DownloadEvent::Finished { id, path });
                }
                Err(err) => {
                    let should_retry = {
                        let mut downloads = downloads.lock();
                        if let Some(task) = downloads.get_mut(&id) {
                            task.retry_count += 1;
                            if task.retry_count <= retry_config.max_retries {
                                true
                            } else {
                                task.state = DownloadState::Failed;
                                false
                            }
                        } else {
                            false
                        }
                    };

                    if should_retry {
                        // Calculate backoff delay
                        let retry_count = downloads
                            .lock()
                            .get(&id)
                            .map(|t| t.retry_count)
                            .unwrap_or(1);
                        let delay = (retry_config.initial_delay_ms as f64
                            * retry_config.backoff_multiplier.powi(retry_count as i32 - 1))
                            as u64;
                        let delay = delay.min(retry_config.max_delay_ms);

                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;

                        // Get current offset and create new cancel channel
                        let (new_cancel_tx, new_cancel_rx) = oneshot::channel();
                        let current_offset = {
                            let mut downloads = downloads.lock();
                            if let Some(task) = downloads.get_mut(&id) {
                                task.cancel_tx = Some(new_cancel_tx);
                                task.bytes_downloaded
                            } else {
                                return;
                            }
                        };

                        // Retry using recursive spawn
                        let client = client.clone();
                        let downloads = downloads.clone();
                        let url = url.clone();
                        let path = path.clone();

                        tokio::spawn(async move {
                            // Re-create emit_event for this spawn
                            let emit_event_inner = |event: DownloadEvent| unsafe {
                                let signal = &*(event_ptr as *const Signal<DownloadEvent>);
                                signal.emit(event);
                            };

                            let result = tokio::select! {
                                result = Self::execute_download(&client, &downloads, id, &url, &path, current_offset, event_ptr) => result,
                                _ = new_cancel_rx => return,
                            };

                            match result {
                                Ok(()) => {
                                    downloads
                                        .lock()
                                        .get_mut(&id)
                                        .map(|t| t.state = DownloadState::Completed);
                                    emit_event_inner(DownloadEvent::Finished { id, path });
                                }
                                Err(err) => {
                                    downloads
                                        .lock()
                                        .get_mut(&id)
                                        .map(|t| t.state = DownloadState::Failed);
                                    emit_event_inner(DownloadEvent::Error {
                                        id,
                                        message: err.to_string(),
                                    });
                                }
                            }
                        });
                    } else {
                        emit_event(DownloadEvent::Error {
                            id,
                            message: err.to_string(),
                        });
                    }
                }
            }
        });
    }

    /// Execute the actual download.
    async fn execute_download(
        client: &HttpClient,
        downloads: &Arc<Mutex<HashMap<DownloadId, DownloadTask>>>,
        id: DownloadId,
        url: &str,
        path: &Path,
        offset: u64,
        event_ptr: usize,
    ) -> Result<()> {
        let emit_progress = |bytes_downloaded: u64, total_bytes: Option<u64>| unsafe {
            let signal = &*(event_ptr as *const Signal<DownloadEvent>);
            signal.emit(DownloadEvent::Progress {
                id,
                bytes_downloaded,
                total_bytes,
            });
        };

        // Build request with Range header if resuming
        let mut builder = client.get(url);
        if offset > 0 {
            builder = builder.header("Range", format!("bytes={}-", offset));
        }

        let response = builder.send().await?;

        // Check for partial content support
        let supports_resume = response.status() == 206;
        let is_success = response.is_success() || response.status() == 206;

        if !is_success {
            return Err(NetworkError::HttpStatus {
                status: response.status(),
                message: Some(format!("HTTP {}", response.status())),
            });
        }

        // Get total size
        let content_length = response.content_length();
        let total_bytes = if supports_resume {
            // For partial content, content_length is remaining bytes
            content_length.map(|len| len + offset)
        } else {
            content_length
        };

        // Update task with size info
        {
            let mut downloads = downloads.lock();
            if let Some(task) = downloads.get_mut(&id) {
                task.supports_resume =
                    supports_resume || response.header("Accept-Ranges") == Some("bytes");
                task.total_bytes = total_bytes;
                if !supports_resume && offset > 0 {
                    // Server doesn't support resume, reset offset
                    task.bytes_downloaded = 0;
                }
            }
        }

        // Open file for writing
        let mut file = if offset > 0 && supports_resume {
            let mut file = OpenOptions::new().write(true).open(path)?;
            file.seek(SeekFrom::Start(offset))?;
            file
        } else {
            File::create(path)?
        };

        // Download with progress updates
        let mut bytes_downloaded = if supports_resume { offset } else { 0 };
        let mut body = response.bytes_stream();

        while let Some(chunk) = body.next_chunk().await? {
            file.write_all(&chunk)?;
            bytes_downloaded += chunk.len() as u64;

            // Update task progress
            {
                let mut downloads = downloads.lock();
                if let Some(task) = downloads.get_mut(&id) {
                    task.bytes_downloaded = bytes_downloaded;
                }
            }

            // Emit progress event
            emit_progress(bytes_downloaded, total_bytes);
        }

        file.flush()?;
        Ok(())
    }
}

impl std::fmt::Debug for DownloadManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DownloadManager")
            .field("downloads", &self.downloads.lock().len())
            .finish()
    }
}
