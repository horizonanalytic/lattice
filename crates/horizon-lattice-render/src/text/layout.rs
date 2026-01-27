//! Text layout for single-line and multi-line text rendering.
//!
//! This module provides text layout capabilities including:
//! - Single-line and multi-line text layout
//! - Horizontal alignment (left, center, right, justified)
//! - Vertical alignment (top, middle, bottom)
//! - Word and character wrapping
//! - Ellipsis truncation
//! - Rich text with multiple fonts and styles
//! - Inline elements (images, widgets)
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::text::{
//!     FontSystem, Font, FontFamily, TextLayout, TextLayoutOptions,
//!     HorizontalAlign, WrapMode,
//! };
//!
//! let mut font_system = FontSystem::new();
//! let font = Font::new(FontFamily::SansSerif, 16.0);
//!
//! // Create a single-line layout
//! let layout = TextLayout::new(&mut font_system, "Hello, World!", &font);
//! println!("Text size: {}x{}", layout.width(), layout.height());
//!
//! // Create a multi-line layout with wrapping
//! let options = TextLayoutOptions::default()
//!     .max_width(200.0)
//!     .wrap(WrapMode::Word);
//!
//! let wrapped = TextLayout::with_options(&mut font_system, "Long text here...", &font, options);
//! println!("Lines: {}", wrapped.line_count());
//! ```

use std::ops::Range;

use cosmic_text::{Attrs, Buffer, CacheKeyFlags, Metrics, Shaping, Wrap};
use fontdb::ID as FontFaceId;
use unicode_segmentation::UnicodeSegmentation;

use super::{Font, FontStyle, FontSystem, FontWeight, TextDecoration, TextDirection};
#[cfg(test)]
use super::FontFamily;

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HorizontalAlign {
    /// Left-aligned text (default for LTR languages).
    #[default]
    Left,
    /// Center-aligned text.
    Center,
    /// Right-aligned text (default for RTL languages).
    Right,
    /// Justified text (stretched to fill width).
    Justified,
}

impl HorizontalAlign {
    /// Convert to cosmic-text Align.
    fn to_cosmic(self) -> cosmic_text::Align {
        match self {
            HorizontalAlign::Left => cosmic_text::Align::Left,
            HorizontalAlign::Center => cosmic_text::Align::Center,
            HorizontalAlign::Right => cosmic_text::Align::Right,
            HorizontalAlign::Justified => cosmic_text::Align::Justified,
        }
    }
}

/// Vertical text alignment within a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VerticalAlign {
    /// Top-aligned text.
    #[default]
    Top,
    /// Center-aligned text (vertically).
    Middle,
    /// Bottom-aligned text.
    Bottom,
}

/// Text wrapping mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum WrapMode {
    /// No wrapping - text extends beyond bounds.
    #[default]
    None,
    /// Wrap at word boundaries.
    Word,
    /// Wrap at character boundaries.
    Character,
    /// Word wrap with character fallback for long words.
    WordOrCharacter,
}

impl WrapMode {
    /// Convert to cosmic-text Wrap.
    fn to_cosmic(self) -> Wrap {
        match self {
            WrapMode::None => Wrap::None,
            WrapMode::Word => Wrap::Word,
            WrapMode::Character => Wrap::Glyph,
            WrapMode::WordOrCharacter => Wrap::WordOrGlyph,
        }
    }
}

/// Options for text layout.
#[derive(Debug, Clone)]
pub struct TextLayoutOptions {
    /// Maximum width for text layout (None = unconstrained).
    pub max_width: Option<f32>,
    /// Maximum height for text layout (None = unconstrained).
    pub max_height: Option<f32>,
    /// Horizontal text alignment.
    pub horizontal_align: HorizontalAlign,
    /// Vertical text alignment.
    pub vertical_align: VerticalAlign,
    /// Text wrapping mode.
    pub wrap: WrapMode,
    /// Line height multiplier (1.0 = normal, 1.5 = 150%).
    pub line_height_multiplier: f32,
    /// Additional paragraph spacing in pixels.
    pub paragraph_spacing: f32,
    /// Whether to truncate with ellipsis when text overflows.
    pub ellipsis: bool,
    /// Custom ellipsis string (defaults to "…").
    pub ellipsis_string: String,
    /// Base text direction for the layout.
    ///
    /// - `TextDirection::Auto` (default): Automatically detect from content.
    /// - `TextDirection::LeftToRight`: Force LTR direction.
    /// - `TextDirection::RightToLeft`: Force RTL direction.
    pub direction: TextDirection,
    /// Left margin indent in pixels.
    /// This shifts the entire text block to the right.
    pub left_indent: f32,
    /// First line indent in pixels (relative to left_indent).
    /// Positive values indent the first line further right.
    /// Negative values create a "hanging indent".
    pub first_line_indent: f32,
}

impl Default for TextLayoutOptions {
    fn default() -> Self {
        Self {
            max_width: None,
            max_height: None,
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            wrap: WrapMode::None,
            line_height_multiplier: 1.2,
            paragraph_spacing: 0.0,
            ellipsis: false,
            ellipsis_string: "…".to_string(),
            direction: TextDirection::Auto,
            left_indent: 0.0,
            first_line_indent: 0.0,
        }
    }
}

impl TextLayoutOptions {
    /// Create new layout options with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum width constraint.
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Set the maximum height constraint.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Set horizontal alignment.
    pub fn horizontal_align(mut self, align: HorizontalAlign) -> Self {
        self.horizontal_align = align;
        self
    }

    /// Set vertical alignment.
    pub fn vertical_align(mut self, align: VerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }

    /// Set text wrapping mode.
    pub fn wrap(mut self, wrap: WrapMode) -> Self {
        self.wrap = wrap;
        self
    }

    /// Set line height multiplier.
    pub fn line_height(mut self, multiplier: f32) -> Self {
        self.line_height_multiplier = multiplier;
        self
    }

    /// Set paragraph spacing in pixels.
    pub fn paragraph_spacing(mut self, spacing: f32) -> Self {
        self.paragraph_spacing = spacing;
        self
    }

    /// Enable ellipsis truncation.
    pub fn with_ellipsis(mut self) -> Self {
        self.ellipsis = true;
        self
    }

    /// Set custom ellipsis string.
    pub fn ellipsis_string(mut self, s: impl Into<String>) -> Self {
        self.ellipsis_string = s.into();
        self.ellipsis = true;
        self
    }

    /// Set the base text direction.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use horizon_lattice_render::text::{TextLayoutOptions, TextDirection};
    ///
    /// // Force RTL direction for Hebrew/Arabic text
    /// let options = TextLayoutOptions::default()
    ///     .direction(TextDirection::RightToLeft);
    ///
    /// // Auto-detect direction from content
    /// let options = TextLayoutOptions::default()
    ///     .direction(TextDirection::Auto);
    /// ```
    pub fn direction(mut self, direction: TextDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Set left-to-right direction.
    pub fn ltr(self) -> Self {
        self.direction(TextDirection::LeftToRight)
    }

    /// Set right-to-left direction.
    pub fn rtl(self) -> Self {
        self.direction(TextDirection::RightToLeft)
    }

    /// Set left margin indent in pixels.
    pub fn left_indent(mut self, indent: f32) -> Self {
        self.left_indent = indent;
        self
    }

    /// Set first line indent in pixels (relative to left_indent).
    pub fn first_line_indent(mut self, indent: f32) -> Self {
        self.first_line_indent = indent;
        self
    }
}

/// A positioned glyph within the text layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutGlyph {
    /// The glyph identifier within the font.
    pub glyph_id: u16,
    /// The font face ID for this glyph.
    pub font_id: FontFaceId,
    /// X position relative to the layout origin.
    pub x: f32,
    /// Y position relative to the layout origin.
    pub y: f32,
    /// Width of the glyph hitbox.
    pub width: f32,
    /// X offset for rendering (subpixel adjustment).
    pub x_offset: f32,
    /// Y offset for rendering (subpixel adjustment).
    pub y_offset: f32,
    /// The byte range in the original text this glyph represents.
    pub cluster: Range<usize>,
    /// The font size this glyph was shaped at.
    pub font_size: f32,
    /// Flags for cache key generation during rasterization.
    pub cache_key_flags: CacheKeyFlags,
    /// The bidirectional level (0 = LTR, 1 = RTL, etc.).
    pub level: u8,
    /// Optional text color for this glyph.
    pub color: Option<[u8; 4]>,
    /// Metadata for identifying inline elements.
    pub metadata: usize,
}

impl LayoutGlyph {
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

    /// Check if this glyph is a placeholder for an inline element.
    pub fn is_inline_element(&self) -> bool {
        self.metadata != 0
    }
}

/// A single line within the text layout.
#[derive(Debug, Clone)]
pub struct LayoutLine {
    /// The glyphs in this line (in visual order).
    pub glyphs: Vec<LayoutGlyph>,
    /// Y offset from the top of the layout to this line's baseline.
    pub baseline_y: f32,
    /// Y offset from the top of the layout to this line's top.
    pub top_y: f32,
    /// Height of this line.
    pub height: f32,
    /// Width of this line's content.
    pub width: f32,
    /// The byte range in the original text that this line covers.
    pub text_range: Range<usize>,
    /// Whether this line ends with a hard break (newline).
    pub is_hard_break: bool,
}

impl LayoutLine {
    /// Check if this line is empty (no glyphs).
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// Get the number of glyphs in this line.
    pub fn glyph_count(&self) -> usize {
        self.glyphs.len()
    }

    /// Find the glyph at a given x position within this line.
    pub fn glyph_at_x(&self, x: f32) -> Option<usize> {
        self.glyphs.iter().position(|g| g.contains_x(x))
    }

