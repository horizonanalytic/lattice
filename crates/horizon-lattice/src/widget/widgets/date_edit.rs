//! DateEdit widget for date input.
//!
//! The DateEdit widget provides a way to enter and modify dates with:
//! - Section-based editing (month, day, year)
//! - Increment/decrement buttons
//! - Calendar popup for date selection
//! - Date format customization
//! - Range constraints (minimum, maximum)
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{DateEdit, DateFormat};
//! use chrono::NaiveDate;
//!
//! // Create a date editor with calendar popup
//! let mut date_edit = DateEdit::new()
//!     .with_date(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap())
//!     .with_display_format(DateFormat::Short)
//!     .with_calendar_popup(true);
//!
//! // Connect to date changes
//! date_edit.date_changed.connect(|date| {
//!     println!("Date changed: {}", date);
//! });
//! ```

use chrono::{Datelike, Local, NaiveDate};
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

/// Display format for dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateFormat {
    /// Short format: MM/DD/YYYY
    #[default]
    Short,
    /// Long format: Month DD, YYYY
    Long,
    /// ISO format: YYYY-MM-DD
    ISO,
}

/// Parts of the DateEdit for hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DateEditPart {
    #[default]
    None,
    /// Month section.
    MonthSection,
    /// Day section.
    DaySection,
    /// Year section.
    YearSection,
    /// Calendar popup button.
    CalendarButton,
    /// Up (increment) button.
    UpButton,
    /// Down (decrement) button.
    DownButton,
    /// Popup calendar area.
    PopupCalendar,
}

/// Which section is currently focused for editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum EditSection {
    #[default]
    None,
    Month,
    Day,
    Year,
}

/// A widget for entering and modifying dates.
///
/// DateEdit provides a text field showing the current date with editable
/// sections for month, day, and year. An optional calendar popup allows
/// visual date selection.
///
/// # Signals
///
/// - `date_changed(NaiveDate)`: Emitted when the date changes
/// - `editing_finished()`: Emitted when editing is completed
pub struct DateEdit {
    /// Widget base.
    base: WidgetBase,

    /// Current date value.
    date: NaiveDate,

    /// Minimum selectable date.
    minimum_date: NaiveDate,

    /// Maximum selectable date.
    maximum_date: NaiveDate,

    /// Display format.
    display_format: DateFormat,

    /// Whether to show the calendar popup button.
    calendar_popup: bool,

    /// Current editing section.
    current_section: EditSection,

    /// Whether read-only.
    read_only: bool,

    // Appearance
    /// Background color.
    background_color: Color,
    /// Text color.
    text_color: Color,
    /// Border color.
    border_color: Color,
    /// Button color.
    button_color: Color,
    /// Button hover color.
    button_hover_color: Color,
    /// Button pressed color.
    button_pressed_color: Color,
    /// Selected section background color.
    section_highlight_color: Color,
    /// Font.
    font: Font,
    /// Border radius.
    border_radius: f32,
    /// Button width.
    button_width: f32,
    /// Calendar button width.
    calendar_button_width: f32,

    /// Which part is currently hovered.
    hover_part: DateEditPart,
    /// Which part is currently pressed.
    pressed_part: DateEditPart,
    /// Whether the popup is visible.
    popup_visible: bool,

    /// Internal calendar widget for popup.
    calendar: CalendarWidget,

    /// Signal emitted when date changes.
    pub date_changed: Signal<NaiveDate>,
    /// Signal emitted when editing is finished.
    pub editing_finished: Signal<()>,
}

