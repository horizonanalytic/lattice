//! Styled document model for rich text editing.
//!
//! This module provides a document model that stores text with character-level
//! formatting attributes. It's designed for efficient editing operations while
//! maintaining formatting consistency.

use std::ops::Range;

use horizon_lattice_render::text::{FontFamily, FontWeight, HorizontalAlign};
use horizon_lattice_render::Color;

/// Character-level formatting attributes.
///
/// Represents the styling applied to a range of text characters.
/// All `Option` fields mean "inherit from default" when `None`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharFormat {
    /// Whether the text is bold.
    pub bold: bool,
    /// Whether the text is italic.
    pub italic: bool,
    /// Whether the text has underline.
    pub underline: bool,
    /// Whether the text has strikethrough.
    pub strikethrough: bool,
    /// Foreground (text) color. None means use default text color.
    pub foreground_color: Option<Color>,
    /// Background (highlight) color. None means no background highlight.
    pub background_color: Option<Color>,
    /// Font family. None means use widget's default font family.
    pub font_family: Option<FontFamily>,
    /// Font size in pixels. None means use widget's default font size.
    pub font_size: Option<f32>,
    /// Font weight (100-900). None means use default weight (or Bold if `bold` is true).
    pub font_weight: Option<FontWeight>,
}

impl CharFormat {
    /// Create a new default (unstyled) format.
    pub fn new() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            foreground_color: None,
            background_color: None,
            font_family: None,
            font_size: None,
            font_weight: None,
        }
    }

    /// Check if this format has any styling applied.
    pub fn is_styled(&self) -> bool {
        self.bold
            || self.italic
            || self.underline
            || self.strikethrough
            || self.foreground_color.is_some()
            || self.background_color.is_some()
            || self.font_family.is_some()
            || self.font_size.is_some()
            || self.font_weight.is_some()
    }

    /// Create a bold format.
    pub fn bold() -> Self {
        Self {
            bold: true,
            italic: false,
            underline: false,
            strikethrough: false,
            foreground_color: None,
            background_color: None,
            font_family: None,
            font_size: None,
            font_weight: None,
        }
    }

    /// Create an italic format.
    pub fn italic() -> Self {
        Self {
            bold: false,
            italic: true,
            underline: false,
            strikethrough: false,
            foreground_color: None,
            background_color: None,
            font_family: None,
            font_size: None,
            font_weight: None,
        }
    }

    /// Builder method to set bold.
    pub fn with_bold(mut self, bold: bool) -> Self {
        self.bold = bold;
        self
    }

    /// Builder method to set italic.
    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    /// Builder method to set underline.
    pub fn with_underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }

    /// Builder method to set strikethrough.
    pub fn with_strikethrough(mut self, strikethrough: bool) -> Self {
        self.strikethrough = strikethrough;
        self
    }

    /// Builder method to set foreground (text) color.
    pub fn with_foreground_color(mut self, color: Option<Color>) -> Self {
        self.foreground_color = color;
        self
    }

    /// Builder method to set background (highlight) color.
    pub fn with_background_color(mut self, color: Option<Color>) -> Self {
        self.background_color = color;
        self
    }

    /// Builder method to set font family.
    pub fn with_font_family(mut self, family: Option<FontFamily>) -> Self {
        self.font_family = family;
        self
    }

    /// Builder method to set font size.
    pub fn with_font_size(mut self, size: Option<f32>) -> Self {
        self.font_size = size;
        self
    }

    /// Builder method to set font weight.
    pub fn with_font_weight(mut self, weight: Option<FontWeight>) -> Self {
        self.font_weight = weight;
        self
    }

    /// Merge another format into this one (toggle style).
    ///
    /// If the other format has a style set, it will be toggled in this format.
    pub fn merge_toggle(&mut self, other: &CharFormat) {
        if other.bold {
            self.bold = !self.bold;
        }
        if other.italic {
            self.italic = !self.italic;
        }
        if other.underline {
            self.underline = !self.underline;
        }
        if other.strikethrough {
            self.strikethrough = !self.strikethrough;
        }
    }
}

/// A run of text with a specific format.
///
/// Format runs are stored as byte ranges in the document text.
#[derive(Debug, Clone, PartialEq)]
pub struct FormatRun {
    /// The byte range this run covers (start..end).
    pub range: Range<usize>,
    /// The format applied to this range.
    pub format: CharFormat,
}

impl FormatRun {
    /// Create a new format run.
    pub fn new(range: Range<usize>, format: CharFormat) -> Self {
        Self { range, format }
    }

    /// Check if this run is empty.
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// Get the length of this run in bytes.
    pub fn len(&self) -> usize {
        self.range.len()
    }

    /// Check if this run overlaps with a byte range.
    pub fn overlaps(&self, range: &Range<usize>) -> bool {
        self.range.start < range.end && range.start < self.range.end
    }

    /// Check if this run contains a byte position.
    pub fn contains(&self, pos: usize) -> bool {
        self.range.contains(&pos)
    }
}

/// Line spacing options for paragraphs.
///
/// Controls the vertical space between lines within a paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LineSpacing {
    /// Single line spacing (1.2x line height multiplier).
    #[default]
    Single,
    /// 1.5x line spacing (1.5x line height multiplier).
    OnePointFive,
    /// Double line spacing (2.0x line height multiplier).
    Double,
    /// Custom line height multiplier.
    Custom(f32),
}

impl LineSpacing {
    /// Convert line spacing to a line height multiplier.
    pub fn to_multiplier(self) -> f32 {
        match self {
            LineSpacing::Single => 1.2,
            LineSpacing::OnePointFive => 1.5,
            LineSpacing::Double => 2.0,
            LineSpacing::Custom(m) => m,
        }
    }
}

/// List marker styles for bulleted and numbered lists.
///
/// Modeled after Qt's QTextListFormat::Style, with negative values
/// for bullets and positive values for numbered lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListStyle {
    // Bullet styles (unordered lists)
    /// Filled circle bullet (•)
    Disc,
    /// Empty circle bullet (○)
    Circle,
    /// Filled square bullet (■)
    Square,

    // Numbered styles (ordered lists)
    /// Decimal numbers (1, 2, 3, ...)
    Decimal,
    /// Lowercase letters (a, b, c, ...)
    LowerAlpha,
    /// Uppercase letters (A, B, C, ...)
    UpperAlpha,
    /// Lowercase Roman numerals (i, ii, iii, ...)
    LowerRoman,
    /// Uppercase Roman numerals (I, II, III, ...)
    UpperRoman,
}

impl ListStyle {
    /// Check if this is a bullet (unordered) list style.
    pub fn is_bullet(&self) -> bool {
        matches!(self, ListStyle::Disc | ListStyle::Circle | ListStyle::Square)
    }

    /// Check if this is a numbered (ordered) list style.
    pub fn is_numbered(&self) -> bool {
        !self.is_bullet()
    }

