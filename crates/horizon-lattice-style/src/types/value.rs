//! Style property value types.
//!
//! This module provides CSS-like value types for styling properties.
//!
//! # Example
//!
//! ```
//! use horizon_lattice_style::prelude::*;
//!
//! // Create length values with different units
//! let px = LengthValue::px(16.0);
//! let em = LengthValue::em(1.5);
//! let percent = LengthValue::percent(50.0);
//!
//! // Resolve to pixels given context
//! let font_size = 14.0;
//! let parent_size = 200.0;
//! let root_font_size = 16.0;
//!
//! assert_eq!(px.to_px(font_size, parent_size, root_font_size), 16.0);
//! assert_eq!(em.to_px(font_size, parent_size, root_font_size), 21.0); // 1.5 * 14
//! assert_eq!(percent.to_px(font_size, parent_size, root_font_size), 100.0); // 50% of 200
//! ```

use horizon_lattice_render::CornerRadii;

/// A style property value that can represent various CSS value types.
///
/// This enum wraps actual values with CSS-like special values for inheritance.
///
/// # Example
///
/// ```
/// use horizon_lattice_style::prelude::StyleValue;
///
/// // Explicit value
/// let color: StyleValue<String> = StyleValue::Set("red".to_string());
/// assert!(color.is_set());
///
/// // Inherit from parent
/// let inherited: StyleValue<i32> = StyleValue::Inherit;
/// let resolved = inherited.resolve(Some(&42), &0);
/// assert_eq!(resolved, 42);
///
/// // Use initial/default value
/// let initial: StyleValue<i32> = StyleValue::Initial;
/// let resolved = initial.resolve(Some(&42), &0);
/// assert_eq!(resolved, 0); // Uses initial, not inherited
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue<T> {
    /// An explicit value.
    Set(T),
    /// Inherit from parent (explicit opt-in).
    Inherit,
    /// Use the initial/default value.
    Initial,
    /// Unset - acts as Inherit for inherited properties, Initial otherwise.
    Unset,
}

impl<T> Default for StyleValue<T> {
    fn default() -> Self {
        Self::Initial
    }
}

impl<T: Clone> StyleValue<T> {
    /// Resolve the value given inherited and initial values.
    ///
    /// - `Set(v)` returns the explicit value
    /// - `Inherit` returns the inherited value, or initial if no parent
    /// - `Initial` returns the initial value
    /// - `Unset` acts like `Inherit`
    pub fn resolve(&self, inherited: Option<&T>, initial: &T) -> T {
        match self {
            StyleValue::Set(v) => v.clone(),
            StyleValue::Inherit | StyleValue::Unset => {
                inherited.cloned().unwrap_or_else(|| initial.clone())
            }
            StyleValue::Initial => initial.clone(),
        }
    }

    /// Check if this value is explicitly set.
    pub fn is_set(&self) -> bool {
        matches!(self, StyleValue::Set(_))
    }

    /// Get the inner value if set.
    pub fn as_set(&self) -> Option<&T> {
        match self {
            StyleValue::Set(v) => Some(v),
            _ => None,
        }
    }
}

impl<T> From<T> for StyleValue<T> {
    fn from(value: T) -> Self {
        StyleValue::Set(value)
    }
}

/// CSS-like length values with various units.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LengthValue {
    /// Absolute pixels.
    Px(f32),
    /// Relative to current font size.
    Em(f32),
    /// Relative to root font size.
    Rem(f32),
    /// Percentage of containing block.
    Percent(f32),
    /// Automatic sizing (context-dependent).
    #[default]
    Auto,
    /// Zero length.
    Zero,
}

impl LengthValue {
    /// Create a pixel value.
    pub fn px(value: f32) -> Self {
        Self::Px(value)
    }

    /// Create an em value.
    pub fn em(value: f32) -> Self {
        Self::Em(value)
    }

    /// Create a rem value.
    pub fn rem(value: f32) -> Self {
        Self::Rem(value)
    }

    /// Create a percentage value.
    pub fn percent(value: f32) -> Self {
        Self::Percent(value)
    }

    /// Resolve to pixels given the context.
    ///
    /// # Arguments
    /// * `font_size` - Current element's font size (for em)
    /// * `parent_size` - Parent's size in the relevant dimension (for %)
    /// * `root_font_size` - Root element's font size (for rem)
    pub fn to_px(&self, font_size: f32, parent_size: f32, root_font_size: f32) -> f32 {
        match self {
            LengthValue::Px(v) => *v,
            LengthValue::Em(v) => v * font_size,
            LengthValue::Rem(v) => v * root_font_size,
            LengthValue::Percent(v) => (v / 100.0) * parent_size,
            LengthValue::Auto => 0.0, // Context-dependent
            LengthValue::Zero => 0.0,
        }
    }

