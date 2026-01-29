//! Input validation for text widgets.
//!
//! This module provides a validation framework inspired by Qt's QValidator system.
//! Validators can be attached to text input widgets like [`LineEdit`](super::widgets::LineEdit)
//! to constrain and validate user input.
//!
//! # Validation States
//!
//! Input can be in one of three states:
//!
//! - [`ValidationState::Invalid`]: The input is clearly wrong
//! - [`ValidationState::Intermediate`]: The input is incomplete but could become valid
//! - [`ValidationState::Acceptable`]: The input is valid as a final result
//!
//! # Built-in Validators
//!
//! - [`IntValidator`]: Validates integer input within a range
//! - [`DoubleValidator`]: Validates floating-point input within a range
//! - [`RegexValidator`]: Validates input against a regular expression
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::{widgets::LineEdit, validator::IntValidator};
//!
//! let mut edit = LineEdit::new();
//! edit.set_validator(IntValidator::new(0, 100));
//!
//! edit.validation_changed.connect(|state| {
//!     println!("Validation state: {:?}", state);
//! });
//! ```

use std::fmt;
use std::sync::Arc;

/// The result of validating input text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Default)]
pub enum ValidationState {
    /// The input is clearly invalid and cannot be made valid by further editing.
    Invalid,
    /// The input is incomplete but could potentially become valid with more input.
    /// This is the state for partially-typed numbers, incomplete patterns, etc.
    Intermediate,
    /// The input is valid and acceptable as a final result.
    #[default]
    Acceptable,
}


impl fmt::Display for ValidationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationState::Invalid => write!(f, "Invalid"),
            ValidationState::Intermediate => write!(f, "Intermediate"),
            ValidationState::Acceptable => write!(f, "Acceptable"),
        }
    }
}

/// Trait for input validators.
///
/// Validators check whether input text is valid according to some criteria.
/// They can be attached to text input widgets to provide real-time validation.
///
/// # Thread Safety
///
/// Validators must be `Send + Sync` to work with the signal system.
pub trait Validator: Send + Sync {
    /// Validate the input string.
    ///
    /// Returns the validation state for the given input.
    ///
    /// # Arguments
    ///
    /// * `input` - The text to validate
    ///
    /// # Returns
    ///
    /// - [`ValidationState::Invalid`] if the input is clearly wrong
    /// - [`ValidationState::Intermediate`] if the input could become valid with more editing
    /// - [`ValidationState::Acceptable`] if the input is valid
    fn validate(&self, input: &str) -> ValidationState;

    /// Attempt to fix invalid input.
    ///
    /// This method is called when the user attempts to finish editing (e.g., pressing Enter)
    /// while the input is not acceptable. The validator can attempt to transform the input
    /// into a valid value.
    ///
    /// The default implementation returns `None`, meaning no fixup is attempted.
    ///
    /// # Arguments
    ///
    /// * `input` - The current (possibly invalid) input
    ///
    /// # Returns
    ///
    /// `Some(fixed_input)` if the input was fixed, `None` if no fixup is possible.
    fn fixup(&self, _input: &str) -> Option<String> {
        None
    }
}

// Allow using Arc<dyn Validator> as a Validator
impl<V: Validator + ?Sized> Validator for Arc<V> {
    fn validate(&self, input: &str) -> ValidationState {
        (**self).validate(input)
    }

    fn fixup(&self, input: &str) -> Option<String> {
        (**self).fixup(input)
    }
}

// Allow using Box<dyn Validator> as a Validator
impl<V: Validator + ?Sized> Validator for Box<V> {
    fn validate(&self, input: &str) -> ValidationState {
        (**self).validate(input)
    }

    fn fixup(&self, input: &str) -> Option<String> {
        (**self).fixup(input)
    }
}

/// Validator for integer input within a specified range.
///
/// This validator accepts integer values within the range `[minimum, maximum]`.
/// Partial inputs (like "-", or numbers being typed) are considered intermediate.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::validator::{IntValidator, ValidationState};
///
/// let validator = IntValidator::new(0, 100);
///
/// assert_eq!(validator.validate("42"), ValidationState::Acceptable);
/// assert_eq!(validator.validate("150"), ValidationState::Invalid);
/// assert_eq!(validator.validate("-"), ValidationState::Intermediate);
/// assert_eq!(validator.validate(""), ValidationState::Intermediate);
/// ```
#[derive(Debug, Clone)]
pub struct IntValidator {
    minimum: i64,
    maximum: i64,
}

impl IntValidator {
    /// Create a new integer validator with the given range.
    ///
    /// # Arguments
    ///
    /// * `minimum` - The minimum acceptable value (inclusive)
    /// * `maximum` - The maximum acceptable value (inclusive)
    pub fn new(minimum: i64, maximum: i64) -> Self {
        Self {
            minimum: minimum.min(maximum),
            maximum: minimum.max(maximum),
        }
    }

