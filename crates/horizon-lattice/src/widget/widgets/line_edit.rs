//! Single-line text input widget.
//!
//! The LineEdit widget provides a single-line text editor with support for:
//! - Text editing with cursor and selection
//! - Placeholder text
//! - Password masking mode
//! - Read-only mode
//! - Maximum length constraint
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::LineEdit;
//!
//! // Create a simple text input
//! let mut edit = LineEdit::new();
//! edit.set_placeholder("Enter your name...");
//!
//! // Create a password field
//! let mut password = LineEdit::new()
//!     .with_echo_mode(EchoMode::Password);
//!
//! // Connect to signals
//! edit.text_changed.connect(|text| {
//!     println!("Text changed: {}", text);
//! });
//!
//! edit.return_pressed.connect(|| {
//!     println!("Enter pressed!");
//! });
//! ```

use parking_lot::RwLock;
use unicode_segmentation::UnicodeSegmentation;

use crate::platform::Clipboard;
use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, Size, Stroke, TextLayout,
    TextLayoutOptions, TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent,
};

/// Echo mode determines how text is displayed in the LineEdit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EchoMode {
    /// Display characters as entered (default).
    #[default]
    Normal,
    /// Display a mask character instead of actual text (for passwords).
    Password,
    /// Don't display anything as the user types.
    NoEcho,
}

/// A single-line text input widget.
///
/// LineEdit provides text editing capabilities including:
/// - Cursor movement and positioning
/// - Text selection (keyboard and mouse)
/// - Character insertion and deletion
/// - Placeholder text when empty
/// - Password masking mode
/// - Read-only mode
/// - Maximum length constraint
///
/// # Signals
///
/// - `text_changed`: Emitted when the text content changes
/// - `editing_finished`: Emitted when editing is finished (focus lost or Enter pressed)
/// - `return_pressed`: Emitted when Enter is pressed
///
/// # Keyboard Shortcuts
///
/// - Arrow keys: Move cursor
/// - Shift+Arrow keys: Extend selection
/// - Home/End: Move to start/end of line
/// - Ctrl+Arrow: Word navigation
/// - Backspace: Delete character before cursor
/// - Delete: Delete character after cursor
/// - Ctrl+Backspace: Delete word before cursor
/// - Ctrl+Delete: Delete word after cursor
/// - Ctrl+A: Select all text
/// - Ctrl+C / Cmd+C: Copy selected text to clipboard
/// - Ctrl+X / Cmd+X: Cut selected text to clipboard
/// - Ctrl+V / Cmd+V: Paste from clipboard
pub struct LineEdit {
    /// Widget base for common functionality.
    base: WidgetBase,

    /// The actual text content.
    text: String,

    /// Placeholder text displayed when empty.
    placeholder: String,

    /// Current cursor position (byte offset in text).
    cursor_pos: usize,

    /// Selection anchor position (byte offset). If Some, selection extends from anchor to cursor.
    selection_anchor: Option<usize>,

    /// Echo mode (normal, password, no echo).
    echo_mode: EchoMode,

    /// Password mask character.
    password_char: char,

    /// Whether the widget is read-only.
    read_only: bool,

    /// Maximum text length (None = unlimited).
    max_length: Option<usize>,

    /// The font for text rendering.
    font: Font,

    /// Text color.
    text_color: Color,

    /// Placeholder text color.
    placeholder_color: Color,

    /// Selection background color.
    selection_color: Color,

    /// Horizontal scroll offset for text that exceeds widget width.
    scroll_offset: f32,

    /// Whether the cursor is currently visible (for blinking).
    cursor_visible: bool,

    /// Cached text layout for efficient rendering.
    cached_layout: RwLock<Option<CachedLayout>>,

    /// Whether we're currently dragging to select.
    is_dragging: bool,

    // Signals

    /// Signal emitted when text changes.
    pub text_changed: Signal<String>,

    /// Signal emitted when editing is finished (focus lost or Enter pressed).
    pub editing_finished: Signal<()>,

    /// Signal emitted when Enter/Return is pressed.
    pub return_pressed: Signal<()>,
}

/// Cached text layout data.
struct CachedLayout {
    /// The computed text layout.
    layout: TextLayout,
    /// The text used for this layout (may be masked for password mode).
    display_text: String,
}

impl LineEdit {
    /// Create a new empty LineEdit.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);

