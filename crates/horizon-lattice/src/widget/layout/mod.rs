//! Layout system for automatic widget positioning and sizing.
//!
//! This module provides the foundational layout architecture including:
//!
//! - [`Layout`] trait: The base trait for all layout managers
//! - [`LayoutItem`]: Items that can be managed by a layout
//! - [`LayoutBase`]: Common implementation for layout functionality
//! - [`ContentMargins`]: Spacing around layout content
//!
//! # Overview
//!
//! The layout system follows Qt's design philosophy while being idiomatic Rust.
//! Layouts manage the positioning and sizing of widgets automatically based on
//! size hints, size policies, and available space.
//!
//! # Layout Algorithm
//!
//! Layouts use a two-pass algorithm:
//!
//! 1. **Bottom-up pass**: Collect size hints from all items to determine
//!    the layout's own size requirements.
//! 2. **Top-down pass**: Distribute available space to items based on their
//!    size policies and stretch factors.
//!
//! # Creating a Layout
//!
//! To create a custom layout:
//!
//! 1. Implement the [`Layout`] trait
//! 2. Use [`LayoutBase`] for common functionality
//! 3. Implement the layout algorithm in `calculate()` and `apply()`
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::layout::*;
//!
//! // Create a horizontal box layout
//! let mut layout = HBoxLayout::new();
//! layout.set_spacing(10.0);
//! layout.set_content_margins(ContentMargins::uniform(8.0));
//!
//! // Add widgets
//! layout.add_widget(button1_id);
//! layout.add_widget(button2_id);
//! layout.add_stretch(1); // Flexible spacer
//! layout.add_widget(button3_id);
//!
//! // Calculate and apply layout
//! let size = layout.calculate(available_size);
//! layout.apply(&mut widget_storage);
//! ```

mod item;
mod traits;
mod base;
mod invalidation;

pub use item::{LayoutItem, SpacerItem, SpacerType};
pub use traits::Layout;
pub use base::LayoutBase;
pub use invalidation::{LayoutInvalidator, InvalidationScope};

use horizon_lattice_render::Size;

/// Content margins around a layout.
///
/// Margins define the spacing between the layout's content and its edges.
/// This is used to add padding around all items in a layout.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ContentMargins {
    /// Left margin.
    pub left: f32,
    /// Top margin.
    pub top: f32,
    /// Right margin.
    pub right: f32,
    /// Bottom margin.
    pub bottom: f32,
}

impl ContentMargins {
    /// Create new content margins.
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// Create uniform margins (same value on all sides).
    pub fn uniform(margin: f32) -> Self {
        Self::new(margin, margin, margin, margin)
    }

    /// Create symmetric margins (same horizontal and vertical).
    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self::new(horizontal, vertical, horizontal, vertical)
    }

    /// Total horizontal margin (left + right).
    #[inline]
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Total vertical margin (top + bottom).
    #[inline]
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    /// Size occupied by margins.
    #[inline]
    pub fn size(&self) -> Size {
        Size::new(self.horizontal(), self.vertical())
    }
}

/// Default spacing between items in a layout.
pub const DEFAULT_SPACING: f32 = 6.0;

/// Default content margins for layouts.
pub const DEFAULT_MARGINS: ContentMargins = ContentMargins {
    left: 9.0,
    top: 9.0,
    right: 9.0,
    bottom: 9.0,
};
