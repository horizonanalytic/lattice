//! Text shaping using cosmic-text.
//!
//! This module provides text shaping functionality that converts a string
//! of text into positioned glyphs ready for rendering. It handles:
//!
//! - Unicode text segmentation
//! - Bidirectional text (RTL support)
//! - Script detection
//! - Language-specific shaping
//! - OpenType features (kerning, ligatures, etc.)
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::text::{FontSystem, Font, FontFamily, TextShaper, ShapingOptions};
//!
//! let mut font_system = FontSystem::new();
//! let font = Font::new(FontFamily::SansSerif, 16.0);
//!
//! let mut shaper = TextShaper::new();
//! let shaped = shaper.shape_text(&mut font_system, "Hello, World!", &font, ShapingOptions::default());
//!
//! for glyph in shaped.glyphs() {
//!     println!("Glyph {} at ({}, {})", glyph.glyph_id.value(), glyph.x, glyph.y);
//! }
//! ```

use std::ops::Range;

use cosmic_text::{Attrs, Buffer, CacheKeyFlags, Metrics, Shaping};
use fontdb::ID as FontFaceId;

use super::{Font, FontFeature, FontSystem};

/// A unique identifier for a glyph within a font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphId(pub u16);

impl GlyphId {
    /// Create a new glyph ID.
    pub const fn new(id: u16) -> Self {
        Self(id)
    }

    /// Get the raw glyph ID value.
    pub const fn value(self) -> u16 {
        self.0
    }
}

impl From<u16> for GlyphId {
    fn from(id: u16) -> Self {
        Self(id)
    }
}

/// A shaped glyph with positioning information.
///
/// After text shaping, each glyph has a specific position relative to
/// the text origin and an advance that determines spacing to the next glyph.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedGlyph {
    /// The glyph identifier within the font.
    pub glyph_id: GlyphId,
    /// The font face ID for this glyph.
    pub font_id: FontFaceId,
    /// X position relative to the text origin.
    pub x: f32,
    /// Y position relative to the text origin (baseline).
    pub y: f32,
    /// Width of the glyph (for hit testing).
    pub width: f32,
    /// The byte range in the original text that this glyph represents.
    /// Multiple glyphs may share the same cluster for ligatures.
    pub cluster: Range<usize>,
    /// The font size this glyph was shaped at.
    pub font_size: f32,
    /// Flags for cache key generation during rasterization.
    pub cache_key_flags: CacheKeyFlags,
    /// The level of this glyph for bidirectional text (0 = LTR, 1 = RTL, etc.).
    pub level: u8,
}

impl ShapedGlyph {
    /// Check if this glyph is part of right-to-left text.
    pub fn is_rtl(&self) -> bool {
        self.level % 2 == 1
    }

    /// Get the rightmost x position of this glyph.
    pub fn x_end(&self) -> f32 {
        self.x + self.width
    }

    /// Check if a point is within this glyph's horizontal bounds.
    pub fn contains_x(&self, x: f32) -> bool {
        x >= self.x && x < self.x_end()
    }
}

/// The result of shaping a line of text.
///
/// `ShapedText` contains all the shaped glyphs for a single line of text,
/// along with metrics for the entire line.
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_render::text::{FontSystem, Font, FontFamily, TextShaper, ShapingOptions};
///
/// let mut font_system = FontSystem::new();
/// let font = Font::new(FontFamily::SansSerif, 16.0);
/// let mut shaper = TextShaper::new();
///
/// let shaped = shaper.shape_text(&mut font_system, "Hello!", &font, ShapingOptions::default());
///
/// // Get metrics
/// println!("Width: {}", shaped.width());
/// println!("Line height: {}", shaped.line_height());
/// println!("Ascent: {}", shaped.ascent());
/// println!("Descent: {}", shaped.descent());
///
/// // Iterate glyphs
/// for glyph in shaped.glyphs() {
///     println!("Glyph {} at ({}, {})", glyph.glyph_id.value(), glyph.x, glyph.y);
/// }
///
/// // Find glyph at position (for hit testing)
/// if let Some(idx) = shaped.glyph_at_x(25.0) {
///     println!("Glyph index at x=25: {}", idx);
/// }
///
/// // Get cursor position for character offset
/// let cursor_x = shaped.x_for_offset(3); // Position after 3rd byte
/// ```
#[derive(Debug, Clone)]
pub struct ShapedText {
    /// The original text that was shaped.
    text: String,
    /// The shaped glyphs in visual order.
    glyphs: Vec<ShapedGlyph>,
    /// Total width of the shaped text.
    width: f32,
    /// Line height (ascent + descent + line gap).
    line_height: f32,
    /// Distance from baseline to top of line.
    ascent: f32,
    /// Distance from baseline to bottom of line (typically negative).
    descent: f32,
}

