//! Type-safe style builder DSL.

use super::StyleProperties;
use crate::types::{BorderStyle, Cursor, EdgeValues, LengthValue, StyleValue, TextAlign};
use horizon_lattice_render::{
    BoxShadow, Color, CornerRadii, Paint,
    text::{FontFamily, FontStretch, FontStyle, FontWeight, TextDecoration},
};

/// Builder for creating style properties with a fluent API.
///
/// # Example
///
/// ```ignore
/// let style = Style::new()
///     .padding(EdgeValues::uniform(LengthValue::px(10.0)))
///     .background_color(Color::from_hex("#007AFF").unwrap())
///     .color(Color::WHITE)
///     .border_radius(CornerRadii::uniform(4.0))
///     .build();
/// ```
#[derive(Debug, Clone, Default)]
pub struct Style {
    props: StyleProperties,
}

impl Style {
    /// Create a new style builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the final StyleProperties.
    pub fn build(self) -> StyleProperties {
        self.props
    }

    // === Box Model ===

    /// Set margin on all sides.
    pub fn margin(mut self, value: EdgeValues) -> Self {
        self.props.margin = StyleValue::Set(value);
        self
    }

    /// Set uniform margin on all sides.
    pub fn margin_all(mut self, value: LengthValue) -> Self {
        self.props.margin = StyleValue::Set(EdgeValues::uniform(value));
        self
    }

    /// Set margin top.
    pub fn margin_top(mut self, value: LengthValue) -> Self {
        let mut edges = self.props.margin.as_set().cloned().unwrap_or_default();
        edges.top = value;
        self.props.margin = StyleValue::Set(edges);
        self
    }

    /// Set margin right.
    pub fn margin_right(mut self, value: LengthValue) -> Self {
        let mut edges = self.props.margin.as_set().cloned().unwrap_or_default();
        edges.right = value;
        self.props.margin = StyleValue::Set(edges);
        self
    }

    /// Set margin bottom.
    pub fn margin_bottom(mut self, value: LengthValue) -> Self {
        let mut edges = self.props.margin.as_set().cloned().unwrap_or_default();
        edges.bottom = value;
        self.props.margin = StyleValue::Set(edges);
        self
    }

    /// Set margin left.
    pub fn margin_left(mut self, value: LengthValue) -> Self {
        let mut edges = self.props.margin.as_set().cloned().unwrap_or_default();
        edges.left = value;
        self.props.margin = StyleValue::Set(edges);
        self
    }

    /// Set padding on all sides.
    pub fn padding(mut self, value: EdgeValues) -> Self {
        self.props.padding = StyleValue::Set(value);
        self
    }

    /// Set uniform padding on all sides.
    pub fn padding_all(mut self, value: LengthValue) -> Self {
        self.props.padding = StyleValue::Set(EdgeValues::uniform(value));
        self
    }

    /// Set padding top.
    pub fn padding_top(mut self, value: LengthValue) -> Self {
        let mut edges = self.props.padding.as_set().cloned().unwrap_or_default();
        edges.top = value;
        self.props.padding = StyleValue::Set(edges);
        self
    }

    /// Set padding right.
    pub fn padding_right(mut self, value: LengthValue) -> Self {
        let mut edges = self.props.padding.as_set().cloned().unwrap_or_default();
        edges.right = value;
        self.props.padding = StyleValue::Set(edges);
        self
    }

    /// Set padding bottom.
    pub fn padding_bottom(mut self, value: LengthValue) -> Self {
        let mut edges = self.props.padding.as_set().cloned().unwrap_or_default();
        edges.bottom = value;
        self.props.padding = StyleValue::Set(edges);
        self
    }

    /// Set padding left.
    pub fn padding_left(mut self, value: LengthValue) -> Self {
        let mut edges = self.props.padding.as_set().cloned().unwrap_or_default();
        edges.left = value;
        self.props.padding = StyleValue::Set(edges);
        self
    }

    /// Set border width on all sides.
    pub fn border_width(mut self, value: EdgeValues) -> Self {
        self.props.border_width = StyleValue::Set(value);
        self
    }

    /// Set uniform border width on all sides.
    pub fn border_width_all(mut self, value: LengthValue) -> Self {
        self.props.border_width = StyleValue::Set(EdgeValues::uniform(value));
        self
    }

    /// Set border color.
    pub fn border_color(mut self, color: Color) -> Self {
        self.props.border_color = StyleValue::Set(color);
        self
    }

