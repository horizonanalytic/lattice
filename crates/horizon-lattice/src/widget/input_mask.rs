//! Input mask support for text widgets.
//!
//! This module provides input masking functionality inspired by Qt's QLineEdit input mask system.
//! Input masks constrain user input to specific patterns, useful for formatted data like
//! phone numbers, dates, IP addresses, and license keys.
//!
//! # Mask Characters
//!
//! | Char | Meaning |
//! |------|---------|
//! | `A`  | Letter required (A-Z, a-z) |
//! | `a`  | Letter permitted but not required |
//! | `N`  | Alphanumeric required (A-Z, a-z, 0-9) |
//! | `n`  | Alphanumeric permitted but not required |
//! | `X`  | Any non-blank character required |
//! | `x`  | Any non-blank character permitted but not required |
//! | `9`  | Digit required (0-9) |
//! | `0`  | Digit permitted but not required |
//! | `D`  | Digit 1-9 required (no zero) |
//! | `d`  | Digit 1-9 permitted but not required |
//! | `#`  | Digit or +/- sign permitted but not required |
//! | `H`  | Hex character required (A-F, a-f, 0-9) |
//! | `h`  | Hex character permitted but not required |
//! | `B`  | Binary character required (0-1) |
//! | `b`  | Binary character permitted but not required |
//!
//! # Meta Characters
//!
//! | Char | Meaning |
//! |------|---------|
//! | `>`  | All following alphabetic characters are uppercased |
//! | `<`  | All following alphabetic characters are lowercased |
//! | `!`  | Switch off case conversion |
//! | `\`  | Escape the following character to use it as a literal separator |
//! | `;c` | Terminates the mask and sets the blank character to `c` |
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::{widgets::LineEdit, input_mask::InputMask};
//!
//! let mut edit = LineEdit::new();
//!
//! // Phone number mask: (999) 999-9999
//! edit.set_input_mask("(999) 999-9999");
//!
//! // IP address with underscore blanks: 000.000.000.000;_
//! edit.set_input_mask("000.000.000.000;_");
//!
//! // MAC address: HH:HH:HH:HH:HH:HH;_
//! edit.set_input_mask("HH:HH:HH:HH:HH:HH;_");
//!
//! // ISO Date: 0000-00-00
//! edit.set_input_mask("0000-00-00");
//!
//! // License key (uppercase letters, # for blanks): >AAAAA-AAAAA-AAAAA-AAAAA-AAAAA;#
//! edit.set_input_mask(">AAAAA-AAAAA-AAAAA-AAAAA-AAAAA;#");
//! ```

use std::fmt;

/// Case conversion mode for alphabetic characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CaseMode {
    /// No case conversion.
    #[default]
    None,
    /// Convert to uppercase.
    Upper,
    /// Convert to lowercase.
    Lower,
}

/// Represents a single element in a parsed input mask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaskElement {
    /// A literal character that is displayed but cannot be edited.
    Literal(char),
    /// A required character slot with specific character class.
    Required(CharClass, CaseMode),
    /// An optional character slot with specific character class.
    Optional(CharClass, CaseMode),
}

impl MaskElement {
    /// Returns true if this element is a literal (separator).
    pub fn is_literal(&self) -> bool {
        matches!(self, MaskElement::Literal(_))
    }

    /// Returns true if this element requires user input.
    pub fn is_required(&self) -> bool {
        matches!(self, MaskElement::Required(_, _))
    }

    /// Returns true if this element is optional.
    pub fn is_optional(&self) -> bool {
        matches!(self, MaskElement::Optional(_, _))
    }

    /// Returns true if this element can accept user input (not a literal).
    pub fn is_editable(&self) -> bool {
        !self.is_literal()
    }

    /// Check if a character is valid for this mask element.
    pub fn accepts(&self, ch: char) -> bool {
        match self {
            MaskElement::Literal(lit) => ch == *lit,
            MaskElement::Required(class, _) | MaskElement::Optional(class, _) => class.accepts(ch),
        }
    }

    /// Transform a character according to case mode.
    pub fn transform(&self, ch: char) -> char {
        match self {
            MaskElement::Required(_, mode) | MaskElement::Optional(_, mode) => match mode {
                CaseMode::None => ch,
                CaseMode::Upper => ch.to_uppercase().next().unwrap_or(ch),
                CaseMode::Lower => ch.to_lowercase().next().unwrap_or(ch),
            },
            MaskElement::Literal(_) => ch,
        }
    }
}

