//! CalendarWidget for date selection.
//!
//! The CalendarWidget provides a monthly calendar view for selecting dates:
//! - Month grid view with day selection
//! - Navigation (previous/next month)
//! - Today highlight
//! - Date range constraints
//! - Optional week numbers
//! - Optional Today button
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::CalendarWidget;
//! use chrono::NaiveDate;
//!
//! let mut calendar = CalendarWidget::new()
//!     .with_date(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap())
//!     .with_week_numbers(true)
//!     .with_today_button(true);
//!
//! calendar.selection_changed.connect(|date| {
//!     if let Some(d) = date {
//!         println!("Selected: {}", d);
//!     }
//! });
//! ```

use chrono::{Datelike, Local, NaiveDate, Weekday};
use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, HorizontalAlign, Point, Rect, Renderer, RoundedRect,
    Stroke, TextLayout, TextLayoutOptions, TextRenderer, VerticalAlign,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

/// Parts of the calendar for hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum CalendarPart {
    #[default]
    None,
    /// Previous month button.
    PrevMonthButton,
    /// Next month button.
    NextMonthButton,
    /// Today button.
    TodayButton,
    /// Month/year label area.
    MonthYearLabel,
    /// A day cell in the grid (day of month 1-31 or 0 for other-month days).
    DayCell(u32, u32), // (row, col)
}

/// A calendar widget for selecting dates.
///
/// CalendarWidget displays a month view with clickable day cells.
/// It supports navigation between months and years, date range
/// constraints, and optional week number display.
///
/// # Signals
///
/// - `selection_changed(Option<NaiveDate>)`: Emitted when selection changes
/// - `activated(NaiveDate)`: Emitted when a date is activated (double-click or Enter)
/// - `page_changed((i32, u32))`: Emitted when the displayed month changes (year, month)
pub struct CalendarWidget {
    /// Widget base.
    base: WidgetBase,

    /// Currently selected date.
    selected_date: Option<NaiveDate>,

    /// First day of the displayed month.
    displayed_month: NaiveDate,

    /// Minimum selectable date.
    minimum_date: Option<NaiveDate>,

    /// Maximum selectable date.
    maximum_date: Option<NaiveDate>,

    /// First day of the week (Sunday or Monday).
    first_day_of_week: Weekday,

    /// Show week numbers column.
    show_week_numbers: bool,

    /// Show grid lines.
    show_grid_lines: bool,

    /// Show Today button.
    show_today_button: bool,

    // Appearance
    /// Background color.
    background_color: Color,
    /// Text color.
    text_color: Color,
    /// Header background color.
    header_background_color: Color,
    /// Header text color.
    header_text_color: Color,
    /// Selected day background color.
    selected_background_color: Color,
    /// Selected day text color.
    selected_text_color: Color,
    /// Today highlight color.
    today_highlight_color: Color,
    /// Weekend text color.
    weekend_color: Color,
    /// Other month days text color.
    other_month_color: Color,
    /// Disabled dates text color.
    disabled_color: Color,
    /// Hover color.
    hover_color: Color,
    /// Border color.
    border_color: Color,

    /// Font for day numbers.
    font: Font,
    /// Font for header.
    header_font: Font,
    /// Size of each day cell.
    cell_size: f32,
    /// Header height.
    header_height: f32,
    /// Weekday header height.
    weekday_header_height: f32,
    /// Navigation button size.
    nav_button_size: f32,
    /// Today button height.
    today_button_height: f32,

    /// Which part is currently hovered.
    hover_part: CalendarPart,
    /// Which part is currently pressed.
    pressed_part: CalendarPart,

    /// Signal emitted when selection changes.
    pub selection_changed: Signal<Option<NaiveDate>>,
    /// Signal emitted when a date is activated.
    pub activated: Signal<NaiveDate>,
    /// Signal emitted when the displayed month changes.
    pub page_changed: Signal<(i32, u32)>,
}

impl CalendarWidget {
    /// Create a new CalendarWidget with today's date displayed.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Fixed));

        let today = Local::now().date_naive();
        let displayed_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();

