//! Timezone selection support for DateTimeEdit widget.
//!
//! This module provides timezone selection functionality including:
//! - A `TimezoneComboModel` implementing `ComboBoxModel` for timezone dropdown
//! - Common timezone list for quick selection
//! - Timezone formatting helpers

use chrono::{Offset, Utc};
use chrono_tz::Tz;

use super::combo_box::{ComboBoxItem, ComboBoxModel};

/// Common timezones shown at the top of the timezone picker.
pub const COMMON_TIMEZONES: &[&str] = &[
    "UTC",
    "America/New_York",
    "America/Chicago",
    "America/Denver",
    "America/Los_Angeles",
    "America/Anchorage",
    "Pacific/Honolulu",
    "Europe/London",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Moscow",
    "Asia/Tokyo",
    "Asia/Shanghai",
    "Asia/Kolkata",
    "Asia/Dubai",
    "Asia/Singapore",
    "Australia/Sydney",
    "Pacific/Auckland",
];

/// How to display timezone names in the picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimezoneDisplayFormat {
    /// Show IANA name only (e.g., "America/New_York")
    #[default]
    IanaName,
    /// Show IANA name with current offset (e.g., "America/New_York (UTC-05:00)")
    IanaNameWithOffset,
    /// Show abbreviation with offset (e.g., "EST (UTC-05:00)")
    AbbreviationWithOffset,
}

/// A ComboBox model providing timezone selection.
///
/// This model provides all IANA timezones from the `chrono-tz` database,
/// with common timezones shown first for convenience.
#[derive(Debug)]
pub struct TimezoneComboModel {
    /// All available timezones (common ones first).
    timezones: Vec<Tz>,
    /// Display format for timezone items.
    display_format: TimezoneDisplayFormat,
    /// Cached display strings.
    display_cache: Vec<String>,
}

impl Default for TimezoneComboModel {
    fn default() -> Self {
        Self::new()
    }
}

impl TimezoneComboModel {
    /// Create a new timezone model with all IANA timezones.
    ///
    /// Common timezones are placed at the beginning of the list.
    pub fn new() -> Self {
        Self::with_format(TimezoneDisplayFormat::default())
    }

    /// Create a new timezone model with a specific display format.
    pub fn with_format(format: TimezoneDisplayFormat) -> Self {
        let mut timezones = Vec::new();

        // Add common timezones first
        for name in COMMON_TIMEZONES {
            if let Ok(tz) = name.parse::<Tz>() {
                timezones.push(tz);
            }
        }

        // Add all other timezones (excluding common ones already added)
        for tz in chrono_tz::TZ_VARIANTS {
            let name = tz.name();
            if !COMMON_TIMEZONES.contains(&name) {
                timezones.push(tz);
            }
        }

        let display_cache = timezones
            .iter()
            .map(|tz| format_timezone(*tz, format))
            .collect();

        Self {
            timezones,
            display_format: format,
            display_cache,
        }
    }

    /// Create a timezone model with only common timezones.
    pub fn common_only() -> Self {
        Self::common_only_with_format(TimezoneDisplayFormat::default())
    }

    /// Create a timezone model with only common timezones and specific format.
    pub fn common_only_with_format(format: TimezoneDisplayFormat) -> Self {
        let timezones: Vec<Tz> = COMMON_TIMEZONES
            .iter()
            .filter_map(|name| name.parse::<Tz>().ok())
            .collect();

        let display_cache = timezones
            .iter()
            .map(|tz| format_timezone(*tz, format))
            .collect();

        Self {
            timezones,
            display_format: format,
            display_cache,
        }
    }

    /// Get the timezone at the given index.
    pub fn timezone(&self, index: usize) -> Option<Tz> {
        self.timezones.get(index).copied()
    }

    /// Find the index of a timezone by its IANA name.
    pub fn find_timezone(&self, tz: Tz) -> Option<usize> {
        self.timezones.iter().position(|&t| t == tz)
    }

    /// Get the display format.
    pub fn display_format(&self) -> TimezoneDisplayFormat {
        self.display_format
    }

    /// Set the display format and rebuild cache.
    pub fn set_display_format(&mut self, format: TimezoneDisplayFormat) {
        if self.display_format != format {
            self.display_format = format;
            self.display_cache = self
                .timezones
                .iter()
                .map(|tz| format_timezone(*tz, format))
                .collect();
        }
    }

    /// Refresh the display cache (call after DST changes if needed).
    pub fn refresh_cache(&mut self) {
        self.display_cache = self
            .timezones
            .iter()
            .map(|tz| format_timezone(*tz, self.display_format))
            .collect();
    }
}

impl ComboBoxModel for TimezoneComboModel {
    fn row_count(&self) -> usize {
        self.timezones.len()
    }

    fn item(&self, index: usize) -> Option<ComboBoxItem> {
        self.display_cache
            .get(index)
            .map(|text| ComboBoxItem::new(text.clone()))
    }

    fn text(&self, index: usize) -> Option<String> {
        self.display_cache.get(index).cloned()
    }

    fn icon(&self, _index: usize) -> Option<horizon_lattice_render::Icon> {
        None
    }

    fn find_text(&self, text: &str) -> Option<usize> {
        // First try exact match on display text
        if let Some(idx) = self.display_cache.iter().position(|t| t == text) {
            return Some(idx);
        }
        // Also try matching by IANA name
        self.timezones.iter().position(|tz| tz.name() == text)
    }

