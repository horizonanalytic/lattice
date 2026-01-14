//! Font representation and configuration.

use super::types::{FontFamily, FontQuery, FontStretch, FontStyle, FontWeight};

/// An OpenType font feature tag.
///
/// Font features control advanced typographic options like ligatures,
/// small caps, stylistic alternates, etc.
///
/// # Example
///
/// ```
/// use horizon_lattice_render::text::FontFeature;
///
/// // Enable common ligatures
/// let liga = FontFeature::new(*b"liga", 1);
///
/// // Disable kerning
/// let no_kern = FontFeature::new(*b"kern", 0);
///
/// // Use predefined constants
/// let smcp = FontFeature::SMALL_CAPS;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FontFeature {
    /// The 4-character OpenType feature tag.
    pub tag: [u8; 4],
    /// The feature value (0 = disabled, 1 = enabled, or higher for alternates).
    pub value: u32,
}

impl FontFeature {
    /// Standard ligatures (liga).
    pub const LIGATURES: Self = Self::new(*b"liga", 1);
    /// Disable ligatures.
    pub const NO_LIGATURES: Self = Self::new(*b"liga", 0);
    /// Contextual ligatures (clig).
    pub const CONTEXTUAL_LIGATURES: Self = Self::new(*b"clig", 1);
    /// Discretionary ligatures (dlig).
    pub const DISCRETIONARY_LIGATURES: Self = Self::new(*b"dlig", 1);
    /// Historical ligatures (hlig).
    pub const HISTORICAL_LIGATURES: Self = Self::new(*b"hlig", 1);
    /// Kerning (kern).
    pub const KERNING: Self = Self::new(*b"kern", 1);
    /// Disable kerning.
    pub const NO_KERNING: Self = Self::new(*b"kern", 0);
    /// Small caps (smcp).
    pub const SMALL_CAPS: Self = Self::new(*b"smcp", 1);
    /// All small caps (c2sc + smcp).
    pub const ALL_SMALL_CAPS: Self = Self::new(*b"c2sc", 1);
    /// Oldstyle figures (onum).
    pub const OLDSTYLE_FIGURES: Self = Self::new(*b"onum", 1);
    /// Lining figures (lnum).
    pub const LINING_FIGURES: Self = Self::new(*b"lnum", 1);
    /// Tabular figures (tnum).
    pub const TABULAR_FIGURES: Self = Self::new(*b"tnum", 1);
    /// Proportional figures (pnum).
    pub const PROPORTIONAL_FIGURES: Self = Self::new(*b"pnum", 1);
    /// Fractions (frac).
    pub const FRACTIONS: Self = Self::new(*b"frac", 1);
    /// Superscript (sups).
    pub const SUPERSCRIPT: Self = Self::new(*b"sups", 1);
    /// Subscript (subs).
    pub const SUBSCRIPT: Self = Self::new(*b"subs", 1);
    /// Ordinals (ordn).
    pub const ORDINALS: Self = Self::new(*b"ordn", 1);
    /// Slashed zero (zero).
    pub const SLASHED_ZERO: Self = Self::new(*b"zero", 1);
    /// Case-sensitive forms (case).
    pub const CASE_SENSITIVE: Self = Self::new(*b"case", 1);

    /// Create a new font feature with the given tag and value.
    pub const fn new(tag: [u8; 4], value: u32) -> Self {
        Self { tag, value }
    }

    /// Create a font feature from a string tag.
    ///
    /// The tag must be exactly 4 ASCII characters.
    pub fn from_str(tag: &str, value: u32) -> Option<Self> {
        if tag.len() != 4 || !tag.is_ascii() {
            return None;
        }
        let bytes = tag.as_bytes();
        Some(Self::new([bytes[0], bytes[1], bytes[2], bytes[3]], value))
    }

    /// Get the feature tag as a string.
    pub fn tag_str(&self) -> &str {
        std::str::from_utf8(&self.tag).unwrap_or("????")
    }

    /// Check if this feature is enabled (value > 0).
    pub fn is_enabled(&self) -> bool {
        self.value > 0
    }
}

