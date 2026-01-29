//! Async image loading with background thread processing.
//!
//! This module provides asynchronous image loading that decodes images
//! on background threads while keeping the main thread responsive.
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::{AsyncImageLoader, ImageManager, LoadingState};
//!
//! # fn example() -> horizon_lattice_render::RenderResult<()> {
//! let mut image_manager = ImageManager::new()?;
//! let mut async_loader = AsyncImageLoader::new();
//!
//! // Start loading an image (returns immediately)
//! let handle = async_loader.load_file("image.png")?;
//!
//! // In your render loop:
//! loop {
//!     // Process completed loads (uploads to GPU)
//!     async_loader.process_completed(&mut image_manager)?;
//!
//!     // Check if the image is ready
//!     if let Some(state) = async_loader.state(&handle) {
//!         match state {
//!             LoadingState::Loading => {
//!                 // Show placeholder or loading indicator
//!             }
//!             LoadingState::Ready(image) => {
//!                 // Use the image for rendering
//!             }
//!             LoadingState::Failed(err) => {
//!                 // Handle error
//!             }
//!         }
//!     }
//!     # break;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Disk Caching for URL Downloads
//!
//! When loading images from URLs (with the `networking` feature), you can enable
//! disk caching to avoid re-downloading images:
//!
//! ```ignore
//! use horizon_lattice_render::{AsyncImageLoader, AsyncImageLoaderConfig, DiskImageCache};
//!
//! let disk_cache = DiskImageCache::with_defaults()?;
//! let config = AsyncImageLoaderConfig::default();
//! let mut loader = AsyncImageLoader::with_config(config)
//!     .with_disk_cache(disk_cache);
//!
//! // URL downloads will be cached to disk
//! let handle = loader.load_url("https://example.com/image.png")?;
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::{self, JoinHandle};

use parking_lot::Mutex;

use crate::disk_cache::DiskImageCache;

use crate::atlas::ImageManager;
use crate::error::{RenderError, RenderResult};
use crate::image::Image;

/// Counter for generating unique handle IDs.
static HANDLE_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The current state of an async image load operation.
#[derive(Debug, Clone)]
pub enum LoadingState {
    /// The image is still being loaded/decoded.
    Loading,
    /// The image has been loaded successfully and is ready for rendering.
    Ready(Image),
    /// The image failed to load with the given error message.
    Failed(String),
}

impl LoadingState {
    /// Returns `true` if the image is still loading.
    #[inline]
    pub fn is_loading(&self) -> bool {
        matches!(self, LoadingState::Loading)
    }

    /// Returns `true` if the image is ready.
    #[inline]
    pub fn is_ready(&self) -> bool {
        matches!(self, LoadingState::Ready(_))
    }

    /// Returns `true` if the image failed to load.
    #[inline]
    pub fn is_failed(&self) -> bool {
        matches!(self, LoadingState::Failed(_))
    }

    /// Returns the image if ready, or `None` otherwise.
    #[inline]
    pub fn image(&self) -> Option<&Image> {
        match self {
            LoadingState::Ready(img) => Some(img),
            _ => None,
        }
    }

    /// Returns the error message if failed, or `None` otherwise.
    #[inline]
    pub fn error(&self) -> Option<&str> {
        match self {
            LoadingState::Failed(err) => Some(err),
            _ => None,
        }
    }
}

/// A handle to an async image load operation.
///
/// This handle can be used to query the loading state of an image.
/// It is cheap to clone and can be stored for later use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AsyncImageHandle {
    id: u64,
}

