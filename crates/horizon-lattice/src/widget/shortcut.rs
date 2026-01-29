//! Keyboard shortcut system for Horizon Lattice.
//!
//! This module provides types and utilities for keyboard shortcuts:
//!
//! - [`KeySequence`]: Represents a keyboard shortcut (up to 4 key combinations)
//! - [`StandardKey`]: Platform-aware standard shortcuts (Copy, Paste, etc.)
//! - [`Shortcut`]: Standalone shortcut with context, enabled state, and signal
//! - [`ShortcutManager`]: Manages shortcuts and handles multi-key chord sequences
//! - Parsing utilities for human-readable strings like "Ctrl+S" or "Ctrl+K, Ctrl+C"
//! - Mnemonic extraction from "&Open" style text
//!
//! # Shortcuts
//!
//! Shortcuts are key combinations that trigger actions:
//!
//! ```ignore
//! use horizon_lattice::widget::{KeySequence, Key, KeyboardModifiers, StandardKey};
//!
//! // Create from key and modifiers
//! let shortcut = KeySequence::single(Key::S, KeyboardModifiers::CTRL);
//!
//! // Parse from string (supports multi-key chords)
//! let save = KeySequence::from_str("Ctrl+S").unwrap();
//! let chord = KeySequence::from_str("Ctrl+K, Ctrl+C").unwrap();  // Two-key chord
//!
//! // Use platform-appropriate standard keys
//! let copy = StandardKey::Copy.key_sequence();
//! ```
//!
//! # Multi-Key Sequences (Chords)
//!
//! Key sequences can contain up to 4 key combinations, separated by commas:
//!
//! ```ignore
//! // GNU Emacs style: "Ctrl+X, Ctrl+S" to save
//! let seq = KeySequence::from_str("Ctrl+X, Ctrl+S").unwrap();
//! assert_eq!(seq.count(), 2);
//!
//! // Match against incoming key presses
//! match seq.matches_partial(&pressed_keys) {
//!     SequenceMatch::ExactMatch => activate_shortcut(),
//!     SequenceMatch::PartialMatch => wait_for_next_key(),
//!     SequenceMatch::NoMatch => ignore(),
//! }
//! ```
//!
//! # Mnemonics
//!
//! Mnemonics are indicated by '&' in button text:
//!
//! ```ignore
//! // "&Open" -> mnemonic is 'o', displayed text is "Open" with underlined 'O'
//! // "Save &As" -> mnemonic is 'a', displayed text is "Save As" with underlined 'A'
//! // "Fish && Chips" -> no mnemonic, displayed text is "Fish & Chips"
//! ```

use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use horizon_lattice_core::{Object, ObjectBase, ObjectId, Signal};
use parking_lot::RwLock;

use crate::widget::events::{Key, KeyboardModifiers};
use crate::widget::widgets::ShortcutContext;

// =============================================================================
// Key Combination (Single Key + Modifiers)
// =============================================================================

/// A single key combination (one key with modifiers).
///
/// This represents a single chord like "Ctrl+S" or "Alt+F4".
/// For multi-key sequences like "Ctrl+K, Ctrl+C", see [`KeySequence`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyCombination {
    /// The primary key.
    pub key: Key,
    /// The modifier keys that must be held.
    pub modifiers: KeyboardModifiers,
}

impl KeyCombination {
    /// Create a new key combination from a key and modifiers.
    pub fn new(key: Key, modifiers: KeyboardModifiers) -> Self {
        Self { key, modifiers }
    }

    /// Create a key combination with no modifiers.
    pub fn key_only(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers::NONE,
        }
    }

    /// Create a Ctrl+key combination.
    pub fn ctrl(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers::CTRL,
        }
    }

    /// Create an Alt+key combination.
    pub fn alt(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers::ALT,
        }
    }

    /// Create a Shift+key combination.
    pub fn shift(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers::SHIFT,
        }
    }

    /// Create a Ctrl+Shift+key combination.
    pub fn ctrl_shift(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers::CTRL_SHIFT,
        }
    }

    /// Check if this key combination matches the given key and modifiers.
    pub fn matches(&self, key: Key, modifiers: KeyboardModifiers) -> bool {
        self.key == key
            && self.modifiers.control == modifiers.control
            && self.modifiers.alt == modifiers.alt
            && self.modifiers.shift == modifiers.shift
            && self.modifiers.meta == modifiers.meta
    }
}

impl fmt::Display for KeyCombination {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();

        if self.modifiers.control {
            parts.push("Ctrl");
        }
        if self.modifiers.alt {
            parts.push("Alt");
        }
        if self.modifiers.shift {
            parts.push("Shift");
        }
        if self.modifiers.meta {
            parts.push("Meta");
        }

        parts.push(key_to_string(self.key));

        write!(f, "{}", parts.join("+"))
    }
}

// =============================================================================
// Key Sequence (Up to 4 Key Combinations)
// =============================================================================

/// Maximum number of key combinations in a sequence.
pub const MAX_KEY_SEQUENCE_LENGTH: usize = 4;

/// A keyboard shortcut represented as a sequence of key combinations.
///
/// KeySequence can represent single shortcuts like "Ctrl+S" or multi-key
/// chords like "Ctrl+K, Ctrl+C" (up to 4 key combinations).
///
/// # String Format
///
/// When parsing from strings, multiple key combinations are separated by commas:
/// - Single: `"Ctrl+S"`, `"Alt+F4"`, `"F1"`
/// - Multi-key: `"Ctrl+K, Ctrl+C"`, `"Alt+X, Ctrl+S, Q"`
///
/// # Matching
///
/// Use [`matches_partial`](Self::matches_partial) to check if a sequence of
/// pressed keys matches this shortcut. This returns [`SequenceMatch::PartialMatch`]
/// when some keys match but more are expected (for chord tracking).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeySequence {
    /// The key combinations in this sequence (1-4).
    combinations: Vec<KeyCombination>,
}

impl KeySequence {
    /// Create a new key sequence from a single key and modifiers.
    ///
    /// This is a convenience method equivalent to `KeySequence::from_combination()`.
    pub fn new(key: Key, modifiers: KeyboardModifiers) -> Self {
        Self {
            combinations: vec![KeyCombination::new(key, modifiers)],
        }
    }

    /// Create a key sequence from a single key combination.
    pub fn single(key: Key, modifiers: KeyboardModifiers) -> Self {
        Self::new(key, modifiers)
    }

    /// Create a key sequence from a [`KeyCombination`].
    pub fn from_combination(combo: KeyCombination) -> Self {
        Self {
            combinations: vec![combo],
        }
    }

    /// Create a key sequence from multiple key combinations.
    ///
    /// # Panics
    ///
    /// Panics if `combos` is empty or has more than 4 elements.
    pub fn from_combinations(combos: Vec<KeyCombination>) -> Self {
        assert!(
            !combos.is_empty() && combos.len() <= MAX_KEY_SEQUENCE_LENGTH,
            "KeySequence must have 1-4 key combinations"
        );
        Self {
            combinations: combos,
        }
    }

    /// Create a key sequence with no modifiers.
    pub fn key_only(key: Key) -> Self {
        Self::from_combination(KeyCombination::key_only(key))
    }

    /// Create a Ctrl+key shortcut.
    pub fn ctrl(key: Key) -> Self {
        Self::from_combination(KeyCombination::ctrl(key))
    }

    /// Create an Alt+key shortcut.
    pub fn alt(key: Key) -> Self {
        Self::from_combination(KeyCombination::alt(key))
    }

    /// Create a Shift+key shortcut.
    pub fn shift(key: Key) -> Self {
        Self::from_combination(KeyCombination::shift(key))
    }

    /// Create a Ctrl+Shift+key shortcut.
    pub fn ctrl_shift(key: Key) -> Self {
        Self::from_combination(KeyCombination::ctrl_shift(key))
    }

    /// Get the number of key combinations in this sequence.
    pub fn count(&self) -> usize {
        self.combinations.len()
    }

    /// Check if this is a single-key sequence (not a chord).
    pub fn is_single(&self) -> bool {
        self.combinations.len() == 1
    }

    /// Get the key combinations in this sequence.
    pub fn combinations(&self) -> &[KeyCombination] {
        &self.combinations
    }

    /// Get the first key combination.
    pub fn first(&self) -> &KeyCombination {
        &self.combinations[0]
    }

    /// For backwards compatibility: get the key of the first combination.
    pub fn key(&self) -> Key {
        self.combinations[0].key
    }

