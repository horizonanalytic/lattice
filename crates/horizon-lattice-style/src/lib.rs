//! CSS-like styling system for Horizon Lattice.
//!
//! This crate provides a comprehensive styling system inspired by CSS, featuring:

#![warn(missing_docs)]
//!
//! - **Selectors**: Type, class, ID, pseudo-class, and combinator selectors
//! - **Cascading**: Style priority and specificity-based resolution
//! - **CSS Parsing**: Load styles from external .css files
//! - **Hot Reload**: Automatically reload stylesheets during development
//! - **Type-safe DSL**: Build styles programmatically with Rust
//!
//! # Example: Programmatic Styling
//!
//! ```
//! use horizon_lattice_style::prelude::*;
//!
//! // Create a selector for buttons with primary class
//! let selector = Selector::type_selector("Button")
//!     .descendant(SelectorPart::class_only("primary"));
//!
//! // Style values
//! let padding = EdgeValues::uniform(LengthValue::px(16.0));
//! let border = EdgeValues::uniform(LengthValue::px(1.0));
//!
//! // Check selector properties
//! assert!(!selector.parts.is_empty());
//! ```
//!
//! # Example: Selectors
//!
//! ```
//! use horizon_lattice_style::prelude::*;
//!
//! // Type selector
//! let button = Selector::type_selector("Button");
//!
//! // Class selector
//! let primary = Selector::class("primary");
//!
//! // ID selector
//! let submit = Selector::id("submit");
//!
//! // Complex selector with pseudo-class
//! let hover_button = Selector::type_selector("Button")
//!     .descendant(
//!         SelectorPart::class_only("primary")
//!             .with_pseudo(PseudoClass::Hover)
//!     );
//!
//! assert_eq!(hover_button.to_string(), "Button .primary:hover");
//! ```
//!
//! # Example: Loading Stylesheets (requires filesystem)
//!
//! ```no_run
//! use horizon_lattice_style::prelude::*;
//!
//! // Load a stylesheet from a file
//! let stylesheet = StyleSheet::from_file("styles/app.css", StylePriority::Application)
//!     .expect("Failed to load stylesheet");
//!
//! // Create a style engine with the light theme
//! let mut engine = StyleEngine::new(Theme::light());
//! engine.add_stylesheet(stylesheet);
//! ```

pub mod types;
pub mod style;
pub mod selector;
pub mod rules;
pub mod resolve;
pub mod parser;
pub mod theme;
pub mod widget;
pub mod icon_theme;

#[cfg(feature = "hot-reload")]
pub mod hot_reload;

mod error;

pub use error::{Error, Result};

/// Prelude module with commonly used types.
pub mod prelude {
    pub use crate::types::{StyleValue, LengthValue, EdgeValues, BorderStyle, TextAlign, Cursor};
    pub use crate::style::{StyleProperties, ComputedStyle, Style};
    pub use crate::selector::{Selector, SelectorPart, PseudoClass, Combinator, Specificity};
    pub use crate::rules::{StyleRule, StyleSheet, StylePriority};
    pub use crate::resolve::{StyleEngine, StyleContext, WidgetStyleState};
    pub use crate::theme::{Theme, ThemeVariables};
    pub use crate::widget::{
        StyledWidget, StylePaintContext,
        paint_styled_box, paint_background, paint_border,
        content_rect, border_box_size, margin_rect,
    };
    pub use crate::icon_theme::{IconContext, IconLookup, IconName, IconResolver, IconThemeLoader};

    #[cfg(feature = "hot-reload")]
    pub use crate::hot_reload::StylesheetWatcher;
}
