//! Keyboard shortcut system for Horizon Lattice.
//!
//! This module provides types and utilities for keyboard shortcuts:
//!
//! - [`KeySequence`]: Represents a keyboard shortcut (key + modifiers)
//! - Parsing utilities for human-readable strings like "Ctrl+S"
//! - Mnemonic extraction from "&Open" style text
//!
//! # Shortcuts
//!
//! Shortcuts are key combinations that trigger button activation:
//!
//! ```ignore
//! use horizon_lattice::widget::{KeySequence, Key, KeyboardModifiers};
//!
//! // Create from key and modifiers
//! let shortcut = KeySequence::new(Key::S, KeyboardModifiers { control: true, ..Default::default() });
//!
//! // Parse from string
//! let shortcut = KeySequence::from_str("Ctrl+S").unwrap();
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

use crate::widget::events::{Key, KeyboardModifiers};

/// A keyboard shortcut represented as a key with modifiers.
///
/// KeySequence represents a single key combination like Ctrl+S or Alt+F4.
/// It can be created directly, parsed from a string, or matched against
/// key press events.
///
/// # String Format
///
/// When parsing from strings, the format is `[modifiers+]key` where:
/// - Modifiers: `Ctrl`, `Alt`, `Shift`, `Meta` (or `Cmd` on macOS)
/// - Key: Letter (A-Z), digit (0-9), or special key name
///
/// Examples: `"Ctrl+S"`, `"Alt+F4"`, `"Ctrl+Shift+N"`, `"F1"`
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeySequence {
    /// The primary key.
    pub key: Key,
    /// The modifier keys that must be held.
    pub modifiers: KeyboardModifiers,
}

impl KeySequence {
    /// Create a new key sequence from a key and modifiers.
    pub fn new(key: Key, modifiers: KeyboardModifiers) -> Self {
        Self { key, modifiers }
    }

    /// Create a key sequence with no modifiers.
    pub fn key_only(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers::NONE,
        }
    }

    /// Create a Ctrl+key shortcut.
    pub fn ctrl(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers {
                control: true,
                ..Default::default()
            },
        }
    }

    /// Create an Alt+key shortcut.
    pub fn alt(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers {
                alt: true,
                ..Default::default()
            },
        }
    }

    /// Create a Shift+key shortcut.
    pub fn shift(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers {
                shift: true,
                ..Default::default()
            },
        }
    }

    /// Create a Ctrl+Shift+key shortcut.
    pub fn ctrl_shift(key: Key) -> Self {
        Self {
            key,
            modifiers: KeyboardModifiers {
                control: true,
                shift: true,
                ..Default::default()
            },
        }
    }

    /// Check if this key sequence matches the given key and modifiers.
    pub fn matches(&self, key: Key, modifiers: KeyboardModifiers) -> bool {
        self.key == key
            && self.modifiers.control == modifiers.control
            && self.modifiers.alt == modifiers.alt
            && self.modifiers.shift == modifiers.shift
            && self.modifiers.meta == modifiers.meta
    }
}

impl fmt::Display for KeySequence {
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

/// Error type for parsing key sequences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeySequenceParseError {
    /// The string is empty.
    Empty,
    /// No key was specified (only modifiers).
    NoKey,
    /// Unknown key name.
    UnknownKey(String),
}

impl fmt::Display for KeySequenceParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty key sequence"),
            Self::NoKey => write!(f, "no key specified (only modifiers)"),
            Self::UnknownKey(s) => write!(f, "unknown key: {}", s),
        }
    }
}

impl std::error::Error for KeySequenceParseError {}

impl FromStr for KeySequence {
    type Err = KeySequenceParseError;

    /// Parse a key sequence from a string like "Ctrl+S" or "Alt+F4".
    ///
    /// The format is case-insensitive for modifiers but case-insensitive
    /// for single letter keys (they are normalized to uppercase).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
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
            Some(k) => Ok(KeySequence::new(k, modifiers)),
            None => Err(KeySequenceParseError::NoKey),
        }
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
        assert_eq!(seq.key, Key::S);
        assert!(seq.modifiers.control);
        assert!(!seq.modifiers.alt);
    }

    #[test]
    fn test_key_sequence_ctrl() {
        let seq = KeySequence::ctrl(Key::S);
        assert_eq!(seq.key, Key::S);
        assert!(seq.modifiers.control);
    }

    #[test]
    fn test_key_sequence_alt() {
        let seq = KeySequence::alt(Key::F4);
        assert_eq!(seq.key, Key::F4);
        assert!(seq.modifiers.alt);
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
        assert_eq!(seq.key, Key::S);
        assert!(!seq.modifiers.control);
    }

    #[test]
    fn test_parse_ctrl_key() {
        let seq: KeySequence = "Ctrl+S".parse().unwrap();
        assert_eq!(seq.key, Key::S);
        assert!(seq.modifiers.control);
    }

    #[test]
    fn test_parse_alt_key() {
        let seq: KeySequence = "Alt+F4".parse().unwrap();
        assert_eq!(seq.key, Key::F4);
        assert!(seq.modifiers.alt);
    }

    #[test]
    fn test_parse_multiple_modifiers() {
        let seq: KeySequence = "Ctrl+Shift+N".parse().unwrap();
        assert_eq!(seq.key, Key::N);
        assert!(seq.modifiers.control);
        assert!(seq.modifiers.shift);
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
        assert_eq!(seq.key, Key::F1);
    }

    #[test]
    fn test_parse_special_key() {
        let seq: KeySequence = "Ctrl+Enter".parse().unwrap();
        assert_eq!(seq.key, Key::Enter);
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
}
