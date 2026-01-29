//! File watching for stylesheet hot-reload.

use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{DebouncedEventKind, Debouncer, new_debouncer};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use crate::resolve::StyleEngine;
use crate::rules::{StylePriority, StyleSheet};
use crate::{Error, Result};

/// Event indicating a stylesheet file changed.
#[derive(Debug, Clone)]
pub struct StylesheetChangeEvent {
    /// Path to the changed file.
    pub path: PathBuf,
    /// Type of change.
    pub kind: ChangeKind,
}

/// Type of file change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// File was modified.
    Modified,
    /// File was created.
    Created,
    /// File was removed.
    Removed,
}

/// Watches stylesheet files for changes.
///
/// # Example
///
/// ```ignore
/// let mut watcher = StylesheetWatcher::new()?;
/// watcher.watch("styles/app.css")?;
///
/// // In your event loop:
/// let changes = watcher.poll();
/// if !changes.is_empty() {
///     watcher.apply_changes(&mut style_engine, &changes)?;
/// }
/// ```
pub struct StylesheetWatcher {
    debouncer: Debouncer<RecommendedWatcher>,
    rx: Receiver<std::result::Result<Vec<notify_debouncer_mini::DebouncedEvent>, notify::Error>>,
    watched_paths: HashSet<PathBuf>,
}

impl StylesheetWatcher {
    /// Create a new stylesheet watcher.
    pub fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let debouncer = new_debouncer(Duration::from_millis(100), tx)
            .map_err(|e| Error::HotReload(e.to_string()))?;

        Ok(Self {
            debouncer,
            rx,
            watched_paths: HashSet::new(),
        })
    }

    /// Start watching a stylesheet file.
    pub fn watch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path
            .as_ref()
            .canonicalize()
            .map_err(|e| Error::io(path.as_ref(), e))?;

        if !self.watched_paths.contains(&path) {
            self.debouncer
                .watcher()
                .watch(&path, RecursiveMode::NonRecursive)
                .map_err(|e| Error::HotReload(e.to_string()))?;

            self.watched_paths.insert(path.clone());
            tracing::info!("Watching stylesheet: {}", path.display());
        }

        Ok(())
    }

    /// Stop watching a stylesheet file.
    pub fn unwatch(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = match path.as_ref().canonicalize() {
            Ok(p) => p,
            Err(_) => return Ok(()), // File doesn't exist, nothing to unwatch
        };

        if self.watched_paths.remove(&path) {
            let _ = self.debouncer.watcher().unwatch(&path);
            tracing::info!("Stopped watching stylesheet: {}", path.display());
        }

        Ok(())
    }

    /// Poll for stylesheet changes.
    ///
    /// Returns a list of changed stylesheets. Call this in your event loop.
    pub fn poll(&mut self) -> Vec<StylesheetChangeEvent> {
        let mut changes = vec![];

        loop {
            match self.rx.try_recv() {
                Ok(Ok(events)) => {
                    for event in events {
                        if event.kind == DebouncedEventKind::Any {
                            let kind = if event.path.exists() {
                                ChangeKind::Modified
                            } else {
                                ChangeKind::Removed
                            };

                            // Only report changes for files we're watching
                            if self.watched_paths.contains(&event.path) {
                                changes.push(StylesheetChangeEvent {
                                    path: event.path,
                                    kind,
                                });
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    tracing::warn!("File watcher error: {}", e);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    tracing::error!("File watcher disconnected");
                    break;
                }
            }
        }

        // Deduplicate changes (same file may have multiple events)
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        changes.dedup_by(|a, b| a.path == b.path);

        changes
    }

    /// Apply changes to the style engine.
    ///
    /// This reloads modified stylesheets and removes deleted ones.
    pub fn apply_changes(
        &self,
        engine: &mut StyleEngine,
        changes: &[StylesheetChangeEvent],
    ) -> Result<()> {
        for change in changes {
            match change.kind {
                ChangeKind::Modified | ChangeKind::Created => {
                    tracing::info!("Reloading stylesheet: {}", change.path.display());

                    // Remove old version
                    engine.remove_stylesheet_by_path(&change.path);

                    // Load new version
                    match StyleSheet::from_file(&change.path, StylePriority::Application) {
                        Ok(sheet) => {
                            engine.add_stylesheet(sheet);
                            tracing::info!(
                                "Reloaded stylesheet: {} ({} rules)",
                                change.path.display(),
                                engine.rule_count()
                            );
                        }
                        Err(e) => {
                            tracing::error!(
                                "Failed to reload stylesheet {}: {}",
                                change.path.display(),
                                e
                            );
                        }
                    }
                }
                ChangeKind::Removed => {
                    tracing::info!("Stylesheet removed: {}", change.path.display());
                    engine.remove_stylesheet_by_path(&change.path);
                }
            }
        }

        Ok(())
    }

    /// Get the number of watched files.
    pub fn watched_count(&self) -> usize {
        self.watched_paths.len()
    }

    /// Get the watched paths.
    pub fn watched_paths(&self) -> impl Iterator<Item = &Path> {
        self.watched_paths.iter().map(|p| p.as_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::tempdir;

    #[test]
    fn watcher_creation() {
        let watcher = StylesheetWatcher::new();
        assert!(watcher.is_ok());
    }

    #[test]
    fn watch_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.css");

        // Create test file
        fs::write(&file_path, "Button { color: red; }").unwrap();

        let mut watcher = StylesheetWatcher::new().unwrap();
        let result = watcher.watch(&file_path);
        assert!(result.is_ok());
        assert_eq!(watcher.watched_count(), 1);
    }

    #[test]
    fn unwatch_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.css");
        fs::write(&file_path, "Button { color: red; }").unwrap();

        let mut watcher = StylesheetWatcher::new().unwrap();
        watcher.watch(&file_path).unwrap();
        assert_eq!(watcher.watched_count(), 1);

        watcher.unwatch(&file_path).unwrap();
        assert_eq!(watcher.watched_count(), 0);
    }
}