    /// Get the text offset at a given x position.
    pub fn offset_at_x(&self, x: f32) -> usize {
        if x <= 0.0 || self.glyphs.is_empty() {
            return self.text_range.start;
        }

        if x >= self.width {
            return self.text_range.end;
        }

        for glyph in &self.glyphs {
            if glyph.contains_x(x) {
                let mid = glyph.x + glyph.width / 2.0;
                if x < mid {
                    return glyph.cluster.start;
                } else {
                    return glyph.cluster.end;
                }
            }
        }

        self.text_range.end
    }

    /// Get the x position for a given text offset.
    pub fn x_for_offset(&self, offset: usize) -> f32 {
        if offset <= self.text_range.start || self.glyphs.is_empty() {
            return 0.0;
        }

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
}

/// A background rectangle for styled text.
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundRect {
    /// X position of the background rectangle.
    pub x: f32,
    /// Y position (top of the text area).
    pub y: f32,
    /// Width of the background rectangle.
    pub width: f32,
    /// Height of the background rectangle.
    pub height: f32,
    /// Background color (RGBA).
    pub color: [u8; 4],
}

impl BackgroundRect {
    /// Create a new background rectangle.
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: [u8; 4]) -> Self {
        Self { x, y, width, height, color }
    }
}

/// A positioned text decoration line for rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct DecorationLine {
    /// X start position.
    pub x_start: f32,
    /// X end position.
    pub x_end: f32,
    /// Y position of the line.
    pub y: f32,
    /// Line thickness in pixels.
    pub thickness: f32,
    /// Decoration color (RGBA).
    pub color: [u8; 4],
    /// The decoration style.
    pub style: super::TextDecorationStyle,
    /// The decoration type.
    pub decoration_type: super::TextDecorationType,
}

impl DecorationLine {
    /// Get the width of the decoration line.
    pub fn width(&self) -> f32 {
        self.x_end - self.x_start
    }
}

/// Information about a span for background and decoration extraction.
#[derive(Debug)]
struct SpanInfo {
    byte_range: std::ops::Range<usize>,
    background_color: Option<[u8; 4]>,
    decorations: Vec<super::TextDecoration>,
    text_color: Option<[u8; 4]>,
}

/// Metadata identifier for inline elements.
const INLINE_ELEMENT_BASE: usize = 0x1000_0000;

/// An inline element that can be embedded in text.
#[derive(Debug, Clone)]
pub struct InlineElement {
    /// Unique identifier for this inline element.
    pub id: usize,
    /// Width of the inline element in pixels.
    pub width: f32,
    /// Height of the inline element in pixels.
    pub height: f32,
    /// Vertical alignment of the element relative to text.
    pub vertical_align: InlineVerticalAlign,
}

/// Vertical alignment for inline elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InlineVerticalAlign {
    /// Align with the text baseline.
    #[default]
    Baseline,
    /// Align with the top of the line.
    Top,
    /// Center vertically within the line.
    Middle,
    /// Align with the bottom of the line.
    Bottom,
}

impl InlineElement {
    /// Create a new inline element with the given dimensions.
    pub fn new(id: usize, width: f32, height: f32) -> Self {
        Self {
            id,
            width,
            height,
            vertical_align: InlineVerticalAlign::Baseline,
        }
    }

    /// Set the vertical alignment.
    pub fn with_vertical_align(mut self, align: InlineVerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }
}

/// A text span with styling for rich text.
#[derive(Debug, Clone)]
pub struct TextSpan<'a> {
    /// The text content of this span.
    pub text: &'a str,
    /// Optional font override for this span.
    pub font: Option<Font>,
    /// Optional color override for this span (RGBA).
    pub color: Option<[u8; 4]>,
    /// Optional background color for this span (RGBA).
    pub background_color: Option<[u8; 4]>,
    /// Text decorations (underline, strikethrough, overline).
    pub decorations: Vec<TextDecoration>,
}

impl<'a> TextSpan<'a> {
    /// Create a new text span.
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            font: None,
            color: None,
            background_color: None,
            decorations: Vec::new(),
        }
    }

    /// Set the font for this span.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Set the color for this span (RGBA).
    pub fn with_color(mut self, color: [u8; 4]) -> Self {
        self.color = Some(color);
        self
    }

    /// Create a bold span.
    pub fn bold(mut self, base_font: &Font) -> Self {
        self.font = Some(base_font.with_weight(FontWeight::BOLD));
        self
    }

    /// Create an italic span.
    pub fn italic(mut self, base_font: &Font) -> Self {
        self.font = Some(base_font.with_style(FontStyle::Italic));
        self
    }

    /// Set the background color for this span (RGBA).
    pub fn with_background_color(mut self, color: [u8; 4]) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Add a text decoration to this span.
    pub fn with_decoration(mut self, decoration: TextDecoration) -> Self {
        self.decorations.push(decoration);
        self
    }

    /// Add an underline decoration.
    pub fn with_underline(mut self) -> Self {
        self.decorations.push(TextDecoration::underline());
        self
    }

    /// Add a strikethrough decoration.
    pub fn with_strikethrough(mut self) -> Self {
        self.decorations.push(TextDecoration::strikethrough());
        self
    }

    /// Add an overline decoration.
    pub fn with_overline(mut self) -> Self {
        self.decorations.push(TextDecoration::overline());
        self
    }

    /// Add a wavy underline (often used for spell check errors).
    pub fn with_wavy_underline(mut self) -> Self {
        self.decorations.push(TextDecoration::wavy_underline());
        self
    }
}

/// A complete text layout with positioned lines and glyphs.
#[derive(Debug, Clone)]
pub struct TextLayout {
    /// The original text that was laid out.
    text: String,
    /// The laid out lines.
    lines: Vec<LayoutLine>,
    /// Total width of the layout.
    width: f32,
    /// Total height of the layout.
    height: f32,
    /// The layout options used.
    options: TextLayoutOptions,
    /// Whether the text was truncated.
    is_truncated: bool,
    /// Inline elements and their positions.
    inline_elements: Vec<(InlineElement, f32, f32)>,
    /// Background rectangles for styled text.
    background_rects: Vec<BackgroundRect>,
    /// Text decoration lines (underline, strikethrough, overline).
    decoration_lines: Vec<DecorationLine>,
    /// The resolved base direction after layout.
    resolved_direction: TextDirection,
}

impl TextLayout {
    /// Create a new text layout with default options.
    pub fn new(font_system: &mut FontSystem, text: &str, font: &Font) -> Self {
        Self::with_options(font_system, text, font, TextLayoutOptions::default())
    }

    /// Create a text layout with custom options.
    pub fn with_options(
        font_system: &mut FontSystem,
        text: &str,
        font: &Font,
        options: TextLayoutOptions,
    ) -> Self {
        // Resolve direction from text content if Auto
        let resolved_direction = options.direction.resolve(text);

        let mut layout = Self {
            text: text.to_string(),
            lines: Vec::new(),
            width: 0.0,
            height: 0.0,
            options,
            is_truncated: false,
            inline_elements: Vec::new(),
            background_rects: Vec::new(),
            decoration_lines: Vec::new(),
            resolved_direction,
        };

        layout.layout_text(font_system, font);
        layout
    }