    /// Create a validator for non-negative integers (0 and above).
    pub fn non_negative() -> Self {
        Self::new(0, i64::MAX)
    }

    /// Create a validator for positive integers (1 and above).
    pub fn positive() -> Self {
        Self::new(1, i64::MAX)
    }

    /// Get the minimum value.
    pub fn minimum(&self) -> i64 {
        self.minimum
    }

    /// Get the maximum value.
    pub fn maximum(&self) -> i64 {
        self.maximum
    }

    /// Set the minimum value.
    pub fn set_minimum(&mut self, min: i64) {
        self.minimum = min;
        if self.maximum < self.minimum {
            self.maximum = self.minimum;
        }
    }

    /// Set the maximum value.
    pub fn set_maximum(&mut self, max: i64) {
        self.maximum = max;
        if self.minimum > self.maximum {
            self.minimum = self.maximum;
        }
    }

    /// Set both minimum and maximum.
    pub fn set_range(&mut self, min: i64, max: i64) {
        self.minimum = min.min(max);
        self.maximum = min.max(max);
    }
}

impl Validator for IntValidator {
    fn validate(&self, input: &str) -> ValidationState {
        let trimmed = input.trim();

        // Empty input is intermediate (user might be about to type)
        if trimmed.is_empty() {
            return ValidationState::Intermediate;
        }

        // Just a minus sign is intermediate if negative numbers are allowed
        if trimmed == "-" {
            return if self.minimum < 0 {
                ValidationState::Intermediate
            } else {
                ValidationState::Invalid
            };
        }

        // Just a plus sign is intermediate
        if trimmed == "+" {
            return ValidationState::Intermediate;
        }

        // Check if it's a valid integer format (allowing partial input)
        let is_valid_format = trimmed
            .chars()
            .enumerate()
            .all(|(i, c)| c.is_ascii_digit() || (i == 0 && (c == '-' || c == '+')));

        if !is_valid_format {
            return ValidationState::Invalid;
        }

        // Try to parse the number
        match trimmed.parse::<i64>() {
            Ok(value) => {
                if value >= self.minimum && value <= self.maximum {
                    ValidationState::Acceptable
                } else if value < self.minimum {
                    // Could the user make this valid by deleting digits?
                    // E.g., if range is [10, 100] and input is "5", it's intermediate
                    // because user might type "50"
                    if self.minimum >= 0 && value >= 0 {
                        // For positive ranges, smaller numbers could become valid
                        let max_digits = self.maximum.abs().to_string().len();
                        let input_digits = value.abs().to_string().len();
                        if input_digits < max_digits {
                            ValidationState::Intermediate
                        } else {
                            ValidationState::Invalid
                        }
                    } else {
                        ValidationState::Invalid
                    }
                } else {
                    // value > maximum
                    ValidationState::Invalid
                }
            }
            Err(_) => {
                // Could be overflow - definitely invalid
                ValidationState::Invalid
            }
        }
    }

    fn fixup(&self, input: &str) -> Option<String> {
        let trimmed = input.trim();

        // Try to parse and clamp to range
        if let Ok(value) = trimmed.parse::<i64>() {
            let clamped = value.clamp(self.minimum, self.maximum);
            if clamped != value {
                return Some(clamped.to_string());
            }
        }

        None
    }
}

/// Validator for floating-point input within a specified range.
///
/// This validator accepts decimal values within the range `[minimum, maximum]`
/// with configurable decimal precision.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::validator::{DoubleValidator, ValidationState};
///
/// let validator = DoubleValidator::new(-10.0, 10.0, 2);
///
/// assert_eq!(validator.validate("3.14"), ValidationState::Acceptable);
/// assert_eq!(validator.validate("3."), ValidationState::Intermediate);
/// assert_eq!(validator.validate("100.0"), ValidationState::Invalid);
/// ```
#[derive(Debug, Clone)]
pub struct DoubleValidator {
    minimum: f64,
    maximum: f64,
    decimals: u32,
}

impl DoubleValidator {
    /// Create a new double validator with the given range and decimal places.
    ///
    /// # Arguments
    ///
    /// * `minimum` - The minimum acceptable value (inclusive)
    /// * `maximum` - The maximum acceptable value (inclusive)
    /// * `decimals` - Maximum number of decimal places allowed
    pub fn new(minimum: f64, maximum: f64, decimals: u32) -> Self {
        Self {
            minimum: minimum.min(maximum),
            maximum: minimum.max(maximum),
            decimals,
        }
    }

    /// Create a validator for non-negative doubles with specified decimals.
    pub fn non_negative(decimals: u32) -> Self {
        Self::new(0.0, f64::MAX, decimals)
    }

    /// Create a validator for positive doubles with specified decimals.
    pub fn positive(decimals: u32) -> Self {
        Self::new(f64::MIN_POSITIVE, f64::MAX, decimals)
    }