    /// For backwards compatibility: get the modifiers of the first combination.
    pub fn modifiers(&self) -> KeyboardModifiers {
        self.combinations[0].modifiers
    }

    /// Check if this key sequence exactly matches the given key and modifiers.
    ///
    /// This only matches single-key sequences. For multi-key sequences,
    /// use [`matches_partial`](Self::matches_partial).
    pub fn matches(&self, key: Key, modifiers: KeyboardModifiers) -> bool {
        self.is_single() && self.combinations[0].matches(key, modifiers)
    }

    /// Check if a sequence of pressed keys matches this shortcut.
    ///
    /// Returns:
    /// - [`SequenceMatch::ExactMatch`] if all keys match exactly
    /// - [`SequenceMatch::PartialMatch`] if some keys match (waiting for more)
    /// - [`SequenceMatch::NoMatch`] if the sequences don't match
    pub fn matches_partial(&self, pressed: &[KeyCombination]) -> SequenceMatch {
        if pressed.is_empty() {
            return SequenceMatch::NoMatch;
        }

        if pressed.len() > self.combinations.len() {
            return SequenceMatch::NoMatch;
        }

        // Check if all pressed keys match
        for (i, combo) in pressed.iter().enumerate() {
            if !self.combinations[i].matches(combo.key, combo.modifiers) {
                return SequenceMatch::NoMatch;
            }
        }

        if pressed.len() == self.combinations.len() {
            SequenceMatch::ExactMatch
        } else {
            SequenceMatch::PartialMatch
        }
    }
}

impl fmt::Display for KeySequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let combo_strs: Vec<String> = self.combinations.iter().map(|c| c.to_string()).collect();
        write!(f, "{}", combo_strs.join(", "))
    }
}

// =============================================================================
// Sequence Match Result
// =============================================================================

/// Result of matching a key sequence against pressed keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SequenceMatch {
    /// The pressed keys exactly match the sequence.
    ExactMatch,
    /// The pressed keys are a valid prefix of the sequence (waiting for more keys).
    PartialMatch,
    /// The pressed keys do not match the sequence.
    NoMatch,
}

impl SequenceMatch {
    /// Check if this is an exact match.
    pub fn is_exact(self) -> bool {
        matches!(self, Self::ExactMatch)
    }

    /// Check if this is a partial match.
    pub fn is_partial(self) -> bool {
        matches!(self, Self::PartialMatch)
    }

    /// Check if there was any match (exact or partial).
    pub fn is_match(self) -> bool {
        !matches!(self, Self::NoMatch)
    }
}

/// Error type for parsing key sequences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySequenceParseError {
    /// The string is empty.
    Empty,
    /// No key was specified (only modifiers).
    NoKey,
    /// Unknown key name.
    UnknownKey(String),
    /// Too many key combinations (max 4).
    TooManyKeys,
}

impl fmt::Display for KeySequenceParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty key sequence"),
            Self::NoKey => write!(f, "no key specified (only modifiers)"),
            Self::UnknownKey(s) => write!(f, "unknown key: {}", s),
            Self::TooManyKeys => write!(f, "too many key combinations (max 4)"),
        }
    }
}

impl std::error::Error for KeySequenceParseError {}

/// Parse a single key combination from a string like "Ctrl+S".
fn parse_key_combination(s: &str) -> Result<KeyCombination, KeySequenceParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(KeySequenceParseError::Empty);
    }

    let mut modifiers = KeyboardModifiers::NONE;
    let mut key: Option<Key> = None;

    for part in s.split('+') {
        let part = part.trim();
        let part_lower = part.to_lowercase();

        match part_lower.as_str() {
            "ctrl" | "control" => modifiers.control = true,
            "alt" | "option" => modifiers.alt = true,
            "shift" => modifiers.shift = true,
            "meta" | "cmd" | "command" | "win" | "windows" | "super" => modifiers.meta = true,
            _ => {
                // This should be the key
                key = Some(parse_key(part)?);
            }
        }
    }

    match key {
        Some(k) => Ok(KeyCombination::new(k, modifiers)),
        None => Err(KeySequenceParseError::NoKey),
    }
}

impl FromStr for KeySequence {
    type Err = KeySequenceParseError;

    /// Parse a key sequence from a string.
    ///
    /// Supports single key combinations like "Ctrl+S" or multi-key chords
    /// like "Ctrl+K, Ctrl+C" (comma-separated, up to 4 combinations).
    ///
    /// # Format
    ///
    /// - Modifiers: `Ctrl`, `Alt`, `Shift`, `Meta` (or `Cmd` on macOS)
    /// - Keys: Letters (A-Z), digits (0-9), function keys (F1-F12), special keys
    /// - Multiple combinations are separated by commas
    ///
    /// # Examples
    ///
    /// - `"Ctrl+S"` - Single key
    /// - `"Ctrl+K, Ctrl+C"` - Two-key chord
    /// - `"Alt+X, Ctrl+S, Q"` - Three-key chord
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s.is_empty() {
            return Err(KeySequenceParseError::Empty);
        }

        // Split by comma to get individual key combinations
        let parts: Vec<&str> = s.split(',').collect();

        if parts.len() > MAX_KEY_SEQUENCE_LENGTH {
            return Err(KeySequenceParseError::TooManyKeys);
        }

        let mut combinations = Vec::with_capacity(parts.len());
        for part in parts {
            combinations.push(parse_key_combination(part)?);
        }

        Ok(KeySequence { combinations })
    }
}

/// Parse a key name to a Key enum value.
fn parse_key(s: &str) -> Result<Key, KeySequenceParseError> {
    let s_lower = s.to_lowercase();

    // Single character keys
    if s.len() == 1 {
        let ch = s.chars().next().unwrap().to_ascii_uppercase();
        return match ch {
            'A' => Ok(Key::A),
            'B' => Ok(Key::B),
            'C' => Ok(Key::C),
            'D' => Ok(Key::D),
            'E' => Ok(Key::E),
            'F' => Ok(Key::F),
            'G' => Ok(Key::G),
            'H' => Ok(Key::H),
            'I' => Ok(Key::I),
            'J' => Ok(Key::J),
            'K' => Ok(Key::K),
            'L' => Ok(Key::L),
            'M' => Ok(Key::M),
            'N' => Ok(Key::N),
            'O' => Ok(Key::O),
            'P' => Ok(Key::P),
            'Q' => Ok(Key::Q),
            'R' => Ok(Key::R),
            'S' => Ok(Key::S),
            'T' => Ok(Key::T),
            'U' => Ok(Key::U),
            'V' => Ok(Key::V),
            'W' => Ok(Key::W),
            'X' => Ok(Key::X),
            'Y' => Ok(Key::Y),
            'Z' => Ok(Key::Z),
            '0' => Ok(Key::Digit0),
            '1' => Ok(Key::Digit1),
            '2' => Ok(Key::Digit2),
            '3' => Ok(Key::Digit3),
            '4' => Ok(Key::Digit4),
            '5' => Ok(Key::Digit5),
            '6' => Ok(Key::Digit6),
            '7' => Ok(Key::Digit7),
            '8' => Ok(Key::Digit8),
            '9' => Ok(Key::Digit9),
            _ => Err(KeySequenceParseError::UnknownKey(s.to_string())),
        };
    }

    // Named keys
    match s_lower.as_str() {
        // Function keys
        "f1" => Ok(Key::F1),
        "f2" => Ok(Key::F2),
        "f3" => Ok(Key::F3),
        "f4" => Ok(Key::F4),
        "f5" => Ok(Key::F5),
        "f6" => Ok(Key::F6),
        "f7" => Ok(Key::F7),
        "f8" => Ok(Key::F8),
        "f9" => Ok(Key::F9),
        "f10" => Ok(Key::F10),
        "f11" => Ok(Key::F11),
        "f12" => Ok(Key::F12),

        // Navigation
        "up" | "arrowup" => Ok(Key::ArrowUp),
        "down" | "arrowdown" => Ok(Key::ArrowDown),
        "left" | "arrowleft" => Ok(Key::ArrowLeft),
        "right" | "arrowright" => Ok(Key::ArrowRight),
        "home" => Ok(Key::Home),
        "end" => Ok(Key::End),
        "pageup" | "pgup" => Ok(Key::PageUp),
        "pagedown" | "pgdn" => Ok(Key::PageDown),

        // Editing
        "backspace" | "back" => Ok(Key::Backspace),
        "delete" | "del" => Ok(Key::Delete),
        "insert" | "ins" => Ok(Key::Insert),
        "enter" | "return" => Ok(Key::Enter),
        "tab" => Ok(Key::Tab),
        "space" | "spacebar" => Ok(Key::Space),
        "escape" | "esc" => Ok(Key::Escape),

        // Punctuation
        "minus" | "-" => Ok(Key::Minus),
        "equal" | "equals" | "=" => Ok(Key::Equal),
        "bracketleft" | "[" => Ok(Key::BracketLeft),
        "bracketright" | "]" => Ok(Key::BracketRight),
        "backslash" | "\\" => Ok(Key::Backslash),
        "semicolon" | ";" => Ok(Key::Semicolon),
        "quote" | "'" => Ok(Key::Quote),
        "comma" | "," => Ok(Key::Comma),
        "period" | "." => Ok(Key::Period),
        "slash" | "/" => Ok(Key::Slash),
        "grave" | "`" => Ok(Key::Grave),

        _ => Err(KeySequenceParseError::UnknownKey(s.to_string())),
    }
}

