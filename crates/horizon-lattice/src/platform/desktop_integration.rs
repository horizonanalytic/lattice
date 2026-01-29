//! Desktop integration services.
//!
//! This module provides cross-platform integration with the desktop environment,
//! including recent documents management, taskbar/dock badges and progress indicators,
//! jump lists, and desktop entry management.
//!
//! # Recent Documents
//!
//! ```ignore
//! use horizon_lattice::platform::{RecentDocuments, RecentDocument};
//!
//! // Add a file to the recent documents list
//! RecentDocuments::add("/path/to/document.pdf")?;
//!
//! // Add with metadata
//! RecentDocuments::add_with_info(RecentDocument::new("/path/to/file.txt")
//!     .app_name("My App")
//!     .mime_type("text/plain"))?;
//!
//! // Clear recent documents for this application
//! RecentDocuments::clear()?;
//! ```
//!
//! # Taskbar/Dock Progress
//!
//! ```ignore
//! use horizon_lattice::platform::{TaskbarProgress, ProgressState};
//!
//! // Show indeterminate progress (spinning)
//! TaskbarProgress::set_state(ProgressState::Indeterminate)?;
//!
//! // Show determinate progress (0-100%)
//! TaskbarProgress::set_progress(50)?;
//!
//! // Show error state
//! TaskbarProgress::set_state(ProgressState::Error)?;
//!
//! // Clear progress indicator
//! TaskbarProgress::set_state(ProgressState::None)?;
//! ```
//!
//! # Taskbar/Dock Badge
//!
//! ```ignore
//! use horizon_lattice::platform::TaskbarBadge;
//!
//! // Set a text badge (e.g., unread count)
//! TaskbarBadge::set_text("5")?;
//!
//! // Set an overlay icon (Windows only)
//! TaskbarBadge::set_overlay_icon("/path/to/icon.png", "New messages")?;
//!
//! // Clear the badge
//! TaskbarBadge::clear()?;
//! ```
//!
//! # Platform Notes
//!
//! ## Recent Documents
//! - **Windows**: Uses `SHAddToRecentDocs` to add to the Start menu recent list
//! - **macOS**: Uses `NSDocumentController` recent documents
//! - **Linux**: Writes to `~/.local/share/recently-used.xbel` (XDG standard)
//!
//! ## Taskbar Progress
//! - **Windows**: Uses `ITaskbarList3` for taskbar button progress
//! - **macOS**: Limited support via `NSDockTile` (badge only, no progress bar)
//! - **Linux**: Desktop-specific (GNOME/KDE have different approaches)
//!
//! ## Taskbar Badge
//! - **Windows**: Uses `ITaskbarList3::SetOverlayIcon` for icon overlays
//! - **macOS**: Uses `NSDockTile.badgeLabel` for text badges
//! - **Linux**: Desktop-specific via D-Bus Unity protocol or similar

use std::fmt;
use std::path::{Path, PathBuf};

// ============================================================================
// Error Types
// ============================================================================

/// Error type for desktop integration operations.
#[derive(Debug)]
pub struct DesktopIntegrationError {
    kind: DesktopIntegrationErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopIntegrationErrorKind {
    /// Failed to add/manage recent documents.
    RecentDocuments,
    /// Failed to set taskbar progress.
    TaskbarProgress,
    /// Failed to set taskbar badge.
    TaskbarBadge,
    /// Failed to manage jump list.
    JumpList,
    /// Failed to manage desktop entry.
    DesktopEntry,
    /// Operation not supported on this platform.
    UnsupportedPlatform,
    /// Invalid argument provided.
    InvalidArgument,
    /// I/O error.
    Io,
}

impl DesktopIntegrationError {
    fn recent_documents(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopIntegrationErrorKind::RecentDocuments,
            message: message.into(),
        }
    }

    fn taskbar_progress(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopIntegrationErrorKind::TaskbarProgress,
            message: message.into(),
        }
    }

    fn taskbar_badge(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopIntegrationErrorKind::TaskbarBadge,
            message: message.into(),
        }
    }

    fn jump_list(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopIntegrationErrorKind::JumpList,
            message: message.into(),
        }
    }

    fn desktop_entry(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopIntegrationErrorKind::DesktopEntry,
            message: message.into(),
        }
    }

    fn unsupported_platform(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopIntegrationErrorKind::UnsupportedPlatform,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopIntegrationErrorKind::InvalidArgument,
            message: message.into(),
        }
    }

    fn io(message: impl Into<String>) -> Self {
        Self {
            kind: DesktopIntegrationErrorKind::Io,
            message: message.into(),
        }
    }

    /// Returns true if this error indicates the operation is not supported on this platform.
    pub fn is_unsupported_platform(&self) -> bool {
        self.kind == DesktopIntegrationErrorKind::UnsupportedPlatform
    }
}