    fn filter(&self, prefix: &str, case_insensitive: bool) -> Vec<usize> {
        let prefix_cmp = if case_insensitive {
            prefix.to_lowercase()
        } else {
            prefix.to_string()
        };

        (0..self.timezones.len())
            .filter(|&i| {
                // Match against display text
                let display_match = if let Some(text) = self.display_cache.get(i) {
                    let text_cmp = if case_insensitive {
                        text.to_lowercase()
                    } else {
                        text.clone()
                    };
                    text_cmp.contains(&prefix_cmp)
                } else {
                    false
                };

                // Also match against IANA name
                let name_match = {
                    let name = self.timezones[i].name();
                    let name_cmp = if case_insensitive {
                        name.to_lowercase()
                    } else {
                        name.to_string()
                    };
                    name_cmp.contains(&prefix_cmp)
                };

                display_match || name_match
            })
            .collect()
    }
}

/// Format a timezone for display.
pub fn format_timezone(tz: Tz, format: TimezoneDisplayFormat) -> String {
    let now = Utc::now().with_timezone(&tz);
    let offset = now.offset();
    let offset_str = format_utc_offset(offset.fix().local_minus_utc());

    match format {
        TimezoneDisplayFormat::IanaName => tz.name().to_string(),
        TimezoneDisplayFormat::IanaNameWithOffset => {
            format!("{} ({})", tz.name(), offset_str)
        }
        TimezoneDisplayFormat::AbbreviationWithOffset => {
            let abbrev = now.format("%Z").to_string();
            format!("{} ({})", abbrev, offset_str)
        }
    }
}

/// Format a UTC offset in seconds to a string like "UTC+05:30" or "UTC-08:00".
pub fn format_utc_offset(offset_seconds: i32) -> String {
    let sign = if offset_seconds >= 0 { '+' } else { '-' };
    let abs_seconds = offset_seconds.abs();
    let hours = abs_seconds / 3600;
    let minutes = (abs_seconds % 3600) / 60;

    if minutes == 0 {
        format!("UTC{}{:02}:00", sign, hours)
    } else {
        format!("UTC{}{:02}:{:02}", sign, hours, minutes)
    }
}

/// Get the current UTC offset for a timezone in seconds.
pub fn get_utc_offset_seconds(tz: Tz) -> i32 {
    let now = Utc::now().with_timezone(&tz);
    now.offset().fix().local_minus_utc()
}

/// Get the timezone abbreviation (e.g., "EST", "PST", "UTC").
pub fn get_timezone_abbreviation(tz: Tz) -> String {
    let now = Utc::now().with_timezone(&tz);
    now.format("%Z").to_string()
}

/// Try to determine the local system timezone.
///
/// Returns `None` if the timezone cannot be determined.
pub fn local_timezone() -> Option<Tz> {
    // Try to get from environment variable
    if let Ok(tz_name) = std::env::var("TZ")
        && let Ok(tz) = tz_name.parse::<Tz>() {
            return Some(tz);
        }

    // Try to read from /etc/localtime symlink on Unix
    #[cfg(unix)]
    {
        if let Ok(link) = std::fs::read_link("/etc/localtime") {
            let path = link.to_string_lossy();
            // Path is usually like /usr/share/zoneinfo/America/New_York
            if let Some(idx) = path.find("zoneinfo/") {
                let tz_name = &path[idx + 9..];
                if let Ok(tz) = tz_name.parse::<Tz>() {
                    return Some(tz);
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timezone_model_creation() {
        let model = TimezoneComboModel::new();
        assert!(model.row_count() > 0);
        // Common timezones should be at the start
        assert!(model.row_count() >= COMMON_TIMEZONES.len());
    }

    #[test]
    fn test_common_timezones_first() {
        let model = TimezoneComboModel::new();
        // First timezone should be UTC
        assert_eq!(model.timezone(0), Some(chrono_tz::UTC));
        // Check that common timezones are at the start
        for (i, name) in COMMON_TIMEZONES.iter().enumerate() {
            if let Ok(expected_tz) = name.parse::<Tz>() {
                assert_eq!(
                    model.timezone(i),
                    Some(expected_tz),
                    "Expected {} at index {}",
                    name,
                    i
                );
            }
        }
    }

    #[test]
    fn test_find_timezone() {
        let model = TimezoneComboModel::new();
        let idx = model.find_timezone(chrono_tz::America::New_York);
        assert!(idx.is_some());
        assert_eq!(
            model.timezone(idx.unwrap()),
            Some(chrono_tz::America::New_York)
        );
    }

    #[test]
    fn test_format_utc_offset() {
        assert_eq!(format_utc_offset(0), "UTC+00:00");
        assert_eq!(format_utc_offset(3600), "UTC+01:00");
        assert_eq!(format_utc_offset(-18000), "UTC-05:00"); // EST
        assert_eq!(format_utc_offset(19800), "UTC+05:30"); // IST
        assert_eq!(format_utc_offset(-28800), "UTC-08:00"); // PST
    }

    #[test]
    fn test_filter_timezones() {
        let model = TimezoneComboModel::new();
        let results = model.filter("America", true);
        assert!(!results.is_empty());
        // All results should contain "America" in the name
        for idx in results {
            let tz = model.timezone(idx).unwrap();
            assert!(
                tz.name().contains("America"),
                "Expected timezone containing 'America', got {}",
                tz.name()
            );
        }
    }

    #[test]
    fn test_common_only_model() {
        let model = TimezoneComboModel::common_only();
        assert_eq!(model.row_count(), COMMON_TIMEZONES.len());
    }

    #[test]
    fn test_display_formats() {
        let tz = chrono_tz::UTC;

        let iana = format_timezone(tz, TimezoneDisplayFormat::IanaName);
        assert_eq!(iana, "UTC");

        let with_offset = format_timezone(tz, TimezoneDisplayFormat::IanaNameWithOffset);
        assert!(with_offset.starts_with("UTC"));
        assert!(with_offset.contains("UTC+00:00") || with_offset.contains("(UTC"));
    }
}
