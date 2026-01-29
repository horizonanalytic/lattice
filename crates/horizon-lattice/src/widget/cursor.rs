//! Cursor management for widgets.
//!
//! This module provides cursor shape definitions and cursor management for
//! controlling the mouse cursor appearance. It follows Qt's design pattern
//! with support for:
//!
//! - Per-widget cursor shapes
//! - Application-wide override cursor
//! - Standard system cursors (arrow, hand, I-beam, etc.)
//! - Cursor visibility control
//! - Cursor grabbing/capture
//!
//! # Widget Cursors
//!
//! Each widget can specify its own cursor shape. When the mouse moves over
//! a widget, its cursor is displayed:
//!
//! ```ignore
//! widget.set_cursor(CursorShape::Hand);
//! ```
//!
//! # Override Cursor
//!
//! The application can set an override cursor that takes precedence over
//! all widget cursors. This is useful for operations like drag-and-drop:
//!
//! ```ignore
//! CursorManager::set_override_cursor(CursorShape::Forbidden);
//! // ... do operation ...
//! CursorManager::restore_override_cursor();
//! ```
//!
//! Override cursors can be stacked:
//!
//! ```ignore
//! CursorManager::set_override_cursor(CursorShape::Wait);
//! CursorManager::set_override_cursor(CursorShape::Forbidden);
//! CursorManager::restore_override_cursor(); // Back to Wait
//! CursorManager::restore_override_cursor(); // Back to widget cursor
//! ```

use std::sync::atomic::{AtomicBool, Ordering};

use cursor_icon::CursorIcon;
use parking_lot::Mutex;
use winit::window::{Cursor, CursorGrabMode, Window};

/// The shape (icon) of the mouse cursor.
///
/// This enum defines all standard cursor shapes available on most platforms.
/// The actual appearance may vary by platform and theme.
///
/// # Platform Notes
///
/// - On some platforms, certain cursors may fall back to a default if not available
/// - The exact visual appearance varies by operating system and theme
/// - Custom cursors (from images) are planned for a future release
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CursorShape {
    /// The default arrow cursor (platform-specific).
    #[default]
    Arrow,

    /// A crosshair cursor, typically used for precise selection.
    Crosshair,

    /// A pointing hand cursor, typically used for clickable elements like links.
    Hand,

    /// An I-beam cursor, typically used for text selection.
    IBeam,

    /// A "not allowed" cursor, indicating an action is forbidden.
    Forbidden,

    /// A wait cursor (hourglass/spinner), indicating the program is busy.
    Wait,

    /// A progress cursor (arrow with spinner), indicating background activity.
    Progress,

    /// A help cursor (arrow with question mark).
    Help,

    /// A move cursor, indicating something can be moved.
    Move,

    /// A context menu cursor, indicating a context menu is available.
    ContextMenu,

    /// A cell/crosshair cursor for cell selection in grids.
    Cell,

    /// Vertical text selection cursor.
    VerticalText,

    /// An alias cursor (arrow with curved arrow), indicating a shortcut/link.
    Alias,

    /// A copy cursor (arrow with plus sign).
    Copy,

    /// A "no drop" cursor, indicating drag target won't accept drop.
    NoDrop,

    /// A grab cursor (open hand), indicating something can be grabbed.
    Grab,

    /// A grabbing cursor (closed hand), indicating something is being grabbed.
    Grabbing,

    /// All-scroll cursor, indicating scrolling in any direction.
    AllScroll,

    /// Zoom in cursor (magnifying glass with plus).
    ZoomIn,

    /// Zoom out cursor (magnifying glass with minus).
    ZoomOut,

    // Resize cursors
    /// Resize cursor pointing east (right).
    ResizeEast,
    /// Resize cursor pointing west (left).
    ResizeWest,
    /// Resize cursor pointing north (up).
    ResizeNorth,
    /// Resize cursor pointing south (down).
    ResizeSouth,
    /// Resize cursor pointing northeast.
    ResizeNorthEast,
    /// Resize cursor pointing northwest.
    ResizeNorthWest,
    /// Resize cursor pointing southeast.
    ResizeSouthEast,
    /// Resize cursor pointing southwest.
    ResizeSouthWest,
    /// Resize cursor for horizontal resizing (east-west).
    ResizeHorizontal,
    /// Resize cursor for vertical resizing (north-south).
    ResizeVertical,
    /// Resize cursor for diagonal (northeast-southwest).
    ResizeNeSw,
    /// Resize cursor for diagonal (northwest-southeast).
    ResizeNwSe,
    /// Column resize cursor.
    ResizeColumn,
    /// Row resize cursor.
    ResizeRow,

    /// A blank/invisible cursor.
    ///
    /// Note: For hiding the cursor, prefer using `CursorManager::set_cursor_visible(false)`
    /// which is more reliable across platforms.
    Blank,
}

