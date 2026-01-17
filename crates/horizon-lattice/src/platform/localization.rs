//! Localization and internationalization support.
//!
//! This module provides cross-platform locale detection and locale-aware formatting
//! for numbers, dates, times, and currencies. It also provides text direction detection
//! for bidirectional text support.
//!
//! # System Locale Detection
//!
//! ```ignore
//! use horizon_lattice::platform::{SystemLocale, LocaleInfo};
//!
//! // Get the system locale identifier
//! let locale = SystemLocale::current();
//! println!("System locale: {}", locale); // e.g., "en-US", "fr-FR"
//!
//! // Get detailed locale information
//! let info = SystemLocale::info();
//! println!("Language: {:?}", info.language);
//! println!("Region: {:?}", info.region);
//! ```
//!
//! # Number Formatting
//!
//! ```ignore
//! use horizon_lattice::platform::NumberFormatter;
//!
//! // Format with system locale
//! let formatter = NumberFormatter::new();
//! println!("{}", formatter.format(1234567.89)); // "1,234,567.89" (en-US)
//!
//! // Format with specific locale
//! let formatter = NumberFormatter::with_locale("de-DE");
//! println!("{}", formatter.format(1234567.89)); // "1.234.567,89" (German)
//! ```
//!
//! # Date/Time Formatting
//!
//! ```ignore
//! use horizon_lattice::platform::{DateTimeFormatter, DateLength, TimeLength};
//! use chrono::Local;
//!
//! let formatter = DateTimeFormatter::new();
//!
//! // Format current date
//! let now = Local::now();
//! println!("{}", formatter.format_date(&now, DateLength::Long));
//! // "January 17, 2026" (en-US) or "17 janvier 2026" (fr-FR)
//!
//! println!("{}", formatter.format_time(&now, TimeLength::Short));
//! // "3:45 PM" (en-US) or "15:45" (fr-FR)
//! ```
//!
//! # Text Direction
//!
//! ```ignore
//! use horizon_lattice::platform::TextDirection;
//!
//! // Detect text direction
//! let dir = TextDirection::detect("Hello");
//! assert_eq!(dir, TextDirection::Ltr);
//!
//! let dir = TextDirection::detect("مرحبا"); // Arabic
//! assert_eq!(dir, TextDirection::Rtl);
//! ```

use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use horizon_lattice_core::Signal;

// ============================================================================
// Error Types
// ============================================================================

