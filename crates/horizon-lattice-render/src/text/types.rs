//! Font-related types and enumerations.

use std::fmt;

/// Font weight, typically ranging from 100 (thin) to 900 (black).
///
/// Common weight constants are provided for convenience.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    /// Thin weight (100).
    pub const THIN: Self = Self(100);
    /// Extra-light weight (200).
    pub const EXTRA_LIGHT: Self = Self(200);
    /// Light weight (300).
    pub const LIGHT: Self = Self(300);
    /// Normal/regular weight (400).
    pub const NORMAL: Self = Self(400);
    /// Medium weight (500).
    pub const MEDIUM: Self = Self(500);
    /// Semi-bold weight (600).
    pub const SEMI_BOLD: Self = Self(600);
    /// Bold weight (700).
    pub const BOLD: Self = Self(700);
    /// Extra-bold weight (800).
    pub const EXTRA_BOLD: Self = Self(800);
    /// Black/heavy weight (900).
    pub const BLACK: Self = Self(900);

    /// Create a font weight from a numeric value (100-900).
    pub fn new(weight: u16) -> Self {
        Self(weight.clamp(100, 900))
    }

    /// Get the numeric weight value.
    pub const fn value(self) -> u16 {
        self.0
    }

    /// Convert to fontdb Weight.
    pub fn to_fontdb(self) -> fontdb::Weight {
        fontdb::Weight(self.0)
    }

    /// Create from fontdb Weight.
    pub fn from_fontdb(weight: fontdb::Weight) -> Self {
        Self(weight.0)
    }

    /// Convert to cosmic-text Weight.
    pub fn to_cosmic(self) -> cosmic_text::Weight {
        cosmic_text::Weight(self.0)
    }

    /// Create from cosmic-text Weight.
    pub fn from_cosmic(weight: cosmic_text::Weight) -> Self {
        Self(weight.0)
    }
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl From<u16> for FontWeight {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

/// Font style (normal, italic, or oblique).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontStyle {
    /// Normal upright style.
    #[default]
    Normal,
    /// Italic style (designed italic glyphs).
    Italic,
    /// Oblique style (slanted normal glyphs).
    Oblique,
}

impl FontStyle {
    /// Convert to fontdb Style.
    pub fn to_fontdb(self) -> fontdb::Style {
        match self {
            FontStyle::Normal => fontdb::Style::Normal,
            FontStyle::Italic => fontdb::Style::Italic,
            FontStyle::Oblique => fontdb::Style::Oblique,
        }
    }

    /// Create from fontdb Style.
    pub fn from_fontdb(style: fontdb::Style) -> Self {
        match style {
            fontdb::Style::Normal => FontStyle::Normal,
            fontdb::Style::Italic => FontStyle::Italic,
            fontdb::Style::Oblique => FontStyle::Oblique,
        }
    }

    /// Convert to cosmic-text Style.
    pub fn to_cosmic(self) -> cosmic_text::Style {
        match self {
            FontStyle::Normal => cosmic_text::Style::Normal,
            FontStyle::Italic => cosmic_text::Style::Italic,
            FontStyle::Oblique => cosmic_text::Style::Oblique,
        }
    }

    /// Create from cosmic-text Style.
    pub fn from_cosmic(style: cosmic_text::Style) -> Self {
        match style {
            cosmic_text::Style::Normal => FontStyle::Normal,
            cosmic_text::Style::Italic => FontStyle::Italic,
            cosmic_text::Style::Oblique => FontStyle::Oblique,
        }
    }
}

/// Font stretch/width (condensed to expanded).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontStretch {
    /// Ultra-condensed width.
    UltraCondensed,
    /// Extra-condensed width.
    ExtraCondensed,
    /// Condensed width.
    Condensed,
    /// Semi-condensed width.
    SemiCondensed,
    /// Normal width.
    #[default]
    Normal,
    /// Semi-expanded width.
    SemiExpanded,
    /// Expanded width.
    Expanded,
    /// Extra-expanded width.
    ExtraExpanded,
    /// Ultra-expanded width.
    UltraExpanded,
}

impl FontStretch {
    /// Convert to fontdb Stretch.
    pub fn to_fontdb(self) -> fontdb::Stretch {
        match self {
            FontStretch::UltraCondensed => fontdb::Stretch::UltraCondensed,
            FontStretch::ExtraCondensed => fontdb::Stretch::ExtraCondensed,
            FontStretch::Condensed => fontdb::Stretch::Condensed,
            FontStretch::SemiCondensed => fontdb::Stretch::SemiCondensed,
            FontStretch::Normal => fontdb::Stretch::Normal,
            FontStretch::SemiExpanded => fontdb::Stretch::SemiExpanded,
            FontStretch::Expanded => fontdb::Stretch::Expanded,
            FontStretch::ExtraExpanded => fontdb::Stretch::ExtraExpanded,
            FontStretch::UltraExpanded => fontdb::Stretch::UltraExpanded,
        }
    }

