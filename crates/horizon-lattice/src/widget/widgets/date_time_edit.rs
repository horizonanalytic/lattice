//! DateTimeEdit widget for combined date and time input.
//!
//! The DateTimeEdit widget provides a way to enter and modify both date and time with:
//! - Section-based editing for all date and time parts
//! - Increment/decrement buttons
//! - Calendar popup for date selection
//! - Date and time format customization
//! - Range constraints
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{DateTimeEdit, DateFormat, TimeFormat};
//! use chrono::NaiveDateTime;
//!
//! // Create a datetime editor
//! let mut dt_edit = DateTimeEdit::new()
//!     .with_date_format(DateFormat::Short)
//!     .with_time_format(TimeFormat::Hour12)
//!     .with_calendar_popup(true);
//!
//! // Connect to datetime changes
//! dt_edit.datetime_changed.connect(|dt| {
//!     println!("DateTime changed: {}", dt);
//! });
//! ```

use chrono::{
    DateTime, Datelike, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike, Utc,
};
use chrono_tz::Tz;
use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, HorizontalAlign, Point, Rect, Renderer, RoundedRect,
    Stroke, TextLayout, TextLayoutOptions, TextRenderer, VerticalAlign,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, WheelEvent, Widget,
    WidgetBase, WidgetEvent,
};

use super::calendar::CalendarWidget;
use super::combo_box::ComboBox;
use super::date_edit::DateFormat;
use super::time_edit::TimeFormat;
use super::timezone::{TimezoneComboModel, get_timezone_abbreviation};

/// How to display the timezone in the DateTimeEdit widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimezoneDisplay {
    /// Don't display timezone (default for backward compatibility).
    #[default]
    Hidden,
    /// Display timezone abbreviation (e.g., "PST", "EST", "UTC").
    Abbreviation,
    /// Display UTC offset (e.g., "+08:00", "-05:00").
    UtcOffset,
    /// Display both abbreviation and offset (e.g., "PST -08:00").
    Full,
    /// Display IANA name (e.g., "America/Los_Angeles").
    IanaName,
}

/// Parts of the DateTimeEdit for hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DateTimeEditPart {
    #[default]
    None,
    // Date sections
    MonthSection,
    DaySection,
    YearSection,
    // Time sections
    HourSection,
    MinuteSection,
    SecondSection,
    AmPmSection,
    // Timezone section
    TimezoneSection,
    // Buttons
    CalendarButton,
    TimezoneButton,
    UpButton,
    DownButton,
    // Popups
    PopupCalendar,
    PopupTimezone,
}

/// Which section is currently focused for editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum EditSection {
    #[default]
    None,
    // Date
    Month,
    Day,
    Year,
    // Time
    Hour,
    Minute,
    Second,
    AmPm,
    // Timezone
    Timezone,
}

impl EditSection {
    fn is_date_section(&self) -> bool {
        matches!(
            self,
            EditSection::Month | EditSection::Day | EditSection::Year
        )
    }

    fn is_time_section(&self) -> bool {
        matches!(
            self,
            EditSection::Hour | EditSection::Minute | EditSection::Second | EditSection::AmPm
        )
    }

    fn is_timezone_section(&self) -> bool {
        matches!(self, EditSection::Timezone)
    }
}

/// A widget for entering and modifying dates and times.
///
/// DateTimeEdit combines date and time editing in a single widget with
/// support for calendar popup, multiple formats, and range constraints.
///
/// # Signals
///
/// - `datetime_changed(NaiveDateTime)`: Emitted when datetime changes
/// - `date_changed(NaiveDate)`: Emitted when just the date changes
/// - `time_changed(NaiveTime)`: Emitted when just the time changes
/// - `editing_finished()`: Emitted when editing is completed
pub struct DateTimeEdit {
    /// Widget base.
    base: WidgetBase,

    /// Current datetime value.
    datetime: NaiveDateTime,

    /// Minimum selectable datetime.
    minimum_datetime: NaiveDateTime,

    /// Maximum selectable datetime.
    maximum_datetime: NaiveDateTime,

    /// Date display format.
    date_format: DateFormat,

    /// Time display format.
    time_format: TimeFormat,

    /// Whether to show seconds (overrides time format).
    show_seconds: Option<bool>,

    /// Whether to show the calendar popup button.
    calendar_popup: bool,

    /// Separator between date and time.
    separator: String,

    /// Current editing section.
    current_section: EditSection,

    /// Whether read-only.
    read_only: bool,

    // Appearance
    background_color: Color,
    text_color: Color,
    border_color: Color,
    button_color: Color,
    button_hover_color: Color,
    button_pressed_color: Color,
    section_highlight_color: Color,
    font: Font,
    border_radius: f32,
    button_width: f32,
    calendar_button_width: f32,

    /// Which part is currently hovered.
    hover_part: DateTimeEditPart,
    /// Which part is currently pressed.
    pressed_part: DateTimeEditPart,
    /// Whether the calendar popup is visible.
    popup_visible: bool,

    /// Internal calendar widget for popup.
    calendar: CalendarWidget,

    // Timezone support
    /// Current timezone (None = naive/local time).
    timezone: Option<Tz>,
    /// How to display the timezone.
    timezone_display: TimezoneDisplay,
    /// Whether to show the timezone picker button.
    show_timezone_picker: bool,
    /// Timezone picker button width.
    timezone_button_width: f32,
    /// Internal ComboBox for timezone selection.
    timezone_combo: ComboBox,
    /// Whether the timezone popup is visible.
    timezone_popup_visible: bool,

    /// Signal emitted when datetime changes.
    pub datetime_changed: Signal<NaiveDateTime>,
    /// Signal emitted when date changes.
    pub date_changed: Signal<NaiveDate>,
    /// Signal emitted when time changes.
    pub time_changed: Signal<NaiveTime>,
    /// Signal emitted when timezone changes.
    pub timezone_changed: Signal<Option<Tz>>,
    /// Signal emitted when editing is finished.
    pub editing_finished: Signal<()>,
}

