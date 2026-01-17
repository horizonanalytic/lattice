//! Native window management module.
//!
//! This module provides native platform window creation and management,
//! bridging the Horizon Lattice widget system with the underlying windowing
//! system (winit).
//!
//! # Window Types
//!
//! Different window types have different default behaviors and platform hints:
//!
//! ```ignore
//! use horizon_lattice::window::{WindowType, NativeWindow, WindowConfig};
//!
//! // Create a normal top-level window
//! let config = WindowConfig::new("My App")
//!     .with_type(WindowType::Normal)
//!     .with_size(800, 600);
//!
//! // Create a tool window (stays on top, smaller title bar)
//! let tool_config = WindowConfig::new("Tools")
//!     .with_type(WindowType::Tool);
//!
//! // Create a splash screen (no decorations, centered)
//! let splash_config = WindowConfig::new("Splash")
//!     .with_type(WindowType::Splash)
//!     .with_size(400, 300);
//! ```
//!
//! # Window Manager
//!
//! The `WindowManager` tracks all application windows:
//!
//! ```ignore
//! use horizon_lattice::window::WindowManager;
//!
//! // Access the global window manager
//! let manager = WindowManager::instance();
//!
//! // Get all windows
//! for window in manager.windows() {
//!     println!("Window: {:?}", window.id());
//! }
//!
//! // Find window by ID
//! if let Some(window) = manager.get(window_id) {
//!     window.set_title("New Title");
//! }
//! ```

mod window_type;
mod native_window;
mod window_manager;
mod window_config;
mod window_icon;

pub use window_type::WindowType;
pub use native_window::{NativeWindow, NativeWindowId};
pub use window_manager::WindowManager;
pub use window_config::WindowConfig;
pub use window_icon::WindowIcon;

// Re-export WindowFlags and related types from the widget module for convenience
pub use crate::widget::widgets::{WindowFlags, WindowModality, WindowState};