    /// Get the marker string for a bullet style.
    /// Returns None for numbered styles.
    pub fn bullet_marker(&self) -> Option<&'static str> {
        match self {
            ListStyle::Disc => Some("•"),
            ListStyle::Circle => Some("○"),
            ListStyle::Square => Some("■"),
            _ => None,
        }
    }

    /// Get the marker string for a numbered style at a given index.
    /// Index is 0-based (first item = 0).
    pub fn number_marker(&self, index: usize, start: usize) -> Option<String> {
        let n = start + index;
        match self {
            ListStyle::Decimal => Some(format!("{}.", n)),
            ListStyle::LowerAlpha => Some(format!("{}.", Self::to_alpha(n, false))),
            ListStyle::UpperAlpha => Some(format!("{}.", Self::to_alpha(n, true))),
            ListStyle::LowerRoman => Some(format!("{}.", Self::to_roman(n, false))),
            ListStyle::UpperRoman => Some(format!("{}.", Self::to_roman(n, true))),
            _ => None,
        }
    }

    /// Convert a number to alphabetic representation (1=a, 2=b, ..., 26=z, 27=aa, ...).
    fn to_alpha(n: usize, uppercase: bool) -> String {
        if n == 0 {
            return if uppercase { "A".to_string() } else { "a".to_string() };
        }
        let mut result = String::new();
        let mut num = n;
        let base = if uppercase { b'A' } else { b'a' };
        while num > 0 {
            let digit = ((num - 1) % 26) as u8;
            result.insert(0, (base + digit) as char);
            num = (num - 1) / 26;
        }
        result
    }

    /// Convert a number to Roman numeral representation.
    /// Supports numbers up to 4999.
    fn to_roman(n: usize, uppercase: bool) -> String {
        if n == 0 || n > 4999 {
            return n.to_string(); // Fallback for out-of-range
        }

        let numerals = if uppercase {
            [
                (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"),
                (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
                (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
            ]
        } else {
            [
                (1000, "m"), (900, "cm"), (500, "d"), (400, "cd"),
                (100, "c"), (90, "xc"), (50, "l"), (40, "xl"),
                (10, "x"), (9, "ix"), (5, "v"), (4, "iv"), (1, "i"),
            ]
        };

        let mut result = String::new();
        let mut num = n;
        for (value, symbol) in numerals {
            while num >= value {
                result.push_str(symbol);
                num -= value;
            }
        }
        result
    }

    /// Get the default bullet styles for each nesting level.
    /// Level 0 = Disc, Level 1 = Circle, Level 2+ = Square.
    pub fn bullet_for_level(level: usize) -> Self {
        match level {
            0 => ListStyle::Disc,
            1 => ListStyle::Circle,
            _ => ListStyle::Square,
        }
    }

    /// Get the default numbered styles for each nesting level.
    /// Level 0 = Decimal, Level 1 = LowerAlpha, Level 2 = LowerRoman, Level 3+ = Decimal.
    pub fn number_for_level(level: usize) -> Self {
        match level {
            0 => ListStyle::Decimal,
            1 => ListStyle::LowerAlpha,
            2 => ListStyle::LowerRoman,
            _ => ListStyle::Decimal,
        }
    }
}

impl Default for ListStyle {
    fn default() -> Self {
        ListStyle::Disc
    }
}

/// List formatting information for a paragraph.
///
/// Contains the style, nesting level, and starting number for list items.
#[derive(Debug, Clone, PartialEq)]
pub struct ListFormat {
    /// The list marker style (bullet or number type).
    pub style: ListStyle,
    /// The nesting level (0 = top level, 1 = first indent, etc.).
    pub indent_level: usize,
    /// The starting number for numbered lists (default: 1).
    /// Ignored for bullet lists.
    pub start: usize,
}

impl ListFormat {
    /// Standard indent step for each list level in pixels.
    pub const INDENT_STEP: f32 = 24.0;

    /// Create a new list format with the given style.
    pub fn new(style: ListStyle) -> Self {
        Self {
            style,
            indent_level: 0,
            start: 1,
        }
    }

    /// Create a bullet list format.
    pub fn bullet() -> Self {
        Self::new(ListStyle::Disc)
    }

    /// Create a numbered list format.
    pub fn numbered() -> Self {
        Self::new(ListStyle::Decimal)
    }

    /// Builder method to set the indent level.
    pub fn with_indent_level(mut self, level: usize) -> Self {
        self.indent_level = level;
        self
    }

    /// Builder method to set the start number.
    pub fn with_start(mut self, start: usize) -> Self {
        self.start = start;
        self
    }

    /// Get the left indent in pixels for this list level.
    pub fn left_indent(&self) -> f32 {
        (self.indent_level + 1) as f32 * Self::INDENT_STEP
    }

    /// Get the marker for this list item.
    /// For bullets, returns the bullet character.
    /// For numbered lists, returns the formatted number with the given index.
    pub fn marker(&self, item_index: usize) -> String {
        if self.style.is_bullet() {
            self.style.bullet_marker().unwrap_or("•").to_string()
        } else {
            self.style.number_marker(item_index, self.start).unwrap_or_default()
        }
    }
}

impl Default for ListFormat {
    fn default() -> Self {
        Self::bullet()
    }
}

/// Paragraph/block-level formatting attributes.
///
/// Represents the styling applied to a paragraph of text.
/// Paragraphs are defined by newline characters.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlockFormat {
    /// Horizontal text alignment.
    pub alignment: HorizontalAlign,
    /// Left margin indent in pixels.
    /// This shifts the entire paragraph to the right.
    pub left_indent: f32,
    /// First line indent in pixels (relative to left_indent).
    /// Positive values indent the first line further right.
    /// Negative values create a "hanging indent" where the first line
    /// starts to the left of subsequent lines.
    pub first_line_indent: f32,
    /// Line spacing within the paragraph.
    pub line_spacing: LineSpacing,
    /// Extra space before the paragraph in pixels.
    pub spacing_before: f32,
    /// Extra space after the paragraph in pixels.
    pub spacing_after: f32,
    /// List formatting. None means this is not a list item.
    pub list_format: Option<ListFormat>,
}

impl BlockFormat {
    /// Default indent step size in pixels for increase/decrease operations.
    pub const INDENT_STEP: f32 = 40.0;

    /// Create a new default block format.
    pub fn new() -> Self {
        Self {
            alignment: HorizontalAlign::Left,
            left_indent: 0.0,
            first_line_indent: 0.0,
            line_spacing: LineSpacing::Single,
            spacing_before: 0.0,
            spacing_after: 0.0,
            list_format: None,
        }
    }

    /// Check if this format has any non-default styling.
    pub fn is_styled(&self) -> bool {
        self.alignment != HorizontalAlign::Left
            || self.left_indent != 0.0
            || self.first_line_indent != 0.0
            || self.line_spacing != LineSpacing::Single
            || self.spacing_before != 0.0
            || self.spacing_after != 0.0
            || self.list_format.is_some()
    }

    /// Check if this paragraph is a list item.
    pub fn is_list_item(&self) -> bool {
        self.list_format.is_some()
    }

    /// Builder method to set list format.
    pub fn with_list_format(mut self, list_format: Option<ListFormat>) -> Self {
        self.list_format = list_format;
        self
    }

    /// Create a bullet list item block format.
    pub fn bullet_list() -> Self {
        Self {
            list_format: Some(ListFormat::bullet()),
            ..Self::new()
        }
    }

    /// Create a numbered list item block format.
    pub fn numbered_list() -> Self {
        Self {
            list_format: Some(ListFormat::numbered()),
            ..Self::new()
        }
    }

    /// Builder method to set alignment.
    pub fn with_alignment(mut self, alignment: HorizontalAlign) -> Self {
        self.alignment = alignment;
        self
    }

    /// Create a left-aligned block format.
    pub fn left() -> Self {
        Self::new()
    }

    /// Create a center-aligned block format.
    pub fn center() -> Self {
        Self {
            alignment: HorizontalAlign::Center,
            ..Self::new()
        }
    }

    /// Create a right-aligned block format.
    pub fn right() -> Self {
        Self {
            alignment: HorizontalAlign::Right,
            ..Self::new()
        }
    }

    /// Create a justified block format.
    pub fn justified() -> Self {
        Self {
            alignment: HorizontalAlign::Justified,
            ..Self::new()
        }
    }

    /// Builder method to set left indent.
    pub fn with_left_indent(mut self, indent: f32) -> Self {
        self.left_indent = indent;
        self
    }

    /// Builder method to set first line indent.
    pub fn with_first_line_indent(mut self, indent: f32) -> Self {
        self.first_line_indent = indent;
        self
    }

    /// Builder method to set line spacing.
    pub fn with_line_spacing(mut self, spacing: LineSpacing) -> Self {
        self.line_spacing = spacing;
        self
    }

    /// Builder method to set spacing before paragraph.
    pub fn with_spacing_before(mut self, spacing: f32) -> Self {
        self.spacing_before = spacing;
        self
    }

    /// Builder method to set spacing after paragraph.
    pub fn with_spacing_after(mut self, spacing: f32) -> Self {
        self.spacing_after = spacing;
        self
    }

    /// Get the effective indent for the first line of the paragraph.
    pub fn first_line_effective_indent(&self) -> f32 {
        self.left_indent + self.first_line_indent
    }

    /// Get the effective indent for subsequent lines of the paragraph.
    pub fn subsequent_lines_indent(&self) -> f32 {
        self.left_indent
    }

    /// Get the line height multiplier for this paragraph.
    pub fn line_height_multiplier(&self) -> f32 {
        self.line_spacing.to_multiplier()
    }
}

/// A run of paragraphs with a specific block format.
///
/// Block runs track paragraph indices (0-based), not byte ranges.
/// A paragraph is defined as text ending with a newline or end-of-document.
#[derive(Debug, Clone, PartialEq)]
pub struct BlockRun {
    /// The paragraph range this run covers (start..end, 0-based indices).
    pub range: Range<usize>,
    /// The format applied to these paragraphs.
    pub format: BlockFormat,
}

impl BlockRun {
    /// Create a new block run.
    pub fn new(range: Range<usize>, format: BlockFormat) -> Self {
        Self { range, format }
    }

    /// Check if this run is empty.
    pub fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    /// Get the number of paragraphs in this run.
    pub fn len(&self) -> usize {
        self.range.len()
    }

    /// Check if this run overlaps with a paragraph range.
    pub fn overlaps(&self, range: &Range<usize>) -> bool {
        self.range.start < range.end && range.start < self.range.end
    }

    /// Check if this run contains a paragraph index.
    pub fn contains(&self, para_idx: usize) -> bool {
        self.range.contains(&para_idx)
    }
}

/// A styled text document that maintains text content with formatting.
///
/// The document stores:
/// - Raw text content as a single String
/// - Format runs that describe character-level formatting for ranges
/// - Block runs that describe paragraph-level formatting
///
/// Format runs are kept sorted by start position and non-overlapping.
/// Unformatted text (default format) doesn't need explicit runs.
#[derive(Debug, Clone)]
pub struct StyledDocument {
    /// The plain text content.
    text: String,
    /// Format runs sorted by start position.
    /// Runs are non-overlapping and only stored for non-default formats.
    format_runs: Vec<FormatRun>,
    /// Block runs for paragraph-level formatting.
    /// Paragraph indices are 0-based, with paragraphs delimited by newlines.
    block_runs: Vec<BlockRun>,
}

impl Default for StyledDocument {
    fn default() -> Self {
        Self::new()
    }
}

impl StyledDocument {
    /// Create a new empty document.
    pub fn new() -> Self {
        Self {
            text: String::new(),
            format_runs: Vec::new(),
            block_runs: Vec::new(),
        }
    }

    /// Create a document from plain text.
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            format_runs: Vec::new(),
            block_runs: Vec::new(),
        }
    }

    /// Get the plain text content.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the text length in bytes.
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Check if the document is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Get the format runs.
    pub fn format_runs(&self) -> &[FormatRun] {
        &self.format_runs
    }

    /// Set the text content, clearing all formatting.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.format_runs.clear();
        self.block_runs.clear();
    }

    /// Clear the document.
    pub fn clear(&mut self) {
        self.text.clear();
        self.format_runs.clear();
        self.block_runs.clear();
    }

    /// Get the block runs.
    pub fn block_runs(&self) -> &[BlockRun] {
        &self.block_runs
    }

    /// Get the format at a specific byte position.
    pub fn format_at(&self, pos: usize) -> CharFormat {
        for run in &self.format_runs {
            if run.contains(pos) {
                return run.format.clone();
            }
            if run.range.start > pos {
                break;
            }
        }
        CharFormat::default()
    }

    /// Get the format for a range (returns format if uniform, or mixed indicator).
    ///
    /// Returns `Some(format)` if the entire range has uniform formatting,
    /// or `None` if the formatting is mixed.
    pub fn format_for_range(&self, range: &Range<usize>) -> Option<CharFormat> {
        if range.is_empty() {
            return Some(self.format_at(range.start));
        }

        // Collect all formats in the range
        let mut formats: Vec<CharFormat> = Vec::new();
        let mut covered = range.start;

        for run in &self.format_runs {
            if run.range.start >= range.end {
                break;
            }
            if run.overlaps(range) {
                // Add default format for any gap before this run
                if covered < run.range.start {
                    formats.push(CharFormat::default());
                }
                formats.push(run.format.clone());
                covered = run.range.end.min(range.end);
            }
        }

        // Add default format for any remaining gap
        if covered < range.end {
            formats.push(CharFormat::default());
        }

        // Check if all formats are the same
        if formats.is_empty() {
            Some(CharFormat::default())
        } else {
            let first = &formats[0];
            if formats.iter().all(|f| f == first) {
                Some(first.clone())
            } else {
                None
            }
        }
    }

    /// Insert text at a position with a specific format.
    pub fn insert(&mut self, pos: usize, text: &str, format: CharFormat) {
        if text.is_empty() {
            return;
        }

        let len = text.len();

        // Insert the text
        self.text.insert_str(pos, text);

        // Adjust existing format runs
        for run in &mut self.format_runs {
            if run.range.start >= pos {
                // Run is after insertion point - shift it
                run.range.start += len;
                run.range.end += len;
            } else if run.range.end > pos {
                // Run contains insertion point - extend it or split
                if run.format == format {
                    // Same format - just extend
                    run.range.end += len;
                } else {
                    // Different format - extend the run past the insertion
                    run.range.end += len;
                }
            }
        }

        // Add a format run for the inserted text if it has styling
        if format.is_styled() {
            self.set_format(pos..pos + len, format);
        }
    }

    /// Delete text in a range.
    pub fn delete(&mut self, range: Range<usize>) -> String {
        if range.is_empty() {
            return String::new();
        }

        let deleted = self.text[range.clone()].to_string();
        let len = range.len();

        // Remove the text
        self.text.replace_range(range.clone(), "");

        // Adjust format runs
        let mut to_remove = Vec::new();
        for (i, run) in self.format_runs.iter_mut().enumerate() {
            if run.range.start >= range.end {
                // Run is after deletion - shift it back
                run.range.start -= len;
                run.range.end -= len;
            } else if run.range.end <= range.start {
                // Run is before deletion - no change
            } else if run.range.start >= range.start && run.range.end <= range.end {
                // Run is entirely within deletion - mark for removal
                to_remove.push(i);
            } else if run.range.start < range.start && run.range.end > range.end {
                // Run spans the deletion - shrink it
                run.range.end -= len;
            } else if run.range.start < range.start {
                // Run overlaps start of deletion
                run.range.end = range.start;
            } else {
                // Run overlaps end of deletion
                run.range.start = range.start;
                run.range.end -= range.end - run.range.start;
            }
        }

        // Remove empty runs in reverse order
        for i in to_remove.into_iter().rev() {
            self.format_runs.remove(i);
        }

        // Remove any runs that became empty
        self.format_runs.retain(|r| !r.is_empty());

        deleted
    }

    /// Set the format for a range of text.
    ///
    /// This will merge with adjacent runs of the same format and split
    /// existing runs as needed.
    pub fn set_format(&mut self, range: Range<usize>, format: CharFormat) {
        if range.is_empty() {
            return;
        }

        // Remove the default format case - just delete overlapping runs
        if !format.is_styled() {
            self.format_runs.retain(|run| !run.overlaps(&range));
            // Split runs that partially overlap
            let mut new_runs = Vec::new();
            for run in &mut self.format_runs {
                if run.range.start < range.start && run.range.end > range.end {
                    // Run spans the range - split it
                    new_runs.push(FormatRun::new(range.end..run.range.end, run.format.clone()));
                    run.range.end = range.start;
                } else if run.range.start < range.start && run.range.end > range.start {
                    // Run overlaps start
                    run.range.end = range.start;
                } else if run.range.start < range.end && run.range.end > range.end {
                    // Run overlaps end
                    run.range.start = range.end;
                }
            }
            self.format_runs.extend(new_runs);
            self.normalize_runs();
            return;
        }

        // Split and remove overlapping runs
        let mut new_runs = Vec::new();
        let mut to_remove = Vec::new();

        for (i, run) in self.format_runs.iter_mut().enumerate() {
            if !run.overlaps(&range) {
                continue;
            }

            if run.format == format {
                // Same format - will be merged later
                continue;
            }

            if run.range.start < range.start && run.range.end > range.end {
                // Run spans the entire range - split into three
                new_runs.push(FormatRun::new(range.end..run.range.end, run.format.clone()));
                run.range.end = range.start;
            } else if run.range.start < range.start {
                // Run overlaps start - truncate
                run.range.end = range.start;
            } else if run.range.end > range.end {
                // Run overlaps end - truncate
                run.range.start = range.end;
            } else {
                // Run is entirely within range - remove
                to_remove.push(i);
            }
        }

        // Remove fully overlapped runs
        for i in to_remove.into_iter().rev() {
            self.format_runs.remove(i);
        }

        // Add the new runs
        self.format_runs.extend(new_runs);

        // Add the format run for the range
        self.format_runs.push(FormatRun::new(range, format));

        // Normalize: sort and merge adjacent runs
        self.normalize_runs();
    }

    /// Toggle a format attribute on a range.
    ///
    /// If all characters in the range have the attribute, it's removed.
    /// Otherwise, it's applied to all characters.
    pub fn toggle_format(&mut self, range: Range<usize>, toggle: CharFormat) {
        if range.is_empty() {
            return;
        }

        // Check if the entire range already has this format
        let should_remove = self.range_has_format(&range, &toggle);

        // Apply or remove the format
        self.apply_format_change(&range, &toggle, !should_remove);
    }

    /// Check if an entire range has a specific format attribute set.
    fn range_has_format(&self, range: &Range<usize>, check: &CharFormat) -> bool {
        let mut pos = range.start;

        while pos < range.end {
            let format = self.format_at(pos);

            if check.bold && !format.bold {
                return false;
            }
            if check.italic && !format.italic {
                return false;
            }
            if check.underline && !format.underline {
                return false;
            }
            if check.strikethrough && !format.strikethrough {
                return false;
            }

            // Find the next position where format might change
            let next_pos = self.next_format_change(pos, range.end);
            pos = next_pos;
        }

        true
    }

    /// Find the next byte position where format might change.
    fn next_format_change(&self, pos: usize, max: usize) -> usize {
        let mut next = max;

        for run in &self.format_runs {
            if run.range.start > pos && run.range.start < next {
                next = run.range.start;
            }
            if run.range.end > pos && run.range.end < next {
                next = run.range.end;
            }
        }

        next
    }

    /// Apply a format change to a range.
    fn apply_format_change(&mut self, range: &Range<usize>, toggle: &CharFormat, apply: bool) {
        // We need to walk through the range and update each section
        let mut pos = range.start;
        let mut sections: Vec<(Range<usize>, CharFormat)> = Vec::new();

        while pos < range.end {
            let current_format = self.format_at(pos);
            let mut new_format = current_format;

            if toggle.bold {
                new_format.bold = apply;
            }
            if toggle.italic {
                new_format.italic = apply;
            }
            if toggle.underline {
                new_format.underline = apply;
            }
            if toggle.strikethrough {
                new_format.strikethrough = apply;
            }

            let next_pos = self.next_format_change(pos, range.end);
            sections.push((pos..next_pos, new_format));
            pos = next_pos;
        }

        // Apply all the sections
        for (section_range, format) in sections {
            self.set_format(section_range, format);
        }
    }

    /// Normalize format runs: sort by position and merge adjacent runs with same format.
    fn normalize_runs(&mut self) {
        // Sort by start position
        self.format_runs.sort_by_key(|r| r.range.start);

        // Merge adjacent runs with same format
        let mut i = 0;
        while i + 1 < self.format_runs.len() {
            if self.format_runs[i].range.end == self.format_runs[i + 1].range.start
                && self.format_runs[i].format == self.format_runs[i + 1].format
            {
                self.format_runs[i].range.end = self.format_runs[i + 1].range.end;
                self.format_runs.remove(i + 1);
            } else {
                i += 1;
            }
        }

        // Remove empty runs and runs with default format
        self.format_runs
            .retain(|r| !r.is_empty() && r.format.is_styled());
    }

    /// Convert the document to styled spans for rendering.
    ///
    /// Returns a list of (text, format) pairs covering the entire document.
    pub fn to_styled_spans(&self) -> Vec<(&str, CharFormat)> {
        if self.text.is_empty() {
            return Vec::new();
        }

        let mut spans = Vec::new();
        let mut pos = 0;

        for run in &self.format_runs {
            // Add default-formatted text before this run
            if pos < run.range.start {
                spans.push((&self.text[pos..run.range.start], CharFormat::default()));
            }

            // Add the formatted run
            let end = run.range.end.min(self.text.len());
            if run.range.start < end {
                spans.push((&self.text[run.range.start..end], run.format.clone()));
            }
            pos = end;
        }

        // Add any remaining default-formatted text
        if pos < self.text.len() {
            spans.push((&self.text[pos..], CharFormat::default()));
        }

        spans
    }

    // =========================================================================
    // Paragraph/Block Formatting
    // =========================================================================

    /// Count the number of paragraphs in the document.
    ///
    /// Paragraphs are delimited by newline characters. An empty document
    /// has 1 paragraph. Each newline creates a new paragraph.
    pub fn paragraph_count(&self) -> usize {
        if self.text.is_empty() {
            return 1;
        }
        self.text.chars().filter(|&c| c == '\n').count() + 1
    }

    /// Get the byte range for a paragraph by index (0-based).
    ///
    /// Returns the byte range including the trailing newline if present.
    /// Returns `None` if the paragraph index is out of bounds.
    pub fn paragraph_range(&self, para_idx: usize) -> Option<Range<usize>> {
        let mut start = 0;
        let mut current_para = 0;

        for (i, c) in self.text.char_indices() {
            if c == '\n' {
                if current_para == para_idx {
                    return Some(start..i + 1);
                }
                current_para += 1;
                start = i + 1;
            }
        }

        // Handle the last paragraph (or only paragraph if no newlines)
        if current_para == para_idx && start <= self.text.len() {
            return Some(start..self.text.len());
        }

        None
    }

    /// Get the paragraph index for a byte position.
    pub fn paragraph_at(&self, pos: usize) -> usize {
        let pos = pos.min(self.text.len());
        self.text[..pos].chars().filter(|&c| c == '\n').count()
    }

    /// Get the block format for a paragraph.
    pub fn block_format_at(&self, para_idx: usize) -> BlockFormat {
        for run in &self.block_runs {
            if run.contains(para_idx) {
                return run.format.clone();
            }
            if run.range.start > para_idx {
                break;
            }
        }
        BlockFormat::default()
    }

    /// Get the block format for a range of paragraphs.
    ///
    /// Returns `Some(format)` if all paragraphs have the same format,
    /// or `None` if the formatting is mixed.
    pub fn block_format_for_range(&self, range: &Range<usize>) -> Option<BlockFormat> {
        if range.is_empty() {
            return Some(self.block_format_at(range.start));
        }

        let first_format = self.block_format_at(range.start);
        for para_idx in range.start + 1..range.end {
            if self.block_format_at(para_idx) != first_format {
                return None;
            }
        }
        Some(first_format)
    }

    /// Set the block format for a range of paragraphs.
    pub fn set_block_format(&mut self, range: Range<usize>, format: BlockFormat) {
        if range.is_empty() {
            return;
        }

        // Remove the default format case - just delete overlapping runs
        if !format.is_styled() {
            self.block_runs.retain(|run| !run.overlaps(&range));
            // Split runs that partially overlap
            let mut new_runs = Vec::new();
            for run in &mut self.block_runs {
                if run.range.start < range.start && run.range.end > range.end {
                    // Run spans the range - split it
                    new_runs.push(BlockRun::new(range.end..run.range.end, run.format.clone()));
                    run.range.end = range.start;
                } else if run.range.start < range.start && run.range.end > range.start {
                    // Run overlaps start
                    run.range.end = range.start;
                } else if run.range.start < range.end && run.range.end > range.end {
                    // Run overlaps end
                    run.range.start = range.end;
                }
            }
            self.block_runs.extend(new_runs);
            self.normalize_block_runs();
            return;
        }

        // Split and remove overlapping runs
        let mut new_runs = Vec::new();
        let mut to_remove = Vec::new();

        for (i, run) in self.block_runs.iter_mut().enumerate() {
            if !run.overlaps(&range) {
                continue;
            }

            if run.format == format {
                // Same format - will be merged later
                continue;
            }

            if run.range.start < range.start && run.range.end > range.end {
                // Run spans the entire range - split into three
                new_runs.push(BlockRun::new(range.end..run.range.end, run.format.clone()));
                run.range.end = range.start;
            } else if run.range.start < range.start {
                // Run overlaps start - truncate
                run.range.end = range.start;
            } else if run.range.end > range.end {
                // Run overlaps end - truncate
                run.range.start = range.end;
            } else {
                // Run is entirely within range - remove
                to_remove.push(i);
            }
        }

        // Remove fully overlapped runs
        for i in to_remove.into_iter().rev() {
            self.block_runs.remove(i);
        }

        // Add the new runs
        self.block_runs.extend(new_runs);

        // Add the block run for the range
        self.block_runs.push(BlockRun::new(range, format));

        // Normalize: sort and merge adjacent runs
        self.normalize_block_runs();
    }

    /// Set alignment for a range of paragraphs.
    pub fn set_alignment(&mut self, range: Range<usize>, alignment: HorizontalAlign) {
        self.set_block_format(range, BlockFormat::new().with_alignment(alignment));
    }

    /// Set left indent for a range of paragraphs.
    pub fn set_left_indent(&mut self, range: Range<usize>, indent: f32) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            let new_format = BlockFormat {
                left_indent: indent,
                ..existing
            };
            self.set_block_format(para_idx..para_idx + 1, new_format);
        }
    }

    /// Set first line indent for a range of paragraphs.
    pub fn set_first_line_indent(&mut self, range: Range<usize>, indent: f32) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            let new_format = BlockFormat {
                first_line_indent: indent,
                ..existing
            };
            self.set_block_format(para_idx..para_idx + 1, new_format);
        }
    }

    /// Increase left indent for a range of paragraphs by the standard step.
    pub fn increase_indent(&mut self, range: Range<usize>) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            let new_indent = existing.left_indent + BlockFormat::INDENT_STEP;
            let new_format = BlockFormat {
                left_indent: new_indent,
                ..existing
            };
            self.set_block_format(para_idx..para_idx + 1, new_format);
        }
    }

    /// Decrease left indent for a range of paragraphs by the standard step.
    /// Indent cannot go below zero.
    pub fn decrease_indent(&mut self, range: Range<usize>) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            let new_indent = (existing.left_indent - BlockFormat::INDENT_STEP).max(0.0);
            let new_format = BlockFormat {
                left_indent: new_indent,
                ..existing
            };
            self.set_block_format(para_idx..para_idx + 1, new_format);
        }
    }

    /// Set line spacing for a range of paragraphs.
    pub fn set_line_spacing(&mut self, range: Range<usize>, spacing: LineSpacing) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            let new_format = BlockFormat {
                line_spacing: spacing,
                ..existing
            };
            self.set_block_format(para_idx..para_idx + 1, new_format);
        }
    }

    /// Set spacing before a range of paragraphs.
    pub fn set_spacing_before(&mut self, range: Range<usize>, spacing: f32) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            let new_format = BlockFormat {
                spacing_before: spacing,
                ..existing
            };
            self.set_block_format(para_idx..para_idx + 1, new_format);
        }
    }

    /// Set spacing after a range of paragraphs.
    pub fn set_spacing_after(&mut self, range: Range<usize>, spacing: f32) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            let new_format = BlockFormat {
                spacing_after: spacing,
                ..existing
            };
            self.set_block_format(para_idx..para_idx + 1, new_format);
        }
    }

    /// Get the uniform left indent if all paragraphs in the document have the same indent.
    /// Returns `None` if different paragraphs have different indents.
    pub fn uniform_left_indent(&self) -> Option<f32> {
        let para_count = self.paragraph_count();
        if para_count == 0 {
            return Some(0.0);
        }

        let first_indent = self.block_format_at(0).left_indent;
        for para_idx in 1..para_count {
            if self.block_format_at(para_idx).left_indent != first_indent {
                return None;
            }
        }
        Some(first_indent)
    }

    /// Get the uniform first line indent if all paragraphs have the same.
    /// Returns `None` if different paragraphs have different first line indents.
    pub fn uniform_first_line_indent(&self) -> Option<f32> {
        let para_count = self.paragraph_count();
        if para_count == 0 {
            return Some(0.0);
        }

        let first_indent = self.block_format_at(0).first_line_indent;
        for para_idx in 1..para_count {
            if self.block_format_at(para_idx).first_line_indent != first_indent {
                return None;
            }
        }
        Some(first_indent)
    }

    /// Get the uniform line spacing if all paragraphs have the same.
    /// Returns `None` if different paragraphs have different line spacing.
    pub fn uniform_line_spacing(&self) -> Option<LineSpacing> {
        let para_count = self.paragraph_count();
        if para_count == 0 {
            return Some(LineSpacing::Single);
        }

        let first_spacing = self.block_format_at(0).line_spacing;
        for para_idx in 1..para_count {
            if self.block_format_at(para_idx).line_spacing != first_spacing {
                return None;
            }
        }
        Some(first_spacing)
    }

    /// Get the uniform spacing before if all paragraphs have the same.
    /// Returns `None` if different paragraphs have different spacing before.
    pub fn uniform_spacing_before(&self) -> Option<f32> {
        let para_count = self.paragraph_count();
        if para_count == 0 {
            return Some(0.0);
        }

        let first_spacing = self.block_format_at(0).spacing_before;
        for para_idx in 1..para_count {
            if self.block_format_at(para_idx).spacing_before != first_spacing {
                return None;
            }
        }
        Some(first_spacing)
    }

    /// Get the uniform spacing after if all paragraphs have the same.
    /// Returns `None` if different paragraphs have different spacing after.
    pub fn uniform_spacing_after(&self) -> Option<f32> {
        let para_count = self.paragraph_count();
        if para_count == 0 {
            return Some(0.0);
        }

        let first_spacing = self.block_format_at(0).spacing_after;
        for para_idx in 1..para_count {
            if self.block_format_at(para_idx).spacing_after != first_spacing {
                return None;
            }
        }
        Some(first_spacing)
    }

    // =========================================================================
    // List Formatting
    // =========================================================================

    /// Get the list format for a paragraph.
    /// Returns `None` if the paragraph is not a list item.
    pub fn list_format_at(&self, para_idx: usize) -> Option<ListFormat> {
        self.block_format_at(para_idx).list_format
    }

    /// Check if a paragraph is a list item.
    pub fn is_list_item(&self, para_idx: usize) -> bool {
        self.block_format_at(para_idx).list_format.is_some()
    }

    /// Set the list format for a range of paragraphs.
    /// Pass `None` to remove list formatting.
    pub fn set_list_format(&mut self, range: Range<usize>, list_format: Option<ListFormat>) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            let new_format = BlockFormat {
                list_format: list_format.clone(),
                ..existing
            };
            self.set_block_format(para_idx..para_idx + 1, new_format);
        }
    }

    /// Toggle bullet list on a range of paragraphs.
    /// If all paragraphs are bullet list items, removes the list.
    /// Otherwise, makes all paragraphs bullet list items.
    pub fn toggle_bullet_list(&mut self, range: Range<usize>) {
        let all_are_bullet_lists = range.clone().all(|para_idx| {
            self.block_format_at(para_idx)
                .list_format
                .as_ref()
                .map_or(false, |lf| lf.style.is_bullet())
        });

        if all_are_bullet_lists {
            // Remove list formatting
            self.set_list_format(range, None);
        } else {
            // Add bullet list formatting
            for para_idx in range {
                let existing = self.block_format_at(para_idx);
                let indent_level = existing.list_format.as_ref().map_or(0, |lf| lf.indent_level);
                let list_format = ListFormat::new(ListStyle::bullet_for_level(indent_level))
                    .with_indent_level(indent_level);
                let new_format = BlockFormat {
                    list_format: Some(list_format),
                    ..existing
                };
                self.set_block_format(para_idx..para_idx + 1, new_format);
            }
        }
    }

    /// Toggle numbered list on a range of paragraphs.
    /// If all paragraphs are numbered list items, removes the list.
    /// Otherwise, makes all paragraphs numbered list items.
    pub fn toggle_numbered_list(&mut self, range: Range<usize>) {
        let all_are_numbered_lists = range.clone().all(|para_idx| {
            self.block_format_at(para_idx)
                .list_format
                .as_ref()
                .map_or(false, |lf| lf.style.is_numbered())
        });

        if all_are_numbered_lists {
            // Remove list formatting
            self.set_list_format(range, None);
        } else {
            // Add numbered list formatting
            for para_idx in range {
                let existing = self.block_format_at(para_idx);
                let indent_level = existing.list_format.as_ref().map_or(0, |lf| lf.indent_level);
                let list_format = ListFormat::new(ListStyle::number_for_level(indent_level))
                    .with_indent_level(indent_level);
                let new_format = BlockFormat {
                    list_format: Some(list_format),
                    ..existing
                };
                self.set_block_format(para_idx..para_idx + 1, new_format);
            }
        }
    }

    /// Increase the indent level for list items in a range.
    /// Non-list paragraphs are not affected.
    pub fn increase_list_indent(&mut self, range: Range<usize>) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            if let Some(mut list_format) = existing.list_format.clone() {
                list_format.indent_level += 1;
                // Update the style for the new indent level
                if list_format.style.is_bullet() {
                    list_format.style = ListStyle::bullet_for_level(list_format.indent_level);
                } else {
                    list_format.style = ListStyle::number_for_level(list_format.indent_level);
                }
                let new_format = BlockFormat {
                    list_format: Some(list_format),
                    ..existing
                };
                self.set_block_format(para_idx..para_idx + 1, new_format);
            }
        }
    }

    /// Decrease the indent level for list items in a range.
    /// Non-list paragraphs are not affected.
    /// If indent level is already 0, the paragraph stays at level 0.
    pub fn decrease_list_indent(&mut self, range: Range<usize>) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            if let Some(mut list_format) = existing.list_format.clone() {
                if list_format.indent_level > 0 {
                    list_format.indent_level -= 1;
                    // Update the style for the new indent level
                    if list_format.style.is_bullet() {
                        list_format.style = ListStyle::bullet_for_level(list_format.indent_level);
                    } else {
                        list_format.style = ListStyle::number_for_level(list_format.indent_level);
                    }
                    let new_format = BlockFormat {
                        list_format: Some(list_format),
                        ..existing
                    };
                    self.set_block_format(para_idx..para_idx + 1, new_format);
                }
            }
        }
    }

    /// Set the list style for a range of list items.
    /// Non-list paragraphs are not affected.
    pub fn set_list_style(&mut self, range: Range<usize>, style: ListStyle) {
        for para_idx in range {
            let existing = self.block_format_at(para_idx);
            if let Some(mut list_format) = existing.list_format.clone() {
                list_format.style = style;
                let new_format = BlockFormat {
                    list_format: Some(list_format),
                    ..existing
                };
                self.set_block_format(para_idx..para_idx + 1, new_format);
            }
        }
    }

    /// Get the list item number for a paragraph within its list context.
    /// This counts the items at the same indent level preceding this paragraph.
    /// Returns 0 for non-list items or the first item in a list sequence.
    pub fn list_item_number(&self, para_idx: usize) -> usize {
        let format = self.block_format_at(para_idx);
        let Some(list_format) = &format.list_format else {
            return 0;
        };

        // Count backwards to find how many items at the same level precede this one
        let mut count = 0;
        let target_level = list_format.indent_level;

        for idx in (0..para_idx).rev() {
            let prev_format = self.block_format_at(idx);
            match &prev_format.list_format {
                Some(prev_list) if prev_list.indent_level == target_level => {
                    // Same level - count it
                    count += 1;
                }
                Some(prev_list) if prev_list.indent_level < target_level => {
                    // Higher level (less indented) - this is a parent, stop counting
                    break;
                }
                None => {
                    // Not a list item - break the sequence
                    break;
                }
                _ => {
                    // More indented - skip but continue looking
                }
            }
        }

        count
    }

    /// Normalize block runs: sort by position and merge adjacent runs with same format.
    fn normalize_block_runs(&mut self) {
        // Sort by start position
        self.block_runs.sort_by_key(|r| r.range.start);

        // Merge adjacent runs with same format
        let mut i = 0;
        while i + 1 < self.block_runs.len() {
            if self.block_runs[i].range.end == self.block_runs[i + 1].range.start
                && self.block_runs[i].format == self.block_runs[i + 1].format
            {
                self.block_runs[i].range.end = self.block_runs[i + 1].range.end;
                self.block_runs.remove(i + 1);
            } else {
                i += 1;
            }
        }

        // Remove empty runs and runs with default format
        self.block_runs
            .retain(|r| !r.is_empty() && r.format.is_styled());
    }

    /// Get all paragraphs with their text, char format spans, and block format.
    ///
    /// Returns a vector of (paragraph_text, char_format_spans, block_format) tuples.
    /// This is useful for rendering with per-paragraph alignment.
    pub fn to_paragraphs(&self) -> Vec<(String, Vec<(Range<usize>, CharFormat)>, BlockFormat)> {
        let mut paragraphs = Vec::new();
        let para_count = self.paragraph_count();

        for para_idx in 0..para_count {
            let Some(para_range) = self.paragraph_range(para_idx) else {
                continue;
            };

            // Get paragraph text (without trailing newline for rendering)
            let para_text = if self.text[para_range.clone()].ends_with('\n') {
                self.text[para_range.start..para_range.end - 1].to_string()
            } else {
                self.text[para_range.clone()].to_string()
            };

            // Get char format spans for this paragraph
            let mut char_spans = Vec::new();
            let para_start = para_range.start;
            let para_end = if self.text[para_range.clone()].ends_with('\n') {
                para_range.end - 1
            } else {
                para_range.end
            };

            // Convert format runs to local paragraph offsets
            for run in &self.format_runs {
                if run.range.end <= para_start || run.range.start >= para_end {
                    continue;
                }
                let local_start = run.range.start.saturating_sub(para_start).min(para_text.len());
                let local_end = run.range.end.saturating_sub(para_start).min(para_text.len());
                if local_start < local_end {
                    char_spans.push((local_start..local_end, run.format.clone()));
                }
            }

            let block_format = self.block_format_at(para_idx);
            paragraphs.push((para_text, char_spans, block_format));
        }

        paragraphs
    }

    // =========================================================================
    // HTML Serialization
    // =========================================================================

    /// Convert the document to an HTML string.
    ///
    /// The generated HTML uses:
    /// - `<p>` for paragraphs with inline styles for alignment and indentation
    /// - `<ul>`, `<ol>`, `<li>` for lists with proper nesting
    /// - `<b>`, `<i>`, `<u>`, `<s>` for basic character formatting
    /// - `<span style="...">` for colors, fonts, and sizes
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut doc = StyledDocument::from_text("Hello world");
    /// doc.set_format(0..5, CharFormat::bold());
    /// let html = doc.to_html();
    /// // Results in: "<p><b>Hello</b> world</p>"
    /// ```
    pub fn to_html(&self) -> String {
        let mut html = String::new();
        let paragraphs = self.to_paragraphs();

        // Track list state for proper nesting
        let mut list_stack: Vec<(bool, usize)> = Vec::new(); // (is_ordered, indent_level)

        for (_para_idx, (text, char_spans, block_format)) in paragraphs.iter().enumerate() {
            let is_list_item = block_format.list_format.is_some();
            let list_format = block_format.list_format.as_ref();

            if is_list_item {
                let list_info = list_format.unwrap();
                let is_ordered = list_info.style.is_numbered();
                let indent_level = list_info.indent_level;

                // Close lists that are deeper than current level
                while let Some(&(_, stack_level)) = list_stack.last() {
                    if stack_level > indent_level {
                        let (was_ordered, _) = list_stack.pop().unwrap();
                        html.push_str(if was_ordered { "</ol>" } else { "</ul>" });
                    } else {
                        break;
                    }
                }

                // Check if we need to change list type at current level
                if let Some(&(stack_ordered, stack_level)) = list_stack.last() {
                    if stack_level == indent_level && stack_ordered != is_ordered {
                        // Close and reopen with different type
                        let (was_ordered, _) = list_stack.pop().unwrap();
                        html.push_str(if was_ordered { "</ol>" } else { "</ul>" });
                        html.push_str(if is_ordered { "<ol>" } else { "<ul>" });
                        list_stack.push((is_ordered, indent_level));
                    }
                }

                // Open new lists as needed
                while list_stack.len() <= indent_level {
                    let target_level = list_stack.len();
                    html.push_str(if is_ordered { "<ol>" } else { "<ul>" });
                    list_stack.push((is_ordered, target_level));
                }

                // Write the list item
                html.push_str("<li>");
                self.write_formatted_text(&mut html, text, char_spans);
                html.push_str("</li>");
            } else {
                // Close all open lists before non-list paragraph
                while let Some((was_ordered, _)) = list_stack.pop() {
                    html.push_str(if was_ordered { "</ol>" } else { "</ul>" });
                }

                // Write paragraph
                html.push_str("<p");
                self.write_paragraph_style(&mut html, block_format);
                html.push('>');
                self.write_formatted_text(&mut html, text, char_spans);
                html.push_str("</p>");
            }
        }

        // Close any remaining open lists
        while let Some((was_ordered, _)) = list_stack.pop() {
            html.push_str(if was_ordered { "</ol>" } else { "</ul>" });
        }

        html
    }

    /// Write paragraph style attributes.
    fn write_paragraph_style(&self, html: &mut String, format: &BlockFormat) {
        let mut styles = Vec::new();

        // Alignment
        match format.alignment {
            HorizontalAlign::Center => styles.push("text-align:center".to_string()),
            HorizontalAlign::Right => styles.push("text-align:right".to_string()),
            HorizontalAlign::Justified => styles.push("text-align:justify".to_string()),
            HorizontalAlign::Left => {} // Default, no style needed
        }

        // Indentation
        if format.left_indent > 0.0 {
            styles.push(format!("margin-left:{}px", format.left_indent as i32));
        }
        if format.first_line_indent != 0.0 {
            styles.push(format!("text-indent:{}px", format.first_line_indent as i32));
        }

        // Spacing
        if format.spacing_before > 0.0 {
            styles.push(format!("margin-top:{}px", format.spacing_before as i32));
        }
        if format.spacing_after > 0.0 {
            styles.push(format!("margin-bottom:{}px", format.spacing_after as i32));
        }

        // Line spacing
        match format.line_spacing {
            LineSpacing::Single => {} // Default
            LineSpacing::OnePointFive => styles.push("line-height:1.5".to_string()),
            LineSpacing::Double => styles.push("line-height:2.0".to_string()),
            LineSpacing::Custom(m) => styles.push(format!("line-height:{}", m)),
        }

        if !styles.is_empty() {
            html.push_str(" style=\"");
            html.push_str(&styles.join(";"));
            html.push('"');
        }
    }

    /// Write formatted text with character formatting tags.
    fn write_formatted_text(
        &self,
        html: &mut String,
        text: &str,
        char_spans: &[(Range<usize>, CharFormat)],
    ) {
        if text.is_empty() {
            return;
        }

        // Build a list of format changes at each position
        let mut positions: Vec<usize> = vec![0, text.len()];
        for (range, _) in char_spans {
            if range.start < text.len() {
                positions.push(range.start);
            }
            if range.end <= text.len() {
                positions.push(range.end);
            }
        }
        positions.sort();
        positions.dedup();

        // Write each segment with its format
        for window in positions.windows(2) {
            let start = window[0];
            let end = window[1];
            if start >= end || start >= text.len() {
                continue;
            }
            let end = end.min(text.len());
            let segment = &text[start..end];

            // Find the format for this segment
            let format = char_spans
                .iter()
                .find(|(range, _)| range.start <= start && range.end >= end)
                .map(|(_, f)| f.clone())
                .unwrap_or_default();

            self.write_formatted_segment(html, segment, &format);
        }
    }

    /// Write a single text segment with its formatting.
    fn write_formatted_segment(&self, html: &mut String, text: &str, format: &CharFormat) {
        // Collect opening and closing tags
        let mut open_tags = Vec::new();

        if format.bold {
            open_tags.push("<b>");
        }
        if format.italic {
            open_tags.push("<i>");
        }
        if format.underline {
            open_tags.push("<u>");
        }
        if format.strikethrough {
            open_tags.push("<s>");
        }

        // Check if we need a span for additional styling
        let mut span_styles = Vec::new();

        if let Some(color) = &format.foreground_color {
            let r = (color.r * 255.0) as u8;
            let g = (color.g * 255.0) as u8;
            let b = (color.b * 255.0) as u8;
            span_styles.push(format!("color:#{:02x}{:02x}{:02x}", r, g, b));
        }
        if let Some(color) = &format.background_color {
            let r = (color.r * 255.0) as u8;
            let g = (color.g * 255.0) as u8;
            let b = (color.b * 255.0) as u8;
            span_styles.push(format!("background-color:#{:02x}{:02x}{:02x}", r, g, b));
        }
        if let Some(size) = format.font_size {
            span_styles.push(format!("font-size:{}px", size as i32));
        }
        if let Some(family) = &format.font_family {
            let family_name = match family {
                FontFamily::SansSerif => "sans-serif",
                FontFamily::Serif => "serif",
                FontFamily::Monospace => "monospace",
                FontFamily::Cursive => "cursive",
                FontFamily::Fantasy => "fantasy",
                FontFamily::Name(name) => name.as_str(),
            };
            span_styles.push(format!("font-family:{}", family_name));
        }
        if let Some(weight) = &format.font_weight {
            span_styles.push(format!("font-weight:{}", weight.0));
        }

        // Write opening tags
        for tag in &open_tags {
            html.push_str(tag);
        }

        // Write span if needed
        let has_span = !span_styles.is_empty();
        if has_span {
            html.push_str("<span style=\"");
            html.push_str(&span_styles.join(";"));
            html.push_str("\">");
        }

        // Write escaped text
        html.push_str(&html_escape(text));

        // Write closing span
        if has_span {
            html.push_str("</span>");
        }

        // Write closing tags in reverse order
        for tag in open_tags.iter().rev() {
            let close_tag = match *tag {
                "<b>" => "</b>",
                "<i>" => "</i>",
                "<u>" => "</u>",
                "<s>" => "</s>",
                _ => continue,
            };
            html.push_str(close_tag);
        }
    }

    /// Parse HTML and create a StyledDocument.
    ///
    /// Supports the following HTML elements:
    /// - `<p>` paragraphs with style attributes
    /// - `<ul>`, `<ol>`, `<li>` lists
    /// - `<b>`, `<strong>` bold
    /// - `<i>`, `<em>` italic
    /// - `<u>` underline
    /// - `<s>`, `<del>`, `<strike>` strikethrough
    /// - `<br>` line breaks
    /// - `<span style="...">` inline styles
    /// - `<font color="..." size="...">` legacy font styling
    ///
    /// # Example
    ///
    /// ```ignore
    /// let doc = StyledDocument::from_html("<p><b>Hello</b> world</p>");
    /// assert_eq!(doc.text(), "Hello world");
    /// ```
    pub fn from_html(html: &str) -> Self {
        HtmlDocumentParser::parse(html)
    }

    /// Convert a range of the document to an HTML string.
    ///
    /// This extracts the specified byte range with all its formatting and
    /// converts it to HTML. Useful for clipboard operations where only
    /// the selected text needs to be exported.
    ///
    /// # Arguments
    ///
    /// * `range` - The byte range to export (start..end)
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut doc = StyledDocument::from_text("Hello world");
    /// doc.set_format(0..5, CharFormat::bold());
    /// let html = doc.range_to_html(0..5);
    /// // Results in: "<p><b>Hello</b></p>"
    /// ```
    pub fn range_to_html(&self, range: std::ops::Range<usize>) -> String {
        if range.start >= range.end || range.start >= self.text.len() {
            return String::new();
        }

        let start = range.start.min(self.text.len());
        let end = range.end.min(self.text.len());

        // Find which paragraphs intersect with the range
        let para_count = self.paragraph_count();
        let mut html = String::new();
        let mut list_stack: Vec<(bool, usize)> = Vec::new();

        for para_idx in 0..para_count {
            let Some(para_range) = self.paragraph_range(para_idx) else {
                continue;
            };

            // Skip paragraphs that don't intersect with our range
            if para_range.end <= start || para_range.start >= end {
                continue;
            }

            // Calculate the intersection
            let intersect_start = para_range.start.max(start);
            let intersect_end = para_range.end.min(end);

            // Get the text within the intersection (excluding trailing newline if at para end)
            let para_text = if intersect_end == para_range.end
                && self.text[para_range.clone()].ends_with('\n')
            {
                let adjusted_end = (intersect_end - 1).max(intersect_start);
                &self.text[intersect_start..adjusted_end]
            } else {
                &self.text[intersect_start..intersect_end]
            };

            if para_text.is_empty() && intersect_start >= intersect_end {
                continue;
            }

            // Get char format spans for this intersection, adjusted to local offsets
            let mut char_spans = Vec::new();
            for run in &self.format_runs {
                if run.range.end <= intersect_start || run.range.start >= intersect_end {
                    continue;
                }
                let local_start = run.range.start.saturating_sub(intersect_start);
                let local_end = (run.range.end - intersect_start).min(para_text.len());
                if local_start < local_end {
                    char_spans.push((local_start..local_end, run.format.clone()));
                }
            }

            let block_format = self.block_format_at(para_idx);
            let is_list_item = block_format.list_format.is_some();
            let list_format = block_format.list_format.as_ref();

            if is_list_item {
                let list_info = list_format.unwrap();
                let is_ordered = list_info.style.is_numbered();
                let indent_level = list_info.indent_level;

                // Close lists that are deeper than current level
                while let Some(&(_, stack_level)) = list_stack.last() {
                    if stack_level > indent_level {
                        let (was_ordered, _) = list_stack.pop().unwrap();
                        html.push_str(if was_ordered { "</ol>" } else { "</ul>" });
                    } else {
                        break;
                    }
                }

                // Check if we need to change list type at current level
                if let Some(&(stack_ordered, stack_level)) = list_stack.last() {
                    if stack_level == indent_level && stack_ordered != is_ordered {
                        let (was_ordered, _) = list_stack.pop().unwrap();
                        html.push_str(if was_ordered { "</ol>" } else { "</ul>" });
                        html.push_str(if is_ordered { "<ol>" } else { "<ul>" });
                        list_stack.push((is_ordered, indent_level));
                    }
                }

                // Open new lists as needed
                while list_stack.len() <= indent_level {
                    let target_level = list_stack.len();
                    html.push_str(if is_ordered { "<ol>" } else { "<ul>" });
                    list_stack.push((is_ordered, target_level));
                }

                // Write the list item
                html.push_str("<li>");
                self.write_formatted_text(&mut html, para_text, &char_spans);
                html.push_str("</li>");
            } else {
                // Close all open lists before non-list paragraph
                while let Some((was_ordered, _)) = list_stack.pop() {
                    html.push_str(if was_ordered { "</ol>" } else { "</ul>" });
                }

                // Write paragraph
                html.push_str("<p");
                self.write_paragraph_style(&mut html, &block_format);
                html.push('>');
                self.write_formatted_text(&mut html, para_text, &char_spans);
                html.push_str("</p>");
            }
        }

        // Close any remaining open lists
        while let Some((was_ordered, _)) = list_stack.pop() {
            html.push_str(if was_ordered { "</ol>" } else { "</ul>" });
        }

        html
    }
}

