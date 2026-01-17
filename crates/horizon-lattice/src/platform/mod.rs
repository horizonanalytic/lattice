//! Platform services and system integration.
//!
//! This module provides cross-platform abstractions for system-level functionality
//! such as clipboard access, notifications, and desktop integration.
//!
//! # Clipboard
//!
//! The clipboard module provides copy/paste functionality:
//!
//! ```ignore
//! use horizon_lattice::platform::Clipboard;
//!
//! let mut clipboard = Clipboard::new()?;
//! clipboard.set_text("Copied text")?;
//! let text = clipboard.get_text()?;
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

pub use clipboard::{Clipboard, ClipboardError};
pub use high_contrast::HighContrast;