/// Character class for mask elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharClass {
    /// Letter (A-Z, a-z) - mask char: A/a
    Letter,
    /// Alphanumeric (A-Z, a-z, 0-9) - mask char: N/n
    Alphanumeric,
    /// Any non-blank character - mask char: X/x
    Any,
    /// Digit (0-9) - mask char: 9/0
    Digit,
    /// Non-zero digit (1-9) - mask char: D/d
    NonZeroDigit,
    /// Digit or sign (+/-) - mask char: #
    DigitOrSign,
    /// Hexadecimal (0-9, A-F, a-f) - mask char: H/h
    Hex,
    /// Binary (0-1) - mask char: B/b
    Binary,
}

impl CharClass {
    /// Check if a character belongs to this character class.
    pub fn accepts(&self, ch: char) -> bool {
        match self {
            CharClass::Letter => ch.is_alphabetic(),
            CharClass::Alphanumeric => ch.is_alphanumeric(),
            CharClass::Any => !ch.is_whitespace(),
            CharClass::Digit => ch.is_ascii_digit(),
            CharClass::NonZeroDigit => ch.is_ascii_digit() && ch != '0',
            CharClass::DigitOrSign => ch.is_ascii_digit() || ch == '+' || ch == '-',
            CharClass::Hex => ch.is_ascii_hexdigit(),
            CharClass::Binary => ch == '0' || ch == '1',
        }
    }
}

/// Parsed input mask that constrains user input to a specific pattern.
///
/// The input mask defines which characters can be entered at each position,
/// automatically inserts literal separators, and optionally transforms case.
#[derive(Debug, Clone)]
pub struct InputMask {
    /// The original mask string.
    pattern: String,
    /// Parsed mask elements.
    elements: Vec<MaskElement>,
    /// Character displayed for empty optional/required positions.
    blank_char: char,
}

impl InputMask {
    /// Create a new input mask from a pattern string.
    ///
    /// Returns `None` if the pattern is empty or contains only the blank specifier.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::widget::input_mask::InputMask;
    ///
    /// let mask = InputMask::new("(999) 999-9999").unwrap();
    /// let mask_with_blank = InputMask::new("000.000.000.000;_").unwrap();
    /// ```
    pub fn new(pattern: &str) -> Option<Self> {
        if pattern.is_empty() {
            return None;
        }

        let (mask_str, blank_char) = Self::parse_blank_specifier(pattern);

        if mask_str.is_empty() {
            return None;
        }

        let elements = Self::parse_pattern(mask_str);

        if elements.is_empty() {
            return None;
        }

        Some(Self {
            pattern: pattern.to_string(),
            elements,
            blank_char,
        })
    }

