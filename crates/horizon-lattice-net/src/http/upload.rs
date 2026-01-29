//! Upload manager with Tus protocol support for resumable uploads.
//!
//! This module provides an upload manager that supports:
//! - Progress tracking
//! - Resumable uploads via the Tus protocol
//! - Pause and resume capability
//! - Chunked uploads for large files
//!
//! # Tus Protocol
//!
//! The [Tus protocol](https://tus.io) is a standardized protocol for resumable uploads.
//! This implementation supports the core protocol and the creation extension.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_net::http::{UploadManager, UploadEvent};
//!
//! let manager = UploadManager::new();
//!
//! // Connect to events
//! manager.event.connect(|event| {
//!     match event {
//!         UploadEvent::Progress { id, bytes_uploaded, total_bytes } => {
//!             println!("Upload {:?}: {}/{} bytes", id, bytes_uploaded, total_bytes);
//!         }
//!         UploadEvent::Finished { id } => {
//!             println!("Upload {:?} completed!", id);
//!         }
//!         UploadEvent::Error { id, message } => {
//!             println!("Upload {:?} failed: {}", id, message);
//!         }
//!         _ => {}
//!     }
//! });
//!
//! // Start a Tus upload
//! let id = manager.upload_tus("/path/to/large-file.zip", "https://example.com/uploads")?;
//!
//! // Pause if needed
//! manager.pause(id);
//!
//! // Resume later
//! manager.resume(id);
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use horizon_lattice_core::Signal;
use parking_lot::Mutex;
use tokio::sync::oneshot;

use super::client::HttpClient;
use crate::error::{NetworkError, Result};

/// Tus protocol version.
const TUS_VERSION: &str = "1.0.0";

/// Default chunk size for uploads (5 MB).
const DEFAULT_CHUNK_SIZE: usize = 5 * 1024 * 1024;

/// Unique identifier for an upload.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UploadId(u64);

impl UploadId {
    fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Current state of an upload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadState {
    /// Upload is queued but not started.
    Pending,
    /// Upload is creating the resource on the server.
    Creating,
    /// Upload is actively transferring data.
    Uploading,
    /// Upload is paused and can be resumed.
    Paused,
    /// Upload completed successfully.
    Completed,
    /// Upload failed with an error.
    Failed,
    /// Upload was cancelled by the user.
    Cancelled,
}

/// Events emitted by the upload manager.
#[derive(Clone, Debug)]
pub enum UploadEvent {
    /// Upload started.
    Started {
        /// The upload ID.
        id: UploadId,
    },
    /// Progress update during upload.
    Progress {
        /// The upload ID.
        id: UploadId,
        /// Bytes uploaded so far.
        bytes_uploaded: u64,
        /// Total bytes to upload.
        total_bytes: u64,
    },
    /// Upload completed successfully.
    Finished {
        /// The upload ID.
        id: UploadId,
        /// The URL of the uploaded resource (if returned by server).
        url: Option<String>,
    },
    /// Upload was paused.
    Paused {
        /// The upload ID.
        id: UploadId,
    },
    /// Upload was resumed.
    Resumed {
        /// The upload ID.
        id: UploadId,
    },
    /// Upload failed with an error.
    Error {
        /// The upload ID.
        id: UploadId,
        /// Error message.
        message: String,
    },
    /// Upload was cancelled.
    Cancelled {
        /// The upload ID.
        id: UploadId,
    },
}

impl UploadEvent {
    /// Get the upload ID associated with this event.
    pub fn id(&self) -> UploadId {
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

/// Configuration for upload behavior.
#[derive(Clone, Debug)]
pub struct UploadConfig {
    /// Chunk size for uploading data (default: 5 MB).
    pub chunk_size: usize,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            chunk_size: DEFAULT_CHUNK_SIZE,
        }
    }
}

/// Internal upload task state.
struct UploadTask {
    file_path: PathBuf,
    endpoint_url: String,
    upload_url: Option<String>,
    state: UploadState,
    bytes_uploaded: u64,
    total_bytes: u64,
    cancel_tx: Option<oneshot::Sender<()>>,
    metadata: HashMap<String, String>,
}

/// An upload manager that handles resumable uploads using the Tus protocol.
///
/// The manager supports multiple concurrent uploads with pause/resume capability.
/// It implements the Tus v1.0.0 core protocol with the creation extension.
pub struct UploadManager {
    client: HttpClient,
    uploads: Arc<Mutex<HashMap<UploadId, UploadTask>>>,
    config: UploadConfig,
    /// Signal emitted for upload events (progress, completion, errors).
    pub event: Signal<UploadEvent>,
}

impl Default for UploadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UploadManager {
    /// Create a new upload manager with default configuration.
    pub fn new() -> Self {
        Self {
            client: HttpClient::new(),
            uploads: Arc::new(Mutex::new(HashMap::new())),
            config: UploadConfig::default(),
            event: Signal::new(),
        }
    }

