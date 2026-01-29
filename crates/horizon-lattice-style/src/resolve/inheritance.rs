//! Property inheritance and resolution to computed values.

use crate::style::{ComputedStyle, StyleProperties};
use crate::types::{BorderStyle, Cursor, LengthValue, StyleValue, TextAlign};
use horizon_lattice_render::{
    Color, CornerRadii, Paint,
    text::{FontFamily, FontStretch, FontStyle, FontWeight},
};

/// Resolve StyleProperties to ComputedStyle, handling inheritance.
///
/// This takes cascaded properties and resolves all values to concrete types:
/// - Relative lengths (em, rem, %) are converted to pixels
/// - Inherit/Initial/Unset are resolved based on parent values
/// - Default values are applied where properties are not set
pub fn resolve_properties(
    props: &StyleProperties,
    parent: Option<&ComputedStyle>,
    root_font_size: f32,
) -> ComputedStyle {
    let mut computed = ComputedStyle::default();

    // Get parent font size for em units (default to root)
    let parent_font_size = parent.map(|p| p.font_size).unwrap_or(root_font_size);

    // First resolve font-size (needed for em units in other properties)
    computed.font_size = resolve_font_size(&props.font_size, parent_font_size, root_font_size);

    // Now we can resolve other properties using the computed font size
    let font_size = computed.font_size;

    // === Typography (inheritable with explicit opt-in) ===
    computed.font_family = resolve_inheritable(
        &props.font_family,
        parent.map(|p| &p.font_family),
        vec![FontFamily::SansSerif],
    );
    computed.font_weight = resolve_inheritable(
        &props.font_weight,
        parent.map(|p| &p.font_weight),
        FontWeight::NORMAL,
    );
    computed.font_style = resolve_inheritable(
        &props.font_style,
        parent.map(|p| &p.font_style),
        FontStyle::Normal,
    );
    computed.font_stretch = resolve_inheritable(
        &props.font_stretch,
        parent.map(|p| &p.font_stretch),
        FontStretch::Normal,
    );
    computed.color = resolve_inheritable(&props.color, parent.map(|p| &p.color), Color::BLACK);
    computed.text_align = resolve_inheritable(
        &props.text_align,
        parent.map(|p| &p.text_align),
        TextAlign::Start,
    );
    computed.line_height =
        resolve_inheritable(&props.line_height, parent.map(|p| &p.line_height), 1.2);
    computed.letter_spacing =
        resolve_length(&props.letter_spacing, font_size, 0.0, root_font_size, 0.0);
    computed.text_decoration = resolve_non_inheritable(&props.text_decoration, None);
    computed.cursor =
        resolve_inheritable(&props.cursor, parent.map(|p| &p.cursor), Cursor::Default);

    // === Box model (not inheritable) ===
    if let StyleValue::Set(edges) = &props.margin {
        computed.margin_top = edges.top.to_px(font_size, 0.0, root_font_size);
        computed.margin_right = edges.right.to_px(font_size, 0.0, root_font_size);
        computed.margin_bottom = edges.bottom.to_px(font_size, 0.0, root_font_size);
        computed.margin_left = edges.left.to_px(font_size, 0.0, root_font_size);
    }

    if let StyleValue::Set(edges) = &props.padding {
        computed.padding_top = edges.top.to_px(font_size, 0.0, root_font_size);
        computed.padding_right = edges.right.to_px(font_size, 0.0, root_font_size);
        computed.padding_bottom = edges.bottom.to_px(font_size, 0.0, root_font_size);
        computed.padding_left = edges.left.to_px(font_size, 0.0, root_font_size);
    }

    // Border
    if let StyleValue::Set(edges) = &props.border_width {
        computed.border_top_width = edges.top.to_px(font_size, 0.0, root_font_size);
        computed.border_right_width = edges.right.to_px(font_size, 0.0, root_font_size);
        computed.border_bottom_width = edges.bottom.to_px(font_size, 0.0, root_font_size);
        computed.border_left_width = edges.left.to_px(font_size, 0.0, root_font_size);
    }
    computed.border_color = resolve_non_inheritable(&props.border_color, Color::TRANSPARENT);
    computed.border_style = resolve_non_inheritable(&props.border_style, BorderStyle::None);
    computed.border_radius =
        resolve_non_inheritable(&props.border_radius, CornerRadii::uniform(0.0));

    // === Background ===
    computed.background = resolve_background(&props.background, &props.background_color);

    // === Size constraints ===
    computed.min_width = resolve_optional_length(&props.min_width, font_size, 0.0, root_font_size);
    computed.min_height =
        resolve_optional_length(&props.min_height, font_size, 0.0, root_font_size);
    computed.max_width = resolve_optional_length(&props.max_width, font_size, 0.0, root_font_size);
    computed.max_height =
        resolve_optional_length(&props.max_height, font_size, 0.0, root_font_size);
    computed.width = resolve_optional_length(&props.width, font_size, 0.0, root_font_size);
    computed.height = resolve_optional_length(&props.height, font_size, 0.0, root_font_size);

    // === Effects ===
    computed.opacity = resolve_non_inheritable(&props.opacity, 1.0);
    computed.box_shadow = resolve_non_inheritable(&props.box_shadow, vec![]);

    // === Interaction ===
    computed.pointer_events = resolve_non_inheritable(&props.pointer_events, true);

    computed
}

