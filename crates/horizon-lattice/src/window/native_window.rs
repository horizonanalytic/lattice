//! Native window wrapper.
//!
//! This module provides `NativeWindow`, a wrapper around the platform's
//! native window (winit::Window) with additional Horizon Lattice functionality.

use std::sync::Arc;

use winit::dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Fullscreen, Window, WindowId, WindowLevel};

use super::window_config::WindowConfig;
use super::window_icon::WindowIcon;
use super::window_type::WindowType;

/// Unique identifier for a native window.
///
/// This wraps winit's `WindowId` and provides additional functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeWindowId(WindowId);

impl NativeWindowId {
    /// Create from a winit WindowId.
    pub fn from_winit(id: WindowId) -> Self {
        Self(id)
    }

    /// Get the underlying winit WindowId.
    pub fn winit_id(&self) -> WindowId {
        self.0
    }
}

impl From<WindowId> for NativeWindowId {
    fn from(id: WindowId) -> Self {
        Self(id)
    }
}

impl From<NativeWindowId> for WindowId {
    fn from(id: NativeWindowId) -> Self {
        id.0
    }
}

/// A native platform window.
///
/// `NativeWindow` wraps a winit window and provides additional functionality
/// for the Horizon Lattice framework, including:
///
/// - High-level window management methods
/// - Window type tracking
/// - Integration with the window manager
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::{NativeWindow, WindowConfig, WindowType};
///
/// // Create a window
/// let config = WindowConfig::new("My Window")
///     .with_type(WindowType::Normal)
///     .with_size(800, 600);
///
/// let window = NativeWindow::create(event_loop, config)?;
///
/// // Window operations
/// window.set_title("New Title");
/// window.set_visible(true);
/// window.request_redraw();
/// ```
pub struct NativeWindow {
    /// The underlying winit window.
    window: Arc<Window>,
    /// The window type.
    window_type: WindowType,
    /// The original configuration.
    title: String,
}

impl NativeWindow {
    /// Create a new native window from a configuration.
    ///
    /// This must be called from within the event loop (typically in `resumed()`).
    ///
    /// # Arguments
    ///
    /// * `event_loop` - The active event loop
    /// * `config` - Window configuration
    ///
    /// # Errors
    ///
    /// Returns an error if window creation fails.
    pub fn create(
        event_loop: &ActiveEventLoop,
        config: WindowConfig,
    ) -> Result<Self, NativeWindowError> {
        let attrs = config.to_window_attributes();
        let window = event_loop
            .create_window(attrs)
            .map_err(|e| NativeWindowError::CreationFailed(e.to_string()))?;

        Ok(Self {
            window: Arc::new(window),
            window_type: config.window_type(),
            title: config.title().to_string(),
        })
    }

    /// Get the unique window identifier.
    pub fn id(&self) -> NativeWindowId {
        NativeWindowId(self.window.id())
    }

    /// Get the winit window ID.
    pub fn winit_id(&self) -> WindowId {
        self.window.id()
    }

    /// Get the window type.
    pub fn window_type(&self) -> WindowType {
        self.window_type
    }

    /// Get a reference to the underlying winit window.
    ///
    /// This is provided for advanced use cases that need direct access
    /// to the winit window.
    pub fn winit_window(&self) -> &Window {
        &self.window
    }

    /// Get an Arc reference to the underlying winit window.
    ///
    /// This is useful when you need to share the window across threads
    /// or store it for later use (e.g., for surface creation).
    pub fn winit_window_arc(&self) -> Arc<Window> {
        Arc::clone(&self.window)
    }

    // =========================================================================
    // Title
    // =========================================================================

