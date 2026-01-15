//! Plain text editing widget optimized for large documents.
//!
//! The PlainTextEdit widget provides a plain text editor with support for:
//! - Large document handling via rope data structure
//! - Virtualized line rendering (only visible lines are rendered)
//! - Syntax highlighting via trait-based hooks
//! - Multi-line text display and editing
//! - Cursor movement (char, word, line, document)
//! - Text selection (keyboard and mouse)
//! - Copy, cut, paste operations
//! - Undo/redo with command coalescing
//! - Scrolling with scrollbars
//! - Read-only mode
//! - Placeholder text
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::PlainTextEdit;
//!
//! // Create a simple text editor
//! let mut editor = PlainTextEdit::new();
//! editor.set_placeholder("Enter your text...");
//!
//! // Connect to signals
//! editor.text_changed.connect(|text| {
//!     println!("Text changed: {} chars", text.len());
//! });
//!
//! editor.cursor_position_changed.connect(|(line, col)| {
//!     println!("Cursor at line {}, column {}", line, col);
//! });
//! ```

use parking_lot::RwLock;
use ropey::Rope;
use std::sync::Arc;

use crate::platform::Clipboard;
use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, Size, Stroke, TextLayout,
    TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, WheelEvent, Widget,
    WidgetBase, WidgetEvent,
};

// =========================================================================
// Line Numbers Configuration
// =========================================================================

/// Configuration for line number display in the gutter.
#[derive(Debug, Clone, PartialEq)]
pub struct LineNumberConfig {
    /// Whether line numbers are visible.
    pub visible: bool,
    /// Background color for the gutter area.
    pub background_color: Color,
    /// Text color for line numbers.
    pub text_color: Color,
    /// Text color for the current line number.
    pub current_line_color: Color,
    /// Background color for the current line in the gutter.
    pub current_line_background: Color,
    /// Minimum number of digits to display (pads with spaces).
    pub min_digits: usize,
    /// Padding on the left side of line numbers.
    pub padding_left: f32,
    /// Padding on the right side of line numbers.
    pub padding_right: f32,
}

impl Default for LineNumberConfig {
    fn default() -> Self {
        Self {
            visible: false,
            background_color: Color::from_rgb8(245, 245, 245),
            text_color: Color::from_rgb8(128, 128, 128),
            current_line_color: Color::from_rgb8(64, 64, 64),
            current_line_background: Color::from_rgb8(232, 232, 232),
            min_digits: 3,
            padding_left: 8.0,
            padding_right: 8.0,
        }
    }
}

impl LineNumberConfig {
    /// Create a new LineNumberConfig with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether line numbers are visible.
    pub fn with_visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set the gutter background color.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Set the line number text color.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the current line number text color.
    pub fn with_current_line_color(mut self, color: Color) -> Self {
        self.current_line_color = color;
        self
    }

    /// Set the current line background color in the gutter.
    pub fn with_current_line_background(mut self, color: Color) -> Self {
        self.current_line_background = color;
        self
    }
}

// =========================================================================
// Syntax Highlighting
// =========================================================================

/// A span of text with a specific style for syntax highlighting.
#[derive(Debug, Clone, PartialEq)]
pub struct HighlightSpan {
    /// Start column (0-indexed character offset within the line).
    pub start: usize,
    /// End column (exclusive, 0-indexed character offset).
    pub end: usize,
    /// Text color for this span.
    pub color: Color,
    /// Whether this span is bold.
    pub bold: bool,
    /// Whether this span is italic.
    pub italic: bool,
}

impl HighlightSpan {
    /// Create a new highlight span with the given range and color.
    pub fn new(start: usize, end: usize, color: Color) -> Self {
        Self {
            start,
            end,
            color,
            bold: false,
            italic: false,
        }
    }

    /// Create a highlight span with bold style.
    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    /// Create a highlight span with italic style.
    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

/// Trait for syntax highlighting implementations.
///
/// Implement this trait to provide syntax highlighting for PlainTextEdit.
/// The highlighter is called for each visible line when rendering.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::widgets::{SyntaxHighlighter, HighlightSpan};
/// use horizon_lattice_render::Color;
///
/// struct KeywordHighlighter;
///
/// impl SyntaxHighlighter for KeywordHighlighter {
///     fn highlight_line(&self, line: &str, line_number: usize) -> Vec<HighlightSpan> {
///         let mut spans = Vec::new();
///         // Highlight "fn" keyword
///         for (idx, _) in line.match_indices("fn ") {
///             spans.push(HighlightSpan::new(idx, idx + 2, Color::from_rgb8(86, 156, 214)));
///         }
///         spans
///     }
/// }
/// ```
pub trait SyntaxHighlighter: Send + Sync {
    /// Highlight a single line of text.
    ///
    /// Returns a vector of highlight spans for the given line.
    /// The spans should not overlap and should be sorted by start position.
    ///
    /// # Arguments
    ///
    /// * `line` - The text content of the line (without trailing newline)
    /// * `line_number` - The 0-indexed line number
    fn highlight_line(&self, line: &str, line_number: usize) -> Vec<HighlightSpan>;