    /// Create a rich text layout with multiple styled spans.
    pub fn rich_text(
        font_system: &mut FontSystem,
        spans: &[TextSpan<'_>],
        default_font: &Font,
        options: TextLayoutOptions,
    ) -> Self {
        let text: String = spans.iter().map(|s| s.text).collect();
        let resolved_direction = options.direction.resolve(&text);
        let mut layout = Self {
            text,
            lines: Vec::new(),
            width: 0.0,
            height: 0.0,
            options,
            is_truncated: false,
            inline_elements: Vec::new(),
            background_rects: Vec::new(),
            decoration_lines: Vec::new(),
            resolved_direction,
        };

        layout.layout_rich_text(font_system, spans, default_font);
        layout
    }

    /// Create a layout with inline elements.
    pub fn with_inline_elements(
        font_system: &mut FontSystem,
        text: &str,
        font: &Font,
        options: TextLayoutOptions,
        elements: &[(usize, InlineElement)],
    ) -> Self {
        let resolved_direction = options.direction.resolve(text);
        let mut layout = Self {
            text: text.to_string(),
            lines: Vec::new(),
            width: 0.0,
            height: 0.0,
            options,
            is_truncated: false,
            inline_elements: Vec::new(),
            background_rects: Vec::new(),
            decoration_lines: Vec::new(),
            resolved_direction,
        };

        layout.layout_with_inline_elements(font_system, font, elements);
        layout
    }

    /// Get the original text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the laid out lines.
    pub fn lines(&self) -> &[LayoutLine] {
        &self.lines
    }

    /// Get the total width of the layout.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Get the total height of the layout.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// Get the number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Check if the text was truncated.
    pub fn is_truncated(&self) -> bool {
        self.is_truncated
    }

    /// Get the inline elements with their positions.
    pub fn inline_elements(&self) -> &[(InlineElement, f32, f32)] {
        &self.inline_elements
    }

    /// Get the background rectangles for styled text.
    pub fn background_rects(&self) -> &[BackgroundRect] {
        &self.background_rects
    }

    /// Get the decoration lines (underline, strikethrough, overline).
    pub fn decoration_lines(&self) -> &[DecorationLine] {
        &self.decoration_lines
    }

    /// Get the resolved base direction of the layout.
    ///
    /// This is either the explicit direction set in options, or the
    /// auto-detected direction from the text content.
    pub fn direction(&self) -> TextDirection {
        self.resolved_direction
    }

    /// Check if the layout's base direction is right-to-left.
    pub fn is_rtl(&self) -> bool {
        self.resolved_direction.is_rtl()
    }

    /// Check if the layout contains any right-to-left glyphs.
    ///
    /// This can be true even for LTR base direction layouts that
    /// contain RTL text segments (e.g., English with Arabic words).
    pub fn has_rtl_content(&self) -> bool {
        self.glyphs().any(|g| g.is_rtl())
    }

    /// Get all glyphs across all lines.
    pub fn glyphs(&self) -> impl Iterator<Item = &LayoutGlyph> {
        self.lines.iter().flat_map(|line| line.glyphs.iter())
    }

    /// Measure the text without full layout (faster for simple cases).
    pub fn measure(font_system: &mut FontSystem, text: &str, font: &Font) -> (f32, f32) {
        let layout = Self::new(font_system, text, font);
        (layout.width, layout.height)
    }

    /// Measure text with width constraint.
    pub fn measure_with_width(
        font_system: &mut FontSystem,
        text: &str,
        font: &Font,
        max_width: f32,
    ) -> (f32, f32) {
        let options = TextLayoutOptions::default()
            .max_width(max_width)
            .wrap(WrapMode::Word);
        let layout = Self::with_options(font_system, text, font, options);
        (layout.width, layout.height)
    }

    /// Find the line containing a given y position.
    pub fn line_at_y(&self, y: f32) -> Option<usize> {
        for (i, line) in self.lines.iter().enumerate() {
            if y >= line.top_y && y < line.top_y + line.height {
                return Some(i);
            }
        }
        if y >= self.height && !self.lines.is_empty() {
            return Some(self.lines.len() - 1);
        }
        None
    }

    /// Find the text offset at a given (x, y) position.
    pub fn offset_at_point(&self, x: f32, y: f32) -> usize {
        if let Some(line_idx) = self.line_at_y(y) {
            let line = &self.lines[line_idx];
            // Adjust x for alignment offset
            let adjusted_x = x - self.alignment_offset(line);
            line.offset_at_x(adjusted_x)
        } else if y < 0.0 {
            0
        } else {
            self.text.len()
        }
    }

    /// Get the position (x, y) for a given text offset.
    pub fn point_for_offset(&self, offset: usize) -> (f32, f32) {
        for line in &self.lines {
            if line.text_range.contains(&offset) || offset == line.text_range.start {
                let x = line.x_for_offset(offset) + self.alignment_offset(line);
                return (x, line.baseline_y);
            }
        }
        // After the last character
        if let Some(line) = self.lines.last() {
            (line.width + self.alignment_offset(line), line.baseline_y)
        } else {
            (0.0, 0.0)
        }
    }

    /// Calculate horizontal alignment offset for a line.
    fn alignment_offset(&self, line: &LayoutLine) -> f32 {
        let container_width = self.options.max_width.unwrap_or(self.width);
        match self.options.horizontal_align {
            HorizontalAlign::Left => 0.0,
            HorizontalAlign::Center => (container_width - line.width) / 2.0,
            HorizontalAlign::Right => container_width - line.width,
            HorizontalAlign::Justified => 0.0, // Justification is handled during layout
        }
    }

    /// Calculate vertical alignment offset.
    pub fn vertical_offset(&self) -> f32 {
        let container_height = self.options.max_height.unwrap_or(self.height);
        match self.options.vertical_align {
            VerticalAlign::Top => 0.0,
            VerticalAlign::Middle => (container_height - self.height) / 2.0,
            VerticalAlign::Bottom => container_height - self.height,
        }
    }

    /// Get the effective horizontal alignment, respecting text direction.
    ///
    /// For RTL text with default (Left) alignment, this returns Right.
    /// For LTR text with default (Left) alignment, this returns Left.
    /// Explicit alignments (Center, Right, Justified) are preserved.
    pub fn effective_alignment(&self) -> HorizontalAlign {
        match self.options.horizontal_align {
            // Default alignment is direction-aware
            HorizontalAlign::Left if self.resolved_direction.is_rtl() => HorizontalAlign::Right,
            HorizontalAlign::Right if self.resolved_direction.is_rtl() => HorizontalAlign::Left,
            other => other,
        }
    }

    /// Perform the actual text layout.
    fn layout_text(&mut self, font_system: &mut FontSystem, font: &Font) {
        if self.text.is_empty() {
            return;
        }

        let font_size = font.size();
        let line_height = font_size * self.options.line_height_multiplier;
        let metrics = Metrics::new(font_size, line_height);

        let mut buffer = Buffer::new(font_system.inner_mut(), metrics);

        // Set wrap mode
        buffer.set_wrap(font_system.inner_mut(), self.options.wrap.to_cosmic());

        // Set size constraints
        buffer.set_size(
            font_system.inner_mut(),
            self.options.max_width,
            self.options.max_height,
        );

        // Build attributes from font
        let attrs = font.to_attrs();

        // Set alignment and text
        buffer.set_text(
            font_system.inner_mut(),
            &self.text,
            attrs,
            Shaping::Advanced,
        );

        // Get effective alignment (respects RTL direction)
        let effective_align = self.effective_alignment();

        // Set alignment for each line
        for line in buffer.lines.iter_mut() {
            line.set_align(Some(effective_align.to_cosmic()));
        }

        // Re-shape after alignment change
        buffer.shape_until_scroll(font_system.inner_mut(), false);

        // Handle ellipsis truncation
        if self.options.ellipsis {
            self.apply_ellipsis(font_system, &mut buffer, font);
        }

        // Extract layout data
        self.extract_layout(&buffer);
    }

    /// Layout rich text with multiple styled spans.
    fn layout_rich_text(
        &mut self,
        font_system: &mut FontSystem,
        spans: &[TextSpan<'_>],
        default_font: &Font,
    ) {
        if spans.is_empty() {
            return;
        }

        let font_size = default_font.size();
        let line_height = font_size * self.options.line_height_multiplier;
        let metrics = Metrics::new(font_size, line_height);

        let mut buffer = Buffer::new(font_system.inner_mut(), metrics);

        // Set wrap mode
        buffer.set_wrap(font_system.inner_mut(), self.options.wrap.to_cosmic());

        // Set size constraints
        buffer.set_size(
            font_system.inner_mut(),
            self.options.max_width,
            self.options.max_height,
        );

        // Build span info for later background/decoration extraction
        // Track byte ranges for each span
        let mut span_info: Vec<SpanInfo> = Vec::new();
        let mut byte_offset = 0;
        for span in spans {
            let span_len = span.text.len();
            span_info.push(SpanInfo {
                byte_range: byte_offset..(byte_offset + span_len),
                background_color: span.background_color,
                decorations: span.decorations.clone(),
                text_color: span.color,
            });
            byte_offset += span_len;
        }

        // Build attributed spans for cosmic-text
        let cosmic_spans: Vec<(&str, Attrs<'_>)> = spans
            .iter()
            .map(|span| {
                let attrs = if let Some(ref font) = span.font {
                    let mut a = font.to_attrs();
                    if let Some(color) = span.color {
                        a = a.color(cosmic_text::Color::rgba(
                            color[0], color[1], color[2], color[3],
                        ));
                    }
                    a
                } else {
                    let mut a = default_font.to_attrs();
                    if let Some(color) = span.color {
                        a = a.color(cosmic_text::Color::rgba(
                            color[0], color[1], color[2], color[3],
                        ));
                    }
                    a
                };
                (span.text, attrs)
            })
            .collect();

        // Set rich text
        buffer.set_rich_text(
            font_system.inner_mut(),
            cosmic_spans,
            default_font.to_attrs(),
            Shaping::Advanced,
        );

        // Set alignment for each line
        for line in buffer.lines.iter_mut() {
            line.set_align(Some(self.effective_alignment().to_cosmic()));
        }

        // Re-shape after setting rich text
        buffer.shape_until_scroll(font_system.inner_mut(), false);

        // Handle ellipsis truncation
        if self.options.ellipsis {
            self.apply_ellipsis(font_system, &mut buffer, default_font);
        }

        // Extract layout data
        self.extract_layout(&buffer);

        // Extract backgrounds and decorations from span info
        self.extract_span_styling(&span_info, default_font.size());
    }

    /// Extract background rectangles and decoration lines from span info.
    fn extract_span_styling(&mut self, span_info: &[SpanInfo], font_size: f32) {
        use super::TextDecorationType;

        // Default decoration thickness based on font size
        let base_thickness = (font_size / 12.0).max(1.0);

        for info in span_info {
            // Skip spans without any styling
            if info.background_color.is_none() && info.decorations.is_empty() {
                continue;
            }

            // Find glyphs that belong to this span
            for line in &self.lines {
                let align_offset = self.alignment_offset(line);

                // Find the range of glyphs in this line that belong to this span
                let mut span_start_x: Option<f32> = None;
                let mut span_end_x: f32 = 0.0;
                let mut found_glyph = false;

                for glyph in &line.glyphs {
                    // Check if this glyph's cluster overlaps with the span's byte range
                    let glyph_in_span = glyph.cluster.start < info.byte_range.end
                        && glyph.cluster.end > info.byte_range.start;

                    if glyph_in_span {
                        if span_start_x.is_none() {
                            span_start_x = Some(glyph.x);
                        }
                        span_end_x = glyph.x + glyph.width;
                        found_glyph = true;
                    }
                }

                if !found_glyph {
                    continue;
                }

                let start_x = span_start_x.unwrap() + align_offset;
                let end_x = span_end_x + align_offset;
                let width = end_x - start_x;

                if width <= 0.0 {
                    continue;
                }

                // Add background rectangle
                if let Some(bg_color) = info.background_color {
                    self.background_rects.push(BackgroundRect::new(
                        start_x,
                        line.top_y,
                        width,
                        line.height,
                        bg_color,
                    ));
                }

                // Add decorations
                for decoration in &info.decorations {
                    let thickness = base_thickness * decoration.thickness;

                    // Determine Y position based on decoration type
                    let y = match decoration.decoration_type {
                        TextDecorationType::Underline => {
                            // Position below baseline
                            line.baseline_y + (font_size * 0.15)
                        }
                        TextDecorationType::Strikethrough => {
                            // Position at middle of x-height (roughly 0.3 from baseline)
                            line.baseline_y - (font_size * 0.3)
                        }
                        TextDecorationType::Overline => {
                            // Position above the text
                            line.top_y + (thickness / 2.0)
                        }
                    };

                    // Determine color (use decoration color, or fall back to text color, or default to black)
                    let color = decoration.color
                        .or(info.text_color)
                        .unwrap_or([0, 0, 0, 255]);

                    self.decoration_lines.push(DecorationLine {
                        x_start: start_x,
                        x_end: end_x,
                        y,
                        thickness,
                        color,
                        style: decoration.style,
                        decoration_type: decoration.decoration_type,
                    });
                }
            }
        }
    }

    /// Layout text with inline elements.
    fn layout_with_inline_elements(
        &mut self,
        font_system: &mut FontSystem,
        font: &Font,
        elements: &[(usize, InlineElement)],
    ) {
        if self.text.is_empty() && elements.is_empty() {
            return;
        }

        let font_size = font.size();
        let line_height = font_size * self.options.line_height_multiplier;
        let metrics = Metrics::new(font_size, line_height);

        let mut buffer = Buffer::new(font_system.inner_mut(), metrics);

        // Set wrap mode
        buffer.set_wrap(font_system.inner_mut(), self.options.wrap.to_cosmic());

        // Set size constraints
        buffer.set_size(
            font_system.inner_mut(),
            self.options.max_width,
            self.options.max_height,
        );

        // Build text with placeholder characters for inline elements
        let mut modified_text = self.text.clone();
        let mut offset_adjustment = 0i64;

        // Sort elements by position
        let mut sorted_elements: Vec<_> = elements.iter().collect();
        sorted_elements.sort_by_key(|(pos, _)| *pos);

        // Insert placeholder characters and build spans
        for (pos, _element) in &sorted_elements {
            let adjusted_pos = (*pos as i64 + offset_adjustment) as usize;
            modified_text.insert(adjusted_pos, '\u{FFFC}'); // Object replacement character
            offset_adjustment += 1;
        }

        // Create spans with metadata for inline elements
        let mut current_pos = 0;
        let mut spans: Vec<(&str, Attrs<'_>)> = Vec::new();
        let mut element_idx = 0;

        let attrs = font.to_attrs();

        for (original_pos, element) in &sorted_elements {
            let adjusted_pos = (*original_pos as i64 + element_idx as i64) as usize;

            // Add text before this element
            if current_pos < adjusted_pos {
                let text_slice = &modified_text[current_pos..adjusted_pos];
                spans.push((text_slice, attrs.clone()));
            }

            // Add placeholder with metadata
            let placeholder_attrs = attrs.clone().metadata(INLINE_ELEMENT_BASE + element.id);
            let placeholder_slice = &modified_text[adjusted_pos..adjusted_pos + 3]; // UTF-8 length of U+FFFC
            spans.push((placeholder_slice, placeholder_attrs));

            current_pos = adjusted_pos + 3;
            element_idx += 1;
        }

        // Add remaining text
        if current_pos < modified_text.len() {
            spans.push((&modified_text[current_pos..], attrs));
        }

        // Set rich text with metadata
        buffer.set_rich_text(
            font_system.inner_mut(),
            spans,
            font.to_attrs(),
            Shaping::Advanced,
        );

        // Set alignment for each line
        for line in buffer.lines.iter_mut() {
            line.set_align(Some(self.effective_alignment().to_cosmic()));
        }

        // Re-shape
        buffer.shape_until_scroll(font_system.inner_mut(), false);

        // Extract layout data
        self.extract_layout(&buffer);

        // Post-process to find inline element positions
        self.locate_inline_elements(elements);
    }

    /// Apply ellipsis truncation to the buffer.
    fn apply_ellipsis(&mut self, font_system: &mut FontSystem, buffer: &mut Buffer, font: &Font) {
        let max_width = match self.options.max_width {
            Some(w) => w,
            None => return, // No truncation needed without width constraint
        };

        // Check if any line exceeds max width
        let mut needs_truncation = false;
        for run in buffer.layout_runs() {
            if run.line_w > max_width {
                needs_truncation = true;
                break;
            }
        }

        if !needs_truncation {
            return;
        }

        self.is_truncated = true;

        // Measure ellipsis width
        let ellipsis_metrics = Metrics::new(font.size(), font.size() * 1.2);
        let mut ellipsis_buffer = Buffer::new(font_system.inner_mut(), ellipsis_metrics);
        ellipsis_buffer.set_text(
            font_system.inner_mut(),
            &self.options.ellipsis_string,
            font.to_attrs(),
            Shaping::Advanced,
        );
        ellipsis_buffer.shape_until_scroll(font_system.inner_mut(), false);

        let ellipsis_width: f32 = ellipsis_buffer
            .layout_runs()
            .map(|r| r.line_w)
            .sum();

        // Truncate text to fit ellipsis
        let target_width = max_width - ellipsis_width;
        if target_width <= 0.0 {
            // Can't fit any text, just show ellipsis
            self.text = self.options.ellipsis_string.clone();
            return;
        }

        // Find truncation point
        let mut truncate_at = 0;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                if glyph.x + glyph.w > target_width {
                    truncate_at = glyph.start;
                    break;
                }
                truncate_at = glyph.end;
            }
        }

        // Update text with ellipsis
        if truncate_at < self.text.len() {
            self.text.truncate(truncate_at);
            self.text.push_str(&self.options.ellipsis_string);

            // Re-layout with truncated text
            buffer.set_text(
                font_system.inner_mut(),
                &self.text,
                font.to_attrs(),
                Shaping::Advanced,
            );
            buffer.shape_until_scroll(font_system.inner_mut(), false);
        }
    }

    /// Extract layout data from the cosmic-text buffer.
    fn extract_layout(&mut self, buffer: &Buffer) {
        self.lines.clear();
        self.width = 0.0;
        self.height = 0.0;

        let mut current_line_i = usize::MAX;
        let mut current_line = LayoutLine {
            glyphs: Vec::new(),
            baseline_y: 0.0,
            top_y: 0.0,
            height: 0.0,
            width: 0.0,
            text_range: 0..0,
            is_hard_break: false,
        };

        for run in buffer.layout_runs() {
            // Check if this is a new line
            if run.line_i != current_line_i {
                // Save previous line if it exists
                if current_line_i != usize::MAX {
                    self.lines.push(current_line.clone());
                }

                current_line_i = run.line_i;
                current_line = LayoutLine {
                    glyphs: Vec::new(),
                    baseline_y: run.line_y,
                    top_y: run.line_top,
                    height: run.line_height,
                    width: 0.0,
                    text_range: usize::MAX..usize::MAX,
                    is_hard_break: false,
                };

                self.height = self.height.max(run.line_top + run.line_height);
            }

            current_line.width = current_line.width.max(run.line_w);
            self.width = self.width.max(run.line_w);

            for layout_glyph in run.glyphs.iter() {
                let glyph = LayoutGlyph {
                    glyph_id: layout_glyph.glyph_id,
                    font_id: layout_glyph.font_id,
                    x: layout_glyph.x,
                    y: run.line_y,
                    width: layout_glyph.w,
                    x_offset: layout_glyph.x_offset,
                    y_offset: layout_glyph.y_offset,
                    cluster: layout_glyph.start..layout_glyph.end,
                    font_size: layout_glyph.font_size,
                    cache_key_flags: layout_glyph.cache_key_flags,
                    level: layout_glyph.level.into(),
                    color: layout_glyph.color_opt.map(|c| [c.r(), c.g(), c.b(), c.a()]),
                    metadata: layout_glyph.metadata,
                };

                // Update text range
                if current_line.text_range.start == usize::MAX {
                    // First glyph on this line - initialize both start and end
                    current_line.text_range.start = layout_glyph.start;
                    current_line.text_range.end = layout_glyph.end;
                } else {
                    current_line.text_range.end = current_line.text_range.end.max(layout_glyph.end);
                }

                current_line.glyphs.push(glyph);
            }
        }

        // Don't forget the last line
        if current_line_i != usize::MAX {
            self.lines.push(current_line);
        }

        // Fix empty text range for empty lines
        for line in &mut self.lines {
            if line.text_range.start == usize::MAX {
                line.text_range = 0..0;
            }
        }

        // Determine paragraph boundaries and set is_hard_break
        for line in &mut self.lines {
            if !line.text_range.is_empty() && line.text_range.end <= self.text.len() {
                // Check if line ends with newline
                line.is_hard_break = self.text[..line.text_range.end].ends_with('\n');
            }
        }

        // Apply indentation if configured
        if self.options.left_indent != 0.0 || self.options.first_line_indent != 0.0 {
            self.apply_indentation();
        }

        // Apply paragraph spacing if configured
        if self.options.paragraph_spacing != 0.0 {
            self.apply_paragraph_spacing();
        }
    }

    /// Apply left and first-line indentation to the layout.
    fn apply_indentation(&mut self) {
        let left_indent = self.options.left_indent;
        let first_line_indent = self.options.first_line_indent;

        // Track whether the next line is the first line of a paragraph
        let mut is_first_line_of_paragraph = true;

        for line in &mut self.lines {
            // Calculate the indent for this line
            let indent = if is_first_line_of_paragraph {
                left_indent + first_line_indent
            } else {
                left_indent
            };

            // Apply indent to all glyphs
            for glyph in &mut line.glyphs {
                glyph.x += indent;
            }

            // Update line width to include indent
            if !line.glyphs.is_empty() {
                line.width += indent;
            }

            // Update overall layout width
            self.width = self.width.max(line.width);

            // The next line is a first line of paragraph if this line has a hard break
            is_first_line_of_paragraph = line.is_hard_break;
        }
    }

    /// Apply paragraph spacing to the layout.
    ///
    /// This adds extra vertical space between paragraphs by shifting
    /// the y positions of lines that start a new paragraph.
    fn apply_paragraph_spacing(&mut self) {
        let paragraph_spacing = self.options.paragraph_spacing;
        if paragraph_spacing == 0.0 || self.lines.is_empty() {
            return;
        }

        // Track cumulative offset and whether previous line was a paragraph break
        let mut y_offset = 0.0;
        let mut previous_was_hard_break = false;

        for (i, line) in self.lines.iter_mut().enumerate() {
            // Add paragraph spacing before lines that start a new paragraph
            // (except the very first line)
            if i > 0 && previous_was_hard_break {
                y_offset += paragraph_spacing;
            }

            // Apply accumulated offset
            if y_offset != 0.0 {
                line.baseline_y += y_offset;
                line.top_y += y_offset;
                for glyph in &mut line.glyphs {
                    glyph.y += y_offset;
                }
            }

            previous_was_hard_break = line.is_hard_break;
        }

        // Update overall height
        if let Some(last_line) = self.lines.last() {
            self.height = last_line.top_y + last_line.height;
        }
    }

    /// Locate inline element positions after layout.
    fn locate_inline_elements(&mut self, elements: &[(usize, InlineElement)]) {
        for line in &self.lines {
            for glyph in &line.glyphs {
                if glyph.metadata >= INLINE_ELEMENT_BASE {
                    let element_id = glyph.metadata - INLINE_ELEMENT_BASE;
                    if let Some((_, element)) = elements.iter().find(|(_, e)| e.id == element_id) {
                        let x = glyph.x;
                        let y = match element.vertical_align {
                            InlineVerticalAlign::Baseline => line.baseline_y - element.height,
                            InlineVerticalAlign::Top => line.top_y,
                            InlineVerticalAlign::Middle => {
                                line.top_y + (line.height - element.height) / 2.0
                            }
                            InlineVerticalAlign::Bottom => line.top_y + line.height - element.height,
                        };
                        self.inline_elements.push((element.clone(), x, y));
                    }
                }
            }
        }
    }
}

impl Default for TextLayout {
    fn default() -> Self {
        Self {
            text: String::new(),
            lines: Vec::new(),
            width: 0.0,
            height: 0.0,
            options: TextLayoutOptions::default(),
            is_truncated: false,
            inline_elements: Vec::new(),
            background_rects: Vec::new(),
            decoration_lines: Vec::new(),
            resolved_direction: TextDirection::LeftToRight,
        }
    }
}

/// A rectangle representing a selection region in text.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionRect {
    /// X position of the selection rectangle.
    pub x: f32,
    /// Y position of the selection rectangle (top of line).
    pub y: f32,
    /// Width of the selection rectangle.
    pub width: f32,
    /// Height of the selection rectangle.
    pub height: f32,
}

impl SelectionRect {
    /// Create a new selection rectangle.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Check if a point is within this rectangle.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Get the right edge x position.
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// Get the bottom edge y position.
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

// =============================================================================
// Text Editing Support
// =============================================================================

impl TextLayout {
    // -------------------------------------------------------------------------
    // Word Boundary Detection
    // -------------------------------------------------------------------------

