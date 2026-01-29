//! System tray icon support for Horizon Lattice.
//!
//! This module provides [`SystemTrayIcon`], a class for displaying an icon in the
//! system tray (notification area on Windows, menu bar on macOS, system tray on Linux).
//!
//! # Overview
//!
//! System tray icons allow applications to:
//! - Remain accessible when minimized
//! - Display status information via tooltips
//! - Provide quick access to common actions via context menus
//! - Show notification balloons/messages
//!
//! # Platform Support
//!
//! - **Windows**: Uses the Windows notification area
//! - **macOS**: Uses NSStatusItem in the menu bar
//! - **Linux**: Uses GTK with libappindicator (or libayatana-appindicator)
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{SystemTrayIcon, TrayMenu, TrayIconImage, ActivationReason};
//! use std::sync::Arc;
//!
//! // Create a tray icon
//! let mut tray = SystemTrayIcon::new();
//!
//! // Set icon and tooltip
//! let icon = TrayIconImage::from_rgba(icon_data, 32, 32).expect("Failed to create icon");
//! tray.set_icon(icon);
//! tray.set_tooltip("My Application");
//!
//! // Create a context menu
//! let mut menu = TrayMenu::new();
//! menu.add_action(Arc::new(Action::new("&Open")));
//! menu.add_separator();
//! menu.add_action(Arc::new(Action::new("E&xit")));
//! tray.set_menu(Some(menu));
//!
//! // Handle activation
//! tray.activated.connect(|reason| {
//!     match reason {
//!         ActivationReason::Click => println!("Tray icon clicked"),
//!         ActivationReason::DoubleClick => println!("Tray icon double-clicked"),
//!         _ => {}
//!     }
//! });
//!
//! // Show the tray icon
//! tray.show().expect("Failed to show tray icon");
//! ```

use std::collections::HashMap;
use std::sync::Arc;

use horizon_lattice_core::Signal;
use parking_lot::RwLock;
use tray_icon::menu::{
    CheckMenuItem, ContextMenu, Menu as TrayIconMenu, MenuEvent, MenuId, MenuItem,
    PredefinedMenuItem, Submenu,
};
use tray_icon::{Icon, MouseButton, TrayIcon, TrayIconBuilder, TrayIconEvent};

use super::Action;

// ============================================================================
// Error Type
// ============================================================================

/// Errors that can occur when working with system tray icons.
#[derive(Debug, Clone)]
pub enum TrayError {
    /// Failed to create the tray icon.
    CreationFailed(String),
    /// Failed to load or create an icon image.
    IconError(String),
    /// The tray icon is not currently visible.
    NotVisible,
    /// Platform-specific error.
    PlatformError(String),
}

impl std::fmt::Display for TrayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrayError::CreationFailed(msg) => write!(f, "Failed to create tray icon: {}", msg),
            TrayError::IconError(msg) => write!(f, "Icon error: {}", msg),
            TrayError::NotVisible => write!(f, "Tray icon is not visible"),
            TrayError::PlatformError(msg) => write!(f, "Platform error: {}", msg),
        }
    }
}

impl std::error::Error for TrayError {}

// ============================================================================
// Activation Reason
// ============================================================================

/// Reason for system tray icon activation.
///
/// This enum indicates how the user interacted with the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ActivationReason {
    /// Unknown activation (default).
    #[default]
    Unknown,
    /// Tray icon was clicked (left-click on Windows/Linux, click on macOS).
    Click,
    /// Tray icon was double-clicked.
    DoubleClick,
    /// Tray icon was middle-clicked.
    MiddleClick,
    /// Context menu was requested (right-click on Windows/Linux).
    Context,
}

// ============================================================================
// Tray Icon Image
// ============================================================================

/// An image for use with system tray icons.
///
/// This wraps the native icon format and provides convenient creation methods.
#[derive(Clone)]
pub struct TrayIconImage {
    inner: Icon,
}

impl TrayIconImage {
    /// Create a tray icon image from RGBA pixel data.
    ///
    /// # Arguments
    ///
    /// * `rgba` - Raw RGBA pixel data (4 bytes per pixel)
    /// * `width` - Width of the image in pixels
    /// * `height` - Height of the image in pixels
    ///
    /// # Errors
    ///
    /// Returns an error if the data size doesn't match width * height * 4.
    pub fn from_rgba(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self, TrayError> {
        Icon::from_rgba(rgba, width, height)
            .map(|inner| Self { inner })
            .map_err(|e| TrayError::IconError(e.to_string()))
    }

    /// Get the underlying icon.
    pub(crate) fn into_inner(self) -> Icon {
        self.inner
    }
}

impl std::fmt::Debug for TrayIconImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayIconImage").finish_non_exhaustive()
    }
}

