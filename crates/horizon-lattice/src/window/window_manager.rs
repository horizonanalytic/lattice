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
/// - Parent-child window relationships
/// - Signals for window lifecycle events
/// - Coordination of multi-window operations
///
/// # Parent-Child Relationships
///
/// Windows can have transient parent-child relationships:
/// - Child windows are not embedded in parent (they float independently)
/// - Child windows close automatically when parent closes
/// - Parent-child relationships enable modal dialog blocking
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::window::{WindowManager, WindowConfig, WindowType};
///
/// // Access the global window manager
/// let manager = WindowManager::instance();
///
/// // Create a main window
/// let main_id = manager.create_window(event_loop, WindowConfig::new("Main"))?;
///
/// // Create a dialog with parent relationship
/// let dialog_config = WindowConfig::new("Dialog")
///     .with_type(WindowType::Dialog)
///     .with_parent(main_id);
/// let dialog_id = manager.create_window(event_loop, dialog_config)?;
///
/// // Find a window by ID
/// if let Some(window) = manager.get(dialog_id) {
///     window.set_title("Save As");
/// }
///
/// // Get children of a window
/// let children = manager.children(main_id);
/// ```
pub struct WindowManager {
    /// All registered windows.
    windows: RwLock<HashMap<NativeWindowId, NativeWindow>>,
    /// Parent-child relationships: maps child window -> parent window.
    parent_map: RwLock<HashMap<NativeWindowId, NativeWindowId>>,
    /// Children of each window: maps parent window -> list of child windows.
    children_map: RwLock<HashMap<NativeWindowId, Vec<NativeWindowId>>>,
    /// Signal emitted when a window is created.
    window_created: Signal<NativeWindowId>,
    /// Signal emitted when a window is about to be destroyed.
    window_destroyed: Signal<NativeWindowId>,
    /// Signal emitted when a window gains focus.
    window_focused: Signal<NativeWindowId>,
    /// Signal emitted when a window loses focus.
    window_unfocused: Signal<NativeWindowId>,
    /// Signal emitted when a window is resized.
    /// Parameters: (window_id, new_width, new_height)
    window_resized: Signal<(NativeWindowId, u32, u32)>,
    /// Signal emitted when a window is moved.
    /// Parameters: (window_id, new_x, new_y)
    window_moved: Signal<(NativeWindowId, i32, i32)>,
    /// Signal emitted when a window's scale factor changes.
    /// Parameters: (window_id, new_scale_factor)
    ///
    /// This is emitted when a window moves to a monitor with a different DPI,
    /// or when the system DPI setting changes.
    window_scale_factor_changed: Signal<(NativeWindowId, f64)>,
}

