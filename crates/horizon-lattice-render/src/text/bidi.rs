//! Bidirectional text support for internationalization.
//!
//! This module provides automatic text direction detection and explicit
//! direction override support using the Unicode Bidirectional Algorithm (UBA).
//!
//! # Overview
//!
//! Text direction handling is important for proper display of:
//! - Right-to-left (RTL) scripts like Arabic, Hebrew, Persian
//! - Mixed directional text (e.g., English with Arabic words)
//! - Neutral characters that need direction context
//!
//! # Example
//!
//! ```no_run
//! use horizon_lattice_render::text::{TextDirection, detect_base_direction};
//!
//! // Auto-detect direction from content
//! let ltr_text = "Hello, World!";
//! assert_eq!(detect_base_direction(ltr_text), TextDirection::LeftToRight);
//!
//! let rtl_text = "مرحبا بالعالم";
//! assert_eq!(detect_base_direction(rtl_text), TextDirection::RightToLeft);
//!
//! // Explicit direction override
//! let direction = TextDirection::RightToLeft;
//! ```

use unicode_bidi::{bidi_class, BidiClass};

/// Text direction for layout and rendering.
///
/// This enum represents the base direction of text, which affects:
/// - Default text alignment
/// - Paragraph direction for bidirectional text
/// - Cursor movement direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TextDirection {
    /// Left-to-right direction (default for Latin, Cyrillic, Greek, etc.).
    #[default]
    LeftToRight,
    /// Right-to-left direction (for Arabic, Hebrew, etc.).
    RightToLeft,
    /// Automatically detect direction from text content.
    ///
    /// Uses the Unicode Bidirectional Algorithm to determine the base
    /// direction from the first strong directional character.
    Auto,
}

impl TextDirection {
    /// Check if this direction is left-to-right.
    pub fn is_ltr(self) -> bool {
        matches!(self, TextDirection::LeftToRight)
    }

    /// Check if this direction is right-to-left.
    pub fn is_rtl(self) -> bool {
        matches!(self, TextDirection::RightToLeft)
    }

    /// Check if this direction is auto-detected.
    pub fn is_auto(self) -> bool {
        matches!(self, TextDirection::Auto)
    }

    /// Resolve auto direction to a concrete direction based on text content.
    ///
    /// If this is `Auto`, detects the direction from the given text.
    /// Otherwise, returns the explicit direction.
    pub fn resolve(self, text: &str) -> TextDirection {
        match self {
            TextDirection::Auto => detect_base_direction(text),
            dir => dir,
        }
    }

    /// Get the bidi level for this direction.
    ///
    /// LTR has level 0, RTL has level 1.
    pub fn bidi_level(self) -> u8 {
        match self {
            TextDirection::LeftToRight | TextDirection::Auto => 0,
            TextDirection::RightToLeft => 1,
        }
    }
}

/// Detect the base direction of text using the Unicode Bidi Algorithm.
///
/// This function implements the first-strong algorithm (P2/P3 of UAX #9):
/// The base direction is determined by the first character with a strong
/// directional type (L, R, or AL).
///
/// # Returns
///
/// - `TextDirection::RightToLeft` if the first strong character is R or AL
/// - `TextDirection::LeftToRight` otherwise (including empty strings)
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_render::text::{TextDirection, detect_base_direction};
///
/// // English text (LTR)
/// assert_eq!(detect_base_direction("Hello"), TextDirection::LeftToRight);
///
/// // Arabic text (RTL)
/// assert_eq!(detect_base_direction("مرحبا"), TextDirection::RightToLeft);
///
/// // Hebrew text (RTL)
/// assert_eq!(detect_base_direction("שלום"), TextDirection::RightToLeft);
///
/// // Numbers and punctuation only - defaults to LTR
/// assert_eq!(detect_base_direction("123!@#"), TextDirection::LeftToRight);
///
/// // Mixed: first strong character determines direction
/// assert_eq!(detect_base_direction("Hello مرحبا"), TextDirection::LeftToRight);
/// assert_eq!(detect_base_direction("مرحبا Hello"), TextDirection::RightToLeft);
/// ```
pub fn detect_base_direction(text: &str) -> TextDirection {
    for c in text.chars() {
        match bidi_class(c) {
            // Strong LTR
            BidiClass::L => return TextDirection::LeftToRight,
            // Strong RTL
            BidiClass::R | BidiClass::AL => return TextDirection::RightToLeft,
            // Continue searching for other classes
            _ => continue,
        }
    }

    // Default to LTR if no strong directional character found
    TextDirection::LeftToRight
}

