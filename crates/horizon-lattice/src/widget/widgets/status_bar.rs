//! StatusBar widget implementation.
//!
//! This module provides [`StatusBar`], a widget that displays status information
//! at the bottom of a window. It supports temporary messages with timeouts,
//! permanent widgets on left and right sides, and an optional size grip for
//! window resizing.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::StatusBar;
//!
//! // Create a status bar
//! let mut status_bar = StatusBar::new();
//!
//! // Show a temporary message (disappears after 3 seconds)
//! status_bar.show_message("Ready", 3000);
//!
//! // Add a permanent widget on the right side
//! status_bar.add_permanent_widget(progress_indicator.object_id());
//!
//! // Enable size grip for window resizing
//! status_bar.set_size_grip_enabled(true);
//! ```

use std::time::Duration;

use horizon_lattice_core::{Object, ObjectId, Signal, TimerId};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, Size, Stroke, TextLayout,
    TextRenderer,
};

use crate::widget::geometry::SizePolicy;
use crate::widget::{
    FocusPolicy, MouseButton, MouseMoveEvent, MousePressEvent, MouseReleaseEvent, PaintContext,
    SizeHint, Widget, WidgetBase, WidgetEvent,
};

// ============================================================================
// StatusBarStyle
// ============================================================================

/// Style configuration for status bar appearance.
#[derive(Clone)]
pub struct StatusBarStyle {
    /// Background color.
    pub background_color: Color,
    /// Text color for messages.
    pub text_color: Color,
    /// Border color (top border).
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Height of the status bar.
    pub height: f32,
    /// Horizontal padding.
    pub padding: f32,
    /// Spacing between sections.
    pub spacing: f32,
    /// Font for message text.
    pub font: Font,
    /// Size grip color.
    pub size_grip_color: Color,
    /// Size grip hover color.
    pub size_grip_hover_color: Color,
    /// Size grip size (width and height).
    pub size_grip_size: f32,
}

impl Default for StatusBarStyle {
    fn default() -> Self {
        Self {
            background_color: Color::from_rgb8(240, 240, 240),
            text_color: Color::from_rgb8(50, 50, 50),
            border_color: Color::from_rgb8(200, 200, 200),
            border_width: 1.0,
            height: 22.0,
            padding: 6.0,
            spacing: 8.0,
            font: Font::new(FontFamily::SansSerif, 12.0),
            size_grip_color: Color::from_rgb8(160, 160, 160),
            size_grip_hover_color: Color::from_rgb8(120, 120, 120),
            size_grip_size: 14.0,
        }
    }
}

// ============================================================================
// StatusBarItem
// ============================================================================

/// An item in the status bar's permanent widget section.
#[derive(Clone)]
struct StatusBarItem {
    /// The widget's object ID.
    widget_id: ObjectId,
    /// Stretch factor (0 = fixed size, >0 = takes extra space proportionally).
    stretch: i32,
    /// Calculated rectangle during layout.
    rect: Rect,
}

impl StatusBarItem {
    fn new(widget_id: ObjectId, stretch: i32) -> Self {
        Self {
            widget_id,
            stretch,
            rect: Rect::ZERO,
        }
    }
}

// ============================================================================
// MessagePriority
// ============================================================================

/// Priority level for status bar messages.
///
/// Higher priority messages take precedence over lower priority ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum MessagePriority {
    /// Normal priority (default).
    #[default]
    Normal,
    /// High priority messages override normal messages.
    High,
    /// Critical priority messages override all other messages.
    Critical,
}

// ============================================================================
// StatusBar
// ============================================================================

/// A status bar widget for displaying messages and status information.
///
/// StatusBar is typically placed at the bottom of a main window. It provides:
///
/// - **Temporary messages**: Text messages that can automatically disappear
///   after a timeout. Messages can have priorities, with higher priority
///   messages overriding lower priority ones.
///
/// - **Permanent widgets**: Custom widgets that remain visible, such as
///   progress bars, labels showing line/column numbers, etc. Widgets can
///   be added to the left or right side.
///
/// - **Size grip**: An optional resize handle in the corner for resizing
///   the parent window.
///
/// # Layout
///
/// The status bar has a horizontal layout:
/// ```text
/// [Left Widgets] [Message Area] [Right Widgets] [Size Grip]
/// ```
///
/// The message area expands to fill available space between the left and
/// right widget sections.
///
/// # Signals
///
/// - [`message_changed`](StatusBar::message_changed): Emitted when the current message changes
pub struct StatusBar {
    /// Widget base.
    base: WidgetBase,