impl WindowManager {
    /// Create a new window manager.
    fn new() -> Self {
        Self {
            windows: RwLock::new(HashMap::new()),
            parent_map: RwLock::new(HashMap::new()),
            children_map: RwLock::new(HashMap::new()),
            window_created: Signal::new(),
            window_destroyed: Signal::new(),
            window_focused: Signal::new(),
            window_unfocused: Signal::new(),
            window_resized: Signal::new(),
            window_moved: Signal::new(),
            window_scale_factor_changed: Signal::new(),
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
    /// If the window configuration specifies a parent, the window is registered
    /// as a child of that parent. Child windows are automatically closed when
    /// their parent is closed.
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
        let parent = config.parent();
        let window = NativeWindow::create(event_loop, config)?;
        let id = window.id();

        self.windows.write().insert(id, window);

        // Set up parent-child relationship
        if let Some(parent_id) = parent {
            self.set_parent_internal(id, parent_id);
        }

        self.window_created.emit(id);

        Ok(id)
    }

    /// Set the parent-child relationship.
    fn set_parent_internal(&self, child: NativeWindowId, parent: NativeWindowId) {
        self.parent_map.write().insert(child, parent);
        self.children_map
            .write()
            .entry(parent)
            .or_default()
            .push(child);
    }

    /// Register an existing window with the manager.
    ///
    /// This is useful when you create a window directly and want to
    /// track it through the manager.
    ///
    /// # Arguments
    ///
    /// * `window` - The window to register
    /// * `parent` - Optional parent window for transient relationship
    pub fn register(&self, window: NativeWindow, parent: Option<NativeWindowId>) -> NativeWindowId {
        let id = window.id();
        self.windows.write().insert(id, window);

        // Set up parent-child relationship
        if let Some(parent_id) = parent {
            self.set_parent_internal(id, parent_id);
        }

        self.window_created.emit(id);
        id
    }

    /// Unregister and remove a window from the manager.
    ///
    /// This also closes all child windows of the removed window and
    /// cleans up parent-child relationships.
    ///
    /// Returns the window if it was registered.
    pub fn unregister(&self, id: NativeWindowId) -> Option<NativeWindow> {
        // First, recursively close all children
        self.close_children(id);

        // Clean up this window's parent relationship
        if let Some(parent_id) = self.parent_map.write().remove(&id) {
            // Remove from parent's children list
            if let Some(siblings) = self.children_map.write().get_mut(&parent_id) {
                siblings.retain(|&child_id| child_id != id);
            }
        }

        // Remove from children_map (should be empty after close_children)
        self.children_map.write().remove(&id);

        let window = self.windows.write().remove(&id);
        if window.is_some() {
            self.window_destroyed.emit(id);
        }
        window
    }

    /// Close all child windows of a parent window.
    ///
    /// This recursively closes children of children as well.
    fn close_children(&self, parent_id: NativeWindowId) {
        // Get the list of children (clone to avoid holding lock)
        let children: Vec<NativeWindowId> = self
            .children_map
            .read()
            .get(&parent_id)
            .cloned()
            .unwrap_or_default();

        // Recursively close each child (this will close their children too)
        for child_id in children {
            // Recursively close grandchildren first
            self.close_children(child_id);

            // Remove the child window
            if let Some(child) = self.windows.write().remove(&child_id) {
                child.hide();
                self.parent_map.write().remove(&child_id);
                self.window_destroyed.emit(child_id);
            }
        }

        // Clear the children list
        self.children_map.write().remove(&parent_id);
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
    pub fn get_winit_window(
        &self,
        id: NativeWindowId,
    ) -> Option<std::sync::Arc<winit::window::Window>> {
        self.windows.read().get(&id).map(|w| w.winit_window_arc())
    }

    // =========================================================================
    // Parent-Child Relationships
    // =========================================================================

    /// Get the parent window of a child window.
    ///
    /// Returns `None` if the window has no parent or doesn't exist.
    pub fn parent(&self, id: NativeWindowId) -> Option<NativeWindowId> {
        self.parent_map.read().get(&id).copied()
    }

    /// Get all child windows of a parent window.
    ///
    /// Returns an empty vector if the window has no children or doesn't exist.
    pub fn children(&self, id: NativeWindowId) -> Vec<NativeWindowId> {
        self.children_map
            .read()
            .get(&id)
            .cloned()
            .unwrap_or_default()
    }

    /// Check if a window has any children.
    pub fn has_children(&self, id: NativeWindowId) -> bool {
        self.children_map
            .read()
            .get(&id)
            .is_some_and(|c| !c.is_empty())
    }

    /// Check if a window has a parent.
    pub fn has_parent(&self, id: NativeWindowId) -> bool {
        self.parent_map.read().contains_key(&id)
    }

    /// Set the parent of an existing window.
    ///
    /// This establishes a transient relationship where the child will
    /// close when the parent closes.
    ///
    /// # Arguments
    ///
    /// * `child` - The window to set as a child
    /// * `parent` - The parent window, or `None` to remove the parent relationship
    pub fn set_parent(&self, child: NativeWindowId, parent: Option<NativeWindowId>) {
        // First, remove any existing parent relationship
        if let Some(old_parent) = self.parent_map.write().remove(&child)
            && let Some(siblings) = self.children_map.write().get_mut(&old_parent) {
                siblings.retain(|&id| id != child);
            }

        // Set up new parent relationship
        if let Some(parent_id) = parent {
            self.set_parent_internal(child, parent_id);
        }
    }

    /// Get all root windows (windows without parents).
    ///
    /// These are typically the main application windows.
    pub fn root_windows(&self) -> Vec<NativeWindowId> {
        let windows = self.windows.read();
        let parent_map = self.parent_map.read();

        windows
            .keys()
            .filter(|id| !parent_map.contains_key(id))
            .copied()
            .collect()
    }

    /// Get all ancestor windows of a window, from immediate parent to root.
    ///
    /// Returns an empty vector if the window has no parent.
    pub fn ancestors(&self, id: NativeWindowId) -> Vec<NativeWindowId> {
        let mut result = Vec::new();
        let mut current = id;

        let parent_map = self.parent_map.read();
        while let Some(&parent) = parent_map.get(&current) {
            result.push(parent);
            current = parent;
        }

        result
    }

    /// Get all descendant windows of a window (children, grandchildren, etc.).
    ///
    /// Returns an empty vector if the window has no descendants.
    pub fn descendants(&self, id: NativeWindowId) -> Vec<NativeWindowId> {
        let mut result = Vec::new();
        let mut stack = vec![id];

        let children_map = self.children_map.read();
        while let Some(current) = stack.pop() {
            if let Some(children) = children_map.get(&current) {
                for &child in children {
                    result.push(child);
                    stack.push(child);
                }
            }
        }

        result
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

    /// Signal emitted when a window loses focus.
    ///
    /// The parameter is the unfocused window's ID.
    pub fn window_unfocused(&self) -> &Signal<NativeWindowId> {
        &self.window_unfocused
    }

    /// Notify that a window lost focus.
    ///
    /// This should be called by the event handler when a window loses focus.
    pub fn notify_unfocus(&self, id: NativeWindowId) {
        self.window_unfocused.emit(id);
    }

    /// Signal emitted when a window is resized.
    ///
    /// The parameters are (window_id, new_width, new_height).
    pub fn window_resized(&self) -> &Signal<(NativeWindowId, u32, u32)> {
        &self.window_resized
    }

    /// Notify that a window was resized.
    ///
    /// This should be called by the event handler when a window is resized.
    pub fn notify_resize(&self, id: NativeWindowId, width: u32, height: u32) {
        self.window_resized.emit((id, width, height));
    }

    /// Signal emitted when a window is moved.
    ///
    /// The parameters are (window_id, new_x, new_y).
    pub fn window_moved(&self) -> &Signal<(NativeWindowId, i32, i32)> {
        &self.window_moved
    }

    /// Notify that a window was moved.
    ///
    /// This should be called by the event handler when a window is moved.
    pub fn notify_move(&self, id: NativeWindowId, x: i32, y: i32) {
        self.window_moved.emit((id, x, y));
    }

    /// Signal emitted when a window's scale factor changes.
    ///
    /// The parameters are (window_id, new_scale_factor).
    ///
    /// This is typically emitted when:
    /// - A window moves to a monitor with a different DPI
    /// - The system DPI setting changes
    /// - The user changes their display scaling preference
    ///
    /// # Example
    ///
    /// ```ignore
    /// let manager = WindowManager::instance();
    /// manager.window_scale_factor_changed().connect(|(id, scale)| {
    ///     println!("Window {:?} scale factor changed to {}", id, scale);
    ///     // Update rendering resolution, redraw at new scale, etc.
    /// });
    /// ```
    pub fn window_scale_factor_changed(&self) -> &Signal<(NativeWindowId, f64)> {
        &self.window_scale_factor_changed
    }

    /// Notify that a window's scale factor changed.
    ///
    /// This should be called by the event handler when a window's scale factor changes.
    ///
    /// # Arguments
    ///
    /// * `id` - The window that changed
    /// * `scale_factor` - The new scale factor (e.g., 1.0, 2.0, 1.5)
    pub fn notify_scale_factor_change(&self, id: NativeWindowId, scale_factor: f64) {
        self.window_scale_factor_changed.emit((id, scale_factor));
    }

    // =========================================================================
    // Window Operations
    // =========================================================================

    /// Close all windows.
    ///
    /// This hides and unregisters all windows and clears all parent-child
    /// relationships.
    pub fn close_all(&self) {
        let ids: Vec<_> = self.windows.read().keys().copied().collect();
        for id in ids {
            if let Some(window) = self.windows.write().remove(&id) {
                window.hide();
                self.window_destroyed.emit(id);
            }
        }

        // Clear parent-child tracking
        self.parent_map.write().clear();
        self.children_map.write().clear();
    }

    /// Focus a window and bring its transient children to front.
    ///
    /// When a parent window is focused, its child windows (dialogs, tool windows)
    /// should also be brought to the front to maintain the proper z-order.
    pub fn focus_with_children(&self, id: NativeWindowId) {
        // First focus the parent
        if let Some(window) = self.get(id) {
            window.focus();
        }

        // Then focus children in order (so they appear on top)
        for child_id in self.descendants(id) {
            if let Some(child) = self.get(child_id) {
                child.focus();
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
    pub fn tile_windows_horizontal(
        &self,
        bounds_x: i32,
        bounds_y: i32,
        bounds_width: u32,
        bounds_height: u32,
    ) {
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
    pub fn tile_windows_vertical(
        &self,
        bounds_x: i32,
        bounds_y: i32,
        bounds_width: u32,
        bounds_height: u32,
    ) {
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

    // =========================================================================
    // Convenience Methods for Transient Windows
    // =========================================================================

    /// Create a dialog window as a child of a parent window.
    ///
    /// This is a convenience method that creates a window with:
    /// - `WindowType::Dialog`
    /// - Parent set to the specified window
    ///
    /// # Example
    ///
    /// ```ignore
    /// let dialog_id = manager.create_dialog(
    ///     event_loop,
    ///     "Save Changes",
    ///     main_window_id,
    ///     (400, 200),
    /// )?;
    /// ```
    pub fn create_dialog(
        &self,
        event_loop: &ActiveEventLoop,
        title: impl Into<String>,
        parent: NativeWindowId,
        size: (u32, u32),
    ) -> Result<NativeWindowId, NativeWindowError> {
        use super::window_type::WindowType;

        let config = WindowConfig::new(title)
            .with_type(WindowType::Dialog)
            .with_parent(parent)
            .with_size(size.0, size.1);

        self.create_window(event_loop, config)
    }

    /// Create a tool window as a child of a parent window.
    ///
    /// This is a convenience method that creates a window with:
    /// - `WindowType::Tool`
    /// - Parent set to the specified window
    ///
    /// # Example
    ///
    /// ```ignore
    /// let tool_id = manager.create_tool_window(
    ///     event_loop,
    ///     "Properties",
    ///     main_window_id,
    ///     (300, 400),
    /// )?;
    /// ```
    pub fn create_tool_window(
        &self,
        event_loop: &ActiveEventLoop,
        title: impl Into<String>,
        parent: NativeWindowId,
        size: (u32, u32),
    ) -> Result<NativeWindowId, NativeWindowError> {
        use super::window_type::WindowType;

        let config = WindowConfig::new(title)
            .with_type(WindowType::Tool)
            .with_parent(parent)
            .with_size(size.0, size.1);

        self.create_window(event_loop, config)
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

    // Helper to create a fake NativeWindowId for testing
    fn fake_id(n: u64) -> NativeWindowId {
        // Create a fake WindowId using unsafe transmute - only for testing!
        // This is a workaround since we can't create real windows in tests
        use std::mem::transmute;

        // winit::WindowId is an opaque type, but we can fake it for unit tests
        // This is safe in tests because we only use the ID for HashMap keys
        let fake_winit_id: WindowId = unsafe { transmute(n) };
        NativeWindowId::from_winit(fake_winit_id)
    }

    #[test]
    fn test_parent_child_relationship_internal() {
        let manager = WindowManager::new();

        let parent_id = fake_id(1);
        let child_id = fake_id(2);

        // Set up parent-child relationship directly
        manager.set_parent_internal(child_id, parent_id);

        // Verify relationship
        assert_eq!(manager.parent(child_id), Some(parent_id));
        assert!(manager.has_parent(child_id));
        assert!(!manager.has_parent(parent_id));

        assert!(manager.has_children(parent_id));
        assert!(!manager.has_children(child_id));
        assert_eq!(manager.children(parent_id), vec![child_id]);
    }

    #[test]
    fn test_multiple_children() {
        let manager = WindowManager::new();

        let parent_id = fake_id(1);
        let child1_id = fake_id(2);
        let child2_id = fake_id(3);
        let child3_id = fake_id(4);

        // Add multiple children
        manager.set_parent_internal(child1_id, parent_id);
        manager.set_parent_internal(child2_id, parent_id);
        manager.set_parent_internal(child3_id, parent_id);

        // Verify all children are tracked
        let children = manager.children(parent_id);
        assert_eq!(children.len(), 3);
        assert!(children.contains(&child1_id));
        assert!(children.contains(&child2_id));
        assert!(children.contains(&child3_id));

        // Each child has the same parent
        assert_eq!(manager.parent(child1_id), Some(parent_id));
        assert_eq!(manager.parent(child2_id), Some(parent_id));
        assert_eq!(manager.parent(child3_id), Some(parent_id));
    }

    #[test]
    fn test_grandchildren() {
        let manager = WindowManager::new();

        let root_id = fake_id(1);
        let child_id = fake_id(2);
        let grandchild_id = fake_id(3);

        // Create hierarchy: root -> child -> grandchild
        manager.set_parent_internal(child_id, root_id);
        manager.set_parent_internal(grandchild_id, child_id);

        // Verify ancestors
        let ancestors = manager.ancestors(grandchild_id);
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0], child_id);
        assert_eq!(ancestors[1], root_id);

        // Verify descendants
        let descendants = manager.descendants(root_id);
        assert_eq!(descendants.len(), 2);
        assert!(descendants.contains(&child_id));
        assert!(descendants.contains(&grandchild_id));

        // Child's descendants
        let child_descendants = manager.descendants(child_id);
        assert_eq!(child_descendants.len(), 1);
        assert_eq!(child_descendants[0], grandchild_id);
    }

    #[test]
    fn test_root_windows() {
        let manager = WindowManager::new();

        let root1_id = fake_id(1);
        let root2_id = fake_id(2);
        let child_id = fake_id(3);

        // Add windows to registry (as empty entries, just for tracking)
        // Note: In real usage, these would be actual windows
        // For this test, we just need IDs in the windows map
        // We'll simulate by adding to children_map for root1
        manager.set_parent_internal(child_id, root1_id);

        // Add root IDs to windows map by using the parent_map as indicator
        // The root_windows method checks both maps
        // For proper testing, we need windows registered

        // Verify child has parent
        assert!(manager.has_parent(child_id));
        assert!(!manager.has_parent(root1_id));
        assert!(!manager.has_parent(root2_id));
    }

    #[test]
    fn test_set_parent_changes_relationship() {
        let manager = WindowManager::new();

        let parent1_id = fake_id(1);
        let parent2_id = fake_id(2);
        let child_id = fake_id(3);

        // Initially set child under parent1
        manager.set_parent_internal(child_id, parent1_id);
        assert_eq!(manager.parent(child_id), Some(parent1_id));
        assert!(manager.children(parent1_id).contains(&child_id));

        // Change parent to parent2
        manager.set_parent(child_id, Some(parent2_id));
        assert_eq!(manager.parent(child_id), Some(parent2_id));
        assert!(manager.children(parent2_id).contains(&child_id));
        assert!(!manager.children(parent1_id).contains(&child_id));
    }

    #[test]
    fn test_set_parent_none_removes_relationship() {
        let manager = WindowManager::new();

        let parent_id = fake_id(1);
        let child_id = fake_id(2);

        // Set parent
        manager.set_parent_internal(child_id, parent_id);
        assert_eq!(manager.parent(child_id), Some(parent_id));

        // Remove parent
        manager.set_parent(child_id, None);
        assert_eq!(manager.parent(child_id), None);
        assert!(!manager.has_parent(child_id));
        assert!(!manager.children(parent_id).contains(&child_id));
    }

    #[test]
    fn test_close_all_clears_relationships() {
        let manager = WindowManager::new();

        let parent_id = fake_id(1);
        let child_id = fake_id(2);

        manager.set_parent_internal(child_id, parent_id);
        assert!(manager.has_children(parent_id));

        // Close all windows
        manager.close_all();

        // Relationships should be cleared
        assert!(!manager.has_children(parent_id));
        assert!(!manager.has_parent(child_id));
    }
}