/// Check if a character is a strong RTL character.
///
/// This includes Arabic Letter (AL) and Right-to-Left (R) bidi classes.
pub fn is_rtl_char(c: char) -> bool {
    matches!(bidi_class(c), BidiClass::R | BidiClass::AL)
}

/// Check if a character is a strong LTR character.
///
/// This includes Left-to-Right (L) bidi class.
pub fn is_ltr_char(c: char) -> bool {
    matches!(bidi_class(c), BidiClass::L)
}

/// Check if a character has strong directionality.
///
/// Strong directional characters are L, R, and AL.
pub fn is_strong_directional(c: char) -> bool {
    matches!(bidi_class(c), BidiClass::L | BidiClass::R | BidiClass::AL)
}

/// Check if text contains any RTL characters.
///
/// This is useful for detecting if special bidi processing is needed.
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_render::text::contains_rtl;
///
/// assert!(!contains_rtl("Hello, World!"));
/// assert!(contains_rtl("Hello مرحبا"));
/// assert!(contains_rtl("שלום"));
/// ```
pub fn contains_rtl(text: &str) -> bool {
    text.chars().any(is_rtl_char)
}

/// Explicit direction isolate markers for bidi text.
///
/// These can be used to override the natural direction of text segments.
pub mod isolates {
    /// Left-to-Right Isolate (U+2066)
    pub const LRI: char = '\u{2066}';
    /// Right-to-Left Isolate (U+2067)
    pub const RLI: char = '\u{2067}';
    /// First Strong Isolate (U+2068)
    pub const FSI: char = '\u{2068}';
    /// Pop Directional Isolate (U+2069)
    pub const PDI: char = '\u{2069}';
}

/// Explicit direction override markers for bidi text.
///
/// These can be used to force the direction of text segments.
/// Use with caution as they can produce unexpected results.
pub mod overrides {
    /// Left-to-Right Override (U+202D)
    pub const LRO: char = '\u{202D}';
    /// Right-to-Left Override (U+202E)
    pub const RLO: char = '\u{202E}';
    /// Pop Directional Formatting (U+202C)
    pub const PDF: char = '\u{202C}';
    /// Left-to-Right Embedding (U+202A)
    pub const LRE: char = '\u{202A}';
    /// Right-to-Left Embedding (U+202B)
    pub const RLE: char = '\u{202B}';
}

