//! TimeEdit widget for time input.
//!
//! The TimeEdit widget provides a way to enter and modify times with:
//! - Section-based editing (hour, minute, second, AM/PM)
//! - Increment/decrement buttons
//! - 12-hour and 24-hour format support
//! - Optional seconds display
//! - Range constraints
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{TimeEdit, TimeFormat};
//! use chrono::NaiveTime;
//!
//! // Create a time editor with 12-hour format
//! let mut time_edit = TimeEdit::new()
//!     .with_time(NaiveTime::from_hms_opt(14, 30, 0).unwrap())
//!     .with_display_format(TimeFormat::Hour12)
//!     .with_show_seconds(false);
//!
//! // Connect to time changes
//! time_edit.time_changed.connect(|time| {
//!     println!("Time changed: {}", time);
//! });
//! ```

use chrono::{NaiveTime, Timelike};
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

/// Display format for times.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeFormat {
    /// 24-hour format without seconds: HH:MM
    #[default]
    Hour24,
    /// 24-hour format with seconds: HH:MM:SS
    Hour24Seconds,
    /// 12-hour format without seconds: hh:MM AM/PM
    Hour12,
    /// 12-hour format with seconds: hh:MM:SS AM/PM
    Hour12Seconds,
}

impl TimeFormat {
    /// Check if this format shows seconds.
    pub fn shows_seconds(&self) -> bool {
        matches!(self, TimeFormat::Hour24Seconds | TimeFormat::Hour12Seconds)
    }

    /// Check if this format uses 12-hour clock.
    pub fn is_12_hour(&self) -> bool {
        matches!(self, TimeFormat::Hour12 | TimeFormat::Hour12Seconds)
    }
}

/// Parts of the TimeEdit for hit testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TimeEditPart {
    #[default]
    None,
    /// Hour section.
    HourSection,
    /// Minute section.
    MinuteSection,
    /// Second section.
    SecondSection,
    /// AM/PM section.
    AmPmSection,
    /// Up (increment) button.
    UpButton,
    /// Down (decrement) button.
    DownButton,
}

/// Which section is currently focused for editing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum EditSection {
    #[default]
    None,
    Hour,
    Minute,
    Second,
    AmPm,
}

/// A widget for entering and modifying times.
///
/// TimeEdit provides a text field showing the current time with editable
/// sections for hour, minute, second (optional), and AM/PM (12-hour mode).
///
/// # Signals
///
/// - `time_changed(NaiveTime)`: Emitted when the time changes
/// - `editing_finished()`: Emitted when editing is completed
pub struct TimeEdit {
    /// Widget base.
    base: WidgetBase,

    /// Current time value.
    time: NaiveTime,

    /// Minimum selectable time.
    minimum_time: NaiveTime,

    /// Maximum selectable time.
    maximum_time: NaiveTime,

    /// Display format.
    display_format: TimeFormat,

    /// Whether to show seconds (overrides format).
    show_seconds: Option<bool>,

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

    /// Which part is currently hovered.
    hover_part: TimeEditPart,
    /// Which part is currently pressed.
    pressed_part: TimeEditPart,

    /// Signal emitted when time changes.
    pub time_changed: Signal<NaiveTime>,
    /// Signal emitted when editing is finished.
    pub editing_finished: Signal<()>,
}