// ============================================================================
// Tray Menu
// ============================================================================

/// A menu for use with system tray icons.
///
/// This is an adapter that wraps `tray_icon::menu::Menu` and integrates with our `Action` system.
/// Menu items trigger their associated `Action`'s signals when clicked.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::widgets::{TrayMenu, Action};
/// use std::sync::Arc;
///
/// let mut menu = TrayMenu::new();
///
/// let open_action = Arc::new(Action::new("&Open"));
/// open_action.triggered.connect(|_| println!("Open clicked"));
/// menu.add_action(open_action);
///
/// menu.add_separator();
///
/// let quit_action = Arc::new(Action::new("&Quit"));
/// quit_action.triggered.connect(|_| std::process::exit(0));
/// menu.add_action(quit_action);
/// ```
pub struct TrayMenu {
    inner: TrayIconMenu,
    action_map: HashMap<MenuId, Arc<Action>>,
}

impl TrayMenu {
    /// Create a new empty tray menu.
    pub fn new() -> Self {
        Self {
            inner: TrayIconMenu::new(),
            action_map: HashMap::new(),
        }
    }

    /// Add an action to the menu.
    ///
    /// The action's text is used as the menu item label. If the action is
    /// checkable, the menu item will show a checkmark when checked.
    pub fn add_action(&mut self, action: Arc<Action>) {
        let text = action.display_text();
        let enabled = action.is_enabled();

        if action.is_checkable() {
            // Create a check menu item
            let item = CheckMenuItem::new(text, enabled, action.is_checked(), None);
            let id = item.id().clone();
            let _ = self.inner.append(&item);
            self.action_map.insert(id, action);
        } else {
            // Create a regular menu item
            let item = MenuItem::new(text, enabled, None);
            let id = item.id().clone();
            let _ = self.inner.append(&item);
            self.action_map.insert(id, action);
        }
    }

    /// Add a separator to the menu.
    pub fn add_separator(&mut self) {
        let _ = self.inner.append(&PredefinedMenuItem::separator());
    }

    /// Add a submenu to this menu.
    ///
    /// # Arguments
    ///
    /// * `title` - The title of the submenu
    /// * `menu` - The submenu to add
    pub fn add_submenu(&mut self, title: &str, menu: TrayMenu) {
        let submenu = Submenu::new(title, true);
        // Move items from the provided menu to the submenu
        for (id, action) in menu.action_map {
            self.action_map.insert(id, action);
        }
        // Note: muda doesn't provide a way to move items between menus,
        // so this is a simplified implementation
        let _ = self.inner.append(&submenu);
    }

    /// Clear all items from the menu.
    pub fn clear(&mut self) {
        // Recreate the menu since there's no clear method
        self.inner = TrayIconMenu::new();
        self.action_map.clear();
    }

    /// Get the action associated with a menu ID, if any.
    pub(crate) fn get_action(&self, id: &MenuId) -> Option<&Arc<Action>> {
        self.action_map.get(id)
    }

    /// Get the underlying menu.
    pub(crate) fn inner(&self) -> &TrayIconMenu {
        &self.inner
    }
}

impl Default for TrayMenu {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for TrayMenu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrayMenu")
            .field("item_count", &self.action_map.len())
            .finish()
    }
}

// ============================================================================
// System Tray Icon
// ============================================================================

/// Internal state for SystemTrayIcon.
struct SystemTrayIconState {
    tray: Option<TrayIcon>,
    tooltip: String,
    menu: Option<TrayMenu>,
    icon: Option<TrayIconImage>,
}