/// Error type for localization operations.
#[derive(Debug)]
pub struct LocalizationError {
    kind: LocalizationErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum LocalizationErrorKind {
    /// Failed to detect locale.
    Detection,
    /// Invalid locale identifier.
    InvalidLocale,
    /// Formatting error.
    Format,
    /// Operation not supported on this platform.
    UnsupportedPlatform,
}

impl LocalizationError {
    fn detection(message: impl Into<String>) -> Self {
        Self {
            kind: LocalizationErrorKind::Detection,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn invalid_locale(message: impl Into<String>) -> Self {
        Self {
            kind: LocalizationErrorKind::InvalidLocale,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn format(message: impl Into<String>) -> Self {
        Self {
            kind: LocalizationErrorKind::Format,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn unsupported_platform(message: impl Into<String>) -> Self {
        Self {
            kind: LocalizationErrorKind::UnsupportedPlatform,
            message: message.into(),
        }
    }

    /// Returns true if this error indicates an invalid locale.
    pub fn is_invalid_locale(&self) -> bool {
        self.kind == LocalizationErrorKind::InvalidLocale
    }

    /// Returns true if this error indicates the operation is not supported.
    pub fn is_unsupported_platform(&self) -> bool {
        self.kind == LocalizationErrorKind::UnsupportedPlatform
    }
}

impl fmt::Display for LocalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            LocalizationErrorKind::Detection => {
                write!(f, "locale detection error: {}", self.message)
            }
            LocalizationErrorKind::InvalidLocale => {
                write!(f, "invalid locale: {}", self.message)
            }
            LocalizationErrorKind::Format => {
                write!(f, "formatting error: {}", self.message)
            }
            LocalizationErrorKind::UnsupportedPlatform => {
                write!(f, "unsupported platform: {}", self.message)
            }
        }
    }
}

impl std::error::Error for LocalizationError {}

// ============================================================================
// Text Direction
// ============================================================================

/// Text direction for bidirectional text support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextDirection {
    /// Left-to-right (e.g., English, French, German)
    Ltr,
    /// Right-to-left (e.g., Arabic, Hebrew)
    Rtl,
    /// Mixed or neutral direction
    Mixed,
}

impl TextDirection {
    /// Detect the text direction of a string.
    ///
    /// This uses the Unicode Bidirectional Algorithm to determine
    /// the base direction of the text.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::platform::TextDirection;
    ///
    /// assert_eq!(TextDirection::detect("Hello"), TextDirection::Ltr);
    /// assert_eq!(TextDirection::detect("مرحبا"), TextDirection::Rtl);
    /// ```
    pub fn detect(text: &str) -> Self {
        use unicode_bidi::{bidi_class, BidiClass};

        if text.is_empty() {
            return TextDirection::Ltr;
        }

        // Find the first strong directional character
        for ch in text.chars() {
            match bidi_class(ch) {
                BidiClass::L => return TextDirection::Ltr,
                BidiClass::R | BidiClass::AL => return TextDirection::Rtl,
                _ => continue,
            }
        }

        // No strong directional character found, default to LTR
        TextDirection::Ltr
    }

    /// Detect if a locale typically uses RTL text.
    ///
    /// This checks if the language code is one of the common RTL languages.
    pub fn for_locale(locale: &str) -> Self {
        // Extract language code (before '-' or '_')
        let lang = locale.split(['-', '_']).next().unwrap_or(locale);

        // Common RTL languages
        match lang.to_lowercase().as_str() {
            "ar" | "he" | "fa" | "ur" | "yi" | "ps" | "sd" | "ug" | "ku" | "ckb" | "dv" | "arc"
            | "syr" => TextDirection::Rtl,
            _ => TextDirection::Ltr,
        }
    }

    /// Returns true if this is left-to-right direction.
    pub fn is_ltr(&self) -> bool {
        matches!(self, TextDirection::Ltr)
    }

    /// Returns true if this is right-to-left direction.
    pub fn is_rtl(&self) -> bool {
        matches!(self, TextDirection::Rtl)
    }
}

impl Default for TextDirection {
    fn default() -> Self {
        TextDirection::Ltr
    }
}

// ============================================================================
// Locale Information
// ============================================================================

/// Detailed information about a locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocaleInfo {
    /// The full locale identifier (e.g., "en-US", "fr-FR").
    pub identifier: String,
    /// The language code (e.g., "en", "fr").
    pub language: String,
    /// The region/country code (e.g., "US", "FR"), if present.
    pub region: Option<String>,
    /// The script code (e.g., "Latn", "Cyrl"), if present.
    pub script: Option<String>,
    /// The text direction for this locale.
    pub direction: TextDirection,
}

impl LocaleInfo {
    /// Parse a locale identifier into its components.
    pub fn parse(identifier: &str) -> Self {
        let mut parts = identifier.split(['-', '_']);

        let language = parts.next().unwrap_or("en").to_lowercase();
        let direction = TextDirection::for_locale(&language);

        let mut region = None;
        let mut script = None;

        for part in parts {
            // Scripts are 4 characters, title case (e.g., "Latn")
            if part.len() == 4 && part.chars().next().is_some_and(|c| c.is_uppercase()) {
                script = Some(part.to_string());
            }
            // Regions are 2 characters uppercase (e.g., "US") or 3 digits
            else if (part.len() == 2 && part.chars().all(|c| c.is_ascii_uppercase()))
                || (part.len() == 3 && part.chars().all(|c| c.is_ascii_digit()))
            {
                region = Some(part.to_uppercase());
            }
        }

        Self {
            identifier: identifier.to_string(),
            language,
            region,
            script,
            direction,
        }
    }
}

impl Default for LocaleInfo {
    fn default() -> Self {
        Self::parse("en-US")
    }
}

// ============================================================================
// System Locale
// ============================================================================

/// Static methods for detecting system locale.
///
/// This struct provides one-shot queries for the current system locale.
/// For real-time change notifications, use [`LocaleWatcher`].
pub struct SystemLocale;

impl SystemLocale {
    /// Get the current system locale identifier.
    ///
    /// Returns a BCP 47 locale identifier (e.g., "en-US", "fr-FR", "de-DE").
    ///
    /// # Platform Behavior
    ///
    /// - **Windows**: Uses `GetUserDefaultLocaleName` API
    /// - **macOS**: Uses `CFLocaleCopyCurrent` API
    /// - **Linux**: Uses `LC_ALL`, `LC_MESSAGES`, or `LANG` environment variables
    #[cfg(feature = "localization")]
    pub fn current() -> String {
        sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string())
    }