    /// Find the start of the word at or before the given byte offset.
    ///
    /// Returns the byte offset of the word boundary. If the offset is in the
    /// middle of a word, returns the start of that word. If the offset is at
    /// whitespace or punctuation, returns the start of the previous word.
    pub fn word_boundary_before(&self, offset: usize) -> usize {
        if offset == 0 || self.text.is_empty() {
            return 0;
        }

        let offset = offset.min(self.text.len());
        let mut last_word_start = 0;
        let mut current_word_start = None;

        for (idx, word) in self.text.split_word_bound_indices() {
            let end = idx + word.len();

            // Track starts of actual words (alphanumeric content)
            if word.chars().any(|c| c.is_alphanumeric()) {
                if idx <= offset && offset <= end {
                    // We're inside or at the start of this word
                    current_word_start = Some(idx);
                } else if idx < offset {
                    // This word is before our offset
                    last_word_start = idx;
                }
            }
        }

        // If we're in a word, return its start; otherwise return last word start
        current_word_start.unwrap_or(last_word_start)
    }

    /// Find the end of the word at or after the given byte offset.
    ///
    /// Returns the byte offset of the word boundary. If the offset is in the
    /// middle of a word, returns the end of that word. If the offset is at
    /// whitespace or punctuation, returns the end of the next word.
    pub fn word_boundary_after(&self, offset: usize) -> usize {
        if offset >= self.text.len() || self.text.is_empty() {
            return self.text.len();
        }

        for (idx, word) in self.text.split_word_bound_indices() {
            let end = idx + word.len();
            if end > offset && word.chars().any(|c| c.is_alphanumeric()) {
                return end;
            }
        }

        self.text.len()
    }

