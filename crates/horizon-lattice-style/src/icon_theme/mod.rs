//! Icon theme system for Horizon Lattice.
//!
//! This module provides a complete icon theming system inspired by the freedesktop
//! icon theme specification. It includes:
//!
//! - **Standard icon names**: Following freedesktop naming conventions
//! - **Theme discovery**: Automatic discovery of installed icon themes
//! - **Inheritance**: Theme fallback chains for missing icons
//! - **Resolution**: Finding the best icon for a given name and size
//! - **Caching**: Performance-optimized icon lookup
//!
//! # Platform Support
//!
//! - **Linux**: Full freedesktop support, XDG search paths
//! - **macOS**: Application Support directories, bundle Resources
//! - **Windows**: Local app data, ProgramData directories
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_style::icon_theme::{IconResolver, IconName, IconLookup};
//! use horizon_lattice_render::IconSize;
//!
//! // Create a resolver and discover themes
//! let mut resolver = IconResolver::new();
//! resolver.discover_themes()?;
//!
//! // Look up an icon
//! let icon = resolver.get_icon(IconName::DOCUMENT_SAVE, IconSize::Size24);
//!
//! // Or use the detailed lookup API
//! let lookup = IconLookup::new("folder", IconSize::Size48)
//!     .with_scale(2)  // HiDPI
//!     .with_context(IconContext::Places);
//! let icon = resolver.resolve(&lookup);
//! ```
//!
//! # Standard Icon Names
//!
//! Common icon names are defined as constants on [`IconName`]:
//!
//! - **Actions**: `DOCUMENT_NEW`, `EDIT_COPY`, `GO_HOME`, `VIEW_REFRESH`, etc.
//! - **Status**: `DIALOG_ERROR`, `DIALOG_WARNING`, `DIALOG_INFORMATION`
//! - **Places**: `FOLDER`, `USER_HOME`, `USER_TRASH`
//! - **Devices**: `COMPUTER`, `DRIVE_HARDDISK`, `PRINTER`

mod loader;
mod resolver;
mod types;

pub use loader::IconThemeLoader;
pub use resolver::{IconResolver, lookup_standard_icon};
pub use types::{
    IconContext, IconLookup, IconName, IconSizeType, IconThemeDirectory, IconThemeInfo,
};