    #[cfg(not(feature = "localization"))]
    pub fn current() -> String {
        "en-US".to_string()
    }

    /// Get detailed information about the current system locale.
    pub fn info() -> LocaleInfo {
        LocaleInfo::parse(&Self::current())
    }

    /// Get the text direction for the current system locale.
    pub fn direction() -> TextDirection {
        Self::info().direction
    }

    /// Get all available system locales.
    ///
    /// Returns a list of locale identifiers supported by the system.
    /// Note: This may be a subset of all possible locales.
    #[cfg(feature = "localization")]
    pub fn available_locales() -> Vec<String> {
        // sys-locale only provides the current locale, not a list
        // We could potentially use platform APIs to enumerate, but for now
        // just return the current locale
        vec![Self::current()]
    }

    #[cfg(not(feature = "localization"))]
    pub fn available_locales() -> Vec<String> {
        vec!["en-US".to_string()]
    }
}

// ============================================================================
// Number Formatting
// ============================================================================

/// Date format length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateLength {
    /// Short format (e.g., "1/17/26" or "17/01/26")
    Short,
    /// Medium format (e.g., "Jan 17, 2026")
    #[default]
    Medium,
    /// Long format (e.g., "January 17, 2026")
    Long,
    /// Full format (e.g., "Friday, January 17, 2026")
    Full,
}

/// Time format length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeLength {
    /// Short format (e.g., "3:45 PM" or "15:45")
    #[default]
    Short,
    /// Medium format (e.g., "3:45:30 PM")
    Medium,
    /// Long format (e.g., "3:45:30 PM EST")
    Long,
}

/// Locale-aware number formatter.
///
/// Formats numbers according to locale conventions (thousands separators,
/// decimal points, etc.).
#[cfg(feature = "localization")]
pub struct NumberFormatter {
    locale: icu::locale::Locale,
    formatter: icu::decimal::DecimalFormatter,
}

#[cfg(feature = "localization")]
impl NumberFormatter {
    /// Create a new number formatter using the system locale.
    pub fn new() -> Self {
        Self::with_locale(&SystemLocale::current())
    }

    /// Create a number formatter for a specific locale.
    ///
    /// # Arguments
    ///
    /// * `locale` - A BCP 47 locale identifier (e.g., "en-US", "de-DE")
    pub fn with_locale(locale: &str) -> Self {
        use icu::decimal::DecimalFormatter;
        use icu::locale::Locale;

        let locale: Locale = locale
            .parse()
            .unwrap_or_else(|_| "en-US".parse().unwrap());

        let formatter =
            DecimalFormatter::try_new(locale.clone().into(), Default::default()).unwrap_or_else(
                |_| {
                    let default_locale: Locale = "en-US".parse().unwrap();
                    DecimalFormatter::try_new(
                        default_locale.into(),
                        Default::default(),
                    )
                    .expect("default locale should always work")
                },
            );

        Self { locale, formatter }
    }

    /// Format an integer.
    pub fn format_integer(&self, value: i64) -> String {
        use icu::decimal::input::Decimal;
        let decimal = Decimal::from(value);
        self.formatter.format(&decimal).to_string()
    }

    /// Format a floating-point number with default precision.
    pub fn format(&self, value: f64) -> String {
        self.format_with_precision(value, 2)
    }

    /// Format a floating-point number with specified decimal places.
    pub fn format_with_precision(&self, value: f64, decimal_places: i16) -> String {
        use icu::decimal::input::Decimal;

        // Convert f64 to Decimal by scaling to integer
        let scale = 10_i64.pow(decimal_places as u32);
        let scaled_value = (value * scale as f64).round() as i64;
        let mut decimal = Decimal::from(scaled_value);
        decimal.multiply_pow10(-decimal_places);

        self.formatter.format(&decimal).to_string()
    }

    /// Get the locale identifier being used.
    pub fn locale(&self) -> String {
        self.locale.to_string()
    }
}

#[cfg(feature = "localization")]
impl Default for NumberFormatter {
    fn default() -> Self {
        Self::new()
    }
}