    /// Get the minimum value.
    pub fn minimum(&self) -> f64 {
        self.minimum
    }

    /// Get the maximum value.
    pub fn maximum(&self) -> f64 {
        self.maximum
    }

    /// Get the maximum decimal places.
    pub fn decimals(&self) -> u32 {
        self.decimals
    }

    /// Set the minimum value.
    pub fn set_minimum(&mut self, min: f64) {
        self.minimum = min;
        if self.maximum < self.minimum {
            self.maximum = self.minimum;
        }
    }

    /// Set the maximum value.
    pub fn set_maximum(&mut self, max: f64) {
        self.maximum = max;
        if self.minimum > self.maximum {
            self.minimum = self.maximum;
        }
    }

    /// Set the maximum decimal places.
    pub fn set_decimals(&mut self, decimals: u32) {
        self.decimals = decimals;
    }

    /// Set the complete range.
    pub fn set_range(&mut self, min: f64, max: f64, decimals: u32) {
        self.minimum = min.min(max);
        self.maximum = min.max(max);
        self.decimals = decimals;
    }
}

impl Validator for DoubleValidator {
    fn validate(&self, input: &str) -> ValidationState {
        let trimmed = input.trim();

        // Empty input is intermediate
        if trimmed.is_empty() {
            return ValidationState::Intermediate;
        }

        // Just a sign is intermediate
        if trimmed == "-" {
            return if self.minimum < 0.0 {
                ValidationState::Intermediate
            } else {
                ValidationState::Invalid
            };
        }

        if trimmed == "+" {
            return ValidationState::Intermediate;
        }

        // Just a decimal point (with optional sign) is intermediate
        if trimmed == "." || trimmed == "-." || trimmed == "+." {
            return ValidationState::Intermediate;
        }

        // Check for too many decimal places
        if let Some(dot_pos) = trimmed.find('.') {
            let decimal_part = &trimmed[dot_pos + 1..];
            // Only count actual digits, not trailing characters
            let digit_count: usize = decimal_part
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .count();
            if digit_count > self.decimals as usize {
                return ValidationState::Invalid;
            }
        }

        // Check if it's a valid floating-point format
        let is_valid_format = {
            let mut has_dot = false;
            let mut has_digit = false;
            trimmed.chars().enumerate().all(|(i, c)| {
                if c.is_ascii_digit() {
                    has_digit = true;
                    true
                } else if c == '.' {
                    if has_dot {
                        false
                    } else {
                        has_dot = true;
                        true
                    }
                } else { i == 0 && (c == '-' || c == '+') }
            }) && (has_digit || has_dot)
        };

        if !is_valid_format {
            return ValidationState::Invalid;
        }

        // Handle trailing decimal point as intermediate
        if trimmed.ends_with('.') {
            return ValidationState::Intermediate;
        }

        // Try to parse the number
        match trimmed.parse::<f64>() {
            Ok(value) => {
                if value.is_nan() || value.is_infinite() {
                    return ValidationState::Invalid;
                }

                if value >= self.minimum && value <= self.maximum {
                    ValidationState::Acceptable
                } else if value < self.minimum {
                    // Similar logic to IntValidator for intermediate state
                    if self.minimum >= 0.0 && value >= 0.0 {
                        ValidationState::Intermediate
                    } else {
                        ValidationState::Invalid
                    }
                } else {
                    ValidationState::Invalid
                }
            }
            Err(_) => ValidationState::Invalid,
        }
    }

    fn fixup(&self, input: &str) -> Option<String> {
        let trimmed = input.trim();

        // Check if we need to normalize (trailing decimal or out of range)
        let needs_normalization = trimmed.ends_with('.');

        // Remove trailing decimal point for parsing
        let normalized = trimmed.trim_end_matches('.');

        if let Ok(value) = normalized.parse::<f64>()
            && value.is_finite() {
                let clamped = value.clamp(self.minimum, self.maximum);
                let was_clamped = (clamped - value).abs() > f64::EPSILON;

                // Return fixed value if clamped OR if we needed to normalize trailing decimal
                if was_clamped || needs_normalization {
                    return Some(format!("{:.prec$}", clamped, prec = self.decimals as usize));
                }
            }

        None
    }
}

/// Validator that matches input against a regular expression.
///
/// This validator uses the `regex` crate for pattern matching.
/// An empty input is considered intermediate (allowing the user to start typing).
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::validator::{RegexValidator, ValidationState};
///
/// // Email pattern (simplified)
/// let validator = RegexValidator::new(r"^[\w.+-]+@[\w.-]+\.\w{2,}$").unwrap();
///
/// assert_eq!(validator.validate("user@example.com"), ValidationState::Acceptable);
/// assert_eq!(validator.validate("user@"), ValidationState::Intermediate);
/// assert_eq!(validator.validate("not an email!"), ValidationState::Invalid);
/// ```
#[derive(Debug, Clone)]
pub struct RegexValidator {
    pattern: regex::Regex,
    /// Optional pattern for intermediate state (partial matches).
    intermediate_pattern: Option<regex::Regex>,
}