    /// Create an upload manager with a custom HTTP client.
    pub fn with_client(client: HttpClient) -> Self {
        Self {
            client,
            uploads: Arc::new(Mutex::new(HashMap::new())),
            config: UploadConfig::default(),
            event: Signal::new(),
        }
    }

    /// Set the upload configuration.
    pub fn set_config(&mut self, config: UploadConfig) {
        self.config = config;
    }

    /// Get the upload configuration.
    pub fn config(&self) -> &UploadConfig {
        &self.config
    }

    /// Start a Tus resumable upload.
    ///
    /// The endpoint URL should be the Tus server creation endpoint.
    /// Returns an `UploadId` that can be used to pause, resume, or cancel the upload.
    pub fn upload_tus(
        &self,
        file_path: impl AsRef<Path>,
        endpoint_url: impl Into<String>,
    ) -> Result<UploadId> {
        self.upload_tus_with_metadata(file_path, endpoint_url, HashMap::new())
    }

    /// Start a Tus resumable upload with metadata.
    ///
    /// Metadata is sent to the server during upload creation. Common keys include:
    /// - `filename`: The original filename
    /// - `filetype`: MIME type of the file
    pub fn upload_tus_with_metadata(
        &self,
        file_path: impl AsRef<Path>,
        endpoint_url: impl Into<String>,
        metadata: HashMap<String, String>,
    ) -> Result<UploadId> {
        let file_path = file_path.as_ref().to_path_buf();
        let endpoint_url = endpoint_url.into();

        // Get file size
        let file = File::open(&file_path)?;
        let total_bytes = file.metadata()?.len();

        let id = UploadId::new();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        // Add filename to metadata if not present
        let mut metadata = metadata;
        if !metadata.contains_key("filename")
            && let Some(name) = file_path.file_name() {
                metadata.insert("filename".to_string(), name.to_string_lossy().to_string());
            }

        let task = UploadTask {
            file_path: file_path.clone(),
            endpoint_url: endpoint_url.clone(),
            upload_url: None,
            state: UploadState::Pending,
            bytes_uploaded: 0,
            total_bytes,
            cancel_tx: Some(cancel_tx),
            metadata,
        };

        self.uploads.lock().insert(id, task);

        // Start the upload task
        self.spawn_upload_task(id, cancel_rx);

        Ok(id)
    }

    /// Resume an upload using an existing Tus upload URL.
    ///
    /// This is useful when you have a previously created upload URL that was
    /// interrupted. The manager will query the server for the current offset
    /// and resume from there.
    pub fn resume_tus_url(
        &self,
        file_path: impl AsRef<Path>,
        upload_url: impl Into<String>,
    ) -> Result<UploadId> {
        let file_path = file_path.as_ref().to_path_buf();
        let upload_url = upload_url.into();

        // Get file size
        let file = File::open(&file_path)?;
        let total_bytes = file.metadata()?.len();

        let id = UploadId::new();
        let (cancel_tx, cancel_rx) = oneshot::channel();

        let task = UploadTask {
            file_path: file_path.clone(),
            endpoint_url: String::new(), // Not needed for resume
            upload_url: Some(upload_url),
            state: UploadState::Pending,
            bytes_uploaded: 0, // Will be updated from HEAD request
            total_bytes,
            cancel_tx: Some(cancel_tx),
            metadata: HashMap::new(),
        };

        self.uploads.lock().insert(id, task);

        // Start the upload task
        self.spawn_upload_task(id, cancel_rx);

        Ok(id)
    }

    /// Pause an upload.
    ///
    /// Returns `true` if the upload was paused, `false` if it was not in a pausable state.
    pub fn pause(&self, id: UploadId) -> bool {
        let mut uploads = self.uploads.lock();
        if let Some(task) = uploads.get_mut(&id)
            && matches!(task.state, UploadState::Creating | UploadState::Uploading) {
                // Send cancel signal to stop the current upload
                if let Some(tx) = task.cancel_tx.take() {
                    let _ = tx.send(());
                }
                task.state = UploadState::Paused;
                drop(uploads);
                self.event.emit(UploadEvent::Paused { id });
                return true;
            }
        false
    }

    /// Resume a paused upload.
    ///
    /// Returns `true` if the upload was resumed, `false` if it was not in a resumable state.
    pub fn resume(&self, id: UploadId) -> bool {
        let mut uploads = self.uploads.lock();
        if let Some(task) = uploads.get_mut(&id)
            && task.state == UploadState::Paused {
                let (cancel_tx, cancel_rx) = oneshot::channel();
                task.cancel_tx = Some(cancel_tx);
                task.state = UploadState::Pending;

                drop(uploads);

                self.event.emit(UploadEvent::Resumed { id });
                self.spawn_upload_task(id, cancel_rx);
                return true;
            }
        false
    }