    /// Parse the blank character specifier (`;c` at end of pattern).
    fn parse_blank_specifier(pattern: &str) -> (&str, char) {
        // Look for unescaped semicolon followed by a character
        let chars: Vec<char> = pattern.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '\\' {
                // Skip escaped character
                i += 2;
                continue;
            }
            if chars[i] == ';' && i + 1 < chars.len() {
                // Found blank specifier
                let blank = chars[i + 1];
                // Return pattern up to semicolon and the blank char
                let byte_pos = pattern
                    .char_indices()
                    .nth(i)
                    .map(|(pos, _)| pos)
                    .unwrap_or(pattern.len());
                return (&pattern[..byte_pos], blank);
            }
            i += 1;
        }
        (pattern, ' ') // Default blank is space
    }

    /// Parse the mask pattern into elements.
    fn parse_pattern(pattern: &str) -> Vec<MaskElement> {
        let mut elements = Vec::new();
        let mut case_mode = CaseMode::None;
        let mut chars = pattern.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                // Escape next character as literal
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        elements.push(MaskElement::Literal(escaped));
                    }
                }
                // Case conversion modes
                '>' => case_mode = CaseMode::Upper,
                '<' => case_mode = CaseMode::Lower,
                '!' => case_mode = CaseMode::None,
                // Required characters
                'A' => elements.push(MaskElement::Required(CharClass::Letter, case_mode)),
                'N' => elements.push(MaskElement::Required(CharClass::Alphanumeric, case_mode)),
                'X' => elements.push(MaskElement::Required(CharClass::Any, case_mode)),
                '9' => elements.push(MaskElement::Required(CharClass::Digit, case_mode)),
                'D' => elements.push(MaskElement::Required(CharClass::NonZeroDigit, case_mode)),
                'H' => elements.push(MaskElement::Required(CharClass::Hex, case_mode)),
                'B' => elements.push(MaskElement::Required(CharClass::Binary, case_mode)),
                // Optional characters
                'a' => elements.push(MaskElement::Optional(CharClass::Letter, case_mode)),
                'n' => elements.push(MaskElement::Optional(CharClass::Alphanumeric, case_mode)),
                'x' => elements.push(MaskElement::Optional(CharClass::Any, case_mode)),
                '0' => elements.push(MaskElement::Optional(CharClass::Digit, case_mode)),
                'd' => elements.push(MaskElement::Optional(CharClass::NonZeroDigit, case_mode)),
                '#' => elements.push(MaskElement::Optional(CharClass::DigitOrSign, case_mode)),
                'h' => elements.push(MaskElement::Optional(CharClass::Hex, case_mode)),
                'b' => elements.push(MaskElement::Optional(CharClass::Binary, case_mode)),
                // Reserved for future use (treat as literals for now)
                '[' | ']' | '{' | '}' => {
                    elements.push(MaskElement::Literal(ch));
                }
                // Any other character is a literal separator
                _ => elements.push(MaskElement::Literal(ch)),
            }
        }

        elements
    }

    /// Get the original pattern string.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Get the blank character.
    pub fn blank_char(&self) -> char {
        self.blank_char
    }

    /// Get the number of mask elements.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Check if the mask is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// Get an iterator over the mask elements.
    pub fn elements(&self) -> &[MaskElement] {
        &self.elements
    }

    /// Get the number of editable positions (non-literal).
    pub fn editable_count(&self) -> usize {
        self.elements.iter().filter(|e| e.is_editable()).count()
    }

    /// Get the number of required positions.
    pub fn required_count(&self) -> usize {
        self.elements.iter().filter(|e| e.is_required()).count()
    }

    /// Get the mask element at a given position.
    pub fn element_at(&self, pos: usize) -> Option<&MaskElement> {
        self.elements.get(pos)
    }

    /// Find the next editable position at or after the given position.
    pub fn next_editable_pos(&self, from: usize) -> Option<usize> {
        (from..self.elements.len()).find(|&i| self.elements[i].is_editable())
    }

    /// Find the previous editable position before the given position.
    pub fn prev_editable_pos(&self, from: usize) -> Option<usize> {
        if from == 0 {
            return None;
        }
        (0..from).rev().find(|&i| self.elements[i].is_editable())
    }

    /// Find the first editable position.
    pub fn first_editable_pos(&self) -> Option<usize> {
        self.next_editable_pos(0)
    }

    /// Find the last editable position.
    pub fn last_editable_pos(&self) -> Option<usize> {
        (0..self.elements.len())
            .rev()
            .find(|&i| self.elements[i].is_editable())
    }

    /// Check if a character can be placed at the given mask position.
    pub fn can_place_at(&self, pos: usize, ch: char) -> bool {
        match self.elements.get(pos) {
            Some(elem) => elem.accepts(ch),
            None => false,
        }
    }

    /// Transform a character according to the case mode at the given position.
    pub fn transform_at(&self, pos: usize, ch: char) -> char {
        match self.elements.get(pos) {
            Some(elem) => elem.transform(ch),
            None => ch,
        }
    }

    /// Generate the display text from user input.
    ///
    /// The input should contain only the user-entered characters (no literals).
    /// Returns the full display text with literals and blanks for empty positions.
    pub fn display_text(&self, input: &str) -> String {
        let mut result = String::with_capacity(self.elements.len());
        let mut input_chars = input.chars();

        for element in &self.elements {
            match element {
                MaskElement::Literal(ch) => {
                    result.push(*ch);
                }
                MaskElement::Required(_, _) | MaskElement::Optional(_, _) => {
                    if let Some(ch) = input_chars.next() {
                        result.push(element.transform(ch));
                    } else {
                        result.push(self.blank_char);
                    }
                }
            }
        }

        result
    }

    /// Extract the user input (without literals or blanks) from display text.
    ///
    /// This is the inverse of `display_text()`.
    pub fn extract_input(&self, display: &str) -> String {
        let mut result = String::new();
        let mut display_chars = display.chars();

        for element in &self.elements {
            if let Some(ch) = display_chars.next() {
                match element {
                    MaskElement::Literal(_) => {
                        // Skip literals
                    }
                    MaskElement::Required(_, _) | MaskElement::Optional(_, _) => {
                        if ch != self.blank_char {
                            result.push(ch);
                        }
                    }
                }
            }
        }

        result
    }

    /// Get the text value (input without blanks), stripping trailing optional blanks.
    ///
    /// This is what `LineEdit::text()` should return when a mask is active.
    pub fn text_value(&self, input: &str) -> String {
        // The input already has blanks stripped, so just return it
        input.to_string()
    }

    /// Check if the input satisfies all required positions.
    ///
    /// Returns true if all required positions have been filled.
    pub fn is_complete(&self, input: &str) -> bool {
        let input_len = input.chars().count();
        let mut input_pos = 0;

        for element in &self.elements {
            match element {
                MaskElement::Literal(_) => {}
                MaskElement::Required(_, _) => {
                    if input_pos < input_len {
                        input_pos += 1;
                    } else {
                        // Required position not filled
                        return false;
                    }
                }
                MaskElement::Optional(_, _) => {
                    if input_pos < input_len {
                        input_pos += 1;
                    }
                }
            }
        }

        true
    }

    /// Convert a display position (including literals) to an input position.
    pub fn display_pos_to_input_pos(&self, display_pos: usize) -> usize {
        let mut input_pos = 0;
        for i in 0..display_pos.min(self.elements.len()) {
            if self.elements[i].is_editable() {
                input_pos += 1;
            }
        }
        input_pos
    }

    /// Convert an input position to a display position.
    pub fn input_pos_to_display_pos(&self, input_pos: usize) -> usize {
        let mut remaining = input_pos;
        for (i, element) in self.elements.iter().enumerate() {
            if element.is_editable() {
                if remaining == 0 {
                    return i;
                }
                remaining -= 1;
            }
        }
        // Return position after last element if input_pos exceeds editable count
        self.elements.len()
    }
}