        Self {
            base,
            selected_date: None,
            displayed_month,
            minimum_date: None,
            maximum_date: None,
            first_day_of_week: Weekday::Sun,
            show_week_numbers: false,
            show_grid_lines: false,
            show_today_button: false,
            background_color: Color::WHITE,
            text_color: Color::BLACK,
            header_background_color: Color::from_rgb8(240, 240, 240),
            header_text_color: Color::BLACK,
            selected_background_color: Color::from_rgb8(51, 153, 255),
            selected_text_color: Color::WHITE,
            today_highlight_color: Color::from_rgb8(255, 193, 7),
            weekend_color: Color::from_rgb8(150, 150, 150),
            other_month_color: Color::from_rgb8(180, 180, 180),
            disabled_color: Color::from_rgb8(200, 200, 200),
            hover_color: Color::from_rgba8(51, 153, 255, 50),
            border_color: Color::from_rgb8(200, 200, 200),
            font: Font::new(FontFamily::SansSerif, 12.0),
            header_font: Font::new(FontFamily::SansSerif, 14.0),
            cell_size: 32.0,
            header_height: 36.0,
            weekday_header_height: 24.0,
            nav_button_size: 28.0,
            today_button_height: 28.0,
            hover_part: CalendarPart::None,
            pressed_part: CalendarPart::None,
            selection_changed: Signal::new(),
            activated: Signal::new(),
            page_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Date Access
    // =========================================================================

    /// Get the currently selected date.
    pub fn selected_date(&self) -> Option<NaiveDate> {
        self.selected_date
    }

    /// Set the selected date.
    pub fn set_selected_date(&mut self, date: Option<NaiveDate>) {
        if let Some(d) = date {
            if !self.is_date_valid(d) {
                return;
            }
        }
        if self.selected_date != date {
            self.selected_date = date;
            self.base.update();
            self.selection_changed.emit(date);
        }
    }

    /// Set selected date using builder pattern.
    pub fn with_date(mut self, date: NaiveDate) -> Self {
        self.selected_date = Some(date);
        // Also navigate to that month
        self.displayed_month = NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap();
        self
    }

    /// Get the displayed year and month.
    pub fn displayed_year_month(&self) -> (i32, u32) {
        (self.displayed_month.year(), self.displayed_month.month())
    }

    // =========================================================================
    // Date Constraints
    // =========================================================================

    /// Get the minimum selectable date.
    pub fn minimum_date(&self) -> Option<NaiveDate> {
        self.minimum_date
    }

    /// Set the minimum selectable date.
    pub fn set_minimum_date(&mut self, date: Option<NaiveDate>) {
        self.minimum_date = date;
        self.base.update();
    }

    /// Set minimum date using builder pattern.
    pub fn with_minimum_date(mut self, date: NaiveDate) -> Self {
        self.minimum_date = Some(date);
        self
    }

    /// Get the maximum selectable date.
    pub fn maximum_date(&self) -> Option<NaiveDate> {
        self.maximum_date
    }

    /// Set the maximum selectable date.
    pub fn set_maximum_date(&mut self, date: Option<NaiveDate>) {
        self.maximum_date = date;
        self.base.update();
    }

    /// Set maximum date using builder pattern.
    pub fn with_maximum_date(mut self, date: NaiveDate) -> Self {
        self.maximum_date = Some(date);
        self
    }

    /// Set the date range.
    pub fn set_date_range(&mut self, min: NaiveDate, max: NaiveDate) {
        self.minimum_date = Some(min);
        self.maximum_date = Some(max);
        self.base.update();
    }

    /// Set date range using builder pattern.
    pub fn with_date_range(mut self, min: NaiveDate, max: NaiveDate) -> Self {
        self.minimum_date = Some(min);
        self.maximum_date = Some(max);
        self
    }

    /// Check if a date is valid (within range).
    pub fn is_date_valid(&self, date: NaiveDate) -> bool {
        if let Some(min) = self.minimum_date {
            if date < min {
                return false;
            }
        }
        if let Some(max) = self.maximum_date {
            if date > max {
                return false;
            }
        }
        true
    }

    // =========================================================================
    // Display Options
    // =========================================================================

    /// Get the first day of the week.
    pub fn first_day_of_week(&self) -> Weekday {
        self.first_day_of_week
    }

    /// Set the first day of the week.
    pub fn set_first_day_of_week(&mut self, day: Weekday) {
        if self.first_day_of_week != day {
            self.first_day_of_week = day;
            self.base.update();
        }
    }

    /// Set first day of week using builder pattern.
    pub fn with_first_day_of_week(mut self, day: Weekday) -> Self {
        self.first_day_of_week = day;
        self
    }

    /// Check if week numbers are shown.
    pub fn show_week_numbers(&self) -> bool {
        self.show_week_numbers
    }

    /// Set whether to show week numbers.
    pub fn set_week_numbers(&mut self, show: bool) {
        if self.show_week_numbers != show {
            self.show_week_numbers = show;
            self.base.update();
        }
    }

    /// Set week numbers using builder pattern.
    pub fn with_week_numbers(mut self, show: bool) -> Self {
        self.show_week_numbers = show;
        self
    }

    /// Check if grid lines are shown.
    pub fn show_grid_lines(&self) -> bool {
        self.show_grid_lines
    }

    /// Set whether to show grid lines.
    pub fn set_grid_lines(&mut self, show: bool) {
        if self.show_grid_lines != show {
            self.show_grid_lines = show;
            self.base.update();
        }
    }

    /// Set grid lines using builder pattern.
    pub fn with_grid_lines(mut self, show: bool) -> Self {
        self.show_grid_lines = show;
        self
    }

    /// Check if Today button is shown.
    pub fn show_today_button(&self) -> bool {
        self.show_today_button
    }

    /// Set whether to show Today button.
    pub fn set_today_button(&mut self, show: bool) {
        if self.show_today_button != show {
            self.show_today_button = show;
            self.base.update();
        }
    }

    /// Set Today button using builder pattern.
    pub fn with_today_button(mut self, show: bool) -> Self {
        self.show_today_button = show;
        self
    }

    // =========================================================================
    // Navigation
    // =========================================================================

    /// Show the previous month.
    pub fn show_previous_month(&mut self) {
        let (year, month) = if self.displayed_month.month() == 1 {
            (self.displayed_month.year() - 1, 12)
        } else {
            (self.displayed_month.year(), self.displayed_month.month() - 1)
        };
        self.displayed_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        self.base.update();
        self.page_changed.emit((year, month));
    }

    /// Show the next month.
    pub fn show_next_month(&mut self) {
        let (year, month) = if self.displayed_month.month() == 12 {
            (self.displayed_month.year() + 1, 1)
        } else {
            (self.displayed_month.year(), self.displayed_month.month() + 1)
        };
        self.displayed_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        self.base.update();
        self.page_changed.emit((year, month));
    }

    /// Show the previous year.
    pub fn show_previous_year(&mut self) {
        let year = self.displayed_month.year() - 1;
        let month = self.displayed_month.month();
        self.displayed_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        self.base.update();
        self.page_changed.emit((year, month));
    }

    /// Show the next year.
    pub fn show_next_year(&mut self) {
        let year = self.displayed_month.year() + 1;
        let month = self.displayed_month.month();
        self.displayed_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        self.base.update();
        self.page_changed.emit((year, month));
    }

    /// Show today's month.
    pub fn show_today(&mut self) {
        let today = Local::now().date_naive();
        self.displayed_month = NaiveDate::from_ymd_opt(today.year(), today.month(), 1).unwrap();
        self.base.update();
        self.page_changed.emit((today.year(), today.month()));
    }

    /// Navigate to show a specific date's month.
    pub fn show_date(&mut self, date: NaiveDate) {
        let year = date.year();
        let month = date.month();
        if self.displayed_month.year() != year || self.displayed_month.month() != month {
            self.displayed_month = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
            self.base.update();
            self.page_changed.emit((year, month));
        }
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    fn week_number_width(&self) -> f32 {
        if self.show_week_numbers {
            28.0
        } else {
            0.0
        }
    }

    fn grid_width(&self) -> f32 {
        7.0 * self.cell_size + self.week_number_width()
    }

    fn total_height(&self) -> f32 {
        self.header_height
            + self.weekday_header_height
            + 6.0 * self.cell_size
            + if self.show_today_button {
                self.today_button_height
            } else {
                0.0
            }
    }

    fn header_rect(&self) -> Rect {
        Rect::new(0.0, 0.0, self.grid_width(), self.header_height)
    }

    fn weekday_header_rect(&self) -> Rect {
        Rect::new(
            self.week_number_width(),
            self.header_height,
            7.0 * self.cell_size,
            self.weekday_header_height,
        )
    }

    fn grid_rect(&self) -> Rect {
        let top = self.header_height + self.weekday_header_height;
        Rect::new(
            self.week_number_width(),
            top,
            7.0 * self.cell_size,
            6.0 * self.cell_size,
        )
    }

    fn cell_rect(&self, row: u32, col: u32) -> Rect {
        let grid = self.grid_rect();
        Rect::new(
            grid.origin.x + col as f32 * self.cell_size,
            grid.origin.y + row as f32 * self.cell_size,
            self.cell_size,
            self.cell_size,
        )
    }

    fn prev_button_rect(&self) -> Rect {
        Rect::new(4.0, 4.0, self.nav_button_size, self.nav_button_size)
    }

    fn next_button_rect(&self) -> Rect {
        Rect::new(
            self.grid_width() - self.nav_button_size - 4.0,
            4.0,
            self.nav_button_size,
            self.nav_button_size,
        )
    }

    fn today_button_rect(&self) -> Option<Rect> {
        if self.show_today_button {
            let top = self.header_height + self.weekday_header_height + 6.0 * self.cell_size;
            Some(Rect::new(4.0, top, self.grid_width() - 8.0, self.today_button_height))
        } else {
            None
        }
    }

    // =========================================================================
    // Grid Calculation
    // =========================================================================

    /// Get the weekday index (0-6) for a given weekday based on first_day_of_week.
    fn weekday_index(&self, day: Weekday) -> u32 {
        let first = self.first_day_of_week.num_days_from_sunday();
        let this = day.num_days_from_sunday();
        (this + 7 - first) % 7
    }

    /// Get days to display in the grid (42 cells = 6 rows x 7 cols).
    fn days_in_grid(&self) -> Vec<(NaiveDate, bool)> {
        let mut days = Vec::with_capacity(42);
        let year = self.displayed_month.year();
        let month = self.displayed_month.month();

        // First day of the month
        let first_of_month = self.displayed_month;
        let first_weekday = first_of_month.weekday();
        let start_offset = self.weekday_index(first_weekday) as i64;

        // Start date (may be in previous month)
        let start_date = first_of_month - chrono::Duration::days(start_offset);

        for i in 0..42 {
            let date = start_date + chrono::Duration::days(i);
            let is_current_month = date.year() == year && date.month() == month;
            days.push((date, is_current_month));
        }

        days
    }

    /// Get the date at a grid position.
    fn date_at_cell(&self, row: u32, col: u32) -> Option<NaiveDate> {
        let days = self.days_in_grid();
        let index = row as usize * 7 + col as usize;
        days.get(index).map(|(date, _)| *date)
    }

    /// Hit test to determine which part is at a point.
    fn hit_test(&self, pos: Point) -> CalendarPart {
        // Check navigation buttons
        if self.prev_button_rect().contains(pos) {
            return CalendarPart::PrevMonthButton;
        }
        if self.next_button_rect().contains(pos) {
            return CalendarPart::NextMonthButton;
        }

        // Check today button
        if let Some(rect) = self.today_button_rect() {
            if rect.contains(pos) {
                return CalendarPart::TodayButton;
            }
        }

        // Check header area
        let header = self.header_rect();
        if header.contains(pos) {
            return CalendarPart::MonthYearLabel;
        }

        // Check grid cells
        let grid = self.grid_rect();
        if grid.contains(pos) {
            let local_x = pos.x - grid.origin.x;
            let local_y = pos.y - grid.origin.y;
            let col = (local_x / self.cell_size) as u32;
            let row = (local_y / self.cell_size) as u32;
            if col < 7 && row < 6 {
                return CalendarPart::DayCell(row, col);
            }
        }

        CalendarPart::None
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
            CalendarPart::PrevMonthButton => {
                self.show_previous_month();
                self.base.update();
                true
            }
            CalendarPart::NextMonthButton => {
                self.show_next_month();
                self.base.update();
                true
            }
            CalendarPart::TodayButton => {
                let today = Local::now().date_naive();
                if self.is_date_valid(today) {
                    self.show_today();
                    self.set_selected_date(Some(today));
                }
                true
            }
            CalendarPart::DayCell(row, col) => {
                if let Some(date) = self.date_at_cell(row, col) {
                    if self.is_date_valid(date) {
                        self.set_selected_date(Some(date));
                        // Navigate to the date's month if it's in a different month
                        if date.month() != self.displayed_month.month()
                            || date.year() != self.displayed_month.year()
                        {
                            self.show_date(date);
                        }
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }
        self.pressed_part = CalendarPart::None;
        self.base.update();
        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let new_hover = self.hit_test(event.local_pos);
        if self.hover_part != new_hover {
            self.hover_part = new_hover;
            self.base.update();
            return true;
        }
        false
    }

    fn handle_double_click(&mut self, event: &crate::widget::MouseDoubleClickEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let part = self.hit_test(event.local_pos);
        if let CalendarPart::DayCell(row, col) = part {
            if let Some(date) = self.date_at_cell(row, col) {
                if self.is_date_valid(date) {
                    self.activated.emit(date);
                    return true;
                }
            }
        }
        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::ArrowLeft => {
                if let Some(date) = self.selected_date {
                    let new_date = date - chrono::Duration::days(1);
                    if self.is_date_valid(new_date) {
                        self.set_selected_date(Some(new_date));
                        self.show_date(new_date);
                    }
                }
                true
            }
            Key::ArrowRight => {
                if let Some(date) = self.selected_date {
                    let new_date = date + chrono::Duration::days(1);
                    if self.is_date_valid(new_date) {
                        self.set_selected_date(Some(new_date));
                        self.show_date(new_date);
                    }
                }
                true
            }
            Key::ArrowUp => {
                if let Some(date) = self.selected_date {
                    let new_date = date - chrono::Duration::days(7);
                    if self.is_date_valid(new_date) {
                        self.set_selected_date(Some(new_date));
                        self.show_date(new_date);
                    }
                }
                true
            }
            Key::ArrowDown => {
                if let Some(date) = self.selected_date {
                    let new_date = date + chrono::Duration::days(7);
                    if self.is_date_valid(new_date) {
                        self.set_selected_date(Some(new_date));
                        self.show_date(new_date);
                    }
                }
                true
            }
            Key::PageUp => {
                if event.modifiers.shift {
                    self.show_previous_year();
                } else {
                    self.show_previous_month();
                }
                true
            }
            Key::PageDown => {
                if event.modifiers.shift {
                    self.show_next_year();
                } else {
                    self.show_next_month();
                }
                true
            }
            Key::Home => {
                // Go to first day of month
                let first = self.displayed_month;
                if self.is_date_valid(first) {
                    self.set_selected_date(Some(first));
                }
                true
            }
            Key::End => {
                // Go to last day of month
                let year = self.displayed_month.year();
                let month = self.displayed_month.month();
                let last_day = if month == 12 {
                    NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap() - chrono::Duration::days(1)
                } else {
                    NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap() - chrono::Duration::days(1)
                };
                if self.is_date_valid(last_day) {
                    self.set_selected_date(Some(last_day));
                }
                true
            }
            Key::Enter => {
                if let Some(date) = self.selected_date {
                    self.activated.emit(date);
                }
                true
            }
            _ => false,
        }
    }

    fn handle_leave(&mut self) -> bool {
        if self.hover_part != CalendarPart::None {
            self.hover_part = CalendarPart::None;
            self.base.update();
        }
        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = Rect::new(0.0, 0.0, self.grid_width(), self.total_height());
        ctx.renderer().fill_rect(rect, self.background_color);

        // Border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rect(rect, &stroke);
    }

    fn paint_header(&self, ctx: &mut PaintContext<'_>) {
        let header = self.header_rect();
        ctx.renderer().fill_rect(header, self.header_background_color);

        // Month/Year text
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
        let month_name = month_names[(self.displayed_month.month() - 1) as usize];
        let text = format!("{} {}", month_name, self.displayed_month.year());

        let mut font_system = FontSystem::new();
        let layout = TextLayout::with_options(
            &mut font_system,
            &text,
            &self.header_font,
            TextLayoutOptions::new()
                .horizontal_align(HorizontalAlign::Center)
                .vertical_align(VerticalAlign::Middle),
        );

        let text_x = header.origin.x + (header.width() - layout.width()) / 2.0;
        let text_y = header.origin.y + (header.height() - layout.height()) / 2.0;

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                self.header_text_color,
            );
        }

        // Navigation buttons
        self.paint_nav_button(ctx, self.prev_button_rect(), true);
        self.paint_nav_button(ctx, self.next_button_rect(), false);
    }

    fn paint_nav_button(&self, ctx: &mut PaintContext<'_>, rect: Rect, is_prev: bool) {
        let is_hovered = if is_prev {
            matches!(self.hover_part, CalendarPart::PrevMonthButton)
        } else {
            matches!(self.hover_part, CalendarPart::NextMonthButton)
        };

        let is_pressed = if is_prev {
            matches!(self.pressed_part, CalendarPart::PrevMonthButton)
        } else {
            matches!(self.pressed_part, CalendarPart::NextMonthButton)
        };

        // Button background
        let bg_color = if is_pressed {
            Color::from_rgb8(180, 180, 180)
        } else if is_hovered {
            Color::from_rgb8(220, 220, 220)
        } else {
            Color::from_rgb8(240, 240, 240)
        };

        let rrect = RoundedRect::new(rect, 4.0);
        ctx.renderer().fill_rounded_rect(rrect, bg_color);

        // Arrow
        let center_x = rect.origin.x + rect.width() / 2.0;
        let center_y = rect.origin.y + rect.height() / 2.0;
        let arrow_size = 6.0;
        let stroke = Stroke::new(Color::from_rgb8(80, 80, 80), 2.0);

        if is_prev {
            // Left arrow
            let p1 = Point::new(center_x + arrow_size / 2.0, center_y - arrow_size);
            let p2 = Point::new(center_x - arrow_size / 2.0, center_y);
            let p3 = Point::new(center_x + arrow_size / 2.0, center_y + arrow_size);
            ctx.renderer().draw_line(p1, p2, &stroke);
            ctx.renderer().draw_line(p2, p3, &stroke);
        } else {
            // Right arrow
            let p1 = Point::new(center_x - arrow_size / 2.0, center_y - arrow_size);
            let p2 = Point::new(center_x + arrow_size / 2.0, center_y);
            let p3 = Point::new(center_x - arrow_size / 2.0, center_y + arrow_size);
            ctx.renderer().draw_line(p1, p2, &stroke);
            ctx.renderer().draw_line(p2, p3, &stroke);
        }
    }

    fn paint_weekday_headers(&self, ctx: &mut PaintContext<'_>) {
        let header_rect = self.weekday_header_rect();
        let weekdays = if self.first_day_of_week == Weekday::Sun {
            ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
        } else {
            ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
        };

        let mut font_system = FontSystem::new();

        for (i, day) in weekdays.iter().enumerate() {
            let cell_rect = Rect::new(
                header_rect.origin.x + i as f32 * self.cell_size,
                header_rect.origin.y,
                self.cell_size,
                self.weekday_header_height,
            );

            let layout = TextLayout::with_options(
                &mut font_system,
                day,
                &self.font,
                TextLayoutOptions::new()
                    .horizontal_align(HorizontalAlign::Center)
                    .vertical_align(VerticalAlign::Middle),
            );

            let text_x = cell_rect.origin.x + (cell_rect.width() - layout.width()) / 2.0;
            let text_y = cell_rect.origin.y + (cell_rect.height() - layout.height()) / 2.0;

            // Weekend days in different color
            let is_weekend = if self.first_day_of_week == Weekday::Sun {
                i == 0 || i == 6
            } else {
                i == 5 || i == 6
            };

            let color = if is_weekend {
                self.weekend_color
            } else {
                self.text_color
            };

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    color,
                );
            }
        }
    }

    fn paint_grid(&self, ctx: &mut PaintContext<'_>) {
        let days = self.days_in_grid();
        let today = Local::now().date_naive();
        let mut font_system = FontSystem::new();

        for row in 0..6 {
            for col in 0..7 {
                let index = row * 7 + col;
                let (date, is_current_month) = days[index];
                let cell_rect = self.cell_rect(row as u32, col as u32);

                self.paint_day_cell(
                    ctx,
                    &mut font_system,
                    cell_rect,
                    date,
                    is_current_month,
                    today,
                    row as u32,
                    col as u32,
                );
            }
        }

        // Grid lines
        if self.show_grid_lines {
            let grid = self.grid_rect();
            let stroke = Stroke::new(Color::from_rgb8(230, 230, 230), 1.0);

            // Horizontal lines
            for row in 1..6 {
                let y = grid.origin.y + row as f32 * self.cell_size;
                ctx.renderer().draw_line(
                    Point::new(grid.origin.x, y),
                    Point::new(grid.origin.x + grid.width(), y),
                    &stroke,
                );
            }

            // Vertical lines
            for col in 1..7 {
                let x = grid.origin.x + col as f32 * self.cell_size;
                ctx.renderer().draw_line(
                    Point::new(x, grid.origin.y),
                    Point::new(x, grid.origin.y + grid.height()),
                    &stroke,
                );
            }
        }
    }

    fn paint_day_cell(
        &self,
        ctx: &mut PaintContext<'_>,
        font_system: &mut FontSystem,
        rect: Rect,
        date: NaiveDate,
        is_current_month: bool,
        today: NaiveDate,
        row: u32,
        col: u32,
    ) {
        let is_selected = self.selected_date == Some(date);
        let is_today = date == today;
        let is_valid = self.is_date_valid(date);
        let is_weekend = date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun;
        let is_hovered = matches!(self.hover_part, CalendarPart::DayCell(r, c) if r == row && c == col);

        // Background
        if is_selected && is_valid {
            let center = Point::new(
                rect.origin.x + rect.width() / 2.0,
                rect.origin.y + rect.height() / 2.0,
            );
            let radius = (rect.width().min(rect.height()) / 2.0 - 2.0).max(0.0);
            let circle_rect = Rect::new(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0);
            let rrect = RoundedRect::new(circle_rect, radius);
            ctx.renderer().fill_rounded_rect(rrect, self.selected_background_color);
        } else if is_hovered && is_valid && is_current_month {
            let center = Point::new(
                rect.origin.x + rect.width() / 2.0,
                rect.origin.y + rect.height() / 2.0,
            );
            let radius = (rect.width().min(rect.height()) / 2.0 - 2.0).max(0.0);
            let circle_rect = Rect::new(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0);
            let rrect = RoundedRect::new(circle_rect, radius);
            ctx.renderer().fill_rounded_rect(rrect, self.hover_color);
        }

        // Today highlight (ring)
        if is_today && !is_selected {
            let center = Point::new(
                rect.origin.x + rect.width() / 2.0,
                rect.origin.y + rect.height() / 2.0,
            );
            let radius = (rect.width().min(rect.height()) / 2.0 - 2.0).max(0.0);
            let circle_rect = Rect::new(center.x - radius, center.y - radius, radius * 2.0, radius * 2.0);
            let rrect = RoundedRect::new(circle_rect, radius);
            let stroke = Stroke::new(self.today_highlight_color, 2.0);
            ctx.renderer().stroke_rounded_rect(rrect, &stroke);
        }

        // Day number
        let text = date.day().to_string();
        let layout = TextLayout::with_options(
            font_system,
            &text,
            &self.font,
            TextLayoutOptions::new()
                .horizontal_align(HorizontalAlign::Center)
                .vertical_align(VerticalAlign::Middle),
        );

        let text_x = rect.origin.x + (rect.width() - layout.width()) / 2.0;
        let text_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;

        // Text color
        let color = if is_selected && is_valid {
            self.selected_text_color
        } else if !is_valid {
            self.disabled_color
        } else if !is_current_month {
            self.other_month_color
        } else if is_weekend {
            self.weekend_color
        } else {
            self.text_color
        };

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                font_system,
                &layout,
                Point::new(text_x, text_y),
                color,
            );
        }
    }

    fn paint_week_numbers(&self, ctx: &mut PaintContext<'_>) {
        if !self.show_week_numbers {
            return;
        }

        let days = self.days_in_grid();
        let mut font_system = FontSystem::new();

        for row in 0..6 {
            let (date, _) = days[row * 7]; // First day of the row
            let week_num = date.iso_week().week();

            let rect = Rect::new(
                0.0,
                self.header_height + self.weekday_header_height + row as f32 * self.cell_size,
                self.week_number_width(),
                self.cell_size,
            );

            let text = week_num.to_string();
            let layout = TextLayout::with_options(
                &mut font_system,
                &text,
                &self.font,
                TextLayoutOptions::new()
                    .horizontal_align(HorizontalAlign::Center)
                    .vertical_align(VerticalAlign::Middle),
            );

            let text_x = rect.origin.x + (rect.width() - layout.width()) / 2.0;
            let text_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    self.other_month_color,
                );
            }
        }
    }

    fn paint_today_button(&self, ctx: &mut PaintContext<'_>) {
        if let Some(rect) = self.today_button_rect() {
            let is_hovered = matches!(self.hover_part, CalendarPart::TodayButton);
            let is_pressed = matches!(self.pressed_part, CalendarPart::TodayButton);

            let bg_color = if is_pressed {
                Color::from_rgb8(180, 180, 180)
            } else if is_hovered {
                Color::from_rgb8(220, 220, 220)
            } else {
                Color::from_rgb8(240, 240, 240)
            };

            let rrect = RoundedRect::new(rect, 4.0);
            ctx.renderer().fill_rounded_rect(rrect, bg_color);

            // Border
            let stroke = Stroke::new(self.border_color, 1.0);
            ctx.renderer().stroke_rounded_rect(rrect, &stroke);

            // Text
            let mut font_system = FontSystem::new();
            let layout = TextLayout::with_options(
                &mut font_system,
                "Today",
                &self.font,
                TextLayoutOptions::new()
                    .horizontal_align(HorizontalAlign::Center)
                    .vertical_align(VerticalAlign::Middle),
            );

            let text_x = rect.origin.x + (rect.width() - layout.width()) / 2.0;
            let text_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    Point::new(text_x, text_y),
                    self.text_color,
                );
            }
        }
    }

    fn paint_focus_indicator(&self, ctx: &mut PaintContext<'_>) {
        if !self.widget_base().has_focus() {
            return;
        }

        let rect = Rect::new(0.0, 0.0, self.grid_width(), self.total_height());
        let focus_color = Color::from_rgba8(66, 133, 244, 180);
        let focus_stroke = Stroke::new(focus_color, 2.0);
        ctx.renderer().stroke_rect(rect, &focus_stroke);
    }
}