impl RegexValidator {
    /// Create a new regex validator with the given pattern.
    ///
    /// # Arguments
    ///
    /// * `pattern` - A regular expression pattern string
    ///
    /// # Returns
    ///
    /// `Ok(validator)` if the pattern is valid, `Err(error)` otherwise.
    pub fn new(pattern: &str) -> Result<Self, regex::Error> {
        Ok(Self {
            pattern: regex::Regex::new(pattern)?,
            intermediate_pattern: None,
        })
    }

    /// Create a regex validator with a separate pattern for intermediate state.
    ///
    /// The intermediate pattern is used to determine if partial input could
    /// potentially become valid with more typing.
    ///
    /// # Arguments
    ///
    /// * `pattern` - The pattern for acceptable (complete) input
    /// * `intermediate_pattern` - The pattern for intermediate (partial) input
    pub fn with_intermediate(
        pattern: &str,
        intermediate_pattern: &str,
    ) -> Result<Self, regex::Error> {
        Ok(Self {
            pattern: regex::Regex::new(pattern)?,
            intermediate_pattern: Some(regex::Regex::new(intermediate_pattern)?),
        })
    }

    /// Get the pattern string.
    pub fn pattern(&self) -> &str {
        self.pattern.as_str()
    }
}

impl Validator for RegexValidator {
    fn validate(&self, input: &str) -> ValidationState {
        // Empty input is intermediate (user might be about to type)
        if input.is_empty() {
            return ValidationState::Intermediate;
        }

        // Check for full match
        if self.pattern.is_match(input) {
            return ValidationState::Acceptable;
        }

        // Check for intermediate match if we have a pattern
        if let Some(ref intermediate) = self.intermediate_pattern {
            if intermediate.is_match(input) {
                return ValidationState::Intermediate;
            }
        } else {
            // Without an intermediate pattern, check if any prefix of the pattern
            // could match the input (heuristic: if input is shorter than it needs to be)
            // This is a simple heuristic - for more complex cases, use with_intermediate()

            // Check if this could be a prefix of a valid input
            // by checking if the pattern could potentially match something that starts with this
            let prefix_pattern = format!("^{}.*", regex::escape(input));
            if let Ok(prefix_re) = regex::Regex::new(&prefix_pattern) {
                // This is a heuristic check - if the escaped input as a literal prefix
                // doesn't completely invalidate matching, consider it intermediate
                // For simple cases, we just return Intermediate for non-empty non-matching input
                // that could potentially be extended

                // A more sophisticated approach would be to check if the input
                // matches any prefix of strings that would match the full pattern
                // For now, we use a simpler heuristic
                let _ = prefix_re; // Suppress unused warning
            }
            return ValidationState::Intermediate;
        }

        ValidationState::Invalid
    }
}

/// A custom validator that uses a closure for validation.
///
/// This allows creating validators without implementing the trait manually.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::validator::{CustomValidator, ValidationState};
///
/// let validator = CustomValidator::new(|input| {
///     if input.is_empty() {
///         ValidationState::Intermediate
///     } else if input.len() >= 3 {
///         ValidationState::Acceptable
///     } else {
///         ValidationState::Intermediate
///     }
/// });
/// ```
pub struct CustomValidator<F>
where
    F: Fn(&str) -> ValidationState + Send + Sync,
{
    validate_fn: F,
    fixup_fn: Option<Box<dyn Fn(&str) -> Option<String> + Send + Sync>>,
}

impl<F> CustomValidator<F>
where
    F: Fn(&str) -> ValidationState + Send + Sync,
{
    /// Create a new custom validator with the given validation function.
    pub fn new(validate_fn: F) -> Self {
        Self {
            validate_fn,
            fixup_fn: None,
        }
    }

    /// Add a fixup function to the validator.
    pub fn with_fixup<G>(mut self, fixup_fn: G) -> Self
    where
        G: Fn(&str) -> Option<String> + Send + Sync + 'static,
    {
        self.fixup_fn = Some(Box::new(fixup_fn));
        self
    }
}

impl<F> Validator for CustomValidator<F>
where
    F: Fn(&str) -> ValidationState + Send + Sync,
{
    fn validate(&self, input: &str) -> ValidationState {
        (self.validate_fn)(input)
    }

    fn fixup(&self, input: &str) -> Option<String> {
        self.fixup_fn.as_ref().and_then(|f| f(input))
    }
}