/// Convert a Key to its string representation.
fn key_to_string(key: Key) -> &'static str {
    match key {
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",
        Key::Digit0 => "0",
        Key::Digit1 => "1",
        Key::Digit2 => "2",
        Key::Digit3 => "3",
        Key::Digit4 => "4",
        Key::Digit5 => "5",
        Key::Digit6 => "6",
        Key::Digit7 => "7",
        Key::Digit8 => "8",
        Key::Digit9 => "9",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::ArrowUp => "Up",
        Key::ArrowDown => "Down",
        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Backspace => "Backspace",
        Key::Delete => "Delete",
        Key::Insert => "Insert",
        Key::Enter => "Enter",
        Key::Tab => "Tab",
        Key::Space => "Space",
        Key::Escape => "Escape",
        Key::Minus => "-",
        Key::Equal => "=",
        Key::BracketLeft => "[",
        Key::BracketRight => "]",
        Key::Backslash => "\\",
        Key::Semicolon => ";",
        Key::Quote => "'",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Slash => "/",
        Key::Grave => "`",
        _ => "Unknown",
    }
}

// =============================================================================
// Standard Key Enum
// =============================================================================

/// Standard keyboard shortcuts with platform-appropriate defaults.
///
/// `StandardKey` provides a way to refer to common operations without
/// hardcoding the specific key combination, which may vary by platform
/// (e.g., Cmd+C on macOS vs Ctrl+C on Windows/Linux).
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::{StandardKey, KeySequence};
///
/// // Get platform-appropriate shortcut for Copy
/// let copy_shortcut = StandardKey::Copy.key_sequence();
///
/// // Create an action with standard shortcut
/// let copy_action = Action::new("&Copy")
///     .with_shortcut(StandardKey::Copy.key_sequence());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StandardKey {
    // =========================================================================
    // Document Operations
    // =========================================================================
    /// Create new document (Ctrl+N / Cmd+N).
    New,
    /// Open document (Ctrl+O / Cmd+O).
    Open,
    /// Save document (Ctrl+S / Cmd+S).
    Save,
    /// Save document with new name (Ctrl+Shift+S / Cmd+Shift+S).
    SaveAs,
    /// Close document/tab (Ctrl+W / Cmd+W).
    Close,
    /// Print document (Ctrl+P / Cmd+P).
    Print,
    /// Quit application (Ctrl+Q / Cmd+Q).
    Quit,

    // =========================================================================
    // Editing Operations
    // =========================================================================
    /// Undo (Ctrl+Z / Cmd+Z).
    Undo,
    /// Redo (Ctrl+Shift+Z or Ctrl+Y / Cmd+Shift+Z).
    Redo,
    /// Cut selection (Ctrl+X / Cmd+X).
    Cut,
    /// Copy selection (Ctrl+C / Cmd+C).
    Copy,
    /// Paste from clipboard (Ctrl+V / Cmd+V).
    Paste,
    /// Select all (Ctrl+A / Cmd+A).
    SelectAll,
    /// Delete selection or character (Delete / Delete).
    Delete,
    /// Backspace (Backspace / Backspace).
    Backspace,

    // =========================================================================
    // Find/Replace
    // =========================================================================
    /// Find (Ctrl+F / Cmd+F).
    Find,
    /// Find next (F3 / Cmd+G).
    FindNext,
    /// Find previous (Shift+F3 / Cmd+Shift+G).
    FindPrevious,
    /// Find and replace (Ctrl+H / Cmd+Option+F).
    Replace,

    // =========================================================================
    // Navigation
    // =========================================================================
    /// Navigate back (Alt+Left / Cmd+[).
    Back,
    /// Navigate forward (Alt+Right / Cmd+]).
    Forward,
    /// Refresh/reload (F5 / Cmd+R).
    Refresh,
    /// Add new tab (Ctrl+T / Cmd+T).
    AddTab,
    /// Next tab/child (Ctrl+Tab / Ctrl+Tab).
    NextChild,
    /// Previous tab/child (Ctrl+Shift+Tab / Ctrl+Shift+Tab).
    PreviousChild,

    // =========================================================================
    // View Operations
    // =========================================================================
    /// Zoom in (Ctrl++ / Cmd++).
    ZoomIn,
    /// Zoom out (Ctrl+- / Cmd+-).
    ZoomOut,
    /// Toggle full screen (F11 / Cmd+Ctrl+F).
    FullScreen,

    // =========================================================================
    // Text Formatting
    // =========================================================================
    /// Bold text (Ctrl+B / Cmd+B).
    Bold,
    /// Italic text (Ctrl+I / Cmd+I).
    Italic,
    /// Underline text (Ctrl+U / Cmd+U).
    Underline,

    // =========================================================================
    // Cursor Movement
    // =========================================================================
    /// Move to next character (Right).
    MoveToNextChar,
    /// Move to previous character (Left).
    MoveToPreviousChar,
    /// Move to next word (Ctrl+Right / Option+Right).
    MoveToNextWord,
    /// Move to previous word (Ctrl+Left / Option+Left).
    MoveToPreviousWord,
    /// Move to next line (Down).
    MoveToNextLine,
    /// Move to previous line (Up).
    MoveToPreviousLine,
    /// Move to start of line (Home / Cmd+Left).
    MoveToStartOfLine,
    /// Move to end of line (End / Cmd+Right).
    MoveToEndOfLine,
    /// Move to start of document (Ctrl+Home / Cmd+Up).
    MoveToStartOfDocument,
    /// Move to end of document (Ctrl+End / Cmd+Down).
    MoveToEndOfDocument,

    // =========================================================================
    // Selection (cursor movement with Shift)
    // =========================================================================
    /// Extend selection to next character (Shift+Right).
    SelectNextChar,
    /// Extend selection to previous character (Shift+Left).
    SelectPreviousChar,
    /// Extend selection to next word (Ctrl+Shift+Right / Option+Shift+Right).
    SelectNextWord,
    /// Extend selection to previous word (Ctrl+Shift+Left / Option+Shift+Left).
    SelectPreviousWord,
    /// Extend selection to next line (Shift+Down).
    SelectNextLine,
    /// Extend selection to previous line (Shift+Up).
    SelectPreviousLine,
    /// Extend selection to start of line (Shift+Home / Cmd+Shift+Left).
    SelectStartOfLine,
    /// Extend selection to end of line (Shift+End / Cmd+Shift+Right).
    SelectEndOfLine,
    /// Extend selection to start of document (Ctrl+Shift+Home / Cmd+Shift+Up).
    SelectStartOfDocument,
    /// Extend selection to end of document (Ctrl+Shift+End / Cmd+Shift+Down).
    SelectEndOfDocument,

    // =========================================================================
    // Deletion
    // =========================================================================
    /// Delete to start of word (Ctrl+Backspace / Option+Backspace).
    DeleteStartOfWord,
    /// Delete to end of word (Ctrl+Delete / Option+Delete).
    DeleteEndOfWord,

    // =========================================================================
    // Help
    // =========================================================================
    /// Open help contents (F1).
    HelpContents,
    /// Activate "What's This?" mode (Shift+F1).
    WhatsThis,

    // =========================================================================
    // Preferences
    // =========================================================================
    /// Open preferences/settings (Ctrl+, / Cmd+,).
    Preferences,

    // =========================================================================
    // Misc
    // =========================================================================
    /// Cancel/escape current operation (Escape).
    Cancel,
    /// Deselect (no standard shortcut).
    Deselect,
}