    /// Create from fontdb Stretch.
    pub fn from_fontdb(stretch: fontdb::Stretch) -> Self {
        match stretch {
            fontdb::Stretch::UltraCondensed => FontStretch::UltraCondensed,
            fontdb::Stretch::ExtraCondensed => FontStretch::ExtraCondensed,
            fontdb::Stretch::Condensed => FontStretch::Condensed,
            fontdb::Stretch::SemiCondensed => FontStretch::SemiCondensed,
            fontdb::Stretch::Normal => FontStretch::Normal,
            fontdb::Stretch::SemiExpanded => FontStretch::SemiExpanded,
            fontdb::Stretch::Expanded => FontStretch::Expanded,
            fontdb::Stretch::ExtraExpanded => FontStretch::ExtraExpanded,
            fontdb::Stretch::UltraExpanded => FontStretch::UltraExpanded,
        }
    }

    /// Convert to cosmic-text Stretch.
    pub fn to_cosmic(self) -> cosmic_text::Stretch {
        match self {
            FontStretch::UltraCondensed => cosmic_text::Stretch::UltraCondensed,
            FontStretch::ExtraCondensed => cosmic_text::Stretch::ExtraCondensed,
            FontStretch::Condensed => cosmic_text::Stretch::Condensed,
            FontStretch::SemiCondensed => cosmic_text::Stretch::SemiCondensed,
            FontStretch::Normal => cosmic_text::Stretch::Normal,
            FontStretch::SemiExpanded => cosmic_text::Stretch::SemiExpanded,
            FontStretch::Expanded => cosmic_text::Stretch::Expanded,
            FontStretch::ExtraExpanded => cosmic_text::Stretch::ExtraExpanded,
            FontStretch::UltraExpanded => cosmic_text::Stretch::UltraExpanded,
        }
    }

    /// Create from cosmic-text Stretch.
    pub fn from_cosmic(stretch: cosmic_text::Stretch) -> Self {
        match stretch {
            cosmic_text::Stretch::UltraCondensed => FontStretch::UltraCondensed,
            cosmic_text::Stretch::ExtraCondensed => FontStretch::ExtraCondensed,
            cosmic_text::Stretch::Condensed => FontStretch::Condensed,
            cosmic_text::Stretch::SemiCondensed => FontStretch::SemiCondensed,
            cosmic_text::Stretch::Normal => FontStretch::Normal,
            cosmic_text::Stretch::SemiExpanded => FontStretch::SemiExpanded,
            cosmic_text::Stretch::Expanded => FontStretch::Expanded,
            cosmic_text::Stretch::ExtraExpanded => FontStretch::ExtraExpanded,
            cosmic_text::Stretch::UltraExpanded => FontStretch::UltraExpanded,
        }
    }
}

/// Font family specification.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FontFamily {
    /// A specific font family by name.
    Name(String),
    /// Generic serif family.
    Serif,
    /// Generic sans-serif family.
    SansSerif,
    /// Generic monospace family.
    Monospace,
    /// Generic cursive family.
    Cursive,
    /// Generic fantasy family.
    Fantasy,
}

impl FontFamily {
    /// Create a named font family.
    pub fn name(name: impl Into<String>) -> Self {
        Self::Name(name.into())
    }

    /// Convert to fontdb Family.
    pub fn to_fontdb(&self) -> fontdb::Family<'_> {
        match self {
            FontFamily::Name(name) => fontdb::Family::Name(name.as_str()),
            FontFamily::Serif => fontdb::Family::Serif,
            FontFamily::SansSerif => fontdb::Family::SansSerif,
            FontFamily::Monospace => fontdb::Family::Monospace,
            FontFamily::Cursive => fontdb::Family::Cursive,
            FontFamily::Fantasy => fontdb::Family::Fantasy,
        }
    }

    /// Convert to cosmic-text Family.
    pub fn to_cosmic(&self) -> cosmic_text::Family<'_> {
        match self {
            FontFamily::Name(name) => cosmic_text::Family::Name(name.as_str()),
            FontFamily::Serif => cosmic_text::Family::Serif,
            FontFamily::SansSerif => cosmic_text::Family::SansSerif,
            FontFamily::Monospace => cosmic_text::Family::Monospace,
            FontFamily::Cursive => cosmic_text::Family::Cursive,
            FontFamily::Fantasy => cosmic_text::Family::Fantasy,
        }
    }
}

impl Default for FontFamily {
    fn default() -> Self {
        Self::SansSerif
    }
}