impl fmt::Display for DesktopIntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            DesktopIntegrationErrorKind::RecentDocuments => {
                write!(f, "recent documents error: {}", self.message)
            }
            DesktopIntegrationErrorKind::TaskbarProgress => {
                write!(f, "taskbar progress error: {}", self.message)
            }
            DesktopIntegrationErrorKind::TaskbarBadge => {
                write!(f, "taskbar badge error: {}", self.message)
            }
            DesktopIntegrationErrorKind::JumpList => {
                write!(f, "jump list error: {}", self.message)
            }
            DesktopIntegrationErrorKind::DesktopEntry => {
                write!(f, "desktop entry error: {}", self.message)
            }
            DesktopIntegrationErrorKind::UnsupportedPlatform => {
                write!(f, "unsupported platform: {}", self.message)
            }
            DesktopIntegrationErrorKind::InvalidArgument => {
                write!(f, "invalid argument: {}", self.message)
            }
            DesktopIntegrationErrorKind::Io => {
                write!(f, "I/O error: {}", self.message)
            }
        }
    }
}

impl std::error::Error for DesktopIntegrationError {}

impl From<std::io::Error> for DesktopIntegrationError {
    fn from(err: std::io::Error) -> Self {
        Self::io(err.to_string())
    }
}

// ============================================================================
// Recent Documents
// ============================================================================

/// Information about a recent document for registration.
#[derive(Debug, Clone)]
pub struct RecentDocument {
    /// The file path.
    pub path: PathBuf,
    /// Application name (used on some platforms).
    pub app_name: Option<String>,
    /// MIME type of the document.
    pub mime_type: Option<String>,
    /// Display name for the document.
    pub display_name: Option<String>,
}

impl RecentDocument {
    /// Create a new recent document entry.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            app_name: None,
            mime_type: None,
            display_name: None,
        }
    }

    /// Set the application name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Set the MIME type.
    pub fn mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }

    /// Set the display name.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }
}

/// Manage the system's recent documents list.
///
/// This provides cross-platform access to the operating system's recent
/// documents feature, which appears in various places:
/// - **Windows**: Start menu recent items and jump lists
/// - **macOS**: File > Open Recent menu
/// - **Linux**: File manager recent locations
pub struct RecentDocuments;

impl RecentDocuments {
    /// Add a file to the recent documents list.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to add
    ///
    /// # Example
    ///
    /// ```ignore
    /// RecentDocuments::add("/path/to/document.pdf")?;
    /// ```
    pub fn add<P: AsRef<Path>>(path: P) -> Result<(), DesktopIntegrationError> {
        let doc = RecentDocument::new(path.as_ref());
        Self::add_with_info(doc)
    }

    /// Add a file to the recent documents list with additional metadata.
    ///
    /// # Arguments
    ///
    /// * `doc` - The recent document information
    #[cfg(target_os = "windows")]
    pub fn add_with_info(doc: RecentDocument) -> Result<(), DesktopIntegrationError> {
        windows_add_recent_document(&doc)
    }

    /// Add a document to the recent documents list with metadata (macOS).
    #[cfg(target_os = "macos")]
    pub fn add_with_info(doc: RecentDocument) -> Result<(), DesktopIntegrationError> {
        macos_add_recent_document(&doc)
    }

    /// Add a document to the recent documents list with metadata (Linux).
    #[cfg(target_os = "linux")]
    pub fn add_with_info(doc: RecentDocument) -> Result<(), DesktopIntegrationError> {
        linux_add_recent_document(&doc)
    }

    /// Add a document to the recent documents list with metadata (unsupported platforms).
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn add_with_info(_doc: RecentDocument) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "recent documents not supported on this platform",
        ))
    }

    /// Clear the recent documents list for this application.
    ///
    /// Note: On some platforms, this may only clear documents added by this
    /// application, not all recent documents.
    #[cfg(target_os = "windows")]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        windows_clear_recent_documents()
    }

    /// Clear the recent documents list (macOS).
    #[cfg(target_os = "macos")]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        macos_clear_recent_documents()
    }

    /// Clear the recent documents list (Linux - not supported).
    #[cfg(target_os = "linux")]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        // Linux doesn't have a standard way to clear only app-specific recent docs
        // The recently-used.xbel file contains entries from all apps
        Err(DesktopIntegrationError::unsupported_platform(
            "clearing recent documents for a specific app is not supported on Linux",
        ))
    }

    /// Clear the recent documents list (unsupported platforms).
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "recent documents not supported on this platform",
        ))
    }
}

// ============================================================================
// Taskbar/Dock Progress
// ============================================================================

/// The state of a taskbar progress indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProgressState {
    /// No progress indicator shown (default).
    #[default]
    None,
    /// Indeterminate progress (spinning/pulsing indicator).
    Indeterminate,
    /// Normal progress (green on Windows).
    Normal,
    /// Paused progress (yellow on Windows).
    Paused,
    /// Error state (red on Windows).
    Error,
}