    /// Get the word range containing the given byte offset.
    ///
    /// Returns the byte range of the word. If the offset is at whitespace
    /// or punctuation, returns an empty range at that position.
    pub fn word_at_offset(&self, offset: usize) -> Range<usize> {
        if self.text.is_empty() {
            return 0..0;
        }

        let offset = offset.min(self.text.len());

        for (idx, word) in self.text.split_word_bound_indices() {
            let end = idx + word.len();
            if idx <= offset && offset < end {
                // Only return range for actual words
                if word.chars().any(|c| c.is_alphanumeric()) {
                    return idx..end;
                }
                // For non-word segments, return empty range
                return offset..offset;
            }
        }

        self.text.len()..self.text.len()
    }

    // -------------------------------------------------------------------------
    // Selection Rendering
    // -------------------------------------------------------------------------

    /// Calculate selection rectangles for the given byte range.
    ///
    /// Returns a vector of rectangles that, when rendered, highlight the
    /// selected text. For multi-line selections, multiple rectangles are
    /// returned (one per line).
    ///
    /// The rectangles are positioned relative to the layout origin and
    /// account for text alignment.
    pub fn selection_rects(&self, start: usize, end: usize) -> Vec<SelectionRect> {
        if start >= end || self.lines.is_empty() {
            return Vec::new();
        }

        let start = start.min(self.text.len());
        let end = end.min(self.text.len());

        let mut rects = Vec::new();

        for line in &self.lines {
            // Skip lines that don't overlap with selection
            if line.text_range.end <= start || line.text_range.start >= end {
                continue;
            }

            // Calculate the portion of this line that's selected
            let line_start = start.max(line.text_range.start);
            let line_end = end.min(line.text_range.end);

            let align_offset = self.alignment_offset(line);

            // Get x positions for selection bounds
            let x_start = line.x_for_offset(line_start) + align_offset;
            let x_end = if line_end >= line.text_range.end {
                // Selection extends to end of line
                line.width + align_offset
            } else {
                line.x_for_offset(line_end) + align_offset
            };

            let width = (x_end - x_start).max(0.0);

            // Use a minimum width for empty selections at line boundaries
            let width = if width < 1.0 && line_start < line_end {
                // At least show something for selections of whitespace/newlines
                4.0
            } else {
                width
            };

            rects.push(SelectionRect::new(x_start, line.top_y, width, line.height));
        }

        rects
    }

    // -------------------------------------------------------------------------
    // Cursor Navigation
    // -------------------------------------------------------------------------

    /// Move the cursor left by one grapheme cluster.
    ///
    /// Returns the new byte offset. If already at the start, returns 0.
    pub fn move_cursor_left(&self, offset: usize) -> usize {
        if offset == 0 || self.text.is_empty() {
            return 0;
        }

        let offset = offset.min(self.text.len());

        // Find the grapheme cluster before the current offset
        let mut prev_offset = 0;
        for (idx, _) in self.text.grapheme_indices(true) {
            if idx >= offset {
                break;
            }
            prev_offset = idx;
        }

        prev_offset
    }

    /// Move the cursor right by one grapheme cluster.
    ///
    /// Returns the new byte offset. If already at the end, returns text length.
    pub fn move_cursor_right(&self, offset: usize) -> usize {
        if offset >= self.text.len() || self.text.is_empty() {
            return self.text.len();
        }

        // Find the next grapheme cluster after the current offset
        for (idx, grapheme) in self.text.grapheme_indices(true) {
            if idx >= offset {
                return idx + grapheme.len();
            }
        }

        self.text.len()
    }

    /// Move the cursor left by one word.
    ///
    /// Skips over whitespace and punctuation to find the start of the
    /// previous word. Returns 0 if at the start.
    pub fn move_cursor_word_left(&self, offset: usize) -> usize {
        if offset == 0 || self.text.is_empty() {
            return 0;
        }

        let offset = offset.min(self.text.len());

        // First, skip any whitespace/punctuation to the left
        let mut current = offset;
        while current > 0 {
            let prev = self.move_cursor_left(current);
            let char_at_prev = self.text[prev..current].chars().next();
            if let Some(c) = char_at_prev {
                if c.is_alphanumeric() {
                    break;
                }
            }
            current = prev;
        }

        // Then find the start of the word
        self.word_boundary_before(current)
    }

    /// Move the cursor right by one word.
    ///
    /// Skips over whitespace and punctuation to find the end of the
    /// next word. Returns text length if at the end.
    pub fn move_cursor_word_right(&self, offset: usize) -> usize {
        if offset >= self.text.len() || self.text.is_empty() {
            return self.text.len();
        }

        // First, skip any whitespace/punctuation to the right
        let mut current = offset;
        while current < self.text.len() {
            let char_at = self.text[current..].chars().next();
            if let Some(c) = char_at {
                if c.is_alphanumeric() {
                    break;
                }
            }
            current = self.move_cursor_right(current);
        }

        // Then find the end of the word
        self.word_boundary_after(current)
    }

    /// Move the cursor to the start of the current line.
    ///
    /// Returns the byte offset of the first character on the line
    /// containing the given offset.
    pub fn move_cursor_to_line_start(&self, offset: usize) -> usize {
        if self.lines.is_empty() {
            return 0;
        }

        for line in &self.lines {
            if line.text_range.contains(&offset)
                || offset == line.text_range.start
                || (offset == line.text_range.end && line.is_hard_break)
            {
                return line.text_range.start;
            }
        }

        // If past the end, return start of last line
        if let Some(line) = self.lines.last() {
            return line.text_range.start;
        }

        0
    }

