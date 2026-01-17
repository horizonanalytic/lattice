//! Input context for text widgets.
//!
//! This module provides per-widget input configuration including input hints
//! that help the platform provide appropriate input assistance (keyboard layout,
//! autocomplete suggestions, IME behavior).
//!
//! # Overview
//!
//! Each text input widget can have an `InputContext` that describes:
//! - What type of content is being entered (email, URL, password, etc.)
//! - Whether autocomplete should be enabled
//! - Whether spellcheck should be enabled
//! - IME-related settings
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice::widget::input_context::{InputContext, InputHints, ContentType};
//!
//! // Create an input context for email input
//! let context = InputContext::new()
//!     .with_content_type(ContentType::Email)
//!     .with_autocomplete(true);
//!
//! // Apply to a text widget
//! text_edit.set_input_context(context);
//! ```
//!
//! # Platform Integration
//!
//! Input hints are used to:
//! - Configure virtual keyboard layout on mobile devices
//! - Provide autocomplete suggestions
//! - Adjust IME behavior
//! - Enable/disable spellcheck

use horizon_lattice_render::{Point, Size};

use super::ime::ImePurpose;

/// The type of content being entered.
///
/// This hint helps the platform provide appropriate input assistance,
/// such as showing an email-optimized keyboard or providing URL autocomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum ContentType {
    /// Normal text input (default).
    #[default]
    Normal,
    /// Password input - may hide characters and disable autocomplete.
    Password,
    /// Email address input - may show @ and .com keys.
    Email,
    /// URL input - may show .com, /, and other URL-related keys.
    Url,
    /// Numeric input - may show numeric keypad.
    Number,
    /// Phone number input - may show phone keypad.
    Phone,
    /// Date input.
    Date,
    /// Time input.
    Time,
    /// Multi-line text input.
    Multiline,
    /// Search input - may show search key.
    Search,
    /// Username input.
    Username,
    /// Credit card number input.
    CreditCard,
}

impl ContentType {
    /// Check if this content type should disable autocomplete.
    pub fn should_disable_autocomplete(&self) -> bool {
        matches!(
            self,
            ContentType::Password | ContentType::CreditCard | ContentType::Username
        )
    }

    /// Check if this content type should disable spellcheck.
    pub fn should_disable_spellcheck(&self) -> bool {
        matches!(
            self,
            ContentType::Password
                | ContentType::Email
                | ContentType::Url
                | ContentType::Number
                | ContentType::Phone
                | ContentType::CreditCard
                | ContentType::Username
        )
    }

    /// Get the corresponding IME purpose for this content type.
    pub fn ime_purpose(&self) -> ImePurpose {
        match self {
            ContentType::Password => ImePurpose::Password,
            ContentType::Normal
            | ContentType::Email
            | ContentType::Url
            | ContentType::Number
            | ContentType::Phone
            | ContentType::Date
            | ContentType::Time
            | ContentType::Multiline
            | ContentType::Search
            | ContentType::Username
            | ContentType::CreditCard => ImePurpose::Normal,
        }
    }
}

/// Capitalization behavior for text input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Capitalization {
    /// No automatic capitalization.
    #[default]
    None,
    /// Capitalize the first letter of each word.
    Words,
    /// Capitalize the first letter of each sentence.
    Sentences,
    /// Capitalize all letters.
    AllCharacters,
}

/// Input hints that describe the expected input behavior.
///
/// These hints are used by the platform to provide appropriate input
/// assistance, such as virtual keyboard configuration and autocomplete.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputHints {
    /// The type of content being entered.
    pub content_type: ContentType,
    /// Whether autocomplete should be enabled.
    pub autocomplete: bool,
    /// Whether spellcheck should be enabled.
    pub spellcheck: bool,
    /// Capitalization behavior.
    pub capitalization: Capitalization,
    /// Whether the input accepts multiple lines.
    pub multiline: bool,
    /// Whether the input is read-only.
    pub read_only: bool,
}

impl InputHints {
    /// Create default input hints for normal text input.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create input hints for password input.
    pub fn password() -> Self {
        Self {
            content_type: ContentType::Password,
            autocomplete: false,
            spellcheck: false,
            capitalization: Capitalization::None,
            multiline: false,
            read_only: false,
        }
    }

    /// Create input hints for email input.
    pub fn email() -> Self {
        Self {
            content_type: ContentType::Email,
            autocomplete: true,
            spellcheck: false,
            capitalization: Capitalization::None,
            multiline: false,
            read_only: false,
        }
    }