impl StandardKey {
    /// Get the platform-appropriate key sequence for this standard key.
    ///
    /// On macOS, uses Cmd instead of Ctrl where appropriate.
    /// On other platforms, uses Ctrl.
    pub fn key_sequence(self) -> KeySequence {
        // Determine if we're on macOS
        let is_macos = cfg!(target_os = "macos");

        // Helper for platform modifier
        let cmd_or_ctrl = if is_macos {
            KeyboardModifiers::META
        } else {
            KeyboardModifiers::CTRL
        };

        let cmd_or_ctrl_shift = if is_macos {
            KeyboardModifiers {
                meta: true,
                shift: true,
                ..Default::default()
            }
        } else {
            KeyboardModifiers::CTRL_SHIFT
        };

        match self {
            // Document operations
            Self::New => KeySequence::from_combination(KeyCombination::new(Key::N, cmd_or_ctrl)),
            Self::Open => KeySequence::from_combination(KeyCombination::new(Key::O, cmd_or_ctrl)),
            Self::Save => KeySequence::from_combination(KeyCombination::new(Key::S, cmd_or_ctrl)),
            Self::SaveAs => {
                KeySequence::from_combination(KeyCombination::new(Key::S, cmd_or_ctrl_shift))
            }
            Self::Close => KeySequence::from_combination(KeyCombination::new(Key::W, cmd_or_ctrl)),
            Self::Print => KeySequence::from_combination(KeyCombination::new(Key::P, cmd_or_ctrl)),
            Self::Quit => KeySequence::from_combination(KeyCombination::new(Key::Q, cmd_or_ctrl)),

            // Editing operations
            Self::Undo => KeySequence::from_combination(KeyCombination::new(Key::Z, cmd_or_ctrl)),
            Self::Redo => {
                KeySequence::from_combination(KeyCombination::new(Key::Z, cmd_or_ctrl_shift))
            }
            Self::Cut => KeySequence::from_combination(KeyCombination::new(Key::X, cmd_or_ctrl)),
            Self::Copy => KeySequence::from_combination(KeyCombination::new(Key::C, cmd_or_ctrl)),
            Self::Paste => KeySequence::from_combination(KeyCombination::new(Key::V, cmd_or_ctrl)),
            Self::SelectAll => {
                KeySequence::from_combination(KeyCombination::new(Key::A, cmd_or_ctrl))
            }
            Self::Delete => KeySequence::key_only(Key::Delete),
            Self::Backspace => KeySequence::key_only(Key::Backspace),

            // Find/Replace
            Self::Find => KeySequence::from_combination(KeyCombination::new(Key::F, cmd_or_ctrl)),
            Self::FindNext => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(Key::G, cmd_or_ctrl))
                } else {
                    KeySequence::key_only(Key::F3)
                }
            }
            Self::FindPrevious => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(Key::G, cmd_or_ctrl_shift))
                } else {
                    KeySequence::shift(Key::F3)
                }
            }
            Self::Replace => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::F,
                        KeyboardModifiers {
                            meta: true,
                            alt: true,
                            ..Default::default()
                        },
                    ))
                } else {
                    KeySequence::ctrl(Key::H)
                }
            }

            // Navigation
            Self::Back => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::BracketLeft,
                        KeyboardModifiers::META,
                    ))
                } else {
                    KeySequence::alt(Key::ArrowLeft)
                }
            }
            Self::Forward => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::BracketRight,
                        KeyboardModifiers::META,
                    ))
                } else {
                    KeySequence::alt(Key::ArrowRight)
                }
            }
            Self::Refresh => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::R,
                        KeyboardModifiers::META,
                    ))
                } else {
                    KeySequence::key_only(Key::F5)
                }
            }
            Self::AddTab => KeySequence::from_combination(KeyCombination::new(Key::T, cmd_or_ctrl)),
            Self::NextChild => KeySequence::from_combination(KeyCombination::ctrl(Key::Tab)),
            Self::PreviousChild => {
                KeySequence::from_combination(KeyCombination::ctrl_shift(Key::Tab))
            }

            // View operations
            Self::ZoomIn => {
                KeySequence::from_combination(KeyCombination::new(Key::Equal, cmd_or_ctrl))
            }
            Self::ZoomOut => {
                KeySequence::from_combination(KeyCombination::new(Key::Minus, cmd_or_ctrl))
            }
            Self::FullScreen => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::F,
                        KeyboardModifiers {
                            meta: true,
                            control: true,
                            ..Default::default()
                        },
                    ))
                } else {
                    KeySequence::key_only(Key::F11)
                }
            }

            // Text formatting
            Self::Bold => KeySequence::from_combination(KeyCombination::new(Key::B, cmd_or_ctrl)),
            Self::Italic => KeySequence::from_combination(KeyCombination::new(Key::I, cmd_or_ctrl)),
            Self::Underline => {
                KeySequence::from_combination(KeyCombination::new(Key::U, cmd_or_ctrl))
            }

            // Cursor movement
            Self::MoveToNextChar => KeySequence::key_only(Key::ArrowRight),
            Self::MoveToPreviousChar => KeySequence::key_only(Key::ArrowLeft),
            Self::MoveToNextWord => {
                if is_macos {
                    KeySequence::alt(Key::ArrowRight)
                } else {
                    KeySequence::ctrl(Key::ArrowRight)
                }
            }
            Self::MoveToPreviousWord => {
                if is_macos {
                    KeySequence::alt(Key::ArrowLeft)
                } else {
                    KeySequence::ctrl(Key::ArrowLeft)
                }
            }
            Self::MoveToNextLine => KeySequence::key_only(Key::ArrowDown),
            Self::MoveToPreviousLine => KeySequence::key_only(Key::ArrowUp),
            Self::MoveToStartOfLine => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowLeft,
                        KeyboardModifiers::META,
                    ))
                } else {
                    KeySequence::key_only(Key::Home)
                }
            }
            Self::MoveToEndOfLine => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowRight,
                        KeyboardModifiers::META,
                    ))
                } else {
                    KeySequence::key_only(Key::End)
                }
            }
            Self::MoveToStartOfDocument => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowUp,
                        KeyboardModifiers::META,
                    ))
                } else {
                    KeySequence::ctrl(Key::Home)
                }
            }
            Self::MoveToEndOfDocument => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowDown,
                        KeyboardModifiers::META,
                    ))
                } else {
                    KeySequence::ctrl(Key::End)
                }
            }

            // Selection
            Self::SelectNextChar => KeySequence::shift(Key::ArrowRight),
            Self::SelectPreviousChar => KeySequence::shift(Key::ArrowLeft),
            Self::SelectNextWord => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowRight,
                        KeyboardModifiers {
                            alt: true,
                            shift: true,
                            ..Default::default()
                        },
                    ))
                } else {
                    KeySequence::ctrl_shift(Key::ArrowRight)
                }
            }
            Self::SelectPreviousWord => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowLeft,
                        KeyboardModifiers {
                            alt: true,
                            shift: true,
                            ..Default::default()
                        },
                    ))
                } else {
                    KeySequence::ctrl_shift(Key::ArrowLeft)
                }
            }
            Self::SelectNextLine => KeySequence::shift(Key::ArrowDown),
            Self::SelectPreviousLine => KeySequence::shift(Key::ArrowUp),
            Self::SelectStartOfLine => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowLeft,
                        KeyboardModifiers {
                            meta: true,
                            shift: true,
                            ..Default::default()
                        },
                    ))
                } else {
                    KeySequence::shift(Key::Home)
                }
            }
            Self::SelectEndOfLine => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowRight,
                        KeyboardModifiers {
                            meta: true,
                            shift: true,
                            ..Default::default()
                        },
                    ))
                } else {
                    KeySequence::shift(Key::End)
                }
            }
            Self::SelectStartOfDocument => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowUp,
                        KeyboardModifiers {
                            meta: true,
                            shift: true,
                            ..Default::default()
                        },
                    ))
                } else {
                    KeySequence::ctrl_shift(Key::Home)
                }
            }
            Self::SelectEndOfDocument => {
                if is_macos {
                    KeySequence::from_combination(KeyCombination::new(
                        Key::ArrowDown,
                        KeyboardModifiers {
                            meta: true,
                            shift: true,
                            ..Default::default()
                        },
                    ))
                } else {
                    KeySequence::ctrl_shift(Key::End)
                }
            }

            // Deletion
            Self::DeleteStartOfWord => {
                if is_macos {
                    KeySequence::alt(Key::Backspace)
                } else {
                    KeySequence::ctrl(Key::Backspace)
                }
            }
            Self::DeleteEndOfWord => {
                if is_macos {
                    KeySequence::alt(Key::Delete)
                } else {
                    KeySequence::ctrl(Key::Delete)
                }
            }

            // Help
            Self::HelpContents => KeySequence::key_only(Key::F1),
            Self::WhatsThis => KeySequence::shift(Key::F1),

            // Preferences
            Self::Preferences => {
                KeySequence::from_combination(KeyCombination::new(Key::Comma, cmd_or_ctrl))
            }

            // Misc
            Self::Cancel => KeySequence::key_only(Key::Escape),
            Self::Deselect => KeySequence::key_only(Key::Escape), // No standard, use Escape
        }
    }

    /// Get the description for this standard key.
    pub fn description(self) -> &'static str {
        match self {
            Self::New => "Create new document",
            Self::Open => "Open document",
            Self::Save => "Save document",
            Self::SaveAs => "Save document as",
            Self::Close => "Close document",
            Self::Print => "Print document",
            Self::Quit => "Quit application",
            Self::Undo => "Undo",
            Self::Redo => "Redo",
            Self::Cut => "Cut",
            Self::Copy => "Copy",
            Self::Paste => "Paste",
            Self::SelectAll => "Select all",
            Self::Delete => "Delete",
            Self::Backspace => "Backspace",
            Self::Find => "Find",
            Self::FindNext => "Find next",
            Self::FindPrevious => "Find previous",
            Self::Replace => "Find and replace",
            Self::Back => "Navigate back",
            Self::Forward => "Navigate forward",
            Self::Refresh => "Refresh",
            Self::AddTab => "Add new tab",
            Self::NextChild => "Next tab",
            Self::PreviousChild => "Previous tab",
            Self::ZoomIn => "Zoom in",
            Self::ZoomOut => "Zoom out",
            Self::FullScreen => "Toggle full screen",
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::Underline => "Underline",
            Self::MoveToNextChar => "Move to next character",
            Self::MoveToPreviousChar => "Move to previous character",
            Self::MoveToNextWord => "Move to next word",
            Self::MoveToPreviousWord => "Move to previous word",
            Self::MoveToNextLine => "Move to next line",
            Self::MoveToPreviousLine => "Move to previous line",
            Self::MoveToStartOfLine => "Move to start of line",
            Self::MoveToEndOfLine => "Move to end of line",
            Self::MoveToStartOfDocument => "Move to start of document",
            Self::MoveToEndOfDocument => "Move to end of document",
            Self::SelectNextChar => "Select next character",
            Self::SelectPreviousChar => "Select previous character",
            Self::SelectNextWord => "Select next word",
            Self::SelectPreviousWord => "Select previous word",
            Self::SelectNextLine => "Select next line",
            Self::SelectPreviousLine => "Select previous line",
            Self::SelectStartOfLine => "Select to start of line",
            Self::SelectEndOfLine => "Select to end of line",
            Self::SelectStartOfDocument => "Select to start of document",
            Self::SelectEndOfDocument => "Select to end of document",
            Self::DeleteStartOfWord => "Delete to start of word",
            Self::DeleteEndOfWord => "Delete to end of word",
            Self::HelpContents => "Help contents",
            Self::WhatsThis => "What's This?",
            Self::Preferences => "Preferences",
            Self::Cancel => "Cancel",
            Self::Deselect => "Deselect",
        }
    }
}