    /// Check if this is an auto value.
    pub fn is_auto(&self) -> bool {
        matches!(self, LengthValue::Auto)
    }

    /// Check if this is zero or would resolve to zero.
    pub fn is_zero(&self) -> bool {
        match self {
            LengthValue::Zero => true,
            LengthValue::Px(v) | LengthValue::Em(v) | LengthValue::Rem(v) | LengthValue::Percent(v) => {
                *v == 0.0
            }
            LengthValue::Auto => false,
        }
    }
}

/// Edge values for margin, padding, and border-width.
///
/// # Example
///
/// ```
/// use horizon_lattice_style::prelude::{EdgeValues, LengthValue};
///
/// // Uniform edges (same value on all sides)
/// let padding = EdgeValues::uniform(LengthValue::px(10.0));
///
/// // Symmetric edges (vertical, horizontal)
/// let margin = EdgeValues::symmetric(
///     LengthValue::px(20.0),  // top/bottom
///     LengthValue::px(10.0),  // left/right
/// );
///
/// // Specific edges
/// let border = EdgeValues::new(
///     LengthValue::px(1.0),   // top
///     LengthValue::px(2.0),   // right
///     LengthValue::px(3.0),   // bottom
///     LengthValue::px(4.0),   // left
/// );
///
/// // Resolve to pixels
/// let resolved = padding.to_px(14.0, 100.0, 16.0);
/// assert_eq!(resolved.horizontal(), 20.0); // left + right
/// assert_eq!(resolved.vertical(), 20.0);   // top + bottom
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EdgeValues {
    /// Top edge value.
    pub top: LengthValue,
    /// Right edge value.
    pub right: LengthValue,
    /// Bottom edge value.
    pub bottom: LengthValue,
    /// Left edge value.
    pub left: LengthValue,
}

impl EdgeValues {
    /// Create uniform edge values.
    pub fn uniform(value: LengthValue) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// Create symmetric edge values (vertical, horizontal).
    pub fn symmetric(vertical: LengthValue, horizontal: LengthValue) -> Self {
        Self {
            top: vertical,
            bottom: vertical,
            left: horizontal,
            right: horizontal,
        }
    }

    /// Create from 4 values (top, right, bottom, left).
    pub fn new(top: LengthValue, right: LengthValue, bottom: LengthValue, left: LengthValue) -> Self {
        Self { top, right, bottom, left }
    }

    /// Create zero edge values.
    pub fn zero() -> Self {
        Self::uniform(LengthValue::Zero)
    }

    /// Resolve all edges to pixels.
    pub fn to_px(&self, font_size: f32, parent_size: f32, root_font_size: f32) -> ResolvedEdges {
        ResolvedEdges {
            top: self.top.to_px(font_size, parent_size, root_font_size),
            right: self.right.to_px(font_size, parent_size, root_font_size),
            bottom: self.bottom.to_px(font_size, parent_size, root_font_size),
            left: self.left.to_px(font_size, parent_size, root_font_size),
        }
    }
}

/// Resolved edge values in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ResolvedEdges {
    /// Top edge in pixels.
    pub top: f32,
    /// Right edge in pixels.
    pub right: f32,
    /// Bottom edge in pixels.
    pub bottom: f32,
    /// Left edge in pixels.
    pub left: f32,
}

impl ResolvedEdges {
    /// Get total horizontal space (left + right).
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Get total vertical space (top + bottom).
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

/// Border style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    /// No border.
    #[default]
    None,
    /// Solid line border.
    Solid,
    /// Dashed line border.
    Dashed,
    /// Dotted line border.
    Dotted,
    /// Double line border.
    Double,
}

impl BorderStyle {
    /// Parse from CSS string.
    pub fn from_css(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => Some(Self::None),
            "solid" => Some(Self::Solid),
            "dashed" => Some(Self::Dashed),
            "dotted" => Some(Self::Dotted),
            "double" => Some(Self::Double),
            _ => None,
        }
    }
}

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlign {
    /// Align to the start of the text direction (left for LTR, right for RTL).
    #[default]
    Start,
    /// Align to the end of the text direction (right for LTR, left for RTL).
    End,
    /// Align to the left edge.
    Left,
    /// Align to the right edge.
    Right,
    /// Center the text.
    Center,
    /// Justify text to fill the available width.
    Justify,
}