    /// Set border style.
    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.props.border_style = StyleValue::Set(style);
        self
    }

    /// Set border radius.
    pub fn border_radius(mut self, radii: CornerRadii) -> Self {
        self.props.border_radius = StyleValue::Set(radii);
        self
    }

    /// Set uniform border radius on all corners.
    pub fn border_radius_all(mut self, radius: f32) -> Self {
        self.props.border_radius = StyleValue::Set(CornerRadii::uniform(radius));
        self
    }

    // === Background ===

    /// Set background paint.
    pub fn background(mut self, paint: Paint) -> Self {
        self.props.background = StyleValue::Set(paint);
        self
    }

    /// Set background color.
    pub fn background_color(mut self, color: Color) -> Self {
        self.props.background_color = StyleValue::Set(color);
        self
    }

    // === Size Constraints ===

    /// Set minimum width.
    pub fn min_width(mut self, value: LengthValue) -> Self {
        self.props.min_width = StyleValue::Set(value);
        self
    }

    /// Set minimum height.
    pub fn min_height(mut self, value: LengthValue) -> Self {
        self.props.min_height = StyleValue::Set(value);
        self
    }

    /// Set maximum width.
    pub fn max_width(mut self, value: LengthValue) -> Self {
        self.props.max_width = StyleValue::Set(value);
        self
    }

    /// Set maximum height.
    pub fn max_height(mut self, value: LengthValue) -> Self {
        self.props.max_height = StyleValue::Set(value);
        self
    }

    /// Set explicit width.
    pub fn width(mut self, value: LengthValue) -> Self {
        self.props.width = StyleValue::Set(value);
        self
    }

    /// Set explicit height.
    pub fn height(mut self, value: LengthValue) -> Self {
        self.props.height = StyleValue::Set(value);
        self
    }

    // === Typography ===

    /// Set font family.
    pub fn font_family(mut self, families: Vec<FontFamily>) -> Self {
        self.props.font_family = StyleValue::Set(families);
        self
    }

    /// Set a single font family.
    pub fn font(mut self, family: FontFamily) -> Self {
        self.props.font_family = StyleValue::Set(vec![family]);
        self
    }

    /// Set font size.
    pub fn font_size(mut self, size: LengthValue) -> Self {
        self.props.font_size = StyleValue::Set(size);
        self
    }

    /// Set font weight.
    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.props.font_weight = StyleValue::Set(weight);
        self
    }

    /// Set font style.
    pub fn font_style(mut self, style: FontStyle) -> Self {
        self.props.font_style = StyleValue::Set(style);
        self
    }

    /// Set font stretch.
    pub fn font_stretch(mut self, stretch: FontStretch) -> Self {
        self.props.font_stretch = StyleValue::Set(stretch);
        self
    }

    /// Set text color.
    pub fn color(mut self, color: Color) -> Self {
        self.props.color = StyleValue::Set(color);
        self
    }

    /// Set text alignment.
    pub fn text_align(mut self, align: TextAlign) -> Self {
        self.props.text_align = StyleValue::Set(align);
        self
    }

    /// Set line height multiplier.
    pub fn line_height(mut self, height: f32) -> Self {
        self.props.line_height = StyleValue::Set(height);
        self
    }

    /// Set letter spacing.
    pub fn letter_spacing(mut self, spacing: LengthValue) -> Self {
        self.props.letter_spacing = StyleValue::Set(spacing);
        self
    }

    /// Set text decoration.
    pub fn text_decoration(mut self, decoration: TextDecoration) -> Self {
        self.props.text_decoration = StyleValue::Set(Some(decoration));
        self
    }

    /// Remove text decoration.
    pub fn no_text_decoration(mut self) -> Self {
        self.props.text_decoration = StyleValue::Set(None);
        self
    }

    // === Effects ===

    /// Set opacity (0.0-1.0).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.props.opacity = StyleValue::Set(opacity.clamp(0.0, 1.0));
        self
    }

    /// Set box shadows.
    pub fn box_shadow(mut self, shadows: Vec<BoxShadow>) -> Self {
        self.props.box_shadow = StyleValue::Set(shadows);
        self
    }

    /// Add a single box shadow.
    pub fn shadow(mut self, shadow: BoxShadow) -> Self {
        self.props.box_shadow = StyleValue::Set(vec![shadow]);
        self
    }

    // === Interaction ===

    /// Set cursor style.
    pub fn cursor(mut self, cursor: Cursor) -> Self {
        self.props.cursor = StyleValue::Set(cursor);
        self
    }

    /// Set whether widget receives pointer events.
    pub fn pointer_events(mut self, enabled: bool) -> Self {
        self.props.pointer_events = StyleValue::Set(enabled);
        self
    }

    // === Special Values ===

    /// Set a property to inherit from parent.
    pub fn inherit_color(mut self) -> Self {
        self.props.color = StyleValue::Inherit;
        self
    }

    /// Set font size to inherit from parent.
    pub fn inherit_font_size(mut self) -> Self {
        self.props.font_size = StyleValue::Inherit;
        self
    }

    /// Set font family to inherit from parent.
    pub fn inherit_font_family(mut self) -> Self {
        self.props.font_family = StyleValue::Inherit;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_chain() {
        let props = Style::new()
            .padding_all(LengthValue::px(10.0))
            .background_color(Color::WHITE)
            .color(Color::BLACK)
            .font_size(LengthValue::px(14.0))
            .border_radius_all(4.0)
            .build();

        assert!(props.padding.is_set());
        assert!(props.background_color.is_set());
        assert!(props.color.is_set());
        assert!(props.font_size.is_set());
        assert!(props.border_radius.is_set());
    }

    #[test]
    fn builder_individual_edges() {
        let props = Style::new()
            .margin_top(LengthValue::px(10.0))
            .margin_bottom(LengthValue::px(20.0))
            .build();

        if let StyleValue::Set(edges) = &props.margin {
            assert!(matches!(edges.top, LengthValue::Px(v) if v == 10.0));
            assert!(matches!(edges.bottom, LengthValue::Px(v) if v == 20.0));
        } else {
            panic!("margin should be set");
        }
    }
}