impl fmt::Display for FontFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FontFamily::Name(name) => write!(f, "{}", name),
            FontFamily::Serif => write!(f, "serif"),
            FontFamily::SansSerif => write!(f, "sans-serif"),
            FontFamily::Monospace => write!(f, "monospace"),
            FontFamily::Cursive => write!(f, "cursive"),
            FontFamily::Fantasy => write!(f, "fantasy"),
        }
    }
}

/// Font metrics containing measurements in font units.
///
/// All values are in font units and need to be scaled by `size / units_per_em`
/// to get pixel values at a specific font size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontMetrics {
    /// The number of font units per em.
    pub units_per_em: u16,
    /// The distance from the baseline to the top of the highest glyph.
    pub ascent: i16,
    /// The distance from the baseline to the bottom of the lowest glyph (typically negative).
    pub descent: i16,
    /// The recommended additional spacing between lines.
    pub line_gap: i16,
    /// The underline position relative to the baseline.
    pub underline_position: i16,
    /// The underline thickness.
    pub underline_thickness: i16,
    /// The strikeout position relative to the baseline.
    pub strikeout_position: i16,
    /// The strikeout thickness.
    pub strikeout_thickness: i16,
    /// The x-height (height of lowercase 'x'), if available.
    pub x_height: Option<i16>,
    /// The cap height (height of capital letters), if available.
    pub cap_height: Option<i16>,
}

impl FontMetrics {
    /// Calculate the line height in font units (ascent - descent + line_gap).
    pub fn line_height(&self) -> i16 {
        self.ascent - self.descent + self.line_gap
    }

    /// Scale font units to pixels for a given font size.
    pub fn scale_to_pixels(&self, font_units: i16, font_size: f32) -> f32 {
        font_units as f32 * font_size / self.units_per_em as f32
    }

    /// Get the ascent scaled to pixels for a given font size.
    pub fn ascent_px(&self, font_size: f32) -> f32 {
        self.scale_to_pixels(self.ascent, font_size)
    }

    /// Get the descent scaled to pixels for a given font size (typically negative).
    pub fn descent_px(&self, font_size: f32) -> f32 {
        self.scale_to_pixels(self.descent, font_size)
    }

    /// Get the line height scaled to pixels for a given font size.
    pub fn line_height_px(&self, font_size: f32) -> f32 {
        self.scale_to_pixels(self.line_height(), font_size)
    }
}

/// Query specification for finding fonts.
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_render::text::{FontQuery, FontFamily, FontWeight, FontStyle};
///
/// let query = FontQuery::new()
///     .family(FontFamily::Name("Helvetica".into()))
///     .weight(FontWeight::BOLD)
///     .style(FontStyle::Italic);
/// ```
#[derive(Debug, Clone, Default)]
pub struct FontQuery {
    /// Font families to search, in preference order.
    pub families: Vec<FontFamily>,
    /// Desired font weight.
    pub weight: FontWeight,
    /// Desired font style.
    pub style: FontStyle,
    /// Desired font stretch.
    pub stretch: FontStretch,
}

impl FontQuery {
    /// Create a new empty font query.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a font family to the query (highest priority).
    pub fn family(mut self, family: FontFamily) -> Self {
        self.families.insert(0, family);
        self
    }

    /// Add a font family to the fallback list (lowest priority).
    pub fn fallback(mut self, family: FontFamily) -> Self {
        self.families.push(family);
        self
    }

    /// Set the desired font weight.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the desired font style.
    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the desired font stretch.
    pub fn stretch(mut self, stretch: FontStretch) -> Self {
        self.stretch = stretch;
        self
    }
}

/// Text decoration type (underline, strikethrough, overline).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextDecorationType {
    /// Underline decoration (below baseline).
    #[default]
    Underline,
    /// Strikethrough decoration (through middle of text).
    Strikethrough,
    /// Overline decoration (above text).
    Overline,
}

/// Text decoration line style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextDecorationStyle {
    /// Solid line.
    #[default]
    Solid,
    /// Dotted line (circular dots).
    Dotted,
    /// Dashed line (short dashes).
    Dashed,
    /// Wavy line (sinusoidal wave).
    Wavy,
}

/// A complete text decoration specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextDecoration {
    /// The type of decoration.
    pub decoration_type: TextDecorationType,
    /// The line style.
    pub style: TextDecorationStyle,
    /// The decoration color (RGBA). If None, uses text color.
    pub color: Option<[u8; 4]>,
    /// Line thickness multiplier (1.0 = default thickness from font metrics).
    pub thickness: f32,
}

impl Default for TextDecoration {
    fn default() -> Self {
        Self {
            decoration_type: TextDecorationType::Underline,
            style: TextDecorationStyle::Solid,
            color: None,
            thickness: 1.0,
        }
    }
}