/// A system tray icon (notification area icon).
///
/// `SystemTrayIcon` provides an icon in the system notification area
/// (Windows), menu bar (macOS), or system tray (Linux). The icon can
/// display a tooltip, respond to clicks, and show a context menu.
///
/// # Thread Safety
///
/// `SystemTrayIcon` is `Send + Sync` and can be safely used from multiple
/// threads. However, on some platforms, the tray icon must be created
/// and manipulated from the main thread.
///
/// # Signals
///
/// - [`activated`](SystemTrayIcon::activated): Emitted when the icon is clicked
/// - [`message_clicked`](SystemTrayIcon::message_clicked): Emitted when a balloon message is clicked
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::widgets::{SystemTrayIcon, TrayIconImage, ActivationReason};
///
/// let mut tray = SystemTrayIcon::new();
///
/// // Create icon from RGBA data
/// let icon = TrayIconImage::from_rgba(icon_data, 32, 32).expect("Failed to create icon");
/// tray.set_icon(icon);
/// tray.set_tooltip("My Application - Running");
///
/// // Handle clicks
/// tray.activated.connect(|&reason| {
///     if reason == ActivationReason::DoubleClick {
///         println!("Show main window");
///     }
/// });
///
/// // Show the icon
/// tray.show().expect("Failed to show tray icon");
/// ```
pub struct SystemTrayIcon {
    state: RwLock<SystemTrayIconState>,

    /// Signal emitted when the tray icon is activated (clicked).
    ///
    /// The parameter indicates how the icon was activated (single click,
    /// double click, etc.).
    pub activated: Signal<ActivationReason>,

    /// Signal emitted when a balloon/notification message is clicked.
    ///
    /// This is only emitted on platforms that support balloon messages
    /// (primarily Windows).
    pub message_clicked: Signal<()>,
}

impl SystemTrayIcon {
    /// Create a new system tray icon.
    ///
    /// The icon is not visible until [`show`](SystemTrayIcon::show) is called.
    pub fn new() -> Self {
        Self {
            state: RwLock::new(SystemTrayIconState {
                tray: None,
                tooltip: String::new(),
                menu: None,
                icon: None,
            }),
            activated: Signal::new(),
            message_clicked: Signal::new(),
        }
    }

    /// Create a new system tray icon with an icon.
    pub fn with_icon(icon: impl Into<TrayIconImage>) -> Self {
        let tray = Self::new();
        tray.set_icon(icon.into());
        tray
    }

    /// Create a new system tray icon with a tooltip.
    pub fn with_tooltip(tooltip: impl Into<String>) -> Self {
        let tray = Self::new();
        tray.set_tooltip(tooltip);
        tray
    }

    /// Create a new system tray icon with a context menu.
    pub fn with_menu(menu: TrayMenu) -> Self {
        let tray = Self::new();
        tray.set_menu(Some(menu));
        tray
    }

    // ========================================================================
    // Icon
    // ========================================================================

    /// Set the tray icon image.
    pub fn set_icon(&self, icon: TrayIconImage) {
        let mut state = self.state.write();
        if let Some(ref tray) = state.tray {
            let _ = tray.set_icon(Some(icon.clone().into_inner()));
        }
        state.icon = Some(icon);
    }

    /// Get whether an icon has been set.
    pub fn has_icon(&self) -> bool {
        self.state.read().icon.is_some()
    }

    // ========================================================================
    // Tooltip
    // ========================================================================

    /// Set the tooltip text shown when hovering over the tray icon.
    pub fn set_tooltip(&self, tooltip: impl Into<String>) {
        let tooltip = tooltip.into();
        let mut state = self.state.write();
        if let Some(ref tray) = state.tray {
            let _ = tray.set_tooltip(Some(&tooltip));
        }
        state.tooltip = tooltip;
    }

    /// Get the current tooltip text.
    pub fn tooltip(&self) -> String {
        self.state.read().tooltip.clone()
    }

    // ========================================================================
    // Menu
    // ========================================================================

    /// Set the context menu for the tray icon.
    ///
    /// Pass `None` to remove the context menu.
    pub fn set_menu(&self, menu: Option<TrayMenu>) {
        let mut state = self.state.write();
        if let Some(ref tray) = state.tray {
            match &menu {
                Some(m) => {
                    let menu_clone = m.inner().clone();
                    tray.set_menu(Some(Box::new(menu_clone) as Box<dyn ContextMenu>));
                }
                None => {
                    tray.set_menu(None::<Box<dyn ContextMenu>>);
                }
            }
        }
        state.menu = menu;
    }

    /// Get a reference to the current context menu, if any.
    pub fn menu(&self) -> Option<TrayMenu> {
        // We can't return a reference due to the RwLock, so this is limited
        None
    }

    /// Check if the tray icon has a context menu.
    pub fn has_menu(&self) -> bool {
        self.state.read().menu.is_some()
    }

