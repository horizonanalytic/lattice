//! Style properties definition.

use horizon_lattice_render::{
    Color, Paint, BoxShadow, CornerRadii,
    text::{FontFamily, FontWeight, FontStyle, FontStretch, TextDecoration},
};
use crate::types::{StyleValue, LengthValue, EdgeValues, BorderStyle, TextAlign, Cursor};

/// Complete set of style properties for a widget.
///
/// Properties are grouped logically and use `StyleValue<T>` to support
/// inherit/initial/unset semantics. Only properties that are explicitly
/// set will be applied during cascading.
#[derive(Debug, Clone, Default)]
pub struct StyleProperties {
    // === Box Model ===
    /// Margin (outer spacing).
    pub margin: StyleValue<EdgeValues>,
    /// Padding (inner spacing).
    pub padding: StyleValue<EdgeValues>,
    /// Border width.
    pub border_width: StyleValue<EdgeValues>,
    /// Border color.
    pub border_color: StyleValue<Color>,
    /// Border style.
    pub border_style: StyleValue<BorderStyle>,
    /// Border radius (corner rounding).
    pub border_radius: StyleValue<CornerRadii>,

    // === Background ===
    /// Background paint (color or gradient).
    pub background: StyleValue<Paint>,
    /// Background color (convenience, overridden by background if both set).
    pub background_color: StyleValue<Color>,

    // === Size Constraints ===
    /// Minimum width.
    pub min_width: StyleValue<LengthValue>,
    /// Minimum height.
    pub min_height: StyleValue<LengthValue>,
    /// Maximum width.
    pub max_width: StyleValue<LengthValue>,
    /// Maximum height.
    pub max_height: StyleValue<LengthValue>,
    /// Explicit width.
    pub width: StyleValue<LengthValue>,
    /// Explicit height.
    pub height: StyleValue<LengthValue>,

    // === Typography ===
    /// Font families (in priority order).
    pub font_family: StyleValue<Vec<FontFamily>>,
    /// Font size.
    pub font_size: StyleValue<LengthValue>,
    /// Font weight (100-900).
    pub font_weight: StyleValue<FontWeight>,
    /// Font style (normal, italic, oblique).
    pub font_style: StyleValue<FontStyle>,
    /// Font stretch (condensed to expanded).
    pub font_stretch: StyleValue<FontStretch>,
    /// Text color.
    pub color: StyleValue<Color>,
    /// Text alignment.
    pub text_align: StyleValue<TextAlign>,
    /// Line height multiplier.
    pub line_height: StyleValue<f32>,
    /// Letter spacing.
    pub letter_spacing: StyleValue<LengthValue>,
    /// Text decoration (underline, strikethrough, etc.).
    pub text_decoration: StyleValue<Option<TextDecoration>>,

    // === Effects ===
    /// Opacity (0.0-1.0).
    pub opacity: StyleValue<f32>,
    /// Box shadows.
    pub box_shadow: StyleValue<Vec<BoxShadow>>,

    // === Interaction ===
    /// Cursor style.
    pub cursor: StyleValue<Cursor>,
    /// Whether widget receives pointer events.
    pub pointer_events: StyleValue<bool>,
}

/// Properties that can be inherited from parent to child.
///
/// In this implementation, inheritance is explicit opt-in only.
/// These properties will inherit when set to `StyleValue::Inherit` or `StyleValue::Unset`.
pub const INHERITABLE_PROPERTIES: &[&str] = &[
    "font_family",
    "font_size",
    "font_weight",
    "font_style",
    "font_stretch",
    "color",
    "text_align",
    "line_height",
    "letter_spacing",
    "cursor",
];

impl StyleProperties {
    /// Create new default style properties.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a property name is inheritable by default.
    pub fn is_inheritable(name: &str) -> bool {
        INHERITABLE_PROPERTIES.contains(&name)
    }

    /// Merge another set of properties into this one.
    ///
    /// Only explicitly set values from `other` will be copied.
    pub fn merge(&mut self, other: &StyleProperties) {
        macro_rules! merge_if_set {
            ($($prop:ident),+ $(,)?) => {
                $(
                    if other.$prop.is_set() {
                        self.$prop = other.$prop.clone();
                    }
                )+
            };
        }

        merge_if_set!(
            // Box model
            margin, padding, border_width, border_color, border_style, border_radius,
            // Background
            background, background_color,
            // Size
            min_width, min_height, max_width, max_height, width, height,
            // Typography
            font_family, font_size, font_weight, font_style, font_stretch,
            color, text_align, line_height, letter_spacing, text_decoration,
            // Effects
            opacity, box_shadow,
            // Interaction
            cursor, pointer_events,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properties_default() {
        let props = StyleProperties::new();
        assert!(!props.margin.is_set());
        assert!(!props.color.is_set());
    }

    #[test]
    fn properties_merge() {
        let mut base = StyleProperties::new();
        base.color = StyleValue::Set(Color::BLACK);

        let mut overlay = StyleProperties::new();
        overlay.color = StyleValue::Set(Color::RED);
        overlay.opacity = StyleValue::Set(0.5);

        base.merge(&overlay);

        assert_eq!(base.color.as_set(), Some(&Color::RED));
        assert_eq!(base.opacity.as_set(), Some(&0.5));
    }

    #[test]
    fn inheritable_properties() {
        assert!(StyleProperties::is_inheritable("color"));
        assert!(StyleProperties::is_inheritable("font_size"));
        assert!(!StyleProperties::is_inheritable("margin"));
        assert!(!StyleProperties::is_inheritable("padding"));
    }
}