/// Escape special HTML characters.
fn html_escape(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '&' => result.push_str("&amp;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(c),
        }
    }
    result
}

/// State for tracking format during HTML parsing.
#[derive(Debug, Clone, Default)]
struct HtmlFormatState {
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    foreground_color: Option<Color>,
    background_color: Option<Color>,
    font_family: Option<FontFamily>,
    font_size: Option<f32>,
    font_weight: Option<FontWeight>,
}

impl HtmlFormatState {
    fn to_char_format(&self) -> CharFormat {
        CharFormat {
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            strikethrough: self.strikethrough,
            foreground_color: self.foreground_color.clone(),
            background_color: self.background_color.clone(),
            font_family: self.font_family.clone(),
            font_size: self.font_size,
            font_weight: self.font_weight.clone(),
        }
    }
}

/// Parser state for block-level elements.
#[derive(Debug, Clone, Default)]
struct HtmlBlockState {
    alignment: HorizontalAlign,
    left_indent: f32,
    first_line_indent: f32,
    line_spacing: LineSpacing,
    spacing_before: f32,
    spacing_after: f32,
    list_format: Option<ListFormat>,
}

impl HtmlBlockState {
    fn to_block_format(&self) -> BlockFormat {
        BlockFormat {
            alignment: self.alignment,
            left_indent: self.left_indent,
            first_line_indent: self.first_line_indent,
            line_spacing: self.line_spacing,
            spacing_before: self.spacing_before,
            spacing_after: self.spacing_after,
            list_format: self.list_format.clone(),
        }
    }
}

