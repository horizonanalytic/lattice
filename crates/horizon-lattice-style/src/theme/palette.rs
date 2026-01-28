//! Color palette definitions.

use horizon_lattice_render::Color;

/// A color palette for theming.
#[derive(Debug, Clone)]
pub struct ColorPalette {
    // Primary colors
    /// Main brand color.
    pub primary: Color,
    /// Lighter variant of the primary color.
    pub primary_light: Color,
    /// Darker variant of the primary color.
    pub primary_dark: Color,
    /// Text/icon color for content on primary color.
    pub on_primary: Color,

    // Secondary colors
    /// Secondary brand/accent color.
    pub secondary: Color,
    /// Lighter variant of the secondary color.
    pub secondary_light: Color,
    /// Darker variant of the secondary color.
    pub secondary_dark: Color,
    /// Text/icon color for content on secondary color.
    pub on_secondary: Color,

    // Background colors
    /// Main background color.
    pub background: Color,
    /// Surface/card background color.
    pub surface: Color,
    /// Variant surface color for differentiation.
    pub surface_variant: Color,

    // Text colors
    /// Primary text color.
    pub text_primary: Color,
    /// Secondary/muted text color.
    pub text_secondary: Color,
    /// Disabled text color.
    pub text_disabled: Color,

    // Semantic colors
    /// Error/danger color.
    pub error: Color,
    /// Warning color.
    pub warning: Color,
    /// Success color.
    pub success: Color,
    /// Informational color.
    pub info: Color,

    // Border colors
    /// Standard border color.
    pub border: Color,
    /// Light border color.
    pub border_light: Color,
    /// Divider/separator color.
    pub divider: Color,
}

impl ColorPalette {
    /// Create a light theme palette.
    pub fn light() -> Self {
        Self {
            // Primary - blue
            primary: Color::from_hex("#007AFF").unwrap(),
            primary_light: Color::from_hex("#4DA3FF").unwrap(),
            primary_dark: Color::from_hex("#0056B3").unwrap(),
            on_primary: Color::WHITE,

            // Secondary - gray
            secondary: Color::from_hex("#6C757D").unwrap(),
            secondary_light: Color::from_hex("#ADB5BD").unwrap(),
            secondary_dark: Color::from_hex("#495057").unwrap(),
            on_secondary: Color::WHITE,

            // Background
            background: Color::from_hex("#FFFFFF").unwrap(),
            surface: Color::from_hex("#F8F9FA").unwrap(),
            surface_variant: Color::from_hex("#E9ECEF").unwrap(),

            // Text
            text_primary: Color::from_hex("#212529").unwrap(),
            text_secondary: Color::from_hex("#6C757D").unwrap(),
            text_disabled: Color::from_hex("#ADB5BD").unwrap(),

            // Semantic
            error: Color::from_hex("#DC3545").unwrap(),
            warning: Color::from_hex("#FFC107").unwrap(),
            success: Color::from_hex("#28A745").unwrap(),
            info: Color::from_hex("#17A2B8").unwrap(),

            // Borders
            border: Color::from_hex("#DEE2E6").unwrap(),
            border_light: Color::from_hex("#E9ECEF").unwrap(),
            divider: Color::from_hex("#CED4DA").unwrap(),
        }
    }

    /// Create a dark theme palette.
    pub fn dark() -> Self {
        Self {
            // Primary - blue (slightly brighter for dark mode)
            primary: Color::from_hex("#0A84FF").unwrap(),
            primary_light: Color::from_hex("#5EB1FF").unwrap(),
            primary_dark: Color::from_hex("#0056B3").unwrap(),
            on_primary: Color::WHITE,

            // Secondary
            secondary: Color::from_hex("#8E8E93").unwrap(),
            secondary_light: Color::from_hex("#AEAEB2").unwrap(),
            secondary_dark: Color::from_hex("#636366").unwrap(),
            on_secondary: Color::WHITE,

            // Background
            background: Color::from_hex("#1C1C1E").unwrap(),
            surface: Color::from_hex("#2C2C2E").unwrap(),
            surface_variant: Color::from_hex("#3A3A3C").unwrap(),

            // Text
            text_primary: Color::from_hex("#FFFFFF").unwrap(),
            text_secondary: Color::from_hex("#8E8E93").unwrap(),
            text_disabled: Color::from_hex("#636366").unwrap(),

            // Semantic
            error: Color::from_hex("#FF453A").unwrap(),
            warning: Color::from_hex("#FFD60A").unwrap(),
            success: Color::from_hex("#32D74B").unwrap(),
            info: Color::from_hex("#64D2FF").unwrap(),

            // Borders
            border: Color::from_hex("#38383A").unwrap(),
            border_light: Color::from_hex("#48484A").unwrap(),
            divider: Color::from_hex("#545456").unwrap(),
        }
    }

    /// Create a high-contrast palette.
    pub fn high_contrast() -> Self {
        Self {
            primary: Color::from_hex("#0000FF").unwrap(),
            primary_light: Color::from_hex("#0000FF").unwrap(),
            primary_dark: Color::from_hex("#0000CC").unwrap(),
            on_primary: Color::WHITE,

            secondary: Color::from_hex("#000000").unwrap(),
            secondary_light: Color::from_hex("#333333").unwrap(),
            secondary_dark: Color::from_hex("#000000").unwrap(),
            on_secondary: Color::WHITE,

            background: Color::WHITE,
            surface: Color::WHITE,
            surface_variant: Color::from_hex("#F0F0F0").unwrap(),

            text_primary: Color::BLACK,
            text_secondary: Color::from_hex("#333333").unwrap(),
            text_disabled: Color::from_hex("#666666").unwrap(),

            error: Color::from_hex("#CC0000").unwrap(),
            warning: Color::from_hex("#CC6600").unwrap(),
            success: Color::from_hex("#006600").unwrap(),
            info: Color::from_hex("#000099").unwrap(),

            border: Color::BLACK,
            border_light: Color::from_hex("#333333").unwrap(),
            divider: Color::BLACK,
        }
    }
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::light()
    }
}
