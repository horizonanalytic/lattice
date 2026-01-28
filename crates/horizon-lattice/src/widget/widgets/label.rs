//! Label widget for text display.
//!
//! The Label widget displays text with support for:
//! - Single-line and multi-line text
//! - Text alignment (horizontal and vertical)
//! - Word wrapping
//! - Text elision (truncation with ellipsis)
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice::widget::widgets::{Label, ElideMode};
//! use horizon_lattice_render::{Color, HorizontalAlign};
//!
//! // Create a simple label
//! let label = Label::new("Hello, World!");
//!
//! // Create a label with word wrapping
//! let wrapped = Label::new("Long text that will wrap...")
//!     .with_word_wrap(true);
//!
//! // Create a label with elision
//! let elided = Label::new("Very long filename.txt")
//!     .with_elide_mode(ElideMode::Right);
//!
//! // Customize alignment and color
//! let styled = Label::new("Centered text")
//!     .with_horizontal_align(HorizontalAlign::Center)
//!     .with_text_color(Color::from_rgb8(100, 100, 100));
//! ```

use parking_lot::RwLock;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, HorizontalAlign, Point, Rect, Renderer, RichText,
    RichTextSpan, Size, TextLayout, TextLayoutOptions, TextRenderer, VerticalAlign, WrapMode,
};

use crate::widget::{
    CursorShape, FocusPolicy, Key, KeyPressEvent, MouseButton, MouseDoubleClickEvent,
    MouseMoveEvent, MousePressEvent, MouseReleaseEvent, PaintContext, SizeHint, SizePolicy,
    SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

/// Text elision mode for truncating text that doesn't fit.
///
/// When text is too long to fit in the available space and word wrapping
/// is disabled, the text can be truncated with an ellipsis ("...").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ElideMode {
    /// No elision - text may extend beyond bounds.
    #[default]
    None,
    /// Elide text at the left: "...filename.txt"
    Left,
    /// Elide text in the middle: "long...name.txt"
    Middle,
    /// Elide text at the right: "longfilena..."
    Right,
}

/// A widget that displays text.
///
/// Label is the primary widget for displaying non-editable text. It supports
/// various text formatting options including alignment, wrapping, elision,
/// and rich text with HTML formatting.
///
/// # Plain Text vs Rich Text
///
/// Labels can display either plain text or rich text:
///
/// ```no_run
/// use horizon_lattice::widget::widgets::Label;
///
/// // Plain text
/// let plain = Label::new("Hello, World!");
///
/// // Rich text from HTML
/// let rich = Label::from_html("Hello <b>bold</b> and <i>italic</i>!");
/// ```
///
/// Rich text supports basic HTML tags:
/// - `<b>`, `<strong>` for bold
/// - `<i>`, `<em>` for italic
/// - `<u>` for underline
/// - `<s>`, `<del>` for strikethrough
/// - `<font color="..." size="...">` for color and size
/// - `<br>` for line breaks
///
/// # Layout Behavior
///
/// By default, Label has a `Preferred` size policy in both dimensions.
/// When word wrapping is enabled, it supports height-for-width layout
/// negotiation.
///
/// # Performance
///
/// The Label caches its text layout and only recalculates when the text,
/// font, or layout options change. The cached layout is also used for
/// hit testing and size hint calculations.
pub struct Label {
    /// Widget base for common functionality.
    base: WidgetBase,

    /// The plain text to display (used when rich_text is None).
    text: String,

    /// Rich text content (takes precedence over text when Some).
    rich_text: Option<RichText>,

    /// Horizontal text alignment within the widget bounds.
    horizontal_align: HorizontalAlign,

    /// Vertical text alignment within the widget bounds.
    vertical_align: VerticalAlign,

    /// Whether word wrapping is enabled.
    word_wrap: bool,

    /// Text elision mode for truncating long text.
    elide_mode: ElideMode,

    /// The font to use for text rendering.
    font: Font,

    /// Text color.
    text_color: Color,

    /// Cached text layout for efficient rendering.
    /// Uses RwLock for thread-safe interior mutability since Widget requires Sync.
    cached_layout: RwLock<Option<CachedLayout>>,

    // =========================================================================
    // Mnemonic and Buddy Support
    // =========================================================================
    /// The mnemonic character (lowercase) for keyboard activation.
    ///
    /// This is extracted from text containing `&` prefix (e.g., "&Name" has mnemonic 'n').
    mnemonic_char: Option<char>,

    /// Byte position of the mnemonic character in the display text (for underlining).
    mnemonic_byte_pos: Option<usize>,

    /// The buddy widget that receives focus when the mnemonic is activated.
    buddy: Option<ObjectId>,

    /// Signal emitted when the text changes.
    pub text_changed: Signal<String>,

    /// Signal emitted when the mnemonic is activated (Alt+key pressed).
    ///
    /// Listeners can use this to perform custom actions when the mnemonic
    /// is triggered. The default behavior transfers focus to the buddy widget.
    pub mnemonic_activated: Signal<()>,

    // =========================================================================
    // Text Selection Support
    // =========================================================================
    /// Whether the label text can be selected with mouse/keyboard.
    selectable: bool,

    /// Current cursor position in text (byte offset).
    cursor_pos: usize,

    /// Selection anchor position (byte offset). If Some, selection extends from anchor to cursor.
    selection_anchor: Option<usize>,

    /// Whether we're currently dragging to select text.
    is_selecting: bool,

    /// Background color for selected text.
    selection_color: Color,

    /// Number of consecutive clicks for multi-click detection.
    click_count: u8,

    /// Signal emitted when the selection changes.
    pub selection_changed: Signal<()>,

    /// Signal emitted when text is copied to clipboard.
    pub copy_available: Signal<bool>,

    // =========================================================================
    // Link Handling Support
    // =========================================================================
    /// The URL of the link currently under the mouse cursor, if any.
    current_hovered_link: Option<String>,

    /// The URL of the link where a mouse press started, for click detection.
    link_click_pending: Option<String>,

    /// Signal emitted when a hyperlink is activated (clicked).
    ///
    /// The signal payload is the URL of the activated link.
    pub link_activated: Signal<String>,

    /// Signal emitted when the mouse hovers over or leaves a hyperlink.
    ///
    /// The signal payload is `Some(url)` when entering a link, or `None` when leaving.
    pub link_hovered: Signal<Option<String>>,
}

/// Cached text layout data.
struct CachedLayout {
    /// The computed text layout.
    layout: TextLayout,
    /// The width constraint used for this layout (None = unconstrained).
    width_constraint: Option<f32>,
    /// Whether the mnemonic underline is shown in this layout.
    show_mnemonic_underline: bool,
}

/// Parsed mnemonic information from text containing `&` characters.
struct MnemonicParseResult {
    /// The display text with `&` characters removed (except `&&` → `&`).
    display_text: String,
    /// The mnemonic character (lowercase), if found.
    mnemonic_char: Option<char>,
    /// Byte position of the mnemonic character in display_text.
    mnemonic_byte_pos: Option<usize>,
}

/// Parse text for mnemonic character.
///
/// The `&` character marks the following character as a mnemonic.
/// Use `&&` to display a literal `&` character.
///
/// Examples:
/// - `"&Name"` → display "Name", mnemonic 'n' at position 0
/// - `"Save && Exit"` → display "Save & Exit", no mnemonic
/// - `"&Save && &Exit"` → display "Save & Exit", mnemonic 's' at position 0
fn parse_mnemonic_text(text: &str) -> MnemonicParseResult {
    let mut display_text = String::with_capacity(text.len());
    let mut mnemonic_char = None;
    let mut mnemonic_byte_pos = None;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '&' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '&' {
                    // `&&` → literal `&`
                    display_text.push('&');
                    chars.next(); // consume second `&`
                } else if mnemonic_char.is_none() {
                    // First `&X` → mnemonic
                    mnemonic_byte_pos = Some(display_text.len());
                    // Use unicode-aware lowercase for mnemonic character
                    mnemonic_char = next_ch.to_lowercase().next();
                    display_text.push(next_ch);
                    chars.next(); // consume mnemonic char
                } else {
                    // Already have a mnemonic, just add the character
                    display_text.push(next_ch);
                    chars.next();
                }
            }
            // Trailing `&` at end of string is ignored
        } else {
            display_text.push(ch);
        }
    }

    MnemonicParseResult {
        display_text,
        mnemonic_char,
        mnemonic_byte_pos,
    }
}