/// HTML parser for StyledDocument.
struct HtmlDocumentParser {
    text: String,
    format_runs: Vec<FormatRun>,
    block_runs: Vec<BlockRun>,
    format_stack: Vec<HtmlFormatState>,
    list_stack: Vec<(bool, usize)>, // (is_ordered, indent_level)
    current_block: HtmlBlockState,
    paragraph_start: usize,
    current_paragraph: usize,
}

impl HtmlDocumentParser {
    fn new() -> Self {
        Self {
            text: String::new(),
            format_runs: Vec::new(),
            block_runs: Vec::new(),
            format_stack: vec![HtmlFormatState::default()],
            list_stack: Vec::new(),
            current_block: HtmlBlockState::default(),
            paragraph_start: 0,
            current_paragraph: 0,
        }
    }

    fn current_format(&self) -> &HtmlFormatState {
        self.format_stack.last().expect("format_stack should never be empty")
    }

    fn parse(html: &str) -> StyledDocument {
        let mut parser = HtmlDocumentParser::new();
        let mut chars = html.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '<' {
                // Parse tag
                let mut tag_content = String::new();
                while let Some(&tc) = chars.peek() {
                    if tc == '>' {
                        chars.next();
                        break;
                    }
                    tag_content.push(chars.next().unwrap());
                }
                parser.handle_tag(&tag_content);
            } else if c == '&' {
                // Parse HTML entity
                let mut entity = String::new();
                while let Some(&ec) = chars.peek() {
                    if ec == ';' {
                        chars.next();
                        break;
                    }
                    if ec == '<' || ec == ' ' || entity.len() > 10 {
                        // Not a valid entity, treat as literal
                        parser.add_text("&");
                        parser.add_text(&entity);
                        entity.clear();
                        break;
                    }
                    entity.push(chars.next().unwrap());
                }
                if !entity.is_empty() {
                    parser.add_text(&decode_html_entity(&entity));
                }
            } else {
                parser.add_text(&c.to_string());
            }
        }