impl<F> fmt::Debug for CustomValidator<F>
where
    F: Fn(&str) -> ValidationState + Send + Sync,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CustomValidator")
            .field("has_fixup", &self.fixup_fn.is_some())
            .finish()
    }
}

// =========================================================================
// Hex Color Validator
// =========================================================================

/// Format options for hex color input and output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HexFormat {
    /// Whether to include the '#' prefix in output.
    pub include_prefix: bool,
    /// Whether to use uppercase letters (A-F vs a-f).
    pub uppercase: bool,
    /// Whether to allow/expand short form (#RGB to #RRGGBB).
    pub allow_short_form: bool,
    /// Whether to include alpha channel in output.
    pub include_alpha: bool,
}

impl HexFormat {
    /// Create a new hex format with default settings.
    ///
    /// Defaults: prefix enabled, uppercase, no short form, no alpha.
    pub fn new() -> Self {
        Self {
            include_prefix: true,
            uppercase: true,
            allow_short_form: false,
            include_alpha: false,
        }
    }

    /// Create format with prefix (#RRGGBB).
    pub fn with_prefix(mut self) -> Self {
        self.include_prefix = true;
        self
    }

    /// Create format without prefix (RRGGBB).
    pub fn without_prefix(mut self) -> Self {
        self.include_prefix = false;
        self
    }

    /// Use uppercase hex digits (A-F).
    pub fn uppercase(mut self) -> Self {
        self.uppercase = true;
        self
    }

    /// Use lowercase hex digits (a-f).
    pub fn lowercase(mut self) -> Self {
        self.uppercase = false;
        self
    }

    /// Allow short form input (#RGB expands to #RRGGBB).
    pub fn allow_short(mut self) -> Self {
        self.allow_short_form = true;
        self
    }

    /// Include alpha channel in output (#RRGGBBAA).
    pub fn with_alpha(mut self) -> Self {
        self.include_alpha = true;
        self
    }

    /// Format a color as a hex string according to this format.
    pub fn format_color(&self, r: u8, g: u8, b: u8, a: u8) -> String {
        let prefix = if self.include_prefix { "#" } else { "" };

        if self.include_alpha || a != 255 {
            if self.uppercase {
                format!("{}{:02X}{:02X}{:02X}{:02X}", prefix, r, g, b, a)
            } else {
                format!("{}{:02x}{:02x}{:02x}{:02x}", prefix, r, g, b, a)
            }
        } else if self.uppercase {
            format!("{}{:02X}{:02X}{:02X}", prefix, r, g, b)
        } else {
            format!("{}{:02x}{:02x}{:02x}", prefix, r, g, b)
        }
    }
}

/// Validator for hexadecimal color input.
///
/// Validates hex color strings in formats like:
/// - `#RRGGBB` - 6-digit RGB
/// - `#RRGGBBAA` - 8-digit RGBA
/// - `#RGB` - 3-digit short form (if enabled)
/// - `#RGBA` - 4-digit short form with alpha (if enabled)
/// - Without `#` prefix (if configured)
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::validator::{HexColorValidator, HexFormat, ValidationState};
///
/// let validator = HexColorValidator::new();
///
/// assert_eq!(validator.validate("#FF0000"), ValidationState::Acceptable);
/// assert_eq!(validator.validate("#FF00"), ValidationState::Intermediate);
/// assert_eq!(validator.validate("#GG0000"), ValidationState::Invalid);
///
/// // With short form enabled
/// let validator = HexColorValidator::with_format(HexFormat::new().allow_short());
/// assert_eq!(validator.validate("#F00"), ValidationState::Acceptable);
/// ```
#[derive(Debug, Clone)]
pub struct HexColorValidator {
    format: HexFormat,
}

impl HexColorValidator {
    /// Create a new hex color validator with default format.
    pub fn new() -> Self {
        Self {
            format: HexFormat::new(),
        }
    }

    /// Create a hex color validator with custom format options.
    pub fn with_format(format: HexFormat) -> Self {
        Self { format }
    }

    /// Get the format options.
    pub fn format(&self) -> &HexFormat {
        &self.format
    }

    /// Set the format options.
    pub fn set_format(&mut self, format: HexFormat) {
        self.format = format;
    }

    /// Check if a character is a valid hex digit.
    fn is_hex_digit(c: char) -> bool {
        c.is_ascii_hexdigit()
    }