// Non-feature stub
#[cfg(not(feature = "localization"))]
pub struct NumberFormatter {
    locale: String,
}

#[cfg(not(feature = "localization"))]
impl NumberFormatter {
    /// Create a new number formatter (stub without localization feature).
    pub fn new() -> Self {
        Self {
            locale: "en-US".to_string(),
        }
    }

    /// Create a number formatter for a specific locale (stub).
    pub fn with_locale(locale: &str) -> Self {
        Self {
            locale: locale.to_string(),
        }
    }

    /// Format an integer (basic formatting without locale support).
    pub fn format_integer(&self, value: i64) -> String {
        format!("{value}")
    }

    /// Format a floating-point number (basic formatting).
    pub fn format(&self, value: f64) -> String {
        format!("{value:.2}")
    }

    /// Format with precision (basic formatting).
    pub fn format_with_precision(&self, value: f64, decimal_places: i16) -> String {
        format!("{value:.prec$}", prec = decimal_places as usize)
    }

    /// Get the locale identifier.
    pub fn locale(&self) -> String {
        self.locale.clone()
    }
}

#[cfg(not(feature = "localization"))]
impl Default for NumberFormatter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Date/Time Formatting
// ============================================================================

/// Locale-aware date and time formatter.
///
/// Formats dates and times according to locale conventions.
#[cfg(feature = "localization")]
pub struct DateTimeFormatter {
    locale: icu::locale::Locale,
}

#[cfg(feature = "localization")]
impl DateTimeFormatter {
    /// Create a new date/time formatter using the system locale.
    pub fn new() -> Self {
        Self::with_locale(&SystemLocale::current())
    }

    /// Create a date/time formatter for a specific locale.
    pub fn with_locale(locale: &str) -> Self {
        use icu::locale::Locale;

        let locale: Locale = locale
            .parse()
            .unwrap_or_else(|_| "en-US".parse().unwrap());

        Self { locale }
    }

    /// Format a date according to the locale.
    pub fn format_date(&self, datetime: &chrono::DateTime<chrono::Local>, length: DateLength) -> String {
        use chrono::Datelike;
        use icu::calendar::Date;
        use icu::datetime::fieldsets;
        use icu::datetime::DateTimeFormatter as IcuDateTimeFormatter;

        let naive = datetime.naive_local();
        let year = naive.year();
        let month = naive.month() as u8;
        let day = naive.day() as u8;

        // Convert to ICU4X Date
        let icu_date = match Date::try_new_iso(year, month, day) {
            Ok(d) => d,
            Err(_) => return datetime.format("%Y-%m-%d").to_string(),
        };

        // Create formatter based on length
        let result = match length {
            DateLength::Short => {
                IcuDateTimeFormatter::try_new(
                    self.locale.clone().into(),
                    fieldsets::YMD::short(),
                ).ok().map(|f| f.format(&icu_date).to_string())
            }
            DateLength::Medium => {
                IcuDateTimeFormatter::try_new(
                    self.locale.clone().into(),
                    fieldsets::YMD::medium(),
                ).ok().map(|f| f.format(&icu_date).to_string())
            }
            DateLength::Long => {
                IcuDateTimeFormatter::try_new(
                    self.locale.clone().into(),
                    fieldsets::YMD::long(),
                ).ok().map(|f| f.format(&icu_date).to_string())
            }
            DateLength::Full => {
                IcuDateTimeFormatter::try_new(
                    self.locale.clone().into(),
                    fieldsets::YMDE::long(),
                ).ok().map(|f| f.format(&icu_date).to_string())
            }
        };

        result.unwrap_or_else(|| datetime.format("%Y-%m-%d").to_string())
    }