        // Finalize any pending paragraph
        parser.finalize_paragraph();

        // Normalize the document
        let mut doc = StyledDocument {
            text: parser.text,
            format_runs: parser.format_runs,
            block_runs: parser.block_runs,
        };
        doc.normalize_runs();
        doc.normalize_block_runs();

        doc
    }

    fn add_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        let start = self.text.len();
        self.text.push_str(text);
        let end = self.text.len();

        // Add format run if styled
        let format = self.current_format().to_char_format();
        if format.is_styled() {
            self.format_runs.push(FormatRun::new(start..end, format));
        }
    }

    fn finalize_paragraph(&mut self) {
        if self.text.len() > self.paragraph_start || self.current_block.list_format.is_some() {
            // Add block format if styled
            let block_format = self.current_block.to_block_format();
            if block_format.is_styled() {
                self.block_runs.push(BlockRun::new(
                    self.current_paragraph..self.current_paragraph + 1,
                    block_format,
                ));
            }
        }
        self.paragraph_start = self.text.len();
        self.current_block = HtmlBlockState::default();
    }

    fn start_new_paragraph(&mut self) {
        // Add newline if not at start and not already ending with newline
        if !self.text.is_empty() && !self.text.ends_with('\n') {
            self.text.push('\n');
            self.current_paragraph += 1;
        }
        self.finalize_paragraph();
    }

    fn handle_tag(&mut self, tag_content: &str) {
        let tag_content = tag_content.trim();

        // Check for self-closing
        let is_self_closing = tag_content.ends_with('/');
        let tag_content = tag_content.trim_end_matches('/').trim();

        // Check for closing tag
        let is_closing = tag_content.starts_with('/');
        let tag_content = tag_content.trim_start_matches('/').trim();

        // Extract tag name and attributes
        let (tag_name, attrs_str) = match tag_content.find(|c: char| c.is_whitespace()) {
            Some(idx) => (&tag_content[..idx], tag_content[idx..].trim()),
            None => (tag_content, ""),
        };
        let tag_name = tag_name.to_lowercase();

        if is_closing {
            self.handle_closing_tag(&tag_name);
        } else {
            let attrs = parse_html_attributes(attrs_str);
            self.handle_opening_tag(&tag_name, &attrs, is_self_closing);
        }
    }

    fn handle_opening_tag(
        &mut self,
        tag_name: &str,
        attrs: &[(String, String)],
        is_self_closing: bool,
    ) {
        match tag_name {
            "b" | "strong" => {
                self.push_format(|f| f.bold = true);
            }
            "i" | "em" => {
                self.push_format(|f| f.italic = true);
            }
            "u" => {
                self.push_format(|f| f.underline = true);
            }
            "s" | "del" | "strike" => {
                self.push_format(|f| f.strikethrough = true);
            }
            "br" => {
                self.text.push('\n');
                self.current_paragraph += 1;
            }
            "p" | "div" => {
                self.start_new_paragraph();
                self.parse_block_style(attrs);
                if !is_self_closing {
                    // Push a dummy format for pairing with close tag
                    let current = self.current_format().clone();
                    self.format_stack.push(current);
                }
            }
            "ul" => {
                let indent_level = self.list_stack.len();
                self.list_stack.push((false, indent_level));
            }
            "ol" => {
                let indent_level = self.list_stack.len();
                self.list_stack.push((true, indent_level));
            }
            "li" => {
                self.start_new_paragraph();
                if let Some(&(is_ordered, _)) = self.list_stack.last() {
                    let indent_level = self.list_stack.len().saturating_sub(1);
                    let style = if is_ordered {
                        ListStyle::number_for_level(indent_level)
                    } else {
                        ListStyle::bullet_for_level(indent_level)
                    };
                    self.current_block.list_format = Some(ListFormat {
                        style,
                        indent_level,
                        start: 1,
                    });
                }
                // Push dummy format for close tag
                let current = self.current_format().clone();
                self.format_stack.push(current);
            }
            "span" => {
                self.push_format(|_| {});
                self.parse_inline_style(attrs);
            }
            "font" => {
                let (size, color) = parse_font_tag_attrs(attrs);
                self.push_format(|f| {
                    if let Some(s) = size {
                        f.font_size = Some(s);
                    }
                    if let Some(c) = color {
                        f.foreground_color = Some(c);
                    }
                });
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.start_new_paragraph();
                let size = match tag_name {
                    "h1" => 32.0,
                    "h2" => 24.0,
                    "h3" => 18.0,
                    "h4" => 16.0,
                    "h5" => 14.0,
                    "h6" => 12.0,
                    _ => 16.0,
                };
                self.push_format(|f| {
                    f.bold = true;
                    f.font_size = Some(size);
                });
            }
            _ => {
                // Unknown tag - push dummy format to balance stack
                if !is_self_closing {
                    let current = self.current_format().clone();
                    self.format_stack.push(current);
                }
            }
        }
    }

    fn handle_closing_tag(&mut self, tag_name: &str) {
        match tag_name {
            "b" | "strong" | "i" | "em" | "u" | "s" | "del" | "strike" | "span" | "font" => {
                self.pop_format();
            }
            "p" | "div" | "li" => {
                self.finalize_paragraph();
                self.pop_format();
            }
            "ul" | "ol" => {
                self.list_stack.pop();
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.finalize_paragraph();
                self.pop_format();
            }
            _ => {
                self.pop_format();
            }
        }
    }

    fn push_format(&mut self, modifier: impl FnOnce(&mut HtmlFormatState)) {
        let mut new_format = self.current_format().clone();
        modifier(&mut new_format);
        self.format_stack.push(new_format);
    }

    fn pop_format(&mut self) {
        if self.format_stack.len() > 1 {
            self.format_stack.pop();
        }
    }

    fn parse_block_style(&mut self, attrs: &[(String, String)]) {
        for (key, value) in attrs {
            if key == "style" {
                self.parse_style_attribute(value, true);
            } else if key == "align" {
                self.current_block.alignment = match value.to_lowercase().as_str() {
                    "center" => HorizontalAlign::Center,
                    "right" => HorizontalAlign::Right,
                    "justify" => HorizontalAlign::Justified,
                    _ => HorizontalAlign::Left,
                };
            }
        }
    }

    fn parse_inline_style(&mut self, attrs: &[(String, String)]) {
        for (key, value) in attrs {
            if key == "style" {
                self.parse_style_attribute(value, false);
            }
        }
    }

    fn parse_style_attribute(&mut self, style: &str, is_block: bool) {
        for declaration in style.split(';') {
            let declaration = declaration.trim();
            if declaration.is_empty() {
                continue;
            }

            let parts: Vec<&str> = declaration.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }

            let prop = parts[0].trim().to_lowercase();
            let value = parts[1].trim();

            match prop.as_str() {
                // Block-level styles
                "text-align" if is_block => {
                    self.current_block.alignment = match value.to_lowercase().as_str() {
                        "center" => HorizontalAlign::Center,
                        "right" => HorizontalAlign::Right,
                        "justify" => HorizontalAlign::Justified,
                        _ => HorizontalAlign::Left,
                    };
                }
                "margin-left" if is_block => {
                    if let Some(px) = parse_px_value(value) {
                        self.current_block.left_indent = px;
                    }
                }
                "text-indent" if is_block => {
                    if let Some(px) = parse_px_value(value) {
                        self.current_block.first_line_indent = px;
                    }
                }
                "margin-top" if is_block => {
                    if let Some(px) = parse_px_value(value) {
                        self.current_block.spacing_before = px;
                    }
                }
                "margin-bottom" if is_block => {
                    if let Some(px) = parse_px_value(value) {
                        self.current_block.spacing_after = px;
                    }
                }
                "line-height" if is_block => {
                    if let Ok(multiplier) = value.parse::<f32>() {
                        self.current_block.line_spacing = if (multiplier - 1.2).abs() < 0.1 {
                            LineSpacing::Single
                        } else if (multiplier - 1.5).abs() < 0.1 {
                            LineSpacing::OnePointFive
                        } else if (multiplier - 2.0).abs() < 0.1 {
                            LineSpacing::Double
                        } else {
                            LineSpacing::Custom(multiplier)
                        };
                    }
                }
                // Inline styles
                "color" => {
                    if let Some(color) = parse_css_color(value) {
                        if let Some(f) = self.format_stack.last_mut() {
                            f.foreground_color = Some(color);
                        }
                    }
                }
                "background-color" => {
                    if let Some(color) = parse_css_color(value) {
                        if let Some(f) = self.format_stack.last_mut() {
                            f.background_color = Some(color);
                        }
                    }
                }
                "font-size" => {
                    if let Some(px) = parse_px_value(value) {
                        if let Some(f) = self.format_stack.last_mut() {
                            f.font_size = Some(px);
                        }
                    }
                }
                "font-weight" => {
                    if let Some(f) = self.format_stack.last_mut() {
                        if value == "bold" || value == "700" || value == "800" || value == "900" {
                            f.bold = true;
                        } else if let Ok(weight) = value.parse::<u16>() {
                            f.font_weight = Some(FontWeight(weight));
                        }
                    }
                }
                "font-style" => {
                    if let Some(f) = self.format_stack.last_mut() {
                        if value == "italic" || value == "oblique" {
                            f.italic = true;
                        }
                    }
                }
                "text-decoration" => {
                    if let Some(f) = self.format_stack.last_mut() {
                        if value.contains("underline") {
                            f.underline = true;
                        }
                        if value.contains("line-through") {
                            f.strikethrough = true;
                        }
                    }
                }
                "font-family" => {
                    if let Some(f) = self.format_stack.last_mut() {
                        let family = value.trim_matches(|c| c == '"' || c == '\'');
                        f.font_family = Some(match family.to_lowercase().as_str() {
                            "sans-serif" => FontFamily::SansSerif,
                            "serif" => FontFamily::Serif,
                            "monospace" => FontFamily::Monospace,
                            "cursive" => FontFamily::Cursive,
                            "fantasy" => FontFamily::Fantasy,
                            _ => FontFamily::name(family),
                        });
                    }
                }
                _ => {}
            }
        }
    }
}

/// Parse HTML attributes from a string.
fn parse_html_attributes(attrs_str: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut chars = attrs_str.chars().peekable();

    while chars.peek().is_some() {
        // Skip whitespace
        while chars.peek().map_or(false, |c| c.is_whitespace()) {
            chars.next();
        }

        // Parse key
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(chars.next().unwrap());
        }

        if key.is_empty() {
            break;
        }

        // Skip whitespace before =
        while chars.peek().map_or(false, |c| c.is_whitespace()) {
            chars.next();
        }

        // Check for =
        if chars.peek() != Some(&'=') {
            continue;
        }
        chars.next();

        // Skip whitespace after =
        while chars.peek().map_or(false, |c| c.is_whitespace()) {
            chars.next();
        }

        // Parse value
        let mut value = String::new();
        let quote_char = chars.peek().copied();

        if quote_char == Some('"') || quote_char == Some('\'') {
            chars.next();
            let quote = quote_char.unwrap();
            while let Some(c) = chars.next() {
                if c == quote {
                    break;
                }
                value.push(c);
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() {
                    break;
                }
                value.push(chars.next().unwrap());
            }
        }

        result.push((key.to_lowercase(), value));
    }

    result
}

/// Decode common HTML entities.
fn decode_html_entity(entity: &str) -> String {
    match entity {
        "lt" => "<".to_string(),
        "gt" => ">".to_string(),
        "amp" => "&".to_string(),
        "quot" => "\"".to_string(),
        "apos" => "'".to_string(),
        "nbsp" => "\u{00A0}".to_string(),
        "ndash" => "–".to_string(),
        "mdash" => "—".to_string(),
        "copy" => "©".to_string(),
        "reg" => "®".to_string(),
        "trade" => "™".to_string(),
        "hellip" => "…".to_string(),
        _ => {
            // Try numeric entity
            if let Some(hex) = entity.strip_prefix('#') {
                if let Some(hex_val) = hex.strip_prefix('x').or_else(|| hex.strip_prefix('X')) {
                    if let Ok(code_point) = u32::from_str_radix(hex_val, 16) {
                        if let Some(c) = char::from_u32(code_point) {
                            return c.to_string();
                        }
                    }
                } else if let Ok(code_point) = hex.parse::<u32>() {
                    if let Some(c) = char::from_u32(code_point) {
                        return c.to_string();
                    }
                }
            }
            format!("&{};", entity)
        }
    }
}

/// Parse pixel value from CSS (e.g., "40px" -> 40.0).
fn parse_px_value(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(px) = value.strip_suffix("px") {
        px.trim().parse().ok()
    } else if let Some(pt) = value.strip_suffix("pt") {
        // Convert points to pixels (1pt ≈ 1.333px at 96dpi)
        pt.trim().parse::<f32>().ok().map(|p| p * 1.333)
    } else {
        value.parse().ok()
    }
}

/// Parse CSS color value.
fn parse_css_color(value: &str) -> Option<Color> {
    let value = value.trim();

    // Hex color
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color_value(hex);
    }

    // RGB/RGBA function
    if value.starts_with("rgb") {
        return parse_rgb_color_function(value);
    }

    // Named colors
    parse_named_color_value(value)
}