/// Resolve an inheritable property.
fn resolve_inheritable<T: Clone>(value: &StyleValue<T>, parent_value: Option<&T>, initial: T) -> T {
    match value {
        StyleValue::Set(v) => v.clone(),
        StyleValue::Inherit | StyleValue::Unset => parent_value.cloned().unwrap_or(initial),
        StyleValue::Initial => initial,
    }
}

/// Resolve a non-inheritable property.
fn resolve_non_inheritable<T: Clone>(value: &StyleValue<T>, initial: T) -> T {
    match value {
        StyleValue::Set(v) => v.clone(),
        StyleValue::Initial | StyleValue::Inherit | StyleValue::Unset => initial,
    }
}

/// Resolve font size specially (inherits by default).
fn resolve_font_size(value: &StyleValue<LengthValue>, parent_size: f32, root_size: f32) -> f32 {
    match value {
        StyleValue::Set(length) => length.to_px(parent_size, parent_size, root_size),
        StyleValue::Inherit | StyleValue::Unset => parent_size,
        StyleValue::Initial => 14.0, // Default font size
    }
}

/// Resolve a length value.
fn resolve_length(
    value: &StyleValue<LengthValue>,
    font_size: f32,
    parent_size: f32,
    root_font_size: f32,
    initial: f32,
) -> f32 {
    match value {
        StyleValue::Set(length) => length.to_px(font_size, parent_size, root_font_size),
        _ => initial,
    }
}

/// Resolve an optional length value.
fn resolve_optional_length(
    value: &StyleValue<LengthValue>,
    font_size: f32,
    parent_size: f32,
    root_font_size: f32,
) -> Option<f32> {
    match value {
        StyleValue::Set(length) if !length.is_auto() => {
            Some(length.to_px(font_size, parent_size, root_font_size))
        }
        _ => None,
    }
}

/// Resolve background from paint and color properties.
fn resolve_background(
    background: &StyleValue<Paint>,
    background_color: &StyleValue<Color>,
) -> Paint {
    // background property takes precedence
    if let StyleValue::Set(paint) = background {
        return paint.clone();
    }

    // Fall back to background-color
    if let StyleValue::Set(color) = background_color {
        return Paint::Solid(*color);
    }

    // Default is transparent
    Paint::Solid(Color::TRANSPARENT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Style;

    #[test]
    fn resolve_basic_properties() {
        let props = Style::new()
            .color(Color::RED)
            .font_size(LengthValue::px(16.0))
            .build();

        let computed = resolve_properties(&props, None, 16.0);

        assert_eq!(computed.color, Color::RED);
        assert_eq!(computed.font_size, 16.0);
    }

    #[test]
    fn resolve_em_units() {
        let props = Style::new()
            .font_size(LengthValue::px(20.0))
            .padding_all(LengthValue::em(1.0))
            .build();

        let computed = resolve_properties(&props, None, 16.0);

        assert_eq!(computed.font_size, 20.0);
        assert_eq!(computed.padding_top, 20.0); // 1em = 20px (computed font size)
    }

    #[test]
    fn resolve_rem_units() {
        let props = Style::new().font_size(LengthValue::rem(1.5)).build();

        let computed = resolve_properties(&props, None, 16.0);

        assert_eq!(computed.font_size, 24.0); // 1.5rem = 24px
    }

    #[test]
    fn resolve_inheritance() {
        let mut parent_style = ComputedStyle::default();
        parent_style.color = Color::BLUE;
        parent_style.font_size = 20.0;

        let props = Style::new().inherit_color().inherit_font_size().build();

        let computed = resolve_properties(&props, Some(&parent_style), 16.0);

        assert_eq!(computed.color, Color::BLUE); // Inherited from parent
        assert_eq!(computed.font_size, 20.0); // Explicit inherit for font-size
    }

    #[test]
    fn resolve_background() {
        // Test background-color
        let props = Style::new().background_color(Color::WHITE).build();
        let computed = resolve_properties(&props, None, 16.0);
        assert!(matches!(computed.background, Paint::Solid(c) if c == Color::WHITE));

        // Test background paint overrides background-color
        let props = Style::new()
            .background(Paint::Solid(Color::RED))
            .background_color(Color::WHITE)
            .build();
        let computed = resolve_properties(&props, None, 16.0);
        assert!(matches!(computed.background, Paint::Solid(c) if c == Color::RED));
    }
}
