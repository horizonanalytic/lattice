//! Find and replace functionality for text editors.
//!
//! This module provides:
//! - [`Searchable`]: Trait for text editors that support find/replace operations
//! - [`FindOptions`]: Configuration for search behavior (case sensitivity, regex, etc.)
//! - [`SearchMatch`]: Represents a single match in the text
//! - [`FindReplaceBar`]: Dockable widget providing find/replace UI
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{PlainTextEdit, FindReplaceBar, FindOptions};
//!
//! let mut editor = PlainTextEdit::new();
//! let mut find_bar = FindReplaceBar::new();
//!
//! // Connect find bar to editor
//! find_bar.attach(&mut editor);
//!
//! // Programmatic search
//! let options = FindOptions::default().with_case_sensitive(true);
//! let matches = editor.find_all("search term", &options);
//! ```

use regex::Regex;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, RoundedRect, Stroke,
    TextLayout, TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MousePressEvent,
    PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

// =========================================================================
// Search Types
// =========================================================================

/// Options for controlling search behavior.
#[derive(Debug, Clone, PartialEq)]
pub struct FindOptions {
    /// Whether search is case sensitive.
    pub case_sensitive: bool,
    /// Whether to match whole words only.
    pub whole_word: bool,
    /// Whether to interpret the pattern as a regex.
    pub use_regex: bool,
    /// Whether to wrap around when reaching the end/start of the document.
    pub wrap_around: bool,
}

impl Default for FindOptions {
    fn default() -> Self {
        Self {
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            wrap_around: true,
        }
    }
}

impl FindOptions {
    /// Create new FindOptions with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set case sensitivity.
    pub fn with_case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    /// Set whole word matching.
    pub fn with_whole_word(mut self, whole_word: bool) -> Self {
        self.whole_word = whole_word;
        self
    }

    /// Set regex mode.
    pub fn with_regex(mut self, use_regex: bool) -> Self {
        self.use_regex = use_regex;
        self
    }

    /// Set wrap around behavior.
    pub fn with_wrap_around(mut self, wrap_around: bool) -> Self {
        self.wrap_around = wrap_around;
        self
    }
}

/// Represents a single match found in the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// Start byte offset in the document.
    pub start: usize,
    /// End byte offset in the document (exclusive).
    pub end: usize,
}

impl SearchMatch {
    /// Create a new search match.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Get the length of the match in bytes.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Check if the match is empty.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

// =========================================================================
// Searchable Trait
// =========================================================================

/// Trait for text widgets that support find and replace operations.
pub trait Searchable {
    /// Get the full text content for searching.
    fn search_text(&self) -> String;

    /// Get the current cursor position (byte offset).
    fn cursor_position(&self) -> usize;

    /// Set the cursor position (byte offset).
    fn set_cursor_position(&mut self, pos: usize);

    /// Get the current selection range, if any.
    fn selection_range(&self) -> Option<(usize, usize)>;

    /// Set the selection range.
    fn set_selection(&mut self, start: usize, end: usize);

    /// Clear the current selection.
    fn clear_selection(&mut self);

    /// Replace text in the given range with new text.
    fn replace_range(&mut self, start: usize, end: usize, replacement: &str);

    /// Scroll to make the given byte position visible.
    fn scroll_to_position(&mut self, pos: usize);

    /// Set the list of matches to highlight.
    fn set_search_matches(&mut self, matches: Vec<SearchMatch>);

    /// Set the index of the current (focused) match.
    fn set_current_match_index(&mut self, index: Option<usize>);

    /// Clear all search highlighting.
    fn clear_search_highlights(&mut self);

    /// Find all matches in the text.
    fn find_all(&self, pattern: &str, options: &FindOptions) -> Vec<SearchMatch> {
        if pattern.is_empty() {
            return Vec::new();
        }

        let text = self.search_text();
        let mut matches = Vec::new();

        if options.use_regex {
            // Build regex pattern
            let pattern_str = if options.whole_word {
                format!(r"\b{}\b", pattern)
            } else {
                pattern.to_string()
            };

            let regex = if options.case_sensitive {
                Regex::new(&pattern_str)
            } else {
                Regex::new(&format!("(?i){}", pattern_str))
            };

            if let Ok(re) = regex {
                for mat in re.find_iter(&text) {
                    matches.push(SearchMatch::new(mat.start(), mat.end()));
                }
            }
        } else {
            // Plain text search
            let (search_text, search_pattern) = if options.case_sensitive {
                (text.clone(), pattern.to_string())
            } else {
                (text.to_lowercase(), pattern.to_lowercase())
            };

            let mut start = 0;
            while let Some(pos) = search_text[start..].find(&search_pattern) {
                let abs_pos = start + pos;

                // Check whole word boundary if required
                let is_word_match = if options.whole_word {
                    let before_ok = abs_pos == 0
                        || !text[..abs_pos]
                            .chars()
                            .last()
                            .map(|c| c.is_alphanumeric() || c == '_')
                            .unwrap_or(false);
                    let after_pos = abs_pos + pattern.len();
                    let after_ok = after_pos >= text.len()
                        || !text[after_pos..]
                            .chars()
                            .next()
                            .map(|c| c.is_alphanumeric() || c == '_')
                            .unwrap_or(false);
                    before_ok && after_ok
                } else {
                    true
                };

                if is_word_match {
                    matches.push(SearchMatch::new(abs_pos, abs_pos + pattern.len()));
                }

                start = abs_pos + 1;
            }
        }

        matches
    }