    /// Move the cursor to the end of the current line.
    ///
    /// Returns the byte offset of the last character on the line
    /// containing the given offset (before any trailing newline).
    pub fn move_cursor_to_line_end(&self, offset: usize) -> usize {
        if self.lines.is_empty() {
            return self.text.len();
        }

        for line in &self.lines {
            if line.text_range.contains(&offset)
                || offset == line.text_range.start
                || (offset == line.text_range.end && line.is_hard_break)
            {
                // Return end of line content (excluding newline if present)
                let mut end = line.text_range.end;
                if end > 0 && self.text.get(end - 1..end) == Some("\n") {
                    end -= 1;
                }
                return end;
            }
        }

        // If past the end, return end of text
        self.text.len()
    }

    /// Move the cursor up to the previous line.
    ///
    /// Attempts to maintain the same horizontal position (preferred_x).
    /// Returns the new byte offset, or the current offset if on the first line.
    ///
    /// The `preferred_x` parameter should be the x position the cursor was
    /// originally at before any vertical movement began. This allows the
    /// cursor to return to its preferred column when moving through lines
    /// of varying lengths.
    pub fn move_cursor_up(&self, offset: usize, preferred_x: f32) -> usize {
        if self.lines.is_empty() {
            return 0;
        }

        // Find current line
        let current_line_idx = self.line_index_for_offset(offset).unwrap_or(0);

        if current_line_idx == 0 {
            // Already on first line
            return self.lines[0].text_range.start;
        }

        // Move to previous line
        let prev_line = &self.lines[current_line_idx - 1];
        let align_offset = self.alignment_offset(prev_line);
        prev_line.offset_at_x(preferred_x - align_offset)
    }

    /// Move the cursor down to the next line.
    ///
    /// Attempts to maintain the same horizontal position (preferred_x).
    /// Returns the new byte offset, or the current offset if on the last line.
    ///
    /// The `preferred_x` parameter should be the x position the cursor was
    /// originally at before any vertical movement began.
    pub fn move_cursor_down(&self, offset: usize, preferred_x: f32) -> usize {
        if self.lines.is_empty() {
            return self.text.len();
        }

        // Find current line
        let current_line_idx = self.line_index_for_offset(offset).unwrap_or(self.lines.len() - 1);

        if current_line_idx >= self.lines.len() - 1 {
            // Already on last line
            return self.lines.last().map(|l| l.text_range.end).unwrap_or(self.text.len());
        }

        // Move to next line
        let next_line = &self.lines[current_line_idx + 1];
        let align_offset = self.alignment_offset(next_line);
        next_line.offset_at_x(preferred_x - align_offset)
    }

    /// Get the line index containing the given byte offset.
    fn line_index_for_offset(&self, offset: usize) -> Option<usize> {
        for (i, line) in self.lines.iter().enumerate() {
            if line.text_range.contains(&offset) || offset == line.text_range.start {
                return Some(i);
            }
            // Handle cursor at end of line with hard break
            if offset == line.text_range.end && line.is_hard_break {
                return Some(i);
            }
        }

        // If offset is at end of text, return last line
        if offset == self.text.len() && !self.lines.is_empty() {
            return Some(self.lines.len() - 1);
        }

        None
    }

    /// Get the x position of the cursor at the given byte offset.
    ///
    /// This is useful for tracking the preferred x position during
    /// vertical cursor navigation.
    pub fn cursor_x_at_offset(&self, offset: usize) -> f32 {
        let (x, _) = self.point_for_offset(offset);
        x
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::FontSystemConfig;

    fn create_test_font_system() -> FontSystem {
        let config = FontSystemConfig::new().load_system_fonts(false);
        FontSystem::with_config(config)
    }

    #[test]
    fn horizontal_align_conversion() {
        assert!(matches!(
            HorizontalAlign::Left.to_cosmic(),
            cosmic_text::Align::Left
        ));
        assert!(matches!(
            HorizontalAlign::Center.to_cosmic(),
            cosmic_text::Align::Center
        ));
        assert!(matches!(
            HorizontalAlign::Right.to_cosmic(),
            cosmic_text::Align::Right
        ));
        assert!(matches!(
            HorizontalAlign::Justified.to_cosmic(),
            cosmic_text::Align::Justified
        ));
    }

    #[test]
    fn wrap_mode_conversion() {
        assert!(matches!(WrapMode::None.to_cosmic(), Wrap::None));
        assert!(matches!(WrapMode::Word.to_cosmic(), Wrap::Word));
        assert!(matches!(WrapMode::Character.to_cosmic(), Wrap::Glyph));
        assert!(matches!(
            WrapMode::WordOrCharacter.to_cosmic(),
            Wrap::WordOrGlyph
        ));
    }

    #[test]
    fn layout_options_builder() {
        let options = TextLayoutOptions::new()
            .max_width(300.0)
            .max_height(200.0)
            .horizontal_align(HorizontalAlign::Center)
            .vertical_align(VerticalAlign::Middle)
            .wrap(WrapMode::Word)
            .line_height(1.5)
            .paragraph_spacing(10.0)
            .with_ellipsis()
            .ellipsis_string("...");

        assert_eq!(options.max_width, Some(300.0));
        assert_eq!(options.max_height, Some(200.0));
        assert_eq!(options.horizontal_align, HorizontalAlign::Center);
        assert_eq!(options.vertical_align, VerticalAlign::Middle);
        assert_eq!(options.wrap, WrapMode::Word);
        assert_eq!(options.line_height_multiplier, 1.5);
        assert_eq!(options.paragraph_spacing, 10.0);
        assert!(options.ellipsis);
        assert_eq!(options.ellipsis_string, "...");
    }

    #[test]
    fn layout_glyph_rtl() {
        let ltr = LayoutGlyph {
            glyph_id: 1,
            font_id: fontdb::ID::dummy(),
            x: 0.0,
            y: 0.0,
            width: 10.0,
            x_offset: 0.0,
            y_offset: 0.0,
            cluster: 0..1,
            font_size: 16.0,
            cache_key_flags: CacheKeyFlags::empty(),
            level: 0,
            color: None,
            metadata: 0,
        };
        assert!(!ltr.is_rtl());

        let rtl = LayoutGlyph { level: 1, ..ltr };
        assert!(rtl.is_rtl());
    }

    #[test]
    fn layout_glyph_bounds() {
        let glyph = LayoutGlyph {
            glyph_id: 1,
            font_id: fontdb::ID::dummy(),
            x: 10.0,
            y: 0.0,
            width: 20.0,
            x_offset: 0.0,
            y_offset: 0.0,
            cluster: 0..1,
            font_size: 16.0,
            cache_key_flags: CacheKeyFlags::empty(),
            level: 0,
            color: None,
            metadata: 0,
        };

        assert_eq!(glyph.x_end(), 30.0);
        assert!(glyph.contains_x(15.0));
        assert!(glyph.contains_x(10.0));
        assert!(!glyph.contains_x(9.9));
        assert!(!glyph.contains_x(30.0));
    }

    #[test]
    fn inline_element_creation() {
        let element = InlineElement::new(1, 32.0, 32.0)
            .with_vertical_align(InlineVerticalAlign::Middle);

        assert_eq!(element.id, 1);
        assert_eq!(element.width, 32.0);
        assert_eq!(element.height, 32.0);
        assert_eq!(element.vertical_align, InlineVerticalAlign::Middle);
    }

    #[test]
    fn text_span_creation() {
        let font = Font::new(FontFamily::SansSerif, 16.0);
        let span = TextSpan::new("Hello")
            .with_color([255, 0, 0, 255])
            .bold(&font);

        assert_eq!(span.text, "Hello");
        assert_eq!(span.color, Some([255, 0, 0, 255]));
        assert!(span.font.is_some());
        assert_eq!(span.font.unwrap().weight(), FontWeight::BOLD);
    }

    #[test]
    fn empty_layout() {
        let mut font_system = create_test_font_system();
        let font = Font::new(FontFamily::SansSerif, 16.0);
        let layout = TextLayout::new(&mut font_system, "", &font);

        assert_eq!(layout.line_count(), 0);
        assert_eq!(layout.width(), 0.0);
        assert_eq!(layout.height(), 0.0);
        assert!(!layout.is_truncated());
    }

    #[test]
    fn text_layout_default() {
        let layout = TextLayout::default();
        assert!(layout.text().is_empty());
        assert_eq!(layout.line_count(), 0);
        assert_eq!(layout.width(), 0.0);
        assert_eq!(layout.height(), 0.0);
    }

    #[test]
    fn layout_line_empty() {
        let line = LayoutLine {
            glyphs: Vec::new(),
            baseline_y: 0.0,
            top_y: 0.0,
            height: 20.0,
            width: 0.0,
            text_range: 0..0,
            is_hard_break: false,
        };

        assert!(line.is_empty());
        assert_eq!(line.glyph_count(), 0);
        assert_eq!(line.offset_at_x(10.0), 0);
        assert_eq!(line.x_for_offset(0), 0.0);
    }

    #[test]
    fn vertical_offset_calculation() {
        // Test vertical offset calculation using manually created layout
        // without requiring font shaping

        // Top alignment - offset should be 0
        let mut layout = TextLayout::default();
        layout.height = 20.0;
        layout.options.max_height = Some(100.0);
        layout.options.vertical_align = VerticalAlign::Top;
        assert_eq!(layout.vertical_offset(), 0.0);

        // Middle alignment - offset should center the content
        layout.options.vertical_align = VerticalAlign::Middle;
        let expected = (100.0 - 20.0) / 2.0;
        assert!((layout.vertical_offset() - expected).abs() < 0.01);

        // Bottom alignment - offset should push content to bottom
        layout.options.vertical_align = VerticalAlign::Bottom;
        let expected = 100.0 - 20.0;
        assert!((layout.vertical_offset() - expected).abs() < 0.01);
    }

    // =========================================================================
    // Text Editing Support Tests
    // =========================================================================

    #[test]
    fn selection_rect_creation() {
        let rect = SelectionRect::new(10.0, 20.0, 100.0, 30.0);
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 30.0);
        assert_eq!(rect.right(), 110.0);
        assert_eq!(rect.bottom(), 50.0);
    }