impl TextDecoration {
    /// Create a new underline decoration.
    pub fn underline() -> Self {
        Self {
            decoration_type: TextDecorationType::Underline,
            ..Default::default()
        }
    }

    /// Create a new strikethrough decoration.
    pub fn strikethrough() -> Self {
        Self {
            decoration_type: TextDecorationType::Strikethrough,
            ..Default::default()
        }
    }

    /// Create a new overline decoration.
    pub fn overline() -> Self {
        Self {
            decoration_type: TextDecorationType::Overline,
            ..Default::default()
        }
    }

    /// Set the line style.
    pub fn with_style(mut self, style: TextDecorationStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the decoration color (RGBA).
    pub fn with_color(mut self, color: [u8; 4]) -> Self {
        self.color = Some(color);
        self
    }

    /// Set the thickness multiplier.
    pub fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    /// Create a dotted underline.
    pub fn dotted_underline() -> Self {
        Self::underline().with_style(TextDecorationStyle::Dotted)
    }

    /// Create a wavy underline (often used for spell check).
    pub fn wavy_underline() -> Self {
        Self::underline().with_style(TextDecorationStyle::Wavy)
    }

    /// Create a dashed underline.
    pub fn dashed_underline() -> Self {
        Self::underline().with_style(TextDecorationStyle::Dashed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_weight_constants() {
        assert_eq!(FontWeight::THIN.value(), 100);
        assert_eq!(FontWeight::NORMAL.value(), 400);
        assert_eq!(FontWeight::BOLD.value(), 700);
        assert_eq!(FontWeight::BLACK.value(), 900);
    }

    #[test]
    fn font_weight_clamping() {
        assert_eq!(FontWeight::new(50).value(), 100);
        assert_eq!(FontWeight::new(1000).value(), 900);
        assert_eq!(FontWeight::new(500).value(), 500);
    }

    #[test]
    fn font_metrics_scaling() {
        let metrics = FontMetrics {
            units_per_em: 1000,
            ascent: 800,
            descent: -200,
            line_gap: 100,
            underline_position: -100,
            underline_thickness: 50,
            strikeout_position: 300,
            strikeout_thickness: 50,
            x_height: Some(500),
            cap_height: Some(700),
        };

        assert_eq!(metrics.line_height(), 1100);
        assert_eq!(metrics.ascent_px(16.0), 12.8);
        assert_eq!(metrics.descent_px(16.0), -3.2);
        assert_eq!(metrics.line_height_px(16.0), 17.6);
    }

    #[test]
    fn font_query_builder() {
        let query = FontQuery::new()
            .family(FontFamily::Name("Inter".into()))
            .fallback(FontFamily::SansSerif)
            .weight(FontWeight::BOLD)
            .style(FontStyle::Italic)
            .stretch(FontStretch::Condensed);

        assert_eq!(query.families.len(), 2);
        assert_eq!(query.families[0], FontFamily::Name("Inter".into()));
        assert_eq!(query.families[1], FontFamily::SansSerif);
        assert_eq!(query.weight, FontWeight::BOLD);
        assert_eq!(query.style, FontStyle::Italic);
        assert_eq!(query.stretch, FontStretch::Condensed);
    }

    #[test]
    fn text_decoration_builders() {
        let underline = TextDecoration::underline();
        assert_eq!(underline.decoration_type, TextDecorationType::Underline);
        assert_eq!(underline.style, TextDecorationStyle::Solid);
        assert_eq!(underline.thickness, 1.0);
        assert!(underline.color.is_none());

        let strikethrough = TextDecoration::strikethrough();
        assert_eq!(strikethrough.decoration_type, TextDecorationType::Strikethrough);

        let overline = TextDecoration::overline();
        assert_eq!(overline.decoration_type, TextDecorationType::Overline);
    }

    #[test]
    fn text_decoration_styles() {
        let dotted = TextDecoration::dotted_underline();
        assert_eq!(dotted.style, TextDecorationStyle::Dotted);

        let wavy = TextDecoration::wavy_underline();
        assert_eq!(wavy.style, TextDecorationStyle::Wavy);

        let dashed = TextDecoration::dashed_underline();
        assert_eq!(dashed.style, TextDecorationStyle::Dashed);
    }

    #[test]
    fn text_decoration_customization() {
        let custom = TextDecoration::underline()
            .with_style(TextDecorationStyle::Wavy)
            .with_color([255, 0, 0, 255])
            .with_thickness(2.0);

        assert_eq!(custom.style, TextDecorationStyle::Wavy);
        assert_eq!(custom.color, Some([255, 0, 0, 255]));
        assert_eq!(custom.thickness, 2.0);
    }
}