impl DateEdit {
    /// Create a new DateEdit with today's date.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Fixed,
        ));

        let today = Local::now().date_naive();

        let mut calendar = CalendarWidget::new();
        calendar.set_selected_date(Some(today));

        Self {
            base,
            date: today,
            minimum_date: NaiveDate::from_ymd_opt(1752, 9, 14).unwrap(), // Gregorian calendar start
            maximum_date: NaiveDate::from_ymd_opt(9999, 12, 31).unwrap(),
            display_format: DateFormat::Short,
            calendar_popup: true,
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
            hover_part: DateEditPart::None,
            pressed_part: DateEditPart::None,
            popup_visible: false,
            calendar,
            date_changed: Signal::new(),
            editing_finished: Signal::new(),
        }
    }

    // =========================================================================
    // Date Access
    // =========================================================================

    /// Get the current date.
    pub fn date(&self) -> NaiveDate {
        self.date
    }

    /// Set the current date.
    pub fn set_date(&mut self, date: NaiveDate) {
        let clamped = date.clamp(self.minimum_date, self.maximum_date);
        if self.date != clamped {
            self.date = clamped;
            self.calendar.set_selected_date(Some(clamped));
            self.calendar.show_date(clamped);
            self.base.update();
            self.date_changed.emit(clamped);
        }
    }

    /// Set date using builder pattern.
    pub fn with_date(mut self, date: NaiveDate) -> Self {
        self.date = date.clamp(self.minimum_date, self.maximum_date);
        self.calendar.set_selected_date(Some(self.date));
        self.calendar.show_date(self.date);
        self
    }

    // =========================================================================
    // Date Constraints
    // =========================================================================

    /// Get the minimum date.
    pub fn minimum_date(&self) -> NaiveDate {
        self.minimum_date
    }

    /// Set the minimum date.
    pub fn set_minimum_date(&mut self, date: NaiveDate) {
        self.minimum_date = date;
        self.calendar.set_minimum_date(Some(date));
        if self.date < date {
            self.set_date(date);
        }
    }

    /// Set minimum date using builder pattern.
    pub fn with_minimum_date(mut self, date: NaiveDate) -> Self {
        self.minimum_date = date;
        self.calendar.set_minimum_date(Some(date));
        if self.date < date {
            self.date = date;
        }
        self
    }

    /// Get the maximum date.
    pub fn maximum_date(&self) -> NaiveDate {
        self.maximum_date
    }

    /// Set the maximum date.
    pub fn set_maximum_date(&mut self, date: NaiveDate) {
        self.maximum_date = date;
        self.calendar.set_maximum_date(Some(date));
        if self.date > date {
            self.set_date(date);
        }
    }

    /// Set maximum date using builder pattern.
    pub fn with_maximum_date(mut self, date: NaiveDate) -> Self {
        self.maximum_date = date;
        self.calendar.set_maximum_date(Some(date));
        if self.date > date {
            self.date = date;
        }
        self
    }

    /// Set the date range.
    pub fn set_date_range(&mut self, min: NaiveDate, max: NaiveDate) {
        self.minimum_date = min;
        self.maximum_date = max;
        self.calendar.set_date_range(min, max);
        let clamped = self.date.clamp(min, max);
        if self.date != clamped {
            self.set_date(clamped);
        }
    }

    /// Set date range using builder pattern.
    pub fn with_date_range(mut self, min: NaiveDate, max: NaiveDate) -> Self {
        self.minimum_date = min;
        self.maximum_date = max;
        self.calendar.set_date_range(min, max);
        self.date = self.date.clamp(min, max);
        self
    }

    // =========================================================================
    // Display Options
    // =========================================================================

    /// Get the display format.
    pub fn display_format(&self) -> DateFormat {
        self.display_format
    }

    /// Set the display format.
    pub fn set_display_format(&mut self, format: DateFormat) {
        if self.display_format != format {
            self.display_format = format;
            self.base.update();
        }
    }

    /// Set display format using builder pattern.
    pub fn with_display_format(mut self, format: DateFormat) -> Self {
        self.display_format = format;
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
        self.calendar.show_date(self.date);
        self.calendar.set_selected_date(Some(self.date));
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
    // Section Navigation
    // =========================================================================

    fn next_section(&mut self) {
        self.current_section = match self.current_section {
            EditSection::None => match self.display_format {
                DateFormat::Short | DateFormat::Long => EditSection::Month,
                DateFormat::ISO => EditSection::Year,
            },
            EditSection::Month => EditSection::Day,
            EditSection::Day => match self.display_format {
                DateFormat::Short | DateFormat::Long => EditSection::Year,
                DateFormat::ISO => EditSection::None,
            },
            EditSection::Year => match self.display_format {
                DateFormat::Short | DateFormat::Long => EditSection::None,
                DateFormat::ISO => EditSection::Month,
            },
        };
        self.base.update();
    }

    fn previous_section(&mut self) {
        self.current_section = match self.current_section {
            EditSection::None => match self.display_format {
                DateFormat::Short | DateFormat::Long => EditSection::Year,
                DateFormat::ISO => EditSection::Day,
            },
            EditSection::Month => match self.display_format {
                DateFormat::Short | DateFormat::Long => EditSection::None,
                DateFormat::ISO => EditSection::Year,
            },
            EditSection::Day => match self.display_format {
                DateFormat::Short | DateFormat::Long => EditSection::Month,
                DateFormat::ISO => EditSection::Month,
            },
            EditSection::Year => match self.display_format {
                DateFormat::Short | DateFormat::Long => EditSection::Day,
                DateFormat::ISO => EditSection::None,
            },
        };
        self.base.update();
    }

    fn step_up_section(&mut self) {
        if self.read_only {
            return;
        }

        let new_date = match self.current_section {
            EditSection::None => return,
            EditSection::Month => {
                let new_month = if self.date.month() == 12 {
                    1
                } else {
                    self.date.month() + 1
                };
                let new_year = if self.date.month() == 12 {
                    self.date.year() + 1
                } else {
                    self.date.year()
                };
                // Clamp day to valid range for new month
                let max_day = days_in_month(new_year, new_month);
                let day = self.date.day().min(max_day);
                NaiveDate::from_ymd_opt(new_year, new_month, day)
            }
            EditSection::Day => {
                let max_day = days_in_month(self.date.year(), self.date.month());
                let new_day = if self.date.day() >= max_day {
                    1
                } else {
                    self.date.day() + 1
                };
                NaiveDate::from_ymd_opt(self.date.year(), self.date.month(), new_day)
            }
            EditSection::Year => {
                let new_year = self.date.year() + 1;
                // Clamp day for leap year handling (Feb 29)
                let max_day = days_in_month(new_year, self.date.month());
                let day = self.date.day().min(max_day);
                NaiveDate::from_ymd_opt(new_year, self.date.month(), day)
            }
        };

        if let Some(d) = new_date {
            self.set_date(d);
        }
    }

    fn step_down_section(&mut self) {
        if self.read_only {
            return;
        }

        let new_date = match self.current_section {
            EditSection::None => return,
            EditSection::Month => {
                let new_month = if self.date.month() == 1 {
                    12
                } else {
                    self.date.month() - 1
                };
                let new_year = if self.date.month() == 1 {
                    self.date.year() - 1
                } else {
                    self.date.year()
                };
                let max_day = days_in_month(new_year, new_month);
                let day = self.date.day().min(max_day);
                NaiveDate::from_ymd_opt(new_year, new_month, day)
            }
            EditSection::Day => {
                let max_day = days_in_month(self.date.year(), self.date.month());
                let new_day = if self.date.day() == 1 {
                    max_day
                } else {
                    self.date.day() - 1
                };
                NaiveDate::from_ymd_opt(self.date.year(), self.date.month(), new_day)
            }
            EditSection::Year => {
                let new_year = self.date.year() - 1;
                let max_day = days_in_month(new_year, self.date.month());
                let day = self.date.day().min(max_day);
                NaiveDate::from_ymd_opt(new_year, self.date.month(), day)
            }
        };

        if let Some(d) = new_date {
            self.set_date(d);
        }
    }

    // =========================================================================
    // Formatting
    // =========================================================================

    fn format_date(&self) -> String {
        match self.display_format {
            DateFormat::Short => format!(
                "{:02}/{:02}/{:04}",
                self.date.month(),
                self.date.day(),
                self.date.year()
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
                    month_names[(self.date.month() - 1) as usize],
                    self.date.day(),
                    self.date.year()
                )
            }
            DateFormat::ISO => format!(
                "{:04}-{:02}-{:02}",
                self.date.year(),
                self.date.month(),
                self.date.day()
            ),
        }
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    fn right_buttons_width(&self) -> f32 {
        let mut width = self.button_width; // Up/down buttons
        if self.calendar_popup {
            width += self.calendar_button_width;
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
        Some(Rect::new(
            rect.width() - self.right_buttons_width(),
            0.0,
            self.calendar_button_width,
            rect.height(),
        ))
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

    fn section_from_position(&self, _pos: Point) -> EditSection {
        // For simplicity, cycle through sections on click in the text area
        // A full implementation would calculate exact section bounds
        match self.current_section {
            EditSection::None => match self.display_format {
                DateFormat::Short | DateFormat::Long => EditSection::Month,
                DateFormat::ISO => EditSection::Year,
            },
            EditSection::Month => EditSection::Day,
            EditSection::Day => EditSection::Year,
            EditSection::Year => EditSection::Month,
        }
    }

    fn hit_test(&self, pos: Point) -> DateEditPart {
        // Check popup first if visible
        if self.popup_visible {
            let popup_rect = self.popup_rect();
            if popup_rect.contains(pos) {
                return DateEditPart::PopupCalendar;
            }
        }

        if self.up_button_rect().contains(pos) {
            return DateEditPart::UpButton;
        }
        if self.down_button_rect().contains(pos) {
            return DateEditPart::DownButton;
        }
        if let Some(cal_rect) = self.calendar_button_rect()
            && cal_rect.contains(pos) {
                return DateEditPart::CalendarButton;
            }

        let text_rect = self.text_field_rect();
        if text_rect.contains(pos) {
            // Determine which section was clicked
            match self.section_from_position(pos) {
                EditSection::Month => return DateEditPart::MonthSection,
                EditSection::Day => return DateEditPart::DaySection,
                EditSection::Year => return DateEditPart::YearSection,
                EditSection::None => return DateEditPart::None,
            }
        }

        DateEditPart::None
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
            DateEditPart::UpButton => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Day;
                    }
                    self.step_up_section();
                    self.base.update();
                }
                true
            }
            DateEditPart::DownButton => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Day;
                    }
                    self.step_down_section();
                    self.base.update();
                }
                true
            }
            DateEditPart::CalendarButton => {
                self.toggle_popup();
                true
            }
            DateEditPart::MonthSection | DateEditPart::DaySection | DateEditPart::YearSection => {
                self.current_section = match part {
                    DateEditPart::MonthSection => EditSection::Month,
                    DateEditPart::DaySection => EditSection::Day,
                    DateEditPart::YearSection => EditSection::Year,
                    _ => EditSection::None,
                };
                self.base.update();
                true
            }
            DateEditPart::PopupCalendar => {
                // For simplicity, clicking in the popup area selects the date and closes popup
                // A full implementation would do proper hit-testing within the calendar
                if let Some(selected) = self.calendar.selected_date()
                    && selected != self.date {
                        self.set_date(selected);
                    }
                self.hide_popup();
                self.base.update();
                true
            }
            DateEditPart::None => {
                if self.popup_visible {
                    self.hide_popup();
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

        if self.pressed_part != DateEditPart::None {
            self.pressed_part = DateEditPart::None;
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
                        self.set_date(selected);
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
                self.set_date(self.minimum_date);
                return true;
            }
            Key::End => {
                self.set_date(self.maximum_date);
                return true;
            }
            Key::PageUp => {
                // Go to previous month
                let new_date = if self.date.month() == 1 {
                    NaiveDate::from_ymd_opt(self.date.year() - 1, 12, self.date.day().min(31))
                } else {
                    let new_month = self.date.month() - 1;
                    let max_day = days_in_month(self.date.year(), new_month);
                    NaiveDate::from_ymd_opt(
                        self.date.year(),
                        new_month,
                        self.date.day().min(max_day),
                    )
                };
                if let Some(d) = new_date {
                    self.set_date(d);
                }
                return true;
            }
            Key::PageDown => {
                // Go to next month
                let new_date = if self.date.month() == 12 {
                    NaiveDate::from_ymd_opt(self.date.year() + 1, 1, self.date.day().min(31))
                } else {
                    let new_month = self.date.month() + 1;
                    let max_day = days_in_month(self.date.year(), new_month);
                    NaiveDate::from_ymd_opt(
                        self.date.year(),
                        new_month,
                        self.date.day().min(max_day),
                    )
                };
                if let Some(d) = new_date {
                    self.set_date(d);
                }
                return true;
            }
            _ => {}
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
        if self.hover_part != DateEditPart::None {
            self.hover_part = DateEditPart::None;
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
        let display = self.format_date();

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

        // Highlight current section
        if self.current_section != EditSection::None && self.widget_base().has_focus() {
            // Calculate section bounds (simplified - highlights whole text)
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
            let is_hovered = matches!(self.hover_part, DateEditPart::CalendarButton);
            let is_pressed = matches!(self.pressed_part, DateEditPart::CalendarButton);

            let bg_color = if is_pressed {
                self.button_pressed_color
            } else if is_hovered {
                self.button_hover_color
            } else {
                self.button_color
            };

            ctx.renderer().fill_rect(cal_rect, bg_color);

            // Draw calendar icon (simplified grid)
            let icon_size = 12.0;
            let icon_x = cal_rect.origin.x + (cal_rect.width() - icon_size) / 2.0;
            let icon_y = cal_rect.origin.y + (cal_rect.height() - icon_size) / 2.0;
            let icon_rect = Rect::new(icon_x, icon_y, icon_size, icon_size);

            let stroke = Stroke::new(Color::from_rgb8(80, 80, 80), 1.0);
            ctx.renderer().stroke_rect(icon_rect, &stroke);

            // Grid lines
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

        // Horizontal separator between up/down buttons
        let mid_y = rect.height() / 2.0;
        ctx.renderer().draw_line(
            Point::new(sep_x, mid_y),
            Point::new(rect.width(), mid_y),
            &sep_stroke,
        );

        // Up button
        let up_rect = self.up_button_rect();
        let up_color = if self.pressed_part == DateEditPart::UpButton {
            self.button_pressed_color
        } else if self.hover_part == DateEditPart::UpButton {
            self.button_hover_color
        } else {
            self.button_color
        };
        ctx.renderer().fill_rect(up_rect, up_color);
        self.paint_arrow(ctx, up_rect, true);

        // Down button
        let down_rect = self.down_button_rect();
        let down_color = if self.pressed_part == DateEditPart::DownButton {
            self.button_pressed_color
        } else if self.hover_part == DateEditPart::DownButton {
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

        // Draw popup background
        ctx.renderer().fill_rect(popup_rect, Color::WHITE);

        // Draw popup border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rect(popup_rect, &stroke);

        // Paint the calendar widget
        // Note: In a full implementation, we'd use a proper sub-context
        // For now, we paint relative to popup position
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
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Check if a year is a leap year.
fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

impl Default for DateEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for DateEdit {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for DateEdit {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let text_width = 100.0; // Approximate for date text
        let width = text_width + self.right_buttons_width() + 16.0;
        let height = 28.0;

        SizeHint::from_dimensions(width.max(120.0), height).with_minimum_dimensions(80.0, 22.0)
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

// Ensure DateEdit is Send + Sync
static_assertions::assert_impl_all!(DateEdit: Send, Sync);

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
    fn test_date_edit_creation() {
        setup();
        let date_edit = DateEdit::new();
        let today = Local::now().date_naive();
        assert_eq!(date_edit.date(), today);
        assert!(date_edit.calendar_popup_enabled());
        assert!(!date_edit.is_read_only());
    }

    #[test]
    fn test_date_edit_with_date() {
        setup();
        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let date_edit = DateEdit::new().with_date(date);
        assert_eq!(date_edit.date(), date);
    }

    #[test]
    fn test_date_edit_date_range() {
        setup();
        let min = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let max = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

        let date_edit = DateEdit::new().with_date_range(min, max).with_date(date);

        assert_eq!(date_edit.minimum_date(), min);
        assert_eq!(date_edit.maximum_date(), max);
        assert_eq!(date_edit.date(), date);
    }

    #[test]
    fn test_date_edit_clamping() {
        setup();
        let min = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let max = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let before = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let after = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();

        let mut date_edit = DateEdit::new().with_date_range(min, max);

        date_edit.set_date(before);
        assert_eq!(date_edit.date(), min);

        date_edit.set_date(after);
        assert_eq!(date_edit.date(), max);
    }

    #[test]
    fn test_date_edit_format() {
        setup();
        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();

        let short = DateEdit::new()
            .with_date(date)
            .with_display_format(DateFormat::Short);
        assert_eq!(short.format_date(), "01/15/2025");

        let iso = DateEdit::new()
            .with_date(date)
            .with_display_format(DateFormat::ISO);
        assert_eq!(iso.format_date(), "2025-01-15");

        let long = DateEdit::new()
            .with_date(date)
            .with_display_format(DateFormat::Long);
        assert_eq!(long.format_date(), "January 15, 2025");
    }

    #[test]
    fn test_date_edit_signal() {
        setup();
        let mut date_edit = DateEdit::new();
        let signal_fired = Arc::new(AtomicBool::new(false));
        let signal_fired_clone = signal_fired.clone();

        date_edit.date_changed.connect(move |_| {
            signal_fired_clone.store(true, Ordering::SeqCst);
        });

        let new_date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        date_edit.set_date(new_date);
        assert!(signal_fired.load(Ordering::SeqCst));
    }

    #[test]
    fn test_date_edit_builder_pattern() {
        setup();
        let min = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let max = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

        let date_edit = DateEdit::new()
            .with_date(date)
            .with_date_range(min, max)
            .with_display_format(DateFormat::ISO)
            .with_calendar_popup(false)
            .with_read_only(true);

        assert_eq!(date_edit.date(), date);
        assert_eq!(date_edit.display_format(), DateFormat::ISO);
        assert!(!date_edit.calendar_popup_enabled());
        assert!(date_edit.is_read_only());
    }

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2025, 1), 31);
        assert_eq!(days_in_month(2025, 2), 28);
        assert_eq!(days_in_month(2024, 2), 29); // Leap year
        assert_eq!(days_in_month(2025, 4), 30);
        assert_eq!(days_in_month(2100, 2), 28); // Not a leap year (divisible by 100 but not 400)
        assert_eq!(days_in_month(2000, 2), 29); // Leap year (divisible by 400)
    }

    #[test]
    fn test_is_leap_year() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2025));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(2000));
    }

    #[test]
    fn test_date_edit_size_hint() {
        setup();
        let date_edit = DateEdit::new();
        let hint = date_edit.size_hint();
        assert!(hint.preferred.width >= 80.0);
        assert!(hint.preferred.height >= 22.0);
    }
}