    #[test]
    fn selection_rect_contains() {
        let rect = SelectionRect::new(10.0, 20.0, 100.0, 30.0);

        // Inside
        assert!(rect.contains(50.0, 35.0));
        assert!(rect.contains(10.0, 20.0)); // Top-left corner

        // Outside
        assert!(!rect.contains(5.0, 35.0)); // Left of rect
        assert!(!rect.contains(115.0, 35.0)); // Right of rect
        assert!(!rect.contains(50.0, 15.0)); // Above rect
        assert!(!rect.contains(50.0, 55.0)); // Below rect

        // On boundary (exclusive right/bottom)
        assert!(!rect.contains(110.0, 35.0)); // Right edge
        assert!(!rect.contains(50.0, 50.0)); // Bottom edge
    }

    #[test]
    fn word_boundary_before_basic() {
        let mut layout = TextLayout::default();
        layout.text = "Hello World Test".to_string();

        // From middle of "World"
        assert_eq!(layout.word_boundary_before(8), 6); // "World" starts at 6

        // From start of "World"
        assert_eq!(layout.word_boundary_before(6), 6);

        // From space before "World"
        assert_eq!(layout.word_boundary_before(5), 0); // Goes back to "Hello"

        // From start
        assert_eq!(layout.word_boundary_before(0), 0);

        // From end
        assert_eq!(layout.word_boundary_before(16), 12); // "Test" starts at 12
    }

    #[test]
    fn word_boundary_after_basic() {
        let mut layout = TextLayout::default();
        layout.text = "Hello World Test".to_string();

        // From start of "Hello"
        assert_eq!(layout.word_boundary_after(0), 5); // "Hello" ends at 5

        // From middle of "World"
        assert_eq!(layout.word_boundary_after(8), 11); // "World" ends at 11

        // From space
        assert_eq!(layout.word_boundary_after(5), 11); // Next word "World" ends at 11

        // From end
        assert_eq!(layout.word_boundary_after(16), 16);
    }

    #[test]
    fn word_at_offset_basic() {
        let mut layout = TextLayout::default();
        layout.text = "Hello World".to_string();

        // Inside "Hello"
        assert_eq!(layout.word_at_offset(2), 0..5);

        // Inside "World"
        assert_eq!(layout.word_at_offset(8), 6..11);

        // On space (non-word)
        let range = layout.word_at_offset(5);
        assert!(range.is_empty() || range == (5..5));
    }

    #[test]
    fn word_boundary_empty_text() {
        let layout = TextLayout::default();

        assert_eq!(layout.word_boundary_before(0), 0);
        assert_eq!(layout.word_boundary_before(10), 0);
        assert_eq!(layout.word_boundary_after(0), 0);
        assert_eq!(layout.word_boundary_after(10), 0);
        assert_eq!(layout.word_at_offset(0), 0..0);
    }

    #[test]
    fn cursor_left_right_basic() {
        let mut layout = TextLayout::default();
        layout.text = "Hello".to_string();

        // Move right from start
        assert_eq!(layout.move_cursor_right(0), 1);
        assert_eq!(layout.move_cursor_right(1), 2);

        // Move left from end
        assert_eq!(layout.move_cursor_left(5), 4);
        assert_eq!(layout.move_cursor_left(4), 3);

        // Edge cases
        assert_eq!(layout.move_cursor_left(0), 0);
        assert_eq!(layout.move_cursor_right(5), 5);
    }

    #[test]
    fn cursor_left_right_unicode() {
        let mut layout = TextLayout::default();
        layout.text = "Héllo 👋".to_string(); // é is 2 bytes, 👋 is 4 bytes

        // Move through 'H'
        assert_eq!(layout.move_cursor_right(0), 1);

        // Move through 'é' (2 bytes)
        let pos = layout.move_cursor_right(1);
        assert!(pos > 1); // Should skip the whole grapheme

        // Move through emoji (4 bytes)
        let emoji_start = "Héllo ".len();
        let pos = layout.move_cursor_right(emoji_start);
        assert_eq!(pos, layout.text.len()); // Should skip the whole emoji
    }

    #[test]
    fn cursor_word_navigation() {
        let mut layout = TextLayout::default();
        layout.text = "Hello World Test".to_string();

        // Word right from start
        assert_eq!(layout.move_cursor_word_right(0), 5); // End of "Hello"

        // Word right from middle of word
        assert_eq!(layout.move_cursor_word_right(2), 5); // End of "Hello"

        // Word right from space
        assert_eq!(layout.move_cursor_word_right(5), 11); // End of "World"

        // Word left from end
        assert_eq!(layout.move_cursor_word_left(16), 12); // Start of "Test"

        // Word left from middle of word
        assert_eq!(layout.move_cursor_word_left(14), 12); // Start of "Test"

        // Word left from start of word
        assert_eq!(layout.move_cursor_word_left(12), 6); // Start of "World"
    }

    #[test]
    fn cursor_line_navigation() {
        let mut layout = TextLayout::default();
        layout.text = "Line one\nLine two".to_string();
        layout.lines = vec![
            LayoutLine {
                glyphs: Vec::new(),
                baseline_y: 16.0,
                top_y: 0.0,
                height: 20.0,
                width: 100.0,
                text_range: 0..9, // "Line one\n"
                is_hard_break: true,
            },
            LayoutLine {
                glyphs: Vec::new(),
                baseline_y: 36.0,
                top_y: 20.0,
                height: 20.0,
                width: 100.0,
                text_range: 9..17, // "Line two"
                is_hard_break: false,
            },
        ];

        // Line start from middle of first line
        assert_eq!(layout.move_cursor_to_line_start(4), 0);

        // Line start from second line
        assert_eq!(layout.move_cursor_to_line_start(12), 9);

        // Line end from middle of first line (excluding newline)
        assert_eq!(layout.move_cursor_to_line_end(4), 8);

        // Line end from second line
        assert_eq!(layout.move_cursor_to_line_end(12), 17);
    }

    #[test]
    fn cursor_vertical_navigation() {
        let mut layout = TextLayout::default();
        layout.text = "Line one\nLine two".to_string();
        layout.lines = vec![
            LayoutLine {
                glyphs: vec![
                    LayoutGlyph {
                        glyph_id: 1,
                        font_id: fontdb::ID::dummy(),
                        x: 0.0,
                        y: 16.0,
                        width: 10.0,
                        x_offset: 0.0,
                        y_offset: 0.0,
                        cluster: 0..1,
                        font_size: 16.0,
                        cache_key_flags: CacheKeyFlags::empty(),
                        level: 0,
                        color: None,
                        metadata: 0,
                    },
                ],
                baseline_y: 16.0,
                top_y: 0.0,
                height: 20.0,
                width: 100.0,
                text_range: 0..9,
                is_hard_break: true,
            },
            LayoutLine {
                glyphs: vec![
                    LayoutGlyph {
                        glyph_id: 1,
                        font_id: fontdb::ID::dummy(),
                        x: 0.0,
                        y: 36.0,
                        width: 10.0,
                        x_offset: 0.0,
                        y_offset: 0.0,
                        cluster: 9..10,
                        font_size: 16.0,
                        cache_key_flags: CacheKeyFlags::empty(),
                        level: 0,
                        color: None,
                        metadata: 0,
                    },
                ],
                baseline_y: 36.0,
                top_y: 20.0,
                height: 20.0,
                width: 100.0,
                text_range: 9..17,
                is_hard_break: false,
            },
        ];

        // Move down from first line
        let new_offset = layout.move_cursor_down(4, 5.0);
        assert!(new_offset >= 9 && new_offset <= 17); // Should be on second line

        // Move up from second line
        let new_offset = layout.move_cursor_up(12, 5.0);
        assert!(new_offset <= 9); // Should be on first line

        // Move up from first line (should stay at start)
        assert_eq!(layout.move_cursor_up(4, 5.0), 0);

        // Move down from last line (should go to end)
        let end_offset = layout.move_cursor_down(12, 5.0);
        assert_eq!(end_offset, 17);
    }

    #[test]
    fn selection_rects_single_line() {
        let mut layout = TextLayout::default();
        layout.text = "Hello World".to_string();
        layout.lines = vec![LayoutLine {
            glyphs: vec![
                LayoutGlyph {
                    glyph_id: 1,
                    font_id: fontdb::ID::dummy(),
                    x: 0.0,
                    y: 16.0,
                    width: 50.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                    cluster: 0..5, // "Hello"
                    font_size: 16.0,
                    cache_key_flags: CacheKeyFlags::empty(),
                    level: 0,
                    color: None,
                    metadata: 0,
                },
                LayoutGlyph {
                    glyph_id: 2,
                    font_id: fontdb::ID::dummy(),
                    x: 50.0,
                    y: 16.0,
                    width: 10.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                    cluster: 5..6, // " "
                    font_size: 16.0,
                    cache_key_flags: CacheKeyFlags::empty(),
                    level: 0,
                    color: None,
                    metadata: 0,
                },
                LayoutGlyph {
                    glyph_id: 3,
                    font_id: fontdb::ID::dummy(),
                    x: 60.0,
                    y: 16.0,
                    width: 50.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                    cluster: 6..11, // "World"
                    font_size: 16.0,
                    cache_key_flags: CacheKeyFlags::empty(),
                    level: 0,
                    color: None,
                    metadata: 0,
                },
            ],
            baseline_y: 16.0,
            top_y: 0.0,
            height: 20.0,
            width: 110.0,
            text_range: 0..11,
            is_hard_break: false,
        }];

        // Select "Hello"
        let rects = layout.selection_rects(0, 5);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].x, 0.0);
        assert!(rects[0].width > 0.0);