impl Label {
    /// Create a new label with the specified text.
    ///
    /// The label is created with default settings:
    /// - Left-aligned horizontally, top-aligned vertically
    /// - No word wrapping
    /// - No elision
    /// - System sans-serif font at 14pt
    /// - Black text color
    ///
    /// # Mnemonic Support
    ///
    /// The text can contain `&` to mark a mnemonic character:
    /// - `"&Name"` displays "Name" with 'N' underlined, mnemonic is Alt+N
    /// - `"&&"` displays a literal `&` character
    ///
    /// To activate the mnemonic functionality, set a buddy widget with
    /// [`set_buddy`](Self::set_buddy).
    pub fn new(text: impl Into<String>) -> Self {
        let mut base = WidgetBase::new::<Self>();
        // Labels don't receive focus by default
        base.set_focus_policy(FocusPolicy::NoFocus);

        let raw_text = text.into();
        let parsed = parse_mnemonic_text(&raw_text);

        Self {
            base,
            text: parsed.display_text,
            rich_text: None,
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            word_wrap: false,
            elide_mode: ElideMode::None,
            font: Font::new(FontFamily::SansSerif, 14.0),
            text_color: Color::BLACK,
            cached_layout: RwLock::new(None),
            mnemonic_char: parsed.mnemonic_char,
            mnemonic_byte_pos: parsed.mnemonic_byte_pos,
            buddy: None,
            text_changed: Signal::new(),
            mnemonic_activated: Signal::new(),
            // Selection state (disabled by default)
            selectable: false,
            cursor_pos: 0,
            selection_anchor: None,
            is_selecting: false,
            selection_color: Color::from_rgba8(51, 153, 255, 128),
            click_count: 0,
            selection_changed: Signal::new(),
            copy_available: Signal::new(),
            // Link handling state
            current_hovered_link: None,
            link_click_pending: None,
            link_activated: Signal::new(),
            link_hovered: Signal::new(),
        }
    }

    /// Create a new label from HTML rich text.
    ///
    /// The HTML is parsed to extract formatting. Supported tags:
    /// - `<b>`, `<strong>` for bold
    /// - `<i>`, `<em>` for italic
    /// - `<u>` for underline
    /// - `<s>`, `<del>` for strikethrough
    /// - `<font color="..." size="...">` for color and size
    /// - `<br>` for line breaks
    /// - `<a href="...">` for hyperlinks
    ///
    /// Hyperlinks are automatically styled with blue color and underline.
    /// Connect to the `link_activated` and `link_hovered` signals to handle
    /// link interactions.
    ///
    /// Note: Rich text labels do not support mnemonics. Use plain text
    /// with `&` prefix for mnemonic support.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let label = Label::from_html("Visit <a href=\"https://example.com\">example</a>!");
    /// label.link_activated.connect(|url| {
    ///     println!("Link clicked: {}", url);
    /// });
    /// ```
    pub fn from_html(html: impl AsRef<str>) -> Self {
        let rich = RichText::from_html(html.as_ref());
        let plain = rich.plain_text();

        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::NoFocus);