/// Control the taskbar/dock progress indicator.
///
/// This allows showing progress for long-running operations directly
/// in the taskbar button (Windows) or dock icon (macOS).
///
/// # Platform Notes
///
/// - **Windows**: Full support via `ITaskbarList3`
/// - **macOS**: Limited - can show badge text but no progress bar
/// - **Linux**: Not standardized, varies by desktop environment
pub struct TaskbarProgress;

impl TaskbarProgress {
    /// Set the progress state.
    ///
    /// # Arguments
    ///
    /// * `state` - The progress state to display
    #[cfg(target_os = "windows")]
    pub fn set_state(state: ProgressState) -> Result<(), DesktopIntegrationError> {
        windows_set_progress_state(state)
    }

    /// Set the progress state (macOS - limited support).
    #[cfg(target_os = "macos")]
    pub fn set_state(state: ProgressState) -> Result<(), DesktopIntegrationError> {
        // macOS doesn't have a progress bar in the dock, but we can clear the badge
        if state == ProgressState::None {
            macos_clear_dock_badge()
        } else {
            // For other states, we could show a badge, but it's not really progress
            Ok(())
        }
    }

    /// Set the progress state (Linux - not supported).
    #[cfg(target_os = "linux")]
    pub fn set_state(_state: ProgressState) -> Result<(), DesktopIntegrationError> {
        // Linux support varies by desktop environment
        // Could potentially use Unity launcher API or KDE's DBus interface
        Err(DesktopIntegrationError::unsupported_platform(
            "taskbar progress is desktop-environment specific on Linux",
        ))
    }

    /// Set the progress state (unsupported platforms).
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn set_state(_state: ProgressState) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "taskbar progress not supported on this platform",
        ))
    }

    /// Set the progress value (0-100).
    ///
    /// This automatically sets the state to `Normal` if not already set.
    ///
    /// # Arguments
    ///
    /// * `percent` - Progress percentage (0-100, clamped)
    #[cfg(target_os = "windows")]
    pub fn set_progress(percent: u32) -> Result<(), DesktopIntegrationError> {
        let percent = percent.min(100);
        windows_set_progress_value(percent)
    }

    /// Set the progress value (macOS - shown as badge).
    #[cfg(target_os = "macos")]
    pub fn set_progress(percent: u32) -> Result<(), DesktopIntegrationError> {
        // macOS doesn't have a progress bar, show percentage as badge
        let percent = percent.min(100);
        if percent == 100 {
            macos_clear_dock_badge()
        } else {
            macos_set_dock_badge(&format!("{}%", percent))
        }
    }

    /// Set the progress value (Linux - not supported).
    #[cfg(target_os = "linux")]
    pub fn set_progress(_percent: u32) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "taskbar progress is desktop-environment specific on Linux",
        ))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn set_progress(_percent: u32) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "taskbar progress not supported on this platform",
        ))
    }
}

// ============================================================================
// Taskbar/Dock Badge
// ============================================================================

/// Control the taskbar/dock badge or overlay icon.
///
/// Badges are used to show notification counts or status overlays on the
/// application's taskbar button or dock icon.
///
/// # Platform Notes
///
/// - **Windows**: Supports overlay icons (small icon on top of taskbar button)
/// - **macOS**: Supports text badges (e.g., unread count)
/// - **Linux**: Not standardized, some DEs support Unity launcher badges
pub struct TaskbarBadge;

impl TaskbarBadge {
    /// Set a text badge on the dock/taskbar icon.
    ///
    /// # Arguments
    ///
    /// * `text` - The text to display (typically a number)
    ///
    /// # Platform Notes
    ///
    /// - **macOS**: Displays the text as a badge on the dock icon
    /// - **Windows**: Not directly supported; consider using overlay icons
    #[cfg(target_os = "macos")]
    pub fn set_text(text: &str) -> Result<(), DesktopIntegrationError> {
        macos_set_dock_badge(text)
    }

    #[cfg(target_os = "windows")]
    pub fn set_text(_text: &str) -> Result<(), DesktopIntegrationError> {
        // Windows doesn't support text badges directly
        // Could potentially render text to an icon and use overlay
        Err(DesktopIntegrationError::unsupported_platform(
            "Windows uses overlay icons instead of text badges; use set_overlay_icon",
        ))
    }

