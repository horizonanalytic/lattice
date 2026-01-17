//! Platform services and system integration.
//!
//! This module provides cross-platform abstractions for system-level functionality
//! such as clipboard access, notifications, and desktop integration.
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
mod high_contrast;

pub use clipboard::{Clipboard, ClipboardData, ClipboardError, ClipboardWatcher, ImageData};
pub use high_contrast::HighContrast;

// X11-specific exports for Linux
#[cfg(target_os = "linux")]
pub use clipboard::{X11Clipboard, X11Selection};