impl TextAlign {
    /// Parse from CSS string.
    pub fn from_css(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "start" => Some(Self::Start),
            "end" => Some(Self::End),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "center" => Some(Self::Center),
            "justify" => Some(Self::Justify),
            _ => None,
        }
    }
}

/// Cursor style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Cursor {
    /// Default arrow cursor.
    #[default]
    Default,
    /// Pointing hand cursor (for clickable elements).
    Pointer,
    /// Text selection cursor (I-beam).
    Text,
    /// Move/drag cursor.
    Move,
    /// Not-allowed cursor (prohibition sign).
    NotAllowed,
    /// Crosshair cursor.
    Crosshair,
    /// Wait/busy cursor.
    Wait,
    /// Progress cursor (busy with arrow).
    Progress,
    /// Help cursor (arrow with question mark).
    Help,
    /// Grab cursor (open hand).
    Grab,
    /// Grabbing cursor (closed hand).
    Grabbing,
    /// North-south resize cursor.
    ResizeNs,
    /// East-west resize cursor.
    ResizeEw,
    /// Northeast-southwest resize cursor.
    ResizeNesw,
    /// Northwest-southeast resize cursor.
    ResizeNwse,
}

impl Cursor {
    /// Parse from CSS string.
    pub fn from_css(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "default" | "auto" => Some(Self::Default),
            "pointer" => Some(Self::Pointer),
            "text" => Some(Self::Text),
            "move" => Some(Self::Move),
            "not-allowed" => Some(Self::NotAllowed),
            "crosshair" => Some(Self::Crosshair),
            "wait" => Some(Self::Wait),
            "progress" => Some(Self::Progress),
            "help" => Some(Self::Help),
            "grab" => Some(Self::Grab),
            "grabbing" => Some(Self::Grabbing),
            "ns-resize" | "n-resize" | "s-resize" => Some(Self::ResizeNs),
            "ew-resize" | "e-resize" | "w-resize" => Some(Self::ResizeEw),
            "nesw-resize" | "ne-resize" | "sw-resize" => Some(Self::ResizeNesw),
            "nwse-resize" | "nw-resize" | "se-resize" => Some(Self::ResizeNwse),
            _ => None,
        }
    }
}

/// Corner radii builder for convenience.
pub trait CornerRadiiExt {
    /// Create uniform corner radii.
    fn uniform(radius: f32) -> Self;
    /// Check if all corners are zero.
    fn is_zero(&self) -> bool;
}

impl CornerRadiiExt for CornerRadii {
    fn uniform(radius: f32) -> Self {
        Self {
            top_left: radius,
            top_right: radius,
            bottom_right: radius,
            bottom_left: radius,
        }
    }

    fn is_zero(&self) -> bool {
        self.top_left == 0.0
            && self.top_right == 0.0
            && self.bottom_right == 0.0
            && self.bottom_left == 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_value_resolve() {
        let set: StyleValue<i32> = StyleValue::Set(42);
        assert_eq!(set.resolve(Some(&100), &0), 42);

        let inherit: StyleValue<i32> = StyleValue::Inherit;
        assert_eq!(inherit.resolve(Some(&100), &0), 100);
        assert_eq!(inherit.resolve(None, &0), 0);

        let initial: StyleValue<i32> = StyleValue::Initial;
        assert_eq!(initial.resolve(Some(&100), &0), 0);
    }

    #[test]
    fn length_value_to_px() {
        assert_eq!(LengthValue::Px(10.0).to_px(14.0, 100.0, 16.0), 10.0);
        assert_eq!(LengthValue::Em(1.5).to_px(14.0, 100.0, 16.0), 21.0);
        assert_eq!(LengthValue::Rem(1.0).to_px(14.0, 100.0, 16.0), 16.0);
        assert_eq!(LengthValue::Percent(50.0).to_px(14.0, 100.0, 16.0), 50.0);
    }

    #[test]
    fn edge_values() {
        let uniform = EdgeValues::uniform(LengthValue::Px(10.0));
        let resolved = uniform.to_px(14.0, 100.0, 16.0);
        assert_eq!(resolved.horizontal(), 20.0);
        assert_eq!(resolved.vertical(), 20.0);
    }
}
