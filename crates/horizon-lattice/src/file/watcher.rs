//! File system watching for detecting file and directory changes.
//!
//! This module provides cross-platform file system monitoring with support for
//! both event-driven (signal-based) and polling usage patterns.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::file::{FileWatcher, WatchEventKind};
//!
//! // Create a watcher
//! let mut watcher = FileWatcher::new()?;
//!
//! // Connect to the changed signal for reactive updates
//! watcher.changed.connect(|event| {
//!     match event.kind {
//!         WatchEventKind::Created => println!("Created: {}", event.path.display()),
//!         WatchEventKind::Modified => println!("Modified: {}", event.path.display()),
//!         WatchEventKind::Removed => println!("Removed: {}", event.path.display()),
//!     }
//! });
//!
//! // Watch a file or directory
//! watcher.watch("config.toml")?;
//! watcher.watch_recursive("src/")?;
//!
//! // In your event loop, call process() to emit signals
//! watcher.process();
//!
//! // Or use poll() for manual event handling
//! let events = watcher.poll();
//! for event in events {
//!     println!("{:?}: {}", event.kind, event.path.display());
//! }
//! ```

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{new_debouncer, Debouncer, DebouncedEvent, DebouncedEventKind};

use horizon_lattice_core::signal::Signal;

use super::error::{FileError, FileErrorKind, FileResult};

/// The type of file system change event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WatchEventKind {
    /// A file or directory was created.
    Created,
    /// A file or directory was modified.
    Modified,
    /// A file or directory was removed.
    Removed,
}

impl std::fmt::Display for WatchEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WatchEventKind::Created => write!(f, "created"),
            WatchEventKind::Modified => write!(f, "modified"),
            WatchEventKind::Removed => write!(f, "removed"),
        }
    }
}

/// An event indicating a file system change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWatchEvent {
    /// The path of the changed file or directory.
    pub path: PathBuf,
    /// The type of change.
    pub kind: WatchEventKind,
}

impl FileWatchEvent {
    /// Create a new file watch event.
    pub fn new(path: PathBuf, kind: WatchEventKind) -> Self {
        Self { path, kind }
    }

    /// Returns true if this is a creation event.
    pub fn is_created(&self) -> bool {
        self.kind == WatchEventKind::Created
    }

    /// Returns true if this is a modification event.
    pub fn is_modified(&self) -> bool {
        self.kind == WatchEventKind::Modified
    }

    /// Returns true if this is a removal event.
    pub fn is_removed(&self) -> bool {
        self.kind == WatchEventKind::Removed
    }
}

/// Configuration options for file watching.
#[derive(Debug, Clone)]
pub struct WatchOptions {
    /// Debounce duration for coalescing rapid changes.
    /// Default: 100ms
    pub debounce_duration: Duration,
}

impl Default for WatchOptions {
    fn default() -> Self {
        Self {
            debounce_duration: Duration::from_millis(100),
        }
    }
}

impl WatchOptions {
    /// Create new watch options with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the debounce duration.
    pub fn debounce(mut self, duration: Duration) -> Self {
        self.debounce_duration = duration;
        self
    }
}

/// Tracks whether a path is watched recursively or non-recursively.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WatchMode {
    Recursive,
    NonRecursive,
}

/// A file system watcher that monitors files and directories for changes.
///
/// The watcher uses the platform's native file system notification APIs
/// (inotify on Linux, FSEvents on macOS, ReadDirectoryChangesW on Windows)
/// with debouncing to coalesce rapid changes.
///
/// # Signal-Based Usage
///
/// Connect to the `changed` signal for reactive event handling:
///
/// ```ignore
/// watcher.changed.connect(|event| {
///     println!("File changed: {}", event.path.display());
/// });
///
/// // Call process() in your event loop to emit signals
/// watcher.process();
/// ```
///
/// # Poll-Based Usage
///
/// Use `poll()` to manually retrieve events:
///
/// ```ignore
/// let events = watcher.poll();
/// for event in events {
///     handle_event(event);
/// }
/// ```
pub struct FileWatcher {
    /// The debounced watcher instance.
    debouncer: Debouncer<RecommendedWatcher>,
    /// Channel receiver for debounced events.
    rx: Receiver<Result<Vec<DebouncedEvent>, notify::Error>>,
    /// Set of paths being watched with their watch modes.
    watched_paths: HashSet<PathBuf>,
    /// Maps paths to their watch modes.
    watch_modes: std::collections::HashMap<PathBuf, WatchMode>,
    /// Signal emitted when a watched file or directory changes.
    pub changed: Signal<FileWatchEvent>,
}

