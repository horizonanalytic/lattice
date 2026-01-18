//! Horizon Lattice - A Rust-native GUI framework inspired by Qt6.
//!
//! This is the main umbrella crate that re-exports all public APIs.
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

/// Graphics rendering module.
pub mod render {
    pub use horizon_lattice_render::*;
}

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
