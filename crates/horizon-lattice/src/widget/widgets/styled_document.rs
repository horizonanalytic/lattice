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

/// Paragraph/block-level formatting attributes.
///
/// Represents the styling applied to a paragraph of text.
/// Paragraphs are defined by newline characters.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlockFormat {
    /// Horizontal text alignment.
    pub alignment: HorizontalAlign,
}

impl BlockFormat {
    /// Create a new default block format.
    pub fn new() -> Self {
        Self {
            alignment: HorizontalAlign::Left,
        }
    }

    /// Check if this format has any non-default styling.
    pub fn is_styled(&self) -> bool {
        self.alignment != HorizontalAlign::Left
    }

    /// Builder method to set alignment.
    pub fn with_alignment(mut self, alignment: HorizontalAlign) -> Self {
        self.alignment = alignment;
        self
    }

    /// Create a left-aligned block format.
    pub fn left() -> Self {
        Self {
            alignment: HorizontalAlign::Left,
        }
    }

    /// Create a center-aligned block format.
    pub fn center() -> Self {
        Self {
            alignment: HorizontalAlign::Center,
        }
    }

    /// Create a right-aligned block format.
    pub fn right() -> Self {
        Self {
            alignment: HorizontalAlign::Right,
        }
    }

    /// Create a justified block format.
    pub fn justified() -> Self {
        Self {
            alignment: HorizontalAlign::Justified,
        }
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
}