        Self {
            base,
            text: plain,
            rich_text: Some(rich),
            horizontal_align: HorizontalAlign::Left,
            vertical_align: VerticalAlign::Top,
            word_wrap: false,
            elide_mode: ElideMode::None,
            font: Font::new(FontFamily::SansSerif, 14.0),
            text_color: Color::BLACK,
            cached_layout: RwLock::new(None),
            // Rich text does not support mnemonics
            mnemonic_char: None,
            mnemonic_byte_pos: None,
            buddy: None,
            text_changed: Signal::new(),
            mnemonic_activated: Signal::new(),
            // Selection state (disabled by default)
            selectable: false,
            cursor_pos: 0,
            selection_anchor: None,
            is_selecting: false,
            selection_color: Color::from_rgba8(51, 153, 255, 128),
            click_count: 0,
            selection_changed: Signal::new(),
            copy_available: Signal::new(),
            // Link handling state
            current_hovered_link: None,
            link_click_pending: None,
            link_activated: Signal::new(),
            link_hovered: Signal::new(),
        }
    }

    /// Get the current plain text content.
    ///
    /// If the label contains rich text, this returns the text without formatting.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the text to display (as plain text).
    ///
    /// This clears any rich text formatting and invalidates the cached layout.
    /// The text is parsed for mnemonic characters (`&` prefix).
    pub fn set_text(&mut self, text: impl Into<String>) {
        let raw_text = text.into();
        let parsed = parse_mnemonic_text(&raw_text);

        if self.text != parsed.display_text || self.rich_text.is_some() {
            self.text = parsed.display_text.clone();
            self.rich_text = None;
            self.mnemonic_char = parsed.mnemonic_char;
            self.mnemonic_byte_pos = parsed.mnemonic_byte_pos;
            self.invalidate_layout();
            self.base.update();
            self.text_changed.emit(parsed.display_text);
        }
    }

    /// Set the text using builder pattern.
    ///
    /// The text is parsed for mnemonic characters (`&` prefix).
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        let raw_text = text.into();
        let parsed = parse_mnemonic_text(&raw_text);
        self.text = parsed.display_text;
        self.rich_text = None;
        self.mnemonic_char = parsed.mnemonic_char;
        self.mnemonic_byte_pos = parsed.mnemonic_byte_pos;
        self.invalidate_layout();
        self
    }

    /// Check if this label has rich text formatting.
    pub fn has_rich_text(&self) -> bool {
        self.rich_text.is_some()
    }

    /// Get the rich text content, if any.
    pub fn rich_text(&self) -> Option<&RichText> {
        self.rich_text.as_ref()
    }

    /// Set rich text content.
    ///
    /// This replaces any existing plain text or rich text.
    /// Note: Rich text clears any mnemonic information.
    pub fn set_rich_text(&mut self, rich_text: RichText) {
        self.text = rich_text.plain_text();
        self.rich_text = Some(rich_text);
        self.mnemonic_char = None;
        self.mnemonic_byte_pos = None;
        self.invalidate_layout();
        self.base.update();
        self.text_changed.emit(self.text.clone());
    }

    /// Set rich text from HTML.
    ///
    /// This is a convenience method that parses the HTML and sets the rich text.
    pub fn set_html(&mut self, html: impl AsRef<str>) {
        self.set_rich_text(RichText::from_html(html.as_ref()));
    }

    /// Set rich text using builder pattern.
    pub fn with_rich_text(mut self, rich_text: RichText) -> Self {
        self.text = rich_text.plain_text();
        self.rich_text = Some(rich_text);
        self.mnemonic_char = None;
        self.mnemonic_byte_pos = None;
        self.invalidate_layout();
        self
    }

    /// Set rich text from HTML using builder pattern.
    pub fn with_html(self, html: impl AsRef<str>) -> Self {
        self.with_rich_text(RichText::from_html(html.as_ref()))
    }

    // =========================================================================
    // Mnemonic and Buddy Widget Support
    // =========================================================================

    /// Get the mnemonic character for this label.
    ///
    /// Returns the lowercase character that triggers the mnemonic when
    /// pressed with Alt (e.g., Alt+N for a label with text "&Name").
    /// Returns `None` if no mnemonic is defined.
    pub fn mnemonic(&self) -> Option<char> {
        self.mnemonic_char
    }

    /// Check if this label has a mnemonic character.
    pub fn has_mnemonic(&self) -> bool {
        self.mnemonic_char.is_some()
    }

    /// Get the buddy widget that receives focus when the mnemonic is activated.
    pub fn buddy(&self) -> Option<ObjectId> {
        self.buddy
    }

    /// Set the buddy widget.
    ///
    /// When the label's mnemonic is activated (Alt+key), focus will be
    /// transferred to this widget. This is commonly used in forms to
    /// associate a label with its input field.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Create a label with mnemonic and an input field
    /// let label = Label::new("&Name:");
    /// let input = LineEdit::new();
    ///
    /// // Associate the label with the input field
    /// label.set_buddy(Some(input.object_id()));
    ///
    /// // Now Alt+N will focus the input field
    /// ```
    pub fn set_buddy(&mut self, buddy: Option<ObjectId>) {
        self.buddy = buddy;
    }

    /// Set buddy widget using builder pattern.
    pub fn with_buddy(mut self, buddy: ObjectId) -> Self {
        self.buddy = Some(buddy);
        self
    }

    /// Activate this label's mnemonic.
    ///
    /// This method is typically called by the window or application when
    /// Alt+mnemonic is pressed. It:
    /// 1. Emits the `mnemonic_activated` signal
    /// 2. Returns the buddy widget ID for focus transfer (if set)
    ///
    /// Returns `Some(buddy_id)` if a buddy is set and should receive focus,
    /// `None` otherwise.
    ///
    /// # Usage
    ///
    /// Applications should call this when handling Alt+key events and the
    /// key matches the label's mnemonic character.
    pub fn activate_mnemonic(&self) -> Option<ObjectId> {
        if self.mnemonic_char.is_some() {
            self.mnemonic_activated.emit(());
            self.buddy
        } else {
            None
        }
    }

    /// Check if a given key matches this label's mnemonic.
    ///
    /// This compares the key (converted to lowercase) with the mnemonic character.
    /// Supports unicode characters.
    pub fn matches_mnemonic_key(&self, key: char) -> bool {
        match (self.mnemonic_char, key.to_lowercase().next()) {
            (Some(m), Some(k)) => m == k,
            _ => false,
        }
    }

    /// Get the horizontal alignment.
    pub fn horizontal_align(&self) -> HorizontalAlign {
        self.horizontal_align
    }

    /// Set the horizontal text alignment.
    pub fn set_horizontal_align(&mut self, align: HorizontalAlign) {
        if self.horizontal_align != align {
            self.horizontal_align = align;
            self.invalidate_layout();
            self.base.update();
        }
    }

    /// Set horizontal alignment using builder pattern.
    pub fn with_horizontal_align(mut self, align: HorizontalAlign) -> Self {
        self.horizontal_align = align;
        self.invalidate_layout();
        self
    }

    /// Get the vertical alignment.
    pub fn vertical_align(&self) -> VerticalAlign {
        self.vertical_align
    }

    /// Set the vertical text alignment.
    pub fn set_vertical_align(&mut self, align: VerticalAlign) {
        if self.vertical_align != align {
            self.vertical_align = align;
            self.base.update();
        }
    }

    /// Set vertical alignment using builder pattern.
    pub fn with_vertical_align(mut self, align: VerticalAlign) -> Self {
        self.vertical_align = align;
        self
    }

    /// Check if word wrapping is enabled.
    pub fn word_wrap(&self) -> bool {
        self.word_wrap
    }

    /// Enable or disable word wrapping.
    ///
    /// When word wrapping is enabled:
    /// - Text will wrap at word boundaries when it exceeds the widget width
    /// - The label supports height-for-width layout negotiation
    /// - Elision is automatically disabled (mutually exclusive)
    pub fn set_word_wrap(&mut self, wrap: bool) {
        if self.word_wrap != wrap {
            self.word_wrap = wrap;
            if wrap {
                // Word wrap and elision are mutually exclusive
                self.elide_mode = ElideMode::None;
            }
            self.invalidate_layout();
            self.update_size_policy();
            self.base.update();
        }
    }

    /// Set word wrapping using builder pattern.
    pub fn with_word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
        if wrap {
            self.elide_mode = ElideMode::None;
        }
        self.invalidate_layout();
        self.update_size_policy();
        self
    }

    /// Get the elide mode.
    pub fn elide_mode(&self) -> ElideMode {
        self.elide_mode
    }

    /// Set the text elision mode.
    ///
    /// Elision truncates text that doesn't fit with an ellipsis.
    /// When elision is enabled, word wrapping is automatically disabled.
    pub fn set_elide_mode(&mut self, mode: ElideMode) {
        if self.elide_mode != mode {
            self.elide_mode = mode;
            if mode != ElideMode::None {
                // Elision and word wrap are mutually exclusive
                self.word_wrap = false;
            }
            self.invalidate_layout();
            self.update_size_policy();
            self.base.update();
        }
    }

    /// Set elide mode using builder pattern.
    pub fn with_elide_mode(mut self, mode: ElideMode) -> Self {
        self.elide_mode = mode;
        if mode != ElideMode::None {
            self.word_wrap = false;
        }
        self.invalidate_layout();
        self.update_size_policy();
        self
    }

    /// Get the font.
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// Set the font for text rendering.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
        self.invalidate_layout();
        self.base.update();
    }

    /// Set the font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self.invalidate_layout();
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

    // =========================================================================
    // Selection API
    // =========================================================================

    /// Check if text selection is enabled.
    pub fn is_selectable(&self) -> bool {
        self.selectable
    }

    /// Enable or disable text selection.
    ///
    /// When enabled, the user can:
    /// - Click to position the cursor
    /// - Click and drag to select text
    /// - Double-click to select a word
    /// - Triple-click to select a line
    /// - Use keyboard shortcuts (Shift+arrows, Ctrl+A, Ctrl+C)
    ///
    /// When selection is enabled, the label becomes focusable.
    pub fn set_selectable(&mut self, selectable: bool) {
        if self.selectable != selectable {
            self.selectable = selectable;
            // Update focus policy based on selectability
            if selectable {
                self.base.set_focus_policy(FocusPolicy::StrongFocus);
            } else {
                self.base.set_focus_policy(FocusPolicy::NoFocus);
                // Clear selection when disabling
                self.clear_selection();
            }
            self.base.update();
        }
    }

    /// Set selectable using builder pattern.
    pub fn with_selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        if selectable {
            self.base.set_focus_policy(FocusPolicy::StrongFocus);
        } else {
            self.base.set_focus_policy(FocusPolicy::NoFocus);
        }
        self
    }

    /// Get the selection background color.
    pub fn selection_color(&self) -> Color {
        self.selection_color
    }

    /// Set the selection background color.
    pub fn set_selection_color(&mut self, color: Color) {
        if self.selection_color != color {
            self.selection_color = color;
            if self.has_selection() {
                self.base.update();
            }
        }
    }

    /// Set selection color using builder pattern.
    pub fn with_selection_color(mut self, color: Color) -> Self {
        self.selection_color = color;
        self
    }

    /// Check if there is any text selected.
    pub fn has_selection(&self) -> bool {
        if let Some(anchor) = self.selection_anchor {
            anchor != self.cursor_pos
        } else {
            false
        }
    }

    /// Get the selection range as (start, end) byte offsets.
    ///
    /// Returns None if no text is selected.
    /// The returned range is always ordered (start < end).
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_anchor.map(|anchor| {
            if anchor < self.cursor_pos {
                (anchor, self.cursor_pos)
            } else {
                (self.cursor_pos, anchor)
            }
        })
    }

    /// Get the currently selected text.
    ///
    /// Returns an empty string if no text is selected.
    pub fn selected_text(&self) -> &str {
        if let Some((start, end)) = self.selection_range() {
            &self.text[start..end]
        } else {
            ""
        }
    }

    /// Select all text in the label.
    pub fn select_all(&mut self) {
        if !self.selectable || self.text.is_empty() {
            return;
        }
        self.selection_anchor = Some(0);
        self.cursor_pos = self.text.len();
        self.base.update();
        self.selection_changed.emit(());
        self.copy_available.emit(true);
    }

    /// Clear the current selection.
    pub fn deselect(&mut self) {
        self.clear_selection();
    }

    /// Clear selection without emitting signals (internal use).
    fn clear_selection(&mut self) {
        if self.selection_anchor.is_some() {
            self.selection_anchor = None;
            self.base.update();
            self.selection_changed.emit(());
            self.copy_available.emit(false);
        }
    }

    /// Set selection to a specific range.
    ///
    /// The cursor will be positioned at `end`.
    pub fn set_selection(&mut self, start: usize, end: usize) {
        if !self.selectable {
            return;
        }
        let start = start.min(self.text.len());
        let end = end.min(self.text.len());

        self.selection_anchor = Some(start);
        self.cursor_pos = end;
        self.base.update();
        self.selection_changed.emit(());
        self.copy_available.emit(start != end);
    }

    /// Get the current cursor position (byte offset).
    pub fn cursor_position(&self) -> usize {
        self.cursor_pos
    }

    /// Copy selected text to clipboard.
    ///
    /// Returns true if text was copied, false if no selection or clipboard error.
    pub fn copy_selection(&self) -> bool {
        if !self.has_selection() {
            return false;
        }

        let text = self.selected_text().to_string();
        if text.is_empty() {
            return false;
        }

        match arboard::Clipboard::new() {
            Ok(mut clipboard) => clipboard.set_text(text).is_ok(),
            Err(_) => false,
        }
    }

    // =========================================================================
    // Link API
    // =========================================================================

    /// Check if this label contains any hyperlinks.
    ///
    /// Returns `true` if the label has rich text with `<a href="...">` tags.
    pub fn has_links(&self) -> bool {
        self.rich_text.as_ref().map_or(false, |rt| rt.has_links())
    }

    /// Get the URL of the link currently under the mouse cursor.
    ///
    /// Returns `None` if the mouse is not over a link or the label has no links.
    pub fn hovered_link(&self) -> Option<&str> {
        self.current_hovered_link.as_deref()
    }

    /// Find the link URL at a given byte offset in the text.
    ///
    /// This is useful for determining which link (if any) is at a specific
    /// text position, such as when converting mouse coordinates to text offsets.
    ///
    /// Returns `None` if there is no link at the given offset.
    pub fn link_at_offset(&self, offset: usize) -> Option<&str> {
        self.rich_text.as_ref()?.link_at_offset(offset)
    }

    // =========================================================================
    // Selection Event Handlers (Internal)
    // =========================================================================

    /// Handle mouse press for selection.
    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if !self.selectable || event.button != MouseButton::Left {
            return false;
        }

        // Track click count for double/triple click detection
        self.click_count = self.click_count.saturating_add(1);

        let mut font_system = FontSystem::new();
        let layout = self.ensure_layout(&mut font_system, None, false);

        // Convert click position to text offset
        let offset = layout.offset_at_point(event.local_pos.x, event.local_pos.y);
        let offset = offset.min(self.text.len());

        if event.modifiers.shift {
            // Extend selection from current anchor
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.cursor_pos);
            }
        } else {
            // Start new selection
            self.selection_anchor = Some(offset);
        }

        self.cursor_pos = offset;
        self.is_selecting = true;
        self.base.update();
        self.selection_changed.emit(());
        self.copy_available.emit(self.has_selection());

        true
    }

    /// Handle mouse release for selection.
    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if !self.selectable || event.button != MouseButton::Left {
            return false;
        }

        self.is_selecting = false;

        // Clear selection if it's empty (single click)
        if let Some(anchor) = self.selection_anchor {
            if anchor == self.cursor_pos {
                self.selection_anchor = None;
                self.selection_changed.emit(());
                self.copy_available.emit(false);
            }
        }

        true
    }

    /// Handle mouse move for drag selection.
    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        if !self.selectable || !self.is_selecting {
            return false;
        }

        let mut font_system = FontSystem::new();
        let layout = self.ensure_layout(&mut font_system, None, false);

        let offset = layout.offset_at_point(event.local_pos.x, event.local_pos.y);
        let offset = offset.min(self.text.len());

        if self.cursor_pos != offset {
            self.cursor_pos = offset;
            self.base.update();
            self.selection_changed.emit(());
            self.copy_available.emit(self.has_selection());
        }

        true
    }

    /// Handle double-click for word selection.
    fn handle_double_click(&mut self, event: &MouseDoubleClickEvent) -> bool {
        if !self.selectable || event.button != MouseButton::Left {
            return false;
        }

        let mut font_system = FontSystem::new();
        let layout = self.ensure_layout(&mut font_system, None, false);

        let offset = layout.offset_at_point(event.local_pos.x, event.local_pos.y);
        let offset = offset.min(self.text.len());

        // Select the word at the click position
        let word_range = layout.word_at_offset(offset);
        if !word_range.is_empty() {
            self.selection_anchor = Some(word_range.start);
            self.cursor_pos = word_range.end;
            self.base.update();
            self.selection_changed.emit(());
            self.copy_available.emit(true);
        }

        self.click_count = 2;
        true
    }

    /// Handle key press for selection navigation and copy.
    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        if !self.selectable {
            return false;
        }

        let shift = event.modifiers.shift;
        let ctrl = event.modifiers.control || event.modifiers.meta;

        match event.key {
            // Select all
            Key::A if ctrl => {
                self.select_all();
                true
            }
            // Copy
            Key::C if ctrl => {
                self.copy_selection();
                true
            }
            // Navigation with optional selection extension
            Key::ArrowLeft => {
                self.move_cursor_left(shift, ctrl);
                true
            }
            Key::ArrowRight => {
                self.move_cursor_right(shift, ctrl);
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
                self.move_cursor_to_start(shift, ctrl);
                true
            }
            Key::End => {
                self.move_cursor_to_end(shift, ctrl);
                true
            }
            _ => false,
        }
    }

    /// Handle focus gained.
    fn handle_focus_in(&mut self) {
        self.base.update();
    }

    /// Handle focus lost.
    fn handle_focus_out(&mut self) {
        self.is_selecting = false;
        self.click_count = 0;
        // Optionally clear selection on focus loss - for now keep it
        self.base.update();
    }

    // =========================================================================
    // Link Event Handlers (Internal)
    // =========================================================================

    /// Update the hovered link state based on mouse position.
    ///
    /// This is called during mouse move events to detect when the cursor
    /// enters or leaves a hyperlink. Emits `link_hovered` signal when the
    /// hovered link changes.
    fn update_hovered_link(&mut self, x: f32, y: f32) {
        if !self.has_links() {
            // No links, clear any stale hover state
            if self.current_hovered_link.is_some() {
                self.current_hovered_link = None;
                self.link_hovered.emit(None);
                // Reset cursor when leaving links
                self.base.unset_cursor();
            }
            return;
        }

        let mut font_system = FontSystem::new();
        let layout = self.ensure_layout(&mut font_system, None, false);

        let offset = layout.offset_at_point(x, y);
        let offset = offset.min(self.text.len());

        // Check if there's a link at this offset
        let new_link = self.link_at_offset(offset).map(|s| s.to_string());

        // Only emit signal and update cursor if the hovered link changed
        if new_link != self.current_hovered_link {
            let was_over_link = self.current_hovered_link.is_some();
            let now_over_link = new_link.is_some();

            self.current_hovered_link = new_link.clone();
            self.link_hovered.emit(new_link);

            // Update cursor based on link hover state
            if now_over_link && !was_over_link {
                // Entered a link - show hand cursor
                self.base.set_cursor(CursorShape::Hand);
            } else if !now_over_link && was_over_link {
                // Left a link - reset to default cursor
                self.base.unset_cursor();
            }

            self.base.update(); // Redraw for potential cursor change
        }
    }

    /// Handle mouse press for link click detection.
    ///
    /// Returns true if the press is on a link (to track for click completion).
    fn handle_link_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left || !self.has_links() {
            self.link_click_pending = None;
            return false;
        }

        let mut font_system = FontSystem::new();
        let layout = self.ensure_layout(&mut font_system, None, false);

        let offset = layout.offset_at_point(event.local_pos.x, event.local_pos.y);
        let offset = offset.min(self.text.len());

        // Track which link (if any) was pressed
        self.link_click_pending = self.link_at_offset(offset).map(|s| s.to_string());
        self.link_click_pending.is_some()
    }

    /// Handle mouse release for link click completion.
    ///
    /// Emits `link_activated` if the release is on the same link that was pressed.
    fn handle_link_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pending_link = self.link_click_pending.take();

        if let Some(pressed_link) = pending_link {
            // Check if we're still on the same link
            if let Some(current_link) = &self.current_hovered_link {
                if current_link == &pressed_link {
                    // Complete click - emit link_activated
                    self.link_activated.emit(pressed_link);
                    return true;
                }
            }
        }

        false
    }

    // =========================================================================
    // Cursor Movement (Internal)
    // =========================================================================

    /// Move cursor left by one grapheme (or word if ctrl).
    fn move_cursor_left(&mut self, extend_selection: bool, word: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection && self.has_selection() {
            // Move to start of selection and clear
            if let Some((start, _)) = self.selection_range() {
                self.cursor_pos = start;
            }
            self.selection_anchor = None;
            self.base.update();
            self.selection_changed.emit(());
            self.copy_available.emit(false);
            return;
        }

        if self.cursor_pos > 0 {
            let mut font_system = FontSystem::new();
            let layout = self.ensure_layout(&mut font_system, None, false);

            self.cursor_pos = if word {
                layout.move_cursor_word_left(self.cursor_pos)
            } else {
                layout.move_cursor_left(self.cursor_pos)
            };
            self.base.update();

            if extend_selection {
                self.selection_changed.emit(());
                self.copy_available.emit(self.has_selection());
            }
        }
    }

    /// Move cursor right by one grapheme (or word if ctrl).
    fn move_cursor_right(&mut self, extend_selection: bool, word: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection && self.has_selection() {
            // Move to end of selection and clear
            if let Some((_, end)) = self.selection_range() {
                self.cursor_pos = end;
            }
            self.selection_anchor = None;
            self.base.update();
            self.selection_changed.emit(());
            self.copy_available.emit(false);
            return;
        }

        if self.cursor_pos < self.text.len() {
            let mut font_system = FontSystem::new();
            let layout = self.ensure_layout(&mut font_system, None, false);

            self.cursor_pos = if word {
                layout.move_cursor_word_right(self.cursor_pos)
            } else {
                layout.move_cursor_right(self.cursor_pos)
            };
            self.base.update();

            if extend_selection {
                self.selection_changed.emit(());
                self.copy_available.emit(self.has_selection());
            }
        }
    }

    /// Move cursor up by one line.
    fn move_cursor_up(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let mut font_system = FontSystem::new();
        let layout = self.ensure_layout(&mut font_system, None, false);

        let preferred_x = layout.cursor_x_at_offset(self.cursor_pos);
        let new_pos = layout.move_cursor_up(self.cursor_pos, preferred_x);

        if new_pos != self.cursor_pos {
            self.cursor_pos = new_pos;
            self.base.update();
            if extend_selection {
                self.selection_changed.emit(());
                self.copy_available.emit(self.has_selection());
            }
        }
    }

    /// Move cursor down by one line.
    fn move_cursor_down(&mut self, extend_selection: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let mut font_system = FontSystem::new();
        let layout = self.ensure_layout(&mut font_system, None, false);

        let preferred_x = layout.cursor_x_at_offset(self.cursor_pos);
        let new_pos = layout.move_cursor_down(self.cursor_pos, preferred_x);

        if new_pos != self.cursor_pos {
            self.cursor_pos = new_pos;
            self.base.update();
            if extend_selection {
                self.selection_changed.emit(());
                self.copy_available.emit(self.has_selection());
            }
        }
    }

    /// Move cursor to start of line (or document if ctrl).
    fn move_cursor_to_start(&mut self, extend_selection: bool, document: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let new_pos = if document {
            0
        } else {
            let mut font_system = FontSystem::new();
            let layout = self.ensure_layout(&mut font_system, None, false);
            layout.move_cursor_to_line_start(self.cursor_pos)
        };

        if new_pos != self.cursor_pos {
            self.cursor_pos = new_pos;
            self.base.update();
            if extend_selection {
                self.selection_changed.emit(());
                self.copy_available.emit(self.has_selection());
            }
        }
    }

    /// Move cursor to end of line (or document if ctrl).
    fn move_cursor_to_end(&mut self, extend_selection: bool, document: bool) {
        if extend_selection && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor_pos);
        } else if !extend_selection {
            self.selection_anchor = None;
        }

        let new_pos = if document {
            self.text.len()
        } else {
            let mut font_system = FontSystem::new();
            let layout = self.ensure_layout(&mut font_system, None, false);
            layout.move_cursor_to_line_end(self.cursor_pos)
        };

        if new_pos != self.cursor_pos {
            self.cursor_pos = new_pos;
            self.base.update();
            if extend_selection {
                self.selection_changed.emit(());
                self.copy_available.emit(self.has_selection());
            }
        }
    }

    /// Invalidate the cached text layout.
    fn invalidate_layout(&self) {
        *self.cached_layout.write() = None;
    }

    /// Update size policy based on current settings.
    fn update_size_policy(&mut self) {
        let policy = if self.word_wrap {
            SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred)
                .with_height_for_width()
        } else {
            SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Fixed)
        };
        self.base.set_size_policy(policy);
    }

    /// Create rich text with the mnemonic character underlined.
    ///
    /// This splits the text into three spans:
    /// 1. Text before the mnemonic (plain)
    /// 2. The mnemonic character (underlined)
    /// 3. Text after the mnemonic (plain)
    fn create_mnemonic_rich_text(&self, text: &str, mnemonic_byte_pos: usize) -> RichText {
        let mut rich = RichText::new();

        // Find the character at mnemonic_byte_pos and its byte length
        if let Some(mnemonic_char) = text[mnemonic_byte_pos..].chars().next() {
            let mnemonic_len = mnemonic_char.len_utf8();

            // Text before mnemonic
            if mnemonic_byte_pos > 0 {
                let before = &text[..mnemonic_byte_pos];
                rich.push(RichTextSpan::new(before));
            }

            // Mnemonic character (underlined)
            let mut mnemonic_span = RichTextSpan::new(mnemonic_char.to_string());
            mnemonic_span.underline = true;
            rich.push(mnemonic_span);

            // Text after mnemonic
            let after_pos = mnemonic_byte_pos + mnemonic_len;
            if after_pos < text.len() {
                let after = &text[after_pos..];
                rich.push(RichTextSpan::new(after));
            }
        } else {
            // Fallback: just use plain text
            rich.push(RichTextSpan::new(text));
        }

        rich
    }

    /// Build layout options based on current label settings.
    fn build_layout_options(&self, width_constraint: Option<f32>) -> TextLayoutOptions {
        let mut options = TextLayoutOptions::new()
            .horizontal_align(self.horizontal_align);

        // Set width constraint
        if let Some(width) = width_constraint {
            options = options.max_width(width);
        }

        // Set wrap mode
        if self.word_wrap {
            options = options.wrap(WrapMode::Word);
        }

        // Set elision
        if self.elide_mode != ElideMode::None && width_constraint.is_some() {
            options = options.with_ellipsis();
        }

        options
    }

    /// Get or create the text layout.
    ///
    /// The layout is cached and only recalculated when necessary.
    ///
    /// # Arguments
    ///
    /// * `font_system` - The font system for text layout.
    /// * `width_constraint` - Optional width constraint for wrapping/elision.
    /// * `show_mnemonic_underline` - Whether to show the mnemonic underline (typically when Alt is held).
    fn ensure_layout(
        &self,
        font_system: &mut FontSystem,
        width_constraint: Option<f32>,
        show_mnemonic_underline: bool,
    ) -> TextLayout {
        let mut cached = self.cached_layout.write();

        // Check if we can reuse the cached layout
        if let Some(ref cache) = *cached {
            if cache.width_constraint == width_constraint
                && cache.show_mnemonic_underline == show_mnemonic_underline
            {
                return cache.layout.clone();
            }
        }

        // Build new layout
        let options = self.build_layout_options(width_constraint);

        let layout = if let Some(ref rich) = self.rich_text {
            // Rich text rendering
            // Note: Elision is not currently supported for rich text
            if self.elide_mode != ElideMode::None && width_constraint.is_some() {
                // Fall back to plain text for elision
                let display_text = self.compute_elided_text(font_system, width_constraint.unwrap());
                TextLayout::with_options(font_system, &display_text, &self.font, options)
            } else {
                // Use rich text layout
                let spans = rich.to_spans(&self.font);
                TextLayout::rich_text(font_system, &spans, &self.font, options)
            }
        } else if show_mnemonic_underline {
            // Show mnemonic underline (Alt is held)
            if let Some(mnemonic_pos) = self.mnemonic_byte_pos {
                // Plain text with mnemonic - create rich text to underline the mnemonic char
                let display_text = if self.elide_mode != ElideMode::None && width_constraint.is_some() {
                    self.compute_elided_text(font_system, width_constraint.unwrap())
                } else {
                    self.text.clone()
                };

                // Only apply mnemonic underlining if the position is still valid
                if mnemonic_pos < display_text.len() {
                    let mnemonic_rich = self.create_mnemonic_rich_text(&display_text, mnemonic_pos);
                    let spans = mnemonic_rich.to_spans(&self.font);
                    TextLayout::rich_text(font_system, &spans, &self.font, options)
                } else {
                    // Mnemonic position invalid (e.g., text was elided), render as plain text
                    TextLayout::with_options(font_system, &display_text, &self.font, options)
                }
            } else {
                // No mnemonic, render as plain text
                let display_text = if self.elide_mode != ElideMode::None && width_constraint.is_some() {
                    self.compute_elided_text(font_system, width_constraint.unwrap())
                } else {
                    self.text.clone()
                };
                TextLayout::with_options(font_system, &display_text, &self.font, options)
            }
        } else {
            // Plain text rendering without mnemonic underline (Alt not held)
            let display_text = if self.elide_mode != ElideMode::None && width_constraint.is_some() {
                self.compute_elided_text(font_system, width_constraint.unwrap())
            } else {
                self.text.clone()
            };
            TextLayout::with_options(font_system, &display_text, &self.font, options)
        };

        *cached = Some(CachedLayout {
            layout: layout.clone(),
            width_constraint,
            show_mnemonic_underline,
        });

        layout
    }

    /// Compute elided text that fits within the given width.
    fn compute_elided_text(&self, font_system: &mut FontSystem, max_width: f32) -> String {
        // First check if elision is needed
        let full_layout = TextLayout::with_options(
            font_system,
            &self.text,
            &self.font,
            TextLayoutOptions::new(),
        );

        if full_layout.width() <= max_width {
            return self.text.clone();
        }

        let ellipsis = "…";

        // Measure ellipsis width
        let ellipsis_layout = TextLayout::with_options(
            font_system,
            ellipsis,
            &self.font,
            TextLayoutOptions::new(),
        );
        let ellipsis_width = ellipsis_layout.width();

        if ellipsis_width >= max_width {
            return ellipsis.to_string();
        }

        let available_width = max_width - ellipsis_width;

        match self.elide_mode {
            ElideMode::None => self.text.clone(),
            ElideMode::Right => {
                self.elide_right(font_system, available_width, ellipsis)
            }
            ElideMode::Left => {
                self.elide_left(font_system, available_width, ellipsis)
            }
            ElideMode::Middle => {
                self.elide_middle(font_system, max_width, ellipsis)
            }
        }
    }

    /// Elide text from the right side.
    fn elide_right(&self, font_system: &mut FontSystem, available_width: f32, ellipsis: &str) -> String {
        let chars: Vec<char> = self.text.chars().collect();

        // Binary search for the right cutoff point
        let mut low = 0;
        let mut high = chars.len();

        while low < high {
            let mid = (low + high + 1) / 2;
            let test_text: String = chars[..mid].iter().collect();
            let test_layout = TextLayout::with_options(
                font_system,
                &test_text,
                &self.font,
                TextLayoutOptions::new(),
            );

            if test_layout.width() <= available_width {
                low = mid;
            } else {
                high = mid - 1;
            }
        }

        if low == 0 {
            ellipsis.to_string()
        } else {
            let truncated: String = chars[..low].iter().collect();
            format!("{}{}", truncated.trim_end(), ellipsis)
        }
    }

    /// Elide text from the left side.
    fn elide_left(&self, font_system: &mut FontSystem, available_width: f32, ellipsis: &str) -> String {
        let chars: Vec<char> = self.text.chars().collect();

        // Binary search for the right cutoff point from the end
        let mut low = 0;
        let mut high = chars.len();

        while low < high {
            let mid = (low + high + 1) / 2;
            let start = chars.len() - mid;
            let test_text: String = chars[start..].iter().collect();
            let test_layout = TextLayout::with_options(
                font_system,
                &test_text,
                &self.font,
                TextLayoutOptions::new(),
            );

            if test_layout.width() <= available_width {
                low = mid;
            } else {
                high = mid - 1;
            }
        }

        if low == 0 {
            ellipsis.to_string()
        } else {
            let start = chars.len() - low;
            let truncated: String = chars[start..].iter().collect();
            format!("{}{}", ellipsis, truncated.trim_start())
        }
    }

    /// Elide text from the middle.
    fn elide_middle(&self, font_system: &mut FontSystem, max_width: f32, ellipsis: &str) -> String {
        let chars: Vec<char> = self.text.chars().collect();
        if chars.is_empty() {
            return ellipsis.to_string();
        }

        // Measure ellipsis
        let ellipsis_layout = TextLayout::with_options(
            font_system,
            ellipsis,
            &self.font,
            TextLayoutOptions::new(),
        );
        let ellipsis_width = ellipsis_layout.width();
        let available_width = max_width - ellipsis_width;

        if available_width <= 0.0 {
            return ellipsis.to_string();
        }

        // Split available width roughly 50/50 between start and end
        let half_width = available_width / 2.0;

        // Find how many chars fit from the start
        let mut start_len = 0;
        for i in 1..=chars.len() {
            let test_text: String = chars[..i].iter().collect();
            let test_layout = TextLayout::with_options(
                font_system,
                &test_text,
                &self.font,
                TextLayoutOptions::new(),
            );
            if test_layout.width() > half_width {
                break;
            }
            start_len = i;
        }

        // Find how many chars fit from the end
        let mut end_len = 0;
        for i in 1..=chars.len() {
            let start_idx = chars.len() - i;
            let test_text: String = chars[start_idx..].iter().collect();
            let test_layout = TextLayout::with_options(
                font_system,
                &test_text,
                &self.font,
                TextLayoutOptions::new(),
            );
            if test_layout.width() > half_width {
                break;
            }
            end_len = i;
        }

        if start_len == 0 && end_len == 0 {
            ellipsis.to_string()
        } else {
            let start_part: String = chars[..start_len].iter().collect();
            let end_start = chars.len() - end_len;
            let end_part: String = chars[end_start..].iter().collect();
            format!("{}{}{}", start_part.trim_end(), ellipsis, end_part.trim_start())
        }
    }

    /// Calculate the size hint for unconstrained text.
    fn calculate_unconstrained_size(&self, font_system: &mut FontSystem) -> Size {
        let layout = TextLayout::with_options(
            font_system,
            &self.text,
            &self.font,
            TextLayoutOptions::new().horizontal_align(self.horizontal_align),
        );
        Size::new(layout.width(), layout.height())
    }
}