    /// Create input hints for URL input.
    pub fn url() -> Self {
        Self {
            content_type: ContentType::Url,
            autocomplete: true,
            spellcheck: false,
            capitalization: Capitalization::None,
            multiline: false,
            read_only: false,
        }
    }

    /// Create input hints for numeric input.
    pub fn number() -> Self {
        Self {
            content_type: ContentType::Number,
            autocomplete: false,
            spellcheck: false,
            capitalization: Capitalization::None,
            multiline: false,
            read_only: false,
        }
    }

    /// Create input hints for search input.
    pub fn search() -> Self {
        Self {
            content_type: ContentType::Search,
            autocomplete: true,
            spellcheck: true,
            capitalization: Capitalization::None,
            multiline: false,
            read_only: false,
        }
    }

    /// Create input hints for multiline text.
    pub fn multiline() -> Self {
        Self {
            content_type: ContentType::Multiline,
            autocomplete: false,
            spellcheck: true,
            capitalization: Capitalization::Sentences,
            multiline: true,
            read_only: false,
        }
    }

    /// Set the content type.
    pub fn with_content_type(mut self, content_type: ContentType) -> Self {
        self.content_type = content_type;
        self
    }

    /// Set whether autocomplete is enabled.
    pub fn with_autocomplete(mut self, autocomplete: bool) -> Self {
        self.autocomplete = autocomplete;
        self
    }

    /// Set whether spellcheck is enabled.
    pub fn with_spellcheck(mut self, spellcheck: bool) -> Self {
        self.spellcheck = spellcheck;
        self
    }

    /// Set the capitalization behavior.
    pub fn with_capitalization(mut self, capitalization: Capitalization) -> Self {
        self.capitalization = capitalization;
        self
    }

    /// Set whether the input accepts multiple lines.
    pub fn with_multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }

    /// Set whether the input is read-only.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }
}

/// The cursor area for IME positioning.
///
/// This describes where the IME candidate window should be positioned
/// relative to the text being edited.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ImeCursorArea {
    /// The position of the cursor in window coordinates.
    pub position: Point,
    /// The size of the editing area (typically cursor height and width).
    pub size: Size,
}

impl ImeCursorArea {
    /// Create a new IME cursor area.
    pub fn new(position: Point, size: Size) -> Self {
        Self { position, size }
    }

    /// Create an IME cursor area from coordinates.
    pub fn from_coords(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            position: Point::new(x, y),
            size: Size::new(width, height),
        }
    }
}

/// Per-widget input context for text input.
///
/// The input context provides configuration for text input including
/// input hints, IME settings, and cursor positioning.
#[derive(Debug, Clone, Default)]
pub struct InputContext {
    /// Input hints describing the expected content.
    pub hints: InputHints,
    /// The current IME cursor area for candidate window positioning.
    pub cursor_area: Option<ImeCursorArea>,
    /// Whether IME should be enabled for this widget.
    pub ime_enabled: bool,
}

impl InputContext {
    /// Create a new input context with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an input context for password input.
    pub fn password() -> Self {
        Self {
            hints: InputHints::password(),
            cursor_area: None,
            ime_enabled: false, // IME typically disabled for passwords
        }
    }

    /// Create an input context for normal text input with IME.
    pub fn text() -> Self {
        Self {
            hints: InputHints::new(),
            cursor_area: None,
            ime_enabled: true,
        }
    }

    /// Set the input hints.
    pub fn with_hints(mut self, hints: InputHints) -> Self {
        self.hints = hints;
        self
    }

    /// Set the content type.
    pub fn with_content_type(mut self, content_type: ContentType) -> Self {
        self.hints.content_type = content_type;
        self
    }

    /// Set whether autocomplete is enabled.
    pub fn with_autocomplete(mut self, autocomplete: bool) -> Self {
        self.hints.autocomplete = autocomplete;
        self
    }

    /// Set whether IME is enabled.
    pub fn with_ime_enabled(mut self, enabled: bool) -> Self {
        self.ime_enabled = enabled;
        self
    }

    /// Set the IME cursor area.
    pub fn with_cursor_area(mut self, area: ImeCursorArea) -> Self {
        self.cursor_area = Some(area);
        self
    }

    /// Update the cursor area position.
    pub fn set_cursor_area(&mut self, area: ImeCursorArea) {
        self.cursor_area = Some(area);
    }

    /// Clear the cursor area.
    pub fn clear_cursor_area(&mut self) {
        self.cursor_area = None;
    }

    /// Get the IME purpose based on the content type.
    pub fn ime_purpose(&self) -> ImePurpose {
        self.hints.content_type.ime_purpose()
    }