    /// Find the next match from the current position.
    fn find_next(
        &self,
        _pattern: &str,
        options: &FindOptions,
        matches: &[SearchMatch],
    ) -> Option<usize> {
        if matches.is_empty() {
            return None;
        }

        let cursor = self.cursor_position();

        // Find first match after cursor
        for (i, m) in matches.iter().enumerate() {
            if m.start > cursor {
                return Some(i);
            }
        }

        // Wrap around if enabled
        if options.wrap_around {
            Some(0)
        } else {
            None
        }
    }

    /// Find the previous match from the current position.
    fn find_previous(
        &self,
        _pattern: &str,
        options: &FindOptions,
        matches: &[SearchMatch],
    ) -> Option<usize> {
        if matches.is_empty() {
            return None;
        }

        let cursor = self.cursor_position();

        // Find last match before cursor
        for (i, m) in matches.iter().enumerate().rev() {
            if m.end <= cursor {
                return Some(i);
            }
        }

        // Wrap around if enabled
        if options.wrap_around {
            Some(matches.len() - 1)
        } else {
            None
        }
    }

    /// Replace the current match.
    fn replace_current(
        &mut self,
        replacement: &str,
        matches: &[SearchMatch],
        current_index: usize,
    ) -> Option<usize> {
        if current_index >= matches.len() {
            return None;
        }

        let m = &matches[current_index];
        self.replace_range(m.start, m.end, replacement);

        // Return the adjustment for subsequent matches
        Some(replacement.len() as isize - m.len() as isize)
            .map(|adj| if adj >= 0 { adj as usize } else { 0 })
    }

    /// Replace all matches with the replacement text.
    fn replace_all(&mut self, pattern: &str, replacement: &str, options: &FindOptions) -> usize {
        let matches = self.find_all(pattern, options);
        if matches.is_empty() {
            return 0;
        }

        // Replace from end to start to preserve byte offsets
        let count = matches.len();
        for m in matches.into_iter().rev() {
            self.replace_range(m.start, m.end, replacement);
        }

        count
    }
}

// =========================================================================
// Find/Replace Bar Widget
// =========================================================================

/// State for the find/replace bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindReplaceMode {
    /// Find-only mode (compact bar).
    Find,
    /// Find and replace mode (expanded bar with replace input).
    Replace,
}

/// Dockable find/replace bar widget.
///
/// Provides a VS Code-style find bar that docks at the top of the text editor.
///
/// # Signals
///
/// - `match_count_changed`: Emitted when the number of matches changes
/// - `current_match_changed`: Emitted when the focused match changes
/// - `closed`: Emitted when the find bar is closed
pub struct FindReplaceBar {
    base: WidgetBase,

    /// Current mode (find only or find+replace).
    mode: FindReplaceMode,

    /// Search pattern text.
    search_text: String,

    /// Replacement text.
    replace_text: String,

    /// Current search options.
    options: FindOptions,

    /// Cached matches from last search.
    matches: Vec<SearchMatch>,

    /// Index of current (focused) match.
    current_match: Option<usize>,

    /// Which input field is focused (0 = search, 1 = replace).
    focused_field: usize,

    /// Cursor position in search field.
    search_cursor: usize,

    /// Cursor position in replace field.
    replace_cursor: usize,

    /// Font for text rendering.
    font: Font,

    /// Colors.
    background_color: Color,
    input_background_color: Color,
    input_border_color: Color,
    input_focus_border_color: Color,
    text_color: Color,
    placeholder_color: Color,
    button_color: Color,
    button_hover_color: Color,
    button_active_color: Color,
    no_match_color: Color,

    /// Button hover states.
    hovered_button: Option<usize>,

    // Signals
    /// Emitted when match count changes.
    pub match_count_changed: Signal<usize>,

    /// Emitted when current match index changes.
    pub current_match_changed: Signal<usize>,

    /// Emitted when the find bar is closed.
    pub closed: Signal<()>,

    /// Emitted when search should be performed.
    pub search_requested: Signal<String>,

    /// Emitted when navigation to next match is requested.
    pub find_next_requested: Signal<()>,

    /// Emitted when navigation to previous match is requested.
    pub find_previous_requested: Signal<()>,

    /// Emitted when replace current is requested.
    pub replace_requested: Signal<String>,

    /// Emitted when replace all is requested.
    pub replace_all_requested: Signal<String>,

    /// Emitted when options change.
    pub options_changed: Signal<FindOptions>,
}

impl FindReplaceBar {
    /// Create a new FindReplaceBar in find-only mode.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Fixed));