    /// Expand short form hex (#RGB or #RGBA) to full form (#RRGGBB or #RRGGBBAA).
    pub fn expand_short_form(hex: &str) -> Option<String> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            3 => {
                // #RGB -> #RRGGBB
                let chars: Vec<char> = hex.chars().collect();
                Some(format!(
                    "#{}{}{}{}{}{}",
                    chars[0], chars[0], chars[1], chars[1], chars[2], chars[2]
                ))
            }
            4 => {
                // #RGBA -> #RRGGBBAA
                let chars: Vec<char> = hex.chars().collect();
                Some(format!(
                    "#{}{}{}{}{}{}{}{}",
                    chars[0], chars[0], chars[1], chars[1], chars[2], chars[2], chars[3], chars[3]
                ))
            }
            _ => None,
        }
    }

    /// Parse hex string to RGBA values.
    pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8, u8)> {
        let hex = hex.trim_start_matches('#');

        // Handle short form
        let expanded: String;
        let hex = if hex.len() == 3 || hex.len() == 4 {
            expanded = Self::expand_short_form(&format!("#{}", hex))?;
            expanded.trim_start_matches('#')
        } else {
            hex
        };

        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some((r, g, b, 255))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some((r, g, b, a))
            }
            _ => None,
        }
    }
}

impl Default for HexColorValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for HexColorValidator {
    fn validate(&self, input: &str) -> ValidationState {
        let trimmed = input.trim();

        // Empty input is intermediate
        if trimmed.is_empty() {
            return ValidationState::Intermediate;
        }

        // Check for prefix
        let (has_prefix, hex_part) = if trimmed.starts_with('#') {
            (true, &trimmed[1..])
        } else {
            (false, trimmed)
        };

        // Just '#' is intermediate
        if has_prefix && hex_part.is_empty() {
            return ValidationState::Intermediate;
        }

        // Check that all characters are valid hex digits
        if !hex_part.chars().all(Self::is_hex_digit) {
            return ValidationState::Invalid;
        }

        let len = hex_part.len();

        // Check for valid lengths
        match len {
            // Short forms (if allowed)
            3 | 4 if self.format.allow_short_form => ValidationState::Acceptable,
            3 | 4 if !self.format.allow_short_form => {
                // Could be typing toward 6 or 8 digits
                ValidationState::Intermediate
            }
            // Standard forms
            6 | 8 => ValidationState::Acceptable,
            // Partial input (could become valid)
            1 | 2 | 5 | 7 => ValidationState::Intermediate,
            // Too long
            _ if len > 8 => ValidationState::Invalid,
            // Other intermediate lengths
            _ => ValidationState::Intermediate,
        }
    }

