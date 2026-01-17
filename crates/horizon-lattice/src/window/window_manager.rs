//! Window manager for tracking and managing multiple windows.
//!
//! The `WindowManager` provides centralized window tracking and management
//! for multi-window applications.

use std::collections::HashMap;
use std::sync::OnceLock;

use parking_lot::RwLock;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use horizon_lattice_core::Signal;

use super::native_window::{NativeWindow, NativeWindowError, NativeWindowId};
use super::window_config::WindowConfig;

/// Global window manager instance.
static WINDOW_MANAGER: OnceLock<WindowManager> = OnceLock::new();

/// Manager for tracking and coordinating multiple windows.
///
/// The `WindowManager` provides:
/// - Central registry of all application windows
/// - Window lookup by ID
/// - Signals for window lifecycle events
/// - Coordination of multi-window operations
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::{WindowManager, WindowConfig};
///
/// // Access the global window manager
/// let manager = WindowManager::instance();
///
/// // Create a window through the manager
/// let window = manager.create_window(event_loop, config)?;
///
/// // Find a window by ID
/// if let Some(window) = manager.get(window_id) {
///     window.set_title("New Title");
/// }
///
/// // Iterate all windows
/// for id in manager.window_ids() {
///     println!("Window: {:?}", id);
/// }
/// ```
pub struct WindowManager {
    /// All registered windows.
    windows: RwLock<HashMap<NativeWindowId, NativeWindow>>,
    /// Signal emitted when a window is created.
    window_created: Signal<NativeWindowId>,
    /// Signal emitted when a window is about to be destroyed.
    window_destroyed: Signal<NativeWindowId>,
    /// Signal emitted when a window gains focus.
    window_focused: Signal<NativeWindowId>,
}

impl WindowManager {
    /// Create a new window manager.
    fn new() -> Self {
        Self {
            windows: RwLock::new(HashMap::new()),
            window_created: Signal::new(),
            window_destroyed: Signal::new(),
            window_focused: Signal::new(),
        }
    }