impl FileWatcher {
    /// Create a new file watcher with default options.
    pub fn new() -> FileResult<Self> {
        Self::with_options(WatchOptions::default())
    }

    /// Create a new file watcher with custom options.
    pub fn with_options(options: WatchOptions) -> FileResult<Self> {
        let (tx, rx) = mpsc::channel();

        let debouncer = new_debouncer(options.debounce_duration, tx)
            .map_err(|e| FileError::new(
                FileErrorKind::Other,
                None,
                Some(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            ))?;

        Ok(Self {
            debouncer,
            rx,
            watched_paths: HashSet::new(),
            watch_modes: std::collections::HashMap::new(),
            changed: Signal::new(),
        })
    }

    /// Start watching a file or directory (non-recursive).
    ///
    /// For directories, only direct children are watched, not subdirectories.
    /// Use [`watch_recursive`](Self::watch_recursive) to watch subdirectories.
    pub fn watch(&mut self, path: impl AsRef<Path>) -> FileResult<()> {
        self.watch_with_mode(path, RecursiveMode::NonRecursive)
    }

    /// Start watching a directory recursively.
    ///
    /// All files and subdirectories under the given path will be watched.
    /// For files, this behaves the same as `watch()`.
    pub fn watch_recursive(&mut self, path: impl AsRef<Path>) -> FileResult<()> {
        self.watch_with_mode(path, RecursiveMode::Recursive)
    }

    /// Internal method to watch with a specific mode.
    fn watch_with_mode(&mut self, path: impl AsRef<Path>, mode: RecursiveMode) -> FileResult<()> {
        let path = path.as_ref();

        // Canonicalize the path for consistent tracking
        let canonical = path.canonicalize()
            .map_err(|e| FileError::from_io(e, path.to_path_buf()))?;

        if self.watched_paths.contains(&canonical) {
            // Already watching this path
            return Ok(());
        }

        self.debouncer
            .watcher()
            .watch(&canonical, mode)
            .map_err(|e| FileError::new(
                FileErrorKind::Other,
                Some(canonical.clone()),
                Some(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
            ))?;

        self.watched_paths.insert(canonical.clone());
        self.watch_modes.insert(
            canonical,
            match mode {
                RecursiveMode::Recursive => WatchMode::Recursive,
                RecursiveMode::NonRecursive => WatchMode::NonRecursive,
            },
        );

        Ok(())
    }

    /// Stop watching a file or directory.
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> FileResult<()> {
        let path = path.as_ref();

        // Try to canonicalize, but don't fail if the file was deleted
        let canonical = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                // File might be deleted, try to find it in watched paths
                // by checking if any watched path ends with this path
                let path_buf = path.to_path_buf();
                let found = self.watched_paths.iter()
                    .find(|p| p.ends_with(path) || *p == &path_buf)
                    .cloned();

                match found {
                    Some(p) => p,
                    None => return Ok(()), // Not watching this path
                }
            }
        };

        if self.watched_paths.remove(&canonical) {
            let _ = self.debouncer.watcher().unwatch(&canonical);
            self.watch_modes.remove(&canonical);
        }

        Ok(())
    }

    /// Poll for file system changes without emitting signals.
    ///
    /// Returns a list of events that have occurred since the last poll.
    /// Events are deduplicated by path.
    pub fn poll(&mut self) -> Vec<FileWatchEvent> {
        self.collect_events()
    }

    /// Process pending events and emit signals.
    ///
    /// Call this method in your event loop to receive change notifications
    /// through the `changed` signal.
    ///
    /// Returns the number of events processed.
    pub fn process(&mut self) -> usize {
        let events = self.collect_events();
        let count = events.len();

        for event in events {
            self.changed.emit(event);
        }

        count
    }