impl DateTimeEdit {
    /// Create a new DateTimeEdit with current date and time.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Fixed,
        ));

        let now = Local::now().naive_local();

        let mut calendar = CalendarWidget::new();
        calendar.set_selected_date(Some(now.date()));

        // Create timezone combo with all IANA timezones
        let mut timezone_combo = ComboBox::new()
            .with_editable(true)
            .with_case_insensitive(true)
            .with_placeholder("Select timezone...");
        timezone_combo.set_model(Box::new(TimezoneComboModel::new()));

        Self {
            base,
            datetime: now,
            minimum_datetime: NaiveDateTime::new(
                NaiveDate::from_ymd_opt(1752, 9, 14).unwrap(),
                NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            ),
            maximum_datetime: NaiveDateTime::new(
                NaiveDate::from_ymd_opt(9999, 12, 31).unwrap(),
                NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            ),
            date_format: DateFormat::Short,
            time_format: TimeFormat::Hour24,
            show_seconds: None,
            calendar_popup: true,
            separator: " ".to_string(),
            current_section: EditSection::None,
            read_only: false,
            background_color: Color::WHITE,
            text_color: Color::BLACK,
            border_color: Color::from_rgb8(180, 180, 180),
            button_color: Color::from_rgb8(240, 240, 240),
            button_hover_color: Color::from_rgb8(220, 220, 220),
            button_pressed_color: Color::from_rgb8(200, 200, 200),
            section_highlight_color: Color::from_rgba8(51, 153, 255, 80),
            font: Font::new(FontFamily::SansSerif, 13.0),
            border_radius: 4.0,
            button_width: 20.0,
            calendar_button_width: 24.0,
            hover_part: DateTimeEditPart::None,
            pressed_part: DateTimeEditPart::None,
            popup_visible: false,
            calendar,
            // Timezone fields (default: disabled for backward compatibility)
            timezone: None,
            timezone_display: TimezoneDisplay::Hidden,
            show_timezone_picker: false,
            timezone_button_width: 24.0,
            timezone_combo,
            timezone_popup_visible: false,
            datetime_changed: Signal::new(),
            date_changed: Signal::new(),
            time_changed: Signal::new(),
            timezone_changed: Signal::new(),
            editing_finished: Signal::new(),
        }
    }

    // =========================================================================
    // DateTime Access
    // =========================================================================

    /// Get the current datetime.
    pub fn datetime(&self) -> NaiveDateTime {
        self.datetime
    }

    /// Set the current datetime.
    pub fn set_datetime(&mut self, datetime: NaiveDateTime) {
        let clamped = datetime.clamp(self.minimum_datetime, self.maximum_datetime);
        if self.datetime != clamped {
            let old_date = self.datetime.date();
            let old_time = self.datetime.time();
            self.datetime = clamped;
            self.calendar.set_selected_date(Some(clamped.date()));
            self.calendar.show_date(clamped.date());
            self.base.update();

            self.datetime_changed.emit(clamped);
            if old_date != clamped.date() {
                self.date_changed.emit(clamped.date());
            }
            if old_time != clamped.time() {
                self.time_changed.emit(clamped.time());
            }
        }
    }

    /// Set datetime using builder pattern.
    pub fn with_datetime(mut self, datetime: NaiveDateTime) -> Self {
        self.datetime = datetime.clamp(self.minimum_datetime, self.maximum_datetime);
        self.calendar.set_selected_date(Some(self.datetime.date()));
        self.calendar.show_date(self.datetime.date());
        self
    }

    /// Get the current date.
    pub fn date(&self) -> NaiveDate {
        self.datetime.date()
    }

    /// Set the date while keeping the time.
    pub fn set_date(&mut self, date: NaiveDate) {
        let new_datetime = NaiveDateTime::new(date, self.datetime.time());
        self.set_datetime(new_datetime);
    }

    /// Get the current time.
    pub fn time(&self) -> NaiveTime {
        self.datetime.time()
    }

    /// Set the time while keeping the date.
    pub fn set_time(&mut self, time: NaiveTime) {
        let new_datetime = NaiveDateTime::new(self.datetime.date(), time);
        self.set_datetime(new_datetime);
    }

    // =========================================================================
    // DateTime Constraints
    // =========================================================================

    /// Get the minimum datetime.
    pub fn minimum_datetime(&self) -> NaiveDateTime {
        self.minimum_datetime
    }

    /// Set the minimum datetime.
    pub fn set_minimum_datetime(&mut self, datetime: NaiveDateTime) {
        self.minimum_datetime = datetime;
        self.calendar.set_minimum_date(Some(datetime.date()));
        if self.datetime < datetime {
            self.set_datetime(datetime);
        }
    }

    /// Set minimum datetime using builder pattern.
    pub fn with_minimum_datetime(mut self, datetime: NaiveDateTime) -> Self {
        self.minimum_datetime = datetime;
        self.calendar.set_minimum_date(Some(datetime.date()));
        if self.datetime < datetime {
            self.datetime = datetime;
        }
        self
    }

    /// Get the maximum datetime.
    pub fn maximum_datetime(&self) -> NaiveDateTime {
        self.maximum_datetime
    }

    /// Set the maximum datetime.
    pub fn set_maximum_datetime(&mut self, datetime: NaiveDateTime) {
        self.maximum_datetime = datetime;
        self.calendar.set_maximum_date(Some(datetime.date()));
        if self.datetime > datetime {
            self.set_datetime(datetime);
        }
    }

    /// Set maximum datetime using builder pattern.
    pub fn with_maximum_datetime(mut self, datetime: NaiveDateTime) -> Self {
        self.maximum_datetime = datetime;
        self.calendar.set_maximum_date(Some(datetime.date()));
        if self.datetime > datetime {
            self.datetime = datetime;
        }
        self
    }

    /// Set the datetime range.
    pub fn set_datetime_range(&mut self, min: NaiveDateTime, max: NaiveDateTime) {
        self.minimum_datetime = min;
        self.maximum_datetime = max;
        self.calendar.set_date_range(min.date(), max.date());
        let clamped = self.datetime.clamp(min, max);
        if self.datetime != clamped {
            self.set_datetime(clamped);
        }
    }

    /// Set datetime range using builder pattern.
    pub fn with_datetime_range(mut self, min: NaiveDateTime, max: NaiveDateTime) -> Self {
        self.minimum_datetime = min;
        self.maximum_datetime = max;
        self.calendar.set_date_range(min.date(), max.date());
        self.datetime = self.datetime.clamp(min, max);
        self
    }

    // =========================================================================
    // Display Options
    // =========================================================================

    /// Get the date display format.
    pub fn date_format(&self) -> DateFormat {
        self.date_format
    }

    /// Set the date display format.
    pub fn set_date_format(&mut self, format: DateFormat) {
        if self.date_format != format {
            self.date_format = format;
            self.base.update();
        }
    }

    /// Set date format using builder pattern.
    pub fn with_date_format(mut self, format: DateFormat) -> Self {
        self.date_format = format;
        self
    }

    /// Get the time display format.
    pub fn time_format(&self) -> TimeFormat {
        self.time_format
    }

    /// Set the time display format.
    pub fn set_time_format(&mut self, format: TimeFormat) {
        if self.time_format != format {
            self.time_format = format;
            self.base.update();
        }
    }

    /// Set time format using builder pattern.
    pub fn with_time_format(mut self, format: TimeFormat) -> Self {
        self.time_format = format;
        self
    }

    /// Check if seconds are shown.
    pub fn seconds_shown(&self) -> bool {
        self.show_seconds
            .unwrap_or_else(|| self.time_format.shows_seconds())
    }

    /// Set whether to show seconds.
    pub fn set_show_seconds(&mut self, show: bool) {
        if self.show_seconds != Some(show) {
            self.show_seconds = Some(show);
            self.base.update();
        }
    }

    /// Set show seconds using builder pattern.
    pub fn with_show_seconds(mut self, show: bool) -> Self {
        self.show_seconds = Some(show);
        self
    }

    /// Check if calendar popup is enabled.
    pub fn calendar_popup_enabled(&self) -> bool {
        self.calendar_popup
    }

    /// Set whether calendar popup is enabled.
    pub fn set_calendar_popup(&mut self, enabled: bool) {
        if self.calendar_popup != enabled {
            self.calendar_popup = enabled;
            self.base.update();
        }
    }

    /// Set calendar popup using builder pattern.
    pub fn with_calendar_popup(mut self, enabled: bool) -> Self {
        self.calendar_popup = enabled;
        self
    }

    /// Get the separator between date and time.
    pub fn separator(&self) -> &str {
        &self.separator
    }

    /// Set the separator between date and time.
    pub fn set_separator(&mut self, separator: impl Into<String>) {
        self.separator = separator.into();
        self.base.update();
    }

    /// Set separator using builder pattern.
    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }

    /// Check if the widget is read-only.
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Set whether the widget is read-only.
    pub fn set_read_only(&mut self, read_only: bool) {
        if self.read_only != read_only {
            self.read_only = read_only;
            if read_only {
                self.current_section = EditSection::None;
                self.hide_popup();
            }
            self.base.update();
        }
    }

    /// Set read-only using builder pattern.
    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    // =========================================================================
    // Popup Control
    // =========================================================================

    /// Show the calendar popup.
    pub fn show_popup(&mut self) {
        if !self.calendar_popup || self.read_only {
            return;
        }
        self.popup_visible = true;
        self.calendar.show_date(self.datetime.date());
        self.calendar.set_selected_date(Some(self.datetime.date()));
        self.base.update();
    }

    /// Hide the calendar popup.
    pub fn hide_popup(&mut self) {
        if self.popup_visible {
            self.popup_visible = false;
            self.base.update();
        }
    }

    /// Toggle the calendar popup.
    pub fn toggle_popup(&mut self) {
        if self.popup_visible {
            self.hide_popup();
        } else {
            self.show_popup();
        }
    }

    // =========================================================================
    // Timezone Popup
    // =========================================================================

    /// Show the timezone picker popup.
    pub fn show_timezone_popup(&mut self) {
        if !self.show_timezone_picker || self.read_only {
            return;
        }
        self.timezone_popup_visible = true;
        // Sync the combo selection with current timezone
        if let Some(tz) = self.timezone {
            // Search by IANA name in the combo's model
            if let Some(idx) = self.timezone_combo.find_text(tz.name()) {
                self.timezone_combo.set_current_index(idx as i32);
            }
        }
        self.base.update();
    }

    /// Hide the timezone picker popup.
    pub fn hide_timezone_popup(&mut self) {
        if self.timezone_popup_visible {
            self.timezone_popup_visible = false;
            self.base.update();
        }
    }

    /// Toggle the timezone picker popup.
    pub fn toggle_timezone_popup(&mut self) {
        if self.timezone_popup_visible {
            self.hide_timezone_popup();
        } else {
            self.show_timezone_popup();
        }
    }

    // =========================================================================
    // Timezone Access
    // =========================================================================

    /// Get the current timezone (None = naive/local time).
    pub fn timezone(&self) -> Option<Tz> {
        self.timezone
    }

    /// Set the current timezone.
    ///
    /// Setting a timezone enables timezone-aware display and conversions.
    /// Use `None` to return to naive/local time mode.
    pub fn set_timezone(&mut self, tz: Option<Tz>) {
        if self.timezone != tz {
            self.timezone = tz;
            // Sync combo selection
            if let Some(tz) = tz
                && let Some(idx) = self.timezone_combo.find_text(tz.name())
            {
                self.timezone_combo.set_current_index(idx as i32);
            }
            self.base.update();
            self.timezone_changed.emit(tz);
        }
    }

    /// Set timezone using builder pattern.
    pub fn with_timezone(mut self, tz: Tz) -> Self {
        self.timezone = Some(tz);
        self
    }

    /// Set to local system timezone using builder pattern.
    pub fn with_local_timezone(mut self) -> Self {
        if let Some(tz) = super::timezone::local_timezone() {
            self.timezone = Some(tz);
        }
        self
    }

    /// Set to UTC using builder pattern.
    pub fn with_utc(self) -> Self {
        self.with_timezone(chrono_tz::UTC)
    }

    /// Get the datetime with timezone applied.
    ///
    /// Returns `None` if no timezone is set.
    pub fn datetime_with_timezone(&self) -> Option<DateTime<Tz>> {
        self.timezone.map(|tz| {
            tz.from_local_datetime(&self.datetime)
                .single()
                .unwrap_or_else(|| tz.from_utc_datetime(&self.datetime))
        })
    }

    /// Get the datetime converted to UTC.
    ///
    /// Returns `None` if no timezone is set.
    pub fn datetime_utc(&self) -> Option<DateTime<Utc>> {
        self.datetime_with_timezone()
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// Set datetime from a timezone-aware value.
    ///
    /// The timezone is also updated to match the input.
    pub fn set_datetime_with_timezone<T: TimeZone>(&mut self, dt: DateTime<T>)
    where
        T::Offset: std::fmt::Display,
    {
        // Convert to naive local time
        let naive = dt.naive_local();
        self.set_datetime(naive);
    }

    /// Get how timezone is displayed.
    pub fn timezone_display(&self) -> TimezoneDisplay {
        self.timezone_display
    }

    /// Set how timezone is displayed.
    pub fn set_timezone_display(&mut self, display: TimezoneDisplay) {
        if self.timezone_display != display {
            self.timezone_display = display;
            self.base.update();
        }
    }

    /// Set timezone display using builder pattern.
    pub fn with_timezone_display(mut self, display: TimezoneDisplay) -> Self {
        self.timezone_display = display;
        self
    }

    /// Check if timezone picker is enabled.
    pub fn timezone_picker_enabled(&self) -> bool {
        self.show_timezone_picker
    }

    /// Enable or disable the timezone picker button.
    pub fn set_timezone_picker(&mut self, enabled: bool) {
        if self.show_timezone_picker != enabled {
            self.show_timezone_picker = enabled;
            self.base.update();
        }
    }

    /// Set timezone picker enabled using builder pattern.
    pub fn with_timezone_picker(mut self, enabled: bool) -> Self {
        self.show_timezone_picker = enabled;
        self
    }

    // =========================================================================
    // Timezone Conversion
    // =========================================================================

    /// Convert the current datetime to a different timezone.
    ///
    /// This updates both the displayed datetime and the timezone.
    pub fn convert_to_timezone(&mut self, tz: Tz) {
        if let Some(current_dt) = self.datetime_with_timezone() {
            let new_dt = current_dt.with_timezone(&tz);
            self.datetime = new_dt.naive_local();
            self.timezone = Some(tz);
            self.base.update();
            self.datetime_changed.emit(self.datetime);
            self.timezone_changed.emit(Some(tz));
        } else {
            // Just set the timezone without conversion
            self.set_timezone(Some(tz));
        }
    }

    /// Convert the current datetime to UTC.
    pub fn convert_to_utc(&mut self) {
        self.convert_to_timezone(chrono_tz::UTC);
    }

    /// Convert the current datetime to local system timezone.
    pub fn convert_to_local(&mut self) {
        if let Some(tz) = super::timezone::local_timezone() {
            self.convert_to_timezone(tz);
        }
    }

    // =========================================================================
    // Section Navigation
    // =========================================================================

    fn is_12_hour(&self) -> bool {
        self.time_format.is_12_hour()
    }

    fn get_section_order(&self) -> Vec<EditSection> {
        let mut sections = Vec::new();

        // Date sections based on format
        match self.date_format {
            DateFormat::Short | DateFormat::Long => {
                sections.push(EditSection::Month);
                sections.push(EditSection::Day);
                sections.push(EditSection::Year);
            }
            DateFormat::ISO => {
                sections.push(EditSection::Year);
                sections.push(EditSection::Month);
                sections.push(EditSection::Day);
            }
        }

        // Time sections
        sections.push(EditSection::Hour);
        sections.push(EditSection::Minute);
        if self.seconds_shown() {
            sections.push(EditSection::Second);
        }
        if self.is_12_hour() {
            sections.push(EditSection::AmPm);
        }

        // Add timezone section if timezone display is not hidden
        if self.timezone_display != TimezoneDisplay::Hidden {
            sections.push(EditSection::Timezone);
        }

        sections
    }

    fn next_section(&mut self) {
        let sections = self.get_section_order();
        let current_idx = sections.iter().position(|&s| s == self.current_section);

        self.current_section = match current_idx {
            None => sections.first().copied().unwrap_or(EditSection::None),
            Some(idx) => {
                if idx + 1 < sections.len() {
                    sections[idx + 1]
                } else {
                    EditSection::None
                }
            }
        };
        self.base.update();
    }

    fn previous_section(&mut self) {
        let sections = self.get_section_order();
        let current_idx = sections.iter().position(|&s| s == self.current_section);

        self.current_section = match current_idx {
            None => sections.last().copied().unwrap_or(EditSection::None),
            Some(0) => EditSection::None,
            Some(idx) => sections[idx - 1],
        };
        self.base.update();
    }

    fn step_up_section(&mut self) {
        if self.read_only {
            return;
        }

        let new_datetime = match self.current_section {
            EditSection::None => return,
            // Date sections
            EditSection::Month => {
                let date = self.datetime.date();
                let (new_year, new_month) = if date.month() == 12 {
                    (date.year() + 1, 1)
                } else {
                    (date.year(), date.month() + 1)
                };
                let max_day = days_in_month(new_year, new_month);
                let day = date.day().min(max_day);
                NaiveDate::from_ymd_opt(new_year, new_month, day)
                    .map(|d| NaiveDateTime::new(d, self.datetime.time()))
            }
            EditSection::Day => {
                let date = self.datetime.date();
                let max_day = days_in_month(date.year(), date.month());
                let new_day = if date.day() >= max_day {
                    1
                } else {
                    date.day() + 1
                };
                NaiveDate::from_ymd_opt(date.year(), date.month(), new_day)
                    .map(|d| NaiveDateTime::new(d, self.datetime.time()))
            }
            EditSection::Year => {
                let date = self.datetime.date();
                let new_year = date.year() + 1;
                let max_day = days_in_month(new_year, date.month());
                let day = date.day().min(max_day);
                NaiveDate::from_ymd_opt(new_year, date.month(), day)
                    .map(|d| NaiveDateTime::new(d, self.datetime.time()))
            }
            // Time sections
            EditSection::Hour => {
                let new_hour = (self.datetime.time().hour() + 1) % 24;
                NaiveTime::from_hms_opt(
                    new_hour,
                    self.datetime.time().minute(),
                    self.datetime.time().second(),
                )
                .map(|t| NaiveDateTime::new(self.datetime.date(), t))
            }
            EditSection::Minute => {
                let new_minute = (self.datetime.time().minute() + 1) % 60;
                NaiveTime::from_hms_opt(
                    self.datetime.time().hour(),
                    new_minute,
                    self.datetime.time().second(),
                )
                .map(|t| NaiveDateTime::new(self.datetime.date(), t))
            }
            EditSection::Second => {
                let new_second = (self.datetime.time().second() + 1) % 60;
                NaiveTime::from_hms_opt(
                    self.datetime.time().hour(),
                    self.datetime.time().minute(),
                    new_second,
                )
                .map(|t| NaiveDateTime::new(self.datetime.date(), t))
            }
            EditSection::AmPm => {
                let new_hour = (self.datetime.time().hour() + 12) % 24;
                NaiveTime::from_hms_opt(
                    new_hour,
                    self.datetime.time().minute(),
                    self.datetime.time().second(),
                )
                .map(|t| NaiveDateTime::new(self.datetime.date(), t))
            }
            EditSection::Timezone => {
                // Open the timezone picker instead of stepping
                self.show_timezone_popup();
                return;
            }
        };

        if let Some(dt) = new_datetime {
            self.set_datetime(dt);
        }
    }

    fn step_down_section(&mut self) {
        if self.read_only {
            return;
        }

        let new_datetime = match self.current_section {
            EditSection::None => return,
            // Date sections
            EditSection::Month => {
                let date = self.datetime.date();
                let (new_year, new_month) = if date.month() == 1 {
                    (date.year() - 1, 12)
                } else {
                    (date.year(), date.month() - 1)
                };
                let max_day = days_in_month(new_year, new_month);
                let day = date.day().min(max_day);
                NaiveDate::from_ymd_opt(new_year, new_month, day)
                    .map(|d| NaiveDateTime::new(d, self.datetime.time()))
            }
            EditSection::Day => {
                let date = self.datetime.date();
                let max_day = days_in_month(date.year(), date.month());
                let new_day = if date.day() == 1 {
                    max_day
                } else {
                    date.day() - 1
                };
                NaiveDate::from_ymd_opt(date.year(), date.month(), new_day)
                    .map(|d| NaiveDateTime::new(d, self.datetime.time()))
            }
            EditSection::Year => {
                let date = self.datetime.date();
                let new_year = date.year() - 1;
                let max_day = days_in_month(new_year, date.month());
                let day = date.day().min(max_day);
                NaiveDate::from_ymd_opt(new_year, date.month(), day)
                    .map(|d| NaiveDateTime::new(d, self.datetime.time()))
            }
            // Time sections
            EditSection::Hour => {
                let hour = self.datetime.time().hour();
                let new_hour = if hour == 0 { 23 } else { hour - 1 };
                NaiveTime::from_hms_opt(
                    new_hour,
                    self.datetime.time().minute(),
                    self.datetime.time().second(),
                )
                .map(|t| NaiveDateTime::new(self.datetime.date(), t))
            }
            EditSection::Minute => {
                let minute = self.datetime.time().minute();
                let new_minute = if minute == 0 { 59 } else { minute - 1 };
                NaiveTime::from_hms_opt(
                    self.datetime.time().hour(),
                    new_minute,
                    self.datetime.time().second(),
                )
                .map(|t| NaiveDateTime::new(self.datetime.date(), t))
            }
            EditSection::Second => {
                let second = self.datetime.time().second();
                let new_second = if second == 0 { 59 } else { second - 1 };
                NaiveTime::from_hms_opt(
                    self.datetime.time().hour(),
                    self.datetime.time().minute(),
                    new_second,
                )
                .map(|t| NaiveDateTime::new(self.datetime.date(), t))
            }
            EditSection::AmPm => {
                let new_hour = (self.datetime.time().hour() + 12) % 24;
                NaiveTime::from_hms_opt(
                    new_hour,
                    self.datetime.time().minute(),
                    self.datetime.time().second(),
                )
                .map(|t| NaiveDateTime::new(self.datetime.date(), t))
            }
            EditSection::Timezone => {
                // Open the timezone picker instead of stepping
                self.show_timezone_popup();
                return;
            }
        };

        if let Some(dt) = new_datetime {
            self.set_datetime(dt);
        }
    }

    // =========================================================================
    // Formatting
    // =========================================================================

    fn format_datetime(&self) -> String {
        let date_str = match self.date_format {
            DateFormat::Short => format!(
                "{:02}/{:02}/{:04}",
                self.datetime.date().month(),
                self.datetime.date().day(),
                self.datetime.date().year()
            ),
            DateFormat::Long => {
                let month_names = [
                    "January",
                    "February",
                    "March",
                    "April",
                    "May",
                    "June",
                    "July",
                    "August",
                    "September",
                    "October",
                    "November",
                    "December",
                ];
                format!(
                    "{} {:02}, {:04}",
                    month_names[(self.datetime.date().month() - 1) as usize],
                    self.datetime.date().day(),
                    self.datetime.date().year()
                )
            }
            DateFormat::ISO => format!(
                "{:04}-{:02}-{:02}",
                self.datetime.date().year(),
                self.datetime.date().month(),
                self.datetime.date().day()
            ),
        };

        let show_seconds = self.seconds_shown();
        let time_str = if self.is_12_hour() {
            let hour = self.datetime.time().hour();
            let display_hour = if hour == 0 {
                12
            } else if hour > 12 {
                hour - 12
            } else {
                hour
            };
            let am_pm = if hour < 12 { "AM" } else { "PM" };

            if show_seconds {
                format!(
                    "{:02}:{:02}:{:02} {}",
                    display_hour,
                    self.datetime.time().minute(),
                    self.datetime.time().second(),
                    am_pm
                )
            } else {
                format!(
                    "{:02}:{:02} {}",
                    display_hour,
                    self.datetime.time().minute(),
                    am_pm
                )
            }
        } else if show_seconds {
            format!(
                "{:02}:{:02}:{:02}",
                self.datetime.time().hour(),
                self.datetime.time().minute(),
                self.datetime.time().second()
            )
        } else {
            format!(
                "{:02}:{:02}",
                self.datetime.time().hour(),
                self.datetime.time().minute()
            )
        };

        // Format timezone if display is not hidden
        let tz_str = self.format_timezone_display();

        if tz_str.is_empty() {
            format!("{}{}{}", date_str, self.separator, time_str)
        } else {
            format!("{}{}{} {}", date_str, self.separator, time_str, tz_str)
        }
    }

    /// Format the timezone for display based on the current display mode.
    fn format_timezone_display(&self) -> String {
        let tz = match self.timezone {
            Some(tz) => tz,
            None => return String::new(),
        };

        match self.timezone_display {
            TimezoneDisplay::Hidden => String::new(),
            TimezoneDisplay::Abbreviation => get_timezone_abbreviation(tz),
            TimezoneDisplay::UtcOffset => {
                let offset_secs = super::timezone::get_utc_offset_seconds(tz);
                let sign = if offset_secs >= 0 { '+' } else { '-' };
                let abs_secs = offset_secs.abs();
                let hours = abs_secs / 3600;
                let mins = (abs_secs % 3600) / 60;
                format!("{}{:02}:{:02}", sign, hours, mins)
            }
            TimezoneDisplay::Full => {
                let abbrev = get_timezone_abbreviation(tz);
                let offset_secs = super::timezone::get_utc_offset_seconds(tz);
                let sign = if offset_secs >= 0 { '+' } else { '-' };
                let abs_secs = offset_secs.abs();
                let hours = abs_secs / 3600;
                let mins = (abs_secs % 3600) / 60;
                format!("{} {}{:02}:{:02}", abbrev, sign, hours, mins)
            }
            TimezoneDisplay::IanaName => tz.name().to_string(),
        }
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    fn right_buttons_width(&self) -> f32 {
        let mut width = self.button_width;
        if self.calendar_popup {
            width += self.calendar_button_width;
        }
        if self.show_timezone_picker {
            width += self.timezone_button_width;
        }
        width
    }

    fn text_field_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            0.0,
            0.0,
            (rect.width() - self.right_buttons_width()).max(0.0),
            rect.height(),
        )
    }

    fn calendar_button_rect(&self) -> Option<Rect> {
        if !self.calendar_popup {
            return None;
        }
        let rect = self.base.rect();
        // Calendar button is positioned before timezone button (if present)
        let x = rect.width() - self.right_buttons_width();
        Some(Rect::new(x, 0.0, self.calendar_button_width, rect.height()))
    }

    fn timezone_button_rect(&self) -> Option<Rect> {
        if !self.show_timezone_picker {
            return None;
        }
        let rect = self.base.rect();
        // Timezone button is between calendar button and up/down buttons
        let x = rect.width() - self.button_width - self.timezone_button_width;
        Some(Rect::new(x, 0.0, self.timezone_button_width, rect.height()))
    }

    fn up_button_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            rect.width() - self.button_width,
            0.0,
            self.button_width,
            rect.height() / 2.0,
        )
    }

    fn down_button_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            rect.width() - self.button_width,
            rect.height() / 2.0,
            self.button_width,
            rect.height() / 2.0,
        )
    }

    fn popup_rect(&self) -> Rect {
        let rect = self.base.rect();
        let cal_hint = self.calendar.size_hint();
        Rect::new(
            0.0,
            rect.height(),
            cal_hint.preferred.width,
            cal_hint.preferred.height,
        )
    }

    fn hit_test(&self, pos: Point) -> DateTimeEditPart {
        if self.popup_visible {
            let popup_rect = self.popup_rect();
            if popup_rect.contains(pos) {
                return DateTimeEditPart::PopupCalendar;
            }
        }

        if self.up_button_rect().contains(pos) {
            return DateTimeEditPart::UpButton;
        }
        if self.down_button_rect().contains(pos) {
            return DateTimeEditPart::DownButton;
        }
        if let Some(cal_rect) = self.calendar_button_rect()
            && cal_rect.contains(pos)
        {
            return DateTimeEditPart::CalendarButton;
        }
        if let Some(tz_rect) = self.timezone_button_rect()
            && tz_rect.contains(pos)
        {
            return DateTimeEditPart::TimezoneButton;
        }

        let text_rect = self.text_field_rect();
        if text_rect.contains(pos) {
            // Cycle through sections on click
            let sections = self.get_section_order();
            let current_idx = sections.iter().position(|&s| s == self.current_section);
            let next_section = match current_idx {
                None => sections.first().copied().unwrap_or(EditSection::None),
                Some(idx) => sections[(idx + 1) % sections.len()],
            };

            return match next_section {
                EditSection::Month => DateTimeEditPart::MonthSection,
                EditSection::Day => DateTimeEditPart::DaySection,
                EditSection::Year => DateTimeEditPart::YearSection,
                EditSection::Hour => DateTimeEditPart::HourSection,
                EditSection::Minute => DateTimeEditPart::MinuteSection,
                EditSection::Second => DateTimeEditPart::SecondSection,
                EditSection::AmPm => DateTimeEditPart::AmPmSection,
                EditSection::Timezone => DateTimeEditPart::TimezoneSection,
                EditSection::None => DateTimeEditPart::None,
            };
        }

        DateTimeEditPart::None
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let part = self.hit_test(event.local_pos);
        self.pressed_part = part;

        match part {
            DateTimeEditPart::UpButton => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Day;
                    }
                    self.step_up_section();
                    self.base.update();
                }
                true
            }
            DateTimeEditPart::DownButton => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Day;
                    }
                    self.step_down_section();
                    self.base.update();
                }
                true
            }
            DateTimeEditPart::CalendarButton => {
                self.toggle_popup();
                true
            }
            DateTimeEditPart::MonthSection => {
                self.current_section = EditSection::Month;
                self.base.update();
                true
            }
            DateTimeEditPart::DaySection => {
                self.current_section = EditSection::Day;
                self.base.update();
                true
            }
            DateTimeEditPart::YearSection => {
                self.current_section = EditSection::Year;
                self.base.update();
                true
            }
            DateTimeEditPart::HourSection => {
                self.current_section = EditSection::Hour;
                self.base.update();
                true
            }
            DateTimeEditPart::MinuteSection => {
                self.current_section = EditSection::Minute;
                self.base.update();
                true
            }
            DateTimeEditPart::SecondSection => {
                self.current_section = EditSection::Second;
                self.base.update();
                true
            }
            DateTimeEditPart::AmPmSection => {
                self.current_section = EditSection::AmPm;
                self.base.update();
                true
            }
            DateTimeEditPart::PopupCalendar => {
                // For simplicity, clicking in the popup area selects the date and closes popup
                // A full implementation would do proper hit-testing within the calendar
                if let Some(selected) = self.calendar.selected_date() {
                    let new_dt = NaiveDateTime::new(selected, self.datetime.time());
                    if new_dt != self.datetime {
                        self.set_datetime(new_dt);
                    }
                }
                self.hide_popup();
                self.base.update();
                true
            }
            DateTimeEditPart::TimezoneSection => {
                self.current_section = EditSection::Timezone;
                self.base.update();
                true
            }
            DateTimeEditPart::TimezoneButton => {
                self.toggle_timezone_popup();
                true
            }
            DateTimeEditPart::PopupTimezone => {
                // Handled by the timezone combo box
                true
            }
            DateTimeEditPart::None => {
                if self.popup_visible {
                    self.hide_popup();
                    true
                } else if self.timezone_popup_visible {
                    self.hide_timezone_popup();
                    true
                } else {
                    false
                }
            }
        }
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.pressed_part != DateTimeEditPart::None {
            self.pressed_part = DateTimeEditPart::None;
            self.base.update();
            return true;
        }
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let new_hover = self.hit_test(event.local_pos);
        if self.hover_part != new_hover {
            self.hover_part = new_hover;
            self.base.update();
        }
        false
    }

    fn handle_wheel(&mut self, event: &WheelEvent) -> bool {
        if self.read_only {
            return false;
        }

        if self.current_section == EditSection::None {
            self.current_section = EditSection::Day;
        }

        if event.delta_y.abs() > 0.0 {
            if event.delta_y > 0.0 {
                self.step_up_section();
            } else {
                self.step_down_section();
            }
            return true;
        }
        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Escape => {
                if self.popup_visible {
                    self.hide_popup();
                    return true;
                }
            }
            Key::Enter => {
                if self.popup_visible {
                    if let Some(selected) = self.calendar.selected_date() {
                        let new_dt = NaiveDateTime::new(selected, self.datetime.time());
                        self.set_datetime(new_dt);
                    }
                    self.hide_popup();
                } else {
                    self.editing_finished.emit(());
                }
                return true;
            }
            Key::Space => {
                if event.modifiers.alt || event.modifiers.control {
                    self.toggle_popup();
                    return true;
                }
            }
            Key::ArrowUp => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Day;
                    }
                    self.step_up_section();
                    return true;
                }
            }
            Key::ArrowDown => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Day;
                    }
                    self.step_down_section();
                    return true;
                }
            }
            Key::ArrowLeft => {
                self.previous_section();
                return true;
            }
            Key::ArrowRight => {
                self.next_section();
                return true;
            }
            Key::Tab => {
                if event.modifiers.shift {
                    self.previous_section();
                } else {
                    self.next_section();
                }
                return true;
            }
            Key::Home => {
                self.set_datetime(self.minimum_datetime);
                return true;
            }
            Key::End => {
                self.set_datetime(self.maximum_datetime);
                return true;
            }
            _ => {
                // Handle A/P for AM/PM toggle
                if self.is_12_hour()
                    && self.current_section == EditSection::AmPm
                    && let Some(ch) = event.text.chars().next()
                {
                    let ch_lower = ch.to_ascii_lowercase();
                    let hour = self.datetime.time().hour();
                    if ch_lower == 'a' && hour >= 12 {
                        if let Some(t) = NaiveTime::from_hms_opt(
                            hour - 12,
                            self.datetime.time().minute(),
                            self.datetime.time().second(),
                        ) {
                            self.set_time(t);
                        }
                        return true;
                    } else if ch_lower == 'p' && hour < 12 {
                        if let Some(t) = NaiveTime::from_hms_opt(
                            hour + 12,
                            self.datetime.time().minute(),
                            self.datetime.time().second(),
                        ) {
                            self.set_time(t);
                        }
                        return true;
                    }
                }
            }
        }
        false
    }

    fn handle_focus_out(&mut self) -> bool {
        self.current_section = EditSection::None;
        self.hide_popup();
        self.editing_finished.emit(());
        self.base.update();
        false
    }

    fn handle_leave(&mut self) -> bool {
        if self.hover_part != DateTimeEditPart::None {
            self.hover_part = DateTimeEditPart::None;
            self.base.update();
        }
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        let bg_rrect = RoundedRect::new(rect, self.border_radius);
        ctx.renderer()
            .fill_rounded_rect(bg_rrect, self.background_color);

        let border_stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(bg_rrect, &border_stroke);
    }

    fn paint_text_field(&self, ctx: &mut PaintContext<'_>) {
        let text_rect = self.text_field_rect();
        let display = self.format_datetime();

        let mut font_system = FontSystem::new();
        let layout = TextLayout::with_options(
            &mut font_system,
            &display,
            &self.font,
            TextLayoutOptions::new()
                .horizontal_align(HorizontalAlign::Left)
                .vertical_align(VerticalAlign::Middle),
        );

        let padding = 6.0;
        let text_x = text_rect.origin.x + padding;
        let text_y = text_rect.origin.y + (text_rect.height() - layout.height()) / 2.0;

        if self.current_section != EditSection::None && self.widget_base().has_focus() {
            let highlight_rect = Rect::new(text_x, text_y, layout.width(), layout.height());
            ctx.renderer()
                .fill_rect(highlight_rect, self.section_highlight_color);
        }

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                self.text_color,
            );
        }
    }

    fn paint_buttons(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Calendar button
        if let Some(cal_rect) = self.calendar_button_rect() {
            let is_hovered = matches!(self.hover_part, DateTimeEditPart::CalendarButton);
            let is_pressed = matches!(self.pressed_part, DateTimeEditPart::CalendarButton);

            let bg_color = if is_pressed {
                self.button_pressed_color
            } else if is_hovered {
                self.button_hover_color
            } else {
                self.button_color
            };

            ctx.renderer().fill_rect(cal_rect, bg_color);

            // Calendar icon
            let icon_size = 12.0;
            let icon_x = cal_rect.origin.x + (cal_rect.width() - icon_size) / 2.0;
            let icon_y = cal_rect.origin.y + (cal_rect.height() - icon_size) / 2.0;
            let icon_rect = Rect::new(icon_x, icon_y, icon_size, icon_size);

            let stroke = Stroke::new(Color::from_rgb8(80, 80, 80), 1.0);
            ctx.renderer().stroke_rect(icon_rect, &stroke);

            let cell = icon_size / 3.0;
            for i in 1..3 {
                ctx.renderer().draw_line(
                    Point::new(icon_x + i as f32 * cell, icon_y),
                    Point::new(icon_x + i as f32 * cell, icon_y + icon_size),
                    &stroke,
                );
                ctx.renderer().draw_line(
                    Point::new(icon_x, icon_y + i as f32 * cell),
                    Point::new(icon_x + icon_size, icon_y + i as f32 * cell),
                    &stroke,
                );
            }
        }

        // Button separator
        let sep_x = rect.width() - self.button_width;
        let sep_stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(sep_x, 0.0),
            Point::new(sep_x, rect.height()),
            &sep_stroke,
        );

        let mid_y = rect.height() / 2.0;
        ctx.renderer().draw_line(
            Point::new(sep_x, mid_y),
            Point::new(rect.width(), mid_y),
            &sep_stroke,
        );

        // Up button
        let up_rect = self.up_button_rect();
        let up_color = if self.pressed_part == DateTimeEditPart::UpButton {
            self.button_pressed_color
        } else if self.hover_part == DateTimeEditPart::UpButton {
            self.button_hover_color
        } else {
            self.button_color
        };
        ctx.renderer().fill_rect(up_rect, up_color);
        self.paint_arrow(ctx, up_rect, true);

        // Down button
        let down_rect = self.down_button_rect();
        let down_color = if self.pressed_part == DateTimeEditPart::DownButton {
            self.button_pressed_color
        } else if self.hover_part == DateTimeEditPart::DownButton {
            self.button_hover_color
        } else {
            self.button_color
        };
        ctx.renderer().fill_rect(down_rect, down_color);
        self.paint_arrow(ctx, down_rect, false);
    }

    fn paint_arrow(&self, ctx: &mut PaintContext<'_>, rect: Rect, up: bool) {
        let center_x = rect.origin.x + rect.width() / 2.0;
        let center_y = rect.origin.y + rect.height() / 2.0;
        let arrow_size = 4.0;
        let arrow_color = Color::from_rgb8(80, 80, 80);
        let stroke = Stroke::new(arrow_color, 1.5);

        if up {
            let p1 = Point::new(center_x - arrow_size, center_y + arrow_size / 2.0);
            let p2 = Point::new(center_x, center_y - arrow_size / 2.0);
            let p3 = Point::new(center_x + arrow_size, center_y + arrow_size / 2.0);
            ctx.renderer().draw_line(p1, p2, &stroke);
            ctx.renderer().draw_line(p2, p3, &stroke);
        } else {
            let p1 = Point::new(center_x - arrow_size, center_y - arrow_size / 2.0);
            let p2 = Point::new(center_x, center_y + arrow_size / 2.0);
            let p3 = Point::new(center_x + arrow_size, center_y - arrow_size / 2.0);
            ctx.renderer().draw_line(p1, p2, &stroke);
            ctx.renderer().draw_line(p2, p3, &stroke);
        }
    }

    fn paint_popup(&self, ctx: &mut PaintContext<'_>) {
        if !self.popup_visible {
            return;
        }

        let popup_rect = self.popup_rect();
        ctx.renderer().fill_rect(popup_rect, Color::WHITE);

        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rect(popup_rect, &stroke);
    }

    fn paint_focus_indicator(&self, ctx: &mut PaintContext<'_>) {
        if !self.widget_base().has_focus() {
            return;
        }

        let rect = ctx.rect();
        let focus_color = Color::from_rgba8(66, 133, 244, 180);
        let focus_stroke = Stroke::new(focus_color, 2.0);
        let focus_rrect = RoundedRect::new(rect, self.border_radius);
        ctx.renderer()
            .stroke_rounded_rect(focus_rrect, &focus_stroke);
    }
}

