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

use super::{Font, FontStyle, FontSystem, FontWeight};
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
}

impl<'a> TextSpan<'a> {
    /// Create a new text span.
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            font: None,
            color: None,
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
        let mut layout = Self {
            text: text.to_string(),
            lines: Vec::new(),
            width: 0.0,
            height: 0.0,
            options,
            is_truncated: false,
            inline_elements: Vec::new(),
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
        let mut layout = Self {
            text,
            lines: Vec::new(),
            width: 0.0,
            height: 0.0,
            options,
            is_truncated: false,
            inline_elements: Vec::new(),
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
        let mut layout = Self {
            text: text.to_string(),
            lines: Vec::new(),
            width: 0.0,
            height: 0.0,
            options,
            is_truncated: false,
            inline_elements: Vec::new(),
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

        // Set alignment for each line
        for line in buffer.lines.iter_mut() {
            line.set_align(Some(self.options.horizontal_align.to_cosmic()));
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
            line.set_align(Some(self.options.horizontal_align.to_cosmic()));
        }

        // Re-shape after setting rich text
        buffer.shape_until_scroll(font_system.inner_mut(), false);

        // Handle ellipsis truncation
        if self.options.ellipsis {
            self.apply_ellipsis(font_system, &mut buffer, default_font);
        }

        // Extract layout data
        self.extract_layout(&buffer);
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
            line.set_align(Some(self.options.horizontal_align.to_cosmic()));
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
                    text_range: usize::MAX..0,
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
                    current_line.text_range.start = layout_glyph.start;
                }
                current_line.text_range.end = current_line.text_range.end.max(layout_glyph.end);

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
        }
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
}