    /// Current temporary message.
    current_message: String,

    /// Priority of current message.
    current_priority: MessagePriority,

    /// Timer ID for message timeout.
    message_timer_id: Option<TimerId>,

    /// Permanent widgets on the left side.
    left_widgets: Vec<StatusBarItem>,

    /// Permanent widgets on the right side.
    right_widgets: Vec<StatusBarItem>,

    /// Whether the size grip is enabled.
    size_grip_enabled: bool,

    /// Whether the size grip is hovered.
    size_grip_hovered: bool,

    /// Whether the size grip is being dragged.
    size_grip_pressed: bool,

    /// Visual style.
    style: StatusBarStyle,

    /// Signal emitted when the message changes.
    pub message_changed: Signal<String>,
}

impl StatusBar {
    /// Create a new status bar.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();

        // Status bar has fixed height, expanding width
        base.set_horizontal_policy(SizePolicy::Expanding);
        base.set_vertical_policy(SizePolicy::Fixed);

        // Status bar doesn't take focus
        base.set_focus_policy(FocusPolicy::NoFocus);

        Self {
            base,
            current_message: String::new(),
            current_priority: MessagePriority::Normal,
            message_timer_id: None,
            left_widgets: Vec::new(),
            right_widgets: Vec::new(),
            size_grip_enabled: true,
            size_grip_hovered: false,
            size_grip_pressed: false,
            style: StatusBarStyle::default(),
            message_changed: Signal::new(),
        }
    }

    /// Set the style using builder pattern.
    pub fn with_style(mut self, style: StatusBarStyle) -> Self {
        self.style = style;
        self
    }

    /// Set size grip enabled using builder pattern.
    pub fn with_size_grip(mut self, enabled: bool) -> Self {
        self.size_grip_enabled = enabled;
        self
    }

    // =========================================================================
    // Style
    // =========================================================================

    /// Get the current style.
    pub fn style(&self) -> &StatusBarStyle {
        &self.style
    }

    /// Set the style.
    pub fn set_style(&mut self, style: StatusBarStyle) {
        self.style = style;
        self.base.update();
    }

    // =========================================================================
    // Message Display
    // =========================================================================

    /// Get the current message text.
    pub fn message(&self) -> &str {
        &self.current_message
    }

    /// Show a temporary message.
    ///
    /// The message will be displayed immediately. If `timeout_ms` is greater
    /// than 0, the message will automatically be cleared after the specified
    /// duration in milliseconds. If `timeout_ms` is 0, the message remains
    /// until explicitly cleared or replaced.
    ///
    /// # Arguments
    ///
    /// * `text` - The message text to display
    /// * `timeout_ms` - Timeout in milliseconds (0 = no timeout)
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Show message for 3 seconds
    /// status_bar.show_message("File saved successfully", 3000);
    ///
    /// // Show persistent message (until cleared)
    /// status_bar.show_message("Ready", 0);
    /// ```
    pub fn show_message(&mut self, text: impl Into<String>, timeout_ms: u64) {
        self.show_message_with_priority(text, timeout_ms, MessagePriority::Normal);
    }

    /// Show a temporary message with a specific priority.
    ///
    /// Higher priority messages take precedence over lower priority ones.
    /// A lower priority message will not replace a higher priority message.
    ///
    /// # Arguments
    ///
    /// * `text` - The message text to display
    /// * `timeout_ms` - Timeout in milliseconds (0 = no timeout)
    /// * `priority` - Message priority level
    pub fn show_message_with_priority(
        &mut self,
        text: impl Into<String>,
        timeout_ms: u64,
        priority: MessagePriority,
    ) {
        // Don't replace higher priority messages with lower priority ones
        if !self.current_message.is_empty() && priority < self.current_priority {
            return;
        }

        // Cancel any existing timer
        if let Some(timer_id) = self.message_timer_id.take() {
            self.base.stop_timer(timer_id);
        }

        let text = text.into();
        self.current_message = text.clone();
        self.current_priority = priority;

        // Start timeout timer if requested
        if timeout_ms > 0 {
            let timer_id = self.base.start_timer(Duration::from_millis(timeout_ms));
            self.message_timer_id = Some(timer_id);
        }

        self.message_changed.emit(text);
        self.base.update();
    }

    /// Clear the current message.
    ///
    /// This removes the temporary message from display. Permanent widgets
    /// remain visible.
    pub fn clear_message(&mut self) {
        if !self.current_message.is_empty() {
            // Cancel any existing timer
            if let Some(timer_id) = self.message_timer_id.take() {
                self.base.stop_timer(timer_id);
            }

            self.current_message.clear();
            self.current_priority = MessagePriority::Normal;
            self.message_changed.emit(String::new());
            self.base.update();
        }
    }

    // =========================================================================
    // Permanent Widgets
    // =========================================================================

    /// Add a permanent widget to the right side of the status bar.
    ///
    /// Permanent widgets remain visible and are not affected by temporary
    /// messages. The widget is added at the end of the right widget section.
    ///
    /// # Arguments
    ///
    /// * `widget_id` - The object ID of the widget to add
    ///
    /// # Example
    ///
    /// ```ignore
    /// let progress = ProgressBar::new();
    /// status_bar.add_permanent_widget(progress.object_id());
    /// ```
    pub fn add_permanent_widget(&mut self, widget_id: ObjectId) {
        self.add_permanent_widget_with_stretch(widget_id, 0);
    }

    /// Add a permanent widget with a stretch factor.
    ///
    /// # Arguments
    ///
    /// * `widget_id` - The object ID of the widget to add
    /// * `stretch` - Stretch factor (0 = fixed size, >0 = takes extra space)
    pub fn add_permanent_widget_with_stretch(&mut self, widget_id: ObjectId, stretch: i32) {
        self.right_widgets
            .push(StatusBarItem::new(widget_id, stretch));
        self.base.update();
    }

    /// Insert a permanent widget on the left side of the status bar.
    ///
    /// Widgets on the left side are displayed before the message area.
    ///
    /// # Arguments
    ///
    /// * `widget_id` - The object ID of the widget to insert
    pub fn insert_permanent_widget(&mut self, widget_id: ObjectId) {
        self.insert_permanent_widget_with_stretch(widget_id, 0);
    }

    /// Insert a permanent widget on the left side with a stretch factor.
    ///
    /// # Arguments
    ///
    /// * `widget_id` - The object ID of the widget to insert
    /// * `stretch` - Stretch factor (0 = fixed size, >0 = takes extra space)
    pub fn insert_permanent_widget_with_stretch(&mut self, widget_id: ObjectId, stretch: i32) {
        self.left_widgets
            .push(StatusBarItem::new(widget_id, stretch));
        self.base.update();
    }

    /// Remove a permanent widget from the status bar.
    ///
    /// Removes the widget from either the left or right section, wherever it
    /// is found.
    ///
    /// # Returns
    ///
    /// `true` if the widget was found and removed, `false` otherwise.
    pub fn remove_permanent_widget(&mut self, widget_id: ObjectId) -> bool {
        let left_removed = self
            .left_widgets
            .iter()
            .position(|w| w.widget_id == widget_id);
        if let Some(index) = left_removed {
            self.left_widgets.remove(index);
            self.base.update();
            return true;
        }

        let right_removed = self
            .right_widgets
            .iter()
            .position(|w| w.widget_id == widget_id);
        if let Some(index) = right_removed {
            self.right_widgets.remove(index);
            self.base.update();
            return true;
        }

        false
    }

    /// Get the object IDs of all permanent widgets on the left side.
    pub fn left_permanent_widgets(&self) -> Vec<ObjectId> {
        self.left_widgets.iter().map(|w| w.widget_id).collect()
    }

    /// Get the object IDs of all permanent widgets on the right side.
    pub fn right_permanent_widgets(&self) -> Vec<ObjectId> {
        self.right_widgets.iter().map(|w| w.widget_id).collect()
    }

    /// Get the number of permanent widgets.
    pub fn permanent_widget_count(&self) -> usize {
        self.left_widgets.len() + self.right_widgets.len()
    }

    // =========================================================================
    // Size Grip
    // =========================================================================

    /// Check if the size grip is enabled.
    pub fn is_size_grip_enabled(&self) -> bool {
        self.size_grip_enabled
    }

    /// Set whether the size grip is enabled.
    ///
    /// When enabled, a resize handle is shown in the bottom-right corner
    /// of the status bar, allowing the user to resize the parent window.
    pub fn set_size_grip_enabled(&mut self, enabled: bool) {
        if self.size_grip_enabled != enabled {
            self.size_grip_enabled = enabled;
            self.base.update();
        }
    }

    // =========================================================================
    // Internal Layout
    // =========================================================================

    /// Get the rectangle for the size grip.
    fn size_grip_rect(&self) -> Rect {
        if !self.size_grip_enabled {
            return Rect::ZERO;
        }

        let widget_rect = self.base.rect();
        let size = self.style.size_grip_size;
        Rect::new(
            widget_rect.width() - size,
            widget_rect.height() - size,
            size,
            size,
        )
    }

    /// Get the rectangle for the message area.
    fn message_rect(&self) -> Rect {
        let widget_rect = self.base.rect();
        let padding = self.style.padding;
        let spacing = self.style.spacing;

        // Calculate left widgets width (placeholder - actual layout would measure widgets)
        let left_width: f32 = self.left_widgets.len() as f32 * 60.0;

        // Calculate right widgets width + size grip
        let right_width: f32 = self.right_widgets.len() as f32 * 60.0;
        let size_grip_width = if self.size_grip_enabled {
            self.style.size_grip_size + spacing
        } else {
            0.0
        };

        let x = padding + left_width + if left_width > 0.0 { spacing } else { 0.0 };
        let available_width = widget_rect.width()
            - padding * 2.0
            - left_width
            - right_width
            - size_grip_width
            - if left_width > 0.0 { spacing } else { 0.0 }
            - if right_width > 0.0 { spacing } else { 0.0 };

        Rect::new(
            x,
            self.style.border_width,
            available_width.max(0.0),
            widget_rect.height() - self.style.border_width,
        )
    }

    // =========================================================================
    // Mouse Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Check size grip
        if self.size_grip_enabled && self.size_grip_rect().contains(event.local_pos) {
            self.size_grip_pressed = true;
            // In a real implementation, this would initiate window resizing
            // through the window system integration
            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.size_grip_pressed {
            self.size_grip_pressed = false;
            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let was_hovered = self.size_grip_hovered;

        // Update size grip hover state
        self.size_grip_hovered =
            self.size_grip_enabled && self.size_grip_rect().contains(event.local_pos);

        if was_hovered != self.size_grip_hovered {
            self.base.update();
            return true;
        }

        false
    }

    // =========================================================================
    // Painting Helpers
    // =========================================================================

    /// Paint the size grip triangular pattern.
    fn paint_size_grip(&self, ctx: &mut PaintContext<'_>) {
        if !self.size_grip_enabled {
            return;
        }

        let rect = self.size_grip_rect();
        let color = if self.size_grip_hovered || self.size_grip_pressed {
            self.style.size_grip_hover_color
        } else {
            self.style.size_grip_color
        };

        let stroke = Stroke::new(color, 1.0);
        let renderer = ctx.renderer();

        // Draw diagonal grip lines (classic Windows/macOS style)
        let line_spacing = 3.0;
        let num_lines = 3;

        for i in 0..num_lines {
            let offset = line_spacing * (i as f32 + 1.0);
            let x1 = rect.origin.x + rect.width() - offset;
            let y1 = rect.origin.y + rect.height();
            let x2 = rect.origin.x + rect.width();
            let y2 = rect.origin.y + rect.height() - offset;

            renderer.draw_line(Point::new(x1, y1), Point::new(x2, y2), &stroke);
        }
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for StatusBar {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for StatusBar {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Status bar has fixed height based on style
        let height = self.style.height + self.style.border_width;
        let preferred = Size::new(200.0, height);
        let minimum = Size::new(100.0, height);
        SizeHint::new(preferred).with_minimum(minimum)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        let renderer = ctx.renderer();

        // Paint background
        renderer.fill_rect(rect, self.style.background_color);

        // Paint top border
        if self.style.border_width > 0.0 {
            let border_stroke = Stroke::new(self.style.border_color, self.style.border_width);
            let y = rect.origin.y + self.style.border_width / 2.0;
            renderer.draw_line(
                Point::new(rect.origin.x, y),
                Point::new(rect.origin.x + rect.width(), y),
                &border_stroke,
            );
        }

        // Paint message text if present
        if !self.current_message.is_empty() {
            let message_rect = self.message_rect();

            // Create text layout
            let mut font_system = FontSystem::new();
            let layout = TextLayout::new(&mut font_system, &self.current_message, &self.style.font);

            // Calculate vertical centering
            let text_y =
                message_rect.origin.y + (message_rect.height() - self.style.font.size()) / 2.0;

            let position = Point::new(
                rect.origin.x + message_rect.origin.x,
                rect.origin.y + text_y,
            );

            // Prepare text for rendering
            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    position,
                    self.style.text_color,
                );
                // Note: Actual glyph rendering requires integration with the
                // application's render pass system.
            }
        }

        // Paint size grip
        self.paint_size_grip(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::Leave(_) => {
                if self.size_grip_hovered {
                    self.size_grip_hovered = false;
                    self.base.update();
                }
                false
            }
            WidgetEvent::Timer(timer_event) => {
                // Check if this is our message timeout timer
                if let Some(timer_id) = self.message_timer_id
                    && timer_event.id == timer_id {
                        self.clear_message();
                        return true;
                    }
                false
            }
            _ => false,
        }
    }
}