// =============================================================================
// Shortcut Class
// =============================================================================

/// Internal state for a Shortcut.
struct ShortcutState {
    key_sequence: KeySequence,
    context: ShortcutContext,
    enabled: bool,
    auto_repeat: bool,
    widget_id: Option<ObjectId>,
}

/// A standalone keyboard shortcut that can trigger actions.
///
/// `Shortcut` provides a way to register keyboard shortcuts independently
/// of actions or menus. It has its own enabled state, context, and emits
/// an `activated` signal when triggered.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::{Shortcut, KeySequence, ShortcutContext};
///
/// // Create a shortcut for Ctrl+K, Ctrl+C (a two-key chord)
/// let shortcut = Shortcut::new("Ctrl+K, Ctrl+C".parse().unwrap())
///     .with_context(ShortcutContext::Window);
///
/// // Connect to the activated signal
/// shortcut.activated.connect(|_| {
///     println!("Shortcut activated!");
/// });
/// ```
pub struct Shortcut {
    object_base: ObjectBase,
    state: RwLock<ShortcutState>,
    generation: AtomicU64,

    /// Signal emitted when the shortcut is activated.
    pub activated: Signal<()>,

    /// Signal emitted when the shortcut is activated with ambiguity
    /// (multiple shortcuts match - this shortcut was chosen).
    pub activated_ambiguously: Signal<()>,
}

impl Shortcut {
    /// Create a new shortcut with the given key sequence.
    pub fn new(key_sequence: KeySequence) -> Self {
        Self {
            object_base: ObjectBase::new::<Self>(),
            state: RwLock::new(ShortcutState {
                key_sequence,
                context: ShortcutContext::Window,
                enabled: true,
                auto_repeat: false,
                widget_id: None,
            }),
            generation: AtomicU64::new(0),
            activated: Signal::new(),
            activated_ambiguously: Signal::new(),
        }
    }

    /// Create a shortcut from a string (convenience).
    pub fn from_str(s: &str) -> Result<Self, KeySequenceParseError> {
        Ok(Self::new(s.parse()?))
    }

    /// Get the key sequence.
    pub fn key_sequence(&self) -> KeySequence {
        self.state.read().key_sequence.clone()
    }

    /// Set the key sequence.
    pub fn set_key_sequence(&self, sequence: KeySequence) {
        self.state.write().key_sequence = sequence;
        self.generation.fetch_add(1, Ordering::Release);
    }

    /// Get the shortcut context.
    pub fn context(&self) -> ShortcutContext {
        self.state.read().context
    }

    /// Set the shortcut context.
    pub fn set_context(&self, context: ShortcutContext) {
        self.state.write().context = context;
        self.generation.fetch_add(1, Ordering::Release);
    }

    /// Builder pattern for context.
    pub fn with_context(self, context: ShortcutContext) -> Self {
        self.set_context(context);
        self
    }

    /// Check if the shortcut is enabled.
    pub fn is_enabled(&self) -> bool {
        self.state.read().enabled
    }

    /// Set whether the shortcut is enabled.
    pub fn set_enabled(&self, enabled: bool) {
        self.state.write().enabled = enabled;
        self.generation.fetch_add(1, Ordering::Release);
    }

    /// Builder pattern for enabled state.
    pub fn with_enabled(self, enabled: bool) -> Self {
        self.set_enabled(enabled);
        self
    }

    /// Check if auto-repeat is enabled.
    pub fn auto_repeat(&self) -> bool {
        self.state.read().auto_repeat
    }

    /// Set whether auto-repeat is enabled.
    pub fn set_auto_repeat(&self, auto_repeat: bool) {
        self.state.write().auto_repeat = auto_repeat;
    }

    /// Get the associated widget ID (for Widget context).
    pub fn widget_id(&self) -> Option<ObjectId> {
        self.state.read().widget_id
    }

    /// Set the associated widget ID.
    pub fn set_widget_id(&self, id: Option<ObjectId>) {
        self.state.write().widget_id = id;
    }

    /// Get the generation counter (for change detection).
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Activate the shortcut (emit the activated signal).
    pub fn activate(&self) {
        if self.is_enabled() {
            self.activated.emit(());
        }
    }

    /// Activate the shortcut with ambiguity notification.
    pub fn activate_ambiguously(&self) {
        if self.is_enabled() {
            self.activated_ambiguously.emit(());
            self.activated.emit(());
        }
    }
}

impl Object for Shortcut {
    fn object_id(&self) -> ObjectId {
        self.object_base.id()
    }
}

impl fmt::Debug for Shortcut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Shortcut")
            .field("id", &self.object_base.id())
            .field("key_sequence", &self.key_sequence())
            .field("context", &self.context())
            .field("enabled", &self.is_enabled())
            .finish()
    }
}

unsafe impl Send for Shortcut {}
unsafe impl Sync for Shortcut {}

// =============================================================================
// Shortcut Manager
// =============================================================================