    /// Called when the document text changes.
    ///
    /// Override this to update any internal state when the document changes.
    /// The default implementation does nothing.
    fn on_text_changed(&mut self, _text: &Rope) {}
}

// =========================================================================
// Undo/Redo System
// =========================================================================

/// Represents an undoable edit operation.
#[derive(Debug, Clone, PartialEq)]
enum EditCommand {
    /// Text was inserted at a position.
    Insert {
        /// Character position where text was inserted.
        pos: usize,
        /// The inserted text.
        text: String,
    },
    /// Text was deleted from a range.
    Delete {
        /// Character position where deletion started.
        pos: usize,
        /// The deleted text.
        text: String,
    },
}

impl EditCommand {
    /// Try to merge another command into this one for coalescing.
    fn try_merge(&mut self, other: &EditCommand) -> bool {
        match (self, other) {
            // Merge consecutive insertions (typing characters)
            (
                EditCommand::Insert { pos, text },
                EditCommand::Insert {
                    pos: other_pos,
                    text: other_text,
                },
            ) => {
                // Can merge if the new insertion is at the end of the current one
                // and doesn't contain newlines
                if *pos + text.chars().count() == *other_pos && !other_text.contains('\n') {
                    text.push_str(other_text);
                    true
                } else {
                    false
                }
            }
            // Merge consecutive backspace deletions
            (
                EditCommand::Delete { pos, text },
                EditCommand::Delete {
                    pos: other_pos,
                    text: other_text,
                },
            ) => {
                // Backspace: deleting characters before current position
                if *other_pos + other_text.chars().count() == *pos && !other_text.contains('\n') {
                    // Prepend the deleted text
                    let mut new_text = other_text.clone();
                    new_text.push_str(text);
                    *text = new_text;
                    *pos = *other_pos;
                    true
                }
                // Forward delete: deleting characters at current position
                else if *other_pos == *pos && !other_text.contains('\n') {
                    text.push_str(other_text);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}

/// Manages undo/redo history.
struct UndoStack {
    /// Stack of edit commands.
    commands: Vec<EditCommand>,
    /// Current position in the stack.
    index: usize,
    /// Maximum stack size.
    max_size: usize,
    /// Whether command merging is enabled.
    merge_enabled: bool,
}

impl UndoStack {
    fn new() -> Self {
        Self {
            commands: Vec::new(),
            index: 0,
            max_size: 100,
            merge_enabled: true,
        }
    }

    fn push(&mut self, command: EditCommand) {
        // Remove any commands after current position
        self.commands.truncate(self.index);

        // Try to merge with the last command if merging is enabled
        if self.merge_enabled {
            if let Some(last) = self.commands.last_mut() {
                if last.try_merge(&command) {
                    return;
                }
            }
        }

        // Add new command
        self.commands.push(command);
        self.index = self.commands.len();

        // Limit stack size
        if self.commands.len() > self.max_size {
            self.commands.remove(0);
            self.index = self.commands.len();
        }
    }

    fn undo(&mut self) -> Option<EditCommand> {
        if self.index > 0 {
            self.index -= 1;
            Some(self.commands[self.index].clone())
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<EditCommand> {
        if self.index < self.commands.len() {
            let cmd = self.commands[self.index].clone();
            self.index += 1;
            Some(cmd)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.commands.clear();
        self.index = 0;
    }

    fn can_undo(&self) -> bool {
        self.index > 0
    }

    fn can_redo(&self) -> bool {
        self.index < self.commands.len()
    }

    fn break_merge(&mut self) {
        self.merge_enabled = false;
    }

    fn enable_merge(&mut self) {
        self.merge_enabled = true;
    }
}

// =========================================================================
// Cached Layout
// =========================================================================

/// Cached layout information for a range of lines.
#[allow(dead_code)]
struct CachedLineLayout {
    /// The line number this layout is for.
    line_number: usize,
    /// The text layout for this line.
    layout: TextLayout,
    /// The text content of this line (for cache validation).
    text: String,
}

/// Layout cache for virtualized rendering.
#[allow(dead_code)]
struct LayoutCache {
    /// Cached layouts for visible lines.
    lines: Vec<CachedLineLayout>,
    /// Width used for layout.
    width: Option<f32>,
}

impl LayoutCache {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            width: None,
        }
    }

    fn invalidate(&mut self) {
        self.lines.clear();
    }
}

// =========================================================================
// PlainTextEdit Widget
// =========================================================================

/// A plain text editor widget optimized for large documents.
///
/// PlainTextEdit uses a rope data structure for efficient handling of large
/// texts and supports virtualized rendering where only visible lines are
/// rendered. It also supports syntax highlighting via a trait-based system.
///
/// # Signals
///
/// - `text_changed`: Emitted when the text content changes
/// - `cursor_position_changed`: Emitted when cursor moves (line, column)
/// - `selection_changed`: Emitted when selection changes
pub struct PlainTextEdit {
    /// Widget base.
    base: WidgetBase,

    /// The text content stored in a rope.
    rope: Rope,

    /// Placeholder text displayed when empty.
    placeholder: String,

    /// Current cursor position (character offset).
    cursor_pos: usize,

    /// Selection anchor position. If Some, selection extends from anchor to cursor.
    selection_anchor: Option<usize>,

    /// Whether the widget is read-only.
    read_only: bool,

    /// The font for text rendering.
    font: Font,

    /// Text color.
    text_color: Color,

    /// Placeholder text color.
    placeholder_color: Color,

    /// Selection background color.
    selection_color: Color,

    /// Background color.
    background_color: Color,

    /// Border color.
    border_color: Color,

    /// Focused border color.
    focus_border_color: Color,

    /// Horizontal scroll offset.
    scroll_x: f32,

    /// Vertical scroll offset.
    scroll_y: f32,

    /// Scrollbar thickness.
    scrollbar_thickness: f32,

    /// Whether cursor is visible (for blinking).
    cursor_visible: bool,

    /// Layout cache for virtualized rendering.
    layout_cache: RwLock<LayoutCache>,

    /// Whether we're currently dragging to select.
    is_dragging: bool,

    /// Undo/redo stack.
    undo_stack: UndoStack,

    /// Tab width in spaces.
    tab_width: usize,

    /// Syntax highlighter (optional).
    highlighter: Option<Arc<RwLock<dyn SyntaxHighlighter>>>,

    /// Cached line height.
    line_height: f32,

    /// Line number configuration.
    line_number_config: LineNumberConfig,

    /// Cached gutter width (recalculated when line count changes significantly).
    cached_gutter_width: f32,

    /// Line count when gutter width was last calculated.
    cached_line_count_for_gutter: usize,

    /// Search matches for highlighting.
    search_matches: Vec<super::find_replace::SearchMatch>,

    /// Current (focused) search match index.
    current_search_match: Option<usize>,

    /// Search match highlight color.
    search_highlight_color: Color,

    /// Current search match highlight color.
    current_search_highlight_color: Color,

    // Signals

    /// Signal emitted when text changes.
    pub text_changed: Signal<String>,

    /// Signal emitted when cursor position changes (line, column).
    pub cursor_position_changed: Signal<(usize, usize)>,

    /// Signal emitted when selection changes.
    pub selection_changed: Signal<()>,

    /// Signal emitted when find is requested (Ctrl+F).
    pub find_requested: Signal<()>,

    /// Signal emitted when find and replace is requested (Ctrl+H).
    pub find_replace_requested: Signal<()>,

    /// Signal emitted when find next is requested (F3).
    pub find_next_requested: Signal<()>,

    /// Signal emitted when find previous is requested (Shift+F3).
    pub find_previous_requested: Signal<()>,
}

impl PlainTextEdit {
    /// Create a new empty PlainTextEdit.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Expanding));

        let font = Font::new(FontFamily::Monospace, 14.0);
        let line_height = font.size() * 1.2;

        Self {
            base,
            rope: Rope::new(),
            placeholder: String::new(),
            cursor_pos: 0,
            selection_anchor: None,
            read_only: false,
            font,
            text_color: Color::BLACK,
            placeholder_color: Color::from_rgb8(160, 160, 160),
            selection_color: Color::from_rgba8(51, 153, 255, 128),
            background_color: Color::WHITE,
            border_color: Color::from_rgb8(200, 200, 200),
            focus_border_color: Color::from_rgb8(51, 153, 255),
            scroll_x: 0.0,
            scroll_y: 0.0,
            scrollbar_thickness: 12.0,
            cursor_visible: true,
            layout_cache: RwLock::new(LayoutCache::new()),
            is_dragging: false,
            undo_stack: UndoStack::new(),
            tab_width: 4,
            highlighter: None,
            line_height,
            line_number_config: LineNumberConfig::default(),
            cached_gutter_width: 0.0,
            cached_line_count_for_gutter: 0,
            search_matches: Vec::new(),
            current_search_match: None,
            search_highlight_color: Color::from_rgba8(255, 255, 0, 100),
            current_search_highlight_color: Color::from_rgba8(255, 165, 0, 150),
            text_changed: Signal::new(),
            cursor_position_changed: Signal::new(),
            selection_changed: Signal::new(),
            find_requested: Signal::new(),
            find_replace_requested: Signal::new(),
            find_next_requested: Signal::new(),
            find_previous_requested: Signal::new(),
        }
    }

    /// Create a new PlainTextEdit with initial text.
    pub fn with_text(text: impl Into<String>) -> Self {
        let mut edit = Self::new();
        let text = text.into();
        edit.rope = Rope::from_str(&text);
        edit.cursor_pos = edit.rope.len_chars();
        edit
    }

    // =========================================================================
    // Text Access
    // =========================================================================

    /// Get the current text as a String.
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Get the rope for direct access.
    pub fn rope(&self) -> &Rope {
        &self.rope
    }

    /// Get the number of characters in the document.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Get the number of lines in the document.
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    /// Check if the document is empty.
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Set the text content.
    ///
    /// This clears any selection, moves the cursor to the end, and clears
    /// the undo history.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let new_text = text.into();
        let current_text = self.rope.to_string();
        if current_text != new_text {
            self.rope = Rope::from_str(&new_text);
            self.cursor_pos = self.rope.len_chars();
            self.selection_anchor = None;
            self.undo_stack.clear();
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.notify_highlighter();
            self.text_changed.emit(self.rope.to_string());
            self.emit_cursor_position();
        }
    }

    /// Get the plain text content.
    pub fn to_plain_text(&self) -> String {
        self.rope.to_string()
    }

    /// Get the text of a specific line.
    pub fn line(&self, line_idx: usize) -> Option<String> {
        if line_idx < self.rope.len_lines() {
            Some(self.rope.line(line_idx).to_string())
        } else {
            None
        }
    }

    /// Get the placeholder text.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Set the placeholder text.
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        let new_placeholder = placeholder.into();
        if self.placeholder != new_placeholder {
            self.placeholder = new_placeholder;
            self.base.update();
        }
    }

    /// Set placeholder using builder pattern.
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    // =========================================================================
    // Read-Only Mode
    // =========================================================================

    /// Check if the widget is read-only.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Set read-only mode.
    pub fn set_read_only(&mut self, read_only: bool) {
        self.read_only = read_only;
    }

    /// Set read-only mode using builder pattern.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    // =========================================================================
    // Syntax Highlighting
    // =========================================================================

    /// Set the syntax highlighter.
    pub fn set_highlighter<H: SyntaxHighlighter + 'static>(&mut self, highlighter: H) {
        self.highlighter = Some(Arc::new(RwLock::new(highlighter)));
        self.base.update();
    }

    /// Set the syntax highlighter using builder pattern.
    pub fn with_highlighter<H: SyntaxHighlighter + 'static>(mut self, highlighter: H) -> Self {
        self.highlighter = Some(Arc::new(RwLock::new(highlighter)));
        self
    }

    /// Clear the syntax highlighter.
    pub fn clear_highlighter(&mut self) {
        self.highlighter = None;
        self.base.update();
    }

    /// Notify the highlighter of text changes.
    fn notify_highlighter(&mut self) {
        if let Some(ref highlighter) = self.highlighter {
            if let Some(mut hl) = highlighter.try_write() {
                hl.on_text_changed(&self.rope);
            }
        }
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the font.
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// Set the font.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
        self.line_height = self.font.size() * 1.2;
        self.invalidate_layout();
        self.base.update();
    }

    /// Set font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self.line_height = self.font.size() * 1.2;
        self
    }

    /// Get the text color.
    pub fn text_color(&self) -> Color {
        self.text_color
    }

    /// Set the text color.
    pub fn set_text_color(&mut self, color: Color) {
        if self.text_color != color {
            self.text_color = color;
            self.base.update();
        }
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Get the background color.
    pub fn background_color(&self) -> Color {
        self.background_color
    }

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Color) {
        if self.background_color != color {
            self.background_color = color;
            self.base.update();
        }
    }

