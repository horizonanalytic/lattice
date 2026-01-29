//! Horizon Lattice - A Rust-native GUI framework inspired by Qt6.
//!
//! This is the main umbrella crate that re-exports all public APIs.

#![warn(missing_docs)]
// Framework crate: many public API items are not exercised internally
#![allow(dead_code)]
// Allow pre-existing clippy lints - many have complex signatures needed for widget APIs
#![allow(clippy::too_many_arguments)]
// Widget APIs often use complex callback types
#![allow(clippy::type_complexity)]
// Test code commonly uses this pattern
#![allow(clippy::field_reassign_with_default)]
// Common in widget rendering code
#![allow(clippy::collapsible_if)]
// Framework methods may shadow std trait names intentionally
#![allow(clippy::should_implement_trait)]
// Widget APIs use closures for flexibility
#![allow(clippy::redundant_closure)]
// Common pattern for optional constraint handling
#![allow(clippy::unnecessary_unwrap)]
// Unit returns are common for init functions
#![allow(clippy::let_unit_value)]
// Borrowed expressions often work with generic APIs
#![allow(clippy::needless_borrows_for_generic_args)]
// Large enums are acceptable for event/error types
#![allow(clippy::large_enum_variant)]
// Some if blocks are intentionally similar for readability
#![allow(clippy::if_same_then_else)]
// Module naming follows Qt conventions
#![allow(clippy::module_inception)]
// Doc formatting is intentional
#![allow(clippy::doc_overindented_list_items)]
// Manual prefix stripping for specific patterns
#![allow(clippy::manual_strip)]
// Manual clamp patterns for clarity
#![allow(clippy::manual_clamp)]
// File options patterns
#![allow(clippy::ineffective_open_options)]
// Loop indexing patterns
#![allow(clippy::needless_range_loop)]
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice::Application;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = Application::new()?;
//!     // Create windows and widgets here...
//!     Ok(app.run()?)
//! }
//! ```
//!
//! # Widgets
//!
//! The widget system provides the foundation for building user interfaces:
//!
//! ```ignore
//! use horizon_lattice::widget::{Widget, WidgetBase, SizeHint, PaintContext};
//!
//! struct MyWidget {
//!     base: WidgetBase,
//! }
//!
//! impl Widget for MyWidget {
//!     fn widget_base(&self) -> &WidgetBase { &self.base }
//!     fn widget_base_mut(&mut self) -> &mut WidgetBase { &mut self.base }
//!
//!     fn size_hint(&self) -> SizeHint {
//!         SizeHint::from_dimensions(100.0, 100.0)
//!     }
//!
//!     fn paint(&self, ctx: &mut PaintContext<'_>) {
//!         // Draw the widget...
//!     }
//! }
//! ```

pub use horizon_lattice_core::*;
pub use horizon_lattice_macros::*;

/// Prelude module for convenient imports.
///
/// Import everything commonly needed with:
/// ```ignore
/// use horizon_lattice::prelude::*;
/// ```
pub mod prelude;

/// Graphics rendering module.
pub mod render {
    pub use horizon_lattice_render::*;
}

/// File I/O operations and utilities.
pub mod file;

/// Platform services and system integration.
pub mod platform;

/// Widget system module.
pub mod widget;

/// Model/View architecture module.
pub mod model;

/// Native window management module.
pub mod window;

/// Networking module (requires `networking` feature).
#[cfg(feature = "networking")]
pub mod net {
    pub use horizon_lattice_net::*;
}

/// Multimedia module (requires `multimedia` feature).
#[cfg(feature = "multimedia")]
pub mod multimedia {
    pub use horizon_lattice_multimedia::*;
}