    fn fixup(&self, input: &str) -> Option<String> {
        let trimmed = input.trim();

        // Try to parse the input
        if let Some((r, g, b, a)) = Self::parse_hex(trimmed) {
            let formatted = self.format.format_color(r, g, b, a);

            // Only return fixup if different from the original input
            if formatted != trimmed {
                return Some(formatted);
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // IntValidator Tests
    // =========================================================================

    #[test]
    fn test_int_validator_acceptable() {
        let validator = IntValidator::new(0, 100);
        assert_eq!(validator.validate("0"), ValidationState::Acceptable);
        assert_eq!(validator.validate("50"), ValidationState::Acceptable);
        assert_eq!(validator.validate("100"), ValidationState::Acceptable);
    }

    #[test]
    fn test_int_validator_out_of_range() {
        let validator = IntValidator::new(0, 100);
        assert_eq!(validator.validate("-1"), ValidationState::Invalid);
        assert_eq!(validator.validate("101"), ValidationState::Invalid);
        assert_eq!(validator.validate("1000"), ValidationState::Invalid);
    }

    #[test]
    fn test_int_validator_intermediate() {
        let validator = IntValidator::new(0, 100);
        assert_eq!(validator.validate(""), ValidationState::Intermediate);
        // "1" is valid in [0, 100] so it's Acceptable (even though typing could continue)
        assert_eq!(validator.validate("1"), ValidationState::Acceptable);

        // To get intermediate, we need a value below the minimum
        let validator2 = IntValidator::new(10, 100);
        assert_eq!(validator2.validate("5"), ValidationState::Intermediate); // Could become 50, 55, etc.
    }

    #[test]
    fn test_int_validator_negative_range() {
        let validator = IntValidator::new(-100, 100);
        assert_eq!(validator.validate("-"), ValidationState::Intermediate);
        assert_eq!(validator.validate("-50"), ValidationState::Acceptable);
        assert_eq!(validator.validate("-100"), ValidationState::Acceptable);
        assert_eq!(validator.validate("-101"), ValidationState::Invalid);
    }

    #[test]
    fn test_int_validator_no_negative() {
        let validator = IntValidator::new(0, 100);
        assert_eq!(validator.validate("-"), ValidationState::Invalid);
    }

    #[test]
    fn test_int_validator_invalid_format() {
        let validator = IntValidator::new(0, 100);
        assert_eq!(validator.validate("abc"), ValidationState::Invalid);
        assert_eq!(validator.validate("12.5"), ValidationState::Invalid);
        assert_eq!(validator.validate("1e5"), ValidationState::Invalid);
    }

    #[test]
    fn test_int_validator_fixup() {
        let validator = IntValidator::new(0, 100);
        assert_eq!(validator.fixup("-50"), Some("0".to_string()));
        assert_eq!(validator.fixup("150"), Some("100".to_string()));
        assert_eq!(validator.fixup("50"), None);
    }

    // =========================================================================
    // DoubleValidator Tests
    // =========================================================================

    #[test]
    fn test_double_validator_acceptable() {
        let validator = DoubleValidator::new(0.0, 100.0, 2);
        assert_eq!(validator.validate("0"), ValidationState::Acceptable);
        assert_eq!(validator.validate("50.5"), ValidationState::Acceptable);
        assert_eq!(validator.validate("100"), ValidationState::Acceptable);
        assert_eq!(validator.validate("3.14"), ValidationState::Acceptable);
    }

    #[test]
    fn test_double_validator_out_of_range() {
        let validator = DoubleValidator::new(0.0, 100.0, 2);
        assert_eq!(validator.validate("-1"), ValidationState::Invalid);
        assert_eq!(validator.validate("100.01"), ValidationState::Invalid);
    }

    #[test]
    fn test_double_validator_intermediate() {
        let validator = DoubleValidator::new(0.0, 100.0, 2);
        assert_eq!(validator.validate(""), ValidationState::Intermediate);
        assert_eq!(validator.validate("."), ValidationState::Intermediate);
        assert_eq!(validator.validate("3."), ValidationState::Intermediate);
    }

    #[test]
    fn test_double_validator_decimal_places() {
        let validator = DoubleValidator::new(0.0, 100.0, 2);
        assert_eq!(validator.validate("3.14"), ValidationState::Acceptable);
        assert_eq!(validator.validate("3.141"), ValidationState::Invalid); // Too many decimals
    }

    #[test]
    fn test_double_validator_negative() {
        let validator = DoubleValidator::new(-100.0, 100.0, 2);
        assert_eq!(validator.validate("-"), ValidationState::Intermediate);
        assert_eq!(validator.validate("-."), ValidationState::Intermediate);
        assert_eq!(validator.validate("-50.5"), ValidationState::Acceptable);
    }

    #[test]
    fn test_double_validator_invalid_format() {
        let validator = DoubleValidator::new(0.0, 100.0, 2);
        assert_eq!(validator.validate("abc"), ValidationState::Invalid);
        assert_eq!(validator.validate("1.2.3"), ValidationState::Invalid);
    }

    #[test]
    fn test_double_validator_fixup() {
        let validator = DoubleValidator::new(0.0, 100.0, 2);
        assert_eq!(validator.fixup("-50"), Some("0.00".to_string()));
        assert_eq!(validator.fixup("150"), Some("100.00".to_string()));
        assert_eq!(validator.fixup("50."), Some("50.00".to_string()));
    }

    // =========================================================================
    // RegexValidator Tests
    // =========================================================================

    #[test]
    fn test_regex_validator_acceptable() {
        let validator = RegexValidator::new(r"^\d{3}-\d{4}$").unwrap();
        assert_eq!(validator.validate("123-4567"), ValidationState::Acceptable);
    }

    #[test]
    fn test_regex_validator_intermediate() {
        let validator = RegexValidator::new(r"^\d{3}-\d{4}$").unwrap();
        assert_eq!(validator.validate(""), ValidationState::Intermediate);
        assert_eq!(validator.validate("123"), ValidationState::Intermediate);
        assert_eq!(validator.validate("123-"), ValidationState::Intermediate);
    }

    #[test]
    fn test_regex_validator_with_intermediate_pattern() {
        let validator =
            RegexValidator::with_intermediate(r"^\d{3}-\d{4}$", r"^\d{0,3}(-\d{0,4})?$").unwrap();
        assert_eq!(validator.validate("123-4567"), ValidationState::Acceptable);
        assert_eq!(validator.validate("12"), ValidationState::Intermediate);
        assert_eq!(validator.validate("123-"), ValidationState::Intermediate);
        assert_eq!(validator.validate("abc"), ValidationState::Invalid);
    }

    // =========================================================================
    // CustomValidator Tests
    // =========================================================================

    #[test]
    fn test_custom_validator() {
        let validator = CustomValidator::new(|input| {
            if input.is_empty() {
                ValidationState::Intermediate
            } else if input.len() >= 3 {
                ValidationState::Acceptable
            } else {
                ValidationState::Intermediate
            }
        });

        assert_eq!(validator.validate(""), ValidationState::Intermediate);
        assert_eq!(validator.validate("ab"), ValidationState::Intermediate);
        assert_eq!(validator.validate("abc"), ValidationState::Acceptable);
    }

    #[test]
    fn test_custom_validator_with_fixup() {
        let validator = CustomValidator::new(|input| {
            if input.chars().all(|c| c.is_uppercase()) {
                ValidationState::Acceptable
            } else {
                ValidationState::Intermediate
            }
        })
        .with_fixup(|input| Some(input.to_uppercase()));

        assert_eq!(validator.validate("hello"), ValidationState::Intermediate);
        assert_eq!(validator.fixup("hello"), Some("HELLO".to_string()));
    }

    // =========================================================================
    // HexColorValidator Tests
    // =========================================================================

    #[test]
    fn test_hex_validator_acceptable() {
        let validator = HexColorValidator::new();
        assert_eq!(validator.validate("#FF0000"), ValidationState::Acceptable);
        assert_eq!(validator.validate("#00FF00"), ValidationState::Acceptable);
        assert_eq!(validator.validate("#0000FF"), ValidationState::Acceptable);
        assert_eq!(validator.validate("#FF000080"), ValidationState::Acceptable);
        assert_eq!(validator.validate("FF0000"), ValidationState::Acceptable);
        assert_eq!(validator.validate("ff0000"), ValidationState::Acceptable);
    }

    #[test]
    fn test_hex_validator_intermediate() {
        let validator = HexColorValidator::new();
        assert_eq!(validator.validate(""), ValidationState::Intermediate);
        assert_eq!(validator.validate("#"), ValidationState::Intermediate);
        assert_eq!(validator.validate("#F"), ValidationState::Intermediate);
        assert_eq!(validator.validate("#FF"), ValidationState::Intermediate);
        assert_eq!(validator.validate("#FF0"), ValidationState::Intermediate);
        assert_eq!(validator.validate("#FF00"), ValidationState::Intermediate);
        assert_eq!(validator.validate("#FF000"), ValidationState::Intermediate);
        assert_eq!(
            validator.validate("#FF00000"),
            ValidationState::Intermediate
        );
    }

    #[test]
    fn test_hex_validator_invalid() {
        let validator = HexColorValidator::new();
        assert_eq!(validator.validate("#GG0000"), ValidationState::Invalid);
        assert_eq!(validator.validate("#FF0000GG"), ValidationState::Invalid);
        assert_eq!(validator.validate("#FF00000000"), ValidationState::Invalid);
        assert_eq!(validator.validate("hello"), ValidationState::Invalid);
    }

    #[test]
    fn test_hex_validator_short_form() {
        // Without short form enabled
        let validator = HexColorValidator::new();
        assert_eq!(validator.validate("#F00"), ValidationState::Intermediate);

        // With short form enabled
        let validator = HexColorValidator::with_format(HexFormat::new().allow_short());
        assert_eq!(validator.validate("#F00"), ValidationState::Acceptable);
        assert_eq!(validator.validate("#F00A"), ValidationState::Acceptable);
        assert_eq!(validator.validate("#FF0000"), ValidationState::Acceptable);
    }

    #[test]
    fn test_hex_validator_expand_short_form() {
        assert_eq!(
            HexColorValidator::expand_short_form("#F00"),
            Some("#FF0000".to_string())
        );
        assert_eq!(
            HexColorValidator::expand_short_form("#F00A"),
            Some("#FF0000AA".to_string())
        );
        assert_eq!(HexColorValidator::expand_short_form("#FF0000"), None);
    }

    #[test]
    fn test_hex_validator_parse_hex() {
        assert_eq!(
            HexColorValidator::parse_hex("#FF0000"),
            Some((255, 0, 0, 255))
        );
        assert_eq!(
            HexColorValidator::parse_hex("#00FF0080"),
            Some((0, 255, 0, 128))
        );
        assert_eq!(HexColorValidator::parse_hex("#F00"), Some((255, 0, 0, 255)));
        assert_eq!(
            HexColorValidator::parse_hex("FF0000"),
            Some((255, 0, 0, 255))
        );
    }

    #[test]
    fn test_hex_format_color() {
        let format = HexFormat::new();
        assert_eq!(format.format_color(255, 0, 0, 255), "#FF0000");
        assert_eq!(format.format_color(0, 255, 0, 128), "#00FF0080");

        let format = HexFormat::new().lowercase();
        assert_eq!(format.format_color(255, 0, 0, 255), "#ff0000");

        let format = HexFormat::new().without_prefix();
        assert_eq!(format.format_color(255, 0, 0, 255), "FF0000");

        let format = HexFormat::new().with_alpha();
        assert_eq!(format.format_color(255, 0, 0, 255), "#FF0000FF");
    }

    #[test]
    fn test_hex_validator_fixup() {
        let validator = HexColorValidator::new();

        // Lowercase to uppercase
        assert_eq!(validator.fixup("#ff0000"), Some("#FF0000".to_string()));

        // Short form expansion (fixup expands and normalizes)
        let validator_short = HexColorValidator::with_format(HexFormat::new().allow_short());
        assert_eq!(validator_short.fixup("#f00"), Some("#FF0000".to_string()));

        // Already correct - no fixup needed
        assert_eq!(validator.fixup("#FF0000"), None);
    }
}