        // Empty selection
        let rects = layout.selection_rects(5, 5);
        assert!(rects.is_empty());

        // Invalid selection (start > end)
        let rects = layout.selection_rects(10, 5);
        assert!(rects.is_empty());
    }

    #[test]
    fn selection_rects_multi_line() {
        let mut layout = TextLayout::default();
        layout.text = "Line one\nLine two".to_string();
        layout.lines = vec![
            LayoutLine {
                glyphs: vec![LayoutGlyph {
                    glyph_id: 1,
                    font_id: fontdb::ID::dummy(),
                    x: 0.0,
                    y: 16.0,
                    width: 80.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                    cluster: 0..8, // "Line one"
                    font_size: 16.0,
                    cache_key_flags: CacheKeyFlags::empty(),
                    level: 0,
                    color: None,
                    metadata: 0,
                }],
                baseline_y: 16.0,
                top_y: 0.0,
                height: 20.0,
                width: 80.0,
                text_range: 0..9, // Including newline
                is_hard_break: true,
            },
            LayoutLine {
                glyphs: vec![LayoutGlyph {
                    glyph_id: 2,
                    font_id: fontdb::ID::dummy(),
                    x: 0.0,
                    y: 36.0,
                    width: 80.0,
                    x_offset: 0.0,
                    y_offset: 0.0,
                    cluster: 9..17, // "Line two"
                    font_size: 16.0,
                    cache_key_flags: CacheKeyFlags::empty(),
                    level: 0,
                    color: None,
                    metadata: 0,
                }],
                baseline_y: 36.0,
                top_y: 20.0,
                height: 20.0,
                width: 80.0,
                text_range: 9..17,
                is_hard_break: false,
            },
        ];

        // Select across both lines
        let rects = layout.selection_rects(4, 14);
        assert_eq!(rects.len(), 2); // One rect per line

        // First rect should be on first line
        assert_eq!(rects[0].y, 0.0);

        // Second rect should be on second line
        assert_eq!(rects[1].y, 20.0);
    }

    #[test]
    fn text_span_with_background_color() {
        let span = TextSpan::new("Highlighted")
            .with_background_color([255, 255, 0, 128]);

        assert_eq!(span.text, "Highlighted");
        assert_eq!(span.background_color, Some([255, 255, 0, 128]));
        assert!(span.decorations.is_empty());
    }

    #[test]
    fn text_span_with_decorations() {
        let span = TextSpan::new("Decorated")
            .with_underline()
            .with_strikethrough();

        assert_eq!(span.text, "Decorated");
        assert_eq!(span.decorations.len(), 2);
        assert_eq!(span.decorations[0].decoration_type, super::super::TextDecorationType::Underline);
        assert_eq!(span.decorations[1].decoration_type, super::super::TextDecorationType::Strikethrough);
    }

    #[test]
    fn text_span_with_wavy_underline() {
        let span = TextSpan::new("Error")
            .with_wavy_underline();

        assert_eq!(span.decorations.len(), 1);
        assert_eq!(span.decorations[0].decoration_type, super::super::TextDecorationType::Underline);
        assert_eq!(span.decorations[0].style, super::super::TextDecorationStyle::Wavy);
    }

    #[test]
    fn text_span_with_overline() {
        let span = TextSpan::new("Overlined")
            .with_overline();

        assert_eq!(span.decorations.len(), 1);
        assert_eq!(span.decorations[0].decoration_type, super::super::TextDecorationType::Overline);
    }

    #[test]
    fn text_span_combined_styling() {
        let font = Font::new(FontFamily::SansSerif, 16.0);
        let span = TextSpan::new("Styled")
            .with_color([255, 0, 0, 255])
            .with_background_color([255, 255, 0, 128])
            .with_underline()
            .bold(&font);

        assert_eq!(span.text, "Styled");
        assert_eq!(span.color, Some([255, 0, 0, 255]));
        assert_eq!(span.background_color, Some([255, 255, 0, 128]));
        assert_eq!(span.decorations.len(), 1);
        assert!(span.font.is_some());
    }

    #[test]
    fn background_rect_new() {
        let rect = BackgroundRect::new(10.0, 20.0, 100.0, 30.0, [255, 0, 0, 255]);
        assert_eq!(rect.x, 10.0);
        assert_eq!(rect.y, 20.0);
        assert_eq!(rect.width, 100.0);
        assert_eq!(rect.height, 30.0);
        assert_eq!(rect.color, [255, 0, 0, 255]);
    }

    #[test]
    fn decoration_line_width() {
        let line = DecorationLine {
            x_start: 10.0,
            x_end: 110.0,
            y: 50.0,
            thickness: 1.5,
            color: [0, 0, 0, 255],
            style: super::super::TextDecorationStyle::Solid,
            decoration_type: super::super::TextDecorationType::Underline,
        };
        assert_eq!(line.width(), 100.0);
    }

    // =========================================================================
    // Internationalization Tests
    // =========================================================================

    #[test]
    fn direction_option_builder() {
        let options = TextLayoutOptions::default()
            .direction(TextDirection::RightToLeft);
        assert_eq!(options.direction, TextDirection::RightToLeft);

        let ltr_options = TextLayoutOptions::default().ltr();
        assert_eq!(ltr_options.direction, TextDirection::LeftToRight);

        let rtl_options = TextLayoutOptions::default().rtl();
        assert_eq!(rtl_options.direction, TextDirection::RightToLeft);

        let auto_options = TextLayoutOptions::default();
        assert_eq!(auto_options.direction, TextDirection::Auto);
    }

    #[test]
    fn direction_resolve_ltr() {
        // Test direction resolution using TextDirection::resolve directly
        // This doesn't require font shaping
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("Hello World"), TextDirection::LeftToRight);
    }

    #[test]
    fn direction_resolve_rtl() {
        // Arabic text should resolve to RTL
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("مرحبا بالعالم"), TextDirection::RightToLeft);
    }

    #[test]
    fn direction_resolve_explicit_override() {
        // Explicit direction should override text content
        let explicit_rtl = TextDirection::RightToLeft;
        assert_eq!(explicit_rtl.resolve("Hello World"), TextDirection::RightToLeft);

        let explicit_ltr = TextDirection::LeftToRight;
        assert_eq!(explicit_ltr.resolve("مرحبا"), TextDirection::LeftToRight);
    }

    #[test]
    fn effective_alignment_ltr() {
        let mut layout = TextLayout::default();
        layout.resolved_direction = TextDirection::LeftToRight;

        // LTR: Left stays Left
        layout.options.horizontal_align = HorizontalAlign::Left;
        assert_eq!(layout.effective_alignment(), HorizontalAlign::Left);

        // LTR: Right stays Right
        layout.options.horizontal_align = HorizontalAlign::Right;
        assert_eq!(layout.effective_alignment(), HorizontalAlign::Right);

        // LTR: Center stays Center
        layout.options.horizontal_align = HorizontalAlign::Center;
        assert_eq!(layout.effective_alignment(), HorizontalAlign::Center);
    }

    #[test]
    fn effective_alignment_rtl() {
        let mut layout = TextLayout::default();
        layout.resolved_direction = TextDirection::RightToLeft;

        // RTL: Left becomes Right
        layout.options.horizontal_align = HorizontalAlign::Left;
        assert_eq!(layout.effective_alignment(), HorizontalAlign::Right);

        // RTL: Right becomes Left
        layout.options.horizontal_align = HorizontalAlign::Right;
        assert_eq!(layout.effective_alignment(), HorizontalAlign::Left);

        // RTL: Center stays Center
        layout.options.horizontal_align = HorizontalAlign::Center;
        assert_eq!(layout.effective_alignment(), HorizontalAlign::Center);

        // RTL: Justified stays Justified
        layout.options.horizontal_align = HorizontalAlign::Justified;
        assert_eq!(layout.effective_alignment(), HorizontalAlign::Justified);
    }

    #[test]
    fn direction_mixed_text() {
        // Mixed English and Arabic - first strong char is English
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("Hello مرحبا World"), TextDirection::LeftToRight);
    }

    #[test]
    fn direction_rtl_first_char() {
        // Arabic first, then English - first strong char is Arabic
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("مرحبا Hello"), TextDirection::RightToLeft);
    }

    #[test]
    fn direction_neutral_only_defaults_ltr() {
        // Numbers and punctuation only - no strong directional characters
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("123!@#"), TextDirection::LeftToRight);
    }

    #[test]
    fn direction_hebrew_text() {
        // Hebrew text should be RTL
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("שלום עולם"), TextDirection::RightToLeft);
    }

    #[test]
    fn direction_cyrillic_is_ltr() {
        // Russian text (Cyrillic script is LTR)
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("Привет мир"), TextDirection::LeftToRight);
    }

    #[test]
    fn direction_chinese_is_ltr() {
        // Chinese text (CJK is LTR)
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("你好世界"), TextDirection::LeftToRight);
    }

    #[test]
    fn direction_persian_is_rtl() {
        // Persian/Farsi text (uses Arabic script, RTL)
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("سلام دنیا"), TextDirection::RightToLeft);
    }

    #[test]
    fn direction_greek_is_ltr() {
        // Greek text is LTR
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("Γειά σου κόσμε"), TextDirection::LeftToRight);
    }

    #[test]
    fn direction_thai_is_ltr() {
        // Thai text is LTR
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("สวัสดี"), TextDirection::LeftToRight);
    }

    #[test]
    fn direction_japanese_is_ltr() {
        // Japanese text (horizontal) is LTR
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("こんにちは"), TextDirection::LeftToRight);
    }

    #[test]
    fn direction_korean_is_ltr() {
        // Korean text is LTR
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("안녕하세요"), TextDirection::LeftToRight);
    }
}