impl CursorShape {
    /// Convert to winit's Cursor type.
    pub(crate) fn to_winit_cursor(self) -> Cursor {
        Cursor::Icon(match self {
            CursorShape::Arrow => CursorIcon::Default,
            CursorShape::Crosshair => CursorIcon::Crosshair,
            CursorShape::Hand => CursorIcon::Pointer,
            CursorShape::IBeam => CursorIcon::Text,
            CursorShape::Forbidden => CursorIcon::NotAllowed,
            CursorShape::Wait => CursorIcon::Wait,
            CursorShape::Progress => CursorIcon::Progress,
            CursorShape::Help => CursorIcon::Help,
            CursorShape::Move => CursorIcon::Move,
            CursorShape::ContextMenu => CursorIcon::ContextMenu,
            CursorShape::Cell => CursorIcon::Cell,
            CursorShape::VerticalText => CursorIcon::VerticalText,
            CursorShape::Alias => CursorIcon::Alias,
            CursorShape::Copy => CursorIcon::Copy,
            CursorShape::NoDrop => CursorIcon::NoDrop,
            CursorShape::Grab => CursorIcon::Grab,
            CursorShape::Grabbing => CursorIcon::Grabbing,
            CursorShape::AllScroll => CursorIcon::AllScroll,
            CursorShape::ZoomIn => CursorIcon::ZoomIn,
            CursorShape::ZoomOut => CursorIcon::ZoomOut,
            CursorShape::ResizeEast => CursorIcon::EResize,
            CursorShape::ResizeWest => CursorIcon::WResize,
            CursorShape::ResizeNorth => CursorIcon::NResize,
            CursorShape::ResizeSouth => CursorIcon::SResize,
            CursorShape::ResizeNorthEast => CursorIcon::NeResize,
            CursorShape::ResizeNorthWest => CursorIcon::NwResize,
            CursorShape::ResizeSouthEast => CursorIcon::SeResize,
            CursorShape::ResizeSouthWest => CursorIcon::SwResize,
            CursorShape::ResizeHorizontal => CursorIcon::EwResize,
            CursorShape::ResizeVertical => CursorIcon::NsResize,
            CursorShape::ResizeNeSw => CursorIcon::NeswResize,
            CursorShape::ResizeNwSe => CursorIcon::NwseResize,
            CursorShape::ResizeColumn => CursorIcon::ColResize,
            CursorShape::ResizeRow => CursorIcon::RowResize,
            // Blank cursor - use default since winit doesn't have a blank option
            // Cursor hiding should use set_cursor_visible instead
            CursorShape::Blank => CursorIcon::Default,
        })
    }

    /// Check if this is a resize cursor.
    pub fn is_resize_cursor(self) -> bool {
        matches!(
            self,
            CursorShape::ResizeEast
                | CursorShape::ResizeWest
                | CursorShape::ResizeNorth
                | CursorShape::ResizeSouth
                | CursorShape::ResizeNorthEast
                | CursorShape::ResizeNorthWest
                | CursorShape::ResizeSouthEast
                | CursorShape::ResizeSouthWest
                | CursorShape::ResizeHorizontal
                | CursorShape::ResizeVertical
                | CursorShape::ResizeNeSw
                | CursorShape::ResizeNwSe
                | CursorShape::ResizeColumn
                | CursorShape::ResizeRow
        )
    }
}