    /// Format a time according to the locale.
    pub fn format_time(&self, datetime: &chrono::DateTime<chrono::Local>, length: TimeLength) -> String {
        use chrono::Timelike;
        use icu::datetime::fieldsets;
        use icu::datetime::NoCalendarFormatter;
        use icu::time::Time;

        let naive = datetime.naive_local();
        let hour = naive.hour() as u8;
        let minute = naive.minute() as u8;
        let second = naive.second() as u8;

        // Convert to ICU4X Time
        let icu_time = match Time::try_new(hour, minute, second, 0) {
            Ok(t) => t,
            Err(_) => return datetime.format("%H:%M:%S").to_string(),
        };

        let result = match length {
            TimeLength::Short => {
                NoCalendarFormatter::try_new(
                    self.locale.clone().into(),
                    fieldsets::T::short(),
                ).ok().map(|f| f.format(&icu_time).to_string())
            }
            TimeLength::Medium => {
                NoCalendarFormatter::try_new(
                    self.locale.clone().into(),
                    fieldsets::T::medium(),
                ).ok().map(|f| f.format(&icu_time).to_string())
            }
            TimeLength::Long => {
                NoCalendarFormatter::try_new(
                    self.locale.clone().into(),
                    fieldsets::T::long(),
                ).ok().map(|f| f.format(&icu_time).to_string())
            }
        };

        result.unwrap_or_else(|| datetime.format("%H:%M:%S").to_string())
    }

    /// Format both date and time according to the locale.
    pub fn format_datetime(
        &self,
        datetime: &chrono::DateTime<chrono::Local>,
        date_length: DateLength,
        time_length: TimeLength,
    ) -> String {
        format!(
            "{} {}",
            self.format_date(datetime, date_length),
            self.format_time(datetime, time_length)
        )
    }

    /// Get the locale identifier being used.
    pub fn locale(&self) -> String {
        self.locale.to_string()
    }
}

#[cfg(feature = "localization")]
impl Default for DateTimeFormatter {
    fn default() -> Self {
        Self::new()
    }
}

// Non-feature stub
#[cfg(not(feature = "localization"))]
pub struct DateTimeFormatter {
    locale: String,
}

#[cfg(not(feature = "localization"))]
impl DateTimeFormatter {
    /// Create a new date/time formatter (stub).
    pub fn new() -> Self {
        Self {
            locale: "en-US".to_string(),
        }
    }

    /// Create a date/time formatter for a specific locale (stub).
    pub fn with_locale(locale: &str) -> Self {
        Self {
            locale: locale.to_string(),
        }
    }

    /// Format a date (basic formatting).
    pub fn format_date(&self, datetime: &chrono::DateTime<chrono::Local>, length: DateLength) -> String {
        match length {
            DateLength::Short => datetime.format("%m/%d/%y").to_string(),
            DateLength::Medium => datetime.format("%b %d, %Y").to_string(),
            DateLength::Long => datetime.format("%B %d, %Y").to_string(),
            DateLength::Full => datetime.format("%A, %B %d, %Y").to_string(),
        }
    }

    /// Format a time (basic formatting).
    pub fn format_time(&self, datetime: &chrono::DateTime<chrono::Local>, length: TimeLength) -> String {
        match length {
            TimeLength::Short => datetime.format("%I:%M %p").to_string(),
            TimeLength::Medium => datetime.format("%I:%M:%S %p").to_string(),
            TimeLength::Long => datetime.format("%I:%M:%S %p %Z").to_string(),
        }
    }

    /// Format both date and time.
    pub fn format_datetime(
        &self,
        datetime: &chrono::DateTime<chrono::Local>,
        date_length: DateLength,
        time_length: TimeLength,
    ) -> String {
        format!(
            "{} {}",
            self.format_date(datetime, date_length),
            self.format_time(datetime, time_length)
        )
    }

    /// Get the locale identifier.
    pub fn locale(&self) -> String {
        self.locale.clone()
    }
}

#[cfg(not(feature = "localization"))]
impl Default for DateTimeFormatter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Currency Formatting
// ============================================================================

/// Currency code (ISO 4217).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CurrencyCode(pub String);

impl CurrencyCode {
    /// Create a new currency code.
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into().to_uppercase())
    }

    /// US Dollar
    pub fn usd() -> Self {
        Self::new("USD")
    }

    /// Euro
    pub fn eur() -> Self {
        Self::new("EUR")
    }

    /// British Pound
    pub fn gbp() -> Self {
        Self::new("GBP")
    }

    /// Japanese Yen
    pub fn jpy() -> Self {
        Self::new("JPY")
    }
}

impl fmt::Display for CurrencyCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Locale-aware currency formatter.
///
/// Note: Full ICU4X currency support is still in development.
/// This provides basic currency formatting with locale-aware number formatting.
pub struct CurrencyFormatter {
    number_formatter: NumberFormatter,
    currency: CurrencyCode,
}

impl CurrencyFormatter {
    /// Create a new currency formatter using the system locale and USD.
    pub fn new() -> Self {
        Self::with_currency(CurrencyCode::usd())
    }