impl ShapedText {
    /// Get the original text that was shaped.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the shaped glyphs in visual order.
    pub fn glyphs(&self) -> &[ShapedGlyph] {
        &self.glyphs
    }

    /// Get the total width of the shaped text.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Get the line height.
    pub fn line_height(&self) -> f32 {
        self.line_height
    }

    /// Get the ascent (distance from baseline to top).
    pub fn ascent(&self) -> f32 {
        self.ascent
    }

    /// Get the descent (distance from baseline to bottom, typically negative).
    pub fn descent(&self) -> f32 {
        self.descent
    }

    /// Check if the shaped text is empty (no glyphs).
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// Get the number of glyphs.
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// Find the glyph at a given x position.
    ///
    /// Returns the index of the glyph containing the x coordinate,
    /// or `None` if x is outside the text bounds.
    pub fn glyph_at_x(&self, x: f32) -> Option<usize> {
        self.glyphs.iter().position(|g| g.contains_x(x))
    }

    /// Find the cluster (character) index at a given x position.
    ///
    /// Returns the byte offset in the original string, suitable for
    /// cursor positioning.
    pub fn cluster_at_x(&self, x: f32) -> usize {
        if x <= 0.0 || self.glyphs.is_empty() {
            return 0;
        }

        if x >= self.width {
            return self.text.len();
        }

        // Find the glyph at this position
        for glyph in &self.glyphs {
            if glyph.contains_x(x) {
                // Determine if we're in the left or right half of the glyph
                let mid = glyph.x + glyph.width / 2.0;
                if x < mid {
                    return glyph.cluster.start;
                } else {
                    return glyph.cluster.end;
                }
            }
        }

        self.text.len()
    }

    /// Get the x position for a given byte offset in the original string.
    ///
    /// Useful for positioning a cursor at a specific character.
    pub fn x_for_offset(&self, offset: usize) -> f32 {
        if offset == 0 || self.glyphs.is_empty() {
            return 0.0;
        }

        // Find the glyph that contains or follows this offset
        for glyph in &self.glyphs {
            if glyph.cluster.start >= offset {
                return glyph.x;
            }
            if glyph.cluster.contains(&offset) {
                return glyph.x;
            }
        }

        self.width
    }

    /// Check if the text contains any RTL (right-to-left) segments.
    pub fn has_rtl(&self) -> bool {
        self.glyphs.iter().any(|g| g.is_rtl())
    }
}

impl Default for ShapedText {
    fn default() -> Self {
        Self {
            text: String::new(),
            glyphs: Vec::new(),
            width: 0.0,
            line_height: 0.0,
            ascent: 0.0,
            descent: 0.0,
        }
    }
}

/// Options for text shaping.
///
/// # Examples
///
/// ```
/// use horizon_lattice_render::text::{ShapingOptions, FontFeature};
///
/// // Default options (advanced shaping enabled)
/// let default = ShapingOptions::default();
/// assert!(default.advanced);
///
/// // Simple shaping for ASCII text (faster)
/// let simple = ShapingOptions::new().simple();
/// assert!(!simple.advanced);
///
/// // Enable specific OpenType features
/// let with_features = ShapingOptions::new()
///     .with_ligatures()
///     .with_kerning();
///
/// // Add custom features
/// let custom = ShapingOptions::new()
///     .feature(FontFeature::SMALL_CAPS)
///     .feature(FontFeature::TABULAR_FIGURES);
/// ```
#[derive(Debug, Clone)]
pub struct ShapingOptions {
    /// OpenType features to apply during shaping.
    pub features: Vec<FontFeature>,
    /// Whether to use advanced shaping (handles complex scripts).
    /// Set to false for simple ASCII text to improve performance.
    pub advanced: bool,
}

impl Default for ShapingOptions {
    fn default() -> Self {
        Self {
            features: Vec::new(),
            advanced: true,
        }
    }
}

impl ShapingOptions {
    /// Create new shaping options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable simple shaping mode (faster for ASCII text).
    pub fn simple(mut self) -> Self {
        self.advanced = false;
        self
    }

    /// Add an OpenType feature.
    pub fn feature(mut self, feature: FontFeature) -> Self {
        self.features.push(feature);
        self
    }

    /// Enable standard ligatures.
    pub fn with_ligatures(self) -> Self {
        self.feature(FontFeature::LIGATURES)
    }

    /// Disable ligatures.
    pub fn without_ligatures(self) -> Self {
        self.feature(FontFeature::NO_LIGATURES)
    }

    /// Enable kerning.
    pub fn with_kerning(self) -> Self {
        self.feature(FontFeature::KERNING)
    }