    /// Check if this is a password input context.
    pub fn is_password(&self) -> bool {
        matches!(self.hints.content_type, ContentType::Password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_default() {
        let content_type = ContentType::default();
        assert_eq!(content_type, ContentType::Normal);
    }

    #[test]
    fn test_content_type_autocomplete() {
        assert!(!ContentType::Normal.should_disable_autocomplete());
        assert!(ContentType::Password.should_disable_autocomplete());
        assert!(!ContentType::Email.should_disable_autocomplete());
        assert!(ContentType::CreditCard.should_disable_autocomplete());
    }

    #[test]
    fn test_content_type_spellcheck() {
        assert!(!ContentType::Normal.should_disable_spellcheck());
        assert!(ContentType::Password.should_disable_spellcheck());
        assert!(ContentType::Email.should_disable_spellcheck());
        assert!(ContentType::Url.should_disable_spellcheck());
    }

    #[test]
    fn test_content_type_ime_purpose() {
        assert_eq!(ContentType::Normal.ime_purpose(), ImePurpose::Normal);
        assert_eq!(ContentType::Password.ime_purpose(), ImePurpose::Password);
        assert_eq!(ContentType::Email.ime_purpose(), ImePurpose::Normal);
    }

    #[test]
    fn test_input_hints_default() {
        let hints = InputHints::new();
        assert_eq!(hints.content_type, ContentType::Normal);
        assert!(!hints.autocomplete);
        assert!(!hints.spellcheck);
    }

    #[test]
    fn test_input_hints_password() {
        let hints = InputHints::password();
        assert_eq!(hints.content_type, ContentType::Password);
        assert!(!hints.autocomplete);
        assert!(!hints.spellcheck);
    }

    #[test]
    fn test_input_hints_email() {
        let hints = InputHints::email();
        assert_eq!(hints.content_type, ContentType::Email);
        assert!(hints.autocomplete);
        assert!(!hints.spellcheck);
    }

    #[test]
    fn test_input_hints_builder() {
        let hints = InputHints::new()
            .with_content_type(ContentType::Search)
            .with_autocomplete(true)
            .with_spellcheck(true)
            .with_capitalization(Capitalization::Words);

        assert_eq!(hints.content_type, ContentType::Search);
        assert!(hints.autocomplete);
        assert!(hints.spellcheck);
        assert_eq!(hints.capitalization, Capitalization::Words);
    }

    #[test]
    fn test_ime_cursor_area() {
        let area = ImeCursorArea::new(Point::new(100.0, 200.0), Size::new(10.0, 20.0));
        assert_eq!(area.position.x, 100.0);
        assert_eq!(area.position.y, 200.0);
        assert_eq!(area.size.width, 10.0);
        assert_eq!(area.size.height, 20.0);
    }

    #[test]
    fn test_ime_cursor_area_from_coords() {
        let area = ImeCursorArea::from_coords(50.0, 75.0, 5.0, 15.0);
        assert_eq!(area.position.x, 50.0);
        assert_eq!(area.position.y, 75.0);
        assert_eq!(area.size.width, 5.0);
        assert_eq!(area.size.height, 15.0);
    }

    #[test]
    fn test_input_context_default() {
        let context = InputContext::new();
        assert!(!context.ime_enabled);
        assert!(context.cursor_area.is_none());
    }

    #[test]
    fn test_input_context_password() {
        let context = InputContext::password();
        assert!(!context.ime_enabled);
        assert!(context.is_password());
    }

    #[test]
    fn test_input_context_text() {
        let context = InputContext::text();
        assert!(context.ime_enabled);
        assert!(!context.is_password());
    }

    #[test]
    fn test_input_context_builder() {
        let area = ImeCursorArea::from_coords(10.0, 20.0, 5.0, 15.0);
        let context = InputContext::new()
            .with_content_type(ContentType::Email)
            .with_autocomplete(true)
            .with_ime_enabled(true)
            .with_cursor_area(area);

        assert_eq!(context.hints.content_type, ContentType::Email);
        assert!(context.hints.autocomplete);
        assert!(context.ime_enabled);
        assert!(context.cursor_area.is_some());
    }

    #[test]
    fn test_input_context_set_cursor_area() {
        let mut context = InputContext::new();
        assert!(context.cursor_area.is_none());

        context.set_cursor_area(ImeCursorArea::from_coords(10.0, 20.0, 5.0, 15.0));
        assert!(context.cursor_area.is_some());

        context.clear_cursor_area();
        assert!(context.cursor_area.is_none());
    }

    #[test]
    fn test_capitalization() {
        assert_eq!(Capitalization::default(), Capitalization::None);
    }
}
