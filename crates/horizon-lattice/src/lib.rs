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

pub use horizon_lattice_core::*;
pub use horizon_lattice_macros::*;

/// Graphics rendering module.
pub mod render {
    pub use horizon_lattice_render::*;
}