/// Get the number of days in a month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

impl Default for DateTimeEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for DateTimeEdit {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for DateTimeEdit {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let date_width = 85.0;
        let time_width = if self.is_12_hour() {
            if self.seconds_shown() { 100.0 } else { 80.0 }
        } else if self.seconds_shown() {
            75.0
        } else {
            55.0
        };
        let width = date_width + time_width + self.right_buttons_width() + 20.0;
        let height = 28.0;

        SizeHint::from_dimensions(width.max(180.0), height).with_minimum_dimensions(150.0, 22.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);
        self.paint_text_field(ctx);
        self.paint_buttons(ctx);
        self.paint_popup(ctx);
        self.paint_focus_indicator(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                if self.handle_mouse_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if self.handle_mouse_release(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseMove(e) => {
                if self.handle_mouse_move(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::Wheel(e) => {
                if self.handle_wheel(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::KeyPress(e) => {
                if self.handle_key_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::FocusOut(_) => {
                self.handle_focus_out();
            }
            WidgetEvent::Leave(_) => {
                self.handle_leave();
            }
            _ => {}
        }
        false
    }
}

// Ensure DateTimeEdit is Send + Sync
static_assertions::assert_impl_all!(DateTimeEdit: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_datetime_edit_creation() {
        setup();
        let dt_edit = DateTimeEdit::new();
        assert!(dt_edit.calendar_popup_enabled());
        assert!(!dt_edit.is_read_only());
        assert_eq!(dt_edit.date_format(), DateFormat::Short);
        assert_eq!(dt_edit.time_format(), TimeFormat::Hour24);
    }

    #[test]
    fn test_datetime_edit_with_datetime() {
        setup();
        let dt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(),
            NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
        );
        let dt_edit = DateTimeEdit::new().with_datetime(dt);
        assert_eq!(dt_edit.datetime(), dt);
        assert_eq!(dt_edit.date(), dt.date());
        assert_eq!(dt_edit.time(), dt.time());
    }

    #[test]
    fn test_datetime_edit_set_date_time_separately() {
        setup();
        let mut dt_edit = DateTimeEdit::new();

        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        dt_edit.set_date(date);
        assert_eq!(dt_edit.date(), date);

        let time = NaiveTime::from_hms_opt(14, 30, 45).unwrap();
        dt_edit.set_time(time);
        assert_eq!(dt_edit.time(), time);
    }

    #[test]
    fn test_datetime_edit_clamping() {
        setup();
        let min = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let max = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
        );

        let mut dt_edit = DateTimeEdit::new().with_datetime_range(min, max);

        let before = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        dt_edit.set_datetime(before);
        assert_eq!(dt_edit.datetime(), min);

        let after = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        dt_edit.set_datetime(after);
        assert_eq!(dt_edit.datetime(), max);
    }

    #[test]
    fn test_datetime_edit_format() {
        setup();
        let dt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
            NaiveTime::from_hms_opt(14, 30, 45).unwrap(),
        );

        let edit = DateTimeEdit::new()
            .with_datetime(dt)
            .with_date_format(DateFormat::Short)
            .with_time_format(TimeFormat::Hour24);
        assert_eq!(edit.format_datetime(), "01/15/2025 14:30");

        let edit_12h = DateTimeEdit::new()
            .with_datetime(dt)
            .with_date_format(DateFormat::ISO)
            .with_time_format(TimeFormat::Hour12);
        assert_eq!(edit_12h.format_datetime(), "2025-01-15 02:30 PM");
    }

    #[test]
    fn test_datetime_edit_signals() {
        setup();
        let mut dt_edit = DateTimeEdit::new();

        let datetime_fired = Arc::new(AtomicBool::new(false));
        let date_fired = Arc::new(AtomicBool::new(false));
        let time_fired = Arc::new(AtomicBool::new(false));

        let dt_clone = datetime_fired.clone();
        dt_edit.datetime_changed.connect(move |_| {
            dt_clone.store(true, Ordering::SeqCst);
        });

        let d_clone = date_fired.clone();
        dt_edit.date_changed.connect(move |_| {
            d_clone.store(true, Ordering::SeqCst);
        });

        let t_clone = time_fired.clone();
        dt_edit.time_changed.connect(move |_| {
            t_clone.store(true, Ordering::SeqCst);
        });

        let new_dt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(),
            NaiveTime::from_hms_opt(14, 30, 0).unwrap(),
        );
        dt_edit.set_datetime(new_dt);

        assert!(datetime_fired.load(Ordering::SeqCst));
        assert!(date_fired.load(Ordering::SeqCst));
        assert!(time_fired.load(Ordering::SeqCst));
    }

    #[test]
    fn test_datetime_edit_builder_pattern() {
        setup();
        let min = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
        );
        let max = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
            NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
        );
        let dt = NaiveDateTime::new(
            NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(),
            NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
        );