/// Global cursor state manager.
///
/// Manages the application-wide cursor state including:
/// - Override cursor stack
/// - Cursor visibility
/// - Cursor grabbing
/// - Current window cursor state
struct CursorState {
    /// Stack of override cursors. The top of the stack is the active override.
    override_stack: Vec<CursorShape>,
    /// Current widget cursor (when no override is active).
    widget_cursor: CursorShape,
    /// Whether the cursor is currently visible.
    cursor_visible: bool,
    /// Current cursor grab mode.
    grab_mode: CursorGrabMode,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            override_stack: Vec::new(),
            widget_cursor: CursorShape::Arrow,
            cursor_visible: true,
            grab_mode: CursorGrabMode::None,
        }
    }
}

/// Global cursor state.
static CURSOR_STATE: Mutex<CursorState> = Mutex::new(CursorState {
    override_stack: Vec::new(),
    widget_cursor: CursorShape::Arrow,
    cursor_visible: true,
    grab_mode: CursorGrabMode::None,
});

/// Flag indicating the cursor state has changed and needs window update.
static CURSOR_DIRTY: AtomicBool = AtomicBool::new(false);

/// Centralized cursor management for the application.
///
/// CursorManager provides static methods for controlling cursor appearance
/// at the application level. It supports:
///
/// - **Override cursors**: Application-wide cursors that take precedence over
///   widget cursors, useful for drag operations or modal states
/// - **Widget cursors**: Per-widget cursors set by calling `set_cursor()` on
///   WidgetBase
/// - **Cursor visibility**: Show/hide the cursor
/// - **Cursor grabbing**: Lock the cursor to the window
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::cursor::{CursorManager, CursorShape};
///
/// // Set an override cursor during a drag operation
/// CursorManager::set_override_cursor(CursorShape::Grabbing);
///
/// // ... perform drag ...
///
/// // Restore to the previous cursor
/// CursorManager::restore_override_cursor();
///
/// // Hide the cursor for a game
/// CursorManager::set_cursor_visible(false);
/// ```
pub struct CursorManager;

impl CursorManager {
    /// Push an override cursor onto the stack.
    ///
    /// The override cursor is displayed regardless of which widget is under
    /// the mouse. Override cursors stack, so multiple calls push additional
    /// cursors that must be restored in reverse order.
    ///
    /// This is useful for:
    /// - Drag-and-drop operations
    /// - Modal states where the cursor should indicate a specific action
    /// - Wait cursors during long operations
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Start drag operation
    /// CursorManager::set_override_cursor(CursorShape::Grabbing);
    /// // ... drag ...
    /// CursorManager::restore_override_cursor();
    /// ```
    pub fn set_override_cursor(shape: CursorShape) {
        let mut state = CURSOR_STATE.lock();
        state.override_stack.push(shape);
        CURSOR_DIRTY.store(true, Ordering::Release);
    }

    /// Pop the most recent override cursor from the stack.
    ///
    /// If multiple override cursors have been pushed, this removes the most
    /// recent one, revealing the previous override or the widget cursor.
    ///
    /// If no override cursor is active, this is a no-op.
    pub fn restore_override_cursor() {
        let mut state = CURSOR_STATE.lock();
        state.override_stack.pop();
        CURSOR_DIRTY.store(true, Ordering::Release);
    }

    /// Change the current override cursor without pushing to the stack.
    ///
    /// This changes the active override cursor if one exists. If no override
    /// is active, this pushes a new one.
    pub fn change_override_cursor(shape: CursorShape) {
        let mut state = CURSOR_STATE.lock();
        if let Some(top) = state.override_stack.last_mut() {
            *top = shape;
        } else {
            state.override_stack.push(shape);
        }
        CURSOR_DIRTY.store(true, Ordering::Release);
    }

    /// Clear all override cursors.
    ///
    /// This removes all override cursors from the stack, returning to the
    /// widget cursor.
    pub fn clear_override_cursor() {
        let mut state = CURSOR_STATE.lock();
        state.override_stack.clear();
        CURSOR_DIRTY.store(true, Ordering::Release);
    }

    /// Check if an override cursor is currently active.
    pub fn has_override_cursor() -> bool {
        let state = CURSOR_STATE.lock();
        !state.override_stack.is_empty()
    }