    /// Set background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Get the tab width in spaces.
    pub fn tab_width(&self) -> usize {
        self.tab_width
    }

    /// Set the tab width in spaces.
    pub fn set_tab_width(&mut self, width: usize) {
        self.tab_width = width;
    }

    // =========================================================================
    // Cursor and Selection
    // =========================================================================

    /// Get the cursor position as (line, column).
    pub fn cursor_position(&self) -> (usize, usize) {
        self.char_pos_to_line_col(self.cursor_pos)
    }

    /// Set the cursor position by (line, column).
    pub fn set_cursor_position(&mut self, line: usize, col: usize) {
        let new_pos = self.line_col_to_char_pos(line, col);
        if self.cursor_pos != new_pos {
            self.cursor_pos = new_pos;
            self.selection_anchor = None;
            self.ensure_cursor_visible();
            self.base.update();
            self.emit_cursor_position();
        }
    }

    /// Check if there is a selection.
    pub fn has_selection(&self) -> bool {
        self.selection_anchor.is_some() && self.selection_anchor != Some(self.cursor_pos)
    }

    /// Get the selected text.
    pub fn selected_text(&self) -> String {
        if let Some(anchor) = self.selection_anchor {
            let start = anchor.min(self.cursor_pos);
            let end = anchor.max(self.cursor_pos);
            if start < end && end <= self.rope.len_chars() {
                self.rope.slice(start..end).to_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }

    /// Select all text.
    pub fn select_all(&mut self) {
        if self.rope.len_chars() != 0 {
            self.selection_anchor = Some(0);
            self.cursor_pos = self.rope.len_chars();
            self.base.update();
            self.selection_changed.emit(());
        }
    }

    /// Clear the selection without deleting text.
    pub fn clear_selection(&mut self) {
        if self.selection_anchor.is_some() {
            self.selection_anchor = None;
            self.base.update();
            self.selection_changed.emit(());
        }
    }

    /// Get selection range as (start, end) in character positions.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            let start = anchor.min(self.cursor_pos);
            let end = anchor.max(self.cursor_pos);
            (start, end)
        })
    }

    // =========================================================================
    // Position Conversion
    // =========================================================================

    /// Convert character position to (line, column).
    fn char_pos_to_line_col(&self, pos: usize) -> (usize, usize) {
        let pos = pos.min(self.rope.len_chars());
        let line = self.rope.char_to_line(pos);
        let line_start = self.rope.line_to_char(line);
        let col = pos - line_start;
        (line, col)
    }

    /// Convert (line, column) to character position.
    fn line_col_to_char_pos(&self, line: usize, col: usize) -> usize {
        let line = line.min(self.rope.len_lines().saturating_sub(1));
        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();
        // Don't count the newline character at the end
        let line_len = if line < self.rope.len_lines() - 1 {
            line_len.saturating_sub(1)
        } else {
            line_len
        };
        let col = col.min(line_len);
        line_start + col
    }

    /// Convert pixel coordinates to character position.
    fn pixel_to_char_pos(&self, x: f32, y: f32) -> usize {
        let content_rect = self.content_rect();

        // Adjust for scroll
        let x = x - content_rect.origin.x + self.scroll_x;
        let y = y - content_rect.origin.y + self.scroll_y;

        // Calculate line from y position
        let line = (y / self.line_height).floor() as usize;
        let line = line.min(self.rope.len_lines().saturating_sub(1));

        // Calculate column from x position
        let char_width = self.font.size() * 0.6; // Approximate
        let col = (x / char_width).round() as usize;

        self.line_col_to_char_pos(line, col)
    }

    /// Get cursor position in pixels relative to content rect.
    fn cursor_position_pixels(&self) -> (f32, f32) {
        let (line, col) = self.char_pos_to_line_col(self.cursor_pos);
        let char_width = self.font.size() * 0.6;
        let x = col as f32 * char_width;
        let y = line as f32 * self.line_height;
        (x, y)
    }

    // =========================================================================
    // Editing Operations
    // =========================================================================

    /// Insert text at the cursor position.
    pub fn insert_text(&mut self, text: &str) {
        if self.read_only {
            return;
        }

        // Delete selection first if any
        self.delete_selection_internal();

        // Record for undo
        self.undo_stack.push(EditCommand::Insert {
            pos: self.cursor_pos,
            text: text.to_string(),
        });

        // Insert text
        self.rope.insert(self.cursor_pos, text);
        self.cursor_pos += text.chars().count();

        self.invalidate_layout();
        self.ensure_cursor_visible();
        self.base.update();
        self.notify_highlighter();
        self.text_changed.emit(self.rope.to_string());
        self.emit_cursor_position();
    }

    /// Delete the selected text.
    pub fn delete_selection(&mut self) {
        if self.read_only || !self.has_selection() {
            return;
        }

        self.delete_selection_internal();
        self.invalidate_layout();
        self.ensure_cursor_visible();
        self.base.update();
        self.notify_highlighter();
        self.text_changed.emit(self.rope.to_string());
        self.emit_cursor_position();
    }

    /// Internal method to delete selection without emitting signals.
    fn delete_selection_internal(&mut self) {
        if let Some(anchor) = self.selection_anchor {
            let start = anchor.min(self.cursor_pos);
            let end = anchor.max(self.cursor_pos);
            if start < end && end <= self.rope.len_chars() {
                let deleted = self.rope.slice(start..end).to_string();

                // Record for undo
                self.undo_stack.push(EditCommand::Delete {
                    pos: start,
                    text: deleted,
                });

                self.rope.remove(start..end);
                self.cursor_pos = start;
                self.selection_anchor = None;
            }
        }
    }

    /// Delete the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection();
            return;
        }

        if self.cursor_pos > 0 {
            let delete_pos = self.cursor_pos - 1;
            let deleted = self.rope.slice(delete_pos..self.cursor_pos).to_string();

            self.undo_stack.push(EditCommand::Delete {
                pos: delete_pos,
                text: deleted,
            });

            self.rope.remove(delete_pos..self.cursor_pos);
            self.cursor_pos = delete_pos;

            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.notify_highlighter();
            self.text_changed.emit(self.rope.to_string());
            self.emit_cursor_position();
        }
    }

    /// Delete the character after the cursor (delete key).
    pub fn delete_forward(&mut self) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection();
            return;
        }

        if self.cursor_pos < self.rope.len_chars() {
            let deleted = self.rope.slice(self.cursor_pos..self.cursor_pos + 1).to_string();

            self.undo_stack.push(EditCommand::Delete {
                pos: self.cursor_pos,
                text: deleted,
            });

            self.rope.remove(self.cursor_pos..self.cursor_pos + 1);

            self.invalidate_layout();
            self.base.update();
            self.notify_highlighter();
            self.text_changed.emit(self.rope.to_string());
        }
    }

    /// Clear all text.
    pub fn clear(&mut self) {
        if self.read_only || self.rope.len_chars() == 0 {
            return;
        }

        let old_text = self.rope.to_string();
        self.undo_stack.push(EditCommand::Delete {
            pos: 0,
            text: old_text,
        });

        self.rope = Rope::new();
        self.cursor_pos = 0;
        self.selection_anchor = None;

        self.invalidate_layout();
        self.base.update();
        self.notify_highlighter();
        self.text_changed.emit(String::new());
        self.emit_cursor_position();
    }

    // =========================================================================
    // Clipboard Operations
    // =========================================================================

    /// Copy selected text to clipboard.
    pub fn copy(&self) {
        if self.has_selection() {
            let text = self.selected_text();
            if !text.is_empty() {
                if let Ok(mut clipboard) = Clipboard::new() {
                    let _ = clipboard.set_text(&text);
                }
            }
        }
    }

    /// Cut selected text to clipboard.
    pub fn cut(&mut self) {
        if self.has_selection() {
            self.copy();
            self.delete_selection();
        }
    }

    /// Paste text from clipboard.
    pub fn paste(&mut self) {
        if self.read_only {
            return;
        }

        if let Ok(mut clipboard) = Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                self.insert_text(&text);
            }
        }
    }

    // =========================================================================
    // Undo/Redo
    // =========================================================================

    /// Undo the last edit.
    pub fn undo(&mut self) {
        if self.read_only {
            return;
        }

        self.undo_stack.break_merge();
        if let Some(cmd) = self.undo_stack.undo() {
            match cmd {
                EditCommand::Insert { pos, text } => {
                    // Undo insert = delete
                    let end = pos + text.chars().count();
                    self.rope.remove(pos..end);
                    self.cursor_pos = pos;
                }
                EditCommand::Delete { pos, text } => {
                    // Undo delete = insert
                    self.rope.insert(pos, &text);
                    self.cursor_pos = pos + text.chars().count();
                }
            }
            self.selection_anchor = None;
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.notify_highlighter();
            self.text_changed.emit(self.rope.to_string());
            self.emit_cursor_position();
        }
        self.undo_stack.enable_merge();
    }

    /// Redo the last undone edit.
    pub fn redo(&mut self) {
        if self.read_only {
            return;
        }

        self.undo_stack.break_merge();
        if let Some(cmd) = self.undo_stack.redo() {
            match cmd {
                EditCommand::Insert { pos, text } => {
                    // Redo insert = insert
                    self.rope.insert(pos, &text);
                    self.cursor_pos = pos + text.chars().count();
                }
                EditCommand::Delete { pos, text } => {
                    // Redo delete = delete
                    let end = pos + text.chars().count();
                    self.rope.remove(pos..end);
                    self.cursor_pos = pos;
                }
            }
            self.selection_anchor = None;
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.notify_highlighter();
            self.text_changed.emit(self.rope.to_string());
            self.emit_cursor_position();
        }
        self.undo_stack.enable_merge();
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }

    // =========================================================================
    // Cursor Movement
    // =========================================================================

    /// Move cursor left by one character.
    pub fn move_cursor_left(&mut self, extend_selection: bool) {
        if !extend_selection && self.has_selection() {
            let (start, _) = self.selection_range().unwrap();
            self.cursor_pos = start;
            self.selection_anchor = None;
            self.selection_changed.emit(());
        } else if self.cursor_pos > 0 {
            if extend_selection && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
            self.cursor_pos -= 1;
            if extend_selection {
                self.selection_changed.emit(());
            }
        }
        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Move cursor right by one character.
    pub fn move_cursor_right(&mut self, extend_selection: bool) {
        if !extend_selection && self.has_selection() {
            let (_, end) = self.selection_range().unwrap();
            self.cursor_pos = end;
            self.selection_anchor = None;
            self.selection_changed.emit(());
        } else if self.cursor_pos < self.rope.len_chars() {
            if extend_selection && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
            self.cursor_pos += 1;
            if extend_selection {
                self.selection_changed.emit(());
            }
        }
        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Move cursor up by one line.
    pub fn move_cursor_up(&mut self, extend_selection: bool) {
        let (line, col) = self.char_pos_to_line_col(self.cursor_pos);

        if line > 0 {
            if extend_selection && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            } else if !extend_selection {
                self.selection_anchor = None;
            }

            self.cursor_pos = self.line_col_to_char_pos(line - 1, col);

            if extend_selection {
                self.selection_changed.emit(());
            }
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Move cursor down by one line.
    pub fn move_cursor_down(&mut self, extend_selection: bool) {
        let (line, col) = self.char_pos_to_line_col(self.cursor_pos);

        if line < self.rope.len_lines().saturating_sub(1) {
            if extend_selection && self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            } else if !extend_selection {
                self.selection_anchor = None;
            }

            self.cursor_pos = self.line_col_to_char_pos(line + 1, col);

            if extend_selection {
                self.selection_changed.emit(());
            }
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Move cursor to the start of the current line.
    pub fn move_cursor_to_line_start(&mut self, extend_selection: bool) {
        let (line, _) = self.char_pos_to_line_col(self.cursor_pos);

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        self.cursor_pos = self.rope.line_to_char(line);

        if extend_selection {
            self.selection_changed.emit(());
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Move cursor to the end of the current line.
    pub fn move_cursor_to_line_end(&mut self, extend_selection: bool) {
        let (line, _) = self.char_pos_to_line_col(self.cursor_pos);

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let line_start = self.rope.line_to_char(line);
        let line_len = self.rope.line(line).len_chars();
        // Don't include the newline
        let line_end = if line < self.rope.len_lines() - 1 {
            line_start + line_len.saturating_sub(1)
        } else {
            line_start + line_len
        };
        self.cursor_pos = line_end;

        if extend_selection {
            self.selection_changed.emit(());
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Move cursor to the start of the document.
    pub fn move_cursor_to_start(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        self.cursor_pos = 0;

        if extend_selection {
            self.selection_changed.emit(());
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Move cursor to the end of the document.
    pub fn move_cursor_to_end(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        self.cursor_pos = self.rope.len_chars();

        if extend_selection {
            self.selection_changed.emit(());
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Move cursor left by one word.
    pub fn move_cursor_word_left(&mut self, extend_selection: bool) {
        if self.cursor_pos == 0 {
            return;
        }

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        // Get text before cursor
        let text_before: String = self.rope.slice(..self.cursor_pos).to_string();

        // Skip whitespace, then skip word characters
        let chars: Vec<char> = text_before.chars().collect();
        let mut idx = chars.len();
        while idx > 0 && chars[idx - 1].is_whitespace() {
            idx -= 1;
        }
        // Skip word characters
        while idx > 0 && !chars[idx - 1].is_whitespace() {
            idx -= 1;
        }

        self.cursor_pos = idx;

        if extend_selection {
            self.selection_changed.emit(());
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Move cursor right by one word.
    pub fn move_cursor_word_right(&mut self, extend_selection: bool) {
        let len = self.rope.len_chars();
        if self.cursor_pos >= len {
            return;
        }

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        // Get text after cursor
        let text_after: String = self.rope.slice(self.cursor_pos..).to_string();
        let chars: Vec<char> = text_after.chars().collect();
        let mut idx = 0;

        // Skip word characters
        while idx < chars.len() && !chars[idx].is_whitespace() {
            idx += 1;
        }
        // Skip whitespace
        while idx < chars.len() && chars[idx].is_whitespace() {
            idx += 1;
        }

        self.cursor_pos += idx;

        if extend_selection {
            self.selection_changed.emit(());
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Page up.
    pub fn page_up(&mut self, extend_selection: bool) {
        let content_rect = self.content_rect();
        let visible_lines = (content_rect.height() / self.line_height).floor() as usize;
        let (line, col) = self.char_pos_to_line_col(self.cursor_pos);

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let new_line = line.saturating_sub(visible_lines);
        self.cursor_pos = self.line_col_to_char_pos(new_line, col);

        if extend_selection {
            self.selection_changed.emit(());
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    /// Page down.
    pub fn page_down(&mut self, extend_selection: bool) {
        let content_rect = self.content_rect();
        let visible_lines = (content_rect.height() / self.line_height).floor() as usize;
        let (line, col) = self.char_pos_to_line_col(self.cursor_pos);

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let max_line = self.rope.len_lines().saturating_sub(1);
        let new_line = (line + visible_lines).min(max_line);
        self.cursor_pos = self.line_col_to_char_pos(new_line, col);

        if extend_selection {
            self.selection_changed.emit(());
        }

        self.ensure_cursor_visible();
        self.base.update();
        self.emit_cursor_position();
    }

    // =========================================================================
    // Line Numbers
    // =========================================================================

    /// Check if line numbers are visible.
    pub fn line_numbers_visible(&self) -> bool {
        self.line_number_config.visible
    }

    /// Set whether line numbers are visible.
    pub fn set_line_numbers_visible(&mut self, visible: bool) {
        if self.line_number_config.visible != visible {
            self.line_number_config.visible = visible;
            self.invalidate_gutter_width();
            self.base.update();
        }
    }

    /// Set line numbers visible using builder pattern.
    pub fn with_line_numbers(mut self, visible: bool) -> Self {
        self.line_number_config.visible = visible;
        self
    }

    /// Get the line number configuration.
    pub fn line_number_config(&self) -> &LineNumberConfig {
        &self.line_number_config
    }

    /// Set the line number configuration.
    pub fn set_line_number_config(&mut self, config: LineNumberConfig) {
        self.line_number_config = config;
        self.invalidate_gutter_width();
        self.base.update();
    }

    /// Set line number configuration using builder pattern.
    pub fn with_line_number_config(mut self, config: LineNumberConfig) -> Self {
        self.line_number_config = config;
        self
    }

    /// Invalidate the cached gutter width.
    fn invalidate_gutter_width(&mut self) {
        self.cached_line_count_for_gutter = 0;
        self.cached_gutter_width = 0.0;
    }

    /// Calculate the width needed for line numbers.
    ///
    /// This caches the result and only recalculates when the number of digits
    /// in the line count changes.
    #[allow(dead_code)]
    fn gutter_width(&mut self) -> f32 {
        if !self.line_number_config.visible {
            return 0.0;
        }

        let line_count = self.rope.len_lines();

        // Check if we need to recalculate (when digit count changes)
        let current_digits = Self::digit_count(line_count);
        let cached_digits = Self::digit_count(self.cached_line_count_for_gutter);

        if current_digits != cached_digits || self.cached_gutter_width == 0.0 {
            let display_digits = current_digits.max(self.line_number_config.min_digits);
            let char_width = self.font.size() * 0.6;
            self.cached_gutter_width = self.line_number_config.padding_left
                + (display_digits as f32 * char_width)
                + self.line_number_config.padding_right;
            self.cached_line_count_for_gutter = line_count;
        }

        self.cached_gutter_width
    }

    /// Get the gutter width without mutating (for const contexts).
    fn gutter_width_const(&self) -> f32 {
        if !self.line_number_config.visible {
            return 0.0;
        }

        let line_count = self.rope.len_lines();
        let display_digits = Self::digit_count(line_count).max(self.line_number_config.min_digits);
        let char_width = self.font.size() * 0.6;

        self.line_number_config.padding_left
            + (display_digits as f32 * char_width)
            + self.line_number_config.padding_right
    }

    /// Count the number of digits in a number.
    fn digit_count(n: usize) -> usize {
        if n == 0 {
            1
        } else {
            ((n as f64).log10().floor() as usize) + 1
        }
    }

    // =========================================================================
    // Scrolling
    // =========================================================================

    /// Get the content rectangle (excluding scrollbars and gutter).
    fn content_rect(&self) -> Rect {
        let rect = self.base.rect();
        let padding = 4.0;
        let gutter_width = self.gutter_width_const();

        // Reserve space for scrollbars and gutter
        let width = rect.width() - padding * 2.0 - self.scrollbar_thickness - gutter_width;
        let height = rect.height() - padding * 2.0 - self.scrollbar_thickness;

        Rect::new(
            rect.origin.x + padding + gutter_width,
            rect.origin.y + padding,
            width.max(0.0),
            height.max(0.0),
        )
    }

    /// Get the gutter rectangle.
    fn gutter_rect(&self) -> Rect {
        let rect = self.base.rect();
        let padding = 4.0;
        let gutter_width = self.gutter_width_const();

        Rect::new(
            rect.origin.x + padding,
            rect.origin.y + padding,
            gutter_width,
            rect.height() - padding * 2.0 - self.scrollbar_thickness,
        )
    }

    /// Get the total content size.
    fn content_size(&self) -> Size {
        let line_count = self.rope.len_lines().max(1);
        let height = line_count as f32 * self.line_height;

        // Estimate max line width
        let mut max_width = 0.0f32;
        let char_width = self.font.size() * 0.6;
        for line_idx in 0..line_count.min(1000) {
            let line = self.rope.line(line_idx);
            let width = line.len_chars() as f32 * char_width;
            max_width = max_width.max(width);
        }

        Size::new(max_width, height)
    }

    /// Get the maximum scroll values.
    fn max_scroll(&self) -> (f32, f32) {
        let content_rect = self.content_rect();
        let content_size = self.content_size();

        let max_x = (content_size.width - content_rect.width()).max(0.0);
        let max_y = (content_size.height - content_rect.height()).max(0.0);

        (max_x, max_y)
    }

    /// Clamp scroll values to valid range.
    fn clamp_scroll(&mut self) {
        let (max_x, max_y) = self.max_scroll();
        self.scroll_x = self.scroll_x.clamp(0.0, max_x);
        self.scroll_y = self.scroll_y.clamp(0.0, max_y);
    }

    /// Ensure the cursor is visible.
    fn ensure_cursor_visible(&mut self) {
        let (cursor_x, cursor_y) = self.cursor_position_pixels();
        self.ensure_visible(cursor_x, cursor_y);
    }

    /// Ensure a point is visible.
    fn ensure_visible(&mut self, x: f32, y: f32) {
        let content_rect = self.content_rect();
        let margin = 5.0;

        // Horizontal scrolling
        if x < self.scroll_x + margin {
            self.scroll_x = (x - margin).max(0.0);
        } else if x > self.scroll_x + content_rect.width() - margin {
            self.scroll_x = x - content_rect.width() + margin;
        }

        // Vertical scrolling
        if y < self.scroll_y + margin {
            self.scroll_y = (y - margin).max(0.0);
        } else if y + self.line_height > self.scroll_y + content_rect.height() - margin {
            self.scroll_y = y + self.line_height - content_rect.height() + margin;
        }

        self.clamp_scroll();
    }

    /// Get the range of visible lines.
    fn visible_line_range(&self) -> (usize, usize) {
        let content_rect = self.content_rect();
        let first_line = (self.scroll_y / self.line_height).floor() as usize;
        let visible_lines = (content_rect.height() / self.line_height).ceil() as usize + 1;
        let last_line = (first_line + visible_lines).min(self.rope.len_lines());
        (first_line, last_line)
    }

    // =========================================================================
    // Layout
    // =========================================================================

    /// Invalidate the layout cache.
    fn invalidate_layout(&mut self) {
        self.layout_cache.write().invalidate();
    }

    // =========================================================================
    // Signal Emission
    // =========================================================================

    /// Emit cursor position signal.
    fn emit_cursor_position(&self) {
        let (line, col) = self.char_pos_to_line_col(self.cursor_pos);
        self.cursor_position_changed.emit((line, col));
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let ctrl = event.modifiers.control;
        let shift = event.modifiers.shift;

        match event.key {
            // Navigation
            Key::ArrowLeft => {
                if ctrl {
                    self.move_cursor_word_left(shift);
                } else {
                    self.move_cursor_left(shift);
                }
                true
            }
            Key::ArrowRight => {
                if ctrl {
                    self.move_cursor_word_right(shift);
                } else {
                    self.move_cursor_right(shift);
                }
                true
            }
            Key::ArrowUp => {
                self.move_cursor_up(shift);
                true
            }
            Key::ArrowDown => {
                self.move_cursor_down(shift);
                true
            }
            Key::Home => {
                if ctrl {
                    self.move_cursor_to_start(shift);
                } else {
                    self.move_cursor_to_line_start(shift);
                }
                true
            }
            Key::End => {
                if ctrl {
                    self.move_cursor_to_end(shift);
                } else {
                    self.move_cursor_to_line_end(shift);
                }
                true
            }
            Key::PageUp => {
                self.page_up(shift);
                true
            }
            Key::PageDown => {
                self.page_down(shift);
                true
            }

            // Editing
            Key::Backspace => {
                self.backspace();
                true
            }
            Key::Delete => {
                self.delete_forward();
                true
            }
            Key::Enter => {
                self.undo_stack.break_merge();
                self.insert_text("\n");
                self.undo_stack.enable_merge();
                true
            }
            Key::Tab => {
                let spaces: String = std::iter::repeat(' ').take(self.tab_width).collect();
                self.insert_text(&spaces);
                true
            }

            // Clipboard
            Key::C if ctrl => {
                self.copy();
                true
            }
            Key::X if ctrl => {
                self.cut();
                true
            }
            Key::V if ctrl => {
                self.paste();
                true
            }

            // Selection
            Key::A if ctrl => {
                self.select_all();
                true
            }

            // Undo/Redo
            Key::Z if ctrl && shift => {
                self.redo();
                true
            }
            Key::Z if ctrl => {
                self.undo();
                true
            }
            Key::Y if ctrl => {
                self.redo();
                true
            }

            // Find/Replace shortcuts
            Key::F if ctrl => {
                self.find_requested.emit(());
                true
            }
            Key::H if ctrl => {
                self.find_replace_requested.emit(());
                true
            }
            Key::F3 => {
                if shift {
                    self.find_previous_requested.emit(());
                } else {
                    self.find_next_requested.emit(());
                }
                true
            }
            Key::G if ctrl => {
                if shift {
                    self.find_previous_requested.emit(());
                } else {
                    self.find_next_requested.emit(());
                }
                true
            }

            // Character input - use the text field from the event
            _ => {
                if !event.text.is_empty() && !ctrl && !event.modifiers.alt && !self.read_only {
                    self.insert_text(&event.text);
                    self.undo_stack.enable_merge();
                    true
                } else {
                    false
                }
            }
        }
    }

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = self.pixel_to_char_pos(event.local_pos.x, event.local_pos.y);

        if event.modifiers.shift {
            // Extend selection
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else {
            // Start new selection
            self.selection_anchor = Some(pos);
        }

        self.cursor_pos = pos;
        self.is_dragging = true;

        self.base.update();
        self.emit_cursor_position();
        self.selection_changed.emit(());

        true
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button == MouseButton::Left && self.is_dragging {
            self.is_dragging = false;
            true
        } else {
            false
        }
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        if self.is_dragging {
            let pos = self.pixel_to_char_pos(event.local_pos.x, event.local_pos.y);
            if self.cursor_pos != pos {
                self.cursor_pos = pos;
                self.ensure_cursor_visible();
                self.base.update();
                self.emit_cursor_position();
                self.selection_changed.emit(());
            }
            true
        } else {
            false
        }
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let delta = 40.0; // Pixels per scroll step

        if event.delta_y != 0.0 {
            self.scroll_y -= event.delta_y * delta;
            self.clamp_scroll();
            self.base.update();
            return true;
        }

        if event.delta_x != 0.0 {
            self.scroll_x -= event.delta_x * delta;
            self.clamp_scroll();
            self.base.update();
            return true;
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();

        // Background
        ctx.renderer().fill_rect(rect, self.background_color);

        // Border
        let border_color = if self.base.has_focus() {
            self.focus_border_color
        } else {
            self.border_color
        };
        let stroke = Stroke::new(border_color, 1.0);
        ctx.renderer().stroke_rect(rect, &stroke);
    }

    fn paint_text(&self, ctx: &mut PaintContext<'_>, font_system: &mut FontSystem) {
        let content_rect = self.content_rect();

        // Save state and set up clipping
        ctx.renderer().save();
        ctx.renderer().translate(
            content_rect.origin.x - self.scroll_x,
            content_rect.origin.y - self.scroll_y,
        );

        // Paint search match highlights (before selection so selection renders on top)
        if !self.search_matches.is_empty() {
            let (first_line, last_line) = self.visible_line_range();
            self.paint_search_matches(ctx, first_line, last_line);
        }

        // Paint selection background
        if self.has_selection() {
            let (first_line, last_line) = self.visible_line_range();
            self.paint_selection(ctx, first_line, last_line);
        }

        // Show placeholder if empty
        if self.rope.len_chars() == 0 {
            if !self.placeholder.is_empty() {
                let layout = TextLayout::new(font_system, &self.placeholder, &self.font);

                // Prepare glyphs for rendering
                if let Ok(mut text_renderer) = TextRenderer::new() {
                    let _ = text_renderer.prepare_layout(
                        font_system,
                        &layout,
                        Point::new(0.0, 0.0),
                        self.placeholder_color,
                    );
                }
            }
        } else {
            // Get visible line range for virtualized rendering
            let (first_line, last_line) = self.visible_line_range();

            // Paint each visible line
            for line_idx in first_line..last_line {
                let y = line_idx as f32 * self.line_height;
                let line_text = self.rope.line(line_idx).to_string();
                // Remove trailing newline for display
                let line_text = line_text.trim_end_matches('\n');

                // Get highlight spans for this line
                let spans = if let Some(ref highlighter) = self.highlighter {
                    if let Some(hl) = highlighter.try_read() {
                        hl.highlight_line(line_text, line_idx)
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                if spans.is_empty() {
                    // No highlighting, render plain text
                    let layout = TextLayout::new(font_system, line_text, &self.font);
                    if let Ok(mut text_renderer) = TextRenderer::new() {
                        let _ = text_renderer.prepare_layout(
                            font_system,
                            &layout,
                            Point::new(0.0, y),
                            self.text_color,
                        );
                    }
                } else {
                    // Render with highlighting
                    self.paint_highlighted_line(ctx, font_system, line_text, &spans, y);
                }
            }
        }

        // Paint cursor
        if self.base.has_focus() && self.cursor_visible {
            let (cursor_x, cursor_y) = self.cursor_position_pixels();
            let cursor_rect = Rect::new(cursor_x, cursor_y, 2.0, self.line_height);
            ctx.renderer().fill_rect(cursor_rect, self.text_color);
        }

        ctx.renderer().restore();
    }

    fn paint_selection(&self, ctx: &mut PaintContext<'_>, first_line: usize, last_line: usize) {
        let (start, end) = match self.selection_range() {
            Some(range) => range,
            None => return,
        };
        let (start_line, start_col) = self.char_pos_to_line_col(start);
        let (end_line, end_col) = self.char_pos_to_line_col(end);

        let char_width = self.font.size() * 0.6;

        for line_idx in first_line..last_line {
            if line_idx < start_line || line_idx > end_line {
                continue;
            }

            let y = line_idx as f32 * self.line_height;
            let line_chars = self.rope.line(line_idx).len_chars();

            let sel_start = if line_idx == start_line { start_col } else { 0 };
            let sel_end = if line_idx == end_line {
                end_col
            } else {
                line_chars
            };

            if sel_start < sel_end {
                let x = sel_start as f32 * char_width;
                let width = (sel_end - sel_start) as f32 * char_width;

                ctx.renderer().fill_rect(
                    Rect::new(x, y, width, self.line_height),
                    self.selection_color,
                );
            }
        }
    }

    fn paint_search_matches(&self, ctx: &mut PaintContext<'_>, first_line: usize, last_line: usize) {
        let char_width = self.font.size() * 0.6;

        for (i, search_match) in self.search_matches.iter().enumerate() {
            let is_current = self.current_search_match == Some(i);
            let color = if is_current {
                self.current_search_highlight_color
            } else {
                self.search_highlight_color
            };

            // Convert byte positions to char positions
            let len_bytes = self.rope.len_bytes();
            let start_byte = search_match.start.min(len_bytes);
            let end_byte = search_match.end.min(len_bytes);
            let start_char = self.rope.byte_to_char(start_byte);
            let end_char = self.rope.byte_to_char(end_byte);

            let (start_line, start_col) = self.char_pos_to_line_col(start_char);
            let (end_line, end_col) = self.char_pos_to_line_col(end_char);

            // Only paint lines that are visible
            for line_idx in first_line..last_line {
                if line_idx < start_line || line_idx > end_line {
                    continue;
                }

                let y = line_idx as f32 * self.line_height;
                let line_chars = self.rope.line(line_idx).len_chars();

                let match_start = if line_idx == start_line { start_col } else { 0 };
                let match_end = if line_idx == end_line {
                    end_col
                } else {
                    line_chars
                };

                if match_start < match_end {
                    let x = match_start as f32 * char_width;
                    let width = (match_end - match_start) as f32 * char_width;

                    ctx.renderer().fill_rect(
                        Rect::new(x, y, width, self.line_height),
                        color,
                    );
                }
            }
        }
    }

    fn paint_line_numbers(&self, ctx: &mut PaintContext<'_>, font_system: &mut FontSystem) {
        if !self.line_number_config.visible {
            return;
        }

        let gutter_rect = self.gutter_rect();
        let config = &self.line_number_config;

        // Paint gutter background
        ctx.renderer().fill_rect(gutter_rect, config.background_color);

        // Get the current line for highlighting
        let (current_line, _) = self.char_pos_to_line_col(self.cursor_pos);

        // Get visible line range
        let (first_line, last_line) = self.visible_line_range();

        // Calculate number formatting
        let line_count = self.rope.len_lines();
        let display_digits = Self::digit_count(line_count).max(config.min_digits);

        // Paint each visible line number
        ctx.renderer().save();
        ctx.renderer().translate(
            gutter_rect.origin.x,
            gutter_rect.origin.y - self.scroll_y,
        );

        for line_idx in first_line..last_line {
            let y = line_idx as f32 * self.line_height;
            let is_current_line = line_idx == current_line;

            // Paint current line background highlight
            if is_current_line {
                let highlight_rect = Rect::new(
                    0.0,
                    y,
                    gutter_rect.width(),
                    self.line_height,
                );
                ctx.renderer().fill_rect(highlight_rect, config.current_line_background);
            }

            // Format line number (1-indexed, right-aligned)
            let line_num = line_idx + 1;
            let line_num_str = format!("{:>width$}", line_num, width = display_digits);

            // Choose color based on whether this is the current line
            let text_color = if is_current_line {
                config.current_line_color
            } else {
                config.text_color
            };

            // Render line number text
            let layout = TextLayout::new(font_system, &line_num_str, &self.font);
            if let Ok(mut text_renderer) = TextRenderer::new() {
                let x = config.padding_left;
                let _ = text_renderer.prepare_layout(
                    font_system,
                    &layout,
                    Point::new(x, y),
                    text_color,
                );
            }
        }

        ctx.renderer().restore();

        // Draw a subtle separator line between gutter and text
        let separator_x = gutter_rect.origin.x + gutter_rect.width() - 1.0;
        let separator_rect = Rect::new(
            separator_x,
            gutter_rect.origin.y,
            1.0,
            gutter_rect.height(),
        );
        ctx.renderer().fill_rect(separator_rect, Color::from_rgb8(220, 220, 220));
    }

    fn paint_highlighted_line(
        &self,
        _ctx: &mut PaintContext<'_>,
        font_system: &mut FontSystem,
        line_text: &str,
        spans: &[HighlightSpan],
        y: f32,
    ) {
        let char_width = self.font.size() * 0.6;
        let chars: Vec<char> = line_text.chars().collect();
        let mut current_pos = 0;

        for span in spans {
            // Paint text before this span in default color
            if span.start > current_pos {
                let text: String = chars[current_pos..span.start].iter().collect();
                let layout = TextLayout::new(font_system, &text, &self.font);
                let x = current_pos as f32 * char_width;
                if let Ok(mut text_renderer) = TextRenderer::new() {
                    let _ = text_renderer.prepare_layout(
                        font_system,
                        &layout,
                        Point::new(x, y),
                        self.text_color,
                    );
                }
            }

            // Paint highlighted span
            let span_end = span.end.min(chars.len());
            if span.start < span_end {
                let text: String = chars[span.start..span_end].iter().collect();
                let mut font = self.font.clone();
                if span.bold {
                    font = font.with_weight(horizon_lattice_render::FontWeight::BOLD);
                }
                if span.italic {
                    font = font.with_style(horizon_lattice_render::FontStyle::Italic);
                }
                let layout = TextLayout::new(font_system, &text, &font);
                let x = span.start as f32 * char_width;
                if let Ok(mut text_renderer) = TextRenderer::new() {
                    let _ = text_renderer.prepare_layout(
                        font_system,
                        &layout,
                        Point::new(x, y),
                        span.color,
                    );
                }
            }

            current_pos = span_end;
        }

        // Paint remaining text after last span
        if current_pos < chars.len() {
            let text: String = chars[current_pos..].iter().collect();
            let layout = TextLayout::new(font_system, &text, &self.font);
            let x = current_pos as f32 * char_width;
            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    font_system,
                    &layout,
                    Point::new(x, y),
                    self.text_color,
                );
            }
        }
    }

    fn paint_scrollbars(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let content_size = self.content_size();
        let viewport = self.content_rect();

        let track_color = Color::from_rgb8(240, 240, 240);
        let thumb_color = Color::from_rgb8(180, 180, 180);

        // Vertical scrollbar
        if content_size.height > viewport.height() {
            let track_rect = Rect::new(
                rect.width() - self.scrollbar_thickness,
                0.0,
                self.scrollbar_thickness,
                rect.height() - self.scrollbar_thickness,
            );
            ctx.renderer().fill_rect(track_rect, track_color);

            let visible_ratio = viewport.height() / content_size.height;
            let thumb_height = (track_rect.height() * visible_ratio).max(20.0);
            let (_, max_y) = self.max_scroll();
            let thumb_y = if max_y > 0.0 {
                (self.scroll_y / max_y) * (track_rect.height() - thumb_height)
            } else {
                0.0
            };

            let thumb_rect = Rect::new(
                track_rect.origin.x + 2.0,
                thumb_y + 2.0,
                self.scrollbar_thickness - 4.0,
                thumb_height - 4.0,
            );
            let thumb_rrect = horizon_lattice_render::RoundedRect::new(thumb_rect, 4.0);
            ctx.renderer().fill_rounded_rect(thumb_rrect, thumb_color);
        }

        // Horizontal scrollbar
        if content_size.width > viewport.width() {
            let track_rect = Rect::new(
                0.0,
                rect.height() - self.scrollbar_thickness,
                rect.width() - self.scrollbar_thickness,
                self.scrollbar_thickness,
            );
            ctx.renderer().fill_rect(track_rect, track_color);

            let visible_ratio = viewport.width() / content_size.width;
            let thumb_width = (track_rect.width() * visible_ratio).max(20.0);
            let (max_x, _) = self.max_scroll();
            let thumb_x = if max_x > 0.0 {
                (self.scroll_x / max_x) * (track_rect.width() - thumb_width)
            } else {
                0.0
            };

            let thumb_rect = Rect::new(
                thumb_x + 2.0,
                track_rect.origin.y + 2.0,
                thumb_width - 4.0,
                self.scrollbar_thickness - 4.0,
            );
            let thumb_rrect = horizon_lattice_render::RoundedRect::new(thumb_rect, 4.0);
            ctx.renderer().fill_rounded_rect(thumb_rrect, thumb_color);
        }

        // Corner (if both scrollbars visible)
        if content_size.width > viewport.width() && content_size.height > viewport.height() {
            let corner_rect = Rect::new(
                rect.width() - self.scrollbar_thickness,
                rect.height() - self.scrollbar_thickness,
                self.scrollbar_thickness,
                self.scrollbar_thickness,
            );
            ctx.renderer().fill_rect(corner_rect, Color::from_rgb8(230, 230, 230));
        }
    }
}

impl Default for PlainTextEdit {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// Trait Implementations
// =========================================================================

impl Object for PlainTextEdit {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for PlainTextEdit {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::from_dimensions(400.0, 300.0)
            .with_minimum_dimensions(100.0, 50.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);

        // Get font system for text rendering
        let mut font_system = FontSystem::new();

        // Paint line numbers gutter (before text so it appears behind)
        self.paint_line_numbers(ctx, &mut font_system);

        self.paint_text(ctx, &mut font_system);
        self.paint_scrollbars(ctx);
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
            WidgetEvent::MouseRelease(e) => {
                if self.handle_mouse_release(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseMove(e) => {
                if self.handle_mouse_move(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::Wheel(e) => {
                if self.handle_wheel(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::FocusIn(_) => {
                self.cursor_visible = true;
                self.base.update();
                return true;
            }
            WidgetEvent::FocusOut(_) => {
                self.cursor_visible = false;
                self.is_dragging = false;
                self.base.update();
                return true;
            }
            _ => {}
        }
        false
    }
}

// =========================================================================
// Searchable Implementation
// =========================================================================

impl super::find_replace::Searchable for PlainTextEdit {
    fn search_text(&self) -> String {
        self.rope.to_string()
    }

    fn cursor_position(&self) -> usize {
        // Return byte offset by converting char position to byte position
        if self.cursor_pos == 0 {
            0
        } else {
            self.rope.char_to_byte(self.cursor_pos.min(self.rope.len_chars()))
        }
    }

    fn set_cursor_position(&mut self, byte_pos: usize) {
        // Convert byte offset to char position
        let len_bytes = self.rope.len_bytes();
        let byte_pos = byte_pos.min(len_bytes);
        let char_pos = self.rope.byte_to_char(byte_pos);

        if self.cursor_pos != char_pos {
            self.cursor_pos = char_pos;
            self.selection_anchor = None;
            self.ensure_cursor_visible();
            self.base.update();
            self.emit_cursor_position();
        }
    }

    fn selection_range(&self) -> Option<(usize, usize)> {
        // Return byte offsets
        self.selection_anchor.map(|anchor| {
            let start_char = anchor.min(self.cursor_pos);
            let end_char = anchor.max(self.cursor_pos);
            let start_byte = self.rope.char_to_byte(start_char);
            let end_byte = self.rope.char_to_byte(end_char);
            (start_byte, end_byte)
        })
    }

    fn set_selection(&mut self, start_byte: usize, end_byte: usize) {
        // Convert byte offsets to char positions
        let len_bytes = self.rope.len_bytes();
        let start_byte = start_byte.min(len_bytes);
        let end_byte = end_byte.min(len_bytes);
        let start_char = self.rope.byte_to_char(start_byte);
        let end_char = self.rope.byte_to_char(end_byte);

        self.selection_anchor = Some(start_char);
        self.cursor_pos = end_char;
        self.ensure_cursor_visible();
        self.base.update();
        self.selection_changed.emit(());
        self.emit_cursor_position();
    }

    fn clear_selection(&mut self) {
        PlainTextEdit::clear_selection(self)
    }

    fn replace_range(&mut self, start_byte: usize, end_byte: usize, replacement: &str) {
        if self.read_only {
            return;
        }

        // Convert byte offsets to char positions
        let len_bytes = self.rope.len_bytes();
        let start_byte = start_byte.min(len_bytes);
        let end_byte = end_byte.min(len_bytes);
        let start_char = self.rope.byte_to_char(start_byte);
        let end_char = self.rope.byte_to_char(end_byte);

        // Record for undo
        let deleted = self.rope.slice(start_char..end_char).to_string();

        // Perform the replacement
        self.rope.remove(start_char..end_char);
        if !replacement.is_empty() {
            self.rope.insert(start_char, replacement);
        }

        // Record undo commands
        if !deleted.is_empty() {
            self.undo_stack.push(EditCommand::Delete {
                pos: start_char,
                text: deleted,
            });
        }
        if !replacement.is_empty() {
            self.undo_stack.push(EditCommand::Insert {
                pos: start_char,
                text: replacement.to_string(),
            });
        }

        // Update cursor
        let replacement_chars = replacement.chars().count();
        self.cursor_pos = start_char + replacement_chars;
        self.selection_anchor = None;

        self.invalidate_layout();
        self.ensure_cursor_visible();
        self.base.update();
        self.text_changed.emit(self.rope.to_string());
        self.emit_cursor_position();
    }

    fn scroll_to_position(&mut self, byte_pos: usize) {
        let len_bytes = self.rope.len_bytes();
        let byte_pos = byte_pos.min(len_bytes);
        let char_pos = self.rope.byte_to_char(byte_pos);
        self.cursor_pos = char_pos;
        self.ensure_cursor_visible();
        self.base.update();
    }

    fn set_search_matches(&mut self, matches: Vec<super::find_replace::SearchMatch>) {
        self.search_matches = matches;
        self.base.update();
    }

    fn set_current_match_index(&mut self, index: Option<usize>) {
        self.current_search_match = index;
        self.base.update();
    }

    fn clear_search_highlights(&mut self) {
        self.search_matches.clear();
        self.current_search_match = None;
        self.base.update();
    }
}

// Verify thread safety
static_assertions::assert_impl_all!(PlainTextEdit: Send, Sync);

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_new() {
        setup();
        let edit = PlainTextEdit::new();
        assert!(edit.is_empty());
        assert_eq!(edit.len_chars(), 0);
        assert_eq!(edit.len_lines(), 1); // Empty rope has 1 line
    }

    #[test]
    fn test_with_text() {
        setup();
        let edit = PlainTextEdit::with_text("Hello, World!");
        assert_eq!(edit.text(), "Hello, World!");
        assert_eq!(edit.len_chars(), 13);
    }

    #[test]
    fn test_set_text() {
        setup();
        let mut edit = PlainTextEdit::new();
        edit.set_text("Test content");
        assert_eq!(edit.text(), "Test content");
        assert_eq!(edit.cursor_position(), (0, 12));
    }

    #[test]
    fn test_multiline() {
        setup();
        let edit = PlainTextEdit::with_text("Line 1\nLine 2\nLine 3");
        assert_eq!(edit.len_lines(), 3);
        assert_eq!(edit.line(0), Some("Line 1\n".to_string()));
        assert_eq!(edit.line(1), Some("Line 2\n".to_string()));
        assert_eq!(edit.line(2), Some("Line 3".to_string()));
    }

    #[test]
    fn test_insert_text() {
        setup();
        let mut edit = PlainTextEdit::new();
        edit.insert_text("Hello");
        assert_eq!(edit.text(), "Hello");
        edit.insert_text(" World");
        assert_eq!(edit.text(), "Hello World");
    }

    #[test]
    fn test_backspace() {
        setup();
        let mut edit = PlainTextEdit::with_text("Hello");
        edit.backspace();
        assert_eq!(edit.text(), "Hell");
    }

    #[test]
    fn test_delete_forward() {
        setup();
        let mut edit = PlainTextEdit::with_text("Hello");
        edit.set_cursor_position(0, 0);
        edit.delete_forward();
        assert_eq!(edit.text(), "ello");
    }

    #[test]
    fn test_selection() {
        setup();
        let mut edit = PlainTextEdit::with_text("Hello World");
        edit.set_cursor_position(0, 0);
        edit.selection_anchor = Some(0);
        edit.cursor_pos = 5;
        assert!(edit.has_selection());
        assert_eq!(edit.selected_text(), "Hello");
    }

    #[test]
    fn test_select_all() {
        setup();
        let mut edit = PlainTextEdit::with_text("Hello World");
        edit.select_all();
        assert!(edit.has_selection());
        assert_eq!(edit.selected_text(), "Hello World");
    }

    #[test]
    fn test_delete_selection() {
        setup();
        let mut edit = PlainTextEdit::with_text("Hello World");
        edit.selection_anchor = Some(0);
        edit.cursor_pos = 6;
        edit.delete_selection();
        assert_eq!(edit.text(), "World");
    }

    #[test]
    fn test_undo_redo() {
        setup();
        let mut edit = PlainTextEdit::new();
        edit.insert_text("Hello");
        assert_eq!(edit.text(), "Hello");

        edit.undo();
        assert_eq!(edit.text(), "");

        edit.redo();
        assert_eq!(edit.text(), "Hello");
    }

    #[test]
    fn test_cursor_movement() {
        setup();
        let mut edit = PlainTextEdit::with_text("Hello\nWorld");
        edit.set_cursor_position(0, 0);

        edit.move_cursor_right(false);
        assert_eq!(edit.cursor_position(), (0, 1));

        edit.move_cursor_down(false);
        assert_eq!(edit.cursor_position(), (1, 1));

        edit.move_cursor_left(false);
        assert_eq!(edit.cursor_position(), (1, 0));

        edit.move_cursor_up(false);
        assert_eq!(edit.cursor_position(), (0, 0));
    }

    #[test]
    fn test_line_navigation() {
        setup();
        let mut edit = PlainTextEdit::with_text("Hello World");
        edit.set_cursor_position(0, 5);

        edit.move_cursor_to_line_start(false);
        assert_eq!(edit.cursor_position(), (0, 0));

        edit.move_cursor_to_line_end(false);
        assert_eq!(edit.cursor_position(), (0, 11));
    }

    #[test]
    fn test_read_only() {
        setup();
        let mut edit = PlainTextEdit::with_text("Hello");
        edit.set_read_only(true);

        edit.insert_text(" World");
        assert_eq!(edit.text(), "Hello"); // Should not change

        edit.backspace();
        assert_eq!(edit.text(), "Hello"); // Should not change
    }

    #[test]
    fn test_clear() {
        setup();
        let mut edit = PlainTextEdit::with_text("Hello World");
        edit.clear();
        assert!(edit.is_empty());
        assert_eq!(edit.cursor_position(), (0, 0));
    }

    #[test]
    fn test_large_document() {
        setup();
        // Create a document with 10000 lines
        let lines: String = (0..10000)
            .map(|i| format!("Line {}\n", i))
            .collect();
        let edit = PlainTextEdit::with_text(&lines);

        assert_eq!(edit.len_lines(), 10001); // 10000 lines + trailing empty
        assert!(edit.line(5000).unwrap().starts_with("Line 5000"));
    }

    #[test]
    fn test_signal_emission() {
        setup();
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let text_changed = Arc::new(AtomicBool::new(false));
        let cursor_changed = Arc::new(AtomicBool::new(false));

        let mut edit = PlainTextEdit::new();

        let tc = text_changed.clone();
        edit.text_changed.connect(move |_| {
            tc.store(true, Ordering::SeqCst);
        });

        let cc = cursor_changed.clone();
        edit.cursor_position_changed.connect(move |_| {
            cc.store(true, Ordering::SeqCst);
        });

        edit.insert_text("Hello");

        assert!(text_changed.load(Ordering::SeqCst));
        assert!(cursor_changed.load(Ordering::SeqCst));
    }

    // Test for syntax highlighter
    struct TestHighlighter;

    impl SyntaxHighlighter for TestHighlighter {
        fn highlight_line(&self, line: &str, _line_number: usize) -> Vec<HighlightSpan> {
            let mut spans = Vec::new();
            for (idx, _) in line.match_indices("fn") {
                spans.push(HighlightSpan::new(idx, idx + 2, Color::from_rgb8(86, 156, 214)));
            }
            spans
        }
    }

    #[test]
    fn test_syntax_highlighter() {
        setup();
        let mut edit = PlainTextEdit::with_text("fn main() {}");
        edit.set_highlighter(TestHighlighter);

        // The highlighter should be set
        assert!(edit.highlighter.is_some());
    }

    // =========================================================================
    // Line Number Tests
    // =========================================================================

    #[test]
    fn test_line_numbers_default_disabled() {
        setup();
        let edit = PlainTextEdit::new();
        assert!(!edit.line_numbers_visible());
    }

    #[test]
    fn test_line_numbers_enable_disable() {
        setup();
        let mut edit = PlainTextEdit::new();

        edit.set_line_numbers_visible(true);
        assert!(edit.line_numbers_visible());

        edit.set_line_numbers_visible(false);
        assert!(!edit.line_numbers_visible());
    }

    #[test]
    fn test_line_numbers_builder_pattern() {
        setup();
        let edit = PlainTextEdit::new().with_line_numbers(true);
        assert!(edit.line_numbers_visible());
    }

    #[test]
    fn test_line_number_config() {
        setup();
        let mut edit = PlainTextEdit::new();

        let config = LineNumberConfig::new()
            .with_visible(true)
            .with_text_color(Color::from_rgb8(100, 100, 100))
            .with_current_line_color(Color::from_rgb8(50, 50, 50));

        edit.set_line_number_config(config);

        assert!(edit.line_numbers_visible());
        assert_eq!(edit.line_number_config().text_color, Color::from_rgb8(100, 100, 100));
        assert_eq!(edit.line_number_config().current_line_color, Color::from_rgb8(50, 50, 50));
    }

    #[test]
    fn test_digit_count() {
        assert_eq!(PlainTextEdit::digit_count(0), 1);
        assert_eq!(PlainTextEdit::digit_count(1), 1);
        assert_eq!(PlainTextEdit::digit_count(9), 1);
        assert_eq!(PlainTextEdit::digit_count(10), 2);
        assert_eq!(PlainTextEdit::digit_count(99), 2);
        assert_eq!(PlainTextEdit::digit_count(100), 3);
        assert_eq!(PlainTextEdit::digit_count(999), 3);
        assert_eq!(PlainTextEdit::digit_count(1000), 4);
    }

    #[test]
    fn test_gutter_width_when_disabled() {
        setup();
        let edit = PlainTextEdit::new();
        assert_eq!(edit.gutter_width_const(), 0.0);
    }

    #[test]
    fn test_gutter_width_when_enabled() {
        setup();
        let edit = PlainTextEdit::new().with_line_numbers(true);
        // Gutter width should be > 0 when enabled
        assert!(edit.gutter_width_const() > 0.0);
    }

    #[test]
    fn test_gutter_width_increases_with_lines() {
        setup();
        // Small document (< 1000 lines)
        let small_doc = PlainTextEdit::with_text("line 1\nline 2\nline 3")
            .with_line_numbers(true);
        let small_width = small_doc.gutter_width_const();

        // Large document (> 1000 lines)
        let large_lines: String = (0..1500)
            .map(|i| format!("Line {}\n", i))
            .collect();
        let large_doc = PlainTextEdit::with_text(&large_lines)
            .with_line_numbers(true);
        let large_width = large_doc.gutter_width_const();

        // Large document needs more space for 4-digit line numbers
        assert!(large_width > small_width);
    }
}
