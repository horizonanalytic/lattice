//! CSS variables (custom properties) support.

use std::collections::HashMap;
use horizon_lattice_render::Color;
use super::ColorPalette;

/// CSS custom properties (variables).
///
/// Supports `var(--name)` syntax in CSS.
#[derive(Debug, Clone, Default)]
pub struct ThemeVariables {
    variables: HashMap<String, String>,
}

impl ThemeVariables {
    /// Create empty variables.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create variables from a color palette.
    pub fn from_palette(palette: &ColorPalette) -> Self {
        let mut vars = Self::new();

        // Primary colors
        vars.set("primary-color", color_to_hex(&palette.primary));
        vars.set("primary-light", color_to_hex(&palette.primary_light));
        vars.set("primary-dark", color_to_hex(&palette.primary_dark));
        vars.set("on-primary", color_to_hex(&palette.on_primary));

        // Secondary colors
        vars.set("secondary-color", color_to_hex(&palette.secondary));
        vars.set("secondary-light", color_to_hex(&palette.secondary_light));
        vars.set("secondary-dark", color_to_hex(&palette.secondary_dark));
        vars.set("on-secondary", color_to_hex(&palette.on_secondary));

        // Background colors
        vars.set("background", color_to_hex(&palette.background));
        vars.set("surface", color_to_hex(&palette.surface));
        vars.set("surface-variant", color_to_hex(&palette.surface_variant));

        // Text colors
        vars.set("text-primary", color_to_hex(&palette.text_primary));
        vars.set("text-secondary", color_to_hex(&palette.text_secondary));
        vars.set("text-disabled", color_to_hex(&palette.text_disabled));

        // Semantic colors
        vars.set("error", color_to_hex(&palette.error));
        vars.set("warning", color_to_hex(&palette.warning));
        vars.set("success", color_to_hex(&palette.success));
        vars.set("info", color_to_hex(&palette.info));

        // Border colors
        vars.set("border", color_to_hex(&palette.border));
        vars.set("border-light", color_to_hex(&palette.border_light));
        vars.set("divider", color_to_hex(&palette.divider));

        // Common spacing
        vars.set("spacing-xs", "4px");
        vars.set("spacing-sm", "8px");
        vars.set("spacing-md", "16px");
        vars.set("spacing-lg", "24px");
        vars.set("spacing-xl", "32px");

        // Border radius
        vars.set("radius-sm", "4px");
        vars.set("radius-md", "8px");
        vars.set("radius-lg", "12px");
        vars.set("radius-full", "9999px");

        // Font sizes
        vars.set("font-size-xs", "12px");
        vars.set("font-size-sm", "14px");
        vars.set("font-size-md", "16px");
        vars.set("font-size-lg", "18px");
        vars.set("font-size-xl", "24px");
        vars.set("font-size-2xl", "32px");

        vars
    }

    /// Set a variable.
    pub fn set(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        // Remove leading "--" if present
        let name = name.strip_prefix("--").unwrap_or(&name).to_string();
        self.variables.insert(name, value.into());
    }

    /// Get a variable value.
    pub fn get(&self, name: &str) -> Option<&str> {
        // Remove leading "--" if present
        let name = name.strip_prefix("--").unwrap_or(name);
        self.variables.get(name).map(|s| s.as_str())
    }

    /// Check if a variable exists.
    pub fn contains(&self, name: &str) -> bool {
        let name = name.strip_prefix("--").unwrap_or(name);
        self.variables.contains_key(name)
    }

    /// Iterate over all variables.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.variables.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Clear all variables.
    pub fn clear(&mut self) {
        self.variables.clear();
    }
}

fn color_to_hex(color: &Color) -> String {
    let r = (color.r * 255.0) as u8;
    let g = (color.g * 255.0) as u8;
    let b = (color.b * 255.0) as u8;
    let a = (color.a * 255.0) as u8;

    if a == 255 {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variables_set_get() {
        let mut vars = ThemeVariables::new();
        vars.set("primary-color", "#007AFF");

        assert_eq!(vars.get("primary-color"), Some("#007AFF"));
        assert_eq!(vars.get("--primary-color"), Some("#007AFF"));
    }

    #[test]
    fn variables_from_palette() {
        let palette = ColorPalette::light();
        let vars = ThemeVariables::from_palette(&palette);

        assert!(vars.contains("primary-color"));
        assert!(vars.contains("background"));
        assert!(vars.contains("spacing-md"));
    }
}