/// Wrap text with directional isolate markers.
///
/// This is useful for ensuring a text segment maintains its natural
/// direction regardless of surrounding context.
///
/// # Example
///
/// ```no_run
/// use horizon_lattice_render::text::{TextDirection, wrap_with_direction};
///
/// // Force LTR direction
/// let isolated = wrap_with_direction("product-123", TextDirection::LeftToRight);
/// assert!(isolated.starts_with('\u{2066}')); // LRI
/// assert!(isolated.ends_with('\u{2069}')); // PDI
/// ```
pub fn wrap_with_direction(text: &str, direction: TextDirection) -> String {
    let start = match direction {
        TextDirection::LeftToRight => isolates::LRI,
        TextDirection::RightToLeft => isolates::RLI,
        TextDirection::Auto => isolates::FSI,
    };
    format!("{}{}{}", start, text, isolates::PDI)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_detection_ltr() {
        assert_eq!(detect_base_direction("Hello"), TextDirection::LeftToRight);
        assert_eq!(
            detect_base_direction("Hello, World!"),
            TextDirection::LeftToRight
        );
        assert_eq!(detect_base_direction("Привет"), TextDirection::LeftToRight); // Russian (Cyrillic)
        assert_eq!(detect_base_direction("Γεια"), TextDirection::LeftToRight); // Greek
        assert_eq!(detect_base_direction("你好"), TextDirection::LeftToRight); // Chinese
    }

    #[test]
    fn direction_detection_rtl() {
        assert_eq!(detect_base_direction("مرحبا"), TextDirection::RightToLeft); // Arabic
        assert_eq!(detect_base_direction("שלום"), TextDirection::RightToLeft); // Hebrew
        assert_eq!(detect_base_direction("سلام"), TextDirection::RightToLeft); // Persian/Arabic
    }

    #[test]
    fn direction_detection_mixed() {
        // First strong character determines base direction
        assert_eq!(
            detect_base_direction("Hello مرحبا"),
            TextDirection::LeftToRight
        );
        assert_eq!(
            detect_base_direction("مرحبا Hello"),
            TextDirection::RightToLeft
        );
        assert_eq!(
            detect_base_direction("123 Hello"),
            TextDirection::LeftToRight
        );
        assert_eq!(
            detect_base_direction("123 مرحبا"),
            TextDirection::RightToLeft
        );
    }

    #[test]
    fn direction_detection_neutral_only() {
        // No strong directional characters - defaults to LTR
        assert_eq!(detect_base_direction("123"), TextDirection::LeftToRight);
        assert_eq!(detect_base_direction("!@#$%"), TextDirection::LeftToRight);
        assert_eq!(detect_base_direction("   "), TextDirection::LeftToRight);
        assert_eq!(detect_base_direction(""), TextDirection::LeftToRight);
    }

    #[test]
    fn direction_resolve() {
        let dir = TextDirection::Auto;
        assert_eq!(dir.resolve("Hello"), TextDirection::LeftToRight);
        assert_eq!(dir.resolve("مرحبا"), TextDirection::RightToLeft);

        let explicit_ltr = TextDirection::LeftToRight;
        assert_eq!(explicit_ltr.resolve("مرحبا"), TextDirection::LeftToRight);

        let explicit_rtl = TextDirection::RightToLeft;
        assert_eq!(explicit_rtl.resolve("Hello"), TextDirection::RightToLeft);
    }

    #[test]
    fn contains_rtl_detection() {
        assert!(!contains_rtl("Hello, World!"));
        assert!(!contains_rtl("123"));
        assert!(contains_rtl("مرحبا"));
        assert!(contains_rtl("Hello مرحبا"));
        assert!(contains_rtl("שלום"));
    }

    #[test]
    fn wrap_direction_isolates() {
        let ltr = wrap_with_direction("test", TextDirection::LeftToRight);
        assert!(ltr.starts_with(isolates::LRI));
        assert!(ltr.ends_with(isolates::PDI));

        let rtl = wrap_with_direction("test", TextDirection::RightToLeft);
        assert!(rtl.starts_with(isolates::RLI));
        assert!(rtl.ends_with(isolates::PDI));

        let auto = wrap_with_direction("test", TextDirection::Auto);
        assert!(auto.starts_with(isolates::FSI));
        assert!(auto.ends_with(isolates::PDI));
    }

    #[test]
    fn bidi_level() {
        assert_eq!(TextDirection::LeftToRight.bidi_level(), 0);
        assert_eq!(TextDirection::RightToLeft.bidi_level(), 1);
        assert_eq!(TextDirection::Auto.bidi_level(), 0); // Auto defaults to LTR level
    }

    #[test]
    fn direction_checks() {
        assert!(TextDirection::LeftToRight.is_ltr());
        assert!(!TextDirection::LeftToRight.is_rtl());
        assert!(!TextDirection::LeftToRight.is_auto());

        assert!(!TextDirection::RightToLeft.is_ltr());
        assert!(TextDirection::RightToLeft.is_rtl());
        assert!(!TextDirection::RightToLeft.is_auto());

        assert!(!TextDirection::Auto.is_ltr());
        assert!(!TextDirection::Auto.is_rtl());
        assert!(TextDirection::Auto.is_auto());
    }
}