/// Default timeout for multi-key chord sequences (in milliseconds).
pub const DEFAULT_CHORD_TIMEOUT_MS: u64 = 1500;

/// State for tracking multi-key chord input.
struct ChordState {
    /// Key combinations pressed so far in the current chord.
    pressed: Vec<KeyCombination>,
    /// When the first key of the chord was pressed.
    started_at: Option<Instant>,
    /// Timeout duration for completing the chord.
    timeout: Duration,
}

impl ChordState {
    fn new(timeout_ms: u64) -> Self {
        Self {
            pressed: Vec::with_capacity(MAX_KEY_SEQUENCE_LENGTH),
            started_at: None,
            timeout: Duration::from_millis(timeout_ms),
        }
    }

    fn reset(&mut self) {
        self.pressed.clear();
        self.started_at = None;
    }

    fn is_active(&self) -> bool {
        !self.pressed.is_empty()
    }

    fn is_expired(&self) -> bool {
        self.started_at
            .map(|start| start.elapsed() > self.timeout)
            .unwrap_or(false)
    }

    fn add_key(&mut self, combo: KeyCombination) {
        if self.pressed.is_empty() {
            self.started_at = Some(Instant::now());
        }
        self.pressed.push(combo);
    }
}

/// Manages shortcuts and handles multi-key chord sequences.
///
/// The `ShortcutManager` tracks registered shortcuts and handles the state
/// machine for multi-key chords. It supports:
///
/// - Single-key shortcuts (immediate activation)
/// - Multi-key chord sequences with timeout
/// - Conflict detection and ambiguous shortcut handling
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::{ShortcutManager, Shortcut, KeySequence};
///
/// let mut manager = ShortcutManager::new();
///
/// // Register shortcuts
/// let save = Arc::new(Shortcut::new("Ctrl+S".parse().unwrap()));
/// let comment = Arc::new(Shortcut::new("Ctrl+K, Ctrl+C".parse().unwrap()));
///
/// manager.register(save.clone());
/// manager.register(comment.clone());
///
/// // Process key events
/// match manager.process_key(Key::S, KeyboardModifiers::CTRL) {
///     ShortcutResult::Activated(shortcuts) => {
///         for s in shortcuts {
///             s.activate();
///         }
///     }
///     ShortcutResult::Pending => {
///         // Waiting for more keys in a chord
///     }
///     ShortcutResult::NoMatch => {
///         // No shortcut matched
///     }
/// }
/// ```
pub struct ShortcutManager {
    /// All registered shortcuts.
    shortcuts: Vec<Arc<Shortcut>>,
    /// Current chord state.
    chord_state: ChordState,
}

/// Result of processing a key event in the ShortcutManager.
#[derive(Debug)]
pub enum ShortcutResult {
    /// One or more shortcuts were activated.
    Activated(Vec<Arc<Shortcut>>),
    /// A partial chord match - waiting for more keys.
    Pending,
    /// No shortcut matched.
    NoMatch,
}

impl ShortcutManager {
    /// Create a new shortcut manager with default chord timeout.
    pub fn new() -> Self {
        Self {
            shortcuts: Vec::new(),
            chord_state: ChordState::new(DEFAULT_CHORD_TIMEOUT_MS),
        }
    }

    /// Create a new shortcut manager with custom chord timeout.
    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self {
            shortcuts: Vec::new(),
            chord_state: ChordState::new(timeout_ms),
        }
    }

    /// Register a shortcut.
    pub fn register(&mut self, shortcut: Arc<Shortcut>) {
        // Avoid duplicates
        if !self
            .shortcuts
            .iter()
            .any(|s| s.object_id() == shortcut.object_id())
        {
            self.shortcuts.push(shortcut);
        }
    }

    /// Unregister a shortcut.
    pub fn unregister(&mut self, shortcut: &Shortcut) {
        self.shortcuts
            .retain(|s| s.object_id() != shortcut.object_id());
    }

    /// Clear all registered shortcuts.
    pub fn clear(&mut self) {
        self.shortcuts.clear();
        self.chord_state.reset();
    }

    /// Get all registered shortcuts.
    pub fn shortcuts(&self) -> &[Arc<Shortcut>] {
        &self.shortcuts
    }

    /// Check if a chord is currently being input.
    pub fn is_chord_pending(&self) -> bool {
        self.chord_state.is_active()
    }

    /// Cancel any pending chord input.
    pub fn cancel_chord(&mut self) {
        self.chord_state.reset();
    }

    /// Process a key event and return the result.
    ///
    /// This is the main entry point for handling keyboard shortcuts.
    /// It maintains state for multi-key chords and returns the appropriate
    /// result based on matches.
    pub fn process_key(&mut self, key: Key, modifiers: KeyboardModifiers) -> ShortcutResult {
        // Check if chord has expired
        if self.chord_state.is_expired() {
            self.chord_state.reset();
        }

        // Create the key combination for this event
        let combo = KeyCombination::new(key, modifiers);

        // Add to current chord
        self.chord_state.add_key(combo);

        // Check all registered shortcuts
        let mut exact_matches = Vec::new();
        let mut has_partial = false;

        for shortcut in &self.shortcuts {
            if !shortcut.is_enabled() {
                continue;
            }

            let seq = shortcut.key_sequence();
            match seq.matches_partial(&self.chord_state.pressed) {
                SequenceMatch::ExactMatch => {
                    exact_matches.push(shortcut.clone());
                }
                SequenceMatch::PartialMatch => {
                    has_partial = true;
                }
                SequenceMatch::NoMatch => {}
            }
        }

        // If we have exact matches, activate them and reset
        if !exact_matches.is_empty() {
            self.chord_state.reset();

            // If there's ambiguity (multiple matches), mark them as ambiguous
            if exact_matches.len() > 1 {
                for s in &exact_matches {
                    s.activate_ambiguously();
                }
            }

            return ShortcutResult::Activated(exact_matches);
        }

        // If we have partial matches, wait for more input
        if has_partial {
            return ShortcutResult::Pending;
        }

        // No matches at all - reset and return
        self.chord_state.reset();
        ShortcutResult::NoMatch
    }

    /// Find all shortcuts that conflict with the given key sequence.
    ///
    /// Two shortcuts conflict if one is a prefix of the other (for chords)
    /// or if they are exactly the same.
    pub fn find_conflicts(&self, sequence: &KeySequence) -> Vec<Arc<Shortcut>> {
        let mut conflicts = Vec::new();

        for shortcut in &self.shortcuts {
            let other = shortcut.key_sequence();

            // Check if sequences are equal
            if sequence == &other {
                conflicts.push(shortcut.clone());
                continue;
            }

            // Check if one is a prefix of the other
            let shorter;
            let longer;
            if sequence.count() < other.count() {
                shorter = sequence;
                longer = &other;
            } else {
                shorter = &other;
                longer = sequence;
            }

            // Check if shorter is a prefix of longer
            let is_prefix = shorter
                .combinations()
                .iter()
                .zip(longer.combinations())
                .all(|(a, b)| a.key == b.key && a.modifiers == b.modifiers);

            if is_prefix {
                conflicts.push(shortcut.clone());
            }
        }

        conflicts
    }
}

impl Default for ShortcutManager {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Mnemonic Utilities
// =============================================================================

/// Result of parsing text for a mnemonic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MnemonicText {
    /// The display text with '&' markers removed (and '&&' converted to '&').
    pub display_text: String,
    /// The mnemonic character (lowercase), if any.
    pub mnemonic: Option<char>,
    /// The index in display_text where the mnemonic character is located.
    pub mnemonic_index: Option<usize>,
}

/// Parse text containing an optional mnemonic marker.
///
/// The '&' character indicates the following character is a mnemonic:
/// - `"&Open"` -> display "Open", mnemonic 'o', index 0
/// - `"Save &As"` -> display "Save As", mnemonic 'a', index 5
/// - `"Fish && Chips"` -> display "Fish & Chips", no mnemonic
/// - `"&&Open"` -> display "&Open", no mnemonic
///
/// # Arguments
///
/// * `text` - The text potentially containing '&' markers.
///
/// # Returns
///
/// A [`MnemonicText`] containing the display text and mnemonic info.
pub fn parse_mnemonic(text: &str) -> MnemonicText {
    let mut display_text = String::with_capacity(text.len());
    let mut mnemonic: Option<char> = None;
    let mut mnemonic_index: Option<usize> = None;

    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            // Check the next character
            match chars.peek() {
                Some('&') => {
                    // '&&' -> literal '&'
                    display_text.push('&');
                    chars.next(); // consume the second '&'
                }
                Some(&next_ch) if next_ch.is_alphanumeric() => {
                    // '&X' -> mnemonic
                    if mnemonic.is_none() {
                        mnemonic = Some(next_ch.to_ascii_lowercase());
                        mnemonic_index = Some(display_text.len());
                    }
                    display_text.push(next_ch);
                    chars.next(); // consume the mnemonic character
                }
                _ => {
                    // Lone '&' at end or followed by non-alphanumeric - keep it
                    display_text.push(ch);
                }
            }
        } else {
            display_text.push(ch);
        }
    }

    MnemonicText {
        display_text,
        mnemonic,
        mnemonic_index,
    }
}

