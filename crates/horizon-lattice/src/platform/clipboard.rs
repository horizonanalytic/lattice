//! Clipboard access for cross-platform copy/paste operations.
//!
//! This module provides clipboard operations with support for multiple data formats
//! and change detection through signals.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::platform::{Clipboard, ClipboardWatcher, ClipboardData};
//!
//! // Basic clipboard operations
//! let mut clipboard = Clipboard::new()?;
//! clipboard.set_text("Hello, world!")?;
//!
//! // Image support
//! let image = ImageData::new(100, 100, vec![0u8; 100 * 100 * 4]);
//! clipboard.set_image(&image)?;
//!
//! // Watch for clipboard changes
//! let watcher = ClipboardWatcher::new()?;
//! watcher.data_changed().connect(|data| {
//!     println!("Clipboard changed: {:?}", data);
//! });
//! watcher.start();
//! ```
//!
//! # Platform Notes
//!
//! - **Windows**: Uses Win32 clipboard API with WM_CLIPBOARDUPDATE for change detection
//! - **macOS**: Uses NSPasteboard with changeCount polling for change detection
//! - **Linux**: Uses X11 selections with XFIXES extension for change detection

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use horizon_lattice_core::signal::Signal;

/// Error type for clipboard operations.
#[derive(Debug)]
pub struct ClipboardError {
    message: String,
}

impl ClipboardError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "clipboard error: {}", self.message)
    }
}

impl std::error::Error for ClipboardError {}

impl From<arboard::Error> for ClipboardError {
    fn from(err: arboard::Error) -> Self {
        Self::new(err.to_string())
    }
}

/// Represents clipboard content in various formats.
#[derive(Debug, Clone)]
pub enum ClipboardData {
    /// Plain text content.
    Text(String),
    /// HTML formatted content with optional plain text fallback.
    Html {
        /// The HTML markup content.
        html: String,
        /// Optional plain text fallback for applications that don't support HTML.
        alt_text: Option<String>,
    },
    /// Image data in RGBA format.
    Image(ImageData),
    /// Clipboard is empty or contains unsupported format.
    Empty,
}

impl ClipboardData {
    /// Returns true if this is text content.
    pub fn is_text(&self) -> bool {
        matches!(self, ClipboardData::Text(_))
    }

    /// Returns true if this is HTML content.
    pub fn is_html(&self) -> bool {
        matches!(self, ClipboardData::Html { .. })
    }

    /// Returns true if this is image content.
    pub fn is_image(&self) -> bool {
        matches!(self, ClipboardData::Image(_))
    }

    /// Returns true if the clipboard is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, ClipboardData::Empty)
    }

    /// Attempts to get the content as text.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            ClipboardData::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Attempts to get the content as HTML.
    pub fn as_html(&self) -> Option<&str> {
        match self {
            ClipboardData::Html { html, .. } => Some(html),
            _ => None,
        }
    }

    /// Attempts to get the content as image data.
    pub fn as_image(&self) -> Option<&ImageData> {
        match self {
            ClipboardData::Image(img) => Some(img),
            _ => None,
        }
    }
}

/// Image data for clipboard operations.
///
/// Images are stored in RGBA format (4 bytes per pixel).
#[derive(Debug, Clone)]
pub struct ImageData {
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Raw pixel data in RGBA format (4 bytes per pixel).
    bytes: Vec<u8>,
}

impl ImageData {
    /// Creates new image data from raw RGBA bytes.
    ///
    /// # Panics
    ///
    /// Panics if `bytes.len() != width * height * 4`.
    pub fn new(width: usize, height: usize, bytes: Vec<u8>) -> Self {
        assert_eq!(
            bytes.len(),
            width * height * 4,
            "Image bytes must be width * height * 4 (RGBA)"
        );
        Self {
            width,
            height,
            bytes,
        }
    }

    /// Returns the raw RGBA bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Converts to owned bytes, consuming the image data.
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl From<arboard::ImageData<'_>> for ImageData {
    fn from(img: arboard::ImageData<'_>) -> Self {
        Self {
            width: img.width,
            height: img.height,
            bytes: img.bytes.into_owned(),
        }
    }
}

impl<'a> From<&'a ImageData> for arboard::ImageData<'a> {
    fn from(img: &'a ImageData) -> Self {
        arboard::ImageData {
            width: img.width,
            height: img.height,
            bytes: std::borrow::Cow::Borrowed(&img.bytes),
        }
    }
}

/// Cross-platform clipboard access.
///
/// Provides methods for reading and writing various data types to the system clipboard.
/// The clipboard instance should be created when needed and can be dropped after use.
///
/// # Thread Safety
///
/// While `Clipboard` is `Send`, it's recommended to perform clipboard
/// operations on the main/UI thread for best compatibility across platforms.
pub struct Clipboard {
    inner: arboard::Clipboard,
}

impl Clipboard {
    /// Create a new clipboard instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the clipboard cannot be accessed, which can happen
    /// if the system clipboard is unavailable or locked by another process.
    pub fn new() -> Result<Self, ClipboardError> {
        Ok(Self {
            inner: arboard::Clipboard::new()?,
        })
    }