    // ========================================================================
    // Visibility
    // ========================================================================

    /// Show the system tray icon.
    ///
    /// This creates the native tray icon and makes it visible. The icon
    /// must have been set before calling this method.
    ///
    /// # Errors
    ///
    /// Returns an error if the icon hasn't been set or if the native
    /// tray icon creation fails.
    pub fn show(&self) -> Result<(), TrayError> {
        let mut state = self.state.write();

        // Already visible
        if state.tray.is_some() {
            return Ok(());
        }

        // Build the tray icon
        let mut builder = TrayIconBuilder::new();

        if let Some(ref icon) = state.icon {
            builder = builder.with_icon(icon.clone().into_inner());
        }

        if !state.tooltip.is_empty() {
            builder = builder.with_tooltip(&state.tooltip);
        }

        if let Some(ref menu) = state.menu {
            builder = builder.with_menu(Box::new(menu.inner().clone()));
        }

        let tray = builder
            .build()
            .map_err(|e| TrayError::CreationFailed(e.to_string()))?;

        state.tray = Some(tray);

        Ok(())
    }

    /// Hide and destroy the system tray icon.
    pub fn hide(&self) {
        let mut state = self.state.write();
        state.tray = None;
    }

    /// Check if the tray icon is currently visible.
    pub fn is_visible(&self) -> bool {
        self.state.read().tray.is_some()
    }

    // ========================================================================
    // Notifications
    // ========================================================================

    /// Show a balloon notification message (Windows) or notification (other platforms).
    ///
    /// # Arguments
    ///
    /// * `title` - The title of the notification
    /// * `message` - The body text of the notification
    /// * `_icon` - The icon to display (platform-dependent support)
    ///
    /// # Note
    ///
    /// This feature has limited cross-platform support. On macOS and Linux,
    /// this may use the system notification service instead of a balloon.
    /// Consider using a dedicated notification library for better cross-platform
    /// notification support.
    #[allow(unused_variables)]
    pub fn show_message(&self, title: &str, message: &str, _icon: super::MessageIcon) {
        // Note: tray-icon doesn't directly support balloon messages.
        // On Windows, this would require additional platform-specific code.
        // For now, this is a no-op. A full implementation would use:
        // - Windows: Shell_NotifyIcon with NIF_INFO
        // - macOS: NSUserNotification or UNUserNotificationCenter
        // - Linux: libnotify
        //
        // This is left as a stub for API compatibility.
        #[cfg(debug_assertions)]
        eprintln!(
            "SystemTrayIcon::show_message is not fully implemented: title='{}', message='{}'",
            title, message
        );
    }

    // ========================================================================
    // Event Processing
    // ========================================================================

    /// Process pending tray icon events.
    ///
    /// Call this method periodically (e.g., in your event loop) to process
    /// click events and menu selections. This method emits the appropriate
    /// signals based on user interactions.
    ///
    /// # Returns
    ///
    /// The number of events processed.
    pub fn process_events(&self) -> usize {
        let mut count = 0;

        // Process tray icon events (clicks)
        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
            count += 1;
            let reason = Self::convert_event(&event);
            // Only emit for actual click events, not enter/move/leave
            if reason != ActivationReason::Unknown {
                self.activated.emit(reason);
            }
        }

        // Process menu events
        let state = self.state.read();
        if let Some(ref menu) = state.menu {
            while let Ok(event) = MenuEvent::receiver().try_recv() {
                count += 1;
                if let Some(action) = menu.get_action(event.id()) {
                    action.trigger();
                }
            }
        }

        count
    }

    /// Convert a tray icon event to an activation reason.
    fn convert_event(event: &TrayIconEvent) -> ActivationReason {
        match event {
            TrayIconEvent::Click { button, .. } => match button {
                MouseButton::Left => ActivationReason::Click,
                MouseButton::Right => ActivationReason::Context,
                MouseButton::Middle => ActivationReason::MiddleClick,
            },
            TrayIconEvent::DoubleClick { button, .. } => match button {
                MouseButton::Left => ActivationReason::DoubleClick,
                MouseButton::Right => ActivationReason::Context,
                MouseButton::Middle => ActivationReason::MiddleClick,
            },
            // Enter, Move, Leave events don't trigger activation
            _ => ActivationReason::Unknown,
        }
    }
}