/// Convert a mnemonic character to the corresponding Key.
///
/// Returns `None` for non-letter/non-digit characters.
pub fn mnemonic_to_key(ch: char) -> Option<Key> {
    match ch.to_ascii_lowercase() {
        'a' => Some(Key::A),
        'b' => Some(Key::B),
        'c' => Some(Key::C),
        'd' => Some(Key::D),
        'e' => Some(Key::E),
        'f' => Some(Key::F),
        'g' => Some(Key::G),
        'h' => Some(Key::H),
        'i' => Some(Key::I),
        'j' => Some(Key::J),
        'k' => Some(Key::K),
        'l' => Some(Key::L),
        'm' => Some(Key::M),
        'n' => Some(Key::N),
        'o' => Some(Key::O),
        'p' => Some(Key::P),
        'q' => Some(Key::Q),
        'r' => Some(Key::R),
        's' => Some(Key::S),
        't' => Some(Key::T),
        'u' => Some(Key::U),
        'v' => Some(Key::V),
        'w' => Some(Key::W),
        'x' => Some(Key::X),
        'y' => Some(Key::Y),
        'z' => Some(Key::Z),
        '0' => Some(Key::Digit0),
        '1' => Some(Key::Digit1),
        '2' => Some(Key::Digit2),
        '3' => Some(Key::Digit3),
        '4' => Some(Key::Digit4),
        '5' => Some(Key::Digit5),
        '6' => Some(Key::Digit6),
        '7' => Some(Key::Digit7),
        '8' => Some(Key::Digit8),
        '9' => Some(Key::Digit9),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // KeySequence Tests
    // =========================================================================

    #[test]
    fn test_key_sequence_new() {
        let seq = KeySequence::new(
            Key::S,
            KeyboardModifiers {
                control: true,
                ..Default::default()
            },
        );
        assert_eq!(seq.key(), Key::S);
        assert!(seq.modifiers().control);
        assert!(!seq.modifiers().alt);
    }

    #[test]
    fn test_key_sequence_ctrl() {
        let seq = KeySequence::ctrl(Key::S);
        assert_eq!(seq.key(), Key::S);
        assert!(seq.modifiers().control);
    }

    #[test]
    fn test_key_sequence_alt() {
        let seq = KeySequence::alt(Key::F4);
        assert_eq!(seq.key(), Key::F4);
        assert!(seq.modifiers().alt);
    }

    #[test]
    fn test_key_sequence_matches() {
        let seq = KeySequence::ctrl(Key::S);
        let mods_ctrl = KeyboardModifiers {
            control: true,
            ..Default::default()
        };
        let mods_none = KeyboardModifiers::NONE;

        assert!(seq.matches(Key::S, mods_ctrl));
        assert!(!seq.matches(Key::S, mods_none));
        assert!(!seq.matches(Key::A, mods_ctrl));
    }

    #[test]
    fn test_key_sequence_display() {
        assert_eq!(KeySequence::ctrl(Key::S).to_string(), "Ctrl+S");
        assert_eq!(KeySequence::alt(Key::F4).to_string(), "Alt+F4");
        assert_eq!(KeySequence::ctrl_shift(Key::N).to_string(), "Ctrl+Shift+N");
        assert_eq!(KeySequence::key_only(Key::F1).to_string(), "F1");
    }

    // =========================================================================
    // Parsing Tests
    // =========================================================================

    #[test]
    fn test_parse_simple_key() {
        let seq: KeySequence = "S".parse().unwrap();
        assert_eq!(seq.key(), Key::S);
        assert!(!seq.modifiers().control);
    }

    #[test]
    fn test_parse_ctrl_key() {
        let seq: KeySequence = "Ctrl+S".parse().unwrap();
        assert_eq!(seq.key(), Key::S);
        assert!(seq.modifiers().control);
    }

    #[test]
    fn test_parse_alt_key() {
        let seq: KeySequence = "Alt+F4".parse().unwrap();
        assert_eq!(seq.key(), Key::F4);
        assert!(seq.modifiers().alt);
    }

    #[test]
    fn test_parse_multiple_modifiers() {
        let seq: KeySequence = "Ctrl+Shift+N".parse().unwrap();
        assert_eq!(seq.key(), Key::N);
        assert!(seq.modifiers().control);
        assert!(seq.modifiers().shift);
    }

    #[test]
    fn test_parse_case_insensitive() {
        let seq1: KeySequence = "ctrl+s".parse().unwrap();
        let seq2: KeySequence = "CTRL+S".parse().unwrap();
        assert_eq!(seq1, seq2);
    }

    #[test]
    fn test_parse_function_key() {
        let seq: KeySequence = "F1".parse().unwrap();
        assert_eq!(seq.key(), Key::F1);
    }

    #[test]
    fn test_parse_special_key() {
        let seq: KeySequence = "Ctrl+Enter".parse().unwrap();
        assert_eq!(seq.key(), Key::Enter);
    }

    // =========================================================================
    // Multi-Key Sequence Tests
    // =========================================================================

    #[test]
    fn test_parse_two_key_chord() {
        let seq: KeySequence = "Ctrl+K, Ctrl+C".parse().unwrap();
        assert_eq!(seq.count(), 2);
        assert_eq!(seq.combinations()[0].key, Key::K);
        assert!(seq.combinations()[0].modifiers.control);
        assert_eq!(seq.combinations()[1].key, Key::C);
        assert!(seq.combinations()[1].modifiers.control);
    }

    #[test]
    fn test_parse_three_key_chord() {
        let seq: KeySequence = "Alt+X, Ctrl+S, Q".parse().unwrap();
        assert_eq!(seq.count(), 3);
        assert_eq!(seq.combinations()[0].key, Key::X);
        assert!(seq.combinations()[0].modifiers.alt);
        assert_eq!(seq.combinations()[1].key, Key::S);
        assert!(seq.combinations()[1].modifiers.control);
        assert_eq!(seq.combinations()[2].key, Key::Q);
        assert!(!seq.combinations()[2].modifiers.control);
    }

    #[test]
    fn test_parse_too_many_keys() {
        let result: Result<KeySequence, _> = "A, B, C, D, E".parse();
        assert_eq!(result.unwrap_err(), KeySequenceParseError::TooManyKeys);
    }

    #[test]
    fn test_key_sequence_partial_match() {
        let seq: KeySequence = "Ctrl+K, Ctrl+C".parse().unwrap();

        // No keys pressed
        assert_eq!(seq.matches_partial(&[]), SequenceMatch::NoMatch);

        // First key matches
        let first_key = KeyCombination::ctrl(Key::K);
        assert_eq!(
            seq.matches_partial(&[first_key]),
            SequenceMatch::PartialMatch
        );

        // Both keys match
        let second_key = KeyCombination::ctrl(Key::C);
        assert_eq!(
            seq.matches_partial(&[first_key, second_key]),
            SequenceMatch::ExactMatch
        );

        // Wrong first key
        let wrong_key = KeyCombination::ctrl(Key::X);
        assert_eq!(seq.matches_partial(&[wrong_key]), SequenceMatch::NoMatch);
    }

    #[test]
    fn test_multi_key_display() {
        let seq: KeySequence = "Ctrl+K, Ctrl+C".parse().unwrap();
        assert_eq!(seq.to_string(), "Ctrl+K, Ctrl+C");
    }

    #[test]
    fn test_parse_empty_error() {
        let result: Result<KeySequence, _> = "".parse();
        assert_eq!(result.unwrap_err(), KeySequenceParseError::Empty);
    }

    #[test]
    fn test_parse_no_key_error() {
        let result: Result<KeySequence, _> = "Ctrl+Alt".parse();
        assert_eq!(result.unwrap_err(), KeySequenceParseError::NoKey);
    }

    #[test]
    fn test_parse_unknown_key_error() {
        let result: Result<KeySequence, _> = "Ctrl+XYZ".parse();
        assert!(matches!(
            result.unwrap_err(),
            KeySequenceParseError::UnknownKey(_)
        ));
    }

    // =========================================================================
    // Mnemonic Tests
    // =========================================================================

    #[test]
    fn test_parse_mnemonic_simple() {
        let result = parse_mnemonic("&Open");
        assert_eq!(result.display_text, "Open");
        assert_eq!(result.mnemonic, Some('o'));
        assert_eq!(result.mnemonic_index, Some(0));
    }

    #[test]
    fn test_parse_mnemonic_middle() {
        let result = parse_mnemonic("Save &As");
        assert_eq!(result.display_text, "Save As");
        assert_eq!(result.mnemonic, Some('a'));
        assert_eq!(result.mnemonic_index, Some(5));
    }

    #[test]
    fn test_parse_mnemonic_escaped() {
        let result = parse_mnemonic("Fish && Chips");
        assert_eq!(result.display_text, "Fish & Chips");
        assert_eq!(result.mnemonic, None);
    }

    #[test]
    fn test_parse_mnemonic_escaped_then_mnemonic() {
        let result = parse_mnemonic("&& &Open");
        assert_eq!(result.display_text, "& Open");
        assert_eq!(result.mnemonic, Some('o'));
        assert_eq!(result.mnemonic_index, Some(2));
    }

    #[test]
    fn test_parse_mnemonic_no_mnemonic() {
        let result = parse_mnemonic("Plain Text");
        assert_eq!(result.display_text, "Plain Text");
        assert_eq!(result.mnemonic, None);
    }

    #[test]
    fn test_parse_mnemonic_first_only() {
        // Only the first '&' creates a mnemonic
        let result = parse_mnemonic("&File &Edit");
        assert_eq!(result.display_text, "File Edit");
        assert_eq!(result.mnemonic, Some('f'));
        assert_eq!(result.mnemonic_index, Some(0));
    }

    #[test]
    fn test_parse_mnemonic_digit() {
        let result = parse_mnemonic("Item &1");
        assert_eq!(result.display_text, "Item 1");
        assert_eq!(result.mnemonic, Some('1'));
        assert_eq!(result.mnemonic_index, Some(5));
    }

    #[test]
    fn test_parse_mnemonic_trailing_ampersand() {
        let result = parse_mnemonic("Test&");
        assert_eq!(result.display_text, "Test&");
        assert_eq!(result.mnemonic, None);
    }

    #[test]
    fn test_mnemonic_to_key() {
        assert_eq!(mnemonic_to_key('a'), Some(Key::A));
        assert_eq!(mnemonic_to_key('Z'), Some(Key::Z));
        assert_eq!(mnemonic_to_key('5'), Some(Key::Digit5));
        assert_eq!(mnemonic_to_key('!'), None);
    }

    // =========================================================================
    // StandardKey Tests
    // =========================================================================

    #[test]
    fn test_standard_key_basic() {
        let copy = StandardKey::Copy.key_sequence();
        // On any platform, Copy should be some form of Ctrl/Cmd+C
        assert_eq!(copy.key(), Key::C);
        assert!(copy.modifiers().control || copy.modifiers().meta);
    }

    #[test]
    fn test_standard_key_description() {
        assert_eq!(StandardKey::Copy.description(), "Copy");
        assert_eq!(StandardKey::Save.description(), "Save document");
        assert_eq!(StandardKey::Undo.description(), "Undo");
    }

    #[test]
    fn test_standard_key_navigation() {
        let back = StandardKey::Back.key_sequence();
        // Back navigation has some key
        assert!(back.count() == 1);
    }

    // =========================================================================
    // Shortcut Tests
    // =========================================================================

    #[test]
    fn test_shortcut_creation() {
        horizon_lattice_core::init_global_registry();
        let shortcut = Shortcut::new(KeySequence::ctrl(Key::S));
        assert!(shortcut.is_enabled());
        assert_eq!(shortcut.context(), ShortcutContext::Window);
        assert_eq!(shortcut.key_sequence(), KeySequence::ctrl(Key::S));
    }

    #[test]
    fn test_shortcut_enabled_state() {
        horizon_lattice_core::init_global_registry();
        let shortcut = Shortcut::new(KeySequence::ctrl(Key::S));
        assert!(shortcut.is_enabled());

        shortcut.set_enabled(false);
        assert!(!shortcut.is_enabled());
    }

    #[test]
    fn test_shortcut_context() {
        horizon_lattice_core::init_global_registry();
        let shortcut =
            Shortcut::new(KeySequence::ctrl(Key::S)).with_context(ShortcutContext::Application);
        assert_eq!(shortcut.context(), ShortcutContext::Application);
    }

    #[test]
    fn test_shortcut_from_str() {
        horizon_lattice_core::init_global_registry();
        let shortcut = Shortcut::from_str("Ctrl+K, Ctrl+C").unwrap();
        assert_eq!(shortcut.key_sequence().count(), 2);
    }

    // =========================================================================
    // ShortcutManager Tests
    // =========================================================================

    #[test]
    fn test_shortcut_manager_register() {
        horizon_lattice_core::init_global_registry();
        let mut manager = ShortcutManager::new();
        let shortcut = Arc::new(Shortcut::new(KeySequence::ctrl(Key::S)));

        manager.register(shortcut.clone());
        assert_eq!(manager.shortcuts().len(), 1);

        // Registering same shortcut again should not duplicate
        manager.register(shortcut.clone());
        assert_eq!(manager.shortcuts().len(), 1);
    }

    #[test]
    fn test_shortcut_manager_single_key() {
        horizon_lattice_core::init_global_registry();
        let mut manager = ShortcutManager::new();
        let shortcut = Arc::new(Shortcut::new(KeySequence::ctrl(Key::S)));
        manager.register(shortcut);

        let result = manager.process_key(Key::S, KeyboardModifiers::CTRL);
        assert!(matches!(result, ShortcutResult::Activated(_)));

        if let ShortcutResult::Activated(shortcuts) = result {
            assert_eq!(shortcuts.len(), 1);
        }
    }

    #[test]
    fn test_shortcut_manager_chord() {
        horizon_lattice_core::init_global_registry();
        let mut manager = ShortcutManager::new();
        let shortcut = Arc::new(Shortcut::new("Ctrl+K, Ctrl+C".parse().unwrap()));
        manager.register(shortcut);

        // First key should be pending
        let result1 = manager.process_key(Key::K, KeyboardModifiers::CTRL);
        assert!(matches!(result1, ShortcutResult::Pending));

        // Second key should activate
        let result2 = manager.process_key(Key::C, KeyboardModifiers::CTRL);
        assert!(matches!(result2, ShortcutResult::Activated(_)));
    }

    #[test]
    fn test_shortcut_manager_no_match() {
        horizon_lattice_core::init_global_registry();
        let mut manager = ShortcutManager::new();
        let shortcut = Arc::new(Shortcut::new(KeySequence::ctrl(Key::S)));
        manager.register(shortcut);

        // Pressing wrong key should not match
        let result = manager.process_key(Key::X, KeyboardModifiers::CTRL);
        assert!(matches!(result, ShortcutResult::NoMatch));
    }

    #[test]
    fn test_shortcut_manager_conflict_detection() {
        horizon_lattice_core::init_global_registry();
        let mut manager = ShortcutManager::new();

        // Register Ctrl+K (single key)
        let single = Arc::new(Shortcut::new(KeySequence::ctrl(Key::K)));
        manager.register(single);

        // Ctrl+K, Ctrl+C conflicts with Ctrl+K (prefix conflict)
        let chord: KeySequence = "Ctrl+K, Ctrl+C".parse().unwrap();
        let conflicts = manager.find_conflicts(&chord);
        assert_eq!(conflicts.len(), 1);
    }

    #[test]
    fn test_shortcut_manager_disabled_shortcut() {
        horizon_lattice_core::init_global_registry();
        let mut manager = ShortcutManager::new();
        let shortcut = Arc::new(Shortcut::new(KeySequence::ctrl(Key::S)));
        shortcut.set_enabled(false);
        manager.register(shortcut);

        // Disabled shortcut should not match
        let result = manager.process_key(Key::S, KeyboardModifiers::CTRL);
        assert!(matches!(result, ShortcutResult::NoMatch));
    }
}