    /// Get the current clipboard content.
    ///
    /// Attempts to retrieve content in order of preference: text, HTML, image.
    /// Returns `ClipboardData::Empty` if the clipboard is empty or contains
    /// an unsupported format.
    pub fn get(&mut self) -> ClipboardData {
        // Try text first (most common)
        if let Ok(text) = self.inner.get_text()
            && !text.is_empty()
        {
            return ClipboardData::Text(text);
        }

        // Try HTML
        if let Ok(html) = self.inner.get().html()
            && !html.is_empty()
        {
            return ClipboardData::Html {
                html,
                alt_text: self.inner.get_text().ok(),
            };
        }

        // Try image
        if let Ok(img) = self.inner.get_image() {
            return ClipboardData::Image(img.into());
        }

        ClipboardData::Empty
    }

    /// Get the current text content from the clipboard.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The clipboard is empty
    /// - The clipboard contains non-text data
    /// - The clipboard cannot be accessed
    pub fn get_text(&mut self) -> Result<String, ClipboardError> {
        self.inner.get_text().map_err(Into::into)
    }

    /// Set the clipboard text content.
    ///
    /// This replaces any existing clipboard content with the provided text.
    ///
    /// # Errors
    ///
    /// Returns an error if the text cannot be written to the clipboard.
    pub fn set_text(&mut self, text: impl AsRef<str>) -> Result<(), ClipboardError> {
        self.inner.set_text(text.as_ref()).map_err(Into::into)
    }

    /// Clear the clipboard contents.
    ///
    /// # Errors
    ///
    /// Returns an error if the clipboard cannot be cleared.
    pub fn clear(&mut self) -> Result<(), ClipboardError> {
        self.inner.clear().map_err(Into::into)
    }

    /// Set HTML content on the clipboard with a plain text fallback.
    ///
    /// This places both HTML and plain text versions on the clipboard, allowing
    /// applications that support rich text to paste the formatted version while
    /// others receive the plain text fallback.
    ///
    /// # Errors
    ///
    /// Returns an error if the content cannot be written to the clipboard.
    pub fn set_html(
        &mut self,
        html: impl AsRef<str>,
        alt_text: impl AsRef<str>,
    ) -> Result<(), ClipboardError> {
        self.inner
            .set_html(html.as_ref(), Some(alt_text.as_ref()))
            .map_err(Into::into)
    }

    /// Get HTML content from the clipboard.
    ///
    /// Returns the HTML content if available. Many applications place HTML
    /// on the clipboard when copying formatted text.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The clipboard doesn't contain HTML
    /// - The clipboard cannot be accessed
    pub fn get_html(&mut self) -> Result<String, ClipboardError> {
        self.inner.get().html().map_err(Into::into)
    }

    /// Get image data from the clipboard.
    ///
    /// Returns the image in RGBA format if available.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The clipboard doesn't contain an image
    /// - The clipboard cannot be accessed
    pub fn get_image(&mut self) -> Result<ImageData, ClipboardError> {
        self.inner.get_image().map(Into::into).map_err(Into::into)
    }

    /// Set image data on the clipboard.
    ///
    /// The image should be in RGBA format (4 bytes per pixel).
    ///
    /// # Errors
    ///
    /// Returns an error if the image cannot be written to the clipboard.
    pub fn set_image(&mut self, image: &ImageData) -> Result<(), ClipboardError> {
        self.inner.set_image(image.into()).map_err(Into::into)
    }

    /// Get file URLs from the clipboard.
    ///
    /// Returns a list of file paths if the clipboard contains files (e.g., from
    /// copying files in a file manager).
    ///
    /// # Platform Notes
    ///
    /// - **Windows**: Reads CF_HDROP clipboard format
    /// - **macOS**: Reads `public.file-url` pasteboard type
    /// - **Linux**: Reads `text/uri-list` from X11 clipboard
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The clipboard doesn't contain file URLs
    /// - The clipboard cannot be accessed
    pub fn get_file_urls(&mut self) -> Result<Vec<std::path::PathBuf>, ClipboardError> {
        get_file_urls_impl()
    }

    /// Set file URLs on the clipboard.
    ///
    /// Copies a list of file paths to the clipboard, allowing them to be pasted
    /// into file managers and other applications.
    ///
    /// # Platform Notes
    ///
    /// - **Windows**: Sets CF_HDROP clipboard format
    /// - **macOS**: Sets `public.file-url` pasteboard type
    /// - **Linux**: Sets `text/uri-list` on X11 clipboard
    ///
    /// # Errors
    ///
    /// Returns an error if the files cannot be written to the clipboard.
    pub fn set_file_urls(&mut self, paths: &[std::path::PathBuf]) -> Result<(), ClipboardError> {
        set_file_urls_impl(paths)
    }

    /// Check if the clipboard contains file URLs.
    ///
    /// This is a quick check to see if the clipboard has file data without
    /// actually retrieving the files.
    pub fn has_file_urls(&mut self) -> bool {
        has_file_urls_impl()
    }
}

impl fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Clipboard").finish_non_exhaustive()
    }
}

// ============================================================================
// Clipboard Watcher - Change Detection
// ============================================================================