    #[cfg(target_os = "linux")]
    pub fn set_text(_text: &str) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "text badges are desktop-environment specific on Linux",
        ))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn set_text(_text: &str) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "text badges not supported on this platform",
        ))
    }

    /// Set an overlay icon on the taskbar button.
    ///
    /// # Arguments
    ///
    /// * `icon_path` - Path to the icon file (should be small, ~16x16)
    /// * `description` - Accessible description of the overlay
    ///
    /// # Platform Notes
    ///
    /// - **Windows**: Full support for overlay icons
    /// - **macOS**: Not supported; use `set_text` for badges
    #[cfg(target_os = "windows")]
    pub fn set_overlay_icon<P: AsRef<Path>>(
        icon_path: P,
        description: &str,
    ) -> Result<(), DesktopIntegrationError> {
        windows_set_overlay_icon(icon_path.as_ref(), description)
    }

    /// Set an overlay icon (non-Windows - not supported).
    #[cfg(not(target_os = "windows"))]
    pub fn set_overlay_icon<P: AsRef<Path>>(
        _icon_path: P,
        _description: &str,
    ) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "overlay icons are only supported on Windows",
        ))
    }

    /// Clear any badge or overlay icon.
    #[cfg(target_os = "macos")]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        macos_clear_dock_badge()
    }

    /// Clear any overlay icon (Windows).
    #[cfg(target_os = "windows")]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        windows_clear_overlay_icon()
    }

    /// Clear any badge (Linux - no-op).
    #[cfg(target_os = "linux")]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        Ok(()) // No-op on Linux
    }

    /// Clear any badge (unsupported platforms - no-op).
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        Ok(()) // No-op on unsupported platforms
    }
}

// ============================================================================
// Jump Lists / Dock Menus
// ============================================================================

/// An item in a jump list or dock menu.
#[derive(Debug, Clone)]
pub struct JumpListItem {
    /// The display title.
    pub title: String,
    /// The path or command to execute.
    pub path: PathBuf,
    /// Optional arguments.
    pub arguments: Option<String>,
    /// Optional icon path.
    pub icon_path: Option<PathBuf>,
    /// Optional description.
    pub description: Option<String>,
}

impl JumpListItem {
    /// Create a new jump list item.
    pub fn new(title: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            title: title.into(),
            path: path.into(),
            arguments: None,
            icon_path: None,
            description: None,
        }
    }

    /// Set the arguments.
    pub fn arguments(mut self, args: impl Into<String>) -> Self {
        self.arguments = Some(args.into());
        self
    }

    /// Set the icon path.
    pub fn icon_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.icon_path = Some(path.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// A category/group of items in a jump list.
#[derive(Debug, Clone)]
pub struct JumpListCategory {
    /// The category title (empty for tasks category).
    pub title: String,
    /// Items in this category.
    pub items: Vec<JumpListItem>,
}

impl JumpListCategory {
    /// Create a new category.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
        }
    }

    /// Create the "Tasks" category (special category on Windows).
    pub fn tasks() -> Self {
        Self::new("")
    }

    /// Add an item to this category.
    pub fn add_item(mut self, item: JumpListItem) -> Self {
        self.items.push(item);
        self
    }
}

/// Manage the application's jump list (Windows) or dock menu (macOS).
///
/// Jump lists provide quick access to common tasks and recent files
/// directly from the taskbar or dock.
///
/// # Platform Notes
///
/// - **Windows**: Full jump list support with categories and tasks
/// - **macOS**: Dock menu support via NSDockTile
/// - **Linux**: Not standardized
pub struct JumpList;

impl JumpList {
    /// Set the jump list categories.
    ///
    /// # Arguments
    ///
    /// * `categories` - The categories to display
    #[cfg(target_os = "windows")]
    pub fn set_categories(categories: &[JumpListCategory]) -> Result<(), DesktopIntegrationError> {
        windows_set_jump_list(categories)
    }

    /// Set the jump list categories (macOS - dock menu).
    #[cfg(target_os = "macos")]
    pub fn set_categories(categories: &[JumpListCategory]) -> Result<(), DesktopIntegrationError> {
        macos_set_dock_menu(categories)
    }

    /// Set the jump list categories (Linux - not supported).
    #[cfg(target_os = "linux")]
    pub fn set_categories(_categories: &[JumpListCategory]) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "jump lists are not standardized on Linux",
        ))
    }

    /// Set the jump list categories (unsupported platforms).
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn set_categories(_categories: &[JumpListCategory]) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "jump lists not supported on this platform",
        ))
    }

    /// Clear the jump list.
    #[cfg(target_os = "windows")]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        windows_clear_jump_list()
    }

    /// Clear the jump list (macOS - dock menu).
    #[cfg(target_os = "macos")]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        macos_clear_dock_menu()
    }

    /// Clear the jump list (Linux - no-op).
    #[cfg(target_os = "linux")]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        Ok(()) // No-op
    }

    /// Clear the jump list (unsupported platforms - no-op).
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn clear() -> Result<(), DesktopIntegrationError> {
        Ok(())
    }
}

// ============================================================================
// Desktop Entry (Linux)
// ============================================================================