    /// Collect events from the channel.
    fn collect_events(&mut self) -> Vec<FileWatchEvent> {
        let mut events = Vec::new();

        loop {
            match self.rx.try_recv() {
                Ok(Ok(debounced_events)) => {
                    for event in debounced_events {
                        if let Some(watch_event) = self.convert_event(&event) {
                            events.push(watch_event);
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!(target: "horizon_lattice::file::watcher", "File watcher error: {}", e);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    tracing::error!(target: "horizon_lattice::file::watcher", "File watcher disconnected");
                    break;
                }
            }
        }

        // Deduplicate events by path (keep last event for each path)
        self.deduplicate_events(&mut events);

        events
    }

    /// Convert a debounced event to a FileWatchEvent.
    fn convert_event(&self, event: &DebouncedEvent) -> Option<FileWatchEvent> {
        // notify-debouncer-mini uses DebouncedEventKind::Any for all events
        // We determine the actual kind by checking if the file exists
        if event.kind != DebouncedEventKind::Any {
            return None;
        }

        let kind = if event.path.exists() {
            // Check if this is a new file (not in our watched set)
            // For non-recursive watches, we only care about explicitly watched paths
            // For recursive watches, new files in subdirectories are "created"
            if self.is_newly_created(&event.path) {
                WatchEventKind::Created
            } else {
                WatchEventKind::Modified
            }
        } else {
            WatchEventKind::Removed
        };

        Some(FileWatchEvent::new(event.path.clone(), kind))
    }

    /// Check if a path appears to be newly created.
    fn is_newly_created(&self, path: &Path) -> bool {
        // A file is considered "created" if:
        // 1. It's not in our explicitly watched set, OR
        // 2. It's inside a recursively watched directory

        if self.watched_paths.contains(path) {
            return false;
        }

        // Check if this is under a recursively watched directory
        for (watched, mode) in &self.watch_modes {
            if *mode == WatchMode::Recursive && path.starts_with(watched) && path != watched {
                // It's a new file in a recursively watched directory
                return true;
            }
        }

        false
    }

    /// Deduplicate events, keeping the last event for each path.
    fn deduplicate_events(&self, events: &mut Vec<FileWatchEvent>) {
        // Sort by path for stable deduplication
        events.sort_by(|a, b| a.path.cmp(&b.path));

        // Remove duplicates, keeping the last one (which is more recent)
        events.dedup_by(|a, b| {
            if a.path == b.path {
                // Keep the more "severe" event: Removed > Created > Modified
                let priority = |k: WatchEventKind| match k {
                    WatchEventKind::Removed => 2,
                    WatchEventKind::Created => 1,
                    WatchEventKind::Modified => 0,
                };
                if priority(a.kind) > priority(b.kind) {
                    b.kind = a.kind;
                }
                true
            } else {
                false
            }
        });
    }

    /// Get the number of watched paths.
    pub fn watched_count(&self) -> usize {
        self.watched_paths.len()
    }

    /// Get an iterator over the watched paths.
    pub fn watched_paths(&self) -> impl Iterator<Item = &Path> {
        self.watched_paths.iter().map(|p| p.as_path())
    }

    /// Check if a specific path is being watched.
    pub fn is_watching(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        if let Ok(canonical) = path.canonicalize() {
            self.watched_paths.contains(&canonical)
        } else {
            self.watched_paths.contains(&path.to_path_buf())
        }
    }

    /// Check if a path is being watched recursively.
    pub fn is_watching_recursive(&self, path: impl AsRef<Path>) -> bool {
        let path = path.as_ref();
        if let Ok(canonical) = path.canonicalize() {
            matches!(self.watch_modes.get(&canonical), Some(WatchMode::Recursive))
        } else {
            matches!(self.watch_modes.get(&path.to_path_buf()), Some(WatchMode::Recursive))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::sync::Arc;
    use parking_lot::Mutex;

    fn temp_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("Failed to create temp directory")
    }

    #[test]
    fn test_watcher_creation() {
        let watcher = FileWatcher::new();
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watcher_with_options() {
        let options = WatchOptions::new().debounce(Duration::from_millis(50));
        let watcher = FileWatcher::with_options(options);
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_watch_file() {
        let dir = temp_dir();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();

        let mut watcher = FileWatcher::new().unwrap();
        let result = watcher.watch(&file_path);
        assert!(result.is_ok());
        assert_eq!(watcher.watched_count(), 1);
        assert!(watcher.is_watching(&file_path));
    }

    #[test]
    fn test_watch_directory() {
        let dir = temp_dir();

        let mut watcher = FileWatcher::new().unwrap();
        let result = watcher.watch(dir.path());
        assert!(result.is_ok());
        assert_eq!(watcher.watched_count(), 1);
    }

    #[test]
    fn test_watch_recursive() {
        let dir = temp_dir();

        let mut watcher = FileWatcher::new().unwrap();
        let result = watcher.watch_recursive(dir.path());
        assert!(result.is_ok());
        assert!(watcher.is_watching_recursive(dir.path()));
    }

    #[test]
    fn test_unwatch() {
        let dir = temp_dir();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();

        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&file_path).unwrap();
        assert_eq!(watcher.watched_count(), 1);

        watcher.unwatch(&file_path).unwrap();
        assert_eq!(watcher.watched_count(), 0);
        assert!(!watcher.is_watching(&file_path));
    }

    #[test]
    fn test_unwatch_nonexistent() {
        let mut watcher = FileWatcher::new().unwrap();
        // Should not error when unwatching a path that wasn't being watched
        let result = watcher.unwatch("/nonexistent/path");
        assert!(result.is_ok());
    }

    #[test]
    fn test_duplicate_watch() {
        let dir = temp_dir();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "hello").unwrap();

        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&file_path).unwrap();
        watcher.watch(&file_path).unwrap(); // Should not error

        assert_eq!(watcher.watched_count(), 1);
    }

    #[test]
    fn test_poll_returns_empty_initially() {
        let mut watcher = FileWatcher::new().unwrap();
        let events = watcher.poll();
        assert!(events.is_empty());
    }

    #[test]
    fn test_signal_connection() {
        let watcher = FileWatcher::new().unwrap();
        let received = Arc::new(Mutex::new(Vec::new()));

        let received_clone = received.clone();
        watcher.changed.connect(move |event| {
            received_clone.lock().push(event.clone());
        });

        assert_eq!(watcher.changed.connection_count(), 1);
    }

    #[test]
    fn test_watch_event_kind_display() {
        assert_eq!(WatchEventKind::Created.to_string(), "created");
        assert_eq!(WatchEventKind::Modified.to_string(), "modified");
        assert_eq!(WatchEventKind::Removed.to_string(), "removed");
    }

    #[test]
    fn test_file_watch_event_helpers() {
        let event = FileWatchEvent::new(PathBuf::from("/test"), WatchEventKind::Created);
        assert!(event.is_created());
        assert!(!event.is_modified());
        assert!(!event.is_removed());

        let event = FileWatchEvent::new(PathBuf::from("/test"), WatchEventKind::Modified);
        assert!(!event.is_created());
        assert!(event.is_modified());
        assert!(!event.is_removed());

        let event = FileWatchEvent::new(PathBuf::from("/test"), WatchEventKind::Removed);
        assert!(!event.is_created());
        assert!(!event.is_modified());
        assert!(event.is_removed());
    }

    #[test]
    fn test_watched_paths_iterator() {
        let dir = temp_dir();
        let file1 = dir.path().join("file1.txt");
        let file2 = dir.path().join("file2.txt");
        fs::write(&file1, "a").unwrap();
        fs::write(&file2, "b").unwrap();

        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&file1).unwrap();
        watcher.watch(&file2).unwrap();

        let paths: Vec<_> = watcher.watched_paths().collect();
        assert_eq!(paths.len(), 2);
    }

    // Integration test that verifies file modification detection
    // Note: This test may be flaky on some systems due to timing
    #[test]
    #[ignore] // Enable manually for integration testing
    fn test_file_modification_detection() {
        let dir = temp_dir();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "initial content").unwrap();

        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&file_path).unwrap();

        // Modify the file
        std::thread::sleep(Duration::from_millis(50));
        {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(&file_path)
                .unwrap();
            writeln!(file, "new content").unwrap();
        }

        // Wait for debounce
        std::thread::sleep(Duration::from_millis(200));

        let events = watcher.poll();
        assert!(!events.is_empty(), "Expected at least one event");

        let event = &events[0];
        assert_eq!(event.path.canonicalize().unwrap(), file_path.canonicalize().unwrap());
        assert!(matches!(event.kind, WatchEventKind::Modified | WatchEventKind::Created));
    }
}