// Ensure StatusBar is Send + Sync
static_assertions::assert_impl_all!(StatusBar: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::Widget;
    use crate::widget::widgets::Separator;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_status_bar_creation() {
        setup();
        let status_bar = StatusBar::new();
        assert!(status_bar.message().is_empty());
        assert!(status_bar.is_size_grip_enabled());
        assert_eq!(status_bar.permanent_widget_count(), 0);
    }

    #[test]
    fn test_show_message() {
        setup();
        let mut status_bar = StatusBar::new();
        status_bar.show_message("Test message", 0);
        assert_eq!(status_bar.message(), "Test message");
    }

    #[test]
    fn test_clear_message() {
        setup();
        let mut status_bar = StatusBar::new();
        status_bar.show_message("Test message", 0);
        status_bar.clear_message();
        assert!(status_bar.message().is_empty());
    }

    #[test]
    fn test_message_priority() {
        setup();
        let mut status_bar = StatusBar::new();

        // Show high priority message
        status_bar.show_message_with_priority("High priority", 0, MessagePriority::High);
        assert_eq!(status_bar.message(), "High priority");

        // Normal priority should not replace high priority
        status_bar.show_message_with_priority("Normal priority", 0, MessagePriority::Normal);
        assert_eq!(status_bar.message(), "High priority");

        // Critical priority should replace high priority
        status_bar.show_message_with_priority("Critical", 0, MessagePriority::Critical);
        assert_eq!(status_bar.message(), "Critical");
    }

    #[test]
    fn test_permanent_widgets() {
        setup();
        let mut status_bar = StatusBar::new();

        // Create real widgets to get valid ObjectIds
        let widget_1 = Separator::horizontal();
        let widget_2 = Separator::vertical();
        let widget_id_1 = widget_1.object_id();
        let widget_id_2 = widget_2.object_id();

        // Add to right
        status_bar.add_permanent_widget(widget_id_1);
        assert_eq!(status_bar.right_permanent_widgets().len(), 1);
        assert_eq!(status_bar.left_permanent_widgets().len(), 0);

        // Insert to left
        status_bar.insert_permanent_widget(widget_id_2);
        assert_eq!(status_bar.right_permanent_widgets().len(), 1);
        assert_eq!(status_bar.left_permanent_widgets().len(), 1);

        assert_eq!(status_bar.permanent_widget_count(), 2);

        // Remove
        assert!(status_bar.remove_permanent_widget(widget_id_1));
        assert_eq!(status_bar.permanent_widget_count(), 1);

        // Try to remove non-existent
        assert!(!status_bar.remove_permanent_widget(widget_id_1));
    }

    #[test]
    fn test_size_grip() {
        setup();
        let mut status_bar = StatusBar::new();

        assert!(status_bar.is_size_grip_enabled());

        status_bar.set_size_grip_enabled(false);
        assert!(!status_bar.is_size_grip_enabled());

        status_bar.set_size_grip_enabled(true);
        assert!(status_bar.is_size_grip_enabled());
    }

    #[test]
    fn test_size_hint() {
        setup();
        let status_bar = StatusBar::new();
        let hint = status_bar.size_hint();

        // Should have fixed height
        let default_style = StatusBarStyle::default();
        let expected_height = default_style.height + default_style.border_width;
        assert_eq!(hint.effective_minimum().height, expected_height);
        assert_eq!(hint.preferred.height, expected_height);
    }

    #[test]
    fn test_builder_pattern() {
        setup();
        let mut style = StatusBarStyle::default();
        style.height = 30.0;

        let status_bar = StatusBar::new().with_style(style).with_size_grip(false);

        assert_eq!(status_bar.style().height, 30.0);
        assert!(!status_bar.is_size_grip_enabled());
    }

    #[test]
    fn test_size_policy() {
        setup();
        let status_bar = StatusBar::new();
        let policy = status_bar.widget_base().size_policy();

        assert_eq!(policy.horizontal, SizePolicy::Expanding);
        assert_eq!(policy.vertical, SizePolicy::Fixed);
    }
}
