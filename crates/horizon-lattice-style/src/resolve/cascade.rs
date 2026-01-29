//! Property cascading logic.

use crate::style::StyleProperties;

/// Cascade source properties onto target.
///
/// Only explicitly set values from `source` will be copied to `target`.
/// This is the core of CSS cascading - later rules override earlier ones.
pub fn cascade_properties(target: &mut StyleProperties, source: &StyleProperties) {
    macro_rules! cascade_if_set {
        ($($prop:ident),+ $(,)?) => {
            $(
                if source.$prop.is_set() {
                    target.$prop = source.$prop.clone();
                }
            )+
        };
    }

    cascade_if_set!(
        // Box model
        margin,
        padding,
        border_width,
        border_color,
        border_style,
        border_radius,
        // Background
        background,
        background_color,
        // Size
        min_width,
        min_height,
        max_width,
        max_height,
        width,
        height,
        // Typography
        font_family,
        font_size,
        font_weight,
        font_style,
        font_stretch,
        color,
        text_align,
        line_height,
        letter_spacing,
        text_decoration,
        // Effects
        opacity,
        box_shadow,
        // Interaction
        cursor,
        pointer_events,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::StyleValue;
    use horizon_lattice_render::Color;

    #[test]
    fn cascade_overwrites_set_values() {
        let mut target = StyleProperties::default();
        target.color = StyleValue::Set(Color::BLACK);
        target.opacity = StyleValue::Set(1.0);

        let mut source = StyleProperties::default();
        source.color = StyleValue::Set(Color::RED);
        // opacity is not set in source

        cascade_properties(&mut target, &source);

        // color should be overwritten
        assert_eq!(target.color.as_set(), Some(&Color::RED));
        // opacity should remain unchanged
        assert_eq!(target.opacity.as_set(), Some(&1.0));
    }

    #[test]
    fn cascade_preserves_unset_target_values() {
        let mut target = StyleProperties::default();
        // target.color is Initial (not set)

        let mut source = StyleProperties::default();
        source.opacity = StyleValue::Set(0.5);

        cascade_properties(&mut target, &source);

        // color should still be Initial
        assert!(!target.color.is_set());
        // opacity should be set
        assert_eq!(target.opacity.as_set(), Some(&0.5));
    }
}