/// A complete font specification including family, size, and styling.
///
/// `Font` represents the styling attributes to apply when rendering text.
/// It does not directly reference font data; use [`FontSystem`] to resolve
/// a `Font` to actual font face data.
///
/// # Example
///
/// ```
/// use horizon_lattice_render::text::{Font, FontFamily, FontWeight, FontStyle, FontFeature};
///
/// // Create a simple font
/// let font = Font::new(FontFamily::SansSerif, 16.0);
///
/// // Create a styled font using the builder
/// let styled = Font::builder()
///     .family(FontFamily::Name("Inter".into()))
///     .size(14.0)
///     .weight(FontWeight::MEDIUM)
///     .style(FontStyle::Italic)
///     .feature(FontFeature::LIGATURES)
///     .build();
/// ```
///
/// [`FontSystem`]: super::FontSystem
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// The font family (or families for fallback).
    families: Vec<FontFamily>,
    /// Font size in pixels.
    size: f32,
    /// Font weight.
    weight: FontWeight,
    /// Font style.
    style: FontStyle,
    /// Font stretch.
    stretch: FontStretch,
    /// OpenType features to enable/disable.
    features: Vec<FontFeature>,
    /// Letter spacing adjustment in pixels.
    letter_spacing: f32,
    /// Word spacing adjustment in pixels.
    word_spacing: f32,
}

impl Font {
    /// Create a new font with the given family and size.
    pub fn new(family: FontFamily, size: f32) -> Self {
        Self {
            families: vec![family],
            size,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
            stretch: FontStretch::Normal,
            features: Vec::new(),
            letter_spacing: 0.0,
            word_spacing: 0.0,
        }
    }

    /// Create a font builder for more complex font specifications.
    pub fn builder() -> FontBuilder {
        FontBuilder::new()
    }

    /// Get the primary font family.
    pub fn family(&self) -> &FontFamily {
        self.families.first().unwrap_or(&FontFamily::SansSerif)
    }

    /// Get all font families (primary and fallbacks).
    pub fn families(&self) -> &[FontFamily] {
        &self.families
    }

    /// Get the font size in pixels.
    pub fn size(&self) -> f32 {
        self.size
    }

    /// Get the font weight.
    pub fn weight(&self) -> FontWeight {
        self.weight
    }

    /// Get the font style.
    pub fn style(&self) -> FontStyle {
        self.style
    }

    /// Get the font stretch.
    pub fn stretch(&self) -> FontStretch {
        self.stretch
    }

    /// Get the OpenType features.
    pub fn features(&self) -> &[FontFeature] {
        &self.features
    }

    /// Get the letter spacing adjustment.
    pub fn letter_spacing(&self) -> f32 {
        self.letter_spacing
    }

    /// Get the word spacing adjustment.
    pub fn word_spacing(&self) -> f32 {
        self.word_spacing
    }

    /// Create a font query from this font specification.
    pub fn to_query(&self) -> FontQuery {
        let mut query = FontQuery::new()
            .weight(self.weight)
            .style(self.style)
            .stretch(self.stretch);

        // Add families in order
        for family in self.families.iter().rev() {
            query = query.family(family.clone());
        }

        query
    }

    /// Create a copy of this font with a different size.
    pub fn with_size(&self, size: f32) -> Self {
        let mut font = self.clone();
        font.size = size;
        font
    }

    /// Create a copy of this font with a different weight.
    pub fn with_weight(&self, weight: FontWeight) -> Self {
        let mut font = self.clone();
        font.weight = weight;
        font
    }

    /// Create a copy of this font with a different style.
    pub fn with_style(&self, style: FontStyle) -> Self {
        let mut font = self.clone();
        font.style = style;
        font
    }

    /// Convert to cosmic-text Attrs for text shaping.
    pub fn to_attrs(&self) -> cosmic_text::Attrs<'_> {
        let family = match self.family() {
            FontFamily::Name(name) => cosmic_text::Family::Name(name.as_str()),
            FontFamily::Serif => cosmic_text::Family::Serif,
            FontFamily::SansSerif => cosmic_text::Family::SansSerif,
            FontFamily::Monospace => cosmic_text::Family::Monospace,
            FontFamily::Cursive => cosmic_text::Family::Cursive,
            FontFamily::Fantasy => cosmic_text::Family::Fantasy,
        };

        cosmic_text::Attrs::new()
            .family(family)
            .weight(self.weight.to_cosmic())
            .style(self.style.to_cosmic())
            .stretch(self.stretch.to_cosmic())
    }
}

impl Default for Font {
    fn default() -> Self {
        Self::new(FontFamily::SansSerif, 16.0)
    }
}