    /// Get the window title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the window title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        let title = title.into();
        self.window.set_title(&title);
        self.title = title;
    }

    // =========================================================================
    // Visibility
    // =========================================================================

    /// Check if the window is visible.
    pub fn is_visible(&self) -> bool {
        self.window.is_visible().unwrap_or(true)
    }

    /// Set window visibility.
    pub fn set_visible(&self, visible: bool) {
        self.window.set_visible(visible);
    }

    /// Show the window.
    pub fn show(&self) {
        self.window.set_visible(true);
    }

    /// Hide the window.
    pub fn hide(&self) {
        self.window.set_visible(false);
    }

    // =========================================================================
    // Size and Position
    // =========================================================================

    /// Get the inner size of the window in physical pixels.
    pub fn inner_size(&self) -> PhysicalSize<u32> {
        self.window.inner_size()
    }

    /// Get the inner size of the window in logical pixels.
    pub fn inner_size_logical(&self) -> LogicalSize<f64> {
        self.window.inner_size().to_logical(self.scale_factor())
    }

    /// Get the outer size of the window (including decorations) in physical pixels.
    pub fn outer_size(&self) -> PhysicalSize<u32> {
        self.window.outer_size()
    }

    /// Request a new inner size for the window.
    ///
    /// Returns the new size if the request was immediately fulfilled,
    /// or `None` if the request will be processed asynchronously.
    pub fn request_inner_size(&self, size: PhysicalSize<u32>) -> Option<PhysicalSize<u32>> {
        self.window.request_inner_size(size)
    }

    /// Request a new inner size in logical pixels.
    pub fn request_inner_size_logical(&self, width: f64, height: f64) -> Option<PhysicalSize<u32>> {
        let physical: PhysicalSize<u32> =
            LogicalSize::new(width, height).to_physical(self.scale_factor());
        self.window.request_inner_size(physical)
    }

    /// Set the minimum inner size.
    pub fn set_min_inner_size(&self, size: Option<PhysicalSize<u32>>) {
        self.window.set_min_inner_size(size);
    }

    /// Set the maximum inner size.
    pub fn set_max_inner_size(&self, size: Option<PhysicalSize<u32>>) {
        self.window.set_max_inner_size(size);
    }

    /// Get the outer position of the window in physical pixels.
    pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, NativeWindowError> {
        self.window
            .outer_position()
            .map_err(|_| NativeWindowError::PositionUnavailable)
    }

    /// Set the outer position of the window.
    pub fn set_outer_position(&self, position: PhysicalPosition<i32>) {
        self.window.set_outer_position(position);
    }

    /// Set the outer position in logical pixels.
    pub fn set_outer_position_logical(&self, x: f64, y: f64) {
        let physical: PhysicalPosition<i32> =
            LogicalPosition::new(x, y).to_physical(self.scale_factor());
        self.window.set_outer_position(physical);
    }

    // =========================================================================
    // Window State
    // =========================================================================

    /// Check if the window is minimized.
    pub fn is_minimized(&self) -> Option<bool> {
        self.window.is_minimized()
    }

    /// Set the window minimized state.
    pub fn set_minimized(&self, minimized: bool) {
        self.window.set_minimized(minimized);
    }

    /// Minimize the window.
    pub fn minimize(&self) {
        self.window.set_minimized(true);
    }

    /// Check if the window is maximized.
    pub fn is_maximized(&self) -> bool {
        self.window.is_maximized()
    }

    /// Set the window maximized state.
    pub fn set_maximized(&self, maximized: bool) {
        self.window.set_maximized(maximized);
    }

    /// Maximize the window.
    pub fn maximize(&self) {
        self.window.set_maximized(true);
    }

    /// Restore the window from minimized/maximized state.
    pub fn restore(&self) {
        self.window.set_minimized(false);
        self.window.set_maximized(false);
    }

    /// Get the current fullscreen state.
    pub fn fullscreen(&self) -> Option<Fullscreen> {
        self.window.fullscreen()
    }

    /// Set fullscreen mode.
    ///
    /// Pass `Some(Fullscreen::Borderless(None))` for borderless fullscreen
    /// on the current monitor, or `None` to exit fullscreen.
    pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
        self.window.set_fullscreen(fullscreen);
    }

    /// Enter borderless fullscreen mode.
    pub fn enter_fullscreen(&self) {
        self.window.set_fullscreen(Some(Fullscreen::Borderless(None)));
    }

    /// Exit fullscreen mode.
    pub fn exit_fullscreen(&self) {
        self.window.set_fullscreen(None);
    }

    /// Check if the window is in fullscreen mode.
    pub fn is_fullscreen(&self) -> bool {
        self.window.fullscreen().is_some()
    }

    // =========================================================================
    // Window Attributes
    // =========================================================================

    /// Check if the window is resizable.
    pub fn is_resizable(&self) -> bool {
        self.window.is_resizable()
    }

    /// Set whether the window is resizable.
    pub fn set_resizable(&self, resizable: bool) {
        self.window.set_resizable(resizable);
    }

    /// Check if the window has decorations.
    pub fn is_decorated(&self) -> bool {
        self.window.is_decorated()
    }

    /// Set whether the window has decorations.
    pub fn set_decorations(&self, decorations: bool) {
        self.window.set_decorations(decorations);
    }

    /// Set the window level (z-ordering).
    pub fn set_window_level(&self, level: WindowLevel) {
        self.window.set_window_level(level);
    }

    /// Set the window icon.
    pub fn set_icon(&self, icon: Option<WindowIcon>) {
        let winit_icon = icon.and_then(|i| i.to_winit_icon().ok());
        self.window.set_window_icon(winit_icon);
    }

    // =========================================================================
    // Scale Factor
    // =========================================================================

    /// Get the window's scale factor.
    ///
    /// This is typically 1.0 for standard displays and 2.0 for HiDPI displays.
    pub fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }

    // =========================================================================
    // Focus
    // =========================================================================

    /// Check if the window has focus.
    pub fn has_focus(&self) -> bool {
        self.window.has_focus()
    }

    /// Request focus for the window.
    pub fn focus(&self) {
        self.window.focus_window();
    }

    // =========================================================================
    // Rendering
    // =========================================================================

    /// Request a redraw of the window content.
    ///
    /// This schedules a `RedrawRequested` event for the window.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Mark the window content as needing to be redrawn before the next
    /// display refresh.
    pub fn pre_present_notify(&self) {
        self.window.pre_present_notify();
    }

    // =========================================================================
    // Drag Operations
    // =========================================================================

    /// Start a window drag operation.
    ///
    /// This allows the user to move the window by dragging from anywhere,
    /// not just the title bar. Typically called in response to a mouse press.
    pub fn drag_window(&self) -> Result<(), NativeWindowError> {
        self.window
            .drag_window()
            .map_err(|_| NativeWindowError::DragFailed)
    }

    /// Start a window resize operation.
    ///
    /// This allows the user to resize the window by dragging from the
    /// current cursor position.
    pub fn drag_resize_window(
        &self,
        direction: winit::window::ResizeDirection,
    ) -> Result<(), NativeWindowError> {
        self.window
            .drag_resize_window(direction)
            .map_err(|_| NativeWindowError::ResizeFailed)
    }

    // =========================================================================
    // Monitor
    // =========================================================================

    /// Get the current monitor that the window is on.
    pub fn current_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        self.window.current_monitor()
    }

    /// Get all available monitors.
    pub fn available_monitors(&self) -> impl Iterator<Item = winit::monitor::MonitorHandle> {
        self.window.available_monitors()
    }

    /// Get the primary monitor.
    pub fn primary_monitor(&self) -> Option<winit::monitor::MonitorHandle> {
        self.window.primary_monitor()
    }
}