        let dt_edit = DateTimeEdit::new()
            .with_datetime(dt)
            .with_datetime_range(min, max)
            .with_date_format(DateFormat::ISO)
            .with_time_format(TimeFormat::Hour12)
            .with_show_seconds(true)
            .with_calendar_popup(false)
            .with_separator(" - ")
            .with_read_only(true);

        assert_eq!(dt_edit.datetime(), dt);
        assert_eq!(dt_edit.date_format(), DateFormat::ISO);
        assert_eq!(dt_edit.time_format(), TimeFormat::Hour12);
        assert!(dt_edit.seconds_shown());
        assert!(!dt_edit.calendar_popup_enabled());
        assert_eq!(dt_edit.separator(), " - ");
        assert!(dt_edit.is_read_only());
    }

    #[test]
    fn test_edit_section_type_detection() {
        assert!(EditSection::Month.is_date_section());
        assert!(EditSection::Day.is_date_section());
        assert!(EditSection::Year.is_date_section());
        assert!(!EditSection::Hour.is_date_section());

        assert!(EditSection::Hour.is_time_section());
        assert!(EditSection::Minute.is_time_section());
        assert!(EditSection::Second.is_time_section());
        assert!(EditSection::AmPm.is_time_section());
        assert!(!EditSection::Month.is_time_section());
    }

    #[test]
    fn test_datetime_edit_size_hint() {
        setup();
        let dt_edit = DateTimeEdit::new();
        let hint = dt_edit.size_hint();
        assert!(hint.preferred.width >= 150.0);
        assert!(hint.preferred.height >= 22.0);
    }
}
