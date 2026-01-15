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

mod clipboard;

pub use clipboard::{Clipboard, ClipboardError};