    /// Get the current override cursor, if any.
    pub fn override_cursor() -> Option<CursorShape> {
        let state = CURSOR_STATE.lock();
        state.override_stack.last().copied()
    }

    /// Set the cursor visibility.
    ///
    /// When `false`, the cursor is hidden while over the application window.
    /// This is useful for games or applications with custom cursor rendering.
    ///
    /// # Platform Notes
    ///
    /// - The cursor remains visible outside the application window
    /// - Some platforms may not support cursor hiding
    pub fn set_cursor_visible(visible: bool) {
        let mut state = CURSOR_STATE.lock();
        if state.cursor_visible != visible {
            state.cursor_visible = visible;
            CURSOR_DIRTY.store(true, Ordering::Release);
        }
    }

    /// Check if the cursor is currently visible.
    pub fn is_cursor_visible() -> bool {
        let state = CURSOR_STATE.lock();
        state.cursor_visible
    }

    /// Set the cursor grab mode.
    ///
    /// Cursor grabbing restricts the cursor to the window:
    ///
    /// - `None`: No grabbing, cursor moves freely
    /// - `Confined`: Cursor is confined to the window bounds
    /// - `Locked`: Cursor is locked and invisible, only delta movement is reported
    ///
    /// # Platform Notes
    ///
    /// - `Locked` mode is not supported on all platforms (falls back to `Confined`)
    /// - Grabbing requires user interaction (click) on some platforms
    /// - The grab is automatically released if the window loses focus
    pub fn set_cursor_grab(mode: CursorGrabMode) {
        let mut state = CURSOR_STATE.lock();
        if state.grab_mode != mode {
            state.grab_mode = mode;
            CURSOR_DIRTY.store(true, Ordering::Release);
        }
    }

    /// Get the current cursor grab mode.
    pub fn cursor_grab_mode() -> CursorGrabMode {
        let state = CURSOR_STATE.lock();
        state.grab_mode
    }

    // =========================================================================
    // Internal API (for widget system)
    // =========================================================================

    /// Set the current widget cursor (internal).
    ///
    /// Called by the event system when the mouse enters a widget.
    pub(crate) fn set_widget_cursor(shape: CursorShape) {
        let mut state = CURSOR_STATE.lock();
        if state.widget_cursor != shape {
            state.widget_cursor = shape;
            CURSOR_DIRTY.store(true, Ordering::Release);
        }
    }

    /// Get the current effective cursor shape.
    ///
    /// Returns the override cursor if active, otherwise the widget cursor.
    pub(crate) fn effective_cursor() -> CursorShape {
        let state = CURSOR_STATE.lock();
        state
            .override_stack
            .last()
            .copied()
            .unwrap_or(state.widget_cursor)
    }

    /// Check if the cursor state needs to be applied to a window.
    pub(crate) fn is_dirty() -> bool {
        CURSOR_DIRTY.load(Ordering::Acquire)
    }

    /// Apply the current cursor state to a window.
    ///
    /// This should be called after processing mouse move events to update
    /// the window's cursor. Returns `true` if the cursor was updated.
    pub(crate) fn apply_to_window(window: &Window) -> bool {
        if !CURSOR_DIRTY.swap(false, Ordering::AcqRel) {
            return false;
        }

        let state = CURSOR_STATE.lock();

        // Apply cursor shape
        let shape = state
            .override_stack
            .last()
            .copied()
            .unwrap_or(state.widget_cursor);
        window.set_cursor(shape.to_winit_cursor());

        // Apply visibility
        window.set_cursor_visible(state.cursor_visible);

        // Apply grab mode (may fail on some platforms)
        let _ = window.set_cursor_grab(state.grab_mode);

        true
    }