impl std::fmt::Debug for NativeWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeWindow")
            .field("id", &self.id())
            .field("title", &self.title)
            .field("window_type", &self.window_type)
            .field("size", &self.inner_size())
            .finish()
    }
}

/// Error type for native window operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeWindowError {
    /// Window creation failed.
    CreationFailed(String),
    /// Window position is unavailable (e.g., on Wayland).
    PositionUnavailable,
    /// Window drag operation failed.
    DragFailed,
    /// Window resize operation failed.
    ResizeFailed,
}

impl std::fmt::Display for NativeWindowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NativeWindowError::CreationFailed(msg) => {
                write!(f, "window creation failed: {}", msg)
            }
            NativeWindowError::PositionUnavailable => {
                write!(f, "window position is unavailable")
            }
            NativeWindowError::DragFailed => {
                write!(f, "window drag operation failed")
            }
            NativeWindowError::ResizeFailed => {
                write!(f, "window resize operation failed")
            }
        }
    }
}

impl std::error::Error for NativeWindowError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_window_id_conversion() {
        // We can't create a real WindowId in tests, but we can test the type structure
        // This test just verifies the types compile correctly
    }

    #[test]
    fn test_native_window_error_display() {
        let err = NativeWindowError::CreationFailed("test error".to_string());
        assert!(format!("{}", err).contains("test error"));

        let err = NativeWindowError::PositionUnavailable;
        assert!(format!("{}", err).contains("position"));

        let err = NativeWindowError::DragFailed;
        assert!(format!("{}", err).contains("drag"));

        let err = NativeWindowError::ResizeFailed;
        assert!(format!("{}", err).contains("resize"));
    }
}