    /// Cancel an upload.
    ///
    /// Returns `true` if the upload was cancelled.
    pub fn cancel(&self, id: UploadId) -> bool {
        let mut uploads = self.uploads.lock();
        if let Some(task) = uploads.get_mut(&id)
            && matches!(
                task.state,
                UploadState::Pending
                    | UploadState::Creating
                    | UploadState::Uploading
                    | UploadState::Paused
            ) {
                // Send cancel signal
                if let Some(tx) = task.cancel_tx.take() {
                    let _ = tx.send(());
                }
                task.state = UploadState::Cancelled;
                drop(uploads);
                self.event.emit(UploadEvent::Cancelled { id });
                return true;
            }
        false
    }

    /// Get the current state of an upload.
    pub fn state(&self, id: UploadId) -> Option<UploadState> {
        self.uploads.lock().get(&id).map(|t| t.state)
    }

    /// Get progress information for an upload.
    pub fn progress(&self, id: UploadId) -> Option<(u64, u64)> {
        self.uploads
            .lock()
            .get(&id)
            .map(|t| (t.bytes_uploaded, t.total_bytes))
    }

    /// Get the upload URL for a Tus upload.
    ///
    /// Returns `None` if the upload hasn't been created yet or if it's not a Tus upload.
    pub fn upload_url(&self, id: UploadId) -> Option<String> {
        self.uploads
            .lock()
            .get(&id)
            .and_then(|t| t.upload_url.clone())
    }

    /// Remove a completed, failed, or cancelled upload from the manager.
    pub fn remove(&self, id: UploadId) -> bool {
        let mut uploads = self.uploads.lock();
        if let Some(task) = uploads.get(&id)
            && matches!(
                task.state,
                UploadState::Completed | UploadState::Failed | UploadState::Cancelled
            ) {
                uploads.remove(&id);
                return true;
            }
        false
    }

    /// Spawn the async upload task.
    fn spawn_upload_task(&self, id: UploadId, cancel_rx: oneshot::Receiver<()>) {
        let client = self.client.clone();
        let uploads = self.uploads.clone();
        let event_ptr = &self.event as *const Signal<UploadEvent> as usize;
        let config = self.config.clone();

        tokio::spawn(async move {
            // SAFETY: Signal pointer is valid as long as UploadManager exists
            let emit_event = |event: UploadEvent| unsafe {
                let signal = &*(event_ptr as *const Signal<UploadEvent>);
                signal.emit(event);
            };

            emit_event(UploadEvent::Started { id });

            // Execute upload with cancellation support
            let result = tokio::select! {
                result = Self::execute_upload(&client, &uploads, id, &config, event_ptr) => result,
                _ = cancel_rx => {
                    // Cancelled - state already updated by pause/cancel
                    return;
                }
            };

            // Handle result
            match result {
                Ok(url) => {
                    let mut uploads = uploads.lock();
                    if let Some(task) = uploads.get_mut(&id) {
                        task.state = UploadState::Completed;
                    }
                    drop(uploads);
                    emit_event(UploadEvent::Finished { id, url });
                }
                Err(err) => {
                    let mut uploads = uploads.lock();
                    if let Some(task) = uploads.get_mut(&id) {
                        task.state = UploadState::Failed;
                    }
                    drop(uploads);
                    emit_event(UploadEvent::Error {
                        id,
                        message: err.to_string(),
                    });
                }
            }
        });
    }

