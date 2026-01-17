//! Platform services and system integration.
//!
//! This module provides cross-platform abstractions for system-level functionality
//! such as clipboard access, notifications, file associations, and desktop integration.
//!
//! # Clipboard
//!
//! The clipboard module provides copy/paste functionality with support for multiple
//! data formats and change detection:
//!
//! ```ignore
//! use horizon_lattice::platform::{Clipboard, ClipboardWatcher, ClipboardData, ImageData};
//!
//! // Basic clipboard operations
//! let mut clipboard = Clipboard::new()?;
//! clipboard.set_text("Copied text")?;
//! let text = clipboard.get_text()?;
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
//! # Notifications
//!
//! The notifications module provides cross-platform desktop notifications:
//!
//! ```ignore
//! use horizon_lattice::platform::{Notification, Timeout, Urgency};
//!
//! // Simple notification
//! Notification::new()
//!     .summary("Download Complete")
//!     .body("Your file has been downloaded.")
//!     .show()?;
//!
//! // Notification with options
//! Notification::new()
//!     .summary("Reminder")
//!     .body("Meeting in 5 minutes")
//!     .urgency(Urgency::Critical)
//!     .timeout(Timeout::Milliseconds(10000))
//!     .show()?;
//! ```
//!
//! # File Associations
//!
//! The file associations module provides file/URL opening and registration:
//!
//! ```ignore
//! use horizon_lattice::platform::{Opener, LaunchArgs, FileTypeRegistration};
//!
//! // Open a file with the default application
//! Opener::open("document.pdf")?;
//!
//! // Open a URL in the browser
//! Opener::open_url("https://example.com")?;
//!
//! // Reveal a file in the file manager
//! Opener::reveal("/path/to/file.txt")?;
//!
//! // Parse launch arguments for files/URLs
//! let args = LaunchArgs::parse();
//! for file in args.files() {
//!     println!("Opening: {}", file.display());
//! }
//!
//! // Register file type association (Windows/Linux only)
//! FileTypeRegistration::new()
//!     .extension("myext")
//!     .description("My Application Document")
//!     .register()?;
//! ```
//!
//! # High Contrast
//!
//! The high contrast module detects accessibility contrast settings:
//!
//! ```ignore
//! use horizon_lattice::platform::HighContrast;
//!
//! if HighContrast::is_enabled() {
//!     // Use high contrast theme
//! }
//! ```

mod clipboard;
mod desktop_integration;
mod file_associations;
mod high_contrast;
#[cfg(feature = "notifications")]
mod notifications;
mod power_management;
mod session_management;

pub use clipboard::{Clipboard, ClipboardData, ClipboardError, ClipboardWatcher, ImageData};
pub use desktop_integration::{
    DesktopEntry, DesktopIntegrationError, JumpList, JumpListCategory, JumpListItem,
    ProgressState, RecentDocument, RecentDocuments, TaskbarBadge, TaskbarProgress,
};
pub use file_associations::{
    FileAssociationError, FileTypeInfo, FileTypeRegistration, LaunchArgs, Opener,
    UrlSchemeInfo, UrlSchemeRegistration,
};
pub use high_contrast::HighContrast;
pub use power_management::{
    BatteryInfo, BatteryState, PowerEventReason, PowerEventWatcher, PowerManagementError,
    PowerSource, PowerState, SleepInhibitOptions, SleepInhibitor, SleepInhibitorGuard,
};
pub use session_management::{
    ApplicationState, SessionEndReason, SessionEventWatcher, SessionInhibitOptions,
    SessionInhibitor, SessionInhibitorGuard, SessionManagementError, StateLocation,
};

// Notification exports
#[cfg(feature = "notifications")]
pub use notifications::{
    CloseReason, Notification, NotificationError, NotificationHandle, Timeout, Urgency,
};

#[cfg(all(feature = "notifications", feature = "notification-actions"))]
pub use notifications::NotificationAction;

// X11-specific exports for Linux
#[cfg(target_os = "linux")]
pub use clipboard::{X11Clipboard, X11Selection};