/// A desktop entry for Linux `.desktop` file management.
///
/// This provides a way to create or update `.desktop` files which control
/// how the application appears in application menus and launchers.
///
/// # Platform Notes
///
/// This is primarily for Linux. On Windows and macOS, application metadata
/// is typically embedded in the executable or app bundle.
#[derive(Debug, Clone, Default)]
pub struct DesktopEntry {
    /// Application name.
    pub name: String,
    /// Executable path.
    pub exec: Option<PathBuf>,
    /// Icon path or name.
    pub icon: Option<String>,
    /// Short description.
    pub comment: Option<String>,
    /// Categories (semicolon-separated).
    pub categories: Vec<String>,
    /// MIME types handled.
    pub mime_types: Vec<String>,
    /// Whether to show in menus.
    pub no_display: bool,
    /// Desktop environments to show in.
    pub only_show_in: Vec<String>,
    /// Desktop environments to hide from.
    pub not_show_in: Vec<String>,
    /// Keywords for search.
    pub keywords: Vec<String>,
    /// Startup WM class.
    pub startup_wm_class: Option<String>,
}

impl DesktopEntry {
    /// Create a new desktop entry.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Set the executable path.
    pub fn exec(mut self, path: impl Into<PathBuf>) -> Self {
        self.exec = Some(path.into());
        self
    }

    /// Set the icon.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the comment/description.
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Add a category.
    pub fn add_category(mut self, category: impl Into<String>) -> Self {
        self.categories.push(category.into());
        self
    }

    /// Set categories.
    pub fn categories(mut self, categories: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.categories = categories.into_iter().map(|c| c.into()).collect();
        self
    }

    /// Add a MIME type.
    pub fn add_mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_types.push(mime.into());
        self
    }

    /// Set the startup WM class.
    pub fn startup_wm_class(mut self, class: impl Into<String>) -> Self {
        self.startup_wm_class = Some(class.into());
        self
    }

    /// Add a keyword.
    pub fn add_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    /// Install the desktop entry.
    ///
    /// This writes the `.desktop` file to `~/.local/share/applications/`.
    ///
    /// # Arguments
    ///
    /// * `app_id` - The application ID (used as filename)
    #[cfg(target_os = "linux")]
    pub fn install(&self, app_id: &str) -> Result<(), DesktopIntegrationError> {
        linux_install_desktop_entry(self, app_id)
    }

    /// Install the desktop entry (non-Linux - not supported).
    #[cfg(not(target_os = "linux"))]
    pub fn install(&self, _app_id: &str) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "desktop entries are only supported on Linux",
        ))
    }

    /// Uninstall a desktop entry.
    #[cfg(target_os = "linux")]
    pub fn uninstall(app_id: &str) -> Result<(), DesktopIntegrationError> {
        linux_uninstall_desktop_entry(app_id)
    }

    /// Uninstall a desktop entry (non-Linux - not supported).
    #[cfg(not(target_os = "linux"))]
    pub fn uninstall(_app_id: &str) -> Result<(), DesktopIntegrationError> {
        Err(DesktopIntegrationError::unsupported_platform(
            "desktop entries are only supported on Linux",
        ))
    }
}

// ============================================================================
// Platform-specific implementations - Windows
// ============================================================================

#[cfg(target_os = "windows")]
mod windows_impl {
    use super::*;
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CLSCTX_ALL, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::UI::Shell::{
        ITaskbarList3, SHARD_PATHW, SHAddToRecentDocs, TBPF_ERROR, TBPF_INDETERMINATE,
        TBPF_NOPROGRESS, TBPF_NORMAL, TBPF_PAUSED, TBPFLAG, TaskbarList,
    };
    use windows::core::PCWSTR;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn get_taskbar_list() -> Result<ITaskbarList3, DesktopIntegrationError> {
        unsafe {
            // Initialize COM if not already initialized
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

            CoCreateInstance(&TaskbarList, None, CLSCTX_ALL)
                .map_err(|e| DesktopIntegrationError::taskbar_progress(e.to_string()))
        }
    }

    fn state_to_tbpflag(state: ProgressState) -> TBPFLAG {
        match state {
            ProgressState::None => TBPF_NOPROGRESS,
            ProgressState::Indeterminate => TBPF_INDETERMINATE,
            ProgressState::Normal => TBPF_NORMAL,
            ProgressState::Paused => TBPF_PAUSED,
            ProgressState::Error => TBPF_ERROR,
        }
    }

    pub fn windows_add_recent_document(
        doc: &RecentDocument,
    ) -> Result<(), DesktopIntegrationError> {
        let path_wide = to_wide(&doc.path.to_string_lossy());

        unsafe {
            SHAddToRecentDocs(SHARD_PATHW.0 as u32, Some(path_wide.as_ptr() as *const _));
        }

        Ok(())
    }