fn parse_hex_color_value(hex: &str) -> Option<Color> {
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some(Color::from_rgb8(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::from_rgb8(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Color::from_rgba8(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_rgb_color_function(value: &str) -> Option<Color> {
    let start = value.find('(')?;
    let end = value.rfind(')')?;
    let content = &value[start + 1..end];

    let parts: Vec<&str> = content.split(',').collect();

    match parts.len() {
        3 => {
            let r: u8 = parts[0].trim().parse().ok()?;
            let g: u8 = parts[1].trim().parse().ok()?;
            let b: u8 = parts[2].trim().parse().ok()?;
            Some(Color::from_rgb8(r, g, b))
        }
        4 => {
            let r: u8 = parts[0].trim().parse().ok()?;
            let g: u8 = parts[1].trim().parse().ok()?;
            let b: u8 = parts[2].trim().parse().ok()?;
            let a_str = parts[3].trim();
            let a = if a_str.contains('.') {
                let a_float: f32 = a_str.parse().ok()?;
                (a_float * 255.0) as u8
            } else {
                a_str.parse().ok()?
            };
            Some(Color::from_rgba8(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_named_color_value(name: &str) -> Option<Color> {
    match name.to_lowercase().as_str() {
        "black" => Some(Color::from_rgb8(0, 0, 0)),
        "white" => Some(Color::from_rgb8(255, 255, 255)),
        "red" => Some(Color::from_rgb8(255, 0, 0)),
        "green" => Some(Color::from_rgb8(0, 128, 0)),
        "blue" => Some(Color::from_rgb8(0, 0, 255)),
        "yellow" => Some(Color::from_rgb8(255, 255, 0)),
        "cyan" | "aqua" => Some(Color::from_rgb8(0, 255, 255)),
        "magenta" | "fuchsia" => Some(Color::from_rgb8(255, 0, 255)),
        "gray" | "grey" => Some(Color::from_rgb8(128, 128, 128)),
        "silver" => Some(Color::from_rgb8(192, 192, 192)),
        "maroon" => Some(Color::from_rgb8(128, 0, 0)),
        "olive" => Some(Color::from_rgb8(128, 128, 0)),
        "lime" => Some(Color::from_rgb8(0, 255, 0)),
        "navy" => Some(Color::from_rgb8(0, 0, 128)),
        "purple" => Some(Color::from_rgb8(128, 0, 128)),
        "teal" => Some(Color::from_rgb8(0, 128, 128)),
        "orange" => Some(Color::from_rgb8(255, 165, 0)),
        "pink" => Some(Color::from_rgb8(255, 192, 203)),
        "brown" => Some(Color::from_rgb8(165, 42, 42)),
        "gold" => Some(Color::from_rgb8(255, 215, 0)),
        "coral" => Some(Color::from_rgb8(255, 127, 80)),
        "crimson" => Some(Color::from_rgb8(220, 20, 60)),
        "darkblue" => Some(Color::from_rgb8(0, 0, 139)),
        "darkgreen" => Some(Color::from_rgb8(0, 100, 0)),
        "darkred" => Some(Color::from_rgb8(139, 0, 0)),
        "indigo" => Some(Color::from_rgb8(75, 0, 130)),
        "violet" => Some(Color::from_rgb8(238, 130, 238)),
        "transparent" => Some(Color::from_rgba8(0, 0, 0, 0)),
        _ => None,
    }
}

/// Parse font tag attributes.
fn parse_font_tag_attrs(attrs: &[(String, String)]) -> (Option<f32>, Option<Color>) {
    let mut size = None;
    let mut color = None;

    for (key, value) in attrs {
        match key.as_str() {
            "size" => {
                size = parse_font_size_value(value);
            }
            "color" => {
                color = parse_css_color(value);
            }
            _ => {}
        }
    }

    (size, color)
}

/// Parse font size value (HTML font sizes 1-7 or px/pt values).
fn parse_font_size_value(value: &str) -> Option<f32> {
    let value = value.trim();

    if let Some(px) = value.strip_suffix("px") {
        return px.trim().parse().ok();
    }

    if let Some(pt) = value.strip_suffix("pt") {
        return pt.trim().parse::<f32>().ok().map(|p| p * 1.333);
    }

    // HTML font size 1-7 mapping
    match value {
        "1" => Some(8.0),
        "2" => Some(10.0),
        "3" => Some(12.0),
        "4" => Some(14.0),
        "5" => Some(18.0),
        "6" => Some(24.0),
        "7" => Some(36.0),
        _ => value.parse().ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_document() {
        let doc = StyledDocument::new();
        assert!(doc.is_empty());
        assert_eq!(doc.text(), "");
        assert!(doc.format_runs().is_empty());
    }

    #[test]
    fn test_from_text() {
        let doc = StyledDocument::from_text("Hello, world!");
        assert_eq!(doc.text(), "Hello, world!");
        assert!(!doc.is_empty());
        assert!(doc.format_runs().is_empty());
    }

    #[test]
    fn test_insert_plain() {
        let mut doc = StyledDocument::new();
        doc.insert(0, "Hello", CharFormat::default());
        assert_eq!(doc.text(), "Hello");
        assert!(doc.format_runs().is_empty());
    }

    #[test]
    fn test_insert_styled() {
        let mut doc = StyledDocument::new();
        doc.insert(0, "Hello", CharFormat::bold());
        assert_eq!(doc.text(), "Hello");
        assert_eq!(doc.format_runs().len(), 1);
        assert!(doc.format_runs()[0].format.bold);
    }

    #[test]
    fn test_delete() {
        let mut doc = StyledDocument::from_text("Hello, world!");
        let deleted = doc.delete(5..7);
        assert_eq!(deleted, ", ");
        assert_eq!(doc.text(), "Helloworld!");
    }

    #[test]
    fn test_format_at() {
        let mut doc = StyledDocument::from_text("Hello, world!");
        doc.set_format(0..5, CharFormat::bold());

        assert!(doc.format_at(0).bold);
        assert!(doc.format_at(4).bold);
        assert!(!doc.format_at(5).bold);
        assert!(!doc.format_at(10).bold);
    }

    #[test]
    fn test_toggle_bold() {
        let mut doc = StyledDocument::from_text("Hello, world!");

        // Toggle bold on
        doc.toggle_format(0..5, CharFormat::new().with_bold(true));
        assert!(doc.format_at(0).bold);

        // Toggle bold off
        doc.toggle_format(0..5, CharFormat::new().with_bold(true));
        assert!(!doc.format_at(0).bold);
    }

    #[test]
    fn test_multiple_formats() {
        let mut doc = StyledDocument::from_text("Hello, world!");

        doc.set_format(0..5, CharFormat::bold());
        doc.set_format(7..12, CharFormat::italic());

        assert!(doc.format_at(0).bold);
        assert!(!doc.format_at(0).italic);
        assert!(!doc.format_at(6).bold);
        assert!(doc.format_at(7).italic);
        assert!(!doc.format_at(7).bold);
    }

    #[test]
    fn test_overlapping_formats() {
        let mut doc = StyledDocument::from_text("Hello, world!");

        doc.set_format(0..7, CharFormat::bold());
        doc.set_format(4..12, CharFormat::italic());

        // 0..4 should be bold only
        assert!(doc.format_at(0).bold);
        assert!(!doc.format_at(0).italic);

        // 4..7 should be italic only (overwritten)
        assert!(!doc.format_at(4).bold);
        assert!(doc.format_at(4).italic);

        // 7..12 should be italic only
        assert!(!doc.format_at(8).bold);
        assert!(doc.format_at(8).italic);
    }

    #[test]
    fn test_insert_shifts_runs() {
        let mut doc = StyledDocument::from_text("Hello world");
        doc.set_format(6..11, CharFormat::bold());

        // Insert at the start
        doc.insert(0, "Hi ", CharFormat::default());

        assert_eq!(doc.text(), "Hi Hello world");
        // The bold run should have shifted
        assert!(!doc.format_at(6).bold);
        assert!(doc.format_at(9).bold); // "world" is now at 9..14
    }

    #[test]
    fn test_delete_adjusts_runs() {
        let mut doc = StyledDocument::from_text("Hello, world!");
        doc.set_format(7..12, CharFormat::bold());

        // Delete the comma and space
        doc.delete(5..7);

        assert_eq!(doc.text(), "Helloworld!");
        // "world" should now be at 5..10 and still bold
        assert!(doc.format_at(5).bold);
    }

    #[test]
    fn test_to_styled_spans() {
        let mut doc = StyledDocument::from_text("Hello, world!");
        doc.set_format(0..5, CharFormat::bold());

        let spans = doc.to_styled_spans();

        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, "Hello");
        assert!(spans[0].1.bold);
        assert_eq!(spans[1].0, ", world!");
        assert!(!spans[1].1.bold);
    }

    #[test]
    fn test_format_for_range_uniform() {
        let mut doc = StyledDocument::from_text("Hello");
        doc.set_format(0..5, CharFormat::bold());

        let format = doc.format_for_range(&(0..5));
        assert!(format.is_some());
        assert!(format.unwrap().bold);
    }

    #[test]
    fn test_format_for_range_mixed() {
        let mut doc = StyledDocument::from_text("Hello, world!");
        doc.set_format(0..5, CharFormat::bold());

        let format = doc.format_for_range(&(0..10));
        assert!(format.is_none()); // Mixed formatting
    }

    #[test]
    fn test_font_family() {
        let mut doc = StyledDocument::from_text("Hello");
        let format = CharFormat::new().with_font_family(Some(FontFamily::Monospace));
        doc.set_format(0..5, format);

        let result = doc.format_at(0);
        assert_eq!(result.font_family, Some(FontFamily::Monospace));
    }

    #[test]
    fn test_font_size() {
        let mut doc = StyledDocument::from_text("Hello");
        let format = CharFormat::new().with_font_size(Some(24.0));
        doc.set_format(0..5, format);

        let result = doc.format_at(0);
        assert_eq!(result.font_size, Some(24.0));
    }

    #[test]
    fn test_font_weight() {
        let mut doc = StyledDocument::from_text("Hello");
        let format = CharFormat::new().with_font_weight(Some(FontWeight::BOLD));
        doc.set_format(0..5, format);

        let result = doc.format_at(0);
        assert_eq!(result.font_weight, Some(FontWeight::BOLD));
    }

    #[test]
    fn test_combined_font_properties() {
        let mut doc = StyledDocument::from_text("Test text");
        let format = CharFormat::new()
            .with_font_family(Some(FontFamily::name("Arial")))
            .with_font_size(Some(18.0))
            .with_font_weight(Some(FontWeight::MEDIUM));
        doc.set_format(0..4, format);

        let result = doc.format_at(0);
        assert_eq!(result.font_family, Some(FontFamily::name("Arial")));
        assert_eq!(result.font_size, Some(18.0));
        assert_eq!(result.font_weight, Some(FontWeight::MEDIUM));

        // Text outside the range should have default font properties
        let default = doc.format_at(5);
        assert!(default.font_family.is_none());
        assert!(default.font_size.is_none());
        assert!(default.font_weight.is_none());
    }

    #[test]
    fn test_is_styled_with_font_properties() {
        // Font family makes it styled
        let f1 = CharFormat::new().with_font_family(Some(FontFamily::Serif));
        assert!(f1.is_styled());

        // Font size makes it styled
        let f2 = CharFormat::new().with_font_size(Some(12.0));
        assert!(f2.is_styled());

        // Font weight makes it styled
        let f3 = CharFormat::new().with_font_weight(Some(FontWeight::LIGHT));
        assert!(f3.is_styled());

        // Default is not styled
        let f4 = CharFormat::new();
        assert!(!f4.is_styled());
    }

    // =========================================================================
    // Paragraph Alignment Tests
    // =========================================================================

    #[test]
    fn test_paragraph_count() {
        // Empty document has 1 paragraph
        let doc = StyledDocument::new();
        assert_eq!(doc.paragraph_count(), 1);

        // Single line has 1 paragraph
        let doc = StyledDocument::from_text("Hello");
        assert_eq!(doc.paragraph_count(), 1);

        // Two lines have 2 paragraphs
        let doc = StyledDocument::from_text("Hello\nWorld");
        assert_eq!(doc.paragraph_count(), 2);

        // Three lines have 3 paragraphs
        let doc = StyledDocument::from_text("Line 1\nLine 2\nLine 3");
        assert_eq!(doc.paragraph_count(), 3);
    }

    #[test]
    fn test_paragraph_range() {
        let doc = StyledDocument::from_text("Hello\nWorld\nTest");

        // First paragraph: "Hello\n" (bytes 0..6)
        assert_eq!(doc.paragraph_range(0), Some(0..6));

        // Second paragraph: "World\n" (bytes 6..12)
        assert_eq!(doc.paragraph_range(1), Some(6..12));

        // Third paragraph: "Test" (bytes 12..16, no trailing newline)
        assert_eq!(doc.paragraph_range(2), Some(12..16));

        // Out of bounds
        assert_eq!(doc.paragraph_range(3), None);
    }

    #[test]
    fn test_paragraph_at() {
        let doc = StyledDocument::from_text("Hello\nWorld\nTest");

        // Positions in first paragraph
        assert_eq!(doc.paragraph_at(0), 0);
        assert_eq!(doc.paragraph_at(5), 0);

        // Positions in second paragraph
        assert_eq!(doc.paragraph_at(6), 1);
        assert_eq!(doc.paragraph_at(11), 1);

        // Positions in third paragraph
        assert_eq!(doc.paragraph_at(12), 2);
        assert_eq!(doc.paragraph_at(15), 2);
    }

    #[test]
    fn test_block_format_default() {
        let doc = StyledDocument::from_text("Hello\nWorld");

        // All paragraphs should have default (left) alignment
        assert_eq!(doc.block_format_at(0).alignment, HorizontalAlign::Left);
        assert_eq!(doc.block_format_at(1).alignment, HorizontalAlign::Left);
    }

    #[test]
    fn test_set_alignment() {
        let mut doc = StyledDocument::from_text("Hello\nWorld\nTest");

        // Set second paragraph to center
        doc.set_alignment(1..2, HorizontalAlign::Center);

        assert_eq!(doc.block_format_at(0).alignment, HorizontalAlign::Left);
        assert_eq!(doc.block_format_at(1).alignment, HorizontalAlign::Center);
        assert_eq!(doc.block_format_at(2).alignment, HorizontalAlign::Left);
    }

    #[test]
    fn test_set_alignment_range() {
        let mut doc = StyledDocument::from_text("Line 1\nLine 2\nLine 3\nLine 4");

        // Set paragraphs 1-2 to right alignment
        doc.set_alignment(1..3, HorizontalAlign::Right);

        assert_eq!(doc.block_format_at(0).alignment, HorizontalAlign::Left);
        assert_eq!(doc.block_format_at(1).alignment, HorizontalAlign::Right);
        assert_eq!(doc.block_format_at(2).alignment, HorizontalAlign::Right);
        assert_eq!(doc.block_format_at(3).alignment, HorizontalAlign::Left);
    }

    #[test]
    fn test_block_format_for_range() {
        let mut doc = StyledDocument::from_text("Line 1\nLine 2\nLine 3");

        // Set all to center
        doc.set_alignment(0..3, HorizontalAlign::Center);

        // Uniform range should return Some
        let format = doc.block_format_for_range(&(0..3));
        assert!(format.is_some());
        assert_eq!(format.unwrap().alignment, HorizontalAlign::Center);

        // Now make them mixed
        doc.set_alignment(1..2, HorizontalAlign::Right);

        // Mixed range should return None
        let format = doc.block_format_for_range(&(0..3));
        assert!(format.is_none());
    }

    #[test]
    fn test_block_format_factories() {
        assert_eq!(BlockFormat::left().alignment, HorizontalAlign::Left);
        assert_eq!(BlockFormat::center().alignment, HorizontalAlign::Center);
        assert_eq!(BlockFormat::right().alignment, HorizontalAlign::Right);
        assert_eq!(BlockFormat::justified().alignment, HorizontalAlign::Justified);
    }

    #[test]
    fn test_block_format_is_styled() {
        // Default (left) is not styled
        assert!(!BlockFormat::new().is_styled());
        assert!(!BlockFormat::left().is_styled());

        // Other alignments are styled
        assert!(BlockFormat::center().is_styled());
        assert!(BlockFormat::right().is_styled());
        assert!(BlockFormat::justified().is_styled());
    }

    #[test]
    fn test_block_format_indentation_is_styled() {
        // Default indent (0) is not styled
        let default = BlockFormat::new();
        assert!(!default.is_styled());

        // With left indent
        let with_left = BlockFormat::new().with_left_indent(40.0);
        assert!(with_left.is_styled());

        // With first line indent
        let with_first = BlockFormat::new().with_first_line_indent(20.0);
        assert!(with_first.is_styled());

        // Negative first line indent (hanging)
        let with_hanging = BlockFormat::new().with_first_line_indent(-20.0);
        assert!(with_hanging.is_styled());
    }

    #[test]
    fn test_block_format_indent_effective_values() {
        let format = BlockFormat::new()
            .with_left_indent(40.0)
            .with_first_line_indent(20.0);

        assert_eq!(format.first_line_effective_indent(), 60.0);
        assert_eq!(format.subsequent_lines_indent(), 40.0);

        // Hanging indent (negative first line)
        let hanging = BlockFormat::new()
            .with_left_indent(40.0)
            .with_first_line_indent(-20.0);

        assert_eq!(hanging.first_line_effective_indent(), 20.0);
        assert_eq!(hanging.subsequent_lines_indent(), 40.0);
    }

    #[test]
    fn test_set_left_indent() {
        let mut doc = StyledDocument::from_text("First paragraph.\nSecond paragraph.\n");
        doc.set_left_indent(0..2, 40.0);

        assert_eq!(doc.block_format_at(0).left_indent, 40.0);
        assert_eq!(doc.block_format_at(1).left_indent, 40.0);
    }

    #[test]
    fn test_set_first_line_indent() {
        let mut doc = StyledDocument::from_text("First paragraph.\nSecond paragraph.\n");
        doc.set_first_line_indent(0..1, 20.0);

        assert_eq!(doc.block_format_at(0).first_line_indent, 20.0);
        assert_eq!(doc.block_format_at(1).first_line_indent, 0.0);
    }

    #[test]
    fn test_increase_indent() {
        let mut doc = StyledDocument::from_text("First paragraph.\n");
        assert_eq!(doc.block_format_at(0).left_indent, 0.0);

        doc.increase_indent(0..1);
        assert_eq!(doc.block_format_at(0).left_indent, BlockFormat::INDENT_STEP);

        doc.increase_indent(0..1);
        assert_eq!(doc.block_format_at(0).left_indent, BlockFormat::INDENT_STEP * 2.0);
    }

    #[test]
    fn test_decrease_indent() {
        let mut doc = StyledDocument::from_text("First paragraph.\n");
        doc.set_left_indent(0..1, 80.0);

        doc.decrease_indent(0..1);
        assert_eq!(doc.block_format_at(0).left_indent, 80.0 - BlockFormat::INDENT_STEP);

        doc.decrease_indent(0..1);
        assert_eq!(doc.block_format_at(0).left_indent, 0.0);

        // Cannot go below 0
        doc.decrease_indent(0..1);
        assert_eq!(doc.block_format_at(0).left_indent, 0.0);
    }

    #[test]
    fn test_uniform_indent() {
        // Note: "First.\nSecond.\nThird." has 3 paragraphs (no trailing newline)
        let mut doc = StyledDocument::from_text("First.\nSecond.\nThird.");

        // All paragraphs have same (default) indent
        assert_eq!(doc.uniform_left_indent(), Some(0.0));
        assert_eq!(doc.uniform_first_line_indent(), Some(0.0));

        // Set same indent on all 3 paragraphs
        doc.set_left_indent(0..3, 40.0);
        assert_eq!(doc.uniform_left_indent(), Some(40.0));

        // Set different indent on one
        doc.set_left_indent(1..2, 80.0);
        assert_eq!(doc.uniform_left_indent(), None);
    }

    #[test]
    fn test_indent_preserves_alignment() {
        let mut doc = StyledDocument::from_text("Centered paragraph.\n");
        doc.set_alignment(0..1, HorizontalAlign::Center);

        // Add indentation
        doc.set_left_indent(0..1, 40.0);

        // Alignment should be preserved
        assert_eq!(doc.block_format_at(0).alignment, HorizontalAlign::Center);
        assert_eq!(doc.block_format_at(0).left_indent, 40.0);
    }

    // =========================================================================
    // Line Spacing Tests
    // =========================================================================

    #[test]
    fn test_line_spacing_enum_to_multiplier() {
        assert_eq!(LineSpacing::Single.to_multiplier(), 1.2);
        assert_eq!(LineSpacing::OnePointFive.to_multiplier(), 1.5);
        assert_eq!(LineSpacing::Double.to_multiplier(), 2.0);
        assert_eq!(LineSpacing::Custom(1.8).to_multiplier(), 1.8);
    }

    #[test]
    fn test_line_spacing_equality() {
        // Verify PartialEq works as expected
        assert_eq!(LineSpacing::Single, LineSpacing::Single);
        assert_eq!(LineSpacing::OnePointFive, LineSpacing::OnePointFive);
        assert_eq!(LineSpacing::Double, LineSpacing::Double);
        assert_ne!(LineSpacing::Single, LineSpacing::OnePointFive);
        assert_ne!(LineSpacing::OnePointFive, LineSpacing::Double);

        // Verify inequality
        let a = LineSpacing::OnePointFive;
        let b = LineSpacing::OnePointFive;
        assert!(!(a != b), "OnePointFive should equal OnePointFive");
    }

    #[test]
    fn test_line_spacing_default() {
        let format = BlockFormat::new();
        assert_eq!(format.line_spacing, LineSpacing::Single);
    }

    #[test]
    fn test_set_line_spacing() {
        let mut doc = StyledDocument::from_text("First paragraph.\nSecond paragraph.\n");
        doc.set_line_spacing(0..1, LineSpacing::Double);

        assert_eq!(doc.block_format_at(0).line_spacing, LineSpacing::Double);
        assert_eq!(doc.block_format_at(1).line_spacing, LineSpacing::Single);
    }

    #[test]
    fn test_uniform_line_spacing_same() {
        // Use text without trailing newline to get exactly 2 paragraphs
        let mut doc = StyledDocument::from_text("First.\nSecond.");

        // Check initial state - "First.\nSecond." has 2 paragraphs
        assert_eq!(doc.paragraph_count(), 2, "Should have 2 paragraphs");

        doc.set_line_spacing(0..2, LineSpacing::OnePointFive);

        // Verify each paragraph has the correct spacing
        assert_eq!(doc.block_format_at(0).line_spacing, LineSpacing::OnePointFive);
        assert_eq!(doc.block_format_at(1).line_spacing, LineSpacing::OnePointFive);

        assert_eq!(
            doc.uniform_line_spacing(),
            Some(LineSpacing::OnePointFive)
        );
    }

    #[test]
    fn test_uniform_line_spacing_different() {
        let mut doc = StyledDocument::from_text("First.\nSecond.\nThird.\n");
        doc.set_line_spacing(0..1, LineSpacing::Double);
        doc.set_line_spacing(1..2, LineSpacing::Single);

        assert_eq!(doc.uniform_line_spacing(), None);
    }

    #[test]
    fn test_line_spacing_with_builder() {
        let format = BlockFormat::new()
            .with_line_spacing(LineSpacing::OnePointFive);
        assert_eq!(format.line_spacing, LineSpacing::OnePointFive);
    }

    // =========================================================================
    // Paragraph Spacing Tests
    // =========================================================================

    #[test]
    fn test_spacing_before_default() {
        let format = BlockFormat::new();
        assert_eq!(format.spacing_before, 0.0);
        assert_eq!(format.spacing_after, 0.0);
    }

    #[test]
    fn test_set_spacing_before() {
        let mut doc = StyledDocument::from_text("First.\nSecond.\n");
        doc.set_spacing_before(0..1, 12.0);

        assert_eq!(doc.block_format_at(0).spacing_before, 12.0);
        assert_eq!(doc.block_format_at(1).spacing_before, 0.0);
    }

    #[test]
    fn test_set_spacing_after() {
        let mut doc = StyledDocument::from_text("First.\nSecond.\n");
        doc.set_spacing_after(0..1, 8.0);

        assert_eq!(doc.block_format_at(0).spacing_after, 8.0);
        assert_eq!(doc.block_format_at(1).spacing_after, 0.0);
    }

    #[test]
    fn test_uniform_spacing_before_same() {
        // Use text without trailing newline to get exactly 2 paragraphs
        let mut doc = StyledDocument::from_text("First.\nSecond.");
        doc.set_spacing_before(0..2, 10.0);

        // Verify each paragraph
        assert_eq!(doc.block_format_at(0).spacing_before, 10.0);
        assert_eq!(doc.block_format_at(1).spacing_before, 10.0);

        assert_eq!(doc.uniform_spacing_before(), Some(10.0));
    }

    #[test]
    fn test_uniform_spacing_before_different() {
        let mut doc = StyledDocument::from_text("First.\nSecond.\n");
        doc.set_spacing_before(0..1, 10.0);
        doc.set_spacing_before(1..2, 20.0);

        assert_eq!(doc.uniform_spacing_before(), None);
    }

    #[test]
    fn test_uniform_spacing_after_same() {
        // Use text without trailing newline to get exactly 2 paragraphs
        let mut doc = StyledDocument::from_text("First.\nSecond.");
        doc.set_spacing_after(0..2, 5.0);

        // Verify each paragraph
        assert_eq!(doc.block_format_at(0).spacing_after, 5.0);
        assert_eq!(doc.block_format_at(1).spacing_after, 5.0);

        assert_eq!(doc.uniform_spacing_after(), Some(5.0));
    }

    #[test]
    fn test_uniform_spacing_after_different() {
        let mut doc = StyledDocument::from_text("First.\nSecond.\n");
        doc.set_spacing_after(0..1, 5.0);
        doc.set_spacing_after(1..2, 15.0);

        assert_eq!(doc.uniform_spacing_after(), None);
    }

    #[test]
    fn test_spacing_with_builder() {
        let format = BlockFormat::new()
            .with_spacing_before(12.0)
            .with_spacing_after(8.0);
        assert_eq!(format.spacing_before, 12.0);
        assert_eq!(format.spacing_after, 8.0);
    }

    #[test]
    fn test_spacing_is_styled() {
        // Default should not be styled
        let default = BlockFormat::new();
        assert!(!default.is_styled());

        // Line spacing should make it styled
        let with_line_spacing = BlockFormat::new()
            .with_line_spacing(LineSpacing::Double);
        assert!(with_line_spacing.is_styled());

        // Spacing before should make it styled
        let with_spacing_before = BlockFormat::new()
            .with_spacing_before(10.0);
        assert!(with_spacing_before.is_styled());

        // Spacing after should make it styled
        let with_spacing_after = BlockFormat::new()
            .with_spacing_after(10.0);
        assert!(with_spacing_after.is_styled());
    }

    #[test]
    fn test_spacing_preserves_other_formatting() {
        let mut doc = StyledDocument::from_text("Centered with spacing.\n");
        doc.set_alignment(0..1, HorizontalAlign::Center);
        doc.set_left_indent(0..1, 40.0);
        doc.set_line_spacing(0..1, LineSpacing::Double);
        doc.set_spacing_after(0..1, 12.0);

        let format = doc.block_format_at(0);
        assert_eq!(format.alignment, HorizontalAlign::Center);
        assert_eq!(format.left_indent, 40.0);
        assert_eq!(format.line_spacing, LineSpacing::Double);
        assert_eq!(format.spacing_after, 12.0);
    }

    // =========================================================================
    // List Formatting Tests
    // =========================================================================

    #[test]
    fn test_list_style_is_bullet() {
        assert!(ListStyle::Disc.is_bullet());
        assert!(ListStyle::Circle.is_bullet());
        assert!(ListStyle::Square.is_bullet());
        assert!(!ListStyle::Decimal.is_bullet());
        assert!(!ListStyle::LowerAlpha.is_bullet());
        assert!(!ListStyle::UpperAlpha.is_bullet());
        assert!(!ListStyle::LowerRoman.is_bullet());
        assert!(!ListStyle::UpperRoman.is_bullet());
    }

    #[test]
    fn test_list_style_is_numbered() {
        assert!(!ListStyle::Disc.is_numbered());
        assert!(!ListStyle::Circle.is_numbered());
        assert!(!ListStyle::Square.is_numbered());
        assert!(ListStyle::Decimal.is_numbered());
        assert!(ListStyle::LowerAlpha.is_numbered());
        assert!(ListStyle::UpperAlpha.is_numbered());
        assert!(ListStyle::LowerRoman.is_numbered());
        assert!(ListStyle::UpperRoman.is_numbered());
    }

    #[test]
    fn test_list_style_bullet_markers() {
        assert_eq!(ListStyle::Disc.bullet_marker(), Some("•"));
        assert_eq!(ListStyle::Circle.bullet_marker(), Some("○"));
        assert_eq!(ListStyle::Square.bullet_marker(), Some("■"));
        assert_eq!(ListStyle::Decimal.bullet_marker(), None);
    }

    #[test]
    fn test_list_style_number_markers() {
        // Decimal
        assert_eq!(ListStyle::Decimal.number_marker(0, 1), Some("1.".to_string()));
        assert_eq!(ListStyle::Decimal.number_marker(1, 1), Some("2.".to_string()));
        assert_eq!(ListStyle::Decimal.number_marker(9, 1), Some("10.".to_string()));

        // Lower alpha
        assert_eq!(ListStyle::LowerAlpha.number_marker(0, 1), Some("a.".to_string()));
        assert_eq!(ListStyle::LowerAlpha.number_marker(1, 1), Some("b.".to_string()));
        assert_eq!(ListStyle::LowerAlpha.number_marker(25, 1), Some("z.".to_string()));
        assert_eq!(ListStyle::LowerAlpha.number_marker(26, 1), Some("aa.".to_string()));

        // Upper alpha
        assert_eq!(ListStyle::UpperAlpha.number_marker(0, 1), Some("A.".to_string()));
        assert_eq!(ListStyle::UpperAlpha.number_marker(25, 1), Some("Z.".to_string()));

        // Lower roman
        assert_eq!(ListStyle::LowerRoman.number_marker(0, 1), Some("i.".to_string()));
        assert_eq!(ListStyle::LowerRoman.number_marker(1, 1), Some("ii.".to_string()));
        assert_eq!(ListStyle::LowerRoman.number_marker(3, 1), Some("iv.".to_string()));
        assert_eq!(ListStyle::LowerRoman.number_marker(8, 1), Some("ix.".to_string()));

        // Upper roman
        assert_eq!(ListStyle::UpperRoman.number_marker(0, 1), Some("I.".to_string()));
        assert_eq!(ListStyle::UpperRoman.number_marker(9, 1), Some("X.".to_string()));
    }

    #[test]
    fn test_list_style_for_level() {
        // Bullet styles by level
        assert_eq!(ListStyle::bullet_for_level(0), ListStyle::Disc);
        assert_eq!(ListStyle::bullet_for_level(1), ListStyle::Circle);
        assert_eq!(ListStyle::bullet_for_level(2), ListStyle::Square);
        assert_eq!(ListStyle::bullet_for_level(3), ListStyle::Square);

        // Number styles by level
        assert_eq!(ListStyle::number_for_level(0), ListStyle::Decimal);
        assert_eq!(ListStyle::number_for_level(1), ListStyle::LowerAlpha);
        assert_eq!(ListStyle::number_for_level(2), ListStyle::LowerRoman);
        assert_eq!(ListStyle::number_for_level(3), ListStyle::Decimal);
    }

    #[test]
    fn test_list_format_creation() {
        let bullet = ListFormat::bullet();
        assert_eq!(bullet.style, ListStyle::Disc);
        assert_eq!(bullet.indent_level, 0);
        assert_eq!(bullet.start, 1);

        let numbered = ListFormat::numbered();
        assert_eq!(numbered.style, ListStyle::Decimal);
        assert_eq!(numbered.indent_level, 0);
        assert_eq!(numbered.start, 1);

        let custom = ListFormat::new(ListStyle::LowerAlpha)
            .with_indent_level(2)
            .with_start(5);
        assert_eq!(custom.style, ListStyle::LowerAlpha);
        assert_eq!(custom.indent_level, 2);
        assert_eq!(custom.start, 5);
    }

    #[test]
    fn test_list_format_left_indent() {
        let level0 = ListFormat::bullet().with_indent_level(0);
        assert_eq!(level0.left_indent(), ListFormat::INDENT_STEP);

        let level1 = ListFormat::bullet().with_indent_level(1);
        assert_eq!(level1.left_indent(), ListFormat::INDENT_STEP * 2.0);

        let level2 = ListFormat::bullet().with_indent_level(2);
        assert_eq!(level2.left_indent(), ListFormat::INDENT_STEP * 3.0);
    }

    #[test]
    fn test_list_format_marker() {
        let bullet = ListFormat::bullet();
        assert_eq!(bullet.marker(0), "•");
        assert_eq!(bullet.marker(5), "•");

        let numbered = ListFormat::numbered();
        assert_eq!(numbered.marker(0), "1.");
        assert_eq!(numbered.marker(1), "2.");
        assert_eq!(numbered.marker(9), "10.");

        let numbered_start5 = ListFormat::numbered().with_start(5);
        assert_eq!(numbered_start5.marker(0), "5.");
        assert_eq!(numbered_start5.marker(1), "6.");
    }

    #[test]
    fn test_block_format_list() {
        let default = BlockFormat::new();
        assert!(default.list_format.is_none());
        assert!(!default.is_list_item());

        let bullet = BlockFormat::bullet_list();
        assert!(bullet.list_format.is_some());
        assert!(bullet.is_list_item());
        assert!(bullet.list_format.as_ref().unwrap().style.is_bullet());

        let numbered = BlockFormat::numbered_list();
        assert!(numbered.list_format.is_some());
        assert!(numbered.is_list_item());
        assert!(numbered.list_format.as_ref().unwrap().style.is_numbered());
    }

    #[test]
    fn test_document_toggle_bullet_list() {
        let mut doc = StyledDocument::from_text("Item 1\nItem 2\nItem 3");

        // All paragraphs start without list formatting
        assert!(!doc.is_list_item(0));
        assert!(!doc.is_list_item(1));
        assert!(!doc.is_list_item(2));

        // Toggle bullet list on all paragraphs
        doc.toggle_bullet_list(0..3);
        assert!(doc.is_list_item(0));
        assert!(doc.is_list_item(1));
        assert!(doc.is_list_item(2));

        // Verify they're bullet lists
        assert!(doc.list_format_at(0).unwrap().style.is_bullet());
        assert!(doc.list_format_at(1).unwrap().style.is_bullet());
        assert!(doc.list_format_at(2).unwrap().style.is_bullet());

        // Toggle again to remove
        doc.toggle_bullet_list(0..3);
        assert!(!doc.is_list_item(0));
        assert!(!doc.is_list_item(1));
        assert!(!doc.is_list_item(2));
    }

    #[test]
    fn test_document_toggle_numbered_list() {
        let mut doc = StyledDocument::from_text("Item 1\nItem 2\nItem 3");

        // Toggle numbered list on all paragraphs
        doc.toggle_numbered_list(0..3);
        assert!(doc.is_list_item(0));
        assert!(doc.is_list_item(1));
        assert!(doc.is_list_item(2));

        // Verify they're numbered lists
        assert!(doc.list_format_at(0).unwrap().style.is_numbered());
        assert!(doc.list_format_at(1).unwrap().style.is_numbered());
        assert!(doc.list_format_at(2).unwrap().style.is_numbered());

        // Toggle again to remove
        doc.toggle_numbered_list(0..3);
        assert!(!doc.is_list_item(0));
        assert!(!doc.is_list_item(1));
        assert!(!doc.is_list_item(2));
    }

    #[test]
    fn test_document_list_indent() {
        let mut doc = StyledDocument::from_text("Item 1\nItem 2");
        doc.toggle_bullet_list(0..2);

        // Initial indent level is 0
        assert_eq!(doc.list_format_at(0).unwrap().indent_level, 0);
        assert_eq!(doc.list_format_at(1).unwrap().indent_level, 0);

        // Increase indent
        doc.increase_list_indent(0..1);
        assert_eq!(doc.list_format_at(0).unwrap().indent_level, 1);
        assert_eq!(doc.list_format_at(1).unwrap().indent_level, 0); // Not affected

        // Increase again
        doc.increase_list_indent(0..1);
        assert_eq!(doc.list_format_at(0).unwrap().indent_level, 2);

        // Decrease indent
        doc.decrease_list_indent(0..1);
        assert_eq!(doc.list_format_at(0).unwrap().indent_level, 1);

        // Decrease to 0
        doc.decrease_list_indent(0..1);
        assert_eq!(doc.list_format_at(0).unwrap().indent_level, 0);

        // Cannot go below 0
        doc.decrease_list_indent(0..1);
        assert_eq!(doc.list_format_at(0).unwrap().indent_level, 0);
    }

    #[test]
    fn test_document_set_list_style() {
        let mut doc = StyledDocument::from_text("Item 1\nItem 2");
        doc.toggle_bullet_list(0..2);

        // Change to circle
        doc.set_list_style(0..2, ListStyle::Circle);
        assert_eq!(doc.list_format_at(0).unwrap().style, ListStyle::Circle);
        assert_eq!(doc.list_format_at(1).unwrap().style, ListStyle::Circle);

        // Change first to numbered
        doc.set_list_style(0..1, ListStyle::Decimal);
        assert_eq!(doc.list_format_at(0).unwrap().style, ListStyle::Decimal);
        assert_eq!(doc.list_format_at(1).unwrap().style, ListStyle::Circle);
    }

    #[test]
    fn test_document_list_item_number() {
        let mut doc = StyledDocument::from_text("Item 1\nItem 2\nItem 3\nNot a list\nItem 4\nItem 5");

        // Make first three items a numbered list
        doc.toggle_numbered_list(0..3);

        // Item numbers (0-indexed count within the list)
        assert_eq!(doc.list_item_number(0), 0);
        assert_eq!(doc.list_item_number(1), 1);
        assert_eq!(doc.list_item_number(2), 2);

        // Non-list item
        assert_eq!(doc.list_item_number(3), 0);

        // Make items 4-5 a separate list
        doc.toggle_numbered_list(4..6);

        // New list starts at 0
        assert_eq!(doc.list_item_number(4), 0);
        assert_eq!(doc.list_item_number(5), 1);
    }

    #[test]
    fn test_nested_list_numbering() {
        let mut doc = StyledDocument::from_text("Item 1\nSub 1\nSub 2\nItem 2");

        // Create list
        doc.toggle_numbered_list(0..4);

        // Indent sub-items
        doc.increase_list_indent(1..3);

        // Top-level items
        assert_eq!(doc.list_item_number(0), 0);
        assert_eq!(doc.list_item_number(3), 1); // After the nested items

        // Nested items have their own numbering
        assert_eq!(doc.list_item_number(1), 0);
        assert_eq!(doc.list_item_number(2), 1);
    }

    // =========================================================================
    // HTML Serialization Tests
    // =========================================================================

    #[test]
    fn test_html_export_plain_text() {
        let doc = StyledDocument::from_text("Hello, world!");
        let html = doc.to_html();
        assert_eq!(html, "<p>Hello, world!</p>");
    }

    #[test]
    fn test_html_export_bold() {
        let mut doc = StyledDocument::from_text("Hello world");
        doc.set_format(0..5, CharFormat::bold());
        let html = doc.to_html();
        assert_eq!(html, "<p><b>Hello</b> world</p>");
    }

    #[test]
    fn test_html_export_multiple_formats() {
        let mut doc = StyledDocument::from_text("Hello world");
        doc.set_format(0..5, CharFormat::bold());
        doc.set_format(6..11, CharFormat::italic());
        let html = doc.to_html();
        assert_eq!(html, "<p><b>Hello</b> <i>world</i></p>");
    }

    #[test]
    fn test_html_export_escapes_special_chars() {
        let doc = StyledDocument::from_text("A < B & C > D");
        let html = doc.to_html();
        assert_eq!(html, "<p>A &lt; B &amp; C &gt; D</p>");
    }

    #[test]
    fn test_html_export_multiple_paragraphs() {
        let doc = StyledDocument::from_text("Line 1\nLine 2\nLine 3");
        let html = doc.to_html();
        assert_eq!(html, "<p>Line 1</p><p>Line 2</p><p>Line 3</p>");
    }

    #[test]
    fn test_html_export_bullet_list() {
        let mut doc = StyledDocument::from_text("Item 1\nItem 2");
        doc.toggle_bullet_list(0..2);
        let html = doc.to_html();
        assert_eq!(html, "<ul><li>Item 1</li><li>Item 2</li></ul>");
    }

    #[test]
    fn test_html_export_numbered_list() {
        let mut doc = StyledDocument::from_text("First\nSecond");
        doc.toggle_numbered_list(0..2);
        let html = doc.to_html();
        assert_eq!(html, "<ol><li>First</li><li>Second</li></ol>");
    }

    #[test]
    fn test_html_import_plain_text() {
        let doc = StyledDocument::from_html("<p>Hello, world!</p>");
        assert_eq!(doc.text(), "Hello, world!");
    }

    #[test]
    fn test_html_import_bold() {
        let doc = StyledDocument::from_html("<p><b>Bold</b> text</p>");
        assert_eq!(doc.text(), "Bold text");
        assert!(doc.format_at(0).bold);
        assert!(!doc.format_at(5).bold);
    }

    #[test]
    fn test_html_import_italic() {
        let doc = StyledDocument::from_html("<p><i>Italic</i> text</p>");
        assert_eq!(doc.text(), "Italic text");
        assert!(doc.format_at(0).italic);
        assert!(!doc.format_at(7).italic);
    }

    #[test]
    fn test_html_import_nested_formatting() {
        let doc = StyledDocument::from_html("<p><b><i>Bold italic</i></b></p>");
        assert_eq!(doc.text(), "Bold italic");
        let format = doc.format_at(0);
        assert!(format.bold);
        assert!(format.italic);
    }

    #[test]
    fn test_html_import_decodes_entities() {
        let doc = StyledDocument::from_html("<p>&lt;tag&gt; &amp; &quot;text&quot;</p>");
        assert_eq!(doc.text(), "<tag> & \"text\"");
    }

    #[test]
    fn test_html_import_bullet_list() {
        let doc = StyledDocument::from_html("<ul><li>One</li><li>Two</li></ul>");
        assert_eq!(doc.text(), "One\nTwo");
        assert!(doc.is_list_item(0));
        assert!(doc.is_list_item(1));
        let list_format = doc.list_format_at(0).unwrap();
        assert!(list_format.style.is_bullet());
    }

    #[test]
    fn test_html_import_numbered_list() {
        let doc = StyledDocument::from_html("<ol><li>First</li><li>Second</li></ol>");
        assert_eq!(doc.text(), "First\nSecond");
        let list_format = doc.list_format_at(0).unwrap();
        assert!(list_format.style.is_numbered());
    }

    #[test]
    fn test_html_roundtrip_basic() {
        let mut original = StyledDocument::from_text("Hello world");
        original.set_format(0..5, CharFormat::bold());
        original.set_format(6..11, CharFormat::italic());

        let html = original.to_html();
        let restored = StyledDocument::from_html(&html);

        assert_eq!(restored.text(), original.text());
        assert!(restored.format_at(0).bold);
        assert!(restored.format_at(6).italic);
    }

    #[test]
    fn test_html_roundtrip_list() {
        let mut original = StyledDocument::from_text("Item 1\nItem 2\nItem 3");
        original.toggle_bullet_list(0..3);

        let html = original.to_html();
        let restored = StyledDocument::from_html(&html);

        assert_eq!(restored.text(), original.text());
        assert!(restored.is_list_item(0));
        assert!(restored.is_list_item(1));
        assert!(restored.is_list_item(2));
    }

    #[test]
    fn test_html_import_color() {
        let doc = StyledDocument::from_html("<p><span style=\"color:#ff0000\">Red</span></p>");
        assert_eq!(doc.text(), "Red");
        let format = doc.format_at(0);
        assert!(format.foreground_color.is_some());
        let color = format.foreground_color.unwrap();
        // Colors are stored as f32 0.0-1.0
        assert!((color.r - 1.0).abs() < 0.01);
        assert!(color.g.abs() < 0.01);
        assert!(color.b.abs() < 0.01);
    }

    #[test]
    fn test_html_import_multiple_paragraphs() {
        let doc = StyledDocument::from_html("<p>First</p><p>Second</p><p>Third</p>");
        assert_eq!(doc.text(), "First\nSecond\nThird");
        assert_eq!(doc.paragraph_count(), 3);
    }

    // =========================================================================
    // Range to HTML Tests (Clipboard Support)
    // =========================================================================

    #[test]
    fn test_range_to_html_empty_range() {
        let doc = StyledDocument::from_text("Hello, world!");
        let html = doc.range_to_html(5..5);
        assert_eq!(html, "");
    }

    #[test]
    fn test_range_to_html_invalid_range() {
        let doc = StyledDocument::from_text("Hello");
        let html = doc.range_to_html(10..20);
        assert_eq!(html, "");
    }

    #[test]
    fn test_range_to_html_plain_text() {
        let doc = StyledDocument::from_text("Hello, world!");
        let html = doc.range_to_html(0..5);
        assert!(html.contains("Hello"));
        assert!(html.contains("<p>"));
    }

    #[test]
    fn test_range_to_html_with_formatting() {
        let mut doc = StyledDocument::from_text("Hello, world!");
        doc.set_format(0..5, CharFormat::bold());

        let html = doc.range_to_html(0..5);
        assert!(html.contains("<b>"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn test_range_to_html_partial_format_run() {
        let mut doc = StyledDocument::from_text("Hello, world!");
        doc.set_format(0..5, CharFormat::bold());

        // Select only part of the bold range
        let html = doc.range_to_html(2..5);
        assert!(html.contains("<b>"));
        assert!(html.contains("llo"));
    }

    #[test]
    fn test_range_to_html_multiple_paragraphs() {
        let doc = StyledDocument::from_text("First\nSecond\nThird");

        // Select across two paragraphs
        let html = doc.range_to_html(0..12);
        assert!(html.contains("First"));
        assert!(html.contains("Second"));
    }

    #[test]
    fn test_range_to_html_preserves_list_format() {
        let mut doc = StyledDocument::from_text("Item 1\nItem 2\n");
        doc.toggle_bullet_list(0..2);

        let html = doc.range_to_html(0..14);
        assert!(html.contains("<ul>"));
        assert!(html.contains("<li>"));
    }

    #[test]
    fn test_range_to_html_roundtrip() {
        let mut original = StyledDocument::from_text("Hello, world!");
        original.set_format(0..5, CharFormat::bold());
        original.set_format(7..12, CharFormat::italic());

        // Export range to HTML
        let html = original.range_to_html(0..13);

        // Parse back
        let restored = StyledDocument::from_html(&html);

        // Verify formatting preserved
        assert!(restored.format_at(0).bold);
        assert!(restored.format_at(7).italic);
    }
}