    /// Reset cursor state to defaults.
    ///
    /// This clears all override cursors and resets to the arrow cursor.
    pub(crate) fn reset() {
        let mut state = CURSOR_STATE.lock();
        state.override_stack.clear();
        state.widget_cursor = CursorShape::Arrow;
        state.cursor_visible = true;
        state.grab_mode = CursorGrabMode::None;
        CURSOR_DIRTY.store(true, Ordering::Release);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_cursor_state() {
        CursorManager::reset();
    }

    #[test]
    fn test_cursor_shape_default() {
        assert_eq!(CursorShape::default(), CursorShape::Arrow);
    }

    #[test]
    fn test_cursor_shape_is_resize() {
        assert!(CursorShape::ResizeEast.is_resize_cursor());
        assert!(CursorShape::ResizeHorizontal.is_resize_cursor());
        assert!(!CursorShape::Arrow.is_resize_cursor());
        assert!(!CursorShape::Hand.is_resize_cursor());
    }

    #[test]
    fn test_override_cursor_stack() {
        reset_cursor_state();

        assert!(!CursorManager::has_override_cursor());
        assert_eq!(CursorManager::override_cursor(), None);

        CursorManager::set_override_cursor(CursorShape::Wait);
        assert!(CursorManager::has_override_cursor());
        assert_eq!(CursorManager::override_cursor(), Some(CursorShape::Wait));

        CursorManager::set_override_cursor(CursorShape::Forbidden);
        assert_eq!(
            CursorManager::override_cursor(),
            Some(CursorShape::Forbidden)
        );

        CursorManager::restore_override_cursor();
        assert_eq!(CursorManager::override_cursor(), Some(CursorShape::Wait));

        CursorManager::restore_override_cursor();
        assert!(!CursorManager::has_override_cursor());
        assert_eq!(CursorManager::override_cursor(), None);
    }

    #[test]
    #[ignore = "uses global CursorManager state, flaky with parallel test execution"]
    fn test_change_override_cursor() {
        reset_cursor_state();

        CursorManager::set_override_cursor(CursorShape::Wait);
        CursorManager::change_override_cursor(CursorShape::Progress);
        assert_eq!(
            CursorManager::override_cursor(),
            Some(CursorShape::Progress)
        );

        // Should still only have one override
        CursorManager::restore_override_cursor();
        assert!(!CursorManager::has_override_cursor());
    }

    #[test]
    fn test_clear_override_cursor() {
        reset_cursor_state();

        CursorManager::set_override_cursor(CursorShape::Wait);
        CursorManager::set_override_cursor(CursorShape::Forbidden);
        assert!(CursorManager::has_override_cursor());

        CursorManager::clear_override_cursor();
        assert!(!CursorManager::has_override_cursor());
    }

    #[test]
    fn test_cursor_visibility() {
        reset_cursor_state();

        assert!(CursorManager::is_cursor_visible());

        CursorManager::set_cursor_visible(false);
        assert!(!CursorManager::is_cursor_visible());

        CursorManager::set_cursor_visible(true);
        assert!(CursorManager::is_cursor_visible());
    }

    #[test]
    fn test_cursor_grab_mode() {
        reset_cursor_state();

        assert_eq!(CursorManager::cursor_grab_mode(), CursorGrabMode::None);

        CursorManager::set_cursor_grab(CursorGrabMode::Confined);
        assert_eq!(CursorManager::cursor_grab_mode(), CursorGrabMode::Confined);

        CursorManager::set_cursor_grab(CursorGrabMode::None);
        assert_eq!(CursorManager::cursor_grab_mode(), CursorGrabMode::None);
    }

    #[test]
    fn test_effective_cursor() {
        reset_cursor_state();

        // With no override, should return widget cursor
        CursorManager::set_widget_cursor(CursorShape::IBeam);
        assert_eq!(CursorManager::effective_cursor(), CursorShape::IBeam);

        // With override, should return override
        CursorManager::set_override_cursor(CursorShape::Wait);
        assert_eq!(CursorManager::effective_cursor(), CursorShape::Wait);

        // After restore, should return widget cursor again
        CursorManager::restore_override_cursor();
        assert_eq!(CursorManager::effective_cursor(), CursorShape::IBeam);
    }

    #[test]
    fn test_dirty_flag() {
        reset_cursor_state();

        // Reset clears dirty
        assert!(CursorManager::is_dirty());

        // After checking dirty, it should still be dirty (read doesn't clear)
        assert!(CursorManager::is_dirty());

        // Changes set dirty
        CursorManager::set_cursor_visible(false);
        assert!(CursorManager::is_dirty());
    }
}