    /// Disable kerning.
    pub fn without_kerning(self) -> Self {
        self.feature(FontFeature::NO_KERNING)
    }
}

/// Text shaper for converting text to positioned glyphs.
///
/// The `TextShaper` maintains an internal buffer for efficient shaping
/// of multiple text strings. Create one shaper and reuse it for multiple
/// shaping operations.
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_render::text::{FontSystem, Font, FontFamily, TextShaper, ShapingOptions};
///
/// let mut font_system = FontSystem::new();
/// let font = Font::new(FontFamily::SansSerif, 16.0);
///
/// let mut shaper = TextShaper::new();
///
/// // Shape multiple strings efficiently
/// let hello = shaper.shape_text(&mut font_system, "Hello", &font, ShapingOptions::default());
/// let world = shaper.shape_text(&mut font_system, "World", &font, ShapingOptions::default());
/// ```
pub struct TextShaper {
    /// Internal buffer for shaping.
    buffer: Buffer,
}

impl TextShaper {
    /// Create a new text shaper.
    pub fn new() -> Self {
        // Create an empty buffer - we'll configure it per-shape call
        let buffer = Buffer::new_empty(Metrics::new(16.0, 20.0));
        Self { buffer }
    }

    /// Shape a string of text using the given font.
    ///
    /// Returns a `ShapedText` containing positioned glyphs ready for rendering.
    pub fn shape_text(
        &mut self,
        font_system: &mut FontSystem,
        text: &str,
        font: &Font,
        options: ShapingOptions,
    ) -> ShapedText {
        if text.is_empty() {
            return ShapedText::default();
        }

        // Set up metrics for this font
        let font_size = font.size();
        let line_height = font_size * 1.2; // Default 120% line height
        let metrics = Metrics::new(font_size, line_height);

        // Reset buffer with new metrics
        self.buffer.set_metrics(font_system.inner_mut(), metrics);

        // Build attributes from font
        let attrs = self.build_attrs(font, &options);

        // Set the text with a single line (no width constraint for single-line shaping)
        self.buffer.set_text(
            font_system.inner_mut(),
            text,
            attrs,
            if options.advanced {
                Shaping::Advanced
            } else {
                Shaping::Basic
            },
        );

        // Make sure text is shaped
        self.buffer.shape_until_scroll(font_system.inner_mut(), false);

        // Extract shaped glyphs
        self.extract_shaped_text(text, font_size, line_height)
    }

    /// Build cosmic-text Attrs from our Font type.
    fn build_attrs<'a>(&self, font: &'a Font, _options: &ShapingOptions) -> Attrs<'a> {
        font.to_attrs()
        // Note: cosmic-text handles OpenType features internally through rustybuzz.
        // The features from ShapingOptions would need to be applied via a custom
        // shaping implementation if fine-grained control is needed.
    }

    /// Extract shaped glyphs from the buffer after shaping.
    fn extract_shaped_text(&self, text: &str, font_size: f32, line_height: f32) -> ShapedText {
        let mut glyphs = Vec::new();
        let mut total_width: f32 = 0.0;
        let mut ascent: f32 = 0.0;
        let mut descent: f32 = 0.0;

        // Iterate through layout runs
        for run in self.buffer.layout_runs() {
            // Track metrics from the run
            ascent = ascent.max(run.line_y - line_height / 2.0 + font_size * 0.8);
            descent = descent.min(run.line_y - line_height / 2.0 - font_size * 0.2);

            for layout_glyph in run.glyphs.iter() {
                let glyph = ShapedGlyph {
                    glyph_id: GlyphId::new(layout_glyph.glyph_id),
                    font_id: layout_glyph.font_id,
                    x: layout_glyph.x,
                    y: run.line_y,
                    width: layout_glyph.w,
                    cluster: layout_glyph.start..layout_glyph.end,
                    font_size: layout_glyph.font_size,
                    cache_key_flags: layout_glyph.cache_key_flags,
                    level: layout_glyph.level.into(),
                };

                total_width = total_width.max(glyph.x_end());
                glyphs.push(glyph);
            }
        }

        // Derive ascent/descent from metrics if we didn't get good values
        if ascent == 0.0 {
            ascent = font_size * 0.8;
        }
        if descent == 0.0 {
            descent = -font_size * 0.2;
        }

        ShapedText {
            text: text.to_string(),
            glyphs,
            width: total_width,
            line_height,
            ascent,
            descent,
        }
    }
}

impl Default for TextShaper {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{FontFamily, FontSystemConfig};

    fn create_test_font_system() -> FontSystem {
        // Create without system fonts for faster testing
        let config = FontSystemConfig::new().load_system_fonts(false);
        FontSystem::with_config(config)
    }

