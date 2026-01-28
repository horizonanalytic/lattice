//! CSS parsing module.
//!
//! This module provides CSS-like stylesheet parsing for Horizon Lattice's styling system.
//! It uses the `cssparser` crate internally for tokenization and parsing.
//!
//! # Supported Syntax
//!
//! The parser supports a subset of CSS syntax designed for widget styling:
//!
//! - **Selectors**: Type selectors (`Button`), class selectors (`.primary`),
//!   ID selectors (`#submit`), and pseudo-classes (`:hover`, `:pressed`, `:disabled`)
//! - **Combinators**: Descendant (` `), child (`>`), adjacent sibling (`+`),
//!   general sibling (`~`)
//! - **Properties**: Box model (margin, padding, border), colors, fonts, and effects
//!
//! # Error Recovery
//!
//! The parser is resilient to syntax errors. When an invalid rule or property is
//! encountered, it logs a warning via `tracing::warn!` and skips to the next rule
//! or declaration, continuing to parse the rest of the stylesheet. This allows
//! partial stylesheets to be applied even when some rules are malformed.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_style::parser::parse_css;
//!
//! let css = r#"
//!     Button {
//!         background-color: #3498db;
//!         color: white;
//!         padding: 8px 16px;
//!         border-radius: 4px;
//!     }
//!
//!     Button:hover {
//!         background-color: #2980b9;
//!     }
//!
//!     .danger {
//!         background-color: #e74c3c;
//!     }
//! "#;
//!
//! let rules = parse_css(css)?;
//! // rules now contains 3 StyleRule objects
//! ```
//!
//! # Supported Properties
//!
//! - **Box Model**: `margin`, `padding`, `border-width`, `border-color`, `border-style`, `border-radius`
//! - **Colors**: `color`, `background-color`, `background`
//! - **Typography**: `font-size`, `font-weight`, `font-style`, `font-family`, `text-align`, `line-height`
//! - **Effects**: `opacity`, `box-shadow`
//! - **Size**: `width`, `height`, `min-width`, `min-height`, `max-width`, `max-height`
//! - **Interaction**: `cursor`, `pointer-events`

mod css_parser;
mod error;

pub use css_parser::parse_css;
pub use error::ParseError;