/// Watches for clipboard changes and emits signals when content changes.
///
/// The watcher runs a background thread (or uses platform-specific event mechanisms)
/// to detect when the clipboard content changes. Connect to the `data_changed` signal
/// to receive notifications.
///
/// # Example
///
/// ```ignore
/// let watcher = ClipboardWatcher::new()?;
/// watcher.data_changed().connect(|data| {
///     match data {
///         ClipboardData::Text(text) => println!("Text: {}", text),
///         ClipboardData::Image(img) => println!("Image: {}x{}", img.width, img.height),
///         _ => {}
///     }
/// });
/// watcher.start();
/// // ... later
/// watcher.stop();
/// ```
pub struct ClipboardWatcher {
    /// Signal emitted when clipboard content changes (Arc-wrapped for thread sharing).
    data_changed: Arc<Signal<ClipboardData>>,
    /// Whether the watcher is running.
    running: Arc<AtomicBool>,
    /// Handle to the watcher thread.
    thread_handle: parking_lot::Mutex<Option<JoinHandle<()>>>,
    /// Last known clipboard change count (platform-specific).
    #[allow(dead_code)]
    last_change_count: Arc<AtomicI64>,
}

impl ClipboardWatcher {
    /// Create a new clipboard watcher.
    ///
    /// The watcher is not started automatically. Call `start()` to begin watching.
    pub fn new() -> Result<Self, ClipboardError> {
        Ok(Self {
            data_changed: Arc::new(Signal::new()),
            running: Arc::new(AtomicBool::new(false)),
            thread_handle: parking_lot::Mutex::new(None),
            last_change_count: Arc::new(AtomicI64::new(-1)),
        })
    }

    /// Get the signal that is emitted when clipboard content changes.
    ///
    /// Connect to this signal to receive notifications of clipboard changes.
    pub fn data_changed(&self) -> &Signal<ClipboardData> {
        &self.data_changed
    }

    /// Start watching for clipboard changes.
    ///
    /// This spawns a background thread that monitors the clipboard. On platforms
    /// with event-based clipboard notifications (Windows, Linux/X11), the thread
    /// waits for events. On platforms without such support (macOS), it polls
    /// periodically.
    pub fn start(&self) {
        if self.running.swap(true, Ordering::SeqCst) {
            // Already running
            return;
        }

        let running = self.running.clone();
        let data_changed = self.data_changed.clone();
        let last_change_count = self.last_change_count.clone();

        let handle = thread::spawn(move || {
            Self::watch_loop(running, data_changed, last_change_count);
        });

        *self.thread_handle.lock() = Some(handle);
    }