impl Default for CalendarWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for CalendarWidget {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for CalendarWidget {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let width = self.grid_width();
        let height = self.total_height();
        SizeHint::from_dimensions(width, height)
            .with_minimum_dimensions(width, height)
            .with_maximum_dimensions(width, height)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);
        self.paint_header(ctx);
        self.paint_weekday_headers(ctx);
        self.paint_week_numbers(ctx);
        self.paint_grid(ctx);
        self.paint_today_button(ctx);
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
            WidgetEvent::DoubleClick(e) => {
                if self.handle_double_click(e) {
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
            WidgetEvent::Leave(_) => {
                self.handle_leave();
            }
            _ => {}
        }
        false
    }
}

// Ensure CalendarWidget is Send + Sync
static_assertions::assert_impl_all!(CalendarWidget: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_calendar_creation() {
        setup();
        let calendar = CalendarWidget::new();
        assert!(calendar.selected_date().is_none());
        assert!(!calendar.show_week_numbers());
        assert!(!calendar.show_today_button());
    }

    #[test]
    fn test_calendar_with_date() {
        setup();
        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let calendar = CalendarWidget::new().with_date(date);
        assert_eq!(calendar.selected_date(), Some(date));
        assert_eq!(calendar.displayed_year_month(), (2025, 6));
    }

    #[test]
    fn test_calendar_date_range() {
        setup();
        let min = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let max = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let calendar = CalendarWidget::new().with_date_range(min, max);

        assert!(calendar.is_date_valid(NaiveDate::from_ymd_opt(2025, 6, 15).unwrap()));
        assert!(!calendar.is_date_valid(NaiveDate::from_ymd_opt(2024, 12, 31).unwrap()));
        assert!(!calendar.is_date_valid(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()));
    }

    #[test]
    fn test_calendar_navigation() {
        setup();
        let mut calendar = CalendarWidget::new();
        let initial = calendar.displayed_year_month();

        calendar.show_next_month();
        let (year, month) = calendar.displayed_year_month();
        assert!(month == (initial.1 % 12) + 1 || (initial.1 == 12 && month == 1));

        calendar.show_previous_month();
        assert_eq!(calendar.displayed_year_month(), initial);
    }

    #[test]
    fn test_calendar_selection_signal() {
        setup();
        let mut calendar = CalendarWidget::new();
        let signal_fired = Arc::new(AtomicBool::new(false));
        let signal_fired_clone = signal_fired.clone();

        calendar.selection_changed.connect(move |_| {
            signal_fired_clone.store(true, Ordering::SeqCst);
        });

        let date = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        calendar.set_selected_date(Some(date));
        assert!(signal_fired.load(Ordering::SeqCst));
    }

    #[test]
    fn test_calendar_builder_pattern() {
        setup();
        let min = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let max = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
        let date = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

        let calendar = CalendarWidget::new()
            .with_date(date)
            .with_date_range(min, max)
            .with_first_day_of_week(Weekday::Mon)
            .with_week_numbers(true)
            .with_today_button(true)
            .with_grid_lines(true);

        assert_eq!(calendar.selected_date(), Some(date));
        assert_eq!(calendar.minimum_date(), Some(min));
        assert_eq!(calendar.maximum_date(), Some(max));
        assert_eq!(calendar.first_day_of_week(), Weekday::Mon);
        assert!(calendar.show_week_numbers());
        assert!(calendar.show_today_button());
        assert!(calendar.show_grid_lines());
    }

    #[test]
    fn test_calendar_days_in_grid() {
        setup();
        let mut calendar = CalendarWidget::new();
        // Set to a known month (January 2025 starts on Wednesday)
        calendar.displayed_month = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();

        let days = calendar.days_in_grid();
        assert_eq!(days.len(), 42); // 6 rows * 7 cols

        // First day should be in December if Sunday start
        let (first_date, first_current) = days[0];
        assert!(!first_current || first_date.month() == 1);
    }

    #[test]
    fn test_calendar_size_hint() {
        setup();
        let calendar = CalendarWidget::new();
        let hint = calendar.size_hint();
        assert!(hint.preferred.width > 0.0);
        assert!(hint.preferred.height > 0.0);
    }
}