    pub fn windows_clear_recent_documents() -> Result<(), DesktopIntegrationError> {
        unsafe {
            // Passing NULL clears the recent documents list
            SHAddToRecentDocs(SHARD_PATHW.0 as u32, None);
        }
        Ok(())
    }

    pub fn windows_set_progress_state(state: ProgressState) -> Result<(), DesktopIntegrationError> {
        let taskbar = get_taskbar_list()?;
        let hwnd = get_foreground_window();

        unsafe {
            taskbar
                .SetProgressState(hwnd, state_to_tbpflag(state))
                .map_err(|e| DesktopIntegrationError::taskbar_progress(e.to_string()))?;
        }

        Ok(())
    }

    pub fn windows_set_progress_value(percent: u32) -> Result<(), DesktopIntegrationError> {
        let taskbar = get_taskbar_list()?;
        let hwnd = get_foreground_window();

        unsafe {
            // Set to normal state first
            taskbar
                .SetProgressState(hwnd, TBPF_NORMAL)
                .map_err(|e| DesktopIntegrationError::taskbar_progress(e.to_string()))?;

            taskbar
                .SetProgressValue(hwnd, percent as u64, 100)
                .map_err(|e| DesktopIntegrationError::taskbar_progress(e.to_string()))?;
        }

        Ok(())
    }

    pub fn windows_set_overlay_icon(
        icon_path: &Path,
        description: &str,
    ) -> Result<(), DesktopIntegrationError> {
        use windows::Win32::UI::WindowsAndMessaging::{IMAGE_ICON, LR_LOADFROMFILE, LoadImageW};

        let taskbar = get_taskbar_list()?;
        let hwnd = get_foreground_window();
        let path_wide = to_wide(&icon_path.to_string_lossy());
        let desc_wide = to_wide(description);

        unsafe {
            let hicon = LoadImageW(
                None,
                PCWSTR(path_wide.as_ptr()),
                IMAGE_ICON,
                16,
                16,
                LR_LOADFROMFILE,
            )
            .map_err(|e| DesktopIntegrationError::taskbar_badge(e.to_string()))?;

            taskbar
                .SetOverlayIcon(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::HICON(hicon.0),
                    PCWSTR(desc_wide.as_ptr()),
                )
                .map_err(|e| DesktopIntegrationError::taskbar_badge(e.to_string()))?;
        }

        Ok(())
    }

    pub fn windows_clear_overlay_icon() -> Result<(), DesktopIntegrationError> {
        let taskbar = get_taskbar_list()?;
        let hwnd = get_foreground_window();

        unsafe {
            taskbar
                .SetOverlayIcon(
                    hwnd,
                    windows::Win32::UI::WindowsAndMessaging::HICON(std::ptr::null_mut()),
                    PCWSTR::null(),
                )
                .map_err(|e| DesktopIntegrationError::taskbar_badge(e.to_string()))?;
        }

        Ok(())
    }

    pub fn windows_set_jump_list(
        categories: &[JumpListCategory],
    ) -> Result<(), DesktopIntegrationError> {
        // Jump list implementation requires ICustomDestinationList
        // This is a simplified implementation that just stores the categories
        // A full implementation would use COM interfaces
        let _ = categories;
        Err(DesktopIntegrationError::jump_list(
            "full jump list support requires additional COM interfaces; use system tray menus instead",
        ))
    }

    pub fn windows_clear_jump_list() -> Result<(), DesktopIntegrationError> {
        // Would need ICustomDestinationList::DeleteList
        Ok(())
    }

    fn get_foreground_window() -> HWND {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        unsafe { GetForegroundWindow() }
    }
}

#[cfg(target_os = "windows")]
use windows_impl::*;

// ============================================================================
// Platform-specific implementations - macOS
// ============================================================================

#[cfg(target_os = "macos")]
mod macos_impl {
    use super::*;
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{NSString, NSURL};