impl TimeEdit {
    /// Create a new TimeEdit with midnight (00:00:00).
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Fixed));

        Self {
            base,
            time: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            minimum_time: NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
            maximum_time: NaiveTime::from_hms_opt(23, 59, 59).unwrap(),
            display_format: TimeFormat::Hour24,
            show_seconds: None,
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
            hover_part: TimeEditPart::None,
            pressed_part: TimeEditPart::None,
            time_changed: Signal::new(),
            editing_finished: Signal::new(),
        }
    }

    // =========================================================================
    // Time Access
    // =========================================================================

    /// Get the current time.
    pub fn time(&self) -> NaiveTime {
        self.time
    }

    /// Set the current time.
    pub fn set_time(&mut self, time: NaiveTime) {
        let clamped = self.clamp_time(time);
        if self.time != clamped {
            self.time = clamped;
            self.base.update();
            self.time_changed.emit(clamped);
        }
    }

    /// Set time using builder pattern.
    pub fn with_time(mut self, time: NaiveTime) -> Self {
        self.time = self.clamp_time(time);
        self
    }

    fn clamp_time(&self, time: NaiveTime) -> NaiveTime {
        if time < self.minimum_time {
            self.minimum_time
        } else if time > self.maximum_time {
            self.maximum_time
        } else {
            time
        }
    }

    // =========================================================================
    // Time Constraints
    // =========================================================================

    /// Get the minimum time.
    pub fn minimum_time(&self) -> NaiveTime {
        self.minimum_time
    }

    /// Set the minimum time.
    pub fn set_minimum_time(&mut self, time: NaiveTime) {
        self.minimum_time = time;
        if self.time < time {
            self.set_time(time);
        }
    }

    /// Set minimum time using builder pattern.
    pub fn with_minimum_time(mut self, time: NaiveTime) -> Self {
        self.minimum_time = time;
        if self.time < time {
            self.time = time;
        }
        self
    }

    /// Get the maximum time.
    pub fn maximum_time(&self) -> NaiveTime {
        self.maximum_time
    }

    /// Set the maximum time.
    pub fn set_maximum_time(&mut self, time: NaiveTime) {
        self.maximum_time = time;
        if self.time > time {
            self.set_time(time);
        }
    }

    /// Set maximum time using builder pattern.
    pub fn with_maximum_time(mut self, time: NaiveTime) -> Self {
        self.maximum_time = time;
        if self.time > time {
            self.time = time;
        }
        self
    }

    /// Set the time range.
    pub fn set_time_range(&mut self, min: NaiveTime, max: NaiveTime) {
        self.minimum_time = min;
        self.maximum_time = max;
        let clamped = self.clamp_time(self.time);
        if self.time != clamped {
            self.set_time(clamped);
        }
    }

    /// Set time range using builder pattern.
    pub fn with_time_range(mut self, min: NaiveTime, max: NaiveTime) -> Self {
        self.minimum_time = min;
        self.maximum_time = max;
        self.time = self.clamp_time(self.time);
        self
    }

    // =========================================================================
    // Display Options
    // =========================================================================

    /// Get the display format.
    pub fn display_format(&self) -> TimeFormat {
        self.display_format
    }

    /// Set the display format.
    pub fn set_display_format(&mut self, format: TimeFormat) {
        if self.display_format != format {
            self.display_format = format;
            self.base.update();
        }
    }

    /// Set display format using builder pattern.
    pub fn with_display_format(mut self, format: TimeFormat) -> Self {
        self.display_format = format;
        self
    }

    /// Check if seconds are shown.
    pub fn seconds_shown(&self) -> bool {
        self.show_seconds.unwrap_or_else(|| self.display_format.shows_seconds())
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
    // Section Navigation
    // =========================================================================

    fn is_12_hour(&self) -> bool {
        self.display_format.is_12_hour()
    }

    fn next_section(&mut self) {
        let show_seconds = self.seconds_shown();
        let is_12_hour = self.is_12_hour();

        self.current_section = match self.current_section {
            EditSection::None => EditSection::Hour,
            EditSection::Hour => EditSection::Minute,
            EditSection::Minute => {
                if show_seconds {
                    EditSection::Second
                } else if is_12_hour {
                    EditSection::AmPm
                } else {
                    EditSection::None
                }
            }
            EditSection::Second => {
                if is_12_hour {
                    EditSection::AmPm
                } else {
                    EditSection::None
                }
            }
            EditSection::AmPm => EditSection::None,
        };
        self.base.update();
    }

    fn previous_section(&mut self) {
        let show_seconds = self.seconds_shown();
        let is_12_hour = self.is_12_hour();

        self.current_section = match self.current_section {
            EditSection::None => {
                if is_12_hour {
                    EditSection::AmPm
                } else if show_seconds {
                    EditSection::Second
                } else {
                    EditSection::Minute
                }
            }
            EditSection::Hour => EditSection::None,
            EditSection::Minute => EditSection::Hour,
            EditSection::Second => EditSection::Minute,
            EditSection::AmPm => {
                if show_seconds {
                    EditSection::Second
                } else {
                    EditSection::Minute
                }
            }
        };
        self.base.update();
    }

    fn step_up_section(&mut self) {
        if self.read_only {
            return;
        }

        let new_time = match self.current_section {
            EditSection::None => return,
            EditSection::Hour => {
                let new_hour = (self.time.hour() + 1) % 24;
                NaiveTime::from_hms_opt(new_hour, self.time.minute(), self.time.second())
            }
            EditSection::Minute => {
                let new_minute = (self.time.minute() + 1) % 60;
                NaiveTime::from_hms_opt(self.time.hour(), new_minute, self.time.second())
            }
            EditSection::Second => {
                let new_second = (self.time.second() + 1) % 60;
                NaiveTime::from_hms_opt(self.time.hour(), self.time.minute(), new_second)
            }
            EditSection::AmPm => {
                // Toggle AM/PM (add/subtract 12 hours)
                let new_hour = (self.time.hour() + 12) % 24;
                NaiveTime::from_hms_opt(new_hour, self.time.minute(), self.time.second())
            }
        };

        if let Some(t) = new_time {
            self.set_time(t);
        }
    }

    fn step_down_section(&mut self) {
        if self.read_only {
            return;
        }

        let new_time = match self.current_section {
            EditSection::None => return,
            EditSection::Hour => {
                let new_hour = if self.time.hour() == 0 { 23 } else { self.time.hour() - 1 };
                NaiveTime::from_hms_opt(new_hour, self.time.minute(), self.time.second())
            }
            EditSection::Minute => {
                let new_minute = if self.time.minute() == 0 { 59 } else { self.time.minute() - 1 };
                NaiveTime::from_hms_opt(self.time.hour(), new_minute, self.time.second())
            }
            EditSection::Second => {
                let new_second = if self.time.second() == 0 { 59 } else { self.time.second() - 1 };
                NaiveTime::from_hms_opt(self.time.hour(), self.time.minute(), new_second)
            }
            EditSection::AmPm => {
                // Toggle AM/PM
                let new_hour = (self.time.hour() + 12) % 24;
                NaiveTime::from_hms_opt(new_hour, self.time.minute(), self.time.second())
            }
        };

        if let Some(t) = new_time {
            self.set_time(t);
        }
    }

    // =========================================================================
    // Formatting
    // =========================================================================

    fn format_time(&self) -> String {
        let show_seconds = self.seconds_shown();

        if self.is_12_hour() {
            let hour = self.time.hour();
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
                    self.time.minute(),
                    self.time.second(),
                    am_pm
                )
            } else {
                format!(
                    "{:02}:{:02} {}",
                    display_hour,
                    self.time.minute(),
                    am_pm
                )
            }
        } else {
            if show_seconds {
                format!(
                    "{:02}:{:02}:{:02}",
                    self.time.hour(),
                    self.time.minute(),
                    self.time.second()
                )
            } else {
                format!(
                    "{:02}:{:02}",
                    self.time.hour(),
                    self.time.minute()
                )
            }
        }
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    fn text_field_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            0.0,
            0.0,
            (rect.width() - self.button_width).max(0.0),
            rect.height(),
        )
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

    fn section_from_position(&self, _pos: Point) -> EditSection {
        // Simplified - cycle through sections
        let is_12_hour = self.is_12_hour();
        let show_seconds = self.seconds_shown();

        match self.current_section {
            EditSection::None => EditSection::Hour,
            EditSection::Hour => EditSection::Minute,
            EditSection::Minute => {
                if show_seconds {
                    EditSection::Second
                } else if is_12_hour {
                    EditSection::AmPm
                } else {
                    EditSection::Hour
                }
            }
            EditSection::Second => {
                if is_12_hour {
                    EditSection::AmPm
                } else {
                    EditSection::Hour
                }
            }
            EditSection::AmPm => EditSection::Hour,
        }
    }

    fn hit_test(&self, pos: Point) -> TimeEditPart {
        if self.up_button_rect().contains(pos) {
            return TimeEditPart::UpButton;
        }
        if self.down_button_rect().contains(pos) {
            return TimeEditPart::DownButton;
        }

        let text_rect = self.text_field_rect();
        if text_rect.contains(pos) {
            match self.section_from_position(pos) {
                EditSection::Hour => return TimeEditPart::HourSection,
                EditSection::Minute => return TimeEditPart::MinuteSection,
                EditSection::Second => return TimeEditPart::SecondSection,
                EditSection::AmPm => return TimeEditPart::AmPmSection,
                EditSection::None => return TimeEditPart::None,
            }
        }

        TimeEditPart::None
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
            TimeEditPart::UpButton => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Minute;
                    }
                    self.step_up_section();
                    self.base.update();
                }
                true
            }
            TimeEditPart::DownButton => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Minute;
                    }
                    self.step_down_section();
                    self.base.update();
                }
                true
            }
            TimeEditPart::HourSection
            | TimeEditPart::MinuteSection
            | TimeEditPart::SecondSection
            | TimeEditPart::AmPmSection => {
                self.current_section = match part {
                    TimeEditPart::HourSection => EditSection::Hour,
                    TimeEditPart::MinuteSection => EditSection::Minute,
                    TimeEditPart::SecondSection => EditSection::Second,
                    TimeEditPart::AmPmSection => EditSection::AmPm,
                    _ => EditSection::None,
                };
                self.base.update();
                true
            }
            TimeEditPart::None => false,
        }
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.pressed_part != TimeEditPart::None {
            self.pressed_part = TimeEditPart::None;
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
            self.current_section = EditSection::Minute;
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
            Key::Enter => {
                self.editing_finished.emit(());
                return true;
            }
            Key::ArrowUp => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Minute;
                    }
                    self.step_up_section();
                    return true;
                }
            }
            Key::ArrowDown => {
                if !self.read_only {
                    if self.current_section == EditSection::None {
                        self.current_section = EditSection::Minute;
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
                self.set_time(self.minimum_time);
                return true;
            }
            Key::End => {
                self.set_time(self.maximum_time);
                return true;
            }
            _ => {
                // Handle A/P for AM/PM toggle
                if self.is_12_hour() && self.current_section == EditSection::AmPm {
                    if let Some(ch) = event.text.chars().next() {
                        let ch_lower = ch.to_ascii_lowercase();
                        if ch_lower == 'a' {
                            // Set to AM
                            if self.time.hour() >= 12 {
                                if let Some(t) = NaiveTime::from_hms_opt(
                                    self.time.hour() - 12,
                                    self.time.minute(),
                                    self.time.second(),
                                ) {
                                    self.set_time(t);
                                }
                            }
                            return true;
                        } else if ch_lower == 'p' {
                            // Set to PM
                            if self.time.hour() < 12 {
                                if let Some(t) = NaiveTime::from_hms_opt(
                                    self.time.hour() + 12,
                                    self.time.minute(),
                                    self.time.second(),
                                ) {
                                    self.set_time(t);
                                }
                            }
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn handle_focus_out(&mut self) -> bool {
        self.current_section = EditSection::None;
        self.editing_finished.emit(());
        self.base.update();
        false
    }

    fn handle_leave(&mut self) -> bool {
        if self.hover_part != TimeEditPart::None {
            self.hover_part = TimeEditPart::None;
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
        ctx.renderer().fill_rounded_rect(bg_rrect, self.background_color);

        let border_stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rounded_rect(bg_rrect, &border_stroke);
    }

    fn paint_text_field(&self, ctx: &mut PaintContext<'_>) {
        let text_rect = self.text_field_rect();
        let display = self.format_time();

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
            let highlight_rect = Rect::new(
                text_x,
                text_y,
                layout.width(),
                layout.height(),
            );
            ctx.renderer().fill_rect(highlight_rect, self.section_highlight_color);
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

        // Button separator
        let sep_x = rect.width() - self.button_width;
        let sep_stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(sep_x, 0.0),
            Point::new(sep_x, rect.height()),
            &sep_stroke,
        );

        // Horizontal separator between buttons
        let mid_y = rect.height() / 2.0;
        ctx.renderer().draw_line(
            Point::new(sep_x, mid_y),
            Point::new(rect.width(), mid_y),
            &sep_stroke,
        );

        // Up button
        let up_rect = self.up_button_rect();
        let up_color = if self.pressed_part == TimeEditPart::UpButton {
            self.button_pressed_color
        } else if self.hover_part == TimeEditPart::UpButton {
            self.button_hover_color
        } else {
            self.button_color
        };
        ctx.renderer().fill_rect(up_rect, up_color);
        self.paint_arrow(ctx, up_rect, true);

        // Down button
        let down_rect = self.down_button_rect();
        let down_color = if self.pressed_part == TimeEditPart::DownButton {
            self.button_pressed_color
        } else if self.hover_part == TimeEditPart::DownButton {
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

    fn paint_focus_indicator(&self, ctx: &mut PaintContext<'_>) {
        if !self.widget_base().has_focus() {
            return;
        }

        let rect = ctx.rect();
        let focus_color = Color::from_rgba8(66, 133, 244, 180);
        let focus_stroke = Stroke::new(focus_color, 2.0);
        let focus_rrect = RoundedRect::new(rect, self.border_radius);
        ctx.renderer().stroke_rounded_rect(focus_rrect, &focus_stroke);
    }
}

impl Default for TimeEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for TimeEdit {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for TimeEdit {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let text_width = if self.is_12_hour() {
            if self.seconds_shown() { 100.0 } else { 80.0 }
        } else {
            if self.seconds_shown() { 75.0 } else { 55.0 }
        };
        let width = text_width + self.button_width + 16.0;
        let height = 28.0;

        SizeHint::from_dimensions(width.max(80.0), height)
            .with_minimum_dimensions(60.0, 22.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);
        self.paint_text_field(ctx);
        self.paint_buttons(ctx);
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

// Ensure TimeEdit is Send + Sync
static_assertions::assert_impl_all!(TimeEdit: Send, Sync);

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
    fn test_time_edit_creation() {
        setup();
        let time_edit = TimeEdit::new();
        assert_eq!(time_edit.time(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
        assert_eq!(time_edit.display_format(), TimeFormat::Hour24);
        assert!(!time_edit.is_read_only());
    }

    #[test]
    fn test_time_edit_with_time() {
        setup();
        let time = NaiveTime::from_hms_opt(14, 30, 45).unwrap();
        let time_edit = TimeEdit::new().with_time(time);
        assert_eq!(time_edit.time(), time);
    }

    #[test]
    fn test_time_edit_time_range() {
        setup();
        let min = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let max = NaiveTime::from_hms_opt(17, 0, 0).unwrap();
        let time = NaiveTime::from_hms_opt(12, 30, 0).unwrap();

        let time_edit = TimeEdit::new()
            .with_time_range(min, max)
            .with_time(time);

        assert_eq!(time_edit.minimum_time(), min);
        assert_eq!(time_edit.maximum_time(), max);
        assert_eq!(time_edit.time(), time);
    }

    #[test]
    fn test_time_edit_clamping() {
        setup();
        let min = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let max = NaiveTime::from_hms_opt(17, 0, 0).unwrap();
        let before = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        let after = NaiveTime::from_hms_opt(18, 0, 0).unwrap();

        let mut time_edit = TimeEdit::new().with_time_range(min, max);

        time_edit.set_time(before);
        assert_eq!(time_edit.time(), min);

        time_edit.set_time(after);
        assert_eq!(time_edit.time(), max);
    }

    #[test]
    fn test_time_edit_format_24_hour() {
        setup();
        let time = NaiveTime::from_hms_opt(14, 30, 45).unwrap();

        let edit = TimeEdit::new()
            .with_time(time)
            .with_display_format(TimeFormat::Hour24);
        assert_eq!(edit.format_time(), "14:30");

        let edit_sec = TimeEdit::new()
            .with_time(time)
            .with_display_format(TimeFormat::Hour24Seconds);
        assert_eq!(edit_sec.format_time(), "14:30:45");
    }

    #[test]
    fn test_time_edit_format_12_hour() {
        setup();
        let time_pm = NaiveTime::from_hms_opt(14, 30, 45).unwrap();
        let time_am = NaiveTime::from_hms_opt(9, 15, 30).unwrap();

        let edit_pm = TimeEdit::new()
            .with_time(time_pm)
            .with_display_format(TimeFormat::Hour12);
        assert_eq!(edit_pm.format_time(), "02:30 PM");

        let edit_am = TimeEdit::new()
            .with_time(time_am)
            .with_display_format(TimeFormat::Hour12);
        assert_eq!(edit_am.format_time(), "09:15 AM");

        let edit_sec = TimeEdit::new()
            .with_time(time_pm)
            .with_display_format(TimeFormat::Hour12Seconds);
        assert_eq!(edit_sec.format_time(), "02:30:45 PM");
    }

    #[test]
    fn test_time_edit_midnight_noon() {
        setup();
        let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();

        let edit_midnight = TimeEdit::new()
            .with_time(midnight)
            .with_display_format(TimeFormat::Hour12);
        assert_eq!(edit_midnight.format_time(), "12:00 AM");

        let edit_noon = TimeEdit::new()
            .with_time(noon)
            .with_display_format(TimeFormat::Hour12);
        assert_eq!(edit_noon.format_time(), "12:00 PM");
    }

    #[test]
    fn test_time_edit_signal() {
        setup();
        let mut time_edit = TimeEdit::new();
        let signal_fired = Arc::new(AtomicBool::new(false));
        let signal_fired_clone = signal_fired.clone();

        time_edit.time_changed.connect(move |_| {
            signal_fired_clone.store(true, Ordering::SeqCst);
        });

        let new_time = NaiveTime::from_hms_opt(14, 30, 0).unwrap();
        time_edit.set_time(new_time);
        assert!(signal_fired.load(Ordering::SeqCst));
    }

    #[test]
    fn test_time_edit_builder_pattern() {
        setup();
        let min = NaiveTime::from_hms_opt(9, 0, 0).unwrap();
        let max = NaiveTime::from_hms_opt(17, 0, 0).unwrap();
        let time = NaiveTime::from_hms_opt(12, 30, 0).unwrap();

        let time_edit = TimeEdit::new()
            .with_time(time)
            .with_time_range(min, max)
            .with_display_format(TimeFormat::Hour12)
            .with_show_seconds(true)
            .with_read_only(true);

        assert_eq!(time_edit.time(), time);
        assert_eq!(time_edit.display_format(), TimeFormat::Hour12);
        assert!(time_edit.seconds_shown());
        assert!(time_edit.is_read_only());
    }

    #[test]
    fn test_time_format_properties() {
        assert!(!TimeFormat::Hour24.shows_seconds());
        assert!(TimeFormat::Hour24Seconds.shows_seconds());
        assert!(!TimeFormat::Hour12.shows_seconds());
        assert!(TimeFormat::Hour12Seconds.shows_seconds());

        assert!(!TimeFormat::Hour24.is_12_hour());
        assert!(!TimeFormat::Hour24Seconds.is_12_hour());
        assert!(TimeFormat::Hour12.is_12_hour());
        assert!(TimeFormat::Hour12Seconds.is_12_hour());
    }

    #[test]
    fn test_time_edit_size_hint() {
        setup();
        let time_edit = TimeEdit::new();
        let hint = time_edit.size_hint();
        assert!(hint.preferred.width >= 60.0);
        assert!(hint.preferred.height >= 22.0);
    }
}
