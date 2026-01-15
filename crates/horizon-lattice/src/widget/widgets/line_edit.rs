//! Single-line text input widget.
//!
//! The LineEdit widget provides a single-line text editor with support for:
//! - Text editing with cursor and selection
//! - Placeholder text
//! - Password masking mode
//! - Read-only mode
//! - Maximum length constraint
//! - Undo/redo with coalescing for character input
//! - Input validation with visual feedback
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::LineEdit;
//! use horizon_lattice::widget::validator::{IntValidator, ValidationState};
//!
//! // Create a simple text input
//! let mut edit = LineEdit::new();
//! edit.set_placeholder("Enter your name...");
//!
//! // Create a password field
//! let mut password = LineEdit::new()
//!     .with_echo_mode(EchoMode::Password);
//!
//! // Create a validated numeric input
//! let mut age_input = LineEdit::new();
//! age_input.set_validator(IntValidator::new(0, 150));
//! age_input.set_placeholder("Enter age (0-150)...");
//!
//! // Connect to signals
//! edit.text_changed.connect(|text| {
//!     println!("Text changed: {}", text);
//! });
//!
//! edit.text_edited.connect(|text| {
//!     println!("Text edited (before validation): {}", text);
//! });
//!
//! edit.return_pressed.connect(|| {
//!     println!("Enter pressed!");
//! });
//! ```

use std::sync::Arc;

use parking_lot::RwLock;
use unicode_segmentation::UnicodeSegmentation;