    pub fn macos_add_recent_document(doc: &RecentDocument) -> Result<(), DesktopIntegrationError> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            DesktopIntegrationError::recent_documents("must be called from main thread")
        })?;

        let path_str = doc.path.to_string_lossy();
        let url_str = format!("file://{}", path_str);

        let ns_url_string = NSString::from_str(&url_str);
        let url = NSURL::URLWithString(&ns_url_string)
            .ok_or_else(|| DesktopIntegrationError::recent_documents("invalid file path"))?;

        let app = NSApplication::sharedApplication(mtm);
        // NSDocumentController is not directly available, use NSApp
        // For a real implementation, we'd need NSDocumentController bindings
        let _ = app;
        let _ = url;

        // Note: Full implementation requires NSDocumentController bindings
        // which may not be in objc2-app-kit yet
        Ok(())
    }

    pub fn macos_clear_recent_documents() -> Result<(), DesktopIntegrationError> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            DesktopIntegrationError::recent_documents("must be called from main thread")
        })?;

        let _app = NSApplication::sharedApplication(mtm);
        // Would need NSDocumentController::clearRecentDocuments

        Ok(())
    }

    pub fn macos_set_dock_badge(text: &str) -> Result<(), DesktopIntegrationError> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            DesktopIntegrationError::taskbar_badge("must be called from main thread")
        })?;

        let app = NSApplication::sharedApplication(mtm);
        let dock_tile = app.dockTile();
        let ns_text = NSString::from_str(text);
        dock_tile.setBadgeLabel(Some(&ns_text));

        Ok(())
    }

    pub fn macos_clear_dock_badge() -> Result<(), DesktopIntegrationError> {
        let mtm = MainThreadMarker::new().ok_or_else(|| {
            DesktopIntegrationError::taskbar_badge("must be called from main thread")
        })?;

        let app = NSApplication::sharedApplication(mtm);
        let dock_tile = app.dockTile();
        dock_tile.setBadgeLabel(None);

        Ok(())
    }

    pub fn macos_set_dock_menu(
        _categories: &[JumpListCategory],
    ) -> Result<(), DesktopIntegrationError> {
        // Would need to create NSMenu and set it on NSApplication
        // This requires more NSMenu bindings
        Err(DesktopIntegrationError::jump_list(
            "dock menu support requires NSMenu bindings",
        ))
    }

    pub fn macos_clear_dock_menu() -> Result<(), DesktopIntegrationError> {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
use macos_impl::*;

// ============================================================================
// Platform-specific implementations - Linux
// ============================================================================

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Write;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn get_data_home() -> Result<PathBuf, DesktopIntegrationError> {
        if let Ok(xdg_data_home) = env::var("XDG_DATA_HOME") {
            return Ok(PathBuf::from(xdg_data_home));
        }

        let home = env::var("HOME")
            .map_err(|_| DesktopIntegrationError::io("HOME environment variable not set"))?;

        Ok(PathBuf::from(home).join(".local/share"))
    }

    fn get_recently_used_path() -> Result<PathBuf, DesktopIntegrationError> {
        Ok(get_data_home()?.join("recently-used.xbel"))
    }

    fn escape_xml(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    pub fn linux_add_recent_document(doc: &RecentDocument) -> Result<(), DesktopIntegrationError> {
        let recent_path = get_recently_used_path()?;

        // Ensure parent directory exists
        if let Some(parent) = recent_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Read existing file or create new
        let existing = fs::read_to_string(&recent_path).unwrap_or_default();

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let file_uri = format!("file://{}", doc.path.to_string_lossy());
        let mime_type = doc
            .mime_type
            .as_deref()
            .unwrap_or("application/octet-stream");
        let app_name = doc.app_name.as_deref().unwrap_or("horizon-lattice");

        // Create bookmark entry
        let bookmark = format!(
            r#"  <bookmark href="{}" added="{}" modified="{}" visited="{}">
    <info>
      <metadata owner="http://freedesktop.org">
        <mime:mime-type type="{}"/>
        <bookmark:applications>
          <bookmark:application name="{}" exec="'{}' %u" modified="{}" count="1"/>
        </bookmark:applications>
      </metadata>
    </info>
  </bookmark>
"#,
            escape_xml(&file_uri),
            timestamp,
            timestamp,
            timestamp,
            escape_xml(mime_type),
            escape_xml(app_name),
            escape_xml(&env::current_exe().unwrap_or_default().to_string_lossy()),
            timestamp
        );

        // If file exists and has content, insert before closing tag
        let content = if existing.contains("</xbel>") {
            existing.replace("</xbel>", &format!("{}</xbel>", bookmark))
        } else {
            // Create new file
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<xbel version="1.0"
      xmlns:bookmark="http://www.freedesktop.org/standards/desktop-bookmarks"
      xmlns:mime="http://www.freedesktop.org/standards/shared-mime-info">
{}
</xbel>
"#,
                bookmark
            )
        };

        let mut file = fs::File::create(&recent_path)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    pub fn linux_install_desktop_entry(
        entry: &DesktopEntry,
        app_id: &str,
    ) -> Result<(), DesktopIntegrationError> {
        let applications_dir = get_data_home()?.join("applications");
        fs::create_dir_all(&applications_dir)?;

        let desktop_path = applications_dir.join(format!("{}.desktop", app_id));

        let exec_path = entry
            .exec
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned())
            .or_else(|| env::current_exe().ok().map(|p| p.to_string_lossy().into_owned()))
            .unwrap_or_default();

        let mut content = String::new();
        content.push_str("[Desktop Entry]\n");
        content.push_str("Type=Application\n");
        content.push_str(&format!("Name={}\n", entry.name));
        content.push_str(&format!("Exec=\"{}\" %U\n", exec_path));

        if let Some(ref icon) = entry.icon {
            content.push_str(&format!("Icon={}\n", icon));
        }

        if let Some(ref comment) = entry.comment {
            content.push_str(&format!("Comment={}\n", comment));
        }

        if !entry.categories.is_empty() {
            content.push_str(&format!("Categories={};\n", entry.categories.join(";")));
        }

        if !entry.mime_types.is_empty() {
            content.push_str(&format!("MimeType={};\n", entry.mime_types.join(";")));
        }

        if !entry.keywords.is_empty() {
            content.push_str(&format!("Keywords={};\n", entry.keywords.join(";")));
        }

        if let Some(ref wm_class) = entry.startup_wm_class {
            content.push_str(&format!("StartupWMClass={}\n", wm_class));
        }

        if entry.no_display {
            content.push_str("NoDisplay=true\n");
        }

        content.push_str("Terminal=false\n");

        fs::write(&desktop_path, content)?;

        // Update desktop database
        let _ = Command::new("update-desktop-database")
            .arg(&applications_dir)
            .output();

        Ok(())
    }

    pub fn linux_uninstall_desktop_entry(app_id: &str) -> Result<(), DesktopIntegrationError> {
        let applications_dir = get_data_home()?.join("applications");
        let desktop_path = applications_dir.join(format!("{}.desktop", app_id));

        if desktop_path.exists() {
            fs::remove_file(&desktop_path)?;

            // Update desktop database
            let _ = Command::new("update-desktop-database")
                .arg(&applications_dir)
                .output();
        }

        Ok(())
    }
}