        Self {
            base,
            mode: FindReplaceMode::Find,
            search_text: String::new(),
            replace_text: String::new(),
            options: FindOptions::default(),
            matches: Vec::new(),
            current_match: None,
            focused_field: 0,
            search_cursor: 0,
            replace_cursor: 0,
            font: Font::new(FontFamily::SansSerif, 13.0),
            background_color: Color::from_rgb8(37, 37, 38),
            input_background_color: Color::from_rgb8(60, 60, 60),
            input_border_color: Color::from_rgb8(69, 69, 69),
            input_focus_border_color: Color::from_rgb8(0, 122, 204),
            text_color: Color::from_rgb8(204, 204, 204),
            placeholder_color: Color::from_rgb8(128, 128, 128),
            button_color: Color::from_rgb8(60, 60, 60),
            button_hover_color: Color::from_rgb8(80, 80, 80),
            button_active_color: Color::from_rgb8(0, 122, 204),
            no_match_color: Color::from_rgb8(206, 92, 0),
            hovered_button: None,
            match_count_changed: Signal::new(),
            current_match_changed: Signal::new(),
            closed: Signal::new(),
            search_requested: Signal::new(),
            find_next_requested: Signal::new(),
            find_previous_requested: Signal::new(),
            replace_requested: Signal::new(),
            replace_all_requested: Signal::new(),
            options_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Accessors
    // =========================================================================

    /// Get the current mode.
    pub fn mode(&self) -> FindReplaceMode {
        self.mode
    }

    /// Set the mode (find only or find+replace).
    pub fn set_mode(&mut self, mode: FindReplaceMode) {
        if self.mode != mode {
            self.mode = mode;
            self.base.update();
        }
    }

    /// Get the search text.
    pub fn search_text(&self) -> &str {
        &self.search_text
    }

    /// Set the search text.
    pub fn set_search_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.search_text != text {
            self.search_text = text;
            self.search_cursor = self.search_text.len();
            self.base.update();
            self.search_requested.emit(self.search_text.clone());
        }
    }

    /// Get the replacement text.
    pub fn replace_text(&self) -> &str {
        &self.replace_text
    }