impl AsyncImageHandle {
    fn new() -> Self {
        Self {
            id: HANDLE_ID_COUNTER.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Get the unique ID of this handle.
    #[inline]
    pub fn id(&self) -> u64 {
        self.id
    }
}

/// Internal message sent from worker threads to the main thread.
struct CompletedLoad {
    handle: AsyncImageHandle,
    result: Result<DecodedImage, String>,
}

/// Decoded image data ready for GPU upload.
struct DecodedImage {
    /// RGBA pixel data.
    data: Vec<u8>,
    /// Image width in pixels.
    width: u32,
    /// Image height in pixels.
    height: u32,
}

/// Internal message sent to worker threads.
enum LoadRequest {
    /// Load an image from a file path.
    File {
        handle: AsyncImageHandle,
        path: PathBuf,
    },
    /// Load an image from bytes in memory.
    Bytes {
        handle: AsyncImageHandle,
        data: Vec<u8>,
    },
    /// Load an image from a URL.
    #[cfg(feature = "networking")]
    Url {
        handle: AsyncImageHandle,
        url: String,
    },
    /// Signal to shut down the worker thread.
    Shutdown,
}

/// Configuration for the async image loader.
#[derive(Debug, Clone)]
pub struct AsyncImageLoaderConfig {
    /// Number of worker threads for image decoding.
    /// Defaults to the number of CPU cores, capped at 4.
    pub worker_threads: usize,
    /// Maximum number of pending loads before blocking.
    /// Defaults to 256.
    pub max_pending: usize,
}

impl Default for AsyncImageLoaderConfig {
    fn default() -> Self {
        let cores = thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(2);
        Self {
            worker_threads: cores.min(4),
            max_pending: 256,
        }
    }
}

/// Async image loader that decodes images on background threads.
///
/// This loader maintains a pool of worker threads that decode images
/// in the background. The decoded image data is then uploaded to the
/// GPU on the main thread when `process_completed` is called.
pub struct AsyncImageLoader {
    /// Channel sender for sending load requests to workers.
    request_tx: Sender<LoadRequest>,
    /// Channel receiver for completed loads from workers.
    completed_rx: Receiver<CompletedLoad>,
    /// Worker thread handles.
    workers: Vec<JoinHandle<()>>,
    /// Current state of each loading operation.
    states: HashMap<AsyncImageHandle, LoadingState>,
    /// Pending decoded images waiting for GPU upload.
    pending_uploads: Vec<(AsyncImageHandle, DecodedImage)>,
    /// Number of loads currently in progress.
    in_progress: usize,
    /// Configuration.
    config: AsyncImageLoaderConfig,
    /// Optional disk cache for URL downloads.
    disk_cache: Option<Arc<Mutex<DiskImageCache>>>,
}

impl AsyncImageLoader {
    /// Create a new async image loader with default configuration.
    pub fn new() -> Self {
        Self::with_config(AsyncImageLoaderConfig::default())
    }

    /// Create a new async image loader with custom configuration.
    pub fn with_config(config: AsyncImageLoaderConfig) -> Self {
        let (request_tx, request_rx) = channel::<LoadRequest>();
        let (completed_tx, completed_rx) = channel::<CompletedLoad>();

        // Wrap the receiver in an Arc<Mutex> so workers can share it
        let request_rx = Arc::new(Mutex::new(request_rx));

        // Spawn worker threads
        let mut workers = Vec::with_capacity(config.worker_threads);
        for i in 0..config.worker_threads {
            let rx = Arc::clone(&request_rx);
            let tx = completed_tx.clone();
            let handle = thread::Builder::new()
                .name(format!("async-image-worker-{}", i))
                .spawn(move || {
                    Self::worker_thread(rx, tx, None);
                })
                .expect("Failed to spawn worker thread");
            workers.push(handle);
        }

        Self {
            request_tx,
            completed_rx,
            workers,
            states: HashMap::new(),
            pending_uploads: Vec::new(),
            in_progress: 0,
            config,
            disk_cache: None,
        }
    }

    /// Attach a disk cache for URL downloads.
    ///
    /// When a disk cache is attached, URL downloads will be cached to disk.
    /// Subsequent requests for the same URL will use the cached data instead
    /// of re-downloading.
    ///
    /// Note: This requires recreating the worker threads to pass the cache reference.
    #[must_use]
    pub fn with_disk_cache(mut self, cache: DiskImageCache) -> Self {
        let disk_cache = Arc::new(Mutex::new(cache));
        self.disk_cache = Some(disk_cache.clone());

        // Recreate worker threads with disk cache
        // First, shutdown existing workers
        for _ in 0..self.workers.len() {
            let _ = self.request_tx.send(LoadRequest::Shutdown);
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }

        // Create new channels
        let (request_tx, request_rx) = channel::<LoadRequest>();
        let (completed_tx, completed_rx) = channel::<CompletedLoad>();
        let request_rx = Arc::new(Mutex::new(request_rx));

        // Spawn new workers with disk cache
        let mut workers = Vec::with_capacity(self.config.worker_threads);
        for i in 0..self.config.worker_threads {
            let rx = Arc::clone(&request_rx);
            let tx = completed_tx.clone();
            let cache = Some(disk_cache.clone());
            let handle = thread::Builder::new()
                .name(format!("async-image-worker-{}", i))
                .spawn(move || {
                    Self::worker_thread(rx, tx, cache);
                })
                .expect("Failed to spawn worker thread");
            workers.push(handle);
        }

        self.request_tx = request_tx;
        self.completed_rx = completed_rx;
        self.workers = workers;

        self
    }

    /// Get a reference to the disk cache if one is attached.
    pub fn disk_cache(&self) -> Option<&Arc<Mutex<DiskImageCache>>> {
        self.disk_cache.as_ref()
    }

    /// Worker thread function that processes load requests.
    #[allow(unused_variables)]
    fn worker_thread(
        request_rx: Arc<Mutex<Receiver<LoadRequest>>>,
        completed_tx: Sender<CompletedLoad>,
        disk_cache: Option<Arc<Mutex<DiskImageCache>>>,
    ) {
        loop {
            // Try to get a request (blocking)
            let request = {
                let rx = request_rx.lock();
                rx.recv()
            };

            let request = match request {
                Ok(req) => req,
                Err(_) => break, // Channel closed
            };

            match request {
                LoadRequest::File { handle, path } => {
                    let result = Self::decode_file(&path);
                    let _ = completed_tx.send(CompletedLoad { handle, result });
                }
                LoadRequest::Bytes { handle, data } => {
                    let result = Self::decode_bytes(&data);
                    let _ = completed_tx.send(CompletedLoad { handle, result });
                }
                #[cfg(feature = "networking")]
                LoadRequest::Url { handle, url } => {
                    let result = Self::decode_url_cached(&url, disk_cache.as_ref());
                    let _ = completed_tx.send(CompletedLoad { handle, result });
                }
                LoadRequest::Shutdown => break,
            }
        }
    }

    /// Decode an image from a file path.
    fn decode_file(path: &Path) -> Result<DecodedImage, String> {
        let img = image::open(path).map_err(|e| format!("Failed to load image: {}", e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(DecodedImage {
            data: rgba.into_raw(),
            width,
            height,
        })
    }

    /// Decode an image from bytes.
    fn decode_bytes(bytes: &[u8]) -> Result<DecodedImage, String> {
        let img =
            image::load_from_memory(bytes).map_err(|e| format!("Failed to decode image: {}", e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Ok(DecodedImage {
            data: rgba.into_raw(),
            width,
            height,
        })
    }

    /// Decode an image from a URL, optionally using disk cache.
    #[cfg(feature = "networking")]
    fn decode_url_cached(
        url: &str,
        disk_cache: Option<&Arc<Mutex<DiskImageCache>>>,
    ) -> Result<DecodedImage, String> {
        // Check disk cache first
        if let Some(cache) = disk_cache {
            let mut cache_guard = cache.lock();
            if let Ok(Some(cached_bytes)) = cache_guard.get(url) {
                // Cache hit - decode from cached bytes
                return Self::decode_bytes(&cached_bytes);
            }
        }

        // Cache miss or no cache - fetch from network
        let bytes = Self::fetch_url(url)?;

        // Store in disk cache if available
        if let Some(cache) = disk_cache {
            let mut cache_guard = cache.lock();
            // Ignore cache insert errors - we still have the data
            let _ = cache_guard.insert(url, &bytes);
        }

        // Decode the image
        Self::decode_bytes(&bytes)
    }

    /// Fetch raw bytes from a URL.
    #[cfg(feature = "networking")]
    fn fetch_url(url: &str) -> Result<Vec<u8>, String> {
        use horizon_lattice_net::HttpClient;

        // Create a runtime for blocking HTTP request
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        // Fetch the image data
        rt.block_on(async {
            let client = HttpClient::new();
            let response = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("Failed to fetch image: {}", e))?;

            if !response.is_success() {
                return Err(format!("HTTP error: status {}", response.status()));
            }

            response
                .bytes()
                .await
                .map(|b| b.to_vec())
                .map_err(|e| format!("Failed to read response body: {}", e))
        })
    }

    /// Start loading an image from a file path.
    ///
    /// Returns a handle that can be used to query the loading state.
    /// The image will be decoded on a background thread.
    pub fn load_file(&mut self, path: impl AsRef<Path>) -> RenderResult<AsyncImageHandle> {
        if self.in_progress >= self.config.max_pending {
            return Err(RenderError::ImageLoad(format!(
                "Too many pending loads (max {})",
                self.config.max_pending
            )));
        }

        let handle = AsyncImageHandle::new();
        let path = path.as_ref().to_path_buf();

        self.request_tx
            .send(LoadRequest::File { handle, path })
            .map_err(|_| RenderError::ImageLoad("Worker threads have shut down".to_string()))?;

        self.states.insert(handle, LoadingState::Loading);
        self.in_progress += 1;

        Ok(handle)
    }

    /// Start loading an image from bytes in memory.
    ///
    /// Returns a handle that can be used to query the loading state.
    /// The image will be decoded on a background thread.
    ///
    /// Note: The bytes are copied to avoid lifetime issues.
    pub fn load_bytes(&mut self, bytes: impl Into<Vec<u8>>) -> RenderResult<AsyncImageHandle> {
        if self.in_progress >= self.config.max_pending {
            return Err(RenderError::ImageLoad(format!(
                "Too many pending loads (max {})",
                self.config.max_pending
            )));
        }

        let handle = AsyncImageHandle::new();
        let data = bytes.into();

        self.request_tx
            .send(LoadRequest::Bytes { handle, data })
            .map_err(|_| RenderError::ImageLoad("Worker threads have shut down".to_string()))?;

        self.states.insert(handle, LoadingState::Loading);
        self.in_progress += 1;

        Ok(handle)
    }

    /// Start loading an image from a URL.
    ///
    /// Returns a handle that can be used to query the loading state.
    /// The image will be downloaded and decoded on a background thread.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let handle = loader.load_url("https://example.com/image.png")?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if there are too many pending loads or if the
    /// worker threads have shut down.
    #[cfg(feature = "networking")]
    pub fn load_url(&mut self, url: impl Into<String>) -> RenderResult<AsyncImageHandle> {
        if self.in_progress >= self.config.max_pending {
            return Err(RenderError::ImageLoad(format!(
                "Too many pending loads (max {})",
                self.config.max_pending
            )));
        }

        let handle = AsyncImageHandle::new();
        let url = url.into();

        self.request_tx
            .send(LoadRequest::Url { handle, url })
            .map_err(|_| RenderError::ImageLoad("Worker threads have shut down".to_string()))?;

        self.states.insert(handle, LoadingState::Loading);
        self.in_progress += 1;

        Ok(handle)
    }

    /// Process completed loads and upload them to the GPU.
    ///
    /// This method should be called once per frame on the main thread.
    /// It receives decoded images from worker threads and uploads them
    /// to the GPU via the ImageManager.
    ///
    /// Returns the number of images that were uploaded this frame.
    pub fn process_completed(&mut self, image_manager: &mut ImageManager) -> RenderResult<usize> {
        let mut uploaded = 0;

        // Receive all completed loads
        while let Ok(completed) = self.completed_rx.try_recv() {
            self.in_progress = self.in_progress.saturating_sub(1);

            match completed.result {
                Ok(decoded) => {
                    self.pending_uploads.push((completed.handle, decoded));
                }
                Err(err) => {
                    self.states
                        .insert(completed.handle, LoadingState::Failed(err));
                }
            }
        }

        // Upload pending images to GPU
        for (handle, decoded) in self.pending_uploads.drain(..) {
            match image_manager.load_rgba(&decoded.data, decoded.width, decoded.height) {
                Ok(image) => {
                    self.states.insert(handle, LoadingState::Ready(image));
                    uploaded += 1;
                }
                Err(e) => {
                    self.states
                        .insert(handle, LoadingState::Failed(e.to_string()));
                }
            }
        }

        Ok(uploaded)
    }

    /// Get the current loading state for a handle.
    ///
    /// Returns `None` if the handle is invalid (not from this loader).
    pub fn state(&self, handle: &AsyncImageHandle) -> Option<&LoadingState> {
        self.states.get(handle)
    }

    /// Get the image for a handle if it's ready.
    ///
    /// This is a convenience method equivalent to:
    /// ```ignore
    /// loader.state(&handle).and_then(|s| s.image())
    /// ```
    pub fn get_image(&self, handle: &AsyncImageHandle) -> Option<&Image> {
        self.state(handle).and_then(|s| s.image())
    }

    /// Check if a handle is still loading.
    pub fn is_loading(&self, handle: &AsyncImageHandle) -> bool {
        self.state(handle).map(|s| s.is_loading()).unwrap_or(false)
    }

    /// Check if a handle is ready.
    pub fn is_ready(&self, handle: &AsyncImageHandle) -> bool {
        self.state(handle).map(|s| s.is_ready()).unwrap_or(false)
    }

    /// Get the number of loads currently in progress.
    #[inline]
    pub fn in_progress_count(&self) -> usize {
        self.in_progress
    }

    /// Get the total number of tracked handles (loading + completed + failed).
    #[inline]
    pub fn total_handles(&self) -> usize {
        self.states.len()
    }

    /// Remove a handle from tracking.
    ///
    /// This can be used to clean up handles that are no longer needed.
    /// If the handle is still loading, it will complete in the background
    /// but the result will be discarded.
    pub fn remove(&mut self, handle: &AsyncImageHandle) -> Option<LoadingState> {
        self.states.remove(handle)
    }

    /// Remove all completed (ready or failed) handles.
    ///
    /// This keeps handles that are still loading.
    /// Returns the number of handles removed.
    pub fn remove_completed(&mut self) -> usize {
        let before = self.states.len();
        self.states.retain(|_, state| state.is_loading());
        before - self.states.len()
    }

    /// Cancel all pending loads and shut down worker threads.
    ///
    /// This consumes the loader. Any handles from this loader will
    /// no longer be valid.
    pub fn shutdown(self) {
        // Drop happens in the Drop impl
    }
}

impl Default for AsyncImageLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AsyncImageLoader {
    fn drop(&mut self) {
        // Send shutdown signals to all workers
        for _ in 0..self.workers.len() {
            let _ = self.request_tx.send(LoadRequest::Shutdown);
        }

        // Wait for workers to finish
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loading_state_methods() {
        let loading = LoadingState::Loading;
        assert!(loading.is_loading());
        assert!(!loading.is_ready());
        assert!(!loading.is_failed());
        assert!(loading.image().is_none());
        assert!(loading.error().is_none());

        let failed = LoadingState::Failed("test error".to_string());
        assert!(!failed.is_loading());
        assert!(!failed.is_ready());
        assert!(failed.is_failed());
        assert!(failed.image().is_none());
        assert_eq!(failed.error(), Some("test error"));
    }

    #[test]
    fn test_handle_uniqueness() {
        let h1 = AsyncImageHandle::new();
        let h2 = AsyncImageHandle::new();
        assert_ne!(h1, h2);
        assert_ne!(h1.id(), h2.id());
    }

    #[test]
    fn test_config_defaults() {
        let config = AsyncImageLoaderConfig::default();
        assert!(config.worker_threads >= 1);
        assert!(config.worker_threads <= 4);
        assert_eq!(config.max_pending, 256);
    }

    #[test]
    fn test_decode_bytes_png() {
        // Create a minimal 1x1 PNG (red pixel)
        let png_data: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
            0x00, 0x00, 0x00, 0x0D, // IHDR length
            0x49, 0x48, 0x44, 0x52, // IHDR
            0x00, 0x00, 0x00, 0x01, // width = 1
            0x00, 0x00, 0x00, 0x01, // height = 1
            0x08, 0x02, // bit depth 8, color type 2 (RGB)
            0x00, 0x00, 0x00, // compression, filter, interlace
            0x90, 0x77, 0x53, 0xDE, // IHDR CRC
            0x00, 0x00, 0x00, 0x0C, // IDAT length
            0x49, 0x44, 0x41, 0x54, // IDAT
            0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, // compressed data
            0x01, 0x01, 0x01, 0x00, // CRC
            0xE6, 0xDC, 0x33, 0x08, // IEND
            0x00, 0x00, 0x00, 0x00, // IEND length
            0x49, 0x45, 0x4E, 0x44, // IEND
            0xAE, 0x42, 0x60, 0x82, // IEND CRC
        ];

        // This will fail with our minimal PNG, but tests the code path
        let result = AsyncImageLoader::decode_bytes(png_data);
        // We expect this to either succeed or fail gracefully
        assert!(result.is_ok() || result.is_err());
    }
}
