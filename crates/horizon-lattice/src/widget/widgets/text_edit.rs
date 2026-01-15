//! Multi-line text editing widget.
//!
//! The TextEdit widget provides a multi-line text editor with support for:
//! - Multi-line text display and editing
//! - Word wrapping modes (none, word, character)
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
//! use horizon_lattice::widget::widgets::{TextEdit, TextWrapMode};
//!
//! // Create a simple text editor
//! let mut editor = TextEdit::new();
//! editor.set_placeholder("Enter your text...");
//! editor.set_wrap_mode(TextWrapMode::Word);
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
use unicode_segmentation::UnicodeSegmentation;

use super::styled_document::{CharFormat, StyledDocument};
use crate::platform::Clipboard;
use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontStyle, FontSystem, FontWeight, Point, Rect, Renderer, Size,
    Stroke, TextDecoration, TextLayout, TextLayoutOptions, TextRenderer, TextSpan, WrapMode,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, WheelEvent, Widget,
    WidgetBase, WidgetEvent,
};

// =========================================================================
// Undo/Redo System
// =========================================================================

/// Represents an undoable edit operation.
#[derive(Debug, Clone, PartialEq)]
enum EditCommand {
    /// Text was inserted at a position.
    Insert {
        /// Byte position where text was inserted.
        pos: usize,
        /// The inserted text.
        text: String,
    },
    /// Text was deleted from a range.
    Delete {
        /// Byte position where deletion started.
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
                // and doesn't contain newlines (break merge on newlines for cleaner undo)
                if *pos + text.len() == *other_pos && !other_text.contains('\n') {
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
                // Don't merge if either contains newlines
                if text.contains('\n') || other_text.contains('\n') {
                    return false;
                }
                // Backspace: deletion at position before current
                if *other_pos + other_text.len() == *pos {
                    let mut new_text = other_text.clone();
                    new_text.push_str(text);
                    *text = new_text;
                    *pos = *other_pos;
                    true
                }
                // Forward delete: deletion at same position
                else if *other_pos == *pos {
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

/// Manages undo/redo history for text editing.
#[derive(Debug)]
struct UndoStack {
    /// Stack of edit commands.
    commands: Vec<EditCommand>,
    /// Current position in the stack.
    index: usize,
    /// Maximum number of commands to keep.
    max_size: usize,
    /// Whether to attempt merging the next command.
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
        // Remove any commands after current index (clear redo history)
        self.commands.truncate(self.index);

        // Try to merge with the last command if merging is enabled
        if self.merge_enabled {
            if let Some(last) = self.commands.last_mut() {
                if last.try_merge(&command) {
                    return;
                }
            }
        }

        self.commands.push(command);
        self.index = self.commands.len();

        // Enforce max size
        if self.commands.len() > self.max_size {
            let excess = self.commands.len() - self.max_size;
            self.commands.drain(0..excess);
            self.index = self.commands.len();
        }
    }

    fn can_undo(&self) -> bool {
        self.index > 0
    }

    fn can_redo(&self) -> bool {
        self.index < self.commands.len()
    }

    fn undo(&mut self) -> Option<&EditCommand> {
        if self.can_undo() {
            self.index -= 1;
            self.merge_enabled = false;
            Some(&self.commands[self.index])
        } else {
            None
        }
    }

    fn redo(&mut self) -> Option<&EditCommand> {
        if self.can_redo() {
            let cmd = &self.commands[self.index];
            self.index += 1;
            self.merge_enabled = false;
            Some(cmd)
        } else {
            None
        }
    }

    fn clear(&mut self) {
        self.commands.clear();
        self.index = 0;
        self.merge_enabled = true;
    }

    fn break_merge(&mut self) {
        self.merge_enabled = false;
    }

    fn enable_merge(&mut self) {
        self.merge_enabled = true;
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// Text Wrap Mode
// =========================================================================

/// Word wrapping mode for TextEdit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextWrapMode {
    /// No wrapping - horizontal scrolling enabled.
    NoWrap,
    /// Wrap at word boundaries (default).
    #[default]
    Word,
    /// Wrap at character boundaries.
    Character,
    /// Word wrap with character fallback for long words.
    WordOrCharacter,
}

impl TextWrapMode {
    /// Convert to render WrapMode.
    fn to_render_wrap(self) -> WrapMode {
        match self {
            TextWrapMode::NoWrap => WrapMode::None,
            TextWrapMode::Word => WrapMode::Word,
            TextWrapMode::Character => WrapMode::Character,
            TextWrapMode::WordOrCharacter => WrapMode::WordOrCharacter,
        }
    }
}

// =========================================================================
// Cached Layout
// =========================================================================

/// Cached text layout data.
struct CachedLayout {
    /// The computed text layout.
    layout: TextLayout,
    /// The text used for this layout.
    text: String,
    /// The width constraint used.
    width: Option<f32>,
    /// Line start byte positions for efficient line lookup.
    line_starts: Vec<usize>,
}

impl CachedLayout {
    /// Build line starts from text.
    fn compute_line_starts(text: &str) -> Vec<usize> {
        let mut starts = vec![0];
        for (i, c) in text.char_indices() {
            if c == '\n' {
                starts.push(i + 1);
            }
        }
        starts
    }
}

// =========================================================================
// TextEdit Widget
// =========================================================================

/// A multi-line text editing widget.
///
/// TextEdit provides multi-line text editing capabilities including:
/// - Cursor movement and positioning
/// - Text selection (keyboard and mouse)
/// - Word wrapping modes
/// - Scrolling with scrollbars
/// - Undo/redo with history
/// - Copy, cut, paste
/// - Read-only mode
/// - Placeholder text
///
/// # Signals
///
/// - `text_changed`: Emitted when the text content changes
/// - `cursor_position_changed`: Emitted when cursor moves (line, column)
/// - `selection_changed`: Emitted when selection changes
pub struct TextEdit {
    /// Widget base.
    base: WidgetBase,

    /// The styled document containing text and formatting.
    document: StyledDocument,

    /// The text content (plain text, for backward compatibility).
    text: String,

    /// Placeholder text displayed when empty.
    placeholder: String,

    /// Current cursor format (for subsequent typing when no selection).
    cursor_format: CharFormat,

    /// Current cursor position (byte offset).
    cursor_pos: usize,

    /// Selection anchor position. If Some, selection extends from anchor to cursor.
    selection_anchor: Option<usize>,

    /// Word wrap mode.
    wrap_mode: TextWrapMode,

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

    /// Cached text layout.
    cached_layout: RwLock<Option<CachedLayout>>,

    /// Whether we're currently dragging to select.
    is_dragging: bool,

    /// Undo/redo stack.
    undo_stack: UndoStack,

    /// Tab width in spaces.
    tab_width: usize,

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

    /// Signal emitted when text format changes (bold state, italic state, underline state, strikethrough state).
    pub format_changed: Signal<(bool, bool, bool, bool)>,
}

impl TextEdit {
    /// Create a new empty TextEdit.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Expanding));

        Self {
            base,
            document: StyledDocument::new(),
            text: String::new(),
            placeholder: String::new(),
            cursor_format: CharFormat::new(),
            cursor_pos: 0,
            selection_anchor: None,
            wrap_mode: TextWrapMode::Word,
            read_only: false,
            font: Font::new(FontFamily::SansSerif, 14.0),
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
            cached_layout: RwLock::new(None),
            is_dragging: false,
            undo_stack: UndoStack::new(),
            tab_width: 4,
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
            format_changed: Signal::new(),
        }
    }

    /// Create a new TextEdit with initial text.
    pub fn with_text(text: impl Into<String>) -> Self {
        let mut edit = Self::new();
        let text_string = text.into();
        edit.document.set_text(&text_string);
        edit.text = text_string;
        edit.cursor_pos = edit.text.len();
        edit
    }

    // =========================================================================
    // Text Access
    // =========================================================================

    /// Get the current text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the text content.
    ///
    /// This clears any selection, moves the cursor to the end, and clears
    /// the undo history and all formatting.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let new_text = text.into();
        if self.text != new_text {
            self.text = new_text.clone();
            self.document.set_text(&new_text);
            self.cursor_pos = self.text.len();
            self.selection_anchor = None;
            self.cursor_format = CharFormat::new();
            self.undo_stack.clear();
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.text_changed.emit(self.text.clone());
            self.emit_cursor_position();
        }
    }

    /// Get the plain text content (alias for text()).
    pub fn to_plain_text(&self) -> String {
        self.text.clone()
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
    // Word Wrap
    // =========================================================================

    /// Get the word wrap mode.
    pub fn wrap_mode(&self) -> TextWrapMode {
        self.wrap_mode
    }

    /// Set the word wrap mode.
    pub fn set_wrap_mode(&mut self, mode: TextWrapMode) {
        if self.wrap_mode != mode {
            self.wrap_mode = mode;
            self.invalidate_layout();
            self.base.update();
        }
    }

    /// Set wrap mode using builder pattern.
    pub fn with_wrap_mode(mut self, mode: TextWrapMode) -> Self {
        self.wrap_mode = mode;
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
        if self.read_only != read_only {
            self.read_only = read_only;
            self.base.update();
        }
    }

    /// Set read-only using builder pattern.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
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
        self.invalidate_layout();
        self.base.update();
    }

    /// Set font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
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
        self.tab_width = width.max(1);
    }

    /// Set tab width using builder pattern.
    pub fn with_tab_width(mut self, width: usize) -> Self {
        self.tab_width = width.max(1);
        self
    }

    // =========================================================================
    // Cursor and Selection
    // =========================================================================

    /// Get the current cursor position as byte offset.
    pub fn cursor_position(&self) -> usize {
        self.cursor_pos
    }

    /// Get the cursor position as (line, column).
    pub fn cursor_line_column(&self) -> (usize, usize) {
        self.byte_pos_to_line_column(self.cursor_pos)
    }

    /// Set the cursor position.
    pub fn set_cursor_position(&mut self, pos: usize) {
        let clamped = pos.min(self.text.len());
        // Ensure we're at a valid char boundary
        let clamped = self.snap_to_char_boundary(clamped);
        if self.cursor_pos != clamped {
            self.cursor_pos = clamped;
            self.ensure_cursor_visible();
            self.base.update();
            self.emit_cursor_position();
        }
    }

    /// Check if there's an active selection.
    pub fn has_selection(&self) -> bool {
        self.selection_anchor.is_some() && self.selection_anchor != Some(self.cursor_pos)
    }

    /// Get the selected text.
    pub fn selected_text(&self) -> String {
        if let Some((start, end)) = self.selection_range() {
            self.text[start..end].to_string()
        } else {
            String::new()
        }
    }

    /// Get the selection range as (start, end) byte positions.
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            let start = anchor.min(self.cursor_pos);
            let end = anchor.max(self.cursor_pos);
            (start, end)
        })
    }

    /// Select all text.
    pub fn select_all(&mut self) {
        if !self.text.is_empty() {
            self.selection_anchor = Some(0);
            self.cursor_pos = self.text.len();
            self.base.update();
            self.selection_changed.emit(());
        }
    }

    /// Clear the selection.
    pub fn clear_selection(&mut self) {
        if self.selection_anchor.is_some() {
            self.selection_anchor = None;
            self.base.update();
            self.selection_changed.emit(());
        }
    }

    /// Set the selection range.
    pub fn set_selection(&mut self, start: usize, end: usize) {
        let start = self.snap_to_char_boundary(start.min(self.text.len()));
        let end = self.snap_to_char_boundary(end.min(self.text.len()));
        self.selection_anchor = Some(start);
        self.cursor_pos = end;
        self.ensure_cursor_visible();
        self.base.update();
        self.selection_changed.emit(());
        self.emit_cursor_position();
    }

    // =========================================================================
    // Text Formatting
    // =========================================================================

    /// Get the current format at cursor position or selection.
    ///
    /// Returns the format at the cursor position, or if there's a selection,
    /// returns the format if it's uniform across the selection, or the cursor format.
    pub fn current_format(&self) -> CharFormat {
        if let Some((start, end)) = self.selection_range() {
            self.document
                .format_for_range(&(start..end))
                .unwrap_or(self.cursor_format)
        } else {
            // Use cursor format if set, otherwise query document
            if self.cursor_format.is_styled() {
                self.cursor_format
            } else {
                self.document.format_at(self.cursor_pos)
            }
        }
    }

    /// Check if the current selection or cursor position has bold formatting.
    pub fn is_bold(&self) -> bool {
        self.current_format().bold
    }

    /// Check if the current selection or cursor position has italic formatting.
    pub fn is_italic(&self) -> bool {
        self.current_format().italic
    }

    /// Check if the current selection or cursor position has underline formatting.
    pub fn is_underline(&self) -> bool {
        self.current_format().underline
    }

    /// Check if the current selection or cursor position has strikethrough formatting.
    pub fn is_strikethrough(&self) -> bool {
        self.current_format().strikethrough
    }

    /// Toggle bold formatting on the selection or set cursor format.
    pub fn toggle_bold(&mut self) {
        if self.read_only {
            return;
        }

        if let Some((start, end)) = self.selection_range() {
            // Toggle bold on selection
            self.document.toggle_format(
                start..end,
                CharFormat::new().with_bold(true),
            );
            // Update text from document
            self.sync_text_from_document();
        } else {
            // Toggle cursor format for subsequent typing
            self.cursor_format.bold = !self.cursor_format.bold;
        }

        self.invalidate_layout();
        self.base.update();
        self.emit_format_changed();
    }

    /// Toggle italic formatting on the selection or set cursor format.
    pub fn toggle_italic(&mut self) {
        if self.read_only {
            return;
        }

        if let Some((start, end)) = self.selection_range() {
            // Toggle italic on selection
            self.document.toggle_format(
                start..end,
                CharFormat::new().with_italic(true),
            );
            // Update text from document
            self.sync_text_from_document();
        } else {
            // Toggle cursor format for subsequent typing
            self.cursor_format.italic = !self.cursor_format.italic;
        }

        self.invalidate_layout();
        self.base.update();
        self.emit_format_changed();
    }

    /// Toggle underline formatting on the selection or set cursor format.
    pub fn toggle_underline(&mut self) {
        if self.read_only {
            return;
        }

        if let Some((start, end)) = self.selection_range() {
            // Toggle underline on selection
            self.document.toggle_format(
                start..end,
                CharFormat::new().with_underline(true),
            );
            // Update text from document
            self.sync_text_from_document();
        } else {
            // Toggle cursor format for subsequent typing
            self.cursor_format.underline = !self.cursor_format.underline;
        }

        self.invalidate_layout();
        self.base.update();
        self.emit_format_changed();
    }

    /// Toggle strikethrough formatting on the selection or set cursor format.
    pub fn toggle_strikethrough(&mut self) {
        if self.read_only {
            return;
        }

        if let Some((start, end)) = self.selection_range() {
            // Toggle strikethrough on selection
            self.document.toggle_format(
                start..end,
                CharFormat::new().with_strikethrough(true),
            );
            // Update text from document
            self.sync_text_from_document();
        } else {
            // Toggle cursor format for subsequent typing
            self.cursor_format.strikethrough = !self.cursor_format.strikethrough;
        }

        self.invalidate_layout();
        self.base.update();
        self.emit_format_changed();
    }

    /// Set bold formatting on the selection or cursor format.
    pub fn set_bold(&mut self, bold: bool) {
        if self.read_only {
            return;
        }

        if let Some((start, end)) = self.selection_range() {
            if bold {
                // Apply format
                self.apply_format_to_range(start..end, CharFormat::new().with_bold(true));
            } else {
                // Remove format
                self.remove_format_from_range(start..end, CharFormat::new().with_bold(true));
            }
            self.sync_text_from_document();
        } else {
            self.cursor_format.bold = bold;
        }

        self.invalidate_layout();
        self.base.update();
        self.emit_format_changed();
    }

    /// Set italic formatting on the selection or cursor format.
    pub fn set_italic(&mut self, italic: bool) {
        if self.read_only {
            return;
        }

        if let Some((start, end)) = self.selection_range() {
            if italic {
                self.apply_format_to_range(start..end, CharFormat::new().with_italic(true));
            } else {
                self.remove_format_from_range(start..end, CharFormat::new().with_italic(true));
            }
            self.sync_text_from_document();
        } else {
            self.cursor_format.italic = italic;
        }

        self.invalidate_layout();
        self.base.update();
        self.emit_format_changed();
    }

    /// Set underline formatting on the selection or cursor format.
    pub fn set_underline(&mut self, underline: bool) {
        if self.read_only {
            return;
        }

        if let Some((start, end)) = self.selection_range() {
            if underline {
                self.apply_format_to_range(start..end, CharFormat::new().with_underline(true));
            } else {
                self.remove_format_from_range(start..end, CharFormat::new().with_underline(true));
            }
            self.sync_text_from_document();
        } else {
            self.cursor_format.underline = underline;
        }

        self.invalidate_layout();
        self.base.update();
        self.emit_format_changed();
    }

    /// Set strikethrough formatting on the selection or cursor format.
    pub fn set_strikethrough(&mut self, strikethrough: bool) {
        if self.read_only {
            return;
        }

        if let Some((start, end)) = self.selection_range() {
            if strikethrough {
                self.apply_format_to_range(start..end, CharFormat::new().with_strikethrough(true));
            } else {
                self.remove_format_from_range(start..end, CharFormat::new().with_strikethrough(true));
            }
            self.sync_text_from_document();
        } else {
            self.cursor_format.strikethrough = strikethrough;
        }

        self.invalidate_layout();
        self.base.update();
        self.emit_format_changed();
    }

    /// Apply a format to a range in the document.
    fn apply_format_to_range(&mut self, range: std::ops::Range<usize>, format: CharFormat) {
        // Get current format for the range and merge with the new format
        let mut pos = range.start;
        while pos < range.end {
            let current = self.document.format_at(pos);
            let mut new_format = current;

            if format.bold {
                new_format.bold = true;
            }
            if format.italic {
                new_format.italic = true;
            }
            if format.underline {
                new_format.underline = true;
            }
            if format.strikethrough {
                new_format.strikethrough = true;
            }

            // Find the end of this segment
            let segment_end = self.find_format_boundary(pos, range.end);
            self.document.set_format(pos..segment_end, new_format);
            pos = segment_end;
        }
    }

    /// Remove a format from a range in the document.
    fn remove_format_from_range(&mut self, range: std::ops::Range<usize>, format: CharFormat) {
        // Get current format for the range and remove the specified format
        let mut pos = range.start;
        while pos < range.end {
            let current = self.document.format_at(pos);
            let mut new_format = current;

            if format.bold {
                new_format.bold = false;
            }
            if format.italic {
                new_format.italic = false;
            }
            if format.underline {
                new_format.underline = false;
            }
            if format.strikethrough {
                new_format.strikethrough = false;
            }

            // Find the end of this segment
            let segment_end = self.find_format_boundary(pos, range.end);
            self.document.set_format(pos..segment_end, new_format);
            pos = segment_end;
        }
    }

    /// Find the next boundary where format might change.
    fn find_format_boundary(&self, pos: usize, max: usize) -> usize {
        let mut next = max;
        for run in self.document.format_runs() {
            if run.range.start > pos && run.range.start < next {
                next = run.range.start;
            }
            if run.range.end > pos && run.range.end < next {
                next = run.range.end;
            }
        }
        next
    }

    /// Sync the plain text field from the document.
    fn sync_text_from_document(&mut self) {
        self.text = self.document.text().to_string();
    }

    /// Sync the document from the plain text field.
    #[allow(dead_code)]
    fn sync_document_from_text(&mut self) {
        if self.document.text() != self.text {
            self.document.set_text(&self.text);
        }
    }

    /// Emit format changed signal with current format state.
    fn emit_format_changed(&self) {
        let format = self.current_format();
        self.format_changed.emit((
            format.bold,
            format.italic,
            format.underline,
            format.strikethrough,
        ));
    }

    /// Clear the cursor format (reset to default for new typing).
    #[allow(dead_code)]
    fn clear_cursor_format(&mut self) {
        self.cursor_format = CharFormat::new();
    }

    /// Convert the styled document to TextSpans for rendering.
    ///
    /// Returns owned spans that can be used with TextLayout::rich_text.
    fn styled_spans_for_rendering(&self) -> Vec<(String, CharFormat)> {
        self.document.to_styled_spans()
            .into_iter()
            .map(|(text, format)| (text.to_string(), format))
            .collect()
    }

    /// Check if the document has any formatting applied.
    fn has_formatting(&self) -> bool {
        !self.document.format_runs().is_empty()
    }

    // =========================================================================
    // Editing Operations
    // =========================================================================

    /// Insert text at the current cursor position.
    ///
    /// If cursor_format has styling applied, the inserted text will inherit that format.
    pub fn insert_text(&mut self, text: &str) {
        if self.read_only || text.is_empty() {
            return;
        }

        // Delete selection first if any
        if self.has_selection() {
            self.delete_selection_internal();
        }

        // Insert into the styled document with current cursor format first
        self.document.insert(self.cursor_pos, text, self.cursor_format);

        // Then insert into the plain text buffer
        self.text.insert_str(self.cursor_pos, text);

        self.undo_stack.push(EditCommand::Insert {
            pos: self.cursor_pos,
            text: text.to_string(),
        });
        self.cursor_pos += text.len();
        self.selection_anchor = None;

        // Clear cursor format after insertion (standard behavior: format applies once)
        // Keep the format if it was explicitly set (user toggled it)
        // Actually, we want to keep the cursor format until the user moves the cursor
        // or changes selection, so don't clear it here.

        self.invalidate_layout();
        self.ensure_cursor_visible();
        self.base.update();
        self.text_changed.emit(self.text.clone());
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
        self.text_changed.emit(self.text.clone());
        self.emit_cursor_position();
    }

    /// Internal method to delete selection without emitting signals.
    fn delete_selection_internal(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            let deleted = self.text[start..end].to_string();

            // Delete from the styled document first (before modifying text)
            self.document.delete(start..end);

            self.text.replace_range(start..end, "");
            self.undo_stack.push(EditCommand::Delete {
                pos: start,
                text: deleted,
            });
            self.cursor_pos = start;
            self.selection_anchor = None;
        }
    }

    /// Clear all text.
    pub fn clear(&mut self) {
        if self.read_only || self.text.is_empty() {
            return;
        }

        self.selection_anchor = None;
        let deleted = std::mem::take(&mut self.text);
        self.undo_stack.push(EditCommand::Delete {
            pos: 0,
            text: deleted,
        });
        self.cursor_pos = 0;

        // Also clear the styled document
        self.document.clear();

        self.invalidate_layout();
        self.base.update();
        self.text_changed.emit(self.text.clone());
        self.emit_cursor_position();
    }

    /// Append text to the end.
    pub fn append(&mut self, text: &str) {
        if self.read_only || text.is_empty() {
            return;
        }

        let pos = self.text.len();
        self.text.push_str(text);

        // Also append to the styled document (with default format)
        self.document.insert(pos, text, CharFormat::default());

        self.undo_stack.push(EditCommand::Insert {
            pos,
            text: text.to_string(),
        });

        self.invalidate_layout();
        self.base.update();
        self.text_changed.emit(self.text.clone());
    }

    // =========================================================================
    // Clipboard Operations
    // =========================================================================

    /// Copy selected text to clipboard.
    pub fn copy(&self) {
        if !self.has_selection() {
            return;
        }

        let selected = self.selected_text();
        if let Ok(mut clipboard) = Clipboard::new() {
            let _ = clipboard.set_text(&selected);
        }
    }

    /// Cut selected text to clipboard.
    pub fn cut(&mut self) {
        if self.read_only || !self.has_selection() {
            return;
        }

        self.copy();
        self.delete_selection();
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

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }

    /// Undo the last edit operation.
    pub fn undo(&mut self) {
        if self.read_only {
            return;
        }

        if let Some(cmd) = self.undo_stack.undo() {
            match cmd.clone() {
                EditCommand::Insert { pos, text } => {
                    // Undo insert = delete the inserted text
                    self.text.replace_range(pos..pos + text.len(), "");
                    self.cursor_pos = pos;
                }
                EditCommand::Delete { pos, text } => {
                    // Undo delete = insert the deleted text back
                    self.text.insert_str(pos, &text);
                    self.cursor_pos = pos + text.len();
                }
            }
            self.selection_anchor = None;
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.text_changed.emit(self.text.clone());
            self.emit_cursor_position();
        }
    }

    /// Redo the last undone operation.
    pub fn redo(&mut self) {
        if self.read_only {
            return;
        }

        if let Some(cmd) = self.undo_stack.redo() {
            match cmd.clone() {
                EditCommand::Insert { pos, text } => {
                    // Redo insert = insert the text again
                    self.text.insert_str(pos, &text);
                    self.cursor_pos = pos + text.len();
                }
                EditCommand::Delete { pos, text } => {
                    // Redo delete = delete the text again
                    self.text.replace_range(pos..pos + text.len(), "");
                    self.cursor_pos = pos;
                }
            }
            self.selection_anchor = None;
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.text_changed.emit(self.text.clone());
            self.emit_cursor_position();
        }
    }

    // =========================================================================
    // Scrolling
    // =========================================================================

    /// Get the scroll position.
    pub fn scroll_position(&self) -> (f32, f32) {
        (self.scroll_x, self.scroll_y)
    }

    /// Set the scroll position.
    pub fn set_scroll_position(&mut self, x: f32, y: f32) {
        let (max_x, max_y) = self.max_scroll();
        let new_x = x.clamp(0.0, max_x);
        let new_y = y.clamp(0.0, max_y);

        if (self.scroll_x - new_x).abs() > 0.1 || (self.scroll_y - new_y).abs() > 0.1 {
            self.scroll_x = new_x;
            self.scroll_y = new_y;
            self.base.update();
        }
    }

    /// Scroll to make a position visible.
    pub fn ensure_visible(&mut self, x: f32, y: f32) {
        let viewport = self.content_rect();
        let margin = 5.0;

        // Horizontal
        if x < self.scroll_x + margin {
            self.scroll_x = (x - margin).max(0.0);
        } else if x > self.scroll_x + viewport.width() - margin {
            self.scroll_x = x - viewport.width() + margin;
        }

        // Vertical
        if y < self.scroll_y + margin {
            self.scroll_y = (y - margin).max(0.0);
        } else if y > self.scroll_y + viewport.height() - margin {
            self.scroll_y = y - viewport.height() + margin;
        }

        // Clamp to valid range
        let (max_x, max_y) = self.max_scroll();
        self.scroll_x = self.scroll_x.clamp(0.0, max_x);
        self.scroll_y = self.scroll_y.clamp(0.0, max_y);
    }

    /// Ensure the cursor is visible in the viewport.
    fn ensure_cursor_visible(&mut self) {
        let (x, y) = self.cursor_position_pixels();
        let line_height = self.font.size() * 1.2;
        self.ensure_visible(x, y);
        self.ensure_visible(x, y + line_height);
    }

    /// Get maximum scroll values.
    fn max_scroll(&self) -> (f32, f32) {
        let viewport = self.content_rect();
        let content_size = self.content_size();

        let max_x = (content_size.width - viewport.width()).max(0.0);
        let max_y = (content_size.height - viewport.height()).max(0.0);

        (max_x, max_y)
    }

    /// Get the content size.
    fn content_size(&self) -> Size {
        let layout = self.cached_layout.read();
        if let Some(ref cached) = *layout {
            Size::new(cached.layout.width(), cached.layout.height())
        } else {
            // Estimate based on text
            let line_height = self.font.size() * 1.2;
            let line_count = self.text.lines().count().max(1);
            Size::new(200.0, line_count as f32 * line_height)
        }
    }

    // =========================================================================
    // Internal Helpers
    // =========================================================================

    /// Get the content rectangle (excluding border).
    fn content_rect(&self) -> Rect {
        let rect = self.base.rect();
        let padding = 4.0;
        Rect::new(
            padding,
            padding,
            (rect.width() - padding * 2.0 - self.scrollbar_thickness).max(0.0),
            (rect.height() - padding * 2.0 - self.scrollbar_thickness).max(0.0),
        )
    }

    /// Invalidate the cached layout.
    fn invalidate_layout(&mut self) {
        *self.cached_layout.write() = None;
    }

    /// Ensure the layout is up to date.
    fn ensure_layout(&self, font_system: &mut FontSystem) {
        let mut cached = self.cached_layout.write();
        let content_rect = self.content_rect();
        let max_width = if self.wrap_mode == TextWrapMode::NoWrap {
            None
        } else {
            Some(content_rect.width())
        };

        // Check if cached layout is still valid
        if let Some(ref c) = *cached {
            if c.text == self.text && c.width == max_width {
                return;
            }
        }

        // Create new layout
        let options = TextLayoutOptions::default()
            .wrap(self.wrap_mode.to_render_wrap())
            .line_height(1.2);

        let options = if let Some(w) = max_width {
            options.max_width(w)
        } else {
            options
        };

        let text = if self.text.is_empty() {
            " " // Use a space for empty text to get line height
        } else {
            &self.text
        };

        let layout = TextLayout::with_options(font_system, text, &self.font, options);
        let line_starts = CachedLayout::compute_line_starts(&self.text);

        *cached = Some(CachedLayout {
            layout,
            text: self.text.clone(),
            width: max_width,
            line_starts,
        });
    }

    /// Snap a byte position to a valid char boundary.
    fn snap_to_char_boundary(&self, pos: usize) -> usize {
        if pos >= self.text.len() {
            return self.text.len();
        }
        // Find the previous char boundary
        let mut p = pos;
        while p > 0 && !self.text.is_char_boundary(p) {
            p -= 1;
        }
        p
    }

    /// Convert byte position to (line, column).
    fn byte_pos_to_line_column(&self, pos: usize) -> (usize, usize) {
        let mut line = 0;
        let mut col = 0;
        let mut current_pos = 0;

        for (i, c) in self.text.char_indices() {
            if i >= pos {
                break;
            }
            if c == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            current_pos = i + c.len_utf8();
        }

        // Handle position at end of text
        if pos > current_pos {
            col += self.text[current_pos..pos].chars().count();
        }

        (line, col)
    }

    /// Convert (line, column) to byte position.
    fn line_column_to_byte_pos(&self, line: usize, col: usize) -> usize {
        let mut current_line = 0;
        let mut current_col = 0;

        for (i, c) in self.text.char_indices() {
            if current_line == line && current_col == col {
                return i;
            }
            if c == '\n' {
                if current_line == line {
                    return i; // End of target line
                }
                current_line += 1;
                current_col = 0;
            } else {
                current_col += 1;
            }
        }

        self.text.len()
    }

    /// Get the byte position at the start of a line.
    fn line_start(&self, line: usize) -> usize {
        let cached = self.cached_layout.read();
        if let Some(ref c) = *cached {
            if line < c.line_starts.len() {
                return c.line_starts[line];
            }
        }
        // Fall back to computing it
        let mut current_line = 0;
        for (i, c) in self.text.char_indices() {
            if current_line == line {
                return i;
            }
            if c == '\n' {
                current_line += 1;
            }
        }
        self.text.len()
    }

    /// Get the byte position at the end of a line.
    fn line_end(&self, line: usize) -> usize {
        let start = self.line_start(line);
        let rest = &self.text[start..];
        if let Some(pos) = rest.find('\n') {
            start + pos
        } else {
            self.text.len()
        }
    }

    /// Get the number of lines.
    fn line_count(&self) -> usize {
        self.text.lines().count().max(1)
    }

    /// Get cursor position in pixels relative to content area.
    fn cursor_position_pixels(&self) -> (f32, f32) {
        let (line, _col) = self.cursor_line_column();
        let line_height = self.font.size() * 1.2;
        let y = line as f32 * line_height;

        // Estimate x based on character width (simplified)
        let line_start = self.line_start(line);
        let text_before_cursor = &self.text[line_start..self.cursor_pos];
        let x = text_before_cursor.chars().count() as f32 * self.font.size() * 0.6;

        (x, y)
    }

    /// Convert pixel position to byte position.
    fn pixel_to_byte_pos(&self, x: f32, y: f32) -> usize {
        let line_height = self.font.size() * 1.2;
        let line = (y / line_height).floor() as usize;
        let line = line.min(self.line_count().saturating_sub(1));

        let line_start = self.line_start(line);
        let line_end = self.line_end(line);
        let line_text = &self.text[line_start..line_end];

        // Estimate character position (simplified)
        let char_width = self.font.size() * 0.6;
        let col = (x / char_width).round() as usize;
        let col = col.min(line_text.chars().count());

        // Convert column to byte position
        let mut byte_pos = line_start;
        for (i, c) in line_text.char_indices() {
            if i >= col {
                break;
            }
            byte_pos = line_start + i + c.len_utf8();
        }

        // Handle clicking past end of line
        if col >= line_text.chars().count() {
            byte_pos = line_end;
        }

        byte_pos.min(self.text.len())
    }

    /// Emit cursor position changed signal.
    fn emit_cursor_position(&self) {
        let (line, col) = self.cursor_line_column();
        self.cursor_position_changed.emit((line, col));
    }

    // =========================================================================
    // Cursor Movement
    // =========================================================================

    /// Move cursor left by one character.
    fn move_cursor_left(&mut self, extend_selection: bool) {
        if self.cursor_pos == 0 {
            return;
        }

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection && self.has_selection() {
            // Move to start of selection
            if let Some((start, _)) = self.selection_range() {
                self.cursor_pos = start;
            }
            self.selection_anchor = None;
            self.emit_cursor_position();
            self.ensure_cursor_visible();
            self.base.update();
            return;
        }

        // Move to previous char boundary
        let mut new_pos = self.cursor_pos - 1;
        while new_pos > 0 && !self.text.is_char_boundary(new_pos) {
            new_pos -= 1;
        }
        self.cursor_pos = new_pos;

        if !extend_selection {
            self.selection_anchor = None;
        }

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor right by one character.
    fn move_cursor_right(&mut self, extend_selection: bool) {
        if self.cursor_pos >= self.text.len() {
            return;
        }

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection && self.has_selection() {
            // Move to end of selection
            if let Some((_, end)) = self.selection_range() {
                self.cursor_pos = end;
            }
            self.selection_anchor = None;
            self.emit_cursor_position();
            self.ensure_cursor_visible();
            self.base.update();
            return;
        }

        // Move to next char boundary
        let c = self.text[self.cursor_pos..].chars().next().unwrap();
        self.cursor_pos += c.len_utf8();

        if !extend_selection {
            self.selection_anchor = None;
        }

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor up one line.
    fn move_cursor_up(&mut self, extend_selection: bool) {
        let (line, col) = self.cursor_line_column();
        if line == 0 {
            // Already at first line, move to start
            self.move_cursor_to_start(extend_selection);
            return;
        }

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        // Move to same column on previous line
        let prev_line = line - 1;
        let prev_line_start = self.line_start(prev_line);
        let prev_line_end = self.line_end(prev_line);
        let prev_line_len = self.text[prev_line_start..prev_line_end].chars().count();

        let target_col = col.min(prev_line_len);
        self.cursor_pos = self.line_column_to_byte_pos(prev_line, target_col);

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor down one line.
    fn move_cursor_down(&mut self, extend_selection: bool) {
        let (line, col) = self.cursor_line_column();
        let total_lines = self.line_count();

        if line >= total_lines - 1 {
            // Already at last line, move to end
            self.move_cursor_to_end(extend_selection);
            return;
        }

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        // Move to same column on next line
        let next_line = line + 1;
        let next_line_start = self.line_start(next_line);
        let next_line_end = self.line_end(next_line);
        let next_line_len = self.text[next_line_start..next_line_end].chars().count();

        let target_col = col.min(next_line_len);
        self.cursor_pos = self.line_column_to_byte_pos(next_line, target_col);

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor to start of current line.
    fn move_cursor_to_line_start(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let (line, _) = self.cursor_line_column();
        self.cursor_pos = self.line_start(line);

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor to end of current line.
    fn move_cursor_to_line_end(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let (line, _) = self.cursor_line_column();
        self.cursor_pos = self.line_end(line);

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor to start of document.
    fn move_cursor_to_start(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        self.cursor_pos = 0;

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor to end of document.
    fn move_cursor_to_end(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        self.cursor_pos = self.text.len();

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor to next word boundary.
    fn move_cursor_word_right(&mut self, extend_selection: bool) {
        if self.cursor_pos >= self.text.len() {
            return;
        }

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        // Find next word boundary
        let rest = &self.text[self.cursor_pos..];
        let mut found_word = false;
        let mut offset = 0;

        for word in rest.split_word_bounds() {
            offset += word.len();
            let is_word = word.chars().any(|c| c.is_alphanumeric());
            if is_word {
                found_word = true;
            } else if found_word {
                offset -= word.len();
                break;
            }
        }

        self.cursor_pos = (self.cursor_pos + offset).min(self.text.len());

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor to previous word boundary.
    fn move_cursor_word_left(&mut self, extend_selection: bool) {
        if self.cursor_pos == 0 {
            return;
        }

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        // Find previous word boundary
        let before = &self.text[..self.cursor_pos];
        let words: Vec<&str> = before.split_word_bounds().collect();

        let mut new_pos = 0;
        let mut found_word = false;

        for word in words.iter().rev() {
            let is_word = word.chars().any(|c| c.is_alphanumeric());
            if is_word && !found_word {
                found_word = true;
                new_pos = self.cursor_pos - word.len();
            } else if found_word && !is_word {
                break;
            } else if found_word {
                new_pos -= word.len();
            }
        }

        self.cursor_pos = new_pos;

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor up by a page.
    fn page_up(&mut self, extend_selection: bool) {
        let viewport = self.content_rect();
        let line_height = self.font.size() * 1.2;
        let page_lines = (viewport.height() / line_height).floor() as usize;

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let (line, col) = self.cursor_line_column();
        let new_line = line.saturating_sub(page_lines);
        let new_line_end = self.line_end(new_line);
        let new_line_start = self.line_start(new_line);
        let new_line_len = self.text[new_line_start..new_line_end].chars().count();
        let target_col = col.min(new_line_len);

        self.cursor_pos = self.line_column_to_byte_pos(new_line, target_col);

        // Scroll the view
        self.scroll_y = (self.scroll_y - viewport.height()).max(0.0);

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    /// Move cursor down by a page.
    fn page_down(&mut self, extend_selection: bool) {
        let viewport = self.content_rect();
        let line_height = self.font.size() * 1.2;
        let page_lines = (viewport.height() / line_height).floor() as usize;

        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let (line, col) = self.cursor_line_column();
        let new_line = (line + page_lines).min(self.line_count().saturating_sub(1));
        let new_line_end = self.line_end(new_line);
        let new_line_start = self.line_start(new_line);
        let new_line_len = self.text[new_line_start..new_line_end].chars().count();
        let target_col = col.min(new_line_len);

        self.cursor_pos = self.line_column_to_byte_pos(new_line, target_col);

        // Scroll the view
        let (_, max_y) = self.max_scroll();
        self.scroll_y = (self.scroll_y + viewport.height()).min(max_y);

        self.emit_cursor_position();
        self.ensure_cursor_visible();
        self.base.update();
        if extend_selection {
            self.selection_changed.emit(());
        }
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let shift = event.modifiers.shift;
        let ctrl = event.modifiers.control;

        match event.key {
            // Cursor movement
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

            // Editing operations
            Key::Backspace => {
                if self.read_only {
                    return true;
                }
                if self.has_selection() {
                    self.delete_selection();
                } else if self.cursor_pos > 0 {
                    let delete_start = if ctrl {
                        // Delete word
                        let before = &self.text[..self.cursor_pos];
                        let words: Vec<&str> = before.split_word_bounds().collect();
                        let mut pos = self.cursor_pos;
                        let mut found_word = false;
                        for word in words.iter().rev() {
                            let is_word = word.chars().any(|c| c.is_alphanumeric());
                            if is_word && !found_word {
                                found_word = true;
                            } else if found_word && !is_word {
                                break;
                            }
                            pos -= word.len();
                        }
                        pos
                    } else {
                        // Delete single char
                        let mut pos = self.cursor_pos - 1;
                        while pos > 0 && !self.text.is_char_boundary(pos) {
                            pos -= 1;
                        }
                        pos
                    };

                    let deleted = self.text[delete_start..self.cursor_pos].to_string();

                    // Delete from the styled document first (before modifying text)
                    self.document.delete(delete_start..self.cursor_pos);

                    self.text.replace_range(delete_start..self.cursor_pos, "");
                    self.undo_stack.push(EditCommand::Delete {
                        pos: delete_start,
                        text: deleted,
                    });
                    self.cursor_pos = delete_start;

                    self.invalidate_layout();
                    self.ensure_cursor_visible();
                    self.base.update();
                    self.text_changed.emit(self.text.clone());
                    self.emit_cursor_position();
                }
                true
            }
            Key::Delete => {
                if self.read_only {
                    return true;
                }
                if self.has_selection() {
                    self.delete_selection();
                } else if self.cursor_pos < self.text.len() {
                    let delete_end = if ctrl {
                        // Delete word forward
                        let rest = &self.text[self.cursor_pos..];
                        let mut offset = 0;
                        let mut found_word = false;
                        for word in rest.split_word_bounds() {
                            let is_word = word.chars().any(|c| c.is_alphanumeric());
                            if is_word {
                                found_word = true;
                            } else if found_word {
                                break;
                            }
                            offset += word.len();
                        }
                        self.cursor_pos + offset
                    } else {
                        // Delete single char
                        let c = self.text[self.cursor_pos..].chars().next().unwrap();
                        self.cursor_pos + c.len_utf8()
                    };

                    let deleted = self.text[self.cursor_pos..delete_end].to_string();

                    // Delete from the styled document first (before modifying text)
                    self.document.delete(self.cursor_pos..delete_end);

                    self.text.replace_range(self.cursor_pos..delete_end, "");
                    self.undo_stack.push(EditCommand::Delete {
                        pos: self.cursor_pos,
                        text: deleted,
                    });

                    self.invalidate_layout();
                    self.base.update();
                    self.text_changed.emit(self.text.clone());
                }
                true
            }
            Key::Enter => {
                if self.read_only {
                    return true;
                }
                self.insert_text("\n");
                self.undo_stack.break_merge(); // Break merge on newline
                true
            }
            Key::Tab => {
                if self.read_only {
                    return true;
                }
                let spaces = " ".repeat(self.tab_width);
                self.insert_text(&spaces);
                true
            }

            // Clipboard operations
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

            // Undo/redo
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

            // Select all
            Key::A if ctrl => {
                self.select_all();
                true
            }

            // Text formatting shortcuts
            Key::B if ctrl => {
                self.toggle_bold();
                true
            }
            Key::I if ctrl => {
                self.toggle_italic();
                true
            }
            Key::U if ctrl => {
                self.toggle_underline();
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

        let content_rect = self.content_rect();
        let local_x = event.local_pos.x - content_rect.origin.x + self.scroll_x;
        let local_y = event.local_pos.y - content_rect.origin.y + self.scroll_y;

        let new_pos = self.pixel_to_byte_pos(local_x, local_y);

        if event.modifiers.shift && self.selection_anchor.is_some() {
            // Extend selection
            self.cursor_pos = new_pos;
        } else {
            // Start new selection
            self.selection_anchor = Some(new_pos);
            self.cursor_pos = new_pos;
        }

        self.is_dragging = true;
        self.emit_cursor_position();
        self.base.update();
        true
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        self.is_dragging = false;

        // If anchor equals cursor, clear selection
        if self.selection_anchor == Some(self.cursor_pos) {
            self.selection_anchor = None;
        } else {
            self.selection_changed.emit(());
        }

        true
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        if !self.is_dragging {
            return false;
        }

        let content_rect = self.content_rect();
        let local_x = event.local_pos.x - content_rect.origin.x + self.scroll_x;
        let local_y = event.local_pos.y - content_rect.origin.y + self.scroll_y;

        let new_pos = self.pixel_to_byte_pos(local_x, local_y);
        if self.cursor_pos != new_pos {
            self.cursor_pos = new_pos;
            self.ensure_cursor_visible();
            self.emit_cursor_position();
            self.base.update();
        }

        true
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        let scroll_amount = 40.0;

        if event.modifiers.shift || event.delta_x.abs() > event.delta_y.abs() {
            // Horizontal scroll
            let delta = if event.modifiers.shift { event.delta_y } else { event.delta_x };
            let new_x = self.scroll_x - delta * scroll_amount / 120.0;
            let (max_x, _) = self.max_scroll();
            self.scroll_x = new_x.clamp(0.0, max_x);
        } else {
            // Vertical scroll
            let new_y = self.scroll_y - event.delta_y * scroll_amount / 120.0;
            let (_, max_y) = self.max_scroll();
            self.scroll_y = new_y.clamp(0.0, max_y);
        }

        self.base.update();
        true
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
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
        let line_height = self.font.size() * 1.2;

        // Save state and set up clipping
        ctx.renderer().save();
        ctx.renderer().translate(
            content_rect.origin.x - self.scroll_x,
            content_rect.origin.y - self.scroll_y,
        );

        // Paint search match highlights (before selection so selection renders on top)
        self.paint_search_matches(ctx, line_height);

        // Paint selection background
        if let Some((start, end)) = self.selection_range() {
            self.paint_selection(ctx, start, end, line_height);
        }

        // Paint text or placeholder
        if self.text.is_empty() {
            if !self.placeholder.is_empty() {
                let options = TextLayoutOptions::default()
                    .wrap(self.wrap_mode.to_render_wrap());
                let options = if self.wrap_mode != TextWrapMode::NoWrap {
                    options.max_width(content_rect.width())
                } else {
                    options
                };
                let layout = TextLayout::with_options(font_system, &self.placeholder, &self.font, options);

                // Prepare glyphs for rendering
                if let Ok(mut text_renderer) = TextRenderer::new() {
                    if let Ok(_prepared_glyphs) = text_renderer.prepare_layout(
                        font_system,
                        &layout,
                        Point::new(0.0, 0.0),
                        self.placeholder_color,
                    ) {
                        // Note: Actual glyph rendering requires integration with the
                        // application's render pass system.
                    }
                }
            }
        } else if self.has_formatting() {
            // Rich text rendering: use styled spans
            let styled_spans = self.styled_spans_for_rendering();
            let text_spans: Vec<TextSpan<'_>> = styled_spans
                .iter()
                .map(|(text, format)| {
                    let mut span = TextSpan::new(text);

                    // Build font with style overrides
                    let mut font = self.font.clone();
                    let mut needs_font = false;

                    if format.bold {
                        font = font.with_weight(FontWeight::BOLD);
                        needs_font = true;
                    }
                    if format.italic {
                        font = font.with_style(FontStyle::Italic);
                        needs_font = true;
                    }
                    if needs_font {
                        span = span.with_font(font);
                    }

                    // Apply decorations
                    if format.underline {
                        span = span.with_decoration(TextDecoration::underline());
                    }
                    if format.strikethrough {
                        span = span.with_decoration(TextDecoration::strikethrough());
                    }

                    span
                })
                .collect();

            let options = TextLayoutOptions::default()
                .wrap(self.wrap_mode.to_render_wrap());
            let options = if self.wrap_mode != TextWrapMode::NoWrap {
                options.max_width(content_rect.width())
            } else {
                options
            };

            let layout = TextLayout::rich_text(font_system, &text_spans, &self.font, options);

            // Prepare glyphs for rendering
            if let Ok(mut text_renderer) = TextRenderer::new() {
                if let Ok(_prepared_glyphs) = text_renderer.prepare_layout(
                    font_system,
                    &layout,
                    Point::new(0.0, 0.0),
                    self.text_color,
                ) {
                    // Note: Actual glyph rendering requires integration with the
                    // application's render pass system.
                }
            }
        } else {
            // Plain text rendering: use cached layout
            self.ensure_layout(font_system);
            let cached = self.cached_layout.read();
            if let Some(ref c) = *cached {
                // Prepare glyphs for rendering
                if let Ok(mut text_renderer) = TextRenderer::new() {
                    if let Ok(_prepared_glyphs) = text_renderer.prepare_layout(
                        font_system,
                        &c.layout,
                        Point::new(0.0, 0.0),
                        self.text_color,
                    ) {
                        // Note: Actual glyph rendering requires integration with the
                        // application's render pass system.
                    }
                }
            }
        }

        // Paint cursor
        if self.base.has_focus() && self.cursor_visible {
            let (cursor_x, cursor_y) = self.cursor_position_pixels();
            let cursor_rect = Rect::new(cursor_x, cursor_y, 2.0, line_height);
            ctx.renderer().fill_rect(cursor_rect, self.text_color);
        }

        ctx.renderer().restore();
    }

    fn paint_selection(&self, ctx: &mut PaintContext<'_>, start: usize, end: usize, line_height: f32) {
        let start_pos = self.byte_pos_to_line_column(start);
        let end_pos = self.byte_pos_to_line_column(end);

        let char_width = self.font.size() * 0.6;

        for line in start_pos.0..=end_pos.0 {
            let line_start_col = if line == start_pos.0 { start_pos.1 } else { 0 };
            let line_end_col = if line == end_pos.0 {
                end_pos.1
            } else {
                let line_start = self.line_start(line);
                let line_end = self.line_end(line);
                self.text[line_start..line_end].chars().count() + 1 // Include newline space
            };

            let x = line_start_col as f32 * char_width;
            let y = line as f32 * line_height;
            let width = (line_end_col - line_start_col) as f32 * char_width;

            let selection_rect = Rect::new(x, y, width.max(char_width), line_height);
            ctx.renderer().fill_rect(selection_rect, self.selection_color);
        }
    }

    fn paint_search_matches(&self, ctx: &mut PaintContext<'_>, line_height: f32) {
        if self.search_matches.is_empty() {
            return;
        }

        let char_width = self.font.size() * 0.6;

        for (i, search_match) in self.search_matches.iter().enumerate() {
            let is_current = self.current_search_match == Some(i);
            let color = if is_current {
                self.current_search_highlight_color
            } else {
                self.search_highlight_color
            };

            let start_pos = self.byte_pos_to_line_column(search_match.start);
            let end_pos = self.byte_pos_to_line_column(search_match.end);

            for line in start_pos.0..=end_pos.0 {
                let line_start_col = if line == start_pos.0 { start_pos.1 } else { 0 };
                let line_end_col = if line == end_pos.0 {
                    end_pos.1
                } else {
                    let line_start = self.line_start(line);
                    let line_end = self.line_end(line);
                    self.text[line_start..line_end].chars().count() + 1
                };

                let x = line_start_col as f32 * char_width;
                let y = line as f32 * line_height;
                let width = (line_end_col - line_start_col) as f32 * char_width;

                let match_rect = Rect::new(x, y, width.max(char_width), line_height);
                ctx.renderer().fill_rect(match_rect, color);
            }
        }
    }

    fn paint_scrollbars(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let content_size = self.content_size();
        let viewport = self.content_rect();

        // Vertical scrollbar
        if content_size.height > viewport.height() {
            let track_rect = Rect::new(
                rect.width() - self.scrollbar_thickness,
                0.0,
                self.scrollbar_thickness,
                rect.height() - self.scrollbar_thickness,
            );
            ctx.renderer().fill_rect(track_rect, Color::from_rgb8(240, 240, 240));

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
            ctx.renderer().fill_rounded_rect(thumb_rrect, Color::from_rgb8(180, 180, 180));
        }

        // Horizontal scrollbar (only if no wrap)
        if self.wrap_mode == TextWrapMode::NoWrap && content_size.width > viewport.width() {
            let track_rect = Rect::new(
                0.0,
                rect.height() - self.scrollbar_thickness,
                rect.width() - self.scrollbar_thickness,
                self.scrollbar_thickness,
            );
            ctx.renderer().fill_rect(track_rect, Color::from_rgb8(240, 240, 240));

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
            ctx.renderer().fill_rounded_rect(thumb_rrect, Color::from_rgb8(180, 180, 180));
        }

        // Corner (if both scrollbars visible)
        if self.wrap_mode == TextWrapMode::NoWrap
            && content_size.width > viewport.width()
            && content_size.height > viewport.height()
        {
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

impl Default for TextEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for TextEdit {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for TextEdit {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::from_dimensions(300.0, 200.0)
            .with_minimum_dimensions(100.0, 50.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);

        // Get font system for text rendering
        let mut font_system = FontSystem::new();

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
                self.clear_selection();
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

impl super::find_replace::Searchable for TextEdit {
    fn search_text(&self) -> String {
        self.text.clone()
    }

    fn cursor_position(&self) -> usize {
        self.cursor_pos
    }

    fn set_cursor_position(&mut self, pos: usize) {
        let pos = self.snap_to_char_boundary(pos.min(self.text.len()));
        if self.cursor_pos != pos {
            self.cursor_pos = pos;
            self.selection_anchor = None;
            self.ensure_cursor_visible();
            self.base.update();
            self.emit_cursor_position();
        }
    }

    fn selection_range(&self) -> Option<(usize, usize)> {
        TextEdit::selection_range(self)
    }

    fn set_selection(&mut self, start: usize, end: usize) {
        TextEdit::set_selection(self, start, end)
    }

    fn clear_selection(&mut self) {
        TextEdit::clear_selection(self)
    }

    fn replace_range(&mut self, start: usize, end: usize, replacement: &str) {
        if self.read_only {
            return;
        }

        let start = self.snap_to_char_boundary(start.min(self.text.len()));
        let end = self.snap_to_char_boundary(end.min(self.text.len()));

        // Record for undo
        let deleted = self.text[start..end].to_string();
        self.text.replace_range(start..end, replacement);

        if !deleted.is_empty() {
            self.undo_stack.push(EditCommand::Delete {
                pos: start,
                text: deleted,
            });
        }
        if !replacement.is_empty() {
            self.undo_stack.push(EditCommand::Insert {
                pos: start,
                text: replacement.to_string(),
            });
        }

        // Update cursor
        self.cursor_pos = start + replacement.len();
        self.selection_anchor = None;

        self.invalidate_layout();
        self.ensure_cursor_visible();
        self.base.update();
        self.text_changed.emit(self.text.clone());
        self.emit_cursor_position();
    }

    fn scroll_to_position(&mut self, pos: usize) {
        let pos = self.snap_to_char_boundary(pos.min(self.text.len()));
        self.cursor_pos = pos;
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

// Ensure TextEdit is Send + Sync
static_assertions::assert_impl_all!(TextEdit: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_text_edit_creation() {
        setup();
        let edit = TextEdit::new();
        assert!(edit.text().is_empty());
        assert_eq!(edit.cursor_position(), 0);
        assert!(!edit.has_selection());
        assert!(!edit.is_read_only());
        assert_eq!(edit.wrap_mode(), TextWrapMode::Word);
    }

    #[test]
    fn test_text_edit_with_text() {
        setup();
        let edit = TextEdit::with_text("Hello\nWorld");
        assert_eq!(edit.text(), "Hello\nWorld");
        assert_eq!(edit.cursor_position(), 11); // End of text
    }

    #[test]
    fn test_text_edit_builder_pattern() {
        setup();
        let edit = TextEdit::new()
            .with_placeholder("Enter text...")
            .with_wrap_mode(TextWrapMode::NoWrap)
            .with_read_only(true)
            .with_tab_width(2);

        assert_eq!(edit.placeholder(), "Enter text...");
        assert_eq!(edit.wrap_mode(), TextWrapMode::NoWrap);
        assert!(edit.is_read_only());
        assert_eq!(edit.tab_width(), 2);
    }

    #[test]
    fn test_set_text() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");
        assert_eq!(edit.text(), "Hello World");
        assert_eq!(edit.cursor_position(), 11);
    }

    #[test]
    fn test_insert_text() {
        setup();
        let mut edit = TextEdit::new();
        edit.insert_text("Hello");
        assert_eq!(edit.text(), "Hello");
        assert_eq!(edit.cursor_position(), 5);

        edit.insert_text(" World");
        assert_eq!(edit.text(), "Hello World");
    }

    #[test]
    fn test_cursor_line_column() {
        setup();
        let mut edit = TextEdit::with_text("Hello\nWorld\nTest");

        // Beginning
        edit.set_cursor_position(0);
        assert_eq!(edit.cursor_line_column(), (0, 0));

        // Middle of first line
        edit.set_cursor_position(3);
        assert_eq!(edit.cursor_line_column(), (0, 3));

        // Start of second line
        edit.set_cursor_position(6);
        assert_eq!(edit.cursor_line_column(), (1, 0));

        // Middle of second line
        edit.set_cursor_position(8);
        assert_eq!(edit.cursor_line_column(), (1, 2));
    }

    #[test]
    fn test_selection() {
        setup();
        let mut edit = TextEdit::with_text("Hello World");

        // No selection initially
        assert!(!edit.has_selection());
        assert!(edit.selected_text().is_empty());

        // Set selection
        edit.set_selection(0, 5);
        assert!(edit.has_selection());
        assert_eq!(edit.selected_text(), "Hello");
        assert_eq!(edit.selection_range(), Some((0, 5)));

        // Clear selection
        edit.clear_selection();
        assert!(!edit.has_selection());
    }

    #[test]
    fn test_select_all() {
        setup();
        let mut edit = TextEdit::with_text("Hello World");
        edit.select_all();
        assert!(edit.has_selection());
        assert_eq!(edit.selected_text(), "Hello World");
    }

    #[test]
    fn test_delete_selection() {
        setup();
        let mut edit = TextEdit::with_text("Hello World");
        edit.set_selection(0, 6);
        edit.delete_selection();
        assert_eq!(edit.text(), "World");
        assert!(!edit.has_selection());
    }

    #[test]
    fn test_undo_redo() {
        setup();
        let mut edit = TextEdit::new();

        // Insert some text
        edit.insert_text("Hello");
        assert_eq!(edit.text(), "Hello");
        assert!(edit.can_undo());
        assert!(!edit.can_redo());

        // Undo
        edit.undo();
        assert_eq!(edit.text(), "");
        assert!(!edit.can_undo());
        assert!(edit.can_redo());

        // Redo
        edit.redo();
        assert_eq!(edit.text(), "Hello");
    }

    #[test]
    fn test_read_only_mode() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_read_only(true);

        // Editing operations should not modify text
        edit.insert_text("Hello");
        assert!(edit.text().is_empty());

        edit.set_text("Initial");
        edit.set_cursor_position(0);
        edit.insert_text("X");
        assert_eq!(edit.text(), "Initial"); // Unchanged
    }

    #[test]
    fn test_clear() {
        setup();
        let mut edit = TextEdit::with_text("Hello World");
        edit.clear();
        assert!(edit.text().is_empty());
        assert_eq!(edit.cursor_position(), 0);
    }

    #[test]
    fn test_append() {
        setup();
        let mut edit = TextEdit::with_text("Hello");
        edit.append(" World");
        assert_eq!(edit.text(), "Hello World");
    }

    #[test]
    fn test_text_changed_signal() {
        setup();
        let mut edit = TextEdit::new();
        let changed = Arc::new(AtomicBool::new(false));
        let changed_clone = changed.clone();

        edit.text_changed.connect(move |_| {
            changed_clone.store(true, Ordering::SeqCst);
        });

        edit.insert_text("Hello");
        assert!(changed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_cursor_position_changed_signal() {
        setup();
        let mut edit = TextEdit::with_text("Hello\nWorld");
        let position = Arc::new(parking_lot::Mutex::new((0usize, 0usize)));
        let position_clone = position.clone();

        edit.cursor_position_changed.connect(move |&(line, col)| {
            *position_clone.lock() = (line, col);
        });

        edit.set_cursor_position(6);
        let pos = *position.lock();
        assert_eq!(pos, (1, 0));
    }

    #[test]
    fn test_line_operations() {
        setup();
        let edit = TextEdit::with_text("Line1\nLine2\nLine3");

        assert_eq!(edit.line_count(), 3);
        assert_eq!(edit.line_start(0), 0);
        assert_eq!(edit.line_start(1), 6);
        assert_eq!(edit.line_start(2), 12);
        assert_eq!(edit.line_end(0), 5);
        assert_eq!(edit.line_end(1), 11);
        assert_eq!(edit.line_end(2), 17);
    }

    #[test]
    fn test_multiline_insert() {
        setup();
        let mut edit = TextEdit::new();
        edit.insert_text("Hello\nWorld");
        assert_eq!(edit.text(), "Hello\nWorld");
        assert_eq!(edit.line_count(), 2);
    }

    #[test]
    fn test_wrap_mode() {
        setup();
        let mut edit = TextEdit::new();

        assert_eq!(edit.wrap_mode(), TextWrapMode::Word);

        edit.set_wrap_mode(TextWrapMode::NoWrap);
        assert_eq!(edit.wrap_mode(), TextWrapMode::NoWrap);

        edit.set_wrap_mode(TextWrapMode::Character);
        assert_eq!(edit.wrap_mode(), TextWrapMode::Character);
    }

    // Find/Replace tests
    use super::super::find_replace::{FindOptions, SearchMatch, Searchable};

    #[test]
    fn test_searchable_find_all() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("hello world hello");

        let options = FindOptions::default();
        let matches = edit.find_all("hello", &options);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], SearchMatch::new(0, 5));
        assert_eq!(matches[1], SearchMatch::new(12, 17));
    }

    #[test]
    fn test_searchable_find_case_sensitive() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World HELLO");

        // Case insensitive (default)
        let matches = edit.find_all("hello", &FindOptions::default());
        assert_eq!(matches.len(), 2);

        // Case sensitive
        let matches = edit.find_all("Hello", &FindOptions::new().with_case_sensitive(true));
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_searchable_find_whole_word() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("hello helloworld hello");

        let matches = edit.find_all("hello", &FindOptions::new().with_whole_word(true));
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], SearchMatch::new(0, 5));
        assert_eq!(matches[1], SearchMatch::new(17, 22));
    }

    #[test]
    fn test_searchable_find_regex() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("cat bat hat sat");

        let matches = edit.find_all("[cbh]at", &FindOptions::new().with_regex(true));
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_searchable_replace_all() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("hello world hello");

        let count = edit.replace_all("hello", "hi", &FindOptions::default());
        assert_eq!(count, 2);
        assert_eq!(edit.text(), "hi world hi");
    }

    #[test]
    fn test_searchable_set_matches() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("hello world hello");

        let matches = vec![
            SearchMatch::new(0, 5),
            SearchMatch::new(12, 17),
        ];
        edit.set_search_matches(matches.clone());

        // Verify internal state is set
        assert_eq!(edit.search_matches.len(), 2);

        edit.set_current_match_index(Some(1));
        assert_eq!(edit.current_search_match, Some(1));

        edit.clear_search_highlights();
        assert!(edit.search_matches.is_empty());
        assert_eq!(edit.current_search_match, None);
    }

    // =========================================================================
    // Text Formatting Tests
    // =========================================================================

    #[test]
    fn test_toggle_bold_no_selection() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        // Initially no formatting
        assert!(!edit.is_bold());

        // Toggle bold on (cursor format)
        edit.toggle_bold();
        assert!(edit.cursor_format.bold);

        // Toggle bold off
        edit.toggle_bold();
        assert!(!edit.cursor_format.bold);
    }

    #[test]
    fn test_toggle_bold_with_selection() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        // Select "Hello" (bytes 0-5)
        edit.set_selection(0, 5);
        assert!(edit.has_selection());

        // Toggle bold on selection
        edit.toggle_bold();

        // Check that the document has formatting
        assert!(edit.has_formatting());
        assert!(edit.document.format_at(0).bold);
        assert!(edit.document.format_at(4).bold);
        assert!(!edit.document.format_at(5).bold); // Space after Hello

        // Toggle bold off
        edit.set_selection(0, 5);
        edit.toggle_bold();
        assert!(!edit.document.format_at(0).bold);
    }

    #[test]
    fn test_toggle_italic() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        // Select "World" (bytes 6-11)
        edit.set_selection(6, 11);
        edit.toggle_italic();

        assert!(edit.document.format_at(6).italic);
        assert!(edit.document.format_at(10).italic);
        assert!(!edit.document.format_at(0).italic);
    }

    #[test]
    fn test_toggle_underline() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        edit.set_selection(0, 5);
        edit.toggle_underline();

        assert!(edit.document.format_at(0).underline);
        assert!(!edit.document.format_at(6).underline);
    }

    #[test]
    fn test_toggle_strikethrough() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        edit.set_selection(0, 5);
        edit.toggle_strikethrough();

        assert!(edit.document.format_at(0).strikethrough);
        assert!(!edit.document.format_at(6).strikethrough);
    }

    #[test]
    fn test_multiple_formats() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        // Make "Hello" bold and italic
        edit.set_selection(0, 5);
        edit.toggle_bold();
        edit.set_selection(0, 5);
        edit.toggle_italic();

        let format = edit.document.format_at(0);
        assert!(format.bold);
        assert!(format.italic);
        assert!(!format.underline);
    }

    #[test]
    fn test_insert_with_cursor_format() {
        setup();
        let mut edit = TextEdit::new();

        // Set cursor format to bold
        edit.cursor_format.bold = true;

        // Insert text
        edit.insert_text("Bold");

        // The inserted text should have bold formatting
        assert!(edit.document.format_at(0).bold);
        assert!(edit.document.format_at(3).bold);
    }

    #[test]
    fn test_format_state_queries() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        assert!(!edit.is_bold());
        assert!(!edit.is_italic());
        assert!(!edit.is_underline());
        assert!(!edit.is_strikethrough());

        edit.set_selection(0, 5);
        edit.toggle_bold();

        // With selection on bold text
        edit.set_selection(0, 5);
        assert!(edit.is_bold());
    }

    #[test]
    fn test_current_format_returns_cursor_format() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        // Set cursor format
        edit.cursor_format = CharFormat::new().with_bold(true).with_italic(true);

        let format = edit.current_format();
        assert!(format.bold);
        assert!(format.italic);
    }

    #[test]
    fn test_delete_preserves_formatting() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        // Make "World" bold
        edit.set_selection(6, 11);
        edit.toggle_bold();

        // Delete "Hello " (keep formatting on "World")
        edit.set_selection(0, 6);
        edit.delete_selection();

        // "World" should still be bold (now at position 0)
        assert_eq!(edit.text(), "World");
        assert!(edit.document.format_at(0).bold);
    }

    #[test]
    fn test_set_text_clears_formatting() {
        setup();
        let mut edit = TextEdit::new();
        edit.set_text("Hello World");

        // Add formatting
        edit.set_selection(0, 5);
        edit.toggle_bold();
        assert!(edit.has_formatting());

        // Set new text should clear formatting
        edit.set_text("New Text");
        assert!(!edit.has_formatting());
    }
}