impl fmt::Display for InputMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.pattern)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_mask() {
        let mask = InputMask::new("999-999-9999").unwrap();
        assert_eq!(mask.len(), 12);
        assert_eq!(mask.editable_count(), 10);
        assert_eq!(mask.blank_char(), ' ');
    }

    #[test]
    fn test_parse_with_blank_char() {
        let mask = InputMask::new("000.000.000.000;_").unwrap();
        assert_eq!(mask.blank_char(), '_');
        assert_eq!(mask.editable_count(), 12);
    }

    #[test]
    fn test_char_class_digit() {
        assert!(CharClass::Digit.accepts('0'));
        assert!(CharClass::Digit.accepts('5'));
        assert!(CharClass::Digit.accepts('9'));
        assert!(!CharClass::Digit.accepts('a'));
        assert!(!CharClass::Digit.accepts('A'));
    }

    #[test]
    fn test_char_class_nonzero_digit() {
        assert!(!CharClass::NonZeroDigit.accepts('0'));
        assert!(CharClass::NonZeroDigit.accepts('1'));
        assert!(CharClass::NonZeroDigit.accepts('9'));
    }

    #[test]
    fn test_char_class_hex() {
        assert!(CharClass::Hex.accepts('0'));
        assert!(CharClass::Hex.accepts('a'));
        assert!(CharClass::Hex.accepts('F'));
        assert!(!CharClass::Hex.accepts('g'));
        assert!(!CharClass::Hex.accepts('G'));
    }

    #[test]
    fn test_char_class_letter() {
        assert!(CharClass::Letter.accepts('a'));
        assert!(CharClass::Letter.accepts('Z'));
        assert!(!CharClass::Letter.accepts('5'));
        assert!(!CharClass::Letter.accepts(' '));
    }

    #[test]
    fn test_display_text_phone() {
        let mask = InputMask::new("(999) 999-9999").unwrap();

        // Empty input
        assert_eq!(mask.display_text(""), "(   )    -    ");

        // Partial input
        assert_eq!(mask.display_text("555"), "(555)    -    ");

        // Full input
        assert_eq!(mask.display_text("5551234567"), "(555) 123-4567");
    }

    #[test]
    fn test_display_text_with_blank() {
        let mask = InputMask::new("000.000.000.000;_").unwrap();
        assert_eq!(mask.display_text(""), "___.___.___.___");
        assert_eq!(mask.display_text("192168"), "192.168.___.___");
    }

    #[test]
    fn test_extract_input() {
        let mask = InputMask::new("(999) 999-9999").unwrap();
        assert_eq!(mask.extract_input("(555) 123-4567"), "5551234567");
        assert_eq!(mask.extract_input("(555)    -    "), "555");
    }

    #[test]
    fn test_case_conversion_upper() {
        let mask = InputMask::new(">AAAAA").unwrap();
        assert_eq!(mask.display_text("hello"), "HELLO");
    }

    #[test]
    fn test_case_conversion_lower() {
        let mask = InputMask::new("<AAAAA").unwrap();
        assert_eq!(mask.display_text("HELLO"), "hello");
    }

    #[test]
    fn test_case_conversion_mixed() {
        let mask = InputMask::new(">AA!AA<AA").unwrap();
        assert_eq!(mask.display_text("aaBBcc"), "AABBcc");
    }

    #[test]
    fn test_is_complete() {
        let mask = InputMask::new("999-999").unwrap();
        assert!(!mask.is_complete(""));
        assert!(!mask.is_complete("123"));
        assert!(mask.is_complete("123456"));
    }

    #[test]
    fn test_is_complete_with_optional() {
        // Mask with optional at the end: "99-990" means 2 required, literal, 2 required, 1 optional
        // Input fills positions in order, so optional positions consume input if available
        let mask = InputMask::new("99-990").unwrap();
        assert!(!mask.is_complete(""));
        assert!(!mask.is_complete("1"));
        assert!(!mask.is_complete("12")); // Only fills first 2 positions
        assert!(!mask.is_complete("123")); // Fills 2 + 1 on second side
        assert!(mask.is_complete("1234")); // Fills 2 + 2, all required done
        assert!(mask.is_complete("12345")); // All filled including optional

        // All-optional mask is always complete
        let mask2 = InputMask::new("000").unwrap();
        assert!(mask2.is_complete(""));
        assert!(mask2.is_complete("1"));
        assert!(mask2.is_complete("123"));
    }

    #[test]
    fn test_editable_positions() {
        let mask = InputMask::new("(999) 999-9999").unwrap();

        assert_eq!(mask.first_editable_pos(), Some(1)); // After '('
        assert_eq!(mask.next_editable_pos(0), Some(1));
        assert_eq!(mask.next_editable_pos(4), Some(6)); // After ') '
        assert_eq!(mask.prev_editable_pos(6), Some(3)); // Before ') '
    }

    #[test]
    fn test_escaped_characters() {
        // Escape special chars to use as literals
        let mask = InputMask::new("\\9\\A99").unwrap();
        assert_eq!(mask.len(), 4);
        // First two are literals '9' and 'A'
        assert!(mask.elements[0].is_literal());
        assert!(mask.elements[1].is_literal());
        // Last two are digit requirements
        assert!(mask.elements[2].is_required());
        assert!(mask.elements[3].is_required());
    }

    #[test]
    fn test_position_conversion() {
        let mask = InputMask::new("(999) 999").unwrap();

        // Display pos 0 is '(' literal, input pos 0
        assert_eq!(mask.display_pos_to_input_pos(0), 0);
        // Display pos 1 is first editable, input pos 0
        assert_eq!(mask.display_pos_to_input_pos(1), 0);
        // Display pos 4 is ')' literal, input pos 3
        assert_eq!(mask.display_pos_to_input_pos(4), 3);

        // Input pos 0 -> display pos 1 (first editable)
        assert_eq!(mask.input_pos_to_display_pos(0), 1);
        // Input pos 3 -> display pos 6 (after ") ")
        assert_eq!(mask.input_pos_to_display_pos(3), 6);
    }

    #[test]
    fn test_empty_mask() {
        assert!(InputMask::new("").is_none());
        assert!(InputMask::new("; ").is_none());
    }

    #[test]
    fn test_hex_mask() {
        let mask = InputMask::new("HH:HH:HH;_").unwrap();
        assert_eq!(mask.display_text(""), "__:__:__");
        assert_eq!(mask.display_text("AABBCC"), "AA:BB:CC");
    }

    #[test]
    fn test_binary_mask() {
        let mask = InputMask::new("BBBB BBBB").unwrap();
        assert_eq!(mask.display_text("01011010"), "0101 1010");
    }
}
