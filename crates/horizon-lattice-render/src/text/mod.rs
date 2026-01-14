//! Text rendering subsystem for Horizon Lattice.
//!
//! This module provides font loading, text shaping, text layout, and glyph rendering
//! capabilities built on top of cosmic-text and fontdb.
//!
//! # Getting Started
//!
//! First, initialize the font system (typically done once per application):
//!
//! ```no_run
//! use horizon_lattice_render::text::FontSystem;
//!
//! // Create font system with auto-loaded system fonts
//! let font_system = FontSystem::new();
//!
//! // Access font database for queries
//! let db = font_system.database();
//! println!("Loaded {} font faces", db.faces().count());
//! ```
//!
//! # Font Queries
//!
//! Query fonts by family, weight, style, and stretch:
//!
//! ```no_run
//! use horizon_lattice_render::text::{FontSystem, FontQuery, FontWeight, FontStyle, FontFamily};
//!
//! let font_system = FontSystem::new();
//!
//! // Query for a specific font
//! let query = FontQuery::new()
//!     .family(FontFamily::name("Inter"))
//!     .weight(FontWeight::BOLD)
//!     .style(FontStyle::Normal);
//!
//! if let Some(face_id) = font_system.query(&query) {
//!     println!("Found matching font");
//! }
//! ```
//!
//! # Text Shaping
//!
//! Shape text to get positioned glyphs:
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
//! println!("Text width: {} pixels", shaped.width());
//! for glyph in shaped.glyphs() {
//!     println!("Glyph {} at ({}, {})", glyph.glyph_id.value(), glyph.x, glyph.y);
//! }
//! ```
//!
//! # Text Layout
//!
//! Layout text with alignment, wrapping, and rich text support:
//!
//! ```no_run
//! use horizon_lattice_render::text::{
//!     FontSystem, Font, FontFamily, TextLayout, TextLayoutOptions,
//!     HorizontalAlign, WrapMode, TextSpan, FontWeight,
//! };
//!
//! let mut font_system = FontSystem::new();
//! let font = Font::new(FontFamily::SansSerif, 16.0);
//!
//! // Simple single-line layout
//! let layout = TextLayout::new(&mut font_system, "Hello, World!", &font);
//! println!("Size: {}x{}", layout.width(), layout.height());
//!
//! // Multi-line layout with wrapping
//! let options = TextLayoutOptions::new()
//!     .max_width(200.0)
//!     .wrap(WrapMode::Word)
//!     .horizontal_align(HorizontalAlign::Center);
//!
//! let layout = TextLayout::with_options(
//!     &mut font_system,
//!     "This is a long text that will wrap",
//!     &font,
//!     options,
//! );
//! println!("Lines: {}", layout.line_count());
//!
//! // Rich text with multiple styles
//! let spans = vec![
//!     TextSpan::new("Normal "),
//!     TextSpan::new("bold").bold(&font),
//!     TextSpan::new(" and "),
//!     TextSpan::new("colored").with_color([255, 0, 0, 255]),
//! ];
//!
//! let rich_layout = TextLayout::rich_text(
//!     &mut font_system,
//!     &spans,
//!     &font,
//!     TextLayoutOptions::default(),
//! );
//! ```

mod font;
mod font_system;
mod glyph_atlas;
mod glyph_cache;
mod layout;
mod shaping;
mod types;

pub use font::{Font, FontBuilder, FontFeature};
pub use font_system::{FontFaceInfo, FontLoadError, FontSystem, FontSystemConfig};
pub use glyph_atlas::{GlyphAllocation, GlyphAtlas, GlyphAtlasStats};
pub use glyph_cache::{
    GlyphCache, GlyphCacheStats, GlyphPixelFormat, GlyphRenderMode, RasterizedGlyph,
};
pub use layout::{
    HorizontalAlign, InlineElement, InlineVerticalAlign, LayoutGlyph, LayoutLine, SelectionRect,
    TextLayout, TextLayoutOptions, TextSpan, VerticalAlign, WrapMode,
};
pub use shaping::{GlyphId, ShapedGlyph, ShapedText, ShapingOptions, TextShaper};
pub use types::{FontFamily, FontMetrics, FontQuery, FontStretch, FontStyle, FontWeight};

// Re-export fontdb::ID for users who need to work with font face IDs
pub use fontdb::ID as FontFaceId;

// Re-export cosmic_text::CacheKey for glyph rendering integration
pub use cosmic_text::CacheKey as GlyphCacheKey;