    /// Create a currency formatter with a specific currency.
    pub fn with_currency(currency: CurrencyCode) -> Self {
        Self {
            number_formatter: NumberFormatter::new(),
            currency,
        }
    }

    /// Create a currency formatter with a specific locale and currency.
    pub fn with_locale_and_currency(locale: &str, currency: CurrencyCode) -> Self {
        Self {
            number_formatter: NumberFormatter::with_locale(locale),
            currency,
        }
    }

    /// Format a currency amount.
    ///
    /// Returns a string with the currency symbol and formatted number.
    pub fn format(&self, amount: f64) -> String {
        let symbol = self.currency_symbol();
        let formatted_number = self.number_formatter.format(amount);

        // Simple formatting - symbol before or after based on common conventions
        match self.currency.0.as_str() {
            "EUR" => format!("{formatted_number} {symbol}"),
            "JPY" | "CNY" | "KRW" => format!("{symbol}{}", self.number_formatter.format_integer(amount as i64)),
            _ => format!("{symbol}{formatted_number}"),
        }
    }

    /// Get the currency symbol for the current currency.
    ///
    /// Returns a common symbol for well-known currencies, or the currency code itself
    /// for unknown currencies.
    pub fn currency_symbol(&self) -> String {
        match self.currency.0.as_str() {
            "USD" => "$".to_string(),
            "EUR" => "\u{20ac}".to_string(), // €
            "GBP" => "\u{00a3}".to_string(), // £
            "JPY" => "\u{00a5}".to_string(), // ¥
            "CNY" => "\u{00a5}".to_string(), // ¥
            "KRW" => "\u{20a9}".to_string(), // ₩
            "INR" => "\u{20b9}".to_string(), // ₹
            "RUB" => "\u{20bd}".to_string(), // ₽
            "BRL" => "R$".to_string(),
            "CAD" => "CA$".to_string(),
            "AUD" => "A$".to_string(),
            "CHF" => "CHF".to_string(),
            "MXN" => "MX$".to_string(),
            _ => self.currency.0.clone(),
        }
    }

    /// Get the currency code.
    pub fn currency(&self) -> &CurrencyCode {
        &self.currency
    }

    /// Get the locale identifier being used.
    pub fn locale(&self) -> String {
        self.number_formatter.locale()
    }
}

impl Default for CurrencyFormatter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Locale Watcher
// ============================================================================

struct LocaleWatcherInner {
    locale_changed: Signal<LocaleInfo>,
    running: AtomicBool,
    stop: AtomicBool,
}

/// Watches for system locale changes.
///
/// This allows applications to be notified when the user changes
/// their system locale settings.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::platform::LocaleWatcher;
///
/// let watcher = LocaleWatcher::new()?;
///
/// watcher.locale_changed().connect(|info| {
///     println!("Locale changed to: {}", info.identifier);
/// });
///
/// watcher.start()?;
/// ```
pub struct LocaleWatcher {
    inner: Arc<LocaleWatcherInner>,
}

impl LocaleWatcher {
    /// Create a new locale watcher.
    pub fn new() -> Result<Self, LocalizationError> {
        Ok(Self {
            inner: Arc::new(LocaleWatcherInner {
                locale_changed: Signal::new(),
                running: AtomicBool::new(false),
                stop: AtomicBool::new(false),
            }),
        })
    }

    /// Signal emitted when the system locale changes.
    pub fn locale_changed(&self) -> &Signal<LocaleInfo> {
        &self.inner.locale_changed
    }