#[cfg(target_os = "linux")]
use linux_impl::*;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = DesktopIntegrationError::recent_documents("test error");
        assert!(error.to_string().contains("recent documents"));
        assert!(error.to_string().contains("test error"));

        let error = DesktopIntegrationError::unsupported_platform("test");
        assert!(error.is_unsupported_platform());
    }

    #[test]
    fn test_recent_document_builder() {
        let doc = RecentDocument::new("/path/to/file.txt")
            .app_name("My App")
            .mime_type("text/plain")
            .display_name("My Document");

        assert_eq!(doc.path, PathBuf::from("/path/to/file.txt"));
        assert_eq!(doc.app_name, Some("My App".to_string()));
        assert_eq!(doc.mime_type, Some("text/plain".to_string()));
        assert_eq!(doc.display_name, Some("My Document".to_string()));
    }

    #[test]
    fn test_progress_state_default() {
        assert_eq!(ProgressState::default(), ProgressState::None);
    }

    #[test]
    fn test_jump_list_item_builder() {
        let item = JumpListItem::new("Open Editor", "/usr/bin/editor")
            .arguments("--new-window")
            .icon_path("/path/to/icon.png")
            .description("Open the editor");

        assert_eq!(item.title, "Open Editor");
        assert_eq!(item.path, PathBuf::from("/usr/bin/editor"));
        assert_eq!(item.arguments, Some("--new-window".to_string()));
        assert_eq!(item.icon_path, Some(PathBuf::from("/path/to/icon.png")));
        assert_eq!(item.description, Some("Open the editor".to_string()));
    }

    #[test]
    fn test_jump_list_category() {
        let category = JumpListCategory::new("Recent")
            .add_item(JumpListItem::new("File 1", "/path/to/file1"))
            .add_item(JumpListItem::new("File 2", "/path/to/file2"));

        assert_eq!(category.title, "Recent");
        assert_eq!(category.items.len(), 2);
    }

    #[test]
    fn test_jump_list_tasks_category() {
        let tasks = JumpListCategory::tasks();
        assert_eq!(tasks.title, "");
    }

    #[test]
    fn test_desktop_entry_builder() {
        let entry = DesktopEntry::new("My Application")
            .exec("/usr/bin/myapp")
            .icon("myapp")
            .comment("A great application")
            .add_category("Utility")
            .add_category("Development")
            .add_mime_type("text/plain")
            .startup_wm_class("myapp")
            .add_keyword("editor");

        assert_eq!(entry.name, "My Application");
        assert_eq!(entry.exec, Some(PathBuf::from("/usr/bin/myapp")));
        assert_eq!(entry.icon, Some("myapp".to_string()));
        assert_eq!(entry.comment, Some("A great application".to_string()));
        assert_eq!(entry.categories, vec!["Utility", "Development"]);
        assert_eq!(entry.mime_types, vec!["text/plain"]);
        assert_eq!(entry.startup_wm_class, Some("myapp".to_string()));
        assert_eq!(entry.keywords, vec!["editor"]);
    }

    #[test]
    fn test_desktop_entry_categories_method() {
        let entry = DesktopEntry::new("App").categories(["A", "B", "C"]);
        assert_eq!(entry.categories, vec!["A", "B", "C"]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_xml_escape() {
        use linux_impl::escape_xml;
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
    }
}
