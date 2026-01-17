//! File drop handling from external applications.
//!
//! This module provides conversion of winit file drop events into the
//! Horizon Lattice drag/drop event system.
//!
//! # External File Drops
//!
//! When a user drags files from the operating system's file manager into
//! a Horizon Lattice window, the following events are generated:
//!
//! 1. `DragEnter` - When files first enter the window
//! 2. `DragMove` - As the cursor moves within the window
//! 3. `DragLeave` - If the cursor leaves without dropping
//! 4. `Drop` - When files are dropped on a widget
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice::widget::file_drop::FileDropHandler;
//!
//! let mut handler = FileDropHandler::new();
//!
//! // In your window event handler:
//! match event {
//!     WindowEvent::HoveredFile(path) => {
//!         handler.handle_hovered_file(path);
//!     }
//!     WindowEvent::DroppedFile(path) => {
//!         if let Some(event) = handler.handle_dropped_file(path, mouse_position) {
//!             // Dispatch event to widgets
//!         }
//!     }
//!     WindowEvent::HoveredFileCancelled => {
//!         handler.handle_hover_cancelled();
//!     }
//!     _ => {}
//! }
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use horizon_lattice_render::Point;

use super::drag_drop::{DragData, DragEnterEvent, DragLeaveEvent, DragMoveEvent, DropAction, DropEvent};

/// Handler for file drop operations from external sources.
///
/// This handler tracks the state of file drops from the operating system
/// and generates appropriate drag/drop events for the widget system.
#[derive(Debug, Default)]
pub struct FileDropHandler {
    /// Files currently being hovered over the window.
    hovered_files: Vec<PathBuf>,
    /// Whether we're in a hovering state.
    is_hovering: bool,
    /// The last known position during hover (if available).
    last_position: Option<Point>,
}

impl FileDropHandler {
    /// Creates a new file drop handler.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if files are currently being hovered over the window.
    pub fn is_hovering(&self) -> bool {
        self.is_hovering
    }

    /// Returns the files currently being hovered.
    pub fn hovered_files(&self) -> &[PathBuf] {
        &self.hovered_files
    }

    /// Handles a `HoveredFile` event from winit.
    ///
    /// Call this for each file being hovered. Files are accumulated until
    /// either dropped or the hover is cancelled.
    ///
    /// Returns `Some(DragEnterEvent)` on the first file of a new hover operation.
    pub fn handle_hovered_file(&mut self, path: PathBuf, position: Point) -> Option<DragEnterEvent> {
        let is_first = !self.is_hovering;
        self.hovered_files.push(path);
        self.is_hovering = true;
        self.last_position = Some(position);

        if is_first {
            // First file - generate DragEnter event
            let data = self.create_drag_data();
            Some(DragEnterEvent::new(
                Arc::new(data),
                position,
                position,
                DropAction::COPY, // External drops are copy operations
            ))
        } else {
            None
        }
    }

    /// Updates the position during a file hover.
    ///
    /// Returns a `DragMoveEvent` if we're in a hover state.
    pub fn update_position(&mut self, position: Point) -> Option<DragMoveEvent> {
        if self.is_hovering {
            self.last_position = Some(position);
            let data = self.create_drag_data();
            Some(DragMoveEvent::new(
                Arc::new(data),
                position,
                position,
                DropAction::COPY,
                DropAction::COPY,
            ))
        } else {
            None
        }
    }

    /// Handles a `DroppedFile` event from winit.
    ///
    /// This is called when a file is actually dropped. Note that winit
    /// sends this event for each dropped file, but we generate a single
    /// drop event containing all files.
    ///
    /// Returns `Some(DropEvent)` when all files have been collected.
    pub fn handle_dropped_file(&mut self, path: PathBuf, position: Point) -> Option<DropEvent> {
        // Add this file to the collection
        if !self.hovered_files.contains(&path) {
            self.hovered_files.push(path);
        }

        // Generate the drop event with all collected files
        let data = self.create_drag_data();
        let event = DropEvent::new(
            Arc::new(data),
            position,
            position,
            DropAction::COPY,
        );

        // Reset state
        self.reset();

        Some(event)
    }

    /// Handles a `HoveredFileCancelled` event from winit.
    ///
    /// Called when the hover is cancelled (files dragged out of window).
    /// Returns a `DragLeaveEvent`.
    pub fn handle_hover_cancelled(&mut self) -> Option<DragLeaveEvent> {
        if self.is_hovering {
            self.reset();
            Some(DragLeaveEvent::new())
        } else {
            None
        }
    }

    /// Resets the handler state.
    pub fn reset(&mut self) {
        self.hovered_files.clear();
        self.is_hovering = false;
        self.last_position = None;
    }

    /// Creates `DragData` from the currently hovered files.
    fn create_drag_data(&self) -> DragData {
        DragData::from_paths(self.hovered_files.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_hover_sequence() {
        let mut handler = FileDropHandler::new();

        // First file generates DragEnter
        let event = handler.handle_hovered_file(
            PathBuf::from("/tmp/file1.txt"),
            Point::new(100.0, 100.0),
        );
        assert!(event.is_some());
        assert!(handler.is_hovering());

        // Second file doesn't generate new DragEnter
        let event = handler.handle_hovered_file(
            PathBuf::from("/tmp/file2.txt"),
            Point::new(100.0, 100.0),
        );
        assert!(event.is_none());
        assert_eq!(handler.hovered_files().len(), 2);
    }

    #[test]
    fn test_file_drop() {
        let mut handler = FileDropHandler::new();

        // Hover first
        handler.handle_hovered_file(
            PathBuf::from("/tmp/file1.txt"),
            Point::new(100.0, 100.0),
        );

        // Drop generates DropEvent
        let event = handler.handle_dropped_file(
            PathBuf::from("/tmp/file1.txt"),
            Point::new(150.0, 150.0),
        );
        assert!(event.is_some());
        let event = event.unwrap();
        assert!(event.data().has_urls());
        assert_eq!(event.data().urls().len(), 1);

        // Handler should be reset
        assert!(!handler.is_hovering());
    }

    #[test]
    fn test_hover_cancelled() {
        let mut handler = FileDropHandler::new();

        // Start hover
        handler.handle_hovered_file(
            PathBuf::from("/tmp/file.txt"),
            Point::new(100.0, 100.0),
        );
        assert!(handler.is_hovering());

        // Cancel generates DragLeave
        let event = handler.handle_hover_cancelled();
        assert!(event.is_some());
        assert!(!handler.is_hovering());
    }

    #[test]
    fn test_position_update() {
        let mut handler = FileDropHandler::new();

        // No event when not hovering
        let event = handler.update_position(Point::new(100.0, 100.0));
        assert!(event.is_none());

        // Start hover
        handler.handle_hovered_file(
            PathBuf::from("/tmp/file.txt"),
            Point::new(100.0, 100.0),
        );

        // Position update generates DragMove
        let event = handler.update_position(Point::new(150.0, 150.0));
        assert!(event.is_some());
    }
}