    /// Get the global window manager instance.
    ///
    /// Initializes the manager on first call.
    pub fn instance() -> &'static WindowManager {
        WINDOW_MANAGER.get_or_init(WindowManager::new)
    }

    /// Create a new window and register it with the manager.
    ///
    /// # Arguments
    ///
    /// * `event_loop` - The active event loop
    /// * `config` - Window configuration
    ///
    /// # Returns
    ///
    /// The window ID on success.
    ///
    /// # Errors
    ///
    /// Returns an error if window creation fails.
    pub fn create_window(
        &self,
        event_loop: &ActiveEventLoop,
        config: WindowConfig,
    ) -> Result<NativeWindowId, NativeWindowError> {
        let window = NativeWindow::create(event_loop, config)?;
        let id = window.id();

        self.windows.write().insert(id, window);
        self.window_created.emit(id);

        Ok(id)
    }

    /// Register an existing window with the manager.
    ///
    /// This is useful when you create a window directly and want to
    /// track it through the manager.
    pub fn register(&self, window: NativeWindow) -> NativeWindowId {
        let id = window.id();
        self.windows.write().insert(id, window);
        self.window_created.emit(id);
        id
    }

    /// Unregister and remove a window from the manager.
    ///
    /// Returns the window if it was registered.
    pub fn unregister(&self, id: NativeWindowId) -> Option<NativeWindow> {
        let window = self.windows.write().remove(&id);
        if window.is_some() {
            self.window_destroyed.emit(id);
        }
        window
    }

    /// Get an immutable reference to a window by ID.
    ///
    /// Note: This returns the window through a read guard. For operations
    /// that need mutable access, use `with_window_mut`.
    pub fn get(&self, id: NativeWindowId) -> Option<WindowRef<'_>> {
        let guard = self.windows.read();
        if guard.contains_key(&id) {
            Some(WindowRef { guard, id })
        } else {
            None
        }
    }

    /// Execute a closure with mutable access to a window.
    ///
    /// Returns `Some(result)` if the window exists, `None` otherwise.
    pub fn with_window_mut<F, R>(&self, id: NativeWindowId, f: F) -> Option<R>
    where
        F: FnOnce(&mut NativeWindow) -> R,
    {
        let mut guard = self.windows.write();
        guard.get_mut(&id).map(f)
    }

    /// Check if a window is registered.
    pub fn contains(&self, id: NativeWindowId) -> bool {
        self.windows.read().contains_key(&id)
    }

    /// Get the number of registered windows.
    pub fn count(&self) -> usize {
        self.windows.read().len()
    }

    /// Check if there are any registered windows.
    pub fn is_empty(&self) -> bool {
        self.windows.read().is_empty()
    }

    /// Get all window IDs.
    pub fn window_ids(&self) -> Vec<NativeWindowId> {
        self.windows.read().keys().copied().collect()
    }

    /// Find a window by its winit WindowId.
    pub fn find_by_winit_id(&self, winit_id: WindowId) -> Option<NativeWindowId> {
        let native_id = NativeWindowId::from_winit(winit_id);
        if self.windows.read().contains_key(&native_id) {
            Some(native_id)
        } else {
            None
        }
    }

    /// Get the winit window for a given ID.
    ///
    /// This is useful for surface creation and other winit-specific operations.
    pub fn get_winit_window(&self, id: NativeWindowId) -> Option<std::sync::Arc<winit::window::Window>> {
        self.windows.read().get(&id).map(|w| w.winit_window_arc())
    }

    // =========================================================================
    // Signals
    // =========================================================================

    /// Signal emitted when a window is created.
    ///
    /// The parameter is the new window's ID.
    pub fn window_created(&self) -> &Signal<NativeWindowId> {
        &self.window_created
    }

    /// Signal emitted when a window is destroyed.
    ///
    /// The parameter is the destroyed window's ID.
    pub fn window_destroyed(&self) -> &Signal<NativeWindowId> {
        &self.window_destroyed
    }

    /// Signal emitted when a window gains focus.
    ///
    /// The parameter is the focused window's ID.
    pub fn window_focused(&self) -> &Signal<NativeWindowId> {
        &self.window_focused
    }

    /// Notify that a window gained focus.
    ///
    /// This should be called by the event handler when a window receives focus.
    pub fn notify_focus(&self, id: NativeWindowId) {
        self.window_focused.emit(id);
    }

    // =========================================================================
    // Window Operations
    // =========================================================================

    /// Close all windows.
    ///
    /// This hides and unregisters all windows.
    pub fn close_all(&self) {
        let ids: Vec<_> = self.windows.read().keys().copied().collect();
        for id in ids {
            if let Some(window) = self.windows.write().remove(&id) {
                window.hide();
                self.window_destroyed.emit(id);
            }
        }
    }

    /// Request redraw for all windows.
    pub fn request_redraw_all(&self) {
        for window in self.windows.read().values() {
            window.request_redraw();
        }
    }

    /// Cascade all windows.
    ///
    /// Arranges windows in a cascading pattern, each offset from the previous.
    pub fn cascade_windows(&self, start_x: i32, start_y: i32, offset: i32) {
        let guard = self.windows.read();
        for (i, window) in guard.values().enumerate() {
            let x = start_x + (i as i32 * offset);
            let y = start_y + (i as i32 * offset);
            window.set_outer_position_logical(x as f64, y as f64);
        }
    }

    /// Tile windows horizontally.
    ///
    /// Arranges windows side by side, each taking an equal portion of the available width.
    pub fn tile_windows_horizontal(&self, bounds_x: i32, bounds_y: i32, bounds_width: u32, bounds_height: u32) {
        let guard = self.windows.read();
        let count = guard.len();
        if count == 0 {
            return;
        }

        let width_per_window = bounds_width / count as u32;
        for (i, window) in guard.values().enumerate() {
            let x = bounds_x + (i as u32 * width_per_window) as i32;
            window.set_outer_position_logical(x as f64, bounds_y as f64);
            window.request_inner_size_logical(width_per_window as f64, bounds_height as f64);
        }
    }

    /// Tile windows vertically.
    ///
    /// Arranges windows stacked vertically, each taking an equal portion of the available height.
    pub fn tile_windows_vertical(&self, bounds_x: i32, bounds_y: i32, bounds_width: u32, bounds_height: u32) {
        let guard = self.windows.read();
        let count = guard.len();
        if count == 0 {
            return;
        }

        let height_per_window = bounds_height / count as u32;
        for (i, window) in guard.values().enumerate() {
            let y = bounds_y + (i as u32 * height_per_window) as i32;
            window.set_outer_position_logical(bounds_x as f64, y as f64);
            window.request_inner_size_logical(bounds_width as f64, height_per_window as f64);
        }
    }
}

impl Default for WindowManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A reference to a window in the manager.
///
/// This holds a read lock on the window storage for the duration of its lifetime.
pub struct WindowRef<'a> {
    guard: parking_lot::RwLockReadGuard<'a, HashMap<NativeWindowId, NativeWindow>>,
    id: NativeWindowId,
}

impl<'a> WindowRef<'a> {
    /// Get a reference to the underlying window.
    pub fn window(&self) -> &NativeWindow {
        self.guard.get(&self.id).expect("window should exist")
    }
}

impl<'a> std::ops::Deref for WindowRef<'a> {
    type Target = NativeWindow;

    fn deref(&self) -> &Self::Target {
        self.window()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_manager_creation() {
        let manager = WindowManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.count(), 0);
    }

    #[test]
    fn test_window_manager_global_instance() {
        let manager1 = WindowManager::instance();
        let manager2 = WindowManager::instance();
        // Both should point to the same instance
        assert!(std::ptr::eq(manager1, manager2));
    }
}
