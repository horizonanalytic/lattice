//! Built-in themes.

use std::collections::HashMap;
use horizon_lattice_render::Color;
use crate::style::{StyleProperties, Style};
use crate::types::{LengthValue, EdgeValues, BorderStyle, Cursor};
use horizon_lattice_render::CornerRadii;
use super::{ColorPalette, ThemeVariables};

/// Theme mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Light,
    Dark,
    HighContrast,
}

/// A complete theme with colors and widget defaults.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme mode.
    pub mode: ThemeMode,
    /// Color palette.
    pub palette: ColorPalette,
    /// CSS variables.
    pub variables: ThemeVariables,
    /// Default styles per widget type.
    pub widget_defaults: HashMap<String, StyleProperties>,
}

impl Theme {
    /// Create a light theme.
    pub fn light() -> Self {
        let palette = ColorPalette::light();
        let variables = ThemeVariables::from_palette(&palette);
        let widget_defaults = create_widget_defaults(&palette);

        Self {
            mode: ThemeMode::Light,
            palette,
            variables,
            widget_defaults,
        }
    }

    /// Create a dark theme.
    pub fn dark() -> Self {
        let palette = ColorPalette::dark();
        let variables = ThemeVariables::from_palette(&palette);
        let widget_defaults = create_widget_defaults(&palette);

        Self {
            mode: ThemeMode::Dark,
            palette,
            variables,
            widget_defaults,
        }
    }

    /// Create a high-contrast theme.
    pub fn high_contrast() -> Self {
        let palette = ColorPalette::high_contrast();
        let variables = ThemeVariables::from_palette(&palette);
        let widget_defaults = create_widget_defaults(&palette);

        Self {
            mode: ThemeMode::HighContrast,
            palette,
            variables,
            widget_defaults,
        }
    }

    /// Create a custom theme from a palette.
    pub fn custom(mode: ThemeMode, palette: ColorPalette) -> Self {
        let variables = ThemeVariables::from_palette(&palette);
        let widget_defaults = create_widget_defaults(&palette);

        Self {
            mode,
            palette,
            variables,
            widget_defaults,
        }
    }

    /// Get the primary color.
    pub fn primary(&self) -> Color {
        self.palette.primary
    }

    /// Get the background color.
    pub fn background(&self) -> Color {
        self.palette.background
    }

    /// Get the text color.
    pub fn text_color(&self) -> Color {
        self.palette.text_primary
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light()
    }
}

/// Create default styles for common widget types.
fn create_widget_defaults(palette: &ColorPalette) -> HashMap<String, StyleProperties> {
    let mut defaults = HashMap::new();

    // Button defaults
    defaults.insert("Button".to_string(), Style::new()
        .padding(EdgeValues::symmetric(LengthValue::px(8.0), LengthValue::px(16.0)))
        .background_color(palette.primary)
        .color(palette.on_primary)
        .border_radius(CornerRadii::uniform(4.0))
        .font_size(LengthValue::px(14.0))
        .cursor(Cursor::Pointer)
        .build());

    // Label defaults
    defaults.insert("Label".to_string(), Style::new()
        .color(palette.text_primary)
        .font_size(LengthValue::px(14.0))
        .line_height(1.4)
        .build());

    // TextInput defaults
    defaults.insert("TextInput".to_string(), Style::new()
        .padding(EdgeValues::symmetric(LengthValue::px(8.0), LengthValue::px(12.0)))
        .background_color(palette.surface)
        .color(palette.text_primary)
        .border_width_all(LengthValue::px(1.0))
        .border_color(palette.border)
        .border_style(BorderStyle::Solid)
        .border_radius(CornerRadii::uniform(4.0))
        .font_size(LengthValue::px(14.0))
        .build());

    // Container defaults
    defaults.insert("Container".to_string(), Style::new()
        .background_color(palette.background)
        .build());

    // Panel defaults
    defaults.insert("Panel".to_string(), Style::new()
        .background_color(palette.surface)
        .padding_all(LengthValue::px(16.0))
        .border_radius(CornerRadii::uniform(8.0))
        .build());

    // Card defaults
    defaults.insert("Card".to_string(), Style::new()
        .background_color(palette.surface)
        .padding_all(LengthValue::px(16.0))
        .border_radius(CornerRadii::uniform(8.0))
        .build());

    // Checkbox defaults
    defaults.insert("Checkbox".to_string(), Style::new()
        .cursor(Cursor::Pointer)
        .build());

    // Slider defaults
    defaults.insert("Slider".to_string(), Style::new()
        .cursor(Cursor::Pointer)
        .build());

    // Divider defaults
    defaults.insert("Divider".to_string(), Style::new()
        .background_color(palette.divider)
        .build());

    defaults
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_creation() {
        let light = Theme::light();
        assert_eq!(light.mode, ThemeMode::Light);

        let dark = Theme::dark();
        assert_eq!(dark.mode, ThemeMode::Dark);
    }

    #[test]
    fn theme_has_widget_defaults() {
        let theme = Theme::light();

        assert!(theme.widget_defaults.contains_key("Button"));
        assert!(theme.widget_defaults.contains_key("Label"));
        assert!(theme.widget_defaults.contains_key("TextInput"));
    }

    #[test]
    fn theme_variables_populated() {
        let theme = Theme::light();

        assert!(theme.variables.contains("primary-color"));
        assert!(theme.variables.contains("background"));
    }
}