use crate::platform::Clipboard;
use crate::widget::input_mask::InputMask;
use crate::widget::validator::{ValidationState, Validator};
use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, Size, Stroke, TextLayout,
    TextLayoutOptions, TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent,
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
    /// Returns true if merge was successful.
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
                if *pos + text.len() == *other_pos {
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
                // Backspace: deletion at position before current
                if *other_pos + other_text.len() == *pos {
                    // Prepend the deleted text
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
    /// Current position in the stack (commands after this are redo-able).
    index: usize,
    /// Maximum number of commands to keep.
    max_size: usize,
    /// Whether to attempt merging the next command.
    merge_enabled: bool,
}

impl UndoStack {
    /// Create a new undo stack.
    fn new() -> Self {
        Self {
            commands: Vec::new(),
            index: 0,
            max_size: 100,
            merge_enabled: true,
        }
    }

    /// Push a new command onto the stack.
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

        // Add the new command
        self.commands.push(command);
        self.index = self.commands.len();

        // Enforce max size by removing oldest commands
        if self.commands.len() > self.max_size {
            let excess = self.commands.len() - self.max_size;
            self.commands.drain(0..excess);
            self.index = self.commands.len();
        }
    }

    /// Check if undo is available.
    fn can_undo(&self) -> bool {
        self.index > 0
    }

    /// Check if redo is available.
    fn can_redo(&self) -> bool {
        self.index < self.commands.len()
    }

    /// Get the command to undo (if any) and decrement index.
    fn undo(&mut self) -> Option<&EditCommand> {
        if self.can_undo() {
            self.index -= 1;
            // Disable merging after undo to prevent merging new edits with old
            self.merge_enabled = false;
            Some(&self.commands[self.index])
        } else {
            None
        }
    }

    /// Get the command to redo (if any) and increment index.
    fn redo(&mut self) -> Option<&EditCommand> {
        if self.can_redo() {
            let cmd = &self.commands[self.index];
            self.index += 1;
            // Disable merging after redo
            self.merge_enabled = false;
            Some(cmd)
        } else {
            None
        }
    }

    /// Clear all undo/redo history.
    fn clear(&mut self) {
        self.commands.clear();
        self.index = 0;
        self.merge_enabled = true;
    }

    /// Break the merge chain (next command won't merge with previous).
    fn break_merge(&mut self) {
        self.merge_enabled = false;
    }

    /// Enable merging for the next command.
    fn enable_merge(&mut self) {
        self.merge_enabled = true;
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new()
    }
}

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
/// - Input validation with visual feedback
///
/// # Signals
///
/// - `text_changed`: Emitted when the text content changes
/// - `text_edited`: Emitted on any text change (emits before validation, unlike text_changed)
/// - `editing_finished`: Emitted when editing is finished (focus lost or Enter pressed)
/// - `return_pressed`: Emitted when Enter is pressed
/// - `input_rejected`: Emitted when input is rejected by the validator
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
/// - Ctrl+Z / Cmd+Z: Undo
/// - Ctrl+Shift+Z / Cmd+Shift+Z or Ctrl+Y: Redo
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

    /// Undo/redo stack for edit operations.
    undo_stack: UndoStack,

    /// Optional validator for input validation.
    validator: Option<Arc<dyn Validator>>,

    /// Current validation state.
    validation_state: ValidationState,

    /// Optional input mask for formatted input.
    input_mask: Option<InputMask>,

    /// User input characters (without mask literals) when mask is active.
    /// When no mask is active, this is empty and `text` is used directly.
    mask_input: String,

    // Signals

    /// Signal emitted when text changes (and validation passes, if a validator is set).
    pub text_changed: Signal<String>,

    /// Signal emitted on any text edit, before validation.
    /// This is useful for tracking all edits regardless of validation state.
    pub text_edited: Signal<String>,

    /// Signal emitted when editing is finished (focus lost or Enter pressed).
    /// With a validator, this is only emitted if input is acceptable.
    pub editing_finished: Signal<()>,

    /// Signal emitted when Enter/Return is pressed.
    pub return_pressed: Signal<()>,

    /// Signal emitted when input is rejected by the validator.
    pub input_rejected: Signal<()>,
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
            undo_stack: UndoStack::new(),
            validator: None,
            validation_state: ValidationState::Acceptable,
            input_mask: None,
            mask_input: String::new(),
            text_changed: Signal::new(),
            text_edited: Signal::new(),
            editing_finished: Signal::new(),
            return_pressed: Signal::new(),
            input_rejected: Signal::new(),
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
    ///
    /// When an input mask is active, this returns only the user-entered characters
    /// (without literal separators or blank characters).
    pub fn text(&self) -> &str {
        if self.input_mask.is_some() {
            &self.mask_input
        } else {
            &self.text
        }
    }

    /// Get the display text (what is shown to the user).
    ///
    /// When an input mask is active, this includes literal separators and
    /// blank characters for unfilled positions.
    pub fn display_text_value(&self) -> &str {
        &self.text
    }

    /// Set the text content.
    ///
    /// This clears any selection, moves the cursor to the end, and clears
    /// the undo history (since this is an external reset of the text).
    /// If max_length is set, the text will be truncated.
    ///
    /// When an input mask is active, the text is filtered to match the mask pattern.
    pub fn set_text(&mut self, text: impl Into<String>) {
        let new_text = text.into();

        if let Some(ref mask) = self.input_mask {
            // Filter input for mask
            let filtered = self.filter_for_mask(&new_text, mask);
            if self.mask_input != filtered {
                self.mask_input = filtered;
                self.text = mask.display_text(&self.mask_input);

                // Position cursor after last filled position
                let input_len = self.mask_input.chars().count();
                let display_pos = mask.input_pos_to_display_pos(input_len);
                self.cursor_pos = self.mask_display_pos_to_byte(display_pos);

                self.selection_anchor = None;
                self.undo_stack.clear();
                self.invalidate_layout();
                self.ensure_cursor_visible();
                self.base.update();

                self.text_edited.emit(self.mask_input.clone());
                self.revalidate();
                self.text_changed.emit(self.mask_input.clone());
            }
        } else {
            // No mask - original behavior
            let mut new_text = new_text;

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
                // Clear undo history since this is an external reset
                self.undo_stack.clear();
                self.invalidate_layout();
                self.ensure_cursor_visible();
                self.base.update();
                // Emit text_edited signal
                self.text_edited.emit(new_text.clone());
                // Validate and emit text_changed
                self.revalidate();
                self.text_changed.emit(new_text);
            }
        }
    }

    /// Clear all text.
    pub fn clear(&mut self) {
        if self.input_mask.is_some() {
            self.mask_input.clear();
            if let Some(ref mask) = self.input_mask {
                self.text = mask.display_text("");
            }
            if let Some(pos) = self.input_mask.as_ref().and_then(|m| m.first_editable_pos()) {
                self.cursor_pos = self.mask_display_pos_to_byte(pos);
            } else {
                self.cursor_pos = 0;
            }
            self.selection_anchor = None;
            self.undo_stack.clear();
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();

            self.text_edited.emit(String::new());
            self.revalidate();
            self.text_changed.emit(String::new());
        } else {
            self.set_text("");
        }
    }

    /// Get the text length in characters.
    ///
    /// When an input mask is active, this returns the number of user-entered characters.
    pub fn text_length(&self) -> usize {
        if self.input_mask.is_some() {
            self.mask_input.chars().count()
        } else {
            self.text.chars().count()
        }
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
    // Input Validation
    // =========================================================================

    /// Get the current validator, if any.
    pub fn validator(&self) -> Option<&Arc<dyn Validator>> {
        self.validator.as_ref()
    }

    /// Set a validator for this LineEdit.
    ///
    /// The validator will be used to validate input in real-time.
    /// Invalid input will be indicated visually.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::widget::validator::IntValidator;
    ///
    /// let mut edit = LineEdit::new();
    /// edit.set_validator(IntValidator::new(0, 100));
    /// ```
    pub fn set_validator<V: Validator + 'static>(&mut self, validator: V) {
        self.validator = Some(Arc::new(validator));
        self.revalidate();
    }

    /// Set a validator using a shared Arc.
    ///
    /// This is useful when you want to share a validator between multiple widgets.
    pub fn set_validator_arc(&mut self, validator: Arc<dyn Validator>) {
        self.validator = Some(validator);
        self.revalidate();
    }

    /// Remove the validator.
    pub fn clear_validator(&mut self) {
        self.validator = None;
        self.validation_state = ValidationState::Acceptable;
        self.base.update();
    }

    /// Set validator using builder pattern.
    pub fn with_validator<V: Validator + 'static>(mut self, validator: V) -> Self {
        self.set_validator(validator);
        self
    }

    /// Get the current validation state.
    ///
    /// Returns `Acceptable` if no validator is set.
    pub fn validation_state(&self) -> ValidationState {
        self.validation_state
    }

    /// Check if the current input is acceptable.
    ///
    /// Returns `true` if no validator is set, or if the validator
    /// returns `Acceptable` for the current text.
    pub fn has_acceptable_input(&self) -> bool {
        self.validation_state == ValidationState::Acceptable
    }

    /// Re-validate the current text against the validator.
    fn revalidate(&mut self) {
        let new_state = if let Some(ref validator) = self.validator {
            validator.validate(&self.text)
        } else {
            ValidationState::Acceptable
        };

        if self.validation_state != new_state {
            self.validation_state = new_state;
            self.base.update();
        }
    }

    // =========================================================================
    // Input Mask
    // =========================================================================

    /// Get the current input mask pattern, if any.
    ///
    /// Returns `None` if no mask is set.
    pub fn input_mask(&self) -> Option<&str> {
        self.input_mask.as_ref().map(|m| m.pattern())
    }

    /// Set an input mask to constrain user input to a specific pattern.
    ///
    /// The input mask defines which characters can be entered at each position,
    /// automatically inserts literal separators, and optionally transforms case.
    ///
    /// # Mask Characters
    ///
    /// | Char | Meaning |
    /// |------|---------|
    /// | `A`  | Letter required (A-Z, a-z) |
    /// | `a`  | Letter permitted but not required |
    /// | `N`  | Alphanumeric required (A-Z, a-z, 0-9) |
    /// | `n`  | Alphanumeric permitted but not required |
    /// | `X`  | Any non-blank character required |
    /// | `x`  | Any non-blank character permitted but not required |
    /// | `9`  | Digit required (0-9) |
    /// | `0`  | Digit permitted but not required |
    /// | `D`  | Digit 1-9 required (no zero) |
    /// | `d`  | Digit 1-9 permitted but not required |
    /// | `#`  | Digit or +/- sign permitted but not required |
    /// | `H`  | Hex character required (A-F, a-f, 0-9) |
    /// | `h`  | Hex character permitted but not required |
    /// | `B`  | Binary character required (0-1) |
    /// | `b`  | Binary character permitted but not required |
    ///
    /// # Meta Characters
    ///
    /// | Char | Meaning |
    /// |------|---------|
    /// | `>`  | All following alphabetic characters are uppercased |
    /// | `<`  | All following alphabetic characters are lowercased |
    /// | `!`  | Switch off case conversion |
    /// | `\`  | Escape the following character to use it as a literal |
    /// | `;c` | Terminates the mask and sets the blank character to `c` |
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::widget::widgets::LineEdit;
    ///
    /// let mut edit = LineEdit::new();
    ///
    /// // Phone number: (999) 999-9999
    /// edit.set_input_mask("(999) 999-9999");
    ///
    /// // IP address with underscore blanks
    /// edit.set_input_mask("000.000.000.000;_");
    ///
    /// // License key (uppercase)
    /// edit.set_input_mask(">AAAAA-AAAAA-AAAAA-AAAAA-AAAAA;#");
    /// ```
    pub fn set_input_mask(&mut self, mask: &str) {
        if mask.is_empty() {
            self.clear_input_mask();
            return;
        }

        if let Some(parsed) = InputMask::new(mask) {
            // Convert existing text to mask input if possible
            let old_text = std::mem::take(&mut self.text);
            self.mask_input = self.filter_for_mask(&old_text, &parsed);

            // Update display text
            self.text = parsed.display_text(&self.mask_input);

            self.input_mask = Some(parsed);

            // Position cursor at first editable position
            if let Some(pos) = self.input_mask.as_ref().and_then(|m| m.first_editable_pos()) {
                self.cursor_pos = self.mask_display_pos_to_byte(pos);
            } else {
                self.cursor_pos = 0;
            }

            self.selection_anchor = None;
            self.undo_stack.clear();
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();

            // Emit signals
            self.text_edited.emit(self.mask_input.clone());
            self.revalidate();
            self.text_changed.emit(self.mask_input.clone());
        }
    }

    /// Set input mask using builder pattern.
    pub fn with_input_mask(mut self, mask: &str) -> Self {
        self.set_input_mask(mask);
        self
    }

    /// Clear the input mask.
    ///
    /// The current text (without mask formatting) is preserved.
    pub fn clear_input_mask(&mut self) {
        if self.input_mask.is_some() {
            // Preserve the user input as the new text
            let user_input = std::mem::take(&mut self.mask_input);
            self.text = user_input;
            self.input_mask = None;

            self.cursor_pos = self.text.len();
            self.selection_anchor = None;
            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();

            self.text_edited.emit(self.text.clone());
            self.revalidate();
            self.text_changed.emit(self.text.clone());
        }
    }

    /// Check if an input mask is currently set.
    pub fn has_input_mask(&self) -> bool {
        self.input_mask.is_some()
    }

    /// Check if the current input satisfies all required positions in the mask.
    ///
    /// Returns `true` if no mask is set, or if all required mask positions have been filled.
    pub fn is_mask_complete(&self) -> bool {
        match &self.input_mask {
            Some(mask) => mask.is_complete(&self.mask_input),
            None => true,
        }
    }

    /// Filter input text to only contain characters valid for the mask.
    fn filter_for_mask(&self, text: &str, mask: &InputMask) -> String {
        let mut result = String::new();
        let mut text_chars = text.chars();
        let editable_count = mask.editable_count();

        for i in 0..editable_count {
            let display_pos = mask.input_pos_to_display_pos(i);
            if let Some(element) = mask.element_at(display_pos) {
                // Find next char from input that matches this position
                while let Some(ch) = text_chars.next() {
                    if element.accepts(ch) {
                        result.push(element.transform(ch));
                        break;
                    }
                }
            }
        }

        result
    }

    /// Convert mask display position (0-based element index) to byte position in text.
    fn mask_display_pos_to_byte(&self, display_pos: usize) -> usize {
        // Each display position corresponds to one character
        self.text
            .char_indices()
            .nth(display_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    /// Convert byte position in text to mask display position.
    fn byte_to_mask_display_pos(&self, byte_pos: usize) -> usize {
        self.text[..byte_pos.min(self.text.len())].chars().count()
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
        self.insert_text_impl(text, true);
    }

    /// Insert text implementation with optional undo recording.
    fn insert_text_impl(&mut self, text: &str, record_undo: bool) {
        if self.read_only || text.is_empty() {
            return;
        }

        // Handle input mask mode
        if self.input_mask.is_some() {
            self.insert_text_masked(text, record_undo);
            return;
        }

        // Delete selection first if any
        if self.has_selection() {
            self.delete_selection_impl(record_undo);
        }

        let insert_pos = self.cursor_pos;

        // Check max length
        let actual_text = if let Some(max) = self.max_length {
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
                truncated
            } else {
                self.text.insert_str(self.cursor_pos, text);
                self.cursor_pos += text.len();
                text.to_string()
            }
        } else {
            self.text.insert_str(self.cursor_pos, text);
            self.cursor_pos += text.len();
            text.to_string()
        };

        // Record undo command
        if record_undo && !actual_text.is_empty() {
            self.undo_stack.push(EditCommand::Insert {
                pos: insert_pos,
                text: actual_text,
            });
        }

        self.invalidate_layout();
        self.ensure_cursor_visible();
        self.base.update();

        // Emit text_edited signal (fires on any change, before validation)
        self.text_edited.emit(self.text.clone());

        // Validate and emit text_changed
        self.revalidate();
        self.text_changed.emit(self.text.clone());
    }

    /// Insert text with input mask active.
    fn insert_text_masked(&mut self, text: &str, record_undo: bool) {
        let mask = match &self.input_mask {
            Some(m) => m.clone(),
            None => return,
        };

        // Delete selection first if any
        if self.has_selection() {
            self.delete_selection_impl(record_undo);
        }

        // Current display position
        let mut display_pos = self.byte_to_mask_display_pos(self.cursor_pos);

        let old_mask_input = self.mask_input.clone();
        let mut inserted_chars = String::new();

        for ch in text.chars() {
            // Find the next editable position
            let editable_pos = match mask.next_editable_pos(display_pos) {
                Some(pos) => pos,
                None => break, // No more editable positions
            };

            // Check if this character is valid for this position
            if let Some(element) = mask.element_at(editable_pos) {
                if element.accepts(ch) {
                    let transformed = element.transform(ch);

                    // Calculate the input position for this display position
                    let input_pos = mask.display_pos_to_input_pos(editable_pos);

                    // Insert or replace the character in mask_input
                    if input_pos < self.mask_input.chars().count() {
                        // Replace existing character
                        let mut chars: Vec<char> = self.mask_input.chars().collect();
                        chars[input_pos] = transformed;
                        self.mask_input = chars.into_iter().collect();
                    } else {
                        // Append new character
                        self.mask_input.push(transformed);
                    }

                    inserted_chars.push(transformed);

                    // Move to next position after this editable one
                    display_pos = editable_pos + 1;
                }
            }
        }

        if !inserted_chars.is_empty() {
            // Update display text
            self.text = mask.display_text(&self.mask_input);

            // Move cursor to next editable position
            if let Some(next_pos) = mask.next_editable_pos(display_pos) {
                self.cursor_pos = self.mask_display_pos_to_byte(next_pos);
            } else {
                // No more editable positions, move to end
                self.cursor_pos = self.text.len();
            }

            // Record undo command
            if record_undo {
                self.undo_stack.push(EditCommand::Insert {
                    pos: 0, // For mask, we track the whole mask_input change
                    text: inserted_chars,
                });
            }

            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();

            // Emit signals with mask_input (user-entered text)
            self.text_edited.emit(self.mask_input.clone());
            self.revalidate();
            self.text_changed.emit(self.mask_input.clone());
        } else if old_mask_input != self.mask_input {
            // Mask input changed but no characters inserted (shouldn't happen, but safety)
            self.invalidate_layout();
            self.base.update();
        }
    }

    /// Delete the selected text.
    fn delete_selection(&mut self) {
        self.delete_selection_impl(true);
    }

    /// Delete selection implementation with optional undo recording.
    fn delete_selection_impl(&mut self, record_undo: bool) {
        if let Some((start, end)) = self.selection_range() {
            // Handle input mask mode
            if self.input_mask.is_some() {
                self.delete_selection_masked(start, end, record_undo);
                return;
            }

            let deleted_text = self.text[start..end].to_string();

            self.text.replace_range(start..end, "");
            self.cursor_pos = start;
            self.selection_anchor = None;

            // Record undo command
            if record_undo && !deleted_text.is_empty() {
                // Break merge chain since selection delete is a distinct operation
                self.undo_stack.break_merge();
                self.undo_stack.push(EditCommand::Delete {
                    pos: start,
                    text: deleted_text,
                });
                self.undo_stack.break_merge();
            }

            self.invalidate_layout();
            self.base.update();

            // Emit text_edited signal (fires on any change, before validation)
            self.text_edited.emit(self.text.clone());

            // Validate and emit text_changed
            self.revalidate();
            self.text_changed.emit(self.text.clone());
        }
    }

    /// Delete selection with input mask active.
    fn delete_selection_masked(&mut self, start: usize, end: usize, record_undo: bool) {
        let mask = match &self.input_mask {
            Some(m) => m.clone(),
            None => return,
        };

        // Convert byte positions to display positions
        let start_display = self.byte_to_mask_display_pos(start);
        let end_display = self.byte_to_mask_display_pos(end);

        // Find which input positions to delete
        let start_input = mask.display_pos_to_input_pos(start_display);
        let end_input = mask.display_pos_to_input_pos(end_display);

        if start_input < end_input && start_input < self.mask_input.chars().count() {
            // Delete the range from mask_input
            let deleted: String = self.mask_input.chars()
                .skip(start_input)
                .take(end_input - start_input)
                .collect();

            let mut chars: Vec<char> = self.mask_input.chars().collect();
            // Remove the characters in the range
            chars.drain(start_input..end_input.min(chars.len()));
            self.mask_input = chars.into_iter().collect();

            // Update display text
            self.text = mask.display_text(&self.mask_input);

            // Position cursor at the start of deletion
            if let Some(pos) = mask.next_editable_pos(start_display) {
                self.cursor_pos = self.mask_display_pos_to_byte(pos);
            } else {
                self.cursor_pos = start;
            }
            self.selection_anchor = None;

            // Record undo command
            if record_undo && !deleted.is_empty() {
                self.undo_stack.break_merge();
                self.undo_stack.push(EditCommand::Delete {
                    pos: start_input,
                    text: deleted,
                });
                self.undo_stack.break_merge();
            }

            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();

            self.text_edited.emit(self.mask_input.clone());
            self.revalidate();
            self.text_changed.emit(self.mask_input.clone());
        } else {
            // Nothing to delete, just clear selection
            self.selection_anchor = None;
            self.base.update();
        }
    }

    /// Delete character before cursor (backspace).
    fn delete_char_before(&mut self) {
        self.delete_char_before_impl(true);
    }

    /// Delete character before cursor implementation with optional undo recording.
    fn delete_char_before_impl(&mut self, record_undo: bool) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection_impl(record_undo);
            return;
        }

        // Handle input mask mode
        if self.input_mask.is_some() {
            self.delete_char_before_masked(record_undo);
            return;
        }

        if self.cursor_pos > 0 {
            let prev_pos = self.prev_grapheme_boundary(self.cursor_pos);
            let deleted_text = self.text[prev_pos..self.cursor_pos].to_string();

            self.text.replace_range(prev_pos..self.cursor_pos, "");
            self.cursor_pos = prev_pos;

            // Record undo command (backspace deletions can be merged)
            if record_undo && !deleted_text.is_empty() {
                self.undo_stack.enable_merge();
                self.undo_stack.push(EditCommand::Delete {
                    pos: prev_pos,
                    text: deleted_text,
                });
            }

            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();

            // Emit text_edited signal (fires on any change, before validation)
            self.text_edited.emit(self.text.clone());

            // Validate and emit text_changed
            self.revalidate();
            self.text_changed.emit(self.text.clone());
        }
    }

    /// Delete character before cursor with input mask active.
    fn delete_char_before_masked(&mut self, record_undo: bool) {
        let mask = match &self.input_mask {
            Some(m) => m.clone(),
            None => return,
        };

        let display_pos = self.byte_to_mask_display_pos(self.cursor_pos);

        // Find the previous editable position
        if let Some(prev_editable) = mask.prev_editable_pos(display_pos) {
            let input_pos = mask.display_pos_to_input_pos(prev_editable);

            if input_pos < self.mask_input.chars().count() {
                // Delete the character at this position
                let deleted: String = self.mask_input.chars().nth(input_pos).into_iter().collect();

                let mut chars: Vec<char> = self.mask_input.chars().collect();
                chars.remove(input_pos);
                self.mask_input = chars.into_iter().collect();

                // Update display text
                self.text = mask.display_text(&self.mask_input);

                // Move cursor to the deleted position
                self.cursor_pos = self.mask_display_pos_to_byte(prev_editable);

                // Record undo command
                if record_undo && !deleted.is_empty() {
                    self.undo_stack.enable_merge();
                    self.undo_stack.push(EditCommand::Delete {
                        pos: input_pos,
                        text: deleted,
                    });
                }

                self.invalidate_layout();
                self.ensure_cursor_visible();
                self.base.update();

                self.text_edited.emit(self.mask_input.clone());
                self.revalidate();
                self.text_changed.emit(self.mask_input.clone());
            }
        }
    }

    /// Delete character after cursor (delete).
    fn delete_char_after(&mut self) {
        self.delete_char_after_impl(true);
    }

    /// Delete character after cursor implementation with optional undo recording.
    fn delete_char_after_impl(&mut self, record_undo: bool) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection_impl(record_undo);
            return;
        }

        // Handle input mask mode
        if self.input_mask.is_some() {
            self.delete_char_after_masked(record_undo);
            return;
        }

        if self.cursor_pos < self.text.len() {
            let next_pos = self.next_grapheme_boundary(self.cursor_pos);
            let deleted_text = self.text[self.cursor_pos..next_pos].to_string();

            self.text.replace_range(self.cursor_pos..next_pos, "");

            // Record undo command (forward deletions can be merged)
            if record_undo && !deleted_text.is_empty() {
                self.undo_stack.enable_merge();
                self.undo_stack.push(EditCommand::Delete {
                    pos: self.cursor_pos,
                    text: deleted_text,
                });
            }

            self.invalidate_layout();
            self.base.update();

            // Emit text_edited signal (fires on any change, before validation)
            self.text_edited.emit(self.text.clone());

            // Validate and emit text_changed
            self.revalidate();
            self.text_changed.emit(self.text.clone());
        }
    }

    /// Delete character after cursor with input mask active.
    fn delete_char_after_masked(&mut self, record_undo: bool) {
        let mask = match &self.input_mask {
            Some(m) => m.clone(),
            None => return,
        };

        let display_pos = self.byte_to_mask_display_pos(self.cursor_pos);

        // Find the current or next editable position
        if let Some(editable_pos) = mask.next_editable_pos(display_pos) {
            let input_pos = mask.display_pos_to_input_pos(editable_pos);

            if input_pos < self.mask_input.chars().count() {
                // Delete the character at this position
                let deleted: String = self.mask_input.chars().nth(input_pos).into_iter().collect();

                let mut chars: Vec<char> = self.mask_input.chars().collect();
                chars.remove(input_pos);
                self.mask_input = chars.into_iter().collect();

                // Update display text
                self.text = mask.display_text(&self.mask_input);

                // Keep cursor at current position (or move to editable if needed)
                self.cursor_pos = self.mask_display_pos_to_byte(editable_pos);

                // Record undo command
                if record_undo && !deleted.is_empty() {
                    self.undo_stack.enable_merge();
                    self.undo_stack.push(EditCommand::Delete {
                        pos: input_pos,
                        text: deleted,
                    });
                }

                self.invalidate_layout();
                self.base.update();

                self.text_edited.emit(self.mask_input.clone());
                self.revalidate();
                self.text_changed.emit(self.mask_input.clone());
            }
        }
    }

    /// Delete word before cursor.
    fn delete_word_before(&mut self) {
        self.delete_word_before_impl(true);
    }

    /// Delete word before cursor implementation with optional undo recording.
    fn delete_word_before_impl(&mut self, record_undo: bool) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection_impl(record_undo);
            return;
        }

        if self.cursor_pos > 0 {
            let word_start = self.word_boundary_before(self.cursor_pos);
            let deleted_text = self.text[word_start..self.cursor_pos].to_string();

            self.text.replace_range(word_start..self.cursor_pos, "");
            self.cursor_pos = word_start;

            // Record undo command (word deletions break merge chain)
            if record_undo && !deleted_text.is_empty() {
                self.undo_stack.break_merge();
                self.undo_stack.push(EditCommand::Delete {
                    pos: word_start,
                    text: deleted_text,
                });
                self.undo_stack.break_merge();
            }

            self.invalidate_layout();
            self.ensure_cursor_visible();
            self.base.update();

            // Emit text_edited signal (fires on any change, before validation)
            self.text_edited.emit(self.text.clone());

            // Validate and emit text_changed
            self.revalidate();
            self.text_changed.emit(self.text.clone());
        }
    }

    /// Delete word after cursor.
    fn delete_word_after(&mut self) {
        self.delete_word_after_impl(true);
    }

    /// Delete word after cursor implementation with optional undo recording.
    fn delete_word_after_impl(&mut self, record_undo: bool) {
        if self.read_only {
            return;
        }

        if self.has_selection() {
            self.delete_selection_impl(record_undo);
            return;
        }

        if self.cursor_pos < self.text.len() {
            let word_end = self.word_boundary_after(self.cursor_pos);
            let deleted_text = self.text[self.cursor_pos..word_end].to_string();

            self.text.replace_range(self.cursor_pos..word_end, "");

            // Record undo command (word deletions break merge chain)
            if record_undo && !deleted_text.is_empty() {
                self.undo_stack.break_merge();
                self.undo_stack.push(EditCommand::Delete {
                    pos: self.cursor_pos,
                    text: deleted_text,
                });
                self.undo_stack.break_merge();
            }

            self.invalidate_layout();
            self.base.update();

            // Emit text_edited signal (fires on any change, before validation)
            self.text_edited.emit(self.text.clone());

            // Validate and emit text_changed
            self.revalidate();
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
    // Undo/Redo Operations
    // =========================================================================

    /// Check if undo is available.
    ///
    /// Returns `true` if there are operations that can be undone.
    pub fn can_undo(&self) -> bool {
        self.undo_stack.can_undo()
    }

    /// Check if redo is available.
    ///
    /// Returns `true` if there are operations that can be redone.
    pub fn can_redo(&self) -> bool {
        self.undo_stack.can_redo()
    }

    /// Undo the last editing operation.
    ///
    /// Returns `true` if an operation was undone, `false` if there was
    /// nothing to undo or the widget is read-only.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut edit = LineEdit::new();
    /// edit.insert_text("Hello");
    /// assert_eq!(edit.text(), "Hello");
    ///
    /// edit.undo();
    /// assert_eq!(edit.text(), "");
    /// ```
    pub fn undo(&mut self) -> bool {
        if self.read_only {
            return false;
        }

        // Clone the command to avoid borrow issues
        let command = self.undo_stack.undo().cloned();

        if let Some(cmd) = command {
            match cmd {
                EditCommand::Insert { pos, text } => {
                    // Undo insert by deleting the text
                    let end = pos + text.len();
                    if end <= self.text.len() {
                        self.text.replace_range(pos..end, "");
                        self.cursor_pos = pos;
                        self.selection_anchor = None;
                        self.invalidate_layout();
                        self.ensure_cursor_visible();
                        self.base.update();
                        self.text_edited.emit(self.text.clone());
                        self.revalidate();
                        self.text_changed.emit(self.text.clone());
                    }
                }
                EditCommand::Delete { pos, text } => {
                    // Undo delete by inserting the text back
                    self.text.insert_str(pos, &text);
                    self.cursor_pos = pos + text.len();
                    self.selection_anchor = None;
                    self.invalidate_layout();
                    self.ensure_cursor_visible();
                    self.base.update();
                    self.text_edited.emit(self.text.clone());
                    self.revalidate();
                    self.text_changed.emit(self.text.clone());
                }
            }
            true
        } else {
            false
        }
    }

    /// Redo the last undone operation.
    ///
    /// Returns `true` if an operation was redone, `false` if there was
    /// nothing to redo or the widget is read-only.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut edit = LineEdit::new();
    /// edit.insert_text("Hello");
    /// edit.undo();
    /// assert_eq!(edit.text(), "");
    ///
    /// edit.redo();
    /// assert_eq!(edit.text(), "Hello");
    /// ```
    pub fn redo(&mut self) -> bool {
        if self.read_only {
            return false;
        }

        // Clone the command to avoid borrow issues
        let command = self.undo_stack.redo().cloned();

        if let Some(cmd) = command {
            match cmd {
                EditCommand::Insert { pos, text } => {
                    // Redo insert by inserting the text
                    self.text.insert_str(pos, &text);
                    self.cursor_pos = pos + text.len();
                    self.selection_anchor = None;
                    self.invalidate_layout();
                    self.ensure_cursor_visible();
                    self.base.update();
                    self.text_edited.emit(self.text.clone());
                    self.revalidate();
                    self.text_changed.emit(self.text.clone());
                }
                EditCommand::Delete { pos, text } => {
                    // Redo delete by removing the text
                    let end = pos + text.len();
                    if end <= self.text.len() {
                        self.text.replace_range(pos..end, "");
                        self.cursor_pos = pos;
                        self.selection_anchor = None;
                        self.invalidate_layout();
                        self.ensure_cursor_visible();
                        self.base.update();
                        self.text_edited.emit(self.text.clone());
                        self.revalidate();
                        self.text_changed.emit(self.text.clone());
                    }
                }
            }
            true
        } else {
            false
        }
    }

    /// Clear the undo/redo history.
    ///
    /// This is useful when you want to reset the undo state without
    /// changing the text content.
    pub fn clear_undo_history(&mut self) {
        self.undo_stack.clear();
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
            // Handle input mask: skip to previous editable position
            if let Some(ref mask) = self.input_mask {
                let display_pos = self.byte_to_mask_display_pos(self.cursor_pos);
                if let Some(prev_editable) = mask.prev_editable_pos(display_pos) {
                    self.cursor_pos = self.mask_display_pos_to_byte(prev_editable);
                } else {
                    // No previous editable, move to first position
                    if let Some(first) = mask.first_editable_pos() {
                        self.cursor_pos = self.mask_display_pos_to_byte(first);
                    }
                }
            } else {
                self.cursor_pos = self.prev_grapheme_boundary(self.cursor_pos);
            }
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
            // Handle input mask: skip to next editable position
            if let Some(ref mask) = self.input_mask {
                let display_pos = self.byte_to_mask_display_pos(self.cursor_pos);
                // Move past current position to find next editable
                if let Some(next_editable) = mask.next_editable_pos(display_pos + 1) {
                    self.cursor_pos = self.mask_display_pos_to_byte(next_editable);
                } else {
                    // No next editable, move to end
                    self.cursor_pos = self.text.len();
                }
            } else {
                self.cursor_pos = self.next_grapheme_boundary(self.cursor_pos);
            }
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

        // Handle input mask: move to first editable position
        if let Some(ref mask) = self.input_mask {
            if let Some(first) = mask.first_editable_pos() {
                self.cursor_pos = self.mask_display_pos_to_byte(first);
            } else {
                self.cursor_pos = 0;
            }
        } else {
            self.cursor_pos = 0;
        }
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

        // Handle input mask: move to position after last filled character
        if let Some(ref mask) = self.input_mask {
            let input_len = self.mask_input.chars().count();
            let display_pos = mask.input_pos_to_display_pos(input_len);
            // Try to find next editable position after last filled
            if let Some(next) = mask.next_editable_pos(display_pos) {
                self.cursor_pos = self.mask_display_pos_to_byte(next);
            } else {
                self.cursor_pos = self.text.len();
            }
        } else {
            self.cursor_pos = self.text.len();
        }
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
                // Try to fixup if we have a validator and input is not acceptable
                if !self.has_acceptable_input() {
                    if let Some(ref validator) = self.validator {
                        if let Some(fixed) = validator.fixup(&self.text) {
                            self.set_text(fixed);
                        }
                    }
                }

                // Only emit signals if input is acceptable (or no validator)
                if self.has_acceptable_input() {
                    self.return_pressed.emit(());
                    self.editing_finished.emit(());
                } else {
                    // Input rejected - emit input_rejected signal
                    self.input_rejected.emit(());
                }
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

            // Undo/Redo
            Key::Z if ctrl && shift => {
                // Ctrl+Shift+Z or Cmd+Shift+Z: Redo
                self.redo();
                true
            }
            Key::Z if ctrl => {
                // Ctrl+Z or Cmd+Z: Undo
                self.undo();
                true
            }
            Key::Y if ctrl => {
                // Ctrl+Y: Redo (alternative shortcut, common on Windows)
                self.redo();
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

        // Try to fixup if we have a validator and input is not acceptable
        if !self.has_acceptable_input() {
            if let Some(ref validator) = self.validator {
                if let Some(fixed) = validator.fixup(&self.text) {
                    self.set_text(fixed);
                }
            }
        }

        // Only emit editing_finished if input is acceptable (or no validator)
        if self.has_acceptable_input() {
            self.editing_finished.emit(());
        }

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

        // Draw background - tint based on validation state
        let bg_color = if !self.base.is_effectively_enabled() {
            Color::from_rgb8(245, 245, 245)
        } else {
            match self.validation_state {
                ValidationState::Invalid => Color::from_rgb8(255, 245, 245), // Light red tint
                ValidationState::Intermediate => Color::WHITE,
                ValidationState::Acceptable => Color::WHITE,
            }
        };
        ctx.renderer().fill_rect(rect, bg_color);

        // Draw border - color based on validation state and focus
        let border_color = if self.base.has_focus() {
            match self.validation_state {
                ValidationState::Invalid => Color::from_rgb8(220, 53, 69),    // Red for invalid
                ValidationState::Intermediate => Color::from_rgb8(255, 193, 7), // Yellow/amber for intermediate
                ValidationState::Acceptable => Color::from_rgb8(51, 153, 255),  // Blue for acceptable
            }
        } else {
            match self.validation_state {
                ValidationState::Invalid => Color::from_rgb8(220, 53, 69),    // Red for invalid
                ValidationState::Intermediate => Color::from_rgb8(200, 200, 200), // Gray for intermediate (unfocused)
                ValidationState::Acceptable => Color::from_rgb8(200, 200, 200),   // Gray for acceptable (unfocused)
            }
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

    // =========================================================================
    // Undo/Redo Tests
    // =========================================================================

    #[test]
    fn test_undo_insert() {
        setup();
        let mut edit = LineEdit::new();

        // Insert text
        edit.insert_text("Hello");
        assert_eq!(edit.text(), "Hello");
        assert!(edit.can_undo());
        assert!(!edit.can_redo());

        // Undo should restore empty state
        assert!(edit.undo());
        assert_eq!(edit.text(), "");
        assert!(!edit.can_undo());
        assert!(edit.can_redo());
    }

    #[test]
    fn test_redo_insert() {
        setup();
        let mut edit = LineEdit::new();

        edit.insert_text("Hello");
        edit.undo();
        assert_eq!(edit.text(), "");

        // Redo should restore the text
        assert!(edit.redo());
        assert_eq!(edit.text(), "Hello");
        assert!(edit.can_undo());
        assert!(!edit.can_redo());
    }

    #[test]
    fn test_undo_delete_backspace() {
        setup();
        let mut edit = LineEdit::with_text("Hello");
        edit.undo_stack.clear(); // Clear undo from set_text in with_text

        // Delete last character
        edit.delete_char_before();
        assert_eq!(edit.text(), "Hell");

        // Undo should restore the 'o'
        assert!(edit.undo());
        assert_eq!(edit.text(), "Hello");
    }

    #[test]
    fn test_undo_delete_forward() {
        setup();
        let mut edit = LineEdit::with_text("Hello");
        edit.set_cursor_position(0);
        edit.undo_stack.clear();

        // Delete first character
        edit.delete_char_after();
        assert_eq!(edit.text(), "ello");

        // Undo should restore the 'H'
        assert!(edit.undo());
        assert_eq!(edit.text(), "Hello");
    }

    #[test]
    fn test_undo_delete_selection() {
        setup();
        let mut edit = LineEdit::with_text("Hello World");
        edit.undo_stack.clear();

        // Select and delete "World"
        edit.selection_anchor = Some(6);
        edit.cursor_pos = 11;
        edit.delete_selection();
        assert_eq!(edit.text(), "Hello ");

        // Undo should restore "World"
        assert!(edit.undo());
        assert_eq!(edit.text(), "Hello World");
    }

    #[test]
    fn test_undo_coalescing_insert() {
        setup();
        let mut edit = LineEdit::new();

        // Type multiple characters - should coalesce
        edit.insert_text("H");
        edit.insert_text("e");
        edit.insert_text("l");
        edit.insert_text("l");
        edit.insert_text("o");
        assert_eq!(edit.text(), "Hello");

        // Single undo should remove all coalesced characters
        assert!(edit.undo());
        assert_eq!(edit.text(), "");

        // No more undo available
        assert!(!edit.can_undo());
    }

    #[test]
    fn test_undo_coalescing_backspace() {
        setup();
        let mut edit = LineEdit::with_text("Hello");
        edit.undo_stack.clear();

        // Delete multiple characters with backspace - should coalesce
        edit.delete_char_before();
        edit.delete_char_before();
        edit.delete_char_before();
        assert_eq!(edit.text(), "He");

        // Single undo should restore all deleted characters
        assert!(edit.undo());
        assert_eq!(edit.text(), "Hello");
    }

    #[test]
    fn test_undo_multiple_operations() {
        setup();
        let mut edit = LineEdit::new();

        // Multiple distinct operations
        edit.insert_text("Hello");
        edit.undo_stack.break_merge(); // Break coalescing
        edit.insert_text(" World");

        assert_eq!(edit.text(), "Hello World");

        // First undo removes " World"
        assert!(edit.undo());
        assert_eq!(edit.text(), "Hello");

        // Second undo removes "Hello"
        assert!(edit.undo());
        assert_eq!(edit.text(), "");

        // Redo restores "Hello"
        assert!(edit.redo());
        assert_eq!(edit.text(), "Hello");

        // Redo restores " World"
        assert!(edit.redo());
        assert_eq!(edit.text(), "Hello World");
    }

    #[test]
    fn test_undo_clears_redo_on_new_edit() {
        setup();
        let mut edit = LineEdit::new();

        edit.insert_text("Hello");
        edit.undo();
        assert_eq!(edit.text(), "");
        assert!(edit.can_redo());

        // New edit should clear redo history
        edit.insert_text("World");
        assert!(!edit.can_redo());
        assert_eq!(edit.text(), "World");
    }

    #[test]
    fn test_undo_read_only_returns_false() {
        setup();
        let mut edit = LineEdit::new();
        edit.insert_text("Hello");
        edit.set_read_only(true);

        // Undo should fail when read-only
        assert!(!edit.undo());
        assert_eq!(edit.text(), "Hello");
    }

    #[test]
    fn test_redo_read_only_returns_false() {
        setup();
        let mut edit = LineEdit::new();
        edit.insert_text("Hello");
        edit.undo();
        edit.set_read_only(true);

        // Redo should fail when read-only
        assert!(!edit.redo());
        assert_eq!(edit.text(), "");
    }

    #[test]
    fn test_set_text_clears_undo() {
        setup();
        let mut edit = LineEdit::new();
        edit.insert_text("Hello");
        assert!(edit.can_undo());

        // set_text should clear undo history
        edit.set_text("New text");
        assert!(!edit.can_undo());
    }

    #[test]
    fn test_clear_undo_history() {
        setup();
        let mut edit = LineEdit::new();
        edit.insert_text("Hello");
        assert!(edit.can_undo());

        edit.clear_undo_history();
        assert!(!edit.can_undo());
        assert!(!edit.can_redo());
    }

    #[test]
    fn test_undo_word_delete() {
        setup();
        let mut edit = LineEdit::with_text("Hello World");
        edit.undo_stack.clear();

        // Delete "World"
        edit.delete_word_before();
        assert_eq!(edit.text(), "Hello ");

        // Undo should restore "World"
        assert!(edit.undo());
        assert_eq!(edit.text(), "Hello World");
    }

    // =========================================================================
    // Input Validation Tests
    // =========================================================================

    #[test]
    fn test_no_validator_always_acceptable() {
        setup();
        let edit = LineEdit::new();
        assert_eq!(edit.validation_state(), ValidationState::Acceptable);
        assert!(edit.has_acceptable_input());
    }

    #[test]
    fn test_int_validator_basic() {
        setup();
        use crate::widget::validator::IntValidator;

        let mut edit = LineEdit::new();
        edit.set_validator(IntValidator::new(0, 100));

        // Empty input is intermediate
        assert_eq!(edit.validation_state(), ValidationState::Intermediate);

        // Valid input
        edit.set_text("50");
        assert_eq!(edit.validation_state(), ValidationState::Acceptable);
        assert!(edit.has_acceptable_input());

        // Out of range (too high)
        edit.set_text("150");
        assert_eq!(edit.validation_state(), ValidationState::Invalid);
        assert!(!edit.has_acceptable_input());

        // Out of range (negative)
        edit.set_text("-5");
        assert_eq!(edit.validation_state(), ValidationState::Invalid);
    }

    #[test]
    fn test_int_validator_intermediate() {
        setup();
        use crate::widget::validator::IntValidator;

        let mut edit = LineEdit::new();
        edit.set_validator(IntValidator::new(10, 100));

        // Single digit that could become valid
        edit.set_text("5");
        assert_eq!(edit.validation_state(), ValidationState::Intermediate);
    }

    #[test]
    fn test_double_validator_basic() {
        setup();
        use crate::widget::validator::DoubleValidator;

        let mut edit = LineEdit::new();
        edit.set_validator(DoubleValidator::new(0.0, 10.0, 2));

        // Valid input
        edit.set_text("5.5");
        assert_eq!(edit.validation_state(), ValidationState::Acceptable);

        // Trailing decimal is intermediate
        edit.set_text("5.");
        assert_eq!(edit.validation_state(), ValidationState::Intermediate);

        // Out of range
        edit.set_text("15.0");
        assert_eq!(edit.validation_state(), ValidationState::Invalid);

        // Too many decimal places
        edit.set_text("5.555");
        assert_eq!(edit.validation_state(), ValidationState::Invalid);
    }

    #[test]
    fn test_regex_validator_basic() {
        setup();
        use crate::widget::validator::RegexValidator;

        let mut edit = LineEdit::new();
        // Simple pattern: exactly 3 digits
        edit.set_validator(RegexValidator::new(r"^\d{3}$").unwrap());

        // Empty is intermediate
        assert_eq!(edit.validation_state(), ValidationState::Intermediate);

        // Valid input
        edit.set_text("123");
        assert_eq!(edit.validation_state(), ValidationState::Acceptable);

        // Partial input is intermediate
        edit.set_text("12");
        assert_eq!(edit.validation_state(), ValidationState::Intermediate);
    }

    #[test]
    fn test_clear_validator() {
        setup();
        use crate::widget::validator::IntValidator;

        let mut edit = LineEdit::new();
        edit.set_validator(IntValidator::new(0, 10));

        edit.set_text("abc");
        assert_eq!(edit.validation_state(), ValidationState::Invalid);

        // Clear validator - should become acceptable
        edit.clear_validator();
        assert_eq!(edit.validation_state(), ValidationState::Acceptable);
    }

    #[test]
    fn test_with_validator_builder() {
        setup();
        use crate::widget::validator::IntValidator;

        let edit = LineEdit::with_text("50")
            .with_validator(IntValidator::new(0, 100));

        assert_eq!(edit.validation_state(), ValidationState::Acceptable);
    }

    #[test]
    fn test_validation_on_edit_operations() {
        setup();
        use crate::widget::validator::IntValidator;

        let mut edit = LineEdit::new();
        // Use range [10, 100] so single digit "5" is intermediate, not acceptable
        edit.set_validator(IntValidator::new(10, 100));

        // Insert text - "5" is intermediate (could become "50")
        edit.insert_text("5");
        assert_eq!(edit.validation_state(), ValidationState::Intermediate);

        edit.insert_text("0");
        assert_eq!(edit.text(), "50");
        assert_eq!(edit.validation_state(), ValidationState::Acceptable);

        // Delete makes it incomplete again
        edit.delete_char_before();
        assert_eq!(edit.text(), "5");
        assert_eq!(edit.validation_state(), ValidationState::Intermediate);
    }

    #[test]
    fn test_text_edited_signal() {
        setup();
        use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

        let mut edit = LineEdit::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        edit.text_edited.connect(move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Text edits should trigger the signal
        edit.insert_text("Hello");
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        edit.delete_char_before();
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        edit.set_text("New");
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    // =========================================================================
    // Input Mask Tests
    // =========================================================================

    #[test]
    fn test_input_mask_set_and_clear() {
        setup();
        let mut edit = LineEdit::new();

        edit.set_input_mask("(999) 999-9999");
        assert!(edit.has_input_mask());
        assert_eq!(edit.input_mask(), Some("(999) 999-9999"));

        edit.clear_input_mask();
        assert!(!edit.has_input_mask());
        assert_eq!(edit.input_mask(), None);
    }

    #[test]
    fn test_input_mask_display() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask("(999) 999-9999");

        // Display should show blank placeholders
        assert_eq!(edit.display_text_value(), "(   )    -    ");
        assert_eq!(edit.text(), "");
    }

    #[test]
    fn test_input_mask_insert_digits() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask("(999) 999-9999");

        edit.insert_text("5");
        assert_eq!(edit.text(), "5");
        assert_eq!(edit.display_text_value(), "(5  )    -    ");

        edit.insert_text("55123");
        assert_eq!(edit.text(), "555123");
        assert_eq!(edit.display_text_value(), "(555) 123-    ");
    }

    #[test]
    fn test_input_mask_rejects_invalid_chars() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask("999-999");

        // Letters should be rejected for digit-only mask
        edit.insert_text("abc");
        assert_eq!(edit.text(), "");

        // Digits should be accepted
        edit.insert_text("123");
        assert_eq!(edit.text(), "123");
    }

    #[test]
    fn test_input_mask_backspace() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask("999-999");
        edit.insert_text("123456");
        assert_eq!(edit.text(), "123456");

        edit.delete_char_before();
        assert_eq!(edit.text(), "12345");
    }

    #[test]
    fn test_input_mask_set_text() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask("(999) 999-9999");

        edit.set_text("5551234567");
        assert_eq!(edit.text(), "5551234567");
        assert_eq!(edit.display_text_value(), "(555) 123-4567");
    }

    #[test]
    fn test_input_mask_clear_text() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask("999-999");
        edit.insert_text("123456");

        edit.clear();
        assert_eq!(edit.text(), "");
        assert_eq!(edit.display_text_value(), "   -   ");
    }

    #[test]
    fn test_input_mask_is_complete() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask("999-999");

        // Empty is not complete
        assert!(!edit.is_mask_complete());

        // Partial is not complete
        edit.insert_text("123");
        assert!(!edit.is_mask_complete());

        // Full is complete
        edit.insert_text("456");
        assert!(edit.is_mask_complete());
    }

    #[test]
    fn test_input_mask_case_conversion() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask(">AAAAA");

        edit.insert_text("hello");
        assert_eq!(edit.text(), "HELLO");
    }

    #[test]
    fn test_input_mask_with_blank_char() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask("000.000.000.000;_");

        assert_eq!(edit.display_text_value(), "___.___.___.___");

        edit.insert_text("192168");
        assert_eq!(edit.display_text_value(), "192.168.___.___");
    }

    #[test]
    fn test_input_mask_hex() {
        setup();
        let mut edit = LineEdit::new();
        edit.set_input_mask("HH:HH:HH");

        edit.insert_text("AABBCC");
        assert_eq!(edit.text(), "AABBCC");
        assert_eq!(edit.display_text_value(), "AA:BB:CC");
    }

    #[test]
    fn test_input_mask_builder() {
        setup();
        let edit = LineEdit::new()
            .with_input_mask("999-999")
            .with_placeholder("Enter code...");

        assert!(edit.has_input_mask());
        assert_eq!(edit.placeholder(), "Enter code...");
    }
}