    /// Set the replacement text.
    pub fn set_replace_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.replace_text != text {
            self.replace_text = text;
            self.replace_cursor = self.replace_text.len();
            self.base.update();
        }
    }

    /// Get the current find options.
    pub fn options(&self) -> &FindOptions {
        &self.options
    }

    /// Set the find options.
    pub fn set_options(&mut self, options: FindOptions) {
        if self.options != options {
            self.options = options.clone();
            self.base.update();
            self.options_changed.emit(options);
        }
    }

    /// Toggle case sensitivity.
    pub fn toggle_case_sensitive(&mut self) {
        self.options.case_sensitive = !self.options.case_sensitive;
        self.base.update();
        self.options_changed.emit(self.options.clone());
    }

    /// Toggle whole word matching.
    pub fn toggle_whole_word(&mut self) {
        self.options.whole_word = !self.options.whole_word;
        self.base.update();
        self.options_changed.emit(self.options.clone());
    }

    /// Toggle regex mode.
    pub fn toggle_regex(&mut self) {
        self.options.use_regex = !self.options.use_regex;
        self.base.update();
        self.options_changed.emit(self.options.clone());
    }

    /// Get the current matches.
    pub fn matches(&self) -> &[SearchMatch] {
        &self.matches
    }

    /// Update the matches (usually called by the editor).
    pub fn set_matches(&mut self, matches: Vec<SearchMatch>) {
        let old_count = self.matches.len();
        self.matches = matches;

        if self.matches.len() != old_count {
            self.match_count_changed.emit(self.matches.len());
        }

        // Reset current match if out of bounds
        if let Some(idx) = self.current_match {
            if idx >= self.matches.len() {
                self.current_match = if self.matches.is_empty() {
                    None
                } else {
                    Some(0)
                };
                if let Some(new_idx) = self.current_match {
                    self.current_match_changed.emit(new_idx);
                }
            }
        } else if !self.matches.is_empty() {
            self.current_match = Some(0);
            self.current_match_changed.emit(0);
        }

        self.base.update();
    }

    /// Get the current match index.
    pub fn current_match_index(&self) -> Option<usize> {
        self.current_match
    }

    /// Set the current match index.
    pub fn set_current_match_index(&mut self, index: Option<usize>) {
        if self.current_match != index {
            self.current_match = index;
            if let Some(idx) = index {
                self.current_match_changed.emit(idx);
            }
            self.base.update();
        }
    }

    /// Navigate to the next match.
    pub fn find_next(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        let next = match self.current_match {
            Some(idx) => {
                if idx + 1 < self.matches.len() {
                    idx + 1
                } else if self.options.wrap_around {
                    0
                } else {
                    idx
                }
            }
            None => 0,
        };

        self.current_match = Some(next);
        self.current_match_changed.emit(next);
        self.find_next_requested.emit(());
        self.base.update();
    }

    /// Navigate to the previous match.
    pub fn find_previous(&mut self) {
        if self.matches.is_empty() {
            return;
        }

        let prev = match self.current_match {
            Some(idx) => {
                if idx > 0 {
                    idx - 1
                } else if self.options.wrap_around {
                    self.matches.len() - 1
                } else {
                    idx
                }
            }
            None => self.matches.len() - 1,
        };

        self.current_match = Some(prev);
        self.current_match_changed.emit(prev);
        self.find_previous_requested.emit(());
        self.base.update();
    }

    /// Request replacement of current match.
    pub fn replace_current(&mut self) {
        if self.current_match.is_some() && !self.matches.is_empty() {
            self.replace_requested.emit(self.replace_text.clone());
        }
    }

    /// Request replacement of all matches.
    pub fn replace_all(&mut self) {
        if !self.matches.is_empty() {
            self.replace_all_requested.emit(self.replace_text.clone());
        }
    }

    /// Close the find bar.
    pub fn close(&mut self) {
        self.closed.emit(());
    }

    /// Focus the search field.
    pub fn focus_search(&mut self) {
        self.focused_field = 0;
        self.base.update();
    }

    /// Focus the replace field.
    pub fn focus_replace(&mut self) {
        if self.mode == FindReplaceMode::Replace {
            self.focused_field = 1;
            self.base.update();
        }
    }

    // =========================================================================
    // Layout Constants
    // =========================================================================

    const PADDING: f32 = 8.0;
    const INPUT_HEIGHT: f32 = 24.0;
    const INPUT_WIDTH: f32 = 200.0;
    const BUTTON_SIZE: f32 = 22.0;
    const BUTTON_SPACING: f32 = 2.0;
    const OPTION_BUTTON_WIDTH: f32 = 28.0;
    const ROW_HEIGHT: f32 = 32.0;

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        ctx.renderer().fill_rect(rect, self.background_color);

        // Bottom border
        let border_rect = Rect::new(0.0, rect.height() - 1.0, rect.width(), 1.0);
        ctx.renderer()
            .fill_rect(border_rect, self.input_border_color);
    }

    fn paint_search_row(&self, ctx: &mut PaintContext<'_>, font_system: &mut FontSystem) {
        let y = Self::PADDING;

        // Search input field
        let input_rect = Rect::new(Self::PADDING, y, Self::INPUT_WIDTH, Self::INPUT_HEIGHT);
        self.paint_input_field(ctx, font_system, input_rect, &self.search_text, "Search", 0);

        // Option buttons (case, word, regex)
        let mut x = Self::PADDING + Self::INPUT_WIDTH + Self::BUTTON_SPACING * 2.0;

        // Case sensitive button
        self.paint_option_button(ctx, font_system, x, y, "Aa", self.options.case_sensitive, 0);
        x += Self::OPTION_BUTTON_WIDTH + Self::BUTTON_SPACING;

        // Whole word button
        self.paint_option_button(ctx, font_system, x, y, "Ab", self.options.whole_word, 1);
        x += Self::OPTION_BUTTON_WIDTH + Self::BUTTON_SPACING;

        // Regex button
        self.paint_option_button(ctx, font_system, x, y, ".*", self.options.use_regex, 2);
        x += Self::OPTION_BUTTON_WIDTH + Self::BUTTON_SPACING * 4.0;

        // Match count badge
        self.paint_match_count(ctx, font_system, x, y);
        x += 70.0 + Self::BUTTON_SPACING * 2.0;

        // Navigation buttons
        self.paint_nav_button(ctx, font_system, x, y, "↑", 3);
        x += Self::BUTTON_SIZE + Self::BUTTON_SPACING;

        self.paint_nav_button(ctx, font_system, x, y, "↓", 4);
        x += Self::BUTTON_SIZE + Self::BUTTON_SPACING * 4.0;

        // Close button
        self.paint_nav_button(ctx, font_system, x, y, "×", 5);
    }

    fn paint_replace_row(&self, ctx: &mut PaintContext<'_>, font_system: &mut FontSystem) {
        if self.mode != FindReplaceMode::Replace {
            return;
        }

        let y = Self::PADDING + Self::ROW_HEIGHT;

        // Replace input field
        let input_rect = Rect::new(Self::PADDING, y, Self::INPUT_WIDTH, Self::INPUT_HEIGHT);
        self.paint_input_field(
            ctx,
            font_system,
            input_rect,
            &self.replace_text,
            "Replace",
            1,
        );

        // Replace buttons
        let mut x = Self::PADDING + Self::INPUT_WIDTH + Self::BUTTON_SPACING * 2.0;

        // Replace current button
        self.paint_text_button(ctx, font_system, x, y, "Replace", 6);
        x += 60.0 + Self::BUTTON_SPACING;

        // Replace all button
        self.paint_text_button(ctx, font_system, x, y, "Replace All", 7);
    }

    fn paint_input_field(
        &self,
        ctx: &mut PaintContext<'_>,
        font_system: &mut FontSystem,
        rect: Rect,
        text: &str,
        placeholder: &str,
        field_index: usize,
    ) {
        let is_focused = self.base.has_focus() && self.focused_field == field_index;

        // Background
        let rrect = RoundedRect::new(rect, 3.0);
        ctx.renderer()
            .fill_rounded_rect(rrect, self.input_background_color);

        // Border
        let border_color = if is_focused {
            self.input_focus_border_color
        } else {
            self.input_border_color
        };
        ctx.renderer()
            .stroke_rounded_rect(rrect, &Stroke::new(border_color, 1.0));

        // Text or placeholder
        let display_text = if text.is_empty() { placeholder } else { text };
        let text_color = if text.is_empty() {
            self.placeholder_color
        } else {
            self.text_color
        };

        let text_x = rect.origin.x + 6.0;
        let text_y = rect.origin.y + (rect.height() - self.font.size()) / 2.0;

        let layout = TextLayout::new(font_system, display_text, &self.font);
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                font_system,
                &layout,
                Point::new(text_x, text_y),
                text_color,
            );
        }

        // Cursor (if focused and not showing placeholder)
        if is_focused && !text.is_empty() {
            let cursor_pos = if field_index == 0 {
                self.search_cursor
            } else {
                self.replace_cursor
            };
            let cursor_x = text_x + (cursor_pos as f32 * self.font.size() * 0.6);
            let cursor_rect = Rect::new(cursor_x, rect.origin.y + 4.0, 1.0, rect.height() - 8.0);
            ctx.renderer().fill_rect(cursor_rect, self.text_color);
        }
    }

    fn paint_option_button(
        &self,
        ctx: &mut PaintContext<'_>,
        font_system: &mut FontSystem,
        x: f32,
        y: f32,
        label: &str,
        active: bool,
        button_index: usize,
    ) {
        let rect = Rect::new(x, y, Self::OPTION_BUTTON_WIDTH, Self::INPUT_HEIGHT);
        let rrect = RoundedRect::new(rect, 3.0);

        let bg_color = if active {
            self.button_active_color
        } else if self.hovered_button == Some(button_index) {
            self.button_hover_color
        } else {
            self.button_color
        };

        ctx.renderer().fill_rounded_rect(rrect, bg_color);

        // Label
        let text_x = x + (Self::OPTION_BUTTON_WIDTH - label.len() as f32 * self.font.size() * 0.5) / 2.0;
        let text_y = y + (Self::INPUT_HEIGHT - self.font.size()) / 2.0;

        let layout = TextLayout::new(font_system, label, &self.font);
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                font_system,
                &layout,
                Point::new(text_x, text_y),
                self.text_color,
            );
        }
    }

    fn paint_nav_button(
        &self,
        ctx: &mut PaintContext<'_>,
        font_system: &mut FontSystem,
        x: f32,
        y: f32,
        label: &str,
        button_index: usize,
    ) {
        let rect = Rect::new(x, y, Self::BUTTON_SIZE, Self::INPUT_HEIGHT);
        let rrect = RoundedRect::new(rect, 3.0);

        let bg_color = if self.hovered_button == Some(button_index) {
            self.button_hover_color
        } else {
            self.button_color
        };

        ctx.renderer().fill_rounded_rect(rrect, bg_color);

        // Label centered
        let text_x = x + (Self::BUTTON_SIZE - self.font.size() * 0.6) / 2.0;
        let text_y = y + (Self::INPUT_HEIGHT - self.font.size()) / 2.0;

        let layout = TextLayout::new(font_system, label, &self.font);
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                font_system,
                &layout,
                Point::new(text_x, text_y),
                self.text_color,
            );
        }
    }

    fn paint_text_button(
        &self,
        ctx: &mut PaintContext<'_>,
        font_system: &mut FontSystem,
        x: f32,
        y: f32,
        label: &str,
        button_index: usize,
    ) {
        let width = label.len() as f32 * self.font.size() * 0.6 + 16.0;
        let rect = Rect::new(x, y, width, Self::INPUT_HEIGHT);
        let rrect = RoundedRect::new(rect, 3.0);

        let bg_color = if self.hovered_button == Some(button_index) {
            self.button_hover_color
        } else {
            self.button_color
        };

        ctx.renderer().fill_rounded_rect(rrect, bg_color);

        // Label
        let text_x = x + 8.0;
        let text_y = y + (Self::INPUT_HEIGHT - self.font.size()) / 2.0;

        let layout = TextLayout::new(font_system, label, &self.font);
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                font_system,
                &layout,
                Point::new(text_x, text_y),
                self.text_color,
            );
        }
    }

    fn paint_match_count(&self, _ctx: &mut PaintContext<'_>, font_system: &mut FontSystem, x: f32, y: f32) {
        let count_text = if self.matches.is_empty() && !self.search_text.is_empty() {
            "No results".to_string()
        } else if let Some(current) = self.current_match {
            format!("{} of {}", current + 1, self.matches.len())
        } else if self.matches.is_empty() {
            String::new()
        } else {
            format!("{} results", self.matches.len())
        };

        if count_text.is_empty() {
            return;
        }

        let text_color = if self.matches.is_empty() && !self.search_text.is_empty() {
            self.no_match_color
        } else {
            self.text_color
        };

        let text_y = y + (Self::INPUT_HEIGHT - self.font.size()) / 2.0;

        let layout = TextLayout::new(font_system, &count_text, &self.font);
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                font_system,
                &layout,
                Point::new(x, text_y),
                text_color,
            );
        }
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Escape closes the bar
        if event.key == Key::Escape {
            self.close();
            return true;
        }

        // Enter triggers find next
        if event.key == Key::Enter {
            if event.modifiers.shift {
                self.find_previous();
            } else {
                self.find_next();
            }
            return true;
        }

        // Tab switches between fields
        if event.key == Key::Tab {
            if self.mode == FindReplaceMode::Replace {
                if event.modifiers.shift {
                    self.focused_field = if self.focused_field == 0 { 1 } else { 0 };
                } else {
                    self.focused_field = if self.focused_field == 1 { 0 } else { 1 };
                }
                self.base.update();
            }
            return true;
        }

        // Text input handling
        let (text, cursor) = if self.focused_field == 0 {
            (&mut self.search_text, &mut self.search_cursor)
        } else {
            (&mut self.replace_text, &mut self.replace_cursor)
        };

        match event.key {
            Key::Backspace => {
                if *cursor > 0 {
                    let prev_char_boundary = text[..*cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    text.drain(prev_char_boundary..*cursor);
                    *cursor = prev_char_boundary;
                    self.base.update();
                    if self.focused_field == 0 {
                        self.search_requested.emit(self.search_text.clone());
                    }
                }
                return true;
            }
            Key::Delete => {
                if *cursor < text.len() {
                    let next_char_boundary = text[*cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| *cursor + i)
                        .unwrap_or(text.len());
                    text.drain(*cursor..next_char_boundary);
                    self.base.update();
                    if self.focused_field == 0 {
                        self.search_requested.emit(self.search_text.clone());
                    }
                }
                return true;
            }
            Key::ArrowLeft => {
                if *cursor > 0 {
                    *cursor = text[..*cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.base.update();
                }
                return true;
            }
            Key::ArrowRight => {
                if *cursor < text.len() {
                    *cursor = text[*cursor..]
                        .char_indices()
                        .nth(1)
                        .map(|(i, _)| *cursor + i)
                        .unwrap_or(text.len());
                    self.base.update();
                }
                return true;
            }
            Key::Home => {
                *cursor = 0;
                self.base.update();
                return true;
            }
            Key::End => {
                *cursor = text.len();
                self.base.update();
                return true;
            }
            _ => {}
        }

        // Character input
        if !event.text.is_empty() && !event.modifiers.control && !event.modifiers.alt {
            text.insert_str(*cursor, &event.text);
            *cursor += event.text.len();
            self.base.update();
            if self.focused_field == 0 {
                self.search_requested.emit(self.search_text.clone());
            }
            return true;
        }

        false
    }

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;
        let y_offset = Self::PADDING;

        // Check if click is in search input
        let search_rect = Rect::new(Self::PADDING, y_offset, Self::INPUT_WIDTH, Self::INPUT_HEIGHT);
        if search_rect.contains(pos) {
            self.focused_field = 0;
            self.base.update();
            return true;
        }

        // Check if click is in replace input (if visible)
        if self.mode == FindReplaceMode::Replace {
            let replace_rect = Rect::new(
                Self::PADDING,
                y_offset + Self::ROW_HEIGHT,
                Self::INPUT_WIDTH,
                Self::INPUT_HEIGHT,
            );
            if replace_rect.contains(pos) {
                self.focused_field = 1;
                self.base.update();
                return true;
            }
        }

        // Check option buttons
        let mut x = Self::PADDING + Self::INPUT_WIDTH + Self::BUTTON_SPACING * 2.0;

        // Case sensitive
        if Rect::new(x, y_offset, Self::OPTION_BUTTON_WIDTH, Self::INPUT_HEIGHT).contains(pos) {
            self.toggle_case_sensitive();
            return true;
        }
        x += Self::OPTION_BUTTON_WIDTH + Self::BUTTON_SPACING;

        // Whole word
        if Rect::new(x, y_offset, Self::OPTION_BUTTON_WIDTH, Self::INPUT_HEIGHT).contains(pos) {
            self.toggle_whole_word();
            return true;
        }
        x += Self::OPTION_BUTTON_WIDTH + Self::BUTTON_SPACING;

        // Regex
        if Rect::new(x, y_offset, Self::OPTION_BUTTON_WIDTH, Self::INPUT_HEIGHT).contains(pos) {
            self.toggle_regex();
            return true;
        }
        x += Self::OPTION_BUTTON_WIDTH + Self::BUTTON_SPACING * 4.0;

        // Skip match count
        x += 70.0 + Self::BUTTON_SPACING * 2.0;

        // Previous button
        if Rect::new(x, y_offset, Self::BUTTON_SIZE, Self::INPUT_HEIGHT).contains(pos) {
            self.find_previous();
            return true;
        }
        x += Self::BUTTON_SIZE + Self::BUTTON_SPACING;

        // Next button
        if Rect::new(x, y_offset, Self::BUTTON_SIZE, Self::INPUT_HEIGHT).contains(pos) {
            self.find_next();
            return true;
        }
        x += Self::BUTTON_SIZE + Self::BUTTON_SPACING * 4.0;

        // Close button
        if Rect::new(x, y_offset, Self::BUTTON_SIZE, Self::INPUT_HEIGHT).contains(pos) {
            self.close();
            return true;
        }

        // Replace row buttons
        if self.mode == FindReplaceMode::Replace {
            let y = y_offset + Self::ROW_HEIGHT;
            let mut x = Self::PADDING + Self::INPUT_WIDTH + Self::BUTTON_SPACING * 2.0;

            // Replace current
            let replace_width = "Replace".len() as f32 * self.font.size() * 0.6 + 16.0;
            if Rect::new(x, y, replace_width, Self::INPUT_HEIGHT).contains(pos) {
                self.replace_current();
                return true;
            }
            x += replace_width + Self::BUTTON_SPACING;

            // Replace all
            let replace_all_width = "Replace All".len() as f32 * self.font.size() * 0.6 + 16.0;
            if Rect::new(x, y, replace_all_width, Self::INPUT_HEIGHT).contains(pos) {
                self.replace_all();
                return true;
            }
        }

        false
    }
}