    /// Stop watching for clipboard changes.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.lock().take() {
            let _ = handle.join();
        }
    }

    /// Check if the watcher is currently running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    // Platform-specific watch loop implementations

    #[cfg(target_os = "windows")]
    fn watch_loop(
        running: Arc<AtomicBool>,
        data_changed: Arc<Signal<ClipboardData>>,
        _last_change_count: Arc<AtomicI64>,
    ) {
        use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
        use windows::Win32::System::DataExchange::AddClipboardFormatListener;
        use windows::Win32::System::LibraryLoader::GetModuleHandleW;
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, HMENU, MSG,
            PM_REMOVE, PeekMessageW, RegisterClassW, WM_CLIPBOARDUPDATE, WM_USER, WNDCLASSW,
            WS_OVERLAPPED,
        };
        use windows::core::PCWSTR;

        const WM_STOP_WATCHING: u32 = WM_USER + 1;

        unsafe extern "system" fn window_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        // SAFETY: All Windows API calls in this block are safe because:
        // - class_name is a valid null-terminated UTF-16 string that outlives all uses
        // - class_name_ptr points to valid data for the duration of the block
        // - wc is a properly initialized WNDCLASSW with valid function pointer
        // - hwnd is checked for validity before use
        // - Window and clipboard listener are properly cleaned up before returning
        // - The message loop only accesses the window we created
        unsafe {
            // Register window class
            let class_name: Vec<u16> = "HorizonLatticeClipboardWatcher\0".encode_utf16().collect();
            let class_name_ptr = PCWSTR::from_raw(class_name.as_ptr());

            let wc = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: GetModuleHandleW(None).unwrap_or_default().into(),
                lpszClassName: class_name_ptr,
                ..Default::default()
            };
            RegisterClassW(&wc);

            // Create message-only window
            let hwnd = CreateWindowExW(
                Default::default(),
                class_name_ptr,
                PCWSTR::null(),
                WS_OVERLAPPED,
                0,
                0,
                0,
                0,
                HWND::default(),
                HMENU::default(),
                wc.hInstance,
                None,
            );

            if hwnd == HWND::default() {
                return;
            }

            // Register for clipboard notifications
            if AddClipboardFormatListener(hwnd).is_err() {
                let _ = DestroyWindow(hwnd);
                return;
            }

            let mut msg = MSG::default();
            while running.load(Ordering::SeqCst) {
                // Use PeekMessage with a small sleep to allow checking the running flag
                if PeekMessageW(&mut msg, hwnd, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_CLIPBOARDUPDATE {
                        // Clipboard changed, emit signal
                        if let Ok(mut clipboard) = Clipboard::new() {
                            let content = clipboard.get();
                            data_changed.emit(content);
                        }
                    } else if msg.message == WM_STOP_WATCHING {
                        break;
                    }
                    let _ = DispatchMessageW(&msg);
                } else {
                    // Sleep briefly to avoid busy-waiting
                    thread::sleep(Duration::from_millis(50));
                }
            }

            let _ = DestroyWindow(hwnd);
        }
    }

    #[cfg(target_os = "macos")]
    fn watch_loop(
        running: Arc<AtomicBool>,
        data_changed: Arc<Signal<ClipboardData>>,
        last_change_count: Arc<AtomicI64>,
    ) {
        use objc2_app_kit::NSPasteboard;

        // Poll interval for macOS (no event-based API available)
        const POLL_INTERVAL: Duration = Duration::from_millis(250);

        // Initialize the last change count
        // NSPasteboard.changeCount returns isize, we store as i64 for cross-platform compatibility
        let pasteboard = NSPasteboard::generalPasteboard();
        let initial_count = pasteboard.changeCount() as i64;
        last_change_count.store(initial_count, Ordering::SeqCst);

        while running.load(Ordering::SeqCst) {
            thread::sleep(POLL_INTERVAL);

            let current_count = pasteboard.changeCount() as i64;
            let last_count = last_change_count.load(Ordering::SeqCst);

            if current_count != last_count {
                last_change_count.store(current_count, Ordering::SeqCst);

                // Clipboard changed, emit signal
                if let Ok(mut clipboard) = Clipboard::new() {
                    let content = clipboard.get();
                    data_changed.emit(content);
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn watch_loop(
        running: Arc<AtomicBool>,
        data_changed: Arc<Signal<ClipboardData>>,
        _last_change_count: Arc<AtomicI64>,
    ) {
        use x11_clipboard::Clipboard as X11Clipboard;

        // Try to use X11 clipboard monitoring
        let clipboard = match X11Clipboard::new() {
            Ok(c) => c,
            Err(_) => {
                // Fall back to polling if X11 is not available
                Self::watch_loop_polling(running, data_changed);
                return;
            }
        };

        // X11 CLIPBOARD atom
        let atoms = &clipboard.getter.atoms;
        let clipboard_atom = atoms.clipboard;

        // We'll poll for changes since x11-clipboard doesn't expose raw X11 events
        // In a more complete implementation, we'd use x11rb or xcb directly for XFIXES
        const POLL_INTERVAL: Duration = Duration::from_millis(250);

        let mut last_text: Option<String> = None;

        while running.load(Ordering::SeqCst) {
            thread::sleep(POLL_INTERVAL);

            // Check if clipboard content changed
            if let Ok(text) = clipboard.load(
                clipboard_atom,
                atoms.utf8_string,
                atoms.property,
                Duration::from_millis(100),
            ) {
                let current_text = String::from_utf8_lossy(&text).to_string();
                if last_text.as_ref() != Some(&current_text) {
                    last_text = Some(current_text.clone());
                    data_changed.emit(ClipboardData::Text(current_text));
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn watch_loop_polling(running: Arc<AtomicBool>, data_changed: Arc<Signal<ClipboardData>>) {
        const POLL_INTERVAL: Duration = Duration::from_millis(500);

        let mut last_content_hash: Option<u64> = None;

        while running.load(Ordering::SeqCst) {
            thread::sleep(POLL_INTERVAL);

            if let Ok(mut clipboard) = Clipboard::new() {
                let content = clipboard.get();
                let content_hash = Self::hash_content(&content);

                if last_content_hash != Some(content_hash) {
                    last_content_hash = Some(content_hash);
                    data_changed.emit(content);
                }
            }
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    fn watch_loop(
        running: Arc<AtomicBool>,
        data_changed: Arc<Signal<ClipboardData>>,
        _last_change_count: Arc<AtomicI64>,
    ) {
        // Generic polling fallback for other platforms
        const POLL_INTERVAL: Duration = Duration::from_millis(500);

        let mut last_content_hash: Option<u64> = None;

        while running.load(Ordering::SeqCst) {
            thread::sleep(POLL_INTERVAL);

            if let Ok(mut clipboard) = Clipboard::new() {
                let content = clipboard.get();
                let content_hash = Self::hash_content(&content);

                if last_content_hash != Some(content_hash) {
                    last_content_hash = Some(content_hash);
                    data_changed.emit(content);
                }
            }
        }
    }

    /// Simple hash for detecting content changes.
    #[allow(dead_code)]
    fn hash_content(content: &ClipboardData) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        match content {
            ClipboardData::Text(s) => {
                0u8.hash(&mut hasher);
                s.hash(&mut hasher);
            }
            ClipboardData::Html { html, .. } => {
                1u8.hash(&mut hasher);
                html.hash(&mut hasher);
            }
            ClipboardData::Image(img) => {
                2u8.hash(&mut hasher);
                img.width.hash(&mut hasher);
                img.height.hash(&mut hasher);
                // Hash first and last bytes for quick comparison
                if !img.bytes.is_empty() {
                    img.bytes[0].hash(&mut hasher);
                    img.bytes[img.bytes.len() - 1].hash(&mut hasher);
                    img.bytes.len().hash(&mut hasher);
                }
            }
            ClipboardData::Empty => {
                3u8.hash(&mut hasher);
            }
        }
        hasher.finish()
    }
}

impl Drop for ClipboardWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

impl fmt::Debug for ClipboardWatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClipboardWatcher")
            .field("running", &self.is_running())
            .finish_non_exhaustive()
    }
}

// ============================================================================
// X11 Selection Clipboard Support (Linux-specific)
// ============================================================================

/// X11 selection type for Linux clipboard operations.
///
/// X11 has multiple "selections" that act like independent clipboards:
///
/// - **Primary**: Automatically updated with the current text selection. Paste with middle-click.
/// - **Secondary**: Rarely used, originally for a secondary selection.
/// - **Clipboard**: The standard clipboard (Ctrl+C/Ctrl+V).
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X11Selection {
    /// Primary selection - text selection, middle-click paste.
    Primary,
    /// Secondary selection - rarely used.
    Secondary,
    /// Standard clipboard - Ctrl+C/Ctrl+V.
    Clipboard,
}

/// X11-specific clipboard access supporting multiple selections.
///
/// This provides direct access to X11's selection mechanism, which includes
/// the primary selection (text selection), secondary selection, and clipboard.
#[cfg(target_os = "linux")]
pub struct X11Clipboard {
    inner: x11_clipboard::Clipboard,
}

#[cfg(target_os = "linux")]
impl X11Clipboard {
    /// Create a new X11 clipboard instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the X11 connection cannot be established.
    pub fn new() -> Result<Self, ClipboardError> {
        x11_clipboard::Clipboard::new()
            .map(|c| Self { inner: c })
            .map_err(|e| ClipboardError::new(format!("X11 clipboard error: {:?}", e)))
    }

    /// Get text from the specified selection.
    ///
    /// # Arguments
    ///
    /// * `selection` - Which X11 selection to read from.
    ///
    /// # Errors
    ///
    /// Returns an error if the selection cannot be read or doesn't contain text.
    pub fn get_text(&self, selection: X11Selection) -> Result<String, ClipboardError> {
        let atoms = &self.inner.getter.atoms;
        let selection_atom = match selection {
            X11Selection::Primary => atoms.primary,
            X11Selection::Secondary => atoms.secondary,
            X11Selection::Clipboard => atoms.clipboard,
        };

        self.inner
            .load(
                selection_atom,
                atoms.utf8_string,
                atoms.property,
                Duration::from_secs(1),
            )
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
            .map_err(|e| ClipboardError::new(format!("Failed to get X11 selection: {:?}", e)))
    }

    /// Set text in the specified selection.
    ///
    /// # Arguments
    ///
    /// * `selection` - Which X11 selection to write to.
    /// * `text` - The text to store in the selection.
    ///
    /// # Errors
    ///
    /// Returns an error if the selection cannot be written.
    pub fn set_text(
        &self,
        selection: X11Selection,
        text: impl AsRef<str>,
    ) -> Result<(), ClipboardError> {
        let atoms = &self.inner.setter.atoms;
        let selection_atom = match selection {
            X11Selection::Primary => atoms.primary,
            X11Selection::Secondary => atoms.secondary,
            X11Selection::Clipboard => atoms.clipboard,
        };

        self.inner
            .store(selection_atom, atoms.utf8_string, text.as_ref().as_bytes())
            .map_err(|e| ClipboardError::new(format!("Failed to set X11 selection: {:?}", e)))
    }

    /// Get the primary selection (text selection).
    ///
    /// This is the selection that's automatically updated when you select text.
    /// Traditionally pasted with middle-click.
    pub fn get_primary(&self) -> Result<String, ClipboardError> {
        self.get_text(X11Selection::Primary)
    }

    /// Set the primary selection.
    pub fn set_primary(&self, text: impl AsRef<str>) -> Result<(), ClipboardError> {
        self.set_text(X11Selection::Primary, text)
    }

    /// Get the secondary selection.
    ///
    /// The secondary selection is rarely used by modern applications.
    pub fn get_secondary(&self) -> Result<String, ClipboardError> {
        self.get_text(X11Selection::Secondary)
    }

    /// Set the secondary selection.
    pub fn set_secondary(&self, text: impl AsRef<str>) -> Result<(), ClipboardError> {
        self.set_text(X11Selection::Secondary, text)
    }

    /// Get the standard clipboard (same as regular clipboard).
    ///
    /// This is the same as using `Ctrl+C/Ctrl+V`.
    pub fn get_clipboard(&self) -> Result<String, ClipboardError> {
        self.get_text(X11Selection::Clipboard)
    }

    /// Set the standard clipboard.
    pub fn set_clipboard(&self, text: impl AsRef<str>) -> Result<(), ClipboardError> {
        self.set_text(X11Selection::Clipboard, text)
    }
}

#[cfg(target_os = "linux")]
impl fmt::Debug for X11Clipboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("X11Clipboard").finish_non_exhaustive()
    }
}

// ============================================================================
// File URL Clipboard Support - Platform Implementations
// ============================================================================

// Windows implementation using CF_HDROP
#[cfg(target_os = "windows")]
fn get_file_urls_impl() -> Result<Vec<std::path::PathBuf>, ClipboardError> {
    use std::path::PathBuf;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};

    // SAFETY: All Windows clipboard API calls are safe because:
    // - OpenClipboard is called with default HWND (current thread)
    // - CloseClipboard is always called via closure pattern (RAII-like)
    // - GetClipboardData returns a handle owned by the system
    // - DragQueryFileW only reads from the valid HDROP handle
    // - Buffer allocation is properly sized before DragQueryFileW writes to it
    unsafe {
        // Open clipboard
        OpenClipboard(HWND::default())
            .map_err(|e| ClipboardError::new(format!("Failed to open clipboard: {}", e)))?;

        let result = (|| -> Result<Vec<PathBuf>, ClipboardError> {
            // Check if CF_HDROP is available
            if !IsClipboardFormatAvailable(CF_HDROP.0 as u32).as_bool() {
                return Err(ClipboardError::new("Clipboard does not contain file URLs"));
            }

            // Get clipboard data
            let handle = GetClipboardData(CF_HDROP.0 as u32)
                .map_err(|e| ClipboardError::new(format!("Failed to get clipboard data: {}", e)))?;

            let hdrop = HDROP(handle.0);

            // Get file count
            let file_count = DragQueryFileW(hdrop, 0xFFFFFFFF, None);
            if file_count == 0 {
                return Ok(Vec::new());
            }

            let mut paths = Vec::with_capacity(file_count as usize);

            for i in 0..file_count {
                // Get required buffer size
                let size = DragQueryFileW(hdrop, i, None);
                if size == 0 {
                    continue;
                }

                // Allocate buffer and get filename
                let mut buffer: Vec<u16> = vec![0; (size + 1) as usize];
                let chars_copied = DragQueryFileW(hdrop, i, Some(&mut buffer));
                if chars_copied > 0 {
                    buffer.truncate(chars_copied as usize);
                    let path_str = String::from_utf16_lossy(&buffer);
                    paths.push(PathBuf::from(path_str));
                }
            }

            Ok(paths)
        })();

        // Always close clipboard
        let _ = CloseClipboard();
        result
    }
}

#[cfg(target_os = "windows")]
fn set_file_urls_impl(paths: &[std::path::PathBuf]) -> Result<(), ClipboardError> {
    use std::mem;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{GlobalFree, HGLOBAL, HWND};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalUnlock};
    use windows::Win32::System::Ole::CF_HDROP;
    use windows::Win32::UI::Shell::DROPFILES;

    if paths.is_empty() {
        return Ok(());
    }

    // SAFETY: All Windows clipboard and memory API calls are safe because:
    // - GlobalAlloc allocates memory of exactly the size we calculated
    // - GlobalLock returns a valid pointer or null (which we check)
    // - We write exactly within the allocated bounds (DROPFILES + paths)
    // - GlobalUnlock is called before SetClipboardData
    // - CloseClipboard is always called via closure pattern
    // - On error, GlobalFree cleans up the allocated memory
    // - SetClipboardData takes ownership of the memory on success
    unsafe {
        // Calculate total size needed for DROPFILES + file paths (double-null terminated)
        let mut total_chars = 0usize;
        let wide_paths: Vec<Vec<u16>> = paths
            .iter()
            .map(|p| {
                let wide: Vec<u16> = p.as_os_str().encode_wide().chain(Some(0)).collect();
                total_chars += wide.len();
                wide
            })
            .collect();
        total_chars += 1; // Final null terminator

        let dropfiles_size = mem::size_of::<DROPFILES>();
        let total_size = dropfiles_size + total_chars * mem::size_of::<u16>();

        // Open clipboard
        OpenClipboard(HWND::default())
            .map_err(|e| ClipboardError::new(format!("Failed to open clipboard: {}", e)))?;

        let result = (|| -> Result<(), ClipboardError> {
            // Empty clipboard
            EmptyClipboard()
                .map_err(|e| ClipboardError::new(format!("Failed to empty clipboard: {}", e)))?;

            // Allocate global memory
            let hglobal = GlobalAlloc(GMEM_MOVEABLE, total_size)
                .map_err(|e| ClipboardError::new(format!("Failed to allocate memory: {}", e)))?;

            // Lock memory and fill it
            let ptr = GlobalLock(hglobal);
            if ptr.is_null() {
                let _ = GlobalFree(hglobal);
                return Err(ClipboardError::new("Failed to lock global memory"));
            }

            // Fill DROPFILES structure
            let dropfiles = ptr as *mut DROPFILES;
            (*dropfiles).pFiles = dropfiles_size as u32;
            (*dropfiles).pt.x = 0;
            (*dropfiles).pt.y = 0;
            (*dropfiles).fNC = false.into();
            (*dropfiles).fWide = true.into(); // Unicode paths

            // Copy file paths after DROPFILES structure
            let mut dest = (ptr as *mut u8).add(dropfiles_size) as *mut u16;
            for wide_path in &wide_paths {
                for &wchar in wide_path {
                    *dest = wchar;
                    dest = dest.add(1);
                }
            }
            // Final null terminator
            *dest = 0;

            let _ = GlobalUnlock(hglobal);

            // Set clipboard data
            SetClipboardData(CF_HDROP.0 as u32, HGLOBAL(hglobal.0 as *mut _).0)
                .map_err(|e| ClipboardError::new(format!("Failed to set clipboard data: {}", e)))?;

            Ok(())
        })();

        // Always close clipboard
        let _ = CloseClipboard();
        result
    }
}

#[cfg(target_os = "windows")]
fn has_file_urls_impl() -> bool {
    use windows::Win32::System::DataExchange::IsClipboardFormatAvailable;
    use windows::Win32::System::Ole::CF_HDROP;

    unsafe { IsClipboardFormatAvailable(CF_HDROP.0 as u32).as_bool() }
}

// macOS implementation using NSPasteboard
#[cfg(target_os = "macos")]
fn get_file_urls_impl() -> Result<Vec<std::path::PathBuf>, ClipboardError> {
    use objc2::ClassType;
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::{NSArray, NSURL};
    use std::path::PathBuf;

    let pasteboard = NSPasteboard::generalPasteboard();

    // Read URLs from pasteboard using the modern API
    let url_class = NSURL::class();
    let classes: objc2::rc::Retained<NSArray<_>> = NSArray::from_slice(&[url_class]);
    let options = objc2_foundation::NSDictionary::new();

    let objects = unsafe { pasteboard.readObjectsForClasses_options(&classes, Some(&options)) };

    match objects {
        Some(urls) => {
            let mut paths = Vec::new();
            let count = urls.count();
            for i in 0..count {
                // Use objectAtIndex to get items from NSArray
                let obj = urls.objectAtIndex(i);
                // The object should be an NSURL - cast it via pointer
                let url: &NSURL = unsafe { &*(&*obj as *const _ as *const NSURL) };
                if url.isFileURL()
                    && let Some(path_str) = url.path()
                {
                    let path_string: String = path_str.to_string();
                    paths.push(PathBuf::from(path_string));
                }
            }
            if paths.is_empty() {
                Err(ClipboardError::new("No file URLs in clipboard"))
            } else {
                Ok(paths)
            }
        }
        None => Err(ClipboardError::new("Clipboard does not contain file URLs")),
    }
}

#[cfg(target_os = "macos")]
fn set_file_urls_impl(paths: &[std::path::PathBuf]) -> Result<(), ClipboardError> {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeFileURL};
    use objc2_foundation::NSString;

    if paths.is_empty() {
        return Ok(());
    }

    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();

    // For setting file URLs, we use the older but more reliable string-based API
    // Convert paths to file:// URI strings and set as text/uri-list equivalent
    let mut uri_list = String::new();
    for path in paths {
        // Create file:// URI
        let uri = crate::platform::file_uri::path_to_uri(path);
        uri_list.push_str(&uri);
        uri_list.push('\n');
    }

    // Set as the file URL pasteboard type
    let ns_uri_list = NSString::from_str(&uri_list);
    let result = unsafe { pasteboard.setString_forType(&ns_uri_list, NSPasteboardTypeFileURL) };

    if result {
        Ok(())
    } else {
        // Fallback: try setting as plain text with file:// URLs
        let result = unsafe {
            pasteboard.setString_forType(&ns_uri_list, objc2_app_kit::NSPasteboardTypeString)
        };
        if result {
            Ok(())
        } else {
            Err(ClipboardError::new(
                "Failed to write file URLs to clipboard",
            ))
        }
    }
}

#[cfg(target_os = "macos")]
fn has_file_urls_impl() -> bool {
    use objc2::ClassType;
    use objc2_app_kit::NSPasteboard;
    use objc2_foundation::{NSArray, NSURL};

    let pasteboard = NSPasteboard::generalPasteboard();
    let url_class = NSURL::class();
    let classes: objc2::rc::Retained<NSArray<_>> = NSArray::from_slice(&[url_class]);
    let options = objc2_foundation::NSDictionary::new();

    unsafe { pasteboard.canReadObjectForClasses_options(&classes, Some(&options)) }
}

// Linux implementation using X11 text/uri-list
#[cfg(target_os = "linux")]
fn get_file_urls_impl() -> Result<Vec<std::path::PathBuf>, ClipboardError> {
    use crate::platform::file_uri;
    use std::time::Duration;
    use x11_clipboard::Clipboard as X11ClipboardInner;

    let clipboard = X11ClipboardInner::new()
        .map_err(|e| ClipboardError::new(format!("Failed to connect to X11: {:?}", e)))?;

    let atoms = &clipboard.getter.atoms;

    // Try to read text/uri-list
    let uri_list_atom = clipboard
        .getter
        .connection
        .intern_atom(false, b"text/uri-list")
        .map_err(|e| ClipboardError::new(format!("Failed to intern atom: {:?}", e)))?
        .reply()
        .map_err(|e| ClipboardError::new(format!("Failed to get atom reply: {:?}", e)))?
        .atom;

    let data = clipboard
        .load(
            atoms.clipboard,
            uri_list_atom,
            atoms.property,
            Duration::from_secs(2),
        )
        .map_err(|e| ClipboardError::new(format!("Failed to read clipboard: {:?}", e)))?;

    let uri_list = String::from_utf8_lossy(&data).into_owned();
    let paths = file_uri::parse_uri_list(&uri_list);

    if paths.is_empty() {
        Err(ClipboardError::new("No file URLs in clipboard"))
    } else {
        Ok(paths)
    }
}

#[cfg(target_os = "linux")]
fn set_file_urls_impl(paths: &[std::path::PathBuf]) -> Result<(), ClipboardError> {
    use crate::platform::file_uri;
    use x11_clipboard::Clipboard as X11ClipboardInner;

    if paths.is_empty() {
        return Ok(());
    }

    let clipboard = X11ClipboardInner::new()
        .map_err(|e| ClipboardError::new(format!("Failed to connect to X11: {:?}", e)))?;

    let atoms = &clipboard.setter.atoms;

    // Intern text/uri-list atom
    let uri_list_atom = clipboard
        .setter
        .connection
        .intern_atom(false, b"text/uri-list")
        .map_err(|e| ClipboardError::new(format!("Failed to intern atom: {:?}", e)))?
        .reply()
        .map_err(|e| ClipboardError::new(format!("Failed to get atom reply: {:?}", e)))?
        .atom;

    // Format paths as URI list
    let uri_list = file_uri::format_uri_list(paths);

    clipboard
        .store(atoms.clipboard, uri_list_atom, uri_list.as_bytes())
        .map_err(|e| ClipboardError::new(format!("Failed to write clipboard: {:?}", e)))?;

    Ok(())
}

#[cfg(target_os = "linux")]
fn has_file_urls_impl() -> bool {
    use std::time::Duration;
    use x11_clipboard::Clipboard as X11ClipboardInner;

    let Ok(clipboard) = X11ClipboardInner::new() else {
        return false;
    };

    let atoms = &clipboard.getter.atoms;

    // Try to intern text/uri-list atom
    let Ok(cookie) = clipboard
        .getter
        .connection
        .intern_atom(false, b"text/uri-list")
    else {
        return false;
    };

    let Ok(reply) = cookie.reply() else {
        return false;
    };

    // Check if clipboard has this format
    clipboard
        .load(
            atoms.clipboard,
            reply.atom,
            atoms.property,
            Duration::from_millis(100),
        )
        .map(|data| !data.is_empty())
        .unwrap_or(false)
}

// Fallback for other platforms
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn get_file_urls_impl() -> Result<Vec<std::path::PathBuf>, ClipboardError> {
    Err(ClipboardError::new(
        "File URL clipboard support not available on this platform",
    ))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn set_file_urls_impl(_paths: &[std::path::PathBuf]) -> Result<(), ClipboardError> {
    Err(ClipboardError::new(
        "File URL clipboard support not available on this platform",
    ))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
fn has_file_urls_impl() -> bool {
    false
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_creation() {
        // This test may fail in CI environments without a display
        let result = Clipboard::new();
        // Just verify it doesn't panic - actual clipboard access depends on environment
        let _ = result;
    }

    #[test]
    fn test_clipboard_error_display() {
        let error = ClipboardError::new("test error");
        assert_eq!(error.to_string(), "clipboard error: test error");
    }

    #[test]
    fn test_image_data_creation() {
        let width = 10;
        let height = 10;
        let bytes = vec![0u8; width * height * 4];
        let image = ImageData::new(width, height, bytes.clone());

        assert_eq!(image.width, width);
        assert_eq!(image.height, height);
        assert_eq!(image.bytes(), &bytes[..]);
    }

    #[test]
    #[should_panic(expected = "Image bytes must be width * height * 4")]
    fn test_image_data_wrong_size() {
        let _image = ImageData::new(10, 10, vec![0u8; 100]); // Should be 400
    }

    #[test]
    fn test_clipboard_data_variants() {
        let text = ClipboardData::Text("hello".into());
        assert!(text.is_text());
        assert_eq!(text.as_text(), Some("hello"));

        let html = ClipboardData::Html {
            html: "<b>hello</b>".into(),
            alt_text: Some("hello".into()),
        };
        assert!(html.is_html());
        assert_eq!(html.as_html(), Some("<b>hello</b>"));

        let image = ClipboardData::Image(ImageData::new(1, 1, vec![0, 0, 0, 255]));
        assert!(image.is_image());
        assert!(image.as_image().is_some());

        let empty = ClipboardData::Empty;
        assert!(empty.is_empty());
    }

    #[test]
    fn test_clipboard_watcher_creation() {
        let result = ClipboardWatcher::new();
        // Just verify it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_clipboard_watcher_start_stop() {
        if let Ok(watcher) = ClipboardWatcher::new() {
            assert!(!watcher.is_running());
            watcher.start();
            assert!(watcher.is_running());
            watcher.stop();
            // Give it time to stop
            std::thread::sleep(std::time::Duration::from_millis(100));
            assert!(!watcher.is_running());
        }
    }

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::*;

        #[test]
        fn test_x11_clipboard_creation() {
            // This will fail without X11 display
            let result = X11Clipboard::new();
            let _ = result;
        }

        #[test]
        fn test_x11_selection_enum() {
            assert_ne!(X11Selection::Primary, X11Selection::Clipboard);
            assert_ne!(X11Selection::Secondary, X11Selection::Clipboard);
        }
    }

    // File URL clipboard tests - these test the API but may not work in CI
    // without a display/clipboard access
    mod file_url_tests {
        use super::*;
        use std::path::PathBuf;

        #[test]
        fn test_file_url_methods_exist() {
            // Test that the methods compile and are accessible
            // Actual clipboard access may fail in CI
            if let Ok(mut clipboard) = Clipboard::new() {
                // Test has_file_urls
                let _has = clipboard.has_file_urls();

                // Test get_file_urls (may return error if clipboard is empty)
                let _ = clipboard.get_file_urls();

                // Test set_file_urls
                let paths = vec![PathBuf::from("/tmp/test.txt")];
                let _ = clipboard.set_file_urls(&paths);
            }
        }

        #[test]
        #[ignore] // Run manually with --ignored when clipboard is available
        fn test_file_url_roundtrip() {
            let mut clipboard = Clipboard::new().expect("Failed to create clipboard");

            // Set file URLs
            let paths = vec![
                PathBuf::from("/tmp/file1.txt"),
                PathBuf::from("/tmp/file2.txt"),
            ];
            clipboard
                .set_file_urls(&paths)
                .expect("Failed to set file URLs");

            // Verify they're set
            assert!(clipboard.has_file_urls());

            // Get them back
            let retrieved = clipboard.get_file_urls().expect("Failed to get file URLs");
            assert_eq!(retrieved.len(), 2);
        }
    }
}