    /// Start watching for locale changes.
    ///
    /// This spawns a background thread that periodically checks for locale changes.
    pub fn start(&self) -> Result<(), LocalizationError> {
        if self.inner.running.swap(true, Ordering::SeqCst) {
            return Err(LocalizationError::detection("watcher already running"));
        }

        self.inner.stop.store(false, Ordering::SeqCst);
        let inner = Arc::clone(&self.inner);

        std::thread::spawn(move || {
            let mut last_locale = SystemLocale::current();
            let poll_interval = Duration::from_secs(2);

            while !inner.stop.load(Ordering::SeqCst) {
                std::thread::sleep(poll_interval);

                let current = SystemLocale::current();
                if current != last_locale {
                    let info = LocaleInfo::parse(&current);
                    inner.locale_changed.emit(info);
                    last_locale = current;
                }
            }

            inner.running.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    /// Stop watching for locale changes.
    pub fn stop(&self) {
        self.inner.stop.store(true, Ordering::SeqCst);
    }

    /// Check if the watcher is currently running.
    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }
}

impl Drop for LocaleWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_direction_detect_ltr() {
        assert_eq!(TextDirection::detect("Hello"), TextDirection::Ltr);
        assert_eq!(TextDirection::detect("Bonjour"), TextDirection::Ltr);
        assert_eq!(TextDirection::detect("123"), TextDirection::Ltr); // Numbers are neutral
        assert_eq!(TextDirection::detect(""), TextDirection::Ltr);
    }

    #[test]
    fn test_text_direction_detect_rtl() {
        // Arabic
        assert_eq!(TextDirection::detect("مرحبا"), TextDirection::Rtl);
        // Hebrew
        assert_eq!(TextDirection::detect("שלום"), TextDirection::Rtl);
    }

    #[test]
    fn test_text_direction_for_locale() {
        assert_eq!(TextDirection::for_locale("en-US"), TextDirection::Ltr);
        assert_eq!(TextDirection::for_locale("fr-FR"), TextDirection::Ltr);
        assert_eq!(TextDirection::for_locale("de"), TextDirection::Ltr);
        assert_eq!(TextDirection::for_locale("ar"), TextDirection::Rtl);
        assert_eq!(TextDirection::for_locale("ar-SA"), TextDirection::Rtl);
        assert_eq!(TextDirection::for_locale("he"), TextDirection::Rtl);
        assert_eq!(TextDirection::for_locale("fa-IR"), TextDirection::Rtl);
    }

    #[test]
    fn test_locale_info_parse() {
        let info = LocaleInfo::parse("en-US");
        assert_eq!(info.language, "en");
        assert_eq!(info.region, Some("US".to_string()));
        assert_eq!(info.direction, TextDirection::Ltr);

        let info = LocaleInfo::parse("ar-SA");
        assert_eq!(info.language, "ar");
        assert_eq!(info.region, Some("SA".to_string()));
        assert_eq!(info.direction, TextDirection::Rtl);

        let info = LocaleInfo::parse("zh-Hant-TW");
        assert_eq!(info.language, "zh");
        assert_eq!(info.script, Some("Hant".to_string()));
        assert_eq!(info.region, Some("TW".to_string()));
    }

    #[test]
    fn test_system_locale_current() {
        let locale = SystemLocale::current();
        // Should return something (even if just the default)
        assert!(!locale.is_empty());
    }

    #[test]
    fn test_number_formatter_basic() {
        let formatter = NumberFormatter::new();

        // Should format without panicking
        let _ = formatter.format_integer(1234567);
        let _ = formatter.format(1234.56);
        let _ = formatter.format_with_precision(1234.56789, 4);
    }

    #[test]
    fn test_currency_formatter() {
        let formatter = CurrencyFormatter::new();
        let formatted = formatter.format(1234.56);
        assert!(formatted.contains("$") || formatted.contains("1234"));

        let formatter = CurrencyFormatter::with_currency(CurrencyCode::eur());
        let formatted = formatter.format(1234.56);
        assert!(formatted.contains("\u{20ac}") || formatted.contains("EUR") || formatted.contains("1234"));
    }

    #[test]
    fn test_currency_symbols() {
        let formatter = CurrencyFormatter::with_currency(CurrencyCode::usd());
        assert_eq!(formatter.currency_symbol(), "$");

        let formatter = CurrencyFormatter::with_currency(CurrencyCode::eur());
        assert_eq!(formatter.currency_symbol(), "\u{20ac}");

        let formatter = CurrencyFormatter::with_currency(CurrencyCode::gbp());
        assert_eq!(formatter.currency_symbol(), "\u{00a3}");
    }

    #[test]
    fn test_localization_error() {
        let err = LocalizationError::detection("test error");
        assert!(err.to_string().contains("test error"));
        assert!(!err.is_invalid_locale());

        let err = LocalizationError::invalid_locale("bad locale");
        assert!(err.is_invalid_locale());
    }

    #[test]
    fn test_locale_watcher_creation() {
        let watcher = LocaleWatcher::new();
        assert!(watcher.is_ok());

        let watcher = watcher.unwrap();
        assert!(!watcher.is_running());
    }
}