/// Builder for creating `Font` instances with complex configurations.
#[derive(Debug, Clone, Default)]
pub struct FontBuilder {
    families: Vec<FontFamily>,
    size: Option<f32>,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    features: Vec<FontFeature>,
    letter_spacing: f32,
    word_spacing: f32,
}

impl FontBuilder {
    /// Create a new font builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the primary font family.
    pub fn family(mut self, family: FontFamily) -> Self {
        if self.families.is_empty() {
            self.families.push(family);
        } else {
            self.families[0] = family;
        }
        self
    }

    /// Add a fallback font family.
    pub fn fallback(mut self, family: FontFamily) -> Self {
        self.families.push(family);
        self
    }

    /// Set the font size in pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Set the font weight.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the font style.
    pub fn style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the font stretch.
    pub fn stretch(mut self, stretch: FontStretch) -> Self {
        self.stretch = stretch;
        self
    }

    /// Add an OpenType feature.
    pub fn feature(mut self, feature: FontFeature) -> Self {
        self.features.push(feature);
        self
    }

    /// Set the letter spacing adjustment in pixels.
    pub fn letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = spacing;
        self
    }

    /// Set the word spacing adjustment in pixels.
    pub fn word_spacing(mut self, spacing: f32) -> Self {
        self.word_spacing = spacing;
        self
    }

    /// Build the font specification.
    ///
    /// If no family was specified, uses SansSerif.
    /// If no size was specified, uses 16.0.
    pub fn build(self) -> Font {
        let families = if self.families.is_empty() {
            vec![FontFamily::SansSerif]
        } else {
            self.families
        };

        Font {
            families,
            size: self.size.unwrap_or(16.0),
            weight: self.weight,
            style: self.style,
            stretch: self.stretch,
            features: self.features,
            letter_spacing: self.letter_spacing,
            word_spacing: self.word_spacing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn font_creation() {
        let font = Font::new(FontFamily::SansSerif, 14.0);
        assert_eq!(font.size(), 14.0);
        assert_eq!(font.weight(), FontWeight::NORMAL);
        assert_eq!(font.style(), FontStyle::Normal);
    }

    #[test]
    fn font_builder() {
        let font = Font::builder()
            .family(FontFamily::Name("Inter".into()))
            .fallback(FontFamily::SansSerif)
            .size(18.0)
            .weight(FontWeight::BOLD)
            .style(FontStyle::Italic)
            .feature(FontFeature::LIGATURES)
            .letter_spacing(0.5)
            .build();

        assert_eq!(font.families().len(), 2);
        assert_eq!(font.family(), &FontFamily::Name("Inter".into()));
        assert_eq!(font.size(), 18.0);
        assert_eq!(font.weight(), FontWeight::BOLD);
        assert_eq!(font.style(), FontStyle::Italic);
        assert_eq!(font.features().len(), 1);
        assert_eq!(font.letter_spacing(), 0.5);
    }

    #[test]
    fn font_with_methods() {
        let font = Font::new(FontFamily::Monospace, 12.0);
        let larger = font.with_size(24.0);
        let bold = font.with_weight(FontWeight::BOLD);

        assert_eq!(larger.size(), 24.0);
        assert_eq!(larger.weight(), FontWeight::NORMAL);
        assert_eq!(bold.size(), 12.0);
        assert_eq!(bold.weight(), FontWeight::BOLD);
    }

    #[test]
    fn font_feature_creation() {
        let liga = FontFeature::LIGATURES;
        assert_eq!(liga.tag_str(), "liga");
        assert!(liga.is_enabled());

        let no_kern = FontFeature::NO_KERNING;
        assert_eq!(no_kern.tag_str(), "kern");
        assert!(!no_kern.is_enabled());

        let custom = FontFeature::from_str("ss01", 1).unwrap();
        assert_eq!(custom.tag_str(), "ss01");
    }

    #[test]
    fn font_to_query() {
        let font = Font::builder()
            .family(FontFamily::Name("Helvetica".into()))
            .fallback(FontFamily::SansSerif)
            .weight(FontWeight::MEDIUM)
            .style(FontStyle::Oblique)
            .build();

        let query = font.to_query();
        assert_eq!(query.families.len(), 2);
        assert_eq!(query.weight, FontWeight::MEDIUM);
        assert_eq!(query.style, FontStyle::Oblique);
    }
}