    #[test]
    fn glyph_id_creation() {
        let id = GlyphId::new(42);
        assert_eq!(id.value(), 42);

        let id2: GlyphId = 100u16.into();
        assert_eq!(id2.value(), 100);
    }

    #[test]
    fn shaped_glyph_rtl_detection() {
        let ltr_glyph = ShapedGlyph {
            glyph_id: GlyphId::new(1),
            font_id: fontdb::ID::dummy(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            cluster: 0..1,
            font_size: 16.0,
            cache_key_flags: CacheKeyFlags::empty(),
            level: 0,
        };
        assert!(!ltr_glyph.is_rtl());

        let rtl_glyph = ShapedGlyph {
            level: 1,
            ..ltr_glyph
        };
        assert!(rtl_glyph.is_rtl());
    }

    #[test]
    fn shaped_glyph_bounds() {
        let glyph = ShapedGlyph {
            glyph_id: GlyphId::new(1),
            font_id: fontdb::ID::dummy(),
            x: 10.0,
            y: 0.0,
            width: 20.0,
            cluster: 0..1,
            font_size: 16.0,
            cache_key_flags: CacheKeyFlags::empty(),
            level: 0,
        };

        assert_eq!(glyph.x_end(), 30.0);
        assert!(glyph.contains_x(15.0));
        assert!(glyph.contains_x(10.0));
        assert!(!glyph.contains_x(9.9));
        assert!(!glyph.contains_x(30.0));
    }

    #[test]
    fn shaped_text_empty() {
        let shaped = ShapedText::default();
        assert!(shaped.is_empty());
        assert_eq!(shaped.glyph_count(), 0);
        assert_eq!(shaped.width(), 0.0);
        assert!(!shaped.has_rtl());
    }

    #[test]
    fn shaping_options_builder() {
        let opts = ShapingOptions::new()
            .simple()
            .with_ligatures()
            .with_kerning();

        assert!(!opts.advanced);
        assert_eq!(opts.features.len(), 2);
    }

    #[test]
    fn text_shaper_creation() {
        let shaper = TextShaper::new();
        // Just verify it doesn't panic
        drop(shaper);
    }

    #[test]
    fn shape_empty_text() {
        let mut font_system = create_test_font_system();
        let font = Font::new(FontFamily::SansSerif, 16.0);
        let mut shaper = TextShaper::new();

        let shaped = shaper.shape_text(
            &mut font_system,
            "",
            &font,
            ShapingOptions::default(),
        );

        assert!(shaped.is_empty());
    }

    #[test]
    fn shaped_text_cluster_lookup() {
        // Test the cluster lookup methods with a manually constructed ShapedText
        let glyphs = vec![
            ShapedGlyph {
                glyph_id: GlyphId::new(1),
                font_id: fontdb::ID::dummy(),
                x: 0.0,
                y: 0.0,
                width: 10.0,
                cluster: 0..1,
                font_size: 16.0,
                cache_key_flags: CacheKeyFlags::empty(),
                level: 0,
            },
            ShapedGlyph {
                glyph_id: GlyphId::new(2),
                font_id: fontdb::ID::dummy(),
                x: 10.0,
                y: 0.0,
                width: 10.0,
                cluster: 1..2,
                font_size: 16.0,
                cache_key_flags: CacheKeyFlags::empty(),
                level: 0,
            },
        ];

        let shaped = ShapedText {
            text: "ab".to_string(),
            glyphs,
            width: 20.0,
            line_height: 20.0,
            ascent: 16.0,
            descent: -4.0,
        };

        // Test glyph_at_x
        assert_eq!(shaped.glyph_at_x(5.0), Some(0));
        assert_eq!(shaped.glyph_at_x(15.0), Some(1));
        assert_eq!(shaped.glyph_at_x(-1.0), None);
        assert_eq!(shaped.glyph_at_x(25.0), None);

        // Test cluster_at_x
        assert_eq!(shaped.cluster_at_x(3.0), 0); // Left half of first glyph
        assert_eq!(shaped.cluster_at_x(7.0), 1); // Right half of first glyph
        assert_eq!(shaped.cluster_at_x(13.0), 1); // Left half of second glyph
        assert_eq!(shaped.cluster_at_x(17.0), 2); // Right half of second glyph
        assert_eq!(shaped.cluster_at_x(-5.0), 0); // Before text
        assert_eq!(shaped.cluster_at_x(100.0), 2); // After text

        // Test x_for_offset
        assert_eq!(shaped.x_for_offset(0), 0.0);
        assert_eq!(shaped.x_for_offset(1), 10.0);
        assert_eq!(shaped.x_for_offset(2), 20.0);
    }
}