impl Object for Label {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Label {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Create a temporary font system to calculate size
        // In a real application, this would be passed in or cached globally
        let mut font_system = FontSystem::new();
        let size = self.calculate_unconstrained_size(&mut font_system);

        if self.word_wrap {
            // For word-wrapped text, we have a minimum width (longest word)
            // and no maximum width. Height will be determined by height_for_width.
            SizeHint::new(size)
                .with_minimum_dimensions(0.0, self.font.size())
        } else if self.elide_mode != ElideMode::None {
            // For elided text, we can shrink to just the ellipsis
            let ellipsis_layout = TextLayout::new(&mut font_system, "…", &self.font);
            SizeHint::new(size)
                .with_minimum_dimensions(ellipsis_layout.width(), size.height)
        } else {
            // Fixed text - preferred size equals natural size
            SizeHint::new(size)
        }
    }

    fn height_for_width(&self, width: f32) -> Option<f32> {
        if !self.word_wrap {
            return None;
        }

        let mut font_system = FontSystem::new();
        let options = TextLayoutOptions::new()
            .max_width(width)
            .wrap(WrapMode::Word)
            .horizontal_align(self.horizontal_align);

        let layout = TextLayout::with_options(&mut font_system, &self.text, &self.font, options);
        Some(layout.height())
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        if self.text.is_empty() {
            return;
        }

        let rect = ctx.rect();
        let width_constraint = if self.word_wrap || self.elide_mode != ElideMode::None {
            Some(rect.width())
        } else {
            None
        };

        // Get font system and text renderer (would normally be from application context)
        let mut font_system = FontSystem::new();

        // Determine if mnemonic underline should be shown (when Alt is held)
        let show_mnemonic_underline = ctx.is_alt_held() && self.mnemonic_byte_pos.is_some();

        // Build the layout
        let layout = self.ensure_layout(&mut font_system, width_constraint, show_mnemonic_underline);

        // Calculate vertical offset based on alignment
        let y_offset = match self.vertical_align {
            VerticalAlign::Top => 0.0,
            VerticalAlign::Middle => (rect.height() - layout.height()) / 2.0,
            VerticalAlign::Bottom => rect.height() - layout.height(),
        };

        // Calculate horizontal offset for non-wrapped single-line text
        let x_offset = if !self.word_wrap && layout.line_count() == 1 {
            match self.horizontal_align {
                HorizontalAlign::Left => 0.0,
                HorizontalAlign::Center => (rect.width() - layout.width()) / 2.0,
                HorizontalAlign::Right => rect.width() - layout.width(),
                HorizontalAlign::Justified => 0.0,
            }
        } else {
            0.0
        };

        let position = Point::new(rect.origin.x + x_offset, rect.origin.y + y_offset);

        // Draw selection background if we have a selection and label is selectable
        if self.selectable && self.has_selection() {
            if let Some((start, end)) = self.selection_range() {
                let selection_rects = layout.selection_rects(start, end);
                for sel_rect in selection_rects {
                    ctx.renderer().fill_rect(
                        Rect::new(
                            position.x + sel_rect.x,
                            position.y + sel_rect.y,
                            sel_rect.width,
                            sel_rect.height,
                        ),
                        self.selection_color,
                    );
                }
            }
        }

        // Create text renderer and prepare glyphs
        if let Ok(mut text_renderer) = TextRenderer::new() {
            if let Ok(prepared_glyphs) = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                position,
                self.text_color,
            ) {
                // In a full implementation, we would render the prepared glyphs
                // through the text render pass. For now, we draw background rectangles
                // for the text bounds to show the label area.
                let _glyphs = prepared_glyphs;

                // Note: Actual glyph rendering requires integration with the
                // application's render pass system. The prepared_glyphs would be
                // submitted to a TextRenderPass during the frame render.
            }
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle link events even when not selectable
        let has_links = self.has_links();

        match event {
            WidgetEvent::MousePress(e) => {
                let mut handled = false;

                // Track link press for click detection
                if has_links {
                    self.handle_link_press(e);
                    handled = true;
                }

                // Handle selection if selectable
                if self.selectable && self.handle_mouse_press(e) {
                    handled = true;
                }

                if handled {
                    event.accept();
                }
                handled
            }
            WidgetEvent::MouseRelease(e) => {
                let mut handled = false;

                // Check for link click completion (before selection handling)
                // Only activate link if we didn't drag to select
                if has_links && !self.has_selection() {
                    if self.handle_link_release(e) {
                        handled = true;
                    }
                } else {
                    // Clear pending link if we have a selection
                    self.link_click_pending = None;
                }

                // Handle selection if selectable
                if self.selectable && self.handle_mouse_release(e) {
                    handled = true;
                }

                if handled {
                    event.accept();
                }
                handled
            }
            WidgetEvent::MouseMove(e) => {
                let mut handled = false;

                // Always update link hover state when we have links
                if has_links {
                    self.update_hovered_link(e.local_pos.x, e.local_pos.y);
                    handled = true;
                }

                // Handle selection drag if selectable and selecting
                if self.selectable && self.handle_mouse_move(e) {
                    handled = true;
                }

                if handled {
                    event.accept();
                }
                handled
            }
            WidgetEvent::DoubleClick(e) => {
                // Double-click is only for selection
                if self.selectable && self.handle_double_click(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::KeyPress(e) => {
                // Key press is only for selection
                if self.selectable && self.handle_key_press(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::FocusIn(_) => {
                if self.selectable {
                    self.handle_focus_in();
                    true
                } else {
                    false
                }
            }
            WidgetEvent::FocusOut(_) => {
                if self.selectable {
                    self.handle_focus_out();
                }
                // Clear link hover state and cursor on focus out
                if self.current_hovered_link.is_some() {
                    self.current_hovered_link = None;
                    self.link_hovered.emit(None);
                    self.base.unset_cursor();
                }
                true
            }
            _ => false,
        }
    }

    fn matches_mnemonic_key(&self, key: char) -> bool {
        // Delegate to our existing method
        Label::matches_mnemonic_key(self, key)
    }

    fn activate_mnemonic(&self) -> Option<ObjectId> {
        // Delegate to our existing method
        Label::activate_mnemonic(self)
    }
}

// Ensure Label is Send + Sync
static_assertions::assert_impl_all!(Label: Send, Sync);

// =========================================================================
// Accessibility
// =========================================================================

#[cfg(feature = "accessibility")]
impl crate::widget::accessibility::Accessible for Label {
    fn accessible_role(&self) -> crate::widget::accessibility::AccessibleRole {
        // Check if the label contains a link
        if self.text().contains("href=") || self.text().contains("<a ") {
            crate::widget::accessibility::AccessibleRole::Link
        } else {
            crate::widget::accessibility::AccessibleRole::Label
        }
    }

    fn accessible_name(&self) -> Option<String> {
        // Use custom accessible name if set, otherwise use the label text
        self.widget_base()
            .accessible_name()
            .map(String::from)
            .or_else(|| {
                let text = self.text();
                if text.is_empty() {
                    None
                } else {
                    // Strip HTML tags for accessible name
                    Some(strip_html_tags_for_a11y(text))
                }
            })
    }

    fn accessible_description(&self) -> Option<String> {
        self.widget_base().accessible_description().map(String::from)
    }

    fn accessible_actions(&self) -> Vec<accesskit::Action> {
        // Labels with links should be focusable
        if self.text().contains("href=") || self.text().contains("<a ") {
            vec![accesskit::Action::Focus, accesskit::Action::Click]
        } else {
            Vec::new()
        }
    }
}

/// Strip HTML tags from text for accessibility purposes.
#[cfg(feature = "accessibility")]
fn strip_html_tags_for_a11y(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_tag = false;

    for c in text.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(c),
            _ => {}
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_label_creation() {
        setup();
        let label = Label::new("Hello, World!");
        assert_eq!(label.text(), "Hello, World!");
        assert_eq!(label.horizontal_align(), HorizontalAlign::Left);
        assert_eq!(label.vertical_align(), VerticalAlign::Top);
        assert!(!label.word_wrap());
        assert_eq!(label.elide_mode(), ElideMode::None);
    }

    #[test]
    fn test_label_builder_pattern() {
        setup();
        let label = Label::new("Test")
            .with_horizontal_align(HorizontalAlign::Center)
            .with_vertical_align(VerticalAlign::Middle)
            .with_word_wrap(true)
            .with_text_color(Color::RED);

        assert_eq!(label.horizontal_align(), HorizontalAlign::Center);
        assert_eq!(label.vertical_align(), VerticalAlign::Middle);
        assert!(label.word_wrap());
        assert_eq!(label.text_color(), Color::RED);
    }

    #[test]
    fn test_word_wrap_and_elide_mutually_exclusive() {
        setup();
        let mut label = Label::new("Test");

        // Enable word wrap
        label.set_word_wrap(true);
        assert!(label.word_wrap());
        assert_eq!(label.elide_mode(), ElideMode::None);

        // Enable elide - should disable word wrap
        label.set_elide_mode(ElideMode::Right);
        assert!(!label.word_wrap());
        assert_eq!(label.elide_mode(), ElideMode::Right);

        // Enable word wrap again - should disable elide
        label.set_word_wrap(true);
        assert!(label.word_wrap());
        assert_eq!(label.elide_mode(), ElideMode::None);
    }

    #[test]
    fn test_text_changed_signal() {
        setup();
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

        let mut label = Label::new("Initial");
        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_received_clone = signal_received.clone();

        label.text_changed.connect(move |_text| {
            signal_received_clone.store(true, Ordering::SeqCst);
        });

        label.set_text("Changed");
        assert!(signal_received.load(Ordering::SeqCst));
        assert_eq!(label.text(), "Changed");
    }

    #[test]
    fn test_elide_mode_variants() {
        setup();
        assert_eq!(ElideMode::default(), ElideMode::None);

        let label_left = Label::new("Test").with_elide_mode(ElideMode::Left);
        assert_eq!(label_left.elide_mode(), ElideMode::Left);

        let label_middle = Label::new("Test").with_elide_mode(ElideMode::Middle);
        assert_eq!(label_middle.elide_mode(), ElideMode::Middle);

        let label_right = Label::new("Test").with_elide_mode(ElideMode::Right);
        assert_eq!(label_right.elide_mode(), ElideMode::Right);
    }

    #[test]
    fn test_size_hint_basic() {
        setup();
        let label = Label::new("Hello");
        let hint = label.size_hint();

        // Should have a non-zero preferred size
        assert!(hint.preferred.width > 0.0);
        assert!(hint.preferred.height > 0.0);
    }

    #[test]
    fn test_height_for_width_without_wrap() {
        setup();
        let label = Label::new("Test");
        assert!(label.height_for_width(100.0).is_none());
    }

    #[test]
    fn test_height_for_width_with_wrap() {
        setup();
        let label = Label::new("This is a longer text that should wrap").with_word_wrap(true);
        let height = label.height_for_width(50.0);
        assert!(height.is_some());
        assert!(height.unwrap() > 0.0);
    }

    #[test]
    fn test_from_html() {
        setup();
        let label = Label::from_html("Hello <b>bold</b> world!");
        assert_eq!(label.text(), "Hello bold world!");
        assert!(label.has_rich_text());
    }

    #[test]
    fn test_with_html() {
        setup();
        let label = Label::new("placeholder").with_html("Hello <i>italic</i>!");
        assert_eq!(label.text(), "Hello italic!");
        assert!(label.has_rich_text());
    }

    #[test]
    fn test_set_html() {
        setup();
        let mut label = Label::new("Plain text");
        assert!(!label.has_rich_text());

        label.set_html("<b>Bold</b> text");
        assert_eq!(label.text(), "Bold text");
        assert!(label.has_rich_text());
    }

    #[test]
    fn test_set_text_clears_rich_text() {
        setup();
        let mut label = Label::from_html("<b>Bold</b>");
        assert!(label.has_rich_text());

        label.set_text("Plain text");
        assert_eq!(label.text(), "Plain text");
        assert!(!label.has_rich_text());
    }

    #[test]
    fn test_rich_text_access() {
        setup();
        let label = Label::from_html("<b>Bold</b> and <i>italic</i>");
        let rich = label.rich_text().expect("should have rich text");
        // spans: "Bold" (bold), " and ", "italic" (italic)
        assert_eq!(rich.spans().len(), 3);
    }

    #[test]
    fn test_complex_html() {
        setup();
        let label = Label::from_html(
            "<b>Bold</b> <i>italic</i> <u>underline</u> <s>strikethrough</s>"
        );
        assert_eq!(label.text(), "Bold italic underline strikethrough");
        assert!(label.has_rich_text());

        let rich = label.rich_text().unwrap();
        // "Bold" (bold), " ", "italic" (italic), " ", "underline" (underline), " ", "strikethrough" (strikethrough)
        assert!(rich.spans()[0].bold);
        assert!(rich.spans()[2].italic);
        assert!(rich.spans()[4].underline);
        assert!(rich.spans()[6].strikethrough);
    }

    #[test]
    fn test_html_with_color() {
        setup();
        let label = Label::from_html("<font color=\"red\">Red text</font>");
        let rich = label.rich_text().unwrap();
        assert_eq!(rich.spans()[0].color, Some([255, 0, 0, 255]));
    }

    #[test]
    fn test_html_with_line_breaks() {
        setup();
        let label = Label::from_html("Line 1<br>Line 2<br/>Line 3");
        assert_eq!(label.text(), "Line 1\nLine 2\nLine 3");
    }

    // =========================================================================
    // Mnemonic Tests
    // =========================================================================

    #[test]
    fn test_mnemonic_parsing_simple() {
        setup();
        let label = Label::new("&Name");
        assert_eq!(label.text(), "Name");
        assert_eq!(label.mnemonic(), Some('n'));
        assert!(label.has_mnemonic());
    }

    #[test]
    fn test_mnemonic_parsing_middle() {
        setup();
        let label = Label::new("Fi&le");
        assert_eq!(label.text(), "File");
        assert_eq!(label.mnemonic(), Some('l'));
    }

    #[test]
    fn test_mnemonic_parsing_escaped_ampersand() {
        setup();
        let label = Label::new("Save && Exit");
        assert_eq!(label.text(), "Save & Exit");
        assert_eq!(label.mnemonic(), None);
        assert!(!label.has_mnemonic());
    }

    #[test]
    fn test_mnemonic_parsing_mixed() {
        setup();
        let label = Label::new("&Save && Exit");
        assert_eq!(label.text(), "Save & Exit");
        assert_eq!(label.mnemonic(), Some('s'));
    }

    #[test]
    fn test_mnemonic_parsing_trailing_ampersand() {
        setup();
        let label = Label::new("Test&");
        assert_eq!(label.text(), "Test");
        assert_eq!(label.mnemonic(), None);
    }

    #[test]
    fn test_mnemonic_parsing_only_first_counted() {
        setup();
        // Only the first & should be treated as mnemonic
        let label = Label::new("&File &Edit");
        assert_eq!(label.text(), "File Edit");
        assert_eq!(label.mnemonic(), Some('f'));
    }

    #[test]
    fn test_mnemonic_with_unicode() {
        setup();
        let label = Label::new("&Ünïcödé");
        assert_eq!(label.text(), "Ünïcödé");
        assert_eq!(label.mnemonic(), Some('ü'));
    }

    #[test]
    fn test_set_text_updates_mnemonic() {
        setup();
        let mut label = Label::new("&First");
        assert_eq!(label.mnemonic(), Some('f'));

        label.set_text("&Second");
        assert_eq!(label.text(), "Second");
        assert_eq!(label.mnemonic(), Some('s'));
    }

    #[test]
    fn test_rich_text_clears_mnemonic() {
        setup();
        let mut label = Label::new("&Name");
        assert_eq!(label.mnemonic(), Some('n'));

        label.set_html("<b>Bold</b>");
        assert_eq!(label.mnemonic(), None);
    }

    #[test]
    fn test_buddy_widget() {
        setup();
        use horizon_lattice_core::Object;

        let mut label = Label::new("&Name:");
        assert_eq!(label.buddy(), None);

        // Use another label as a dummy buddy
        let buddy_label = Label::new("Buddy");
        let buddy_id = buddy_label.object_id();

        label.set_buddy(Some(buddy_id));
        assert_eq!(label.buddy(), Some(buddy_id));

        label.set_buddy(None);
        assert_eq!(label.buddy(), None);
    }

    #[test]
    fn test_with_buddy_builder() {
        setup();
        use horizon_lattice_core::Object;

        let buddy_label = Label::new("Buddy");
        let buddy_id = buddy_label.object_id();

        let label = Label::new("&Name:").with_buddy(buddy_id);
        assert_eq!(label.buddy(), Some(buddy_id));
    }

    #[test]
    fn test_matches_mnemonic_key() {
        setup();
        let label = Label::new("&Name");

        assert!(label.matches_mnemonic_key('n'));
        assert!(label.matches_mnemonic_key('N'));
        assert!(!label.matches_mnemonic_key('m'));
    }

    #[test]
    fn test_activate_mnemonic_without_buddy() {
        setup();
        let label = Label::new("&Name");

        // Should emit signal but return None (no buddy)
        let result = label.activate_mnemonic();
        assert_eq!(result, None);
    }

    #[test]
    fn test_activate_mnemonic_with_buddy() {
        setup();
        use horizon_lattice_core::Object;

        let buddy_label = Label::new("Buddy");
        let buddy_id = buddy_label.object_id();
        let label = Label::new("&Name").with_buddy(buddy_id);

        let result = label.activate_mnemonic();
        assert_eq!(result, Some(buddy_id));
    }

    #[test]
    fn test_activate_mnemonic_signal() {
        setup();
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

        let label = Label::new("&Name");
        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_received_clone = signal_received.clone();

        label.mnemonic_activated.connect(move |()| {
            signal_received_clone.store(true, Ordering::SeqCst);
        });

        label.activate_mnemonic();
        assert!(signal_received.load(Ordering::SeqCst));
    }

    #[test]
    fn test_mnemonic_no_signal_without_mnemonic() {
        setup();
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

        let label = Label::new("No mnemonic");
        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_received_clone = signal_received.clone();

        label.mnemonic_activated.connect(move |()| {
            signal_received_clone.store(true, Ordering::SeqCst);
        });

        let result = label.activate_mnemonic();
        assert_eq!(result, None);
        assert!(!signal_received.load(Ordering::SeqCst));
    }

    // =========================================================================
    // Selection Tests
    // =========================================================================

    #[test]
    fn test_label_not_selectable_by_default() {
        setup();
        let label = Label::new("Hello, World!");
        assert!(!label.is_selectable());
        assert!(!label.has_selection());
    }

    #[test]
    fn test_label_selectable_mode() {
        setup();
        let mut label = Label::new("Hello, World!");

        label.set_selectable(true);
        assert!(label.is_selectable());

        label.set_selectable(false);
        assert!(!label.is_selectable());
    }

    #[test]
    fn test_label_selectable_builder() {
        setup();
        let label = Label::new("Hello, World!").with_selectable(true);
        assert!(label.is_selectable());
    }

    #[test]
    fn test_label_select_all() {
        setup();
        let mut label = Label::new("Hello, World!").with_selectable(true);

        label.select_all();
        assert!(label.has_selection());
        assert_eq!(label.selected_text(), "Hello, World!");
        assert_eq!(label.selection_range(), Some((0, 13)));
    }

    #[test]
    fn test_label_select_all_not_selectable() {
        setup();
        let mut label = Label::new("Hello, World!");

        // Should not select when not selectable
        label.select_all();
        assert!(!label.has_selection());
    }

    #[test]
    fn test_label_deselect() {
        setup();
        let mut label = Label::new("Hello, World!").with_selectable(true);

        label.select_all();
        assert!(label.has_selection());

        label.deselect();
        assert!(!label.has_selection());
        assert_eq!(label.selected_text(), "");
    }

    #[test]
    fn test_label_set_selection() {
        setup();
        let mut label = Label::new("Hello, World!").with_selectable(true);

        label.set_selection(0, 5);
        assert!(label.has_selection());
        assert_eq!(label.selected_text(), "Hello");
        assert_eq!(label.selection_range(), Some((0, 5)));
    }

    #[test]
    fn test_label_set_selection_range_clamped() {
        setup();
        let mut label = Label::new("Hello").with_selectable(true);

        // Setting selection beyond text length should be clamped
        label.set_selection(0, 100);
        assert_eq!(label.selected_text(), "Hello");
        assert_eq!(label.selection_range(), Some((0, 5)));
    }

    #[test]
    fn test_label_selection_color() {
        setup();
        let mut label = Label::new("Hello").with_selectable(true);

        let custom_color = Color::from_rgba8(255, 0, 0, 128);
        label.set_selection_color(custom_color);
        assert_eq!(label.selection_color(), custom_color);
    }

    #[test]
    fn test_label_selection_color_builder() {
        setup();
        let custom_color = Color::from_rgba8(255, 0, 0, 128);
        let label = Label::new("Hello")
            .with_selectable(true)
            .with_selection_color(custom_color);
        assert_eq!(label.selection_color(), custom_color);
    }

    #[test]
    fn test_label_cursor_position() {
        setup();
        let mut label = Label::new("Hello, World!").with_selectable(true);

        // After set_selection, cursor should be at end position
        label.set_selection(0, 5);
        assert_eq!(label.cursor_position(), 5);
    }

    #[test]
    fn test_label_selection_changed_signal() {
        setup();
        use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

        let mut label = Label::new("Hello, World!").with_selectable(true);
        let signal_count = Arc::new(AtomicUsize::new(0));
        let signal_count_clone = signal_count.clone();

        label.selection_changed.connect(move |()| {
            signal_count_clone.fetch_add(1, Ordering::SeqCst);
        });

        label.select_all();
        assert_eq!(signal_count.load(Ordering::SeqCst), 1);

        label.deselect();
        assert_eq!(signal_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_label_copy_available_signal() {
        setup();
        use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

        let mut label = Label::new("Hello, World!").with_selectable(true);
        let copy_available = Arc::new(AtomicBool::new(false));
        let copy_available_clone = copy_available.clone();

        label.copy_available.connect(move |available| {
            copy_available_clone.store(*available, Ordering::SeqCst);
        });

        label.select_all();
        assert!(copy_available.load(Ordering::SeqCst));

        label.deselect();
        assert!(!copy_available.load(Ordering::SeqCst));
    }

    #[test]
    fn test_label_disabling_selectable_clears_selection() {
        setup();
        let mut label = Label::new("Hello, World!").with_selectable(true);

        label.select_all();
        assert!(label.has_selection());

        label.set_selectable(false);
        assert!(!label.has_selection());
    }

    // =========================================================================
    // Link Tests
    // =========================================================================

    #[test]
    fn test_label_has_links() {
        setup();
        let label_without_links = Label::new("Plain text");
        assert!(!label_without_links.has_links());

        let label_with_links = Label::from_html("Visit <a href=\"url\">here</a>!");
        assert!(label_with_links.has_links());
    }

    #[test]
    fn test_label_link_at_offset() {
        setup();
        let label = Label::from_html("Click <a href=\"https://example.com\">here</a>!");
        // "Click " is 6 bytes, "here" is at 6..10

        assert_eq!(label.link_at_offset(0), None); // In "Click "
        assert_eq!(label.link_at_offset(6), Some("https://example.com")); // Start of "here"
        assert_eq!(label.link_at_offset(9), Some("https://example.com")); // In "here"
        assert_eq!(label.link_at_offset(10), None); // In "!"
    }

    #[test]
    fn test_label_hovered_link_initially_none() {
        setup();
        let label = Label::from_html("Visit <a href=\"url\">here</a>!");
        assert_eq!(label.hovered_link(), None);
    }

    #[test]
    fn test_label_from_html_with_links() {
        setup();
        let label = Label::from_html("Click <a href=\"https://example.com\">here</a> to visit.");
        assert_eq!(label.text(), "Click here to visit.");
        assert!(label.has_links());
        assert!(label.has_rich_text());
    }

    #[test]
    fn test_label_multiple_links() {
        setup();
        let label = Label::from_html(
            "<a href=\"url1\">Link 1</a> and <a href=\"url2\">Link 2</a>"
        );
        assert_eq!(label.text(), "Link 1 and Link 2");
        assert!(label.has_links());

        // Check link at different positions
        assert_eq!(label.link_at_offset(0), Some("url1")); // In "Link 1"
        assert_eq!(label.link_at_offset(7), None); // In " and "
        assert_eq!(label.link_at_offset(12), Some("url2")); // In "Link 2"
    }

    #[test]
    fn test_label_link_activated_signal() {
        setup();
        use std::sync::{Arc, Mutex};

        let label = Label::from_html("<a href=\"https://example.com\">Click</a>");
        let activated_url = Arc::new(Mutex::new(None::<String>));
        let activated_url_clone = activated_url.clone();

        label.link_activated.connect(move |url| {
            *activated_url_clone.lock().unwrap() = Some(url.clone());
        });

        // Signal is emitted when link_activated.emit() is called
        // (In actual usage, this happens when a link is clicked)
        label.link_activated.emit("https://example.com".to_string());

        let result = activated_url.lock().unwrap();
        assert_eq!(*result, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_label_link_hovered_signal() {
        setup();
        use std::sync::{Arc, Mutex};

        let label = Label::from_html("<a href=\"url\">Link</a>");
        let hovered_url = Arc::new(Mutex::new(None::<Option<String>>));
        let hovered_url_clone = hovered_url.clone();

        label.link_hovered.connect(move |url| {
            *hovered_url_clone.lock().unwrap() = Some(url.clone());
        });

        // Signal is emitted when link_hovered.emit() is called
        label.link_hovered.emit(Some("url".to_string()));

        let result = hovered_url.lock().unwrap();
        assert_eq!(*result, Some(Some("url".to_string())));
    }

    #[test]
    fn test_label_plain_text_no_links() {
        setup();
        let label = Label::new("Plain text without links");
        assert!(!label.has_links());
        assert_eq!(label.link_at_offset(0), None);
    }

    #[test]
    fn test_label_link_with_formatting() {
        setup();
        let label = Label::from_html("<a href=\"url\"><b>Bold link</b></a>");
        assert_eq!(label.text(), "Bold link");
        assert!(label.has_links());
        assert_eq!(label.link_at_offset(0), Some("url"));
    }
}