        Self {
            base,
            text: String::new(),
            placeholder: String::new(),
            cursor_pos: 0,
            selection_anchor: None,
            echo_mode: EchoMode::Normal,
            password_char: '•',
            read_only: false,
            max_length: None,
            font: Font::new(FontFamily::SansSerif, 14.0),
            text_color: Color::BLACK,
            placeholder_color: Color::from_rgb8(160, 160, 160),
            selection_color: Color::from_rgba8(51, 153, 255, 128),
            scroll_offset: 0.0,
            cursor_visible: true,
            cached_layout: RwLock::new(None),
            is_dragging: false,
            text_changed: Signal::new(),
            editing_finished: Signal::new(),
            return_pressed: Signal::new(),
        }
    }

    /// Create a new LineEdit with initial text.
    pub fn with_text(text: impl Into<String>) -> Self {
        let mut edit = Self::new();
        edit.text = text.into();
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
    /// This clears any selection and moves the cursor to the end.
    /// If max_length is set, the text will be truncated.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let mut new_text = text.into();

        // Truncate to max_length if set
        if let Some(max) = self.max_length {
            if new_text.chars().count() > max {
                new_text = new_text.chars().take(max).collect();
            }
        }

        if self.text != new_text {
            self.text = new_text.clone();
            self.cursor_pos = self.text.len();
            self.selection_anchor = None;
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.text_changed.emit(new_text);
        }
    }

    /// Clear all text.
    pub fn clear(&mut self) {
        self.set_text("");
    }

    /// Get the text length in characters.
    pub fn text_length(&self) -> usize {
        self.text.chars().count()
    }

    // =========================================================================
    // Placeholder
    // =========================================================================

    /// Get the placeholder text.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Set the placeholder text.
    pub fn set_placeholder(&mut self, text: impl Into<String>) {
        self.placeholder = text.into();
        self.base.update();
    }

    /// Set placeholder using builder pattern.
    pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    // =========================================================================
    // Echo Mode
    // =========================================================================

    /// Get the echo mode.
    pub fn echo_mode(&self) -> EchoMode {
        self.echo_mode
    }

    /// Set the echo mode.
    pub fn set_echo_mode(&mut self, mode: EchoMode) {
        if self.echo_mode != mode {
            self.echo_mode = mode;
            self.invalidate_layout();
            self.base.update();
        }
    }

    /// Set echo mode using builder pattern.
    pub fn with_echo_mode(mut self, mode: EchoMode) -> Self {
        self.echo_mode = mode;
        self
    }

    /// Get the password mask character.
    pub fn password_char(&self) -> char {
        self.password_char
    }

    /// Set the password mask character.
    pub fn set_password_char(&mut self, ch: char) {
        if self.password_char != ch {
            self.password_char = ch;
            if self.echo_mode == EchoMode::Password {
                self.invalidate_layout();
                self.base.update();
            }
        }
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

    /// Set read-only using builder pattern.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    // =========================================================================
    // Max Length
    // =========================================================================

    /// Get the maximum text length.
    pub fn max_length(&self) -> Option<usize> {
        self.max_length
    }

    /// Set the maximum text length (in characters).
    pub fn set_max_length(&mut self, max: Option<usize>) {
        self.max_length = max;
        // Truncate if necessary
        if let Some(max) = max {
            if self.text_length() > max {
                let truncated: String = self.text.chars().take(max).collect();
                self.set_text(truncated);
            }
        }
    }

    /// Set max length using builder pattern.
    pub fn with_max_length(mut self, max: usize) -> Self {
        self.max_length = Some(max);
        self
    }

    // =========================================================================
    // Cursor and Selection
    // =========================================================================

    /// Get the cursor position (byte offset).
    pub fn cursor_position(&self) -> usize {
        self.cursor_pos
    }

    /// Set the cursor position.
    pub fn set_cursor_position(&mut self, pos: usize) {
        let pos = pos.min(self.text.len());
        // Ensure we're at a valid UTF-8 boundary
        let pos = self.snap_to_grapheme_boundary(pos);
        if self.cursor_pos != pos {
            self.cursor_pos = pos;
            self.selection_anchor = None;
            self.ensure_cursor_visible();
            self.base.update();
        }
    }

    /// Check if there is a selection.
    pub fn has_selection(&self) -> bool {
        self.selection_anchor.is_some() && self.selection_anchor != Some(self.cursor_pos)
    }

    /// Get the selected text.
    pub fn selected_text(&self) -> &str {
        if let Some(anchor) = self.selection_anchor {
            let start = anchor.min(self.cursor_pos);
            let end = anchor.max(self.cursor_pos);
            &self.text[start..end]
        } else {
            ""
        }
    }

    /// Get the selection range (start, end) in byte offsets.
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
        }
    }

    /// Clear selection without deleting text.
    pub fn deselect(&mut self) {
        if self.selection_anchor.is_some() {
            self.selection_anchor = None;
            self.base.update();
        }
    }

    // =========================================================================
    // Font and Colors
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
        self.text_color = color;
        self.base.update();
    }

    /// Set text color using builder pattern.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Get the placeholder color.
    pub fn placeholder_color(&self) -> Color {
        self.placeholder_color
    }

    /// Set the placeholder color.
    pub fn set_placeholder_color(&mut self, color: Color) {
        self.placeholder_color = color;
        self.base.update();
    }

    /// Get the selection color.
    pub fn selection_color(&self) -> Color {
        self.selection_color
    }

    /// Set the selection color.
    pub fn set_selection_color(&mut self, color: Color) {
        self.selection_color = color;
        self.base.update();
    }

    // =========================================================================
    // Internal: Text Manipulation
    // =========================================================================

    /// Insert text at the cursor position.
    fn insert_text(&mut self, text: &str) {
        if self.read_only || text.is_empty() {
            return;
        }

        // Delete selection first if any
        if self.has_selection() {
            self.delete_selection();
        }

        // Check max length
        if let Some(max) = self.max_length {
            let current_len = self.text_length();
            let insert_len = text.chars().count();
            if current_len + insert_len > max {
                // Truncate the inserted text
                let allowed = max - current_len;
                if allowed == 0 {
                    return;
                }
                let truncated: String = text.chars().take(allowed).collect();
                self.text.insert_str(self.cursor_pos, &truncated);
                self.cursor_pos += truncated.len();
            } else {
                self.text.insert_str(self.cursor_pos, text);
                self.cursor_pos += text.len();
            }
        } else {
            self.text.insert_str(self.cursor_pos, text);
            self.cursor_pos += text.len();
        }

        self.invalidate_layout();
        self.ensure_cursor_visible();
        self.base.update();
        self.text_changed.emit(self.text.clone());
    }

    /// Delete the selected text.
    fn delete_selection(&mut self) {
        if let Some((start, end)) = self.selection_range() {
            self.text.replace_range(start..end, "");
            self.cursor_pos = start;
            self.selection_anchor = None;
            self.invalidate_layout();
            self.base.update();
            self.text_changed.emit(self.text.clone());
        }
    }

    /// Delete character before cursor (backspace).
    fn delete_char_before(&mut self) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection();
            return;
        }

        if self.cursor_pos > 0 {
            let prev_pos = self.prev_grapheme_boundary(self.cursor_pos);
            self.text.replace_range(prev_pos..self.cursor_pos, "");
            self.cursor_pos = prev_pos;
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.text_changed.emit(self.text.clone());
        }
    }

    /// Delete character after cursor (delete).
    fn delete_char_after(&mut self) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection();
            return;
        }

        if self.cursor_pos < self.text.len() {
            let next_pos = self.next_grapheme_boundary(self.cursor_pos);
            self.text.replace_range(self.cursor_pos..next_pos, "");
            self.invalidate_layout();
            self.base.update();
            self.text_changed.emit(self.text.clone());
        }
    }

    /// Delete word before cursor.
    fn delete_word_before(&mut self) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection();
            return;
        }

        if self.cursor_pos > 0 {
            let word_start = self.word_boundary_before(self.cursor_pos);
            self.text.replace_range(word_start..self.cursor_pos, "");
            self.cursor_pos = word_start;
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();
            self.text_changed.emit(self.text.clone());
        }
    }

    /// Delete word after cursor.
    fn delete_word_after(&mut self) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection();
            return;
        }

        if self.cursor_pos < self.text.len() {
            let word_end = self.word_boundary_after(self.cursor_pos);
            self.text.replace_range(self.cursor_pos..word_end, "");
            self.invalidate_layout();
            self.base.update();
            self.text_changed.emit(self.text.clone());
        }
    }

    // =========================================================================
    // Clipboard Operations
    // =========================================================================

    /// Copy the selected text to the clipboard.
    ///
    /// Returns `true` if text was copied, `false` if there was no selection
    /// or the clipboard operation failed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut edit = LineEdit::with_text("Hello World");
    /// edit.select_all();
    /// edit.copy(); // "Hello World" is now in clipboard
    /// ```
    pub fn copy(&self) -> bool {
        if !self.has_selection() {
            return false;
        }

        let selected = self.selected_text().to_owned();
        if selected.is_empty() {
            return false;
        }

        // Don't copy password text to clipboard
        if self.echo_mode == EchoMode::Password || self.echo_mode == EchoMode::NoEcho {
            return false;
        }

        if let Ok(mut clipboard) = Clipboard::new() {
            clipboard.set_text(&selected).is_ok()
        } else {
            false
        }
    }

    /// Cut the selected text to the clipboard.
    ///
    /// Copies the selected text to the clipboard and then deletes it.
    /// Returns `true` if text was cut, `false` if there was no selection,
    /// the widget is read-only, or the clipboard operation failed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut edit = LineEdit::with_text("Hello World");
    /// edit.select_all();
    /// edit.cut(); // "Hello World" is now in clipboard, text is cleared
    /// assert_eq!(edit.text(), "");
    /// ```
    pub fn cut(&mut self) -> bool {
        if self.read_only {
            return false;
        }

        if !self.has_selection() {
            return false;
        }

        // Don't cut password text to clipboard
        if self.echo_mode == EchoMode::Password || self.echo_mode == EchoMode::NoEcho {
            return false;
        }

        let selected = self.selected_text().to_owned();
        if selected.is_empty() {
            return false;
        }

        if let Ok(mut clipboard) = Clipboard::new() {
            if clipboard.set_text(&selected).is_ok() {
                self.delete_selection();
                return true;
            }
        }
        false
    }

    /// Paste text from the clipboard at the cursor position.
    ///
    /// If there is a selection, it will be replaced with the pasted text.
    /// Returns `true` if text was pasted, `false` if the widget is read-only
    /// or the clipboard operation failed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut edit = LineEdit::new();
    /// // Assuming clipboard contains "Hello"
    /// edit.paste(); // Text is now "Hello"
    /// ```
    pub fn paste(&mut self) -> bool {
        if self.read_only {
            return false;
        }

        if let Ok(mut clipboard) = Clipboard::new() {
            if let Ok(text) = clipboard.get_text() {
                // Filter out newlines and other control characters
                let filtered: String = text
                    .chars()
                    .filter(|c| !c.is_control() || *c == '\t')
                    .collect();

                if !filtered.is_empty() {
                    self.insert_text(&filtered);
                    return true;
                }
            }
        }
        false
    }

    // =========================================================================
    // Internal: Cursor Movement
    // =========================================================================

    /// Move cursor left by one grapheme.
    fn move_cursor_left(&mut self, extend_selection: bool) {
        if extend_selection {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else if self.has_selection() {
            // Move to start of selection
            let (start, _) = self.selection_range().unwrap();
            self.cursor_pos = start;
            self.selection_anchor = None;
            self.ensure_cursor_visible();
            self.base.update();
            return;
        }

        if self.cursor_pos > 0 {
            self.cursor_pos = self.prev_grapheme_boundary(self.cursor_pos);
            self.ensure_cursor_visible();
            self.base.update();
        }
    }

    /// Move cursor right by one grapheme.
    fn move_cursor_right(&mut self, extend_selection: bool) {
        if extend_selection {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else if self.has_selection() {
            // Move to end of selection
            let (_, end) = self.selection_range().unwrap();
            self.cursor_pos = end;
            self.selection_anchor = None;
            self.ensure_cursor_visible();
            self.base.update();
            return;
        }

        if self.cursor_pos < self.text.len() {
            self.cursor_pos = self.next_grapheme_boundary(self.cursor_pos);
            self.ensure_cursor_visible();
            self.base.update();
        }
    }

    /// Move cursor to start of word before.
    fn move_cursor_word_left(&mut self, extend_selection: bool) {
        if extend_selection {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else {
            self.selection_anchor = None;
        }

        self.cursor_pos = self.word_boundary_before(self.cursor_pos);
        self.ensure_cursor_visible();
        self.base.update();
    }

    /// Move cursor to end of word after.
    fn move_cursor_word_right(&mut self, extend_selection: bool) {
        if extend_selection {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else {
            self.selection_anchor = None;
        }

        self.cursor_pos = self.word_boundary_after(self.cursor_pos);
        self.ensure_cursor_visible();
        self.base.update();
    }

    /// Move cursor to start of line.
    fn move_cursor_to_start(&mut self, extend_selection: bool) {
        if extend_selection {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else {
            self.selection_anchor = None;
        }

        self.cursor_pos = 0;
        self.ensure_cursor_visible();
        self.base.update();
    }

    /// Move cursor to end of line.
    fn move_cursor_to_end(&mut self, extend_selection: bool) {
        if extend_selection {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else {
            self.selection_anchor = None;
        }

        self.cursor_pos = self.text.len();
        self.ensure_cursor_visible();
        self.base.update();
    }

    // =========================================================================
    // Internal: Grapheme/Word Boundaries
    // =========================================================================

    /// Find the previous grapheme boundary.
    fn prev_grapheme_boundary(&self, pos: usize) -> usize {
        if pos == 0 {
            return 0;
        }

        let mut offset = 0;
        let mut prev_offset = 0;
        for grapheme in self.text.graphemes(true) {
            if offset >= pos {
                return prev_offset;
            }
            prev_offset = offset;
            offset += grapheme.len();
        }
        prev_offset
    }

    /// Find the next grapheme boundary.
    fn next_grapheme_boundary(&self, pos: usize) -> usize {
        let mut offset = 0;
        for grapheme in self.text.graphemes(true) {
            if offset >= pos {
                return offset + grapheme.len();
            }
            offset += grapheme.len();
        }
        self.text.len()
    }

    /// Snap a position to the nearest grapheme boundary.
    fn snap_to_grapheme_boundary(&self, pos: usize) -> usize {
        let mut offset = 0;
        for grapheme in self.text.graphemes(true) {
            let next_offset = offset + grapheme.len();
            if pos <= offset {
                return offset;
            }
            if pos < next_offset {
                // Return closer boundary
                if pos - offset <= next_offset - pos {
                    return offset;
                } else {
                    return next_offset;
                }
            }
            offset = next_offset;
        }
        self.text.len()
    }

    /// Find word boundary before position.
    fn word_boundary_before(&self, pos: usize) -> usize {
        if pos == 0 || self.text.is_empty() {
            return 0;
        }

        let chars: Vec<char> = self.text.chars().collect();
        let mut char_idx = 0;
        let mut byte_idx = 0;

        // Find char index for byte position
        while byte_idx < pos && char_idx < chars.len() {
            byte_idx += chars[char_idx].len_utf8();
            char_idx += 1;
        }

        if char_idx == 0 {
            return 0;
        }
        char_idx -= 1;

        // Skip whitespace/punctuation
        while char_idx > 0 && !chars[char_idx].is_alphanumeric() {
            char_idx -= 1;
        }

        // Skip word characters
        while char_idx > 0 && chars[char_idx - 1].is_alphanumeric() {
            char_idx -= 1;
        }

        // Convert back to byte offset
        chars[..char_idx].iter().map(|c| c.len_utf8()).sum()
    }

    /// Find word boundary after position.
    fn word_boundary_after(&self, pos: usize) -> usize {
        if pos >= self.text.len() || self.text.is_empty() {
            return self.text.len();
        }

        let chars: Vec<char> = self.text.chars().collect();
        let mut char_idx = 0;
        let mut byte_idx = 0;

        // Find char index for byte position
        while byte_idx < pos && char_idx < chars.len() {
            byte_idx += chars[char_idx].len_utf8();
            char_idx += 1;
        }

        // Skip word characters
        while char_idx < chars.len() && chars[char_idx].is_alphanumeric() {
            char_idx += 1;
        }

        // Skip whitespace/punctuation
        while char_idx < chars.len() && !chars[char_idx].is_alphanumeric() {
            char_idx += 1;
        }

        // Convert back to byte offset
        chars[..char_idx].iter().map(|c| c.len_utf8()).sum()
    }

    // =========================================================================
    // Internal: Layout and Display
    // =========================================================================

    /// Get the display text (masked for password mode).
    fn display_text(&self) -> String {
        match self.echo_mode {
            EchoMode::Normal => self.text.clone(),
            EchoMode::Password => {
                self.password_char
                    .to_string()
                    .repeat(self.text.chars().count())
            }
            EchoMode::NoEcho => String::new(),
        }
    }

    /// Invalidate the cached layout.
    fn invalidate_layout(&self) {
        *self.cached_layout.write() = None;
    }

    /// Get or create the text layout.
    fn ensure_layout(&self, font_system: &mut FontSystem) -> TextLayout {
        let mut cached = self.cached_layout.write();

        let display_text = self.display_text();

        if let Some(ref cache) = *cached {
            if cache.display_text == display_text {
                return cache.layout.clone();
            }
        }

        let options = TextLayoutOptions::new();
        let layout = TextLayout::with_options(font_system, &display_text, &self.font, options);

        *cached = Some(CachedLayout {
            layout: layout.clone(),
            display_text,
        });

        layout
    }

    /// Ensure the cursor is visible by adjusting scroll offset.
    fn ensure_cursor_visible(&mut self) {
        let padding = 2.0;
        let width = self.base.size().width - padding * 2.0;

        if width <= 0.0 {
            return;
        }

        // Get cursor X position
        let mut font_system = FontSystem::new();
        let _layout = self.ensure_layout(&mut font_system);

        let cursor_x = if self.echo_mode == EchoMode::NoEcho {
            0.0
        } else {
            let display_text = self.display_text();
            let display_cursor_pos = self.display_cursor_pos();
            if display_cursor_pos == 0 {
                0.0
            } else {
                let prefix = &display_text[..display_cursor_pos.min(display_text.len())];
                let prefix_layout =
                    TextLayout::with_options(&mut font_system, prefix, &self.font, TextLayoutOptions::new());
                prefix_layout.width()
            }
        };

        // Adjust scroll to keep cursor visible
        if cursor_x - self.scroll_offset < 0.0 {
            self.scroll_offset = cursor_x;
        } else if cursor_x - self.scroll_offset > width {
            self.scroll_offset = cursor_x - width;
        }
    }

    /// Get cursor position in display text (accounting for password mode).
    fn display_cursor_pos(&self) -> usize {
        match self.echo_mode {
            EchoMode::Normal => self.cursor_pos,
            EchoMode::Password => {
                // Count characters up to cursor position
                let char_count = self.text[..self.cursor_pos].chars().count();
                char_count * self.password_char.len_utf8()
            }
            EchoMode::NoEcho => 0,
        }
    }

    /// Convert display text position to real text position.
    fn display_pos_to_real(&self, display_pos: usize) -> usize {
        match self.echo_mode {
            EchoMode::Normal => display_pos.min(self.text.len()),
            EchoMode::Password => {
                // Each password char maps to one real char
                let char_idx = display_pos / self.password_char.len_utf8();
                self.text
                    .char_indices()
                    .nth(char_idx)
                    .map(|(i, _)| i)
                    .unwrap_or(self.text.len())
            }
            EchoMode::NoEcho => 0,
        }
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    /// Handle a key press event.
    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let shift = event.modifiers.shift;
        let ctrl = event.modifiers.control || event.modifiers.meta;

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
            Key::Home => {
                self.move_cursor_to_start(shift);
                true
            }
            Key::End => {
                self.move_cursor_to_end(shift);
                true
            }

            // Deletion
            Key::Backspace => {
                if ctrl {
                    self.delete_word_before();
                } else {
                    self.delete_char_before();
                }
                true
            }
            Key::Delete => {
                if ctrl {
                    self.delete_word_after();
                } else {
                    self.delete_char_after();
                }
                true
            }

            // Enter
            Key::Enter => {
                self.return_pressed.emit(());
                self.editing_finished.emit(());
                true
            }

            // Select all
            Key::A if ctrl => {
                self.select_all();
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

            // Character input
            _ => {
                if !event.text.is_empty() && !ctrl && !event.modifiers.alt {
                    self.insert_text(&event.text);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Handle a mouse press event.
    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Calculate cursor position from click
        let mut font_system = FontSystem::new();
        let layout = self.ensure_layout(&mut font_system);

        let x = event.local_pos.x + self.scroll_offset - 2.0; // Adjust for padding
        let display_pos = layout.offset_at_point(x, 0.0);
        let real_pos = self.display_pos_to_real(display_pos);

        if event.modifiers.shift {
            // Extend selection
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else {
            // Start new selection
            self.selection_anchor = Some(real_pos);
        }

        self.cursor_pos = real_pos;
        self.is_dragging = true;
        self.ensure_cursor_visible();
        self.base.update();

        true
    }

    /// Handle a mouse release event.
    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        self.is_dragging = false;

        // Clear selection if it's empty (single click)
        if let Some(anchor) = self.selection_anchor {
            if anchor == self.cursor_pos {
                self.selection_anchor = None;
            }
        }

        true
    }

    /// Handle a mouse move event.
    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        if !self.is_dragging {
            return false;
        }

        // Calculate cursor position from drag
        let mut font_system = FontSystem::new();
        let layout = self.ensure_layout(&mut font_system);

        let x = event.local_pos.x + self.scroll_offset - 2.0;
        let display_pos = layout.offset_at_point(x.max(0.0), 0.0);
        let real_pos = self.display_pos_to_real(display_pos);

        if self.cursor_pos != real_pos {
            self.cursor_pos = real_pos;
            self.ensure_cursor_visible();
            self.base.update();
        }

        true
    }

    /// Handle focus gained.
    fn handle_focus_in(&mut self) {
        self.cursor_visible = true;
        self.base.update();
    }

    /// Handle focus lost.
    fn handle_focus_out(&mut self) {
        self.cursor_visible = false;
        self.is_dragging = false;
        self.editing_finished.emit(());
        self.base.update();
    }

    // =========================================================================
    // Rendering Helpers
    // =========================================================================

    /// Get the effective text color based on state.
    fn effective_text_color(&self) -> Color {
        if !self.base.is_effectively_enabled() {
            Color::from_rgb8(160, 160, 160)
        } else {
            self.text_color
        }
    }
}

impl Default for LineEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for LineEdit {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for LineEdit {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // LineEdit has a fixed height based on font, expanding width
        let line_height = self.font.size() * 1.2;
        let padding = 8.0;
        let min_width = 80.0;
        let preferred_width = 200.0;

        SizeHint::new(Size::new(preferred_width, line_height + padding))
            .with_minimum_dimensions(min_width, line_height + padding)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        let padding = 2.0;
        let text_rect = Rect::new(
            rect.origin.x + padding,
            rect.origin.y,
            rect.width() - padding * 2.0,
            rect.height(),
        );

        // Draw background
        let bg_color = if self.base.is_effectively_enabled() {
            Color::WHITE
        } else {
            Color::from_rgb8(245, 245, 245)
        };
        ctx.renderer().fill_rect(rect, bg_color);

        // Draw border
        let border_color = if self.base.has_focus() {
            Color::from_rgb8(51, 153, 255)
        } else {
            Color::from_rgb8(200, 200, 200)
        };
        ctx.renderer()
            .stroke_rect(rect, &Stroke::new(border_color, 1.0));

        // Get font system
        let mut font_system = FontSystem::new();

        // Determine what to show: placeholder or content
        let show_placeholder = self.text.is_empty() && !self.placeholder.is_empty();

        if show_placeholder {
            // Draw placeholder text
            let layout = TextLayout::with_options(
                &mut font_system,
                &self.placeholder,
                &self.font,
                TextLayoutOptions::new(),
            );

            let y = rect.origin.y + (rect.height() - layout.height()) / 2.0;

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_rect.origin.x, y),
                    self.placeholder_color,
                );
            }
        } else if self.echo_mode != EchoMode::NoEcho {
            // Draw text content
            let display_text = self.display_text();
            let layout = self.ensure_layout(&mut font_system);

            // Calculate vertical centering
            let y = rect.origin.y + (rect.height() - layout.height()) / 2.0;
            let x = text_rect.origin.x - self.scroll_offset;

            // Draw selection background if we have a selection and are focused
            if self.has_selection() && self.base.has_focus() {
                if let Some((start, end)) = self.selection_range() {
                    // Convert to display positions
                    let display_start = match self.echo_mode {
                        EchoMode::Normal => start,
                        EchoMode::Password => {
                            self.text[..start].chars().count() * self.password_char.len_utf8()
                        }
                        EchoMode::NoEcho => 0,
                    };
                    let display_end = match self.echo_mode {
                        EchoMode::Normal => end,
                        EchoMode::Password => {
                            self.text[..end].chars().count() * self.password_char.len_utf8()
                        }
                        EchoMode::NoEcho => 0,
                    };

                    let selection_rects = layout.selection_rects(display_start, display_end);
                    for sel_rect in selection_rects {
                        ctx.renderer().fill_rect(
                            Rect::new(
                                x + sel_rect.x,
                                y + sel_rect.y,
                                sel_rect.width,
                                sel_rect.height,
                            ),
                            self.selection_color,
                        );
                    }
                }
            }

            // Draw text
            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(x, y),
                    self.effective_text_color(),
                );
            }

            // Draw cursor if focused and visible
            if self.base.has_focus() && self.cursor_visible {
                let cursor_display_pos = self.display_cursor_pos();
                let cursor_x = if cursor_display_pos == 0 {
                    0.0
                } else {
                    let prefix = &display_text[..cursor_display_pos.min(display_text.len())];
                    let prefix_layout = TextLayout::with_options(
                        &mut font_system,
                        prefix,
                        &self.font,
                        TextLayoutOptions::new(),
                    );
                    prefix_layout.width()
                };

                let cursor_rect = Rect::new(
                    x + cursor_x,
                    y,
                    1.5,
                    layout.height(),
                );
                ctx.renderer().fill_rect(cursor_rect, self.text_color);
            }
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::KeyPress(e) => {
                if self.handle_key_press(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::MousePress(e) => {
                if self.handle_mouse_press(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if self.handle_mouse_release(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::MouseMove(e) => {
                if self.handle_mouse_move(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::FocusIn(_) => {
                self.handle_focus_in();
                true
            }
            WidgetEvent::FocusOut(_) => {
                self.handle_focus_out();
                true
            }
            _ => false,
        }
    }
}

// Ensure LineEdit is Send + Sync
static_assertions::assert_impl_all!(LineEdit: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_line_edit_creation() {
        setup();
        let edit = LineEdit::new();
        assert_eq!(edit.text(), "");
        assert_eq!(edit.cursor_position(), 0);
        assert!(!edit.has_selection());
        assert!(!edit.is_read_only());
        assert_eq!(edit.echo_mode(), EchoMode::Normal);
    }

    #[test]
    fn test_line_edit_with_text() {
        setup();
        let edit = LineEdit::with_text("Hello");
        assert_eq!(edit.text(), "Hello");
        assert_eq!(edit.cursor_position(), 5);
    }

    #[test]
    fn test_set_text() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_text("Test");
        assert_eq!(edit.text(), "Test");
        assert_eq!(edit.cursor_position(), 4);
    }

    #[test]
    fn test_placeholder() {
        setup();
        let edit = LineEdit::new().with_placeholder("Enter text...");
        assert_eq!(edit.placeholder(), "Enter text...");
    }

    #[test]
    fn test_echo_mode() {
        setup();
        let edit = LineEdit::new().with_echo_mode(EchoMode::Password);
        assert_eq!(edit.echo_mode(), EchoMode::Password);
    }

    #[test]
    fn test_password_display() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_echo_mode(EchoMode::Password);
        edit.set_text("secret");
        assert_eq!(edit.display_text(), "••••••");
    }

    #[test]
    fn test_read_only() {
        setup();
        let edit = LineEdit::new().with_read_only(true);
        assert!(edit.is_read_only());
    }

    #[test]
    fn test_max_length() {
        setup();
        let mut edit = LineEdit::new().with_max_length(5);
        edit.set_text("Hello World");
        assert_eq!(edit.text(), "Hello");
        assert_eq!(edit.text_length(), 5);
    }

    #[test]
    fn test_insert_respects_max_length() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_max_length(Some(5));
        edit.insert_text("Hello World");
        assert_eq!(edit.text(), "Hello");
    }

    #[test]
    fn test_cursor_movement() {
        setup();
        let mut edit = LineEdit::with_text("Hello");

        edit.move_cursor_left(false);
        assert_eq!(edit.cursor_position(), 4);

        edit.move_cursor_right(false);
        assert_eq!(edit.cursor_position(), 5);

        edit.move_cursor_to_start(false);
        assert_eq!(edit.cursor_position(), 0);

        edit.move_cursor_to_end(false);
        assert_eq!(edit.cursor_position(), 5);
    }

    #[test]
    fn test_selection() {
        setup();
        let mut edit = LineEdit::with_text("Hello World");

        edit.select_all();
        assert!(edit.has_selection());
        assert_eq!(edit.selected_text(), "Hello World");

        edit.deselect();
        assert!(!edit.has_selection());
    }

    #[test]
    fn test_selection_with_shift() {
        setup();
        let mut edit = LineEdit::with_text("Hello");
        edit.set_cursor_position(0);

        // Select "He" by moving right twice with shift
        edit.move_cursor_right(true);
        edit.move_cursor_right(true);

        assert!(edit.has_selection());
        assert_eq!(edit.selected_text(), "He");
    }

    #[test]
    fn test_delete_selection() {
        setup();
        let mut edit = LineEdit::with_text("Hello World");

        edit.select_all();
        edit.delete_selection();

        assert_eq!(edit.text(), "");
        assert!(!edit.has_selection());
    }

    #[test]
    fn test_backspace() {
        setup();
        let mut edit = LineEdit::with_text("Hello");

        edit.delete_char_before();
        assert_eq!(edit.text(), "Hell");
    }

    #[test]
    fn test_delete() {
        setup();
        let mut edit = LineEdit::with_text("Hello");
        edit.set_cursor_position(0);

        edit.delete_char_after();
        assert_eq!(edit.text(), "ello");
    }

    #[test]
    fn test_word_boundaries() {
        setup();
        let edit = LineEdit::with_text("Hello World Test");

        // Word boundary before from position 11 (after "World")
        let boundary = edit.word_boundary_before(11);
        assert_eq!(boundary, 6); // Start of "World"

        // Word boundary after from position 0
        let boundary = edit.word_boundary_after(0);
        assert_eq!(boundary, 6); // After "Hello " at start of "World"
    }

    #[test]
    fn test_grapheme_boundaries() {
        setup();
        let edit = LineEdit::with_text("Héllo"); // é is composed of 2 bytes

        let next = edit.next_grapheme_boundary(0);
        assert_eq!(next, 1); // After 'H'

        let next = edit.next_grapheme_boundary(1);
        assert!(next > 1); // After 'é' (multi-byte)
    }

    #[test]
    fn test_clear() {
        setup();
        let mut edit = LineEdit::with_text("Hello");
        edit.clear();
        assert_eq!(edit.text(), "");
    }

    #[test]
    fn test_text_changed_signal() {
        setup();
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

        let mut edit = LineEdit::new();
        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_clone = signal_received.clone();

        edit.text_changed.connect(move |_| {
            signal_clone.store(true, Ordering::SeqCst);
        });

        edit.set_text("Hello");
        assert!(signal_received.load(Ordering::SeqCst));
    }

    #[test]
    fn test_size_hint() {
        setup();
        let edit = LineEdit::new();
        let hint = edit.size_hint();

        assert!(hint.preferred.width > 0.0);
        assert!(hint.preferred.height > 0.0);
        assert!(hint.minimum.is_some());
    }

    // =========================================================================
    // Clipboard Tests
    // =========================================================================

    #[test]
    fn test_copy_without_selection_returns_false() {
        setup();
        let edit = LineEdit::with_text("Hello");
        // No selection, copy should return false
        assert!(!edit.copy());
    }

    #[test]
    fn test_copy_password_mode_returns_false() {
        setup();
        let mut edit = LineEdit::with_text("secret");
        edit.set_echo_mode(EchoMode::Password);
        edit.select_all();
        // Should not copy password text
        assert!(!edit.copy());
    }

    #[test]
    fn test_copy_no_echo_mode_returns_false() {
        setup();
        let mut edit = LineEdit::with_text("secret");
        edit.set_echo_mode(EchoMode::NoEcho);
        edit.select_all();
        // Should not copy no-echo text
        assert!(!edit.copy());
    }

    #[test]
    fn test_cut_without_selection_returns_false() {
        setup();
        let mut edit = LineEdit::with_text("Hello");
        // No selection, cut should return false
        assert!(!edit.cut());
    }

    #[test]
    fn test_cut_read_only_returns_false() {
        setup();
        let mut edit = LineEdit::with_text("Hello");
        edit.set_read_only(true);
        edit.select_all();
        // Read-only, cut should return false
        assert!(!edit.cut());
    }

    #[test]
    fn test_cut_password_mode_returns_false() {
        setup();
        let mut edit = LineEdit::with_text("secret");
        edit.set_echo_mode(EchoMode::Password);
        edit.select_all();
        // Should not cut password text
        assert!(!edit.cut());
    }

    #[test]
    fn test_paste_read_only_returns_false() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_read_only(true);
        // Read-only, paste should return false
        assert!(!edit.paste());
    }

    #[test]
    #[ignore] // Requires system clipboard - run manually with: cargo test -- --ignored
    fn test_copy_paste_integration() {
        setup();
        let mut edit1 = LineEdit::with_text("Hello World");
        edit1.select_all();

        // Copy from edit1
        if edit1.copy() {
            let mut edit2 = LineEdit::new();
            // Paste into edit2
            if edit2.paste() {
                assert_eq!(edit2.text(), "Hello World");
            }
        }
    }

    #[test]
    #[ignore] // Requires system clipboard - run manually with: cargo test -- --ignored
    fn test_cut_integration() {
        setup();
        let mut edit = LineEdit::with_text("Hello World");
        edit.select_all();

        // Cut should remove text and copy to clipboard
        if edit.cut() {
            assert_eq!(edit.text(), "");

            // Paste should restore the text
            if edit.paste() {
                assert_eq!(edit.text(), "Hello World");
            }
        }
    }
}