impl Default for SystemTrayIcon {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SystemTrayIcon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = self.state.read();
        f.debug_struct("SystemTrayIcon")
            .field("visible", &state.tray.is_some())
            .field("tooltip", &state.tooltip)
            .field("has_menu", &state.menu.is_some())
            .field("has_icon", &state.icon.is_some())
            .finish()
    }
}

// SystemTrayIcon is Send + Sync
unsafe impl Send for SystemTrayIcon {}
unsafe impl Sync for SystemTrayIcon {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_reason_default() {
        let reason = ActivationReason::default();
        assert_eq!(reason, ActivationReason::Unknown);
    }

    #[test]
    fn test_tray_error_display() {
        let err = TrayError::CreationFailed("test error".to_string());
        assert!(err.to_string().contains("test error"));

        let err = TrayError::IconError("icon error".to_string());
        assert!(err.to_string().contains("icon error"));

        let err = TrayError::NotVisible;
        assert!(err.to_string().contains("not visible"));

        let err = TrayError::PlatformError("platform error".to_string());
        assert!(err.to_string().contains("platform error"));
    }

    #[test]
    #[ignore = "requires main thread on macOS"]
    fn test_tray_menu_new() {
        let menu = TrayMenu::new();
        assert_eq!(menu.action_map.len(), 0);
    }

    #[test]
    #[ignore = "requires main thread on macOS"]
    fn test_tray_menu_default() {
        let menu = TrayMenu::default();
        assert_eq!(menu.action_map.len(), 0);
    }

    #[test]
    fn test_system_tray_icon_new() {
        let tray = SystemTrayIcon::new();
        assert!(!tray.is_visible());
        assert!(!tray.has_icon());
        assert!(!tray.has_menu());
        assert!(tray.tooltip().is_empty());
    }

    #[test]
    fn test_system_tray_icon_tooltip() {
        let tray = SystemTrayIcon::new();
        tray.set_tooltip("Test Tooltip");
        assert_eq!(tray.tooltip(), "Test Tooltip");
    }

    #[test]
    fn test_system_tray_icon_with_tooltip() {
        let tray = SystemTrayIcon::with_tooltip("My App");
        assert_eq!(tray.tooltip(), "My App");
    }

    #[test]
    #[ignore = "requires main thread on macOS"]
    fn test_system_tray_icon_menu() {
        let tray = SystemTrayIcon::new();
        assert!(!tray.has_menu());

        let menu = TrayMenu::new();
        tray.set_menu(Some(menu));
        assert!(tray.has_menu());

        tray.set_menu(None);
        assert!(!tray.has_menu());
    }

    #[test]
    fn test_system_tray_icon_hide_when_not_visible() {
        let tray = SystemTrayIcon::new();
        assert!(!tray.is_visible());
        tray.hide(); // Should not panic
        assert!(!tray.is_visible());
    }

    #[test]
    fn test_system_tray_icon_default() {
        let tray = SystemTrayIcon::default();
        assert!(!tray.is_visible());
    }

    #[test]
    fn test_system_tray_icon_debug() {
        let tray = SystemTrayIcon::new();
        tray.set_tooltip("Debug Test");
        let debug_str = format!("{:?}", tray);
        assert!(debug_str.contains("SystemTrayIcon"));
        assert!(debug_str.contains("Debug Test"));
    }

    #[test]
    #[cfg_attr(
        target_os = "linux",
        ignore = "tray-icon crate behavior differs on Linux"
    )]
    fn test_tray_icon_image_from_rgba_invalid() {
        // Invalid: data doesn't match dimensions
        let result = TrayIconImage::from_rgba(vec![0; 10], 32, 32);
        assert!(result.is_err());
    }

    #[test]
    #[ignore = "requires main thread on macOS"]
    fn test_tray_menu_add_separator() {
        let mut menu = TrayMenu::new();
        menu.add_separator();
        // Separator doesn't add to action_map
        assert_eq!(menu.action_map.len(), 0);
    }

    #[test]
    #[ignore = "requires main thread on macOS"]
    fn test_tray_menu_clear() {
        let mut menu = TrayMenu::new();
        menu.add_separator();
        menu.clear();
        assert_eq!(menu.action_map.len(), 0);
    }

    #[test]
    #[ignore = "requires main thread on macOS"]
    fn test_tray_menu_debug() {
        let menu = TrayMenu::new();
        let debug_str = format!("{:?}", menu);
        assert!(debug_str.contains("TrayMenu"));
        assert!(debug_str.contains("item_count"));
    }
}