impl Default for FindReplaceBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for FindReplaceBar {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for FindReplaceBar {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let height = match self.mode {
            FindReplaceMode::Find => Self::ROW_HEIGHT + Self::PADDING * 2.0,
            FindReplaceMode::Replace => Self::ROW_HEIGHT * 2.0 + Self::PADDING * 2.0,
        };
        SizeHint::from_dimensions(600.0, height)
            .with_minimum_dimensions(400.0, height)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);

        let mut font_system = FontSystem::new();
        self.paint_search_row(ctx, &mut font_system);
        self.paint_replace_row(ctx, &mut font_system);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::KeyPress(e) => {
                if self.handle_key_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MousePress(e) => {
                if self.handle_mouse_press(e) {
                    event.accept();
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_options_default() {
        let options = FindOptions::default();
        assert!(!options.case_sensitive);
        assert!(!options.whole_word);
        assert!(!options.use_regex);
        assert!(options.wrap_around);
    }

    #[test]
    fn test_find_options_builder() {
        let options = FindOptions::new()
            .with_case_sensitive(true)
            .with_whole_word(true)
            .with_regex(true)
            .with_wrap_around(false);

        assert!(options.case_sensitive);
        assert!(options.whole_word);
        assert!(options.use_regex);
        assert!(!options.wrap_around);
    }

    #[test]
    fn test_search_match() {
        let m = SearchMatch::new(10, 20);
        assert_eq!(m.start, 10);
        assert_eq!(m.end, 20);
        assert_eq!(m.len(), 10);
        assert!(!m.is_empty());

        let empty = SearchMatch::new(5, 5);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_find_replace_bar_new() {
        let bar = FindReplaceBar::new();
        assert_eq!(bar.mode(), FindReplaceMode::Find);
        assert!(bar.search_text().is_empty());
        assert!(bar.replace_text().is_empty());
        assert!(bar.matches().is_empty());
        assert_eq!(bar.current_match_index(), None);
    }

    #[test]
    fn test_find_replace_bar_mode() {
        let mut bar = FindReplaceBar::new();
        assert_eq!(bar.mode(), FindReplaceMode::Find);

        bar.set_mode(FindReplaceMode::Replace);
        assert_eq!(bar.mode(), FindReplaceMode::Replace);
    }

    #[test]
    fn test_find_replace_bar_text() {
        let mut bar = FindReplaceBar::new();

        bar.set_search_text("hello");
        assert_eq!(bar.search_text(), "hello");

        bar.set_replace_text("world");
        assert_eq!(bar.replace_text(), "world");
    }

    #[test]
    fn test_find_replace_bar_options() {
        let mut bar = FindReplaceBar::new();

        bar.toggle_case_sensitive();
        assert!(bar.options().case_sensitive);

        bar.toggle_whole_word();
        assert!(bar.options().whole_word);

        bar.toggle_regex();
        assert!(bar.options().use_regex);
    }

    #[test]
    fn test_find_replace_bar_matches() {
        let mut bar = FindReplaceBar::new();

        let matches = vec![
            SearchMatch::new(0, 5),
            SearchMatch::new(10, 15),
            SearchMatch::new(20, 25),
        ];
        bar.set_matches(matches);

        assert_eq!(bar.matches().len(), 3);
        assert_eq!(bar.current_match_index(), Some(0));
    }

    #[test]
    fn test_find_replace_bar_navigation() {
        let mut bar = FindReplaceBar::new();

        let matches = vec![
            SearchMatch::new(0, 5),
            SearchMatch::new(10, 15),
            SearchMatch::new(20, 25),
        ];
        bar.set_matches(matches);

        assert_eq!(bar.current_match_index(), Some(0));

        bar.find_next();
        assert_eq!(bar.current_match_index(), Some(1));

        bar.find_next();
        assert_eq!(bar.current_match_index(), Some(2));

        // Wrap around
        bar.find_next();
        assert_eq!(bar.current_match_index(), Some(0));

        bar.find_previous();
        assert_eq!(bar.current_match_index(), Some(2));
    }

    // Test implementation of Searchable for a simple mock
    struct MockSearchable {
        text: String,
        cursor: usize,
        selection: Option<(usize, usize)>,
        matches: Vec<SearchMatch>,
        current_match: Option<usize>,
    }

    impl MockSearchable {
        fn new(text: &str) -> Self {
            Self {
                text: text.to_string(),
                cursor: 0,
                selection: None,
                matches: Vec::new(),
                current_match: None,
            }
        }
    }

    impl Searchable for MockSearchable {
        fn search_text(&self) -> String {
            self.text.clone()
        }

        fn cursor_position(&self) -> usize {
            self.cursor
        }

        fn set_cursor_position(&mut self, pos: usize) {
            self.cursor = pos;
        }

        fn selection_range(&self) -> Option<(usize, usize)> {
            self.selection
        }

        fn set_selection(&mut self, start: usize, end: usize) {
            self.selection = Some((start, end));
        }

        fn clear_selection(&mut self) {
            self.selection = None;
        }

        fn replace_range(&mut self, start: usize, end: usize, replacement: &str) {
            self.text.replace_range(start..end, replacement);
        }

        fn scroll_to_position(&mut self, _pos: usize) {}

        fn set_search_matches(&mut self, matches: Vec<SearchMatch>) {
            self.matches = matches;
        }

        fn set_current_match_index(&mut self, index: Option<usize>) {
            self.current_match = index;
        }

        fn clear_search_highlights(&mut self) {
            self.matches.clear();
            self.current_match = None;
        }
    }

    #[test]
    fn test_searchable_find_all_simple() {
        let searchable = MockSearchable::new("hello world hello");
        let options = FindOptions::default();
        let matches = searchable.find_all("hello", &options);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], SearchMatch::new(0, 5));
        assert_eq!(matches[1], SearchMatch::new(12, 17));
    }

    #[test]
    fn test_searchable_find_all_case_insensitive() {
        let searchable = MockSearchable::new("Hello World HELLO");
        let options = FindOptions::default(); // case insensitive by default
        let matches = searchable.find_all("hello", &options);

        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_searchable_find_all_case_sensitive() {
        let searchable = MockSearchable::new("Hello World HELLO");
        let options = FindOptions::new().with_case_sensitive(true);
        let matches = searchable.find_all("Hello", &options);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], SearchMatch::new(0, 5));
    }

    #[test]
    fn test_searchable_find_all_whole_word() {
        let searchable = MockSearchable::new("hello helloworld hello");
        let options = FindOptions::new().with_whole_word(true);
        let matches = searchable.find_all("hello", &options);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], SearchMatch::new(0, 5));
        assert_eq!(matches[1], SearchMatch::new(17, 22));
    }

    #[test]
    fn test_searchable_find_all_regex() {
        let searchable = MockSearchable::new("cat bat hat sat");
        let options = FindOptions::new().with_regex(true);
        let matches = searchable.find_all("[cbh]at", &options);

        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_searchable_find_next() {
        let mut searchable = MockSearchable::new("aaa bbb aaa ccc aaa");
        let options = FindOptions::default();
        let matches = searchable.find_all("aaa", &options);

        searchable.cursor = 0;
        let next = searchable.find_next("aaa", &options, &matches);
        assert_eq!(next, Some(1)); // First match after cursor 0 is index 1 (at pos 8)

        searchable.cursor = 10;
        let next = searchable.find_next("aaa", &options, &matches);
        assert_eq!(next, Some(2)); // Match at position 16
    }

    #[test]
    fn test_searchable_find_previous() {
        let mut searchable = MockSearchable::new("aaa bbb aaa ccc aaa");
        let options = FindOptions::default();
        let matches = searchable.find_all("aaa", &options);

        searchable.cursor = 19;
        let prev = searchable.find_previous("aaa", &options, &matches);
        assert_eq!(prev, Some(2)); // Last match before cursor

        searchable.cursor = 10;
        let prev = searchable.find_previous("aaa", &options, &matches);
        assert_eq!(prev, Some(1)); // Match at position 8
    }

    #[test]
    fn test_searchable_replace_all() {
        let mut searchable = MockSearchable::new("hello world hello");
        let options = FindOptions::default();
        let count = searchable.replace_all("hello", "hi", &options);

        assert_eq!(count, 2);
        assert_eq!(searchable.text, "hi world hi");
    }
}