    /// Execute the Tus upload.
    async fn execute_upload(
        client: &HttpClient,
        uploads: &Arc<Mutex<HashMap<UploadId, UploadTask>>>,
        id: UploadId,
        config: &UploadConfig,
        event_ptr: usize,
    ) -> Result<Option<String>> {
        let emit_progress = |bytes_uploaded: u64, total_bytes: u64| unsafe {
            let signal = &*(event_ptr as *const Signal<UploadEvent>);
            signal.emit(UploadEvent::Progress {
                id,
                bytes_uploaded,
                total_bytes,
            });
        };

        // Get task info
        let (file_path, endpoint_url, upload_url, total_bytes, metadata) = {
            let uploads = uploads.lock();
            let task = uploads
                .get(&id)
                .ok_or_else(|| NetworkError::InvalidBody("Upload task not found".to_string()))?;
            (
                task.file_path.clone(),
                task.endpoint_url.clone(),
                task.upload_url.clone(),
                task.total_bytes,
                task.metadata.clone(),
            )
        };

        // Step 1: Create upload resource if we don't have an upload URL
        let upload_url = if let Some(url) = upload_url {
            // Update state
            uploads
                .lock()
                .get_mut(&id)
                .map(|t| t.state = UploadState::Uploading);
            url
        } else {
            // Update state to creating
            uploads
                .lock()
                .get_mut(&id)
                .map(|t| t.state = UploadState::Creating);

            let url =
                Self::create_tus_upload(client, &endpoint_url, total_bytes, &metadata).await?;

            // Store upload URL
            uploads.lock().get_mut(&id).map(|t| {
                t.upload_url = Some(url.clone());
                t.state = UploadState::Uploading;
            });

            url
        };

        // Step 2: Get current offset from server
        let current_offset = Self::get_tus_offset(client, &upload_url).await?;

        // Update bytes_uploaded
        uploads
            .lock()
            .get_mut(&id)
            .map(|t| t.bytes_uploaded = current_offset);

        if current_offset > 0 {
            emit_progress(current_offset, total_bytes);
        }

        if current_offset >= total_bytes {
            // Already complete
            return Ok(Some(upload_url));
        }

        // Step 3: Upload data in chunks
        let mut file = File::open(&file_path)?;
        file.seek(SeekFrom::Start(current_offset))?;

        let mut offset = current_offset;
        let mut buffer = vec![0u8; config.chunk_size];

        while offset < total_bytes {
            let remaining = (total_bytes - offset) as usize;
            let chunk_size = remaining.min(config.chunk_size);
            let chunk_buffer = &mut buffer[..chunk_size];

            file.read_exact(chunk_buffer)?;

            // Upload chunk
            let new_offset =
                Self::upload_tus_chunk(client, &upload_url, offset, chunk_buffer).await?;

            offset = new_offset;

            // Update progress
            uploads
                .lock()
                .get_mut(&id)
                .map(|t| t.bytes_uploaded = offset);
            emit_progress(offset, total_bytes);
        }

        Ok(Some(upload_url))
    }

    /// Create a new Tus upload resource.
    async fn create_tus_upload(
        client: &HttpClient,
        endpoint_url: &str,
        file_size: u64,
        metadata: &HashMap<String, String>,
    ) -> Result<String> {
        let mut builder = client
            .post(endpoint_url)
            .header("Tus-Resumable", TUS_VERSION)
            .header("Upload-Length", file_size.to_string());

        // Encode metadata as base64 key-value pairs
        if !metadata.is_empty() {
            let encoded: Vec<String> = metadata
                .iter()
                .map(|(k, v)| {
                    use base64::Engine;
                    let encoded_value =
                        base64::engine::general_purpose::STANDARD.encode(v.as_bytes());
                    format!("{} {}", k, encoded_value)
                })
                .collect();
            builder = builder.header("Upload-Metadata", encoded.join(","));
        }

        let response = builder.send().await?;

        if response.status() != 201 {
            return Err(NetworkError::HttpStatus {
                status: response.status(),
                message: Some("Failed to create Tus upload".to_string()),
            });
        }

        // Get upload URL from Location header
        response
            .header("Location")
            .map(|s| s.to_string())
            .ok_or_else(|| {
                NetworkError::InvalidBody("Missing Location header in Tus response".to_string())
            })
    }

    /// Get the current upload offset from the server.
    async fn get_tus_offset(client: &HttpClient, upload_url: &str) -> Result<u64> {
        let response = client
            .head(upload_url)
            .header("Tus-Resumable", TUS_VERSION)
            .send()
            .await?;

        if !response.is_success() {
            return Err(NetworkError::HttpStatus {
                status: response.status(),
                message: Some("Failed to get Tus upload offset".to_string()),
            });
        }

        response
            .header("Upload-Offset")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| NetworkError::InvalidBody("Missing Upload-Offset header".to_string()))
    }

    /// Upload a chunk of data.
    async fn upload_tus_chunk(
        client: &HttpClient,
        upload_url: &str,
        offset: u64,
        data: &[u8],
    ) -> Result<u64> {
        let response = client
            .patch(upload_url)
            .header("Tus-Resumable", TUS_VERSION)
            .header("Upload-Offset", offset.to_string())
            .header("Content-Type", "application/offset+octet-stream")
            .bytes(data.to_vec())
            .send()
            .await?;

        if response.status() != 204 {
            return Err(NetworkError::HttpStatus {
                status: response.status(),
                message: Some("Failed to upload Tus chunk".to_string()),
            });
        }

        // Get new offset from response
        response
            .header("Upload-Offset")
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                NetworkError::InvalidBody(
                    "Missing Upload-Offset header in chunk response".to_string(),
                )
            })
    }
}

impl std::fmt::Debug for UploadManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UploadManager")
            .field("uploads", &self.uploads.lock().len())
            .finish()
    }
}
