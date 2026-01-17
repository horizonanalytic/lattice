//! MessageBox dialog implementation.
//!
//! This module provides [`MessageBox`], a modal dialog for displaying messages to the user
//! with standardized icons and button configurations.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{MessageBox, MessageIcon, StandardButton};
//!
//! // Using static helper
//! let msg = MessageBox::information("Success", "The file was saved successfully.");
//!
//! // Using builder pattern
//! let mut msg = MessageBox::new()
//!     .with_icon(MessageIcon::Warning)
//!     .with_title("Confirm Delete")
//!     .with_text("Are you sure you want to delete this file?")
//!     .with_informative_text("This action cannot be undone.")
//!     .with_standard_buttons(StandardButton::YES | StandardButton::NO);
//!
//! msg.finished.connect(|result| {
//!     println!("User responded: {:?}", result);
//! });
//!
//! msg.open();
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, RoundedRect, Size, Stroke};

use crate::widget::{
    Key, KeyPressEvent, MouseButton,
    MouseMoveEvent, MousePressEvent, MouseReleaseEvent, PaintContext, SizeHint,
    Widget, WidgetBase, WidgetEvent,
};

use super::dialog::{Dialog, DialogResult};
use super::dialog_button_box::{ButtonRole, StandardButton};
use super::native_dialogs::{self, NativeMessageButtons, NativeMessageLevel, NativeMessageOptions, NativeMessageResult};

// ============================================================================
// Message Icon
// ============================================================================

/// The icon to display in a message box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MessageIcon {
    /// No icon is displayed.
    #[default]
    NoIcon,

    /// An information icon (typically "i" in a circle).
    /// Used for reporting information about normal operations.
    Information,

    /// A warning icon (typically "!" in a triangle).
    /// Used for reporting non-critical errors or potential issues.
    Warning,

    /// A critical error icon (typically "X" in a circle).
    /// Used for reporting critical errors.
    Critical,

    /// A question icon (typically "?" in a circle).
    /// Used when asking the user a question during normal operations.
    Question,
}

impl MessageIcon {
    /// Get the primary color for this icon type.
    pub fn color(&self) -> Color {
        match self {
            MessageIcon::NoIcon => Color::TRANSPARENT,
            MessageIcon::Information => Color::from_rgb8(0, 120, 215),    // Blue
            MessageIcon::Warning => Color::from_rgb8(255, 185, 0),        // Orange/Yellow
            MessageIcon::Critical => Color::from_rgb8(232, 17, 35),       // Red
            MessageIcon::Question => Color::from_rgb8(0, 120, 215),       // Blue
        }
    }

    /// Check if this icon type should be displayed.
    pub fn is_visible(&self) -> bool {
        !matches!(self, MessageIcon::NoIcon)
    }
}

// ============================================================================
// Detail Button State
// ============================================================================

/// State of the "Show Details" button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct DetailButtonState {
    hovered: bool,
    pressed: bool,
    expanded: bool,
}

// ============================================================================
// Custom Button Info
// ============================================================================

/// Information about a custom button added to the message box.
#[derive(Debug, Clone)]
pub struct CustomButtonInfo {
    /// The button's unique ID.
    pub id: u32,
    /// The button text.
    pub text: String,
    /// The button's role.
    pub role: ButtonRole,
}

// ============================================================================
// MessageBox
// ============================================================================

/// A modal dialog for displaying messages to the user.
///
/// MessageBox provides a standardized way to show information, warnings, errors,
/// and questions to the user. It supports:
///
/// - Standard icons (information, warning, critical, question)
/// - Primary message text
/// - Informative text (secondary description)
/// - Detailed text (expandable section)
/// - Standard and custom buttons
///
/// # Static Helpers
///
/// For common use cases, use the static helper methods:
///
/// - [`MessageBox::information()`]: Display an informational message
/// - [`MessageBox::warning()`]: Display a warning message
/// - [`MessageBox::critical()`]: Display a critical error message
/// - [`MessageBox::question()`]: Ask the user a question
pub struct MessageBox {
    /// The underlying dialog.
    dialog: Dialog,

    /// The icon to display.
    icon: MessageIcon,

    /// The primary message text.
    text: String,

    /// Additional informative text (shown below the main text).
    informative_text: String,

    /// Detailed text (shown in an expandable section).
    detailed_text: String,

    /// Custom buttons added to the message box.
    custom_buttons: Vec<CustomButtonInfo>,

    /// Next custom button ID.
    next_button_id: u32,

    /// The button that was clicked to close the dialog.
    clicked_button: Option<StandardButton>,

    /// The custom button ID that was clicked (if any).
    clicked_custom_button: Option<u32>,

    /// Default button to activate on Enter.
    default_button: StandardButton,

    /// Escape button to activate on Escape.
    escape_button: StandardButton,

    // Visual styling
    /// Icon size.
    icon_size: f32,
    /// Spacing between icon and text.
    icon_text_spacing: f32,
    /// Content padding.
    content_padding: f32,
    /// Text color.
    text_color: Color,
    /// Informative text color (typically lighter).
    informative_text_color: Color,
    /// Detailed text background color.
    detailed_text_background: Color,
    /// Button row height.
    button_row_height: f32,

    // Detail button state
    detail_button_state: DetailButtonState,
    /// Detail section height when expanded.
    detail_section_height: f32,

    /// Whether to prefer native dialogs when available.
    use_native_dialog: bool,

    // Signals
    /// Signal emitted when a button is clicked.
    /// The argument is the StandardButton that was clicked.
    pub button_clicked: Signal<StandardButton>,

    /// Signal emitted when a custom button is clicked.
    /// The argument is the custom button ID.
    pub custom_button_clicked: Signal<u32>,
}

impl MessageBox {
    /// Create a new message box with default settings.
    pub fn new() -> Self {
        let dialog = Dialog::new("Message")
            .with_size(400.0, 160.0)
            .with_standard_buttons(StandardButton::OK);

        Self {
            dialog,
            icon: MessageIcon::NoIcon,
            text: String::new(),
            informative_text: String::new(),
            detailed_text: String::new(),
            custom_buttons: Vec::new(),
            next_button_id: 1000,
            clicked_button: None,
            clicked_custom_button: None,
            default_button: StandardButton::NONE,
            escape_button: StandardButton::NONE,
            icon_size: 32.0,
            icon_text_spacing: 16.0,
            content_padding: 16.0,
            text_color: Color::from_rgb8(32, 32, 32),
            informative_text_color: Color::from_rgb8(96, 96, 96),
            detailed_text_background: Color::from_rgb8(248, 248, 248),
            button_row_height: 48.0,
            detail_button_state: DetailButtonState::default(),
            detail_section_height: 100.0,
            use_native_dialog: false,
            button_clicked: Signal::new(),
            custom_button_clicked: Signal::new(),
        }
    }

    // =========================================================================
    // Static Helper Methods
    // =========================================================================

    /// Create an information message box.
    ///
    /// # Arguments
    ///
    /// * `title` - The window title
    /// * `text` - The message text
    ///
    /// # Example
    ///
    /// ```ignore
    /// let msg = MessageBox::information("Success", "Operation completed successfully.");
    /// msg.open();
    /// ```
    pub fn information(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new()
            .with_icon(MessageIcon::Information)
            .with_title(title)
            .with_text(text)
            .with_standard_buttons(StandardButton::OK)
    }

    /// Create an information message box with custom buttons.
    pub fn information_with_buttons(
        title: impl Into<String>,
        text: impl Into<String>,
        buttons: StandardButton,
    ) -> Self {
        Self::new()
            .with_icon(MessageIcon::Information)
            .with_title(title)
            .with_text(text)
            .with_standard_buttons(buttons)
    }

    /// Create a warning message box.
    ///
    /// # Arguments
    ///
    /// * `title` - The window title
    /// * `text` - The warning message text
    ///
    /// # Example
    ///
    /// ```ignore
    /// let msg = MessageBox::warning("Warning", "This action may have unintended consequences.");
    /// msg.open();
    /// ```
    pub fn warning(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new()
            .with_icon(MessageIcon::Warning)
            .with_title(title)
            .with_text(text)
            .with_standard_buttons(StandardButton::OK)
    }

    /// Create a warning message box with custom buttons.
    pub fn warning_with_buttons(
        title: impl Into<String>,
        text: impl Into<String>,
        buttons: StandardButton,
    ) -> Self {
        Self::new()
            .with_icon(MessageIcon::Warning)
            .with_title(title)
            .with_text(text)
            .with_standard_buttons(buttons)
    }

    /// Create a critical error message box.
    ///
    /// # Arguments
    ///
    /// * `title` - The window title
    /// * `text` - The error message text
    ///
    /// # Example
    ///
    /// ```ignore
    /// let msg = MessageBox::critical("Error", "An unexpected error occurred.");
    /// msg.open();
    /// ```
    pub fn critical(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new()
            .with_icon(MessageIcon::Critical)
            .with_title(title)
            .with_text(text)
            .with_standard_buttons(StandardButton::OK)
    }

    /// Create a critical error message box with custom buttons.
    pub fn critical_with_buttons(
        title: impl Into<String>,
        text: impl Into<String>,
        buttons: StandardButton,
    ) -> Self {
        Self::new()
            .with_icon(MessageIcon::Critical)
            .with_title(title)
            .with_text(text)
            .with_standard_buttons(buttons)
    }

    /// Create a question message box.
    ///
    /// # Arguments
    ///
    /// * `title` - The window title
    /// * `text` - The question text
    ///
    /// # Example
    ///
    /// ```ignore
    /// let msg = MessageBox::question("Confirm", "Do you want to save changes?");
    /// msg.open();
    /// ```
    pub fn question(title: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new()
            .with_icon(MessageIcon::Question)
            .with_title(title)
            .with_text(text)
            .with_standard_buttons(StandardButton::YES | StandardButton::NO)
    }

    /// Create a question message box with custom buttons.
    pub fn question_with_buttons(
        title: impl Into<String>,
        text: impl Into<String>,
        buttons: StandardButton,
    ) -> Self {
        Self::new()
            .with_icon(MessageIcon::Question)
            .with_title(title)
            .with_text(text)
            .with_standard_buttons(buttons)
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the icon using builder pattern.
    pub fn with_icon(mut self, icon: MessageIcon) -> Self {
        self.icon = icon;
        self
    }

    /// Set the title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.dialog.set_title(title);
        self
    }

    /// Set the primary message text using builder pattern.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self.update_size();
        self
    }

    /// Set the informative text using builder pattern.
    pub fn with_informative_text(mut self, text: impl Into<String>) -> Self {
        self.informative_text = text.into();
        self.update_size();
        self
    }

    /// Set the detailed text using builder pattern.
    pub fn with_detailed_text(mut self, text: impl Into<String>) -> Self {
        self.detailed_text = text.into();
        self.update_size();
        self
    }

    /// Set the standard buttons using builder pattern.
    pub fn with_standard_buttons(mut self, buttons: StandardButton) -> Self {
        self.dialog.set_standard_buttons(buttons);
        // Set sensible defaults for default and escape buttons
        if buttons.has(StandardButton::OK) {
            self.default_button = StandardButton::OK;
        } else if buttons.has(StandardButton::YES) {
            self.default_button = StandardButton::YES;
        } else if buttons.has(StandardButton::SAVE) {
            self.default_button = StandardButton::SAVE;
        }

        if buttons.has(StandardButton::CANCEL) {
            self.escape_button = StandardButton::CANCEL;
        } else if buttons.has(StandardButton::NO) {
            self.escape_button = StandardButton::NO;
        } else if buttons.has(StandardButton::CLOSE) {
            self.escape_button = StandardButton::CLOSE;
        }
        self
    }

    /// Set the default button using builder pattern.
    pub fn with_default_button(mut self, button: StandardButton) -> Self {
        self.default_button = button;
        self
    }

    /// Set the escape button using builder pattern.
    pub fn with_escape_button(mut self, button: StandardButton) -> Self {
        self.escape_button = button;
        self
    }

    /// Set the dialog size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.dialog = std::mem::take(&mut self.dialog).with_size(width, height);
        self
    }

    /// Set whether to prefer native dialogs using builder pattern.
    ///
    /// When enabled, the message box will use native system dialogs
    /// (NSAlert on macOS, TaskDialog on Windows) if available.
    pub fn with_native_dialog(mut self, use_native: bool) -> Self {
        self.use_native_dialog = use_native;
        self
    }

    // =========================================================================
    // Properties
    // =========================================================================

    /// Get the icon.
    pub fn icon(&self) -> MessageIcon {
        self.icon
    }

    /// Set the icon.
    pub fn set_icon(&mut self, icon: MessageIcon) {
        if self.icon != icon {
            self.icon = icon;
            self.dialog.widget_base_mut().update();
        }
    }

    /// Get the title.
    pub fn title(&self) -> &str {
        self.dialog.title()
    }

    /// Set the title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.dialog.set_title(title);
    }

    /// Get the primary message text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Set the primary message text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.update_size();
        self.dialog.widget_base_mut().update();
    }

    /// Get the informative text.
    pub fn informative_text(&self) -> &str {
        &self.informative_text
    }

    /// Set the informative text.
    pub fn set_informative_text(&mut self, text: impl Into<String>) {
        self.informative_text = text.into();
        self.update_size();
        self.dialog.widget_base_mut().update();
    }

    /// Get the detailed text.
    pub fn detailed_text(&self) -> &str {
        &self.detailed_text
    }

    /// Set the detailed text.
    pub fn set_detailed_text(&mut self, text: impl Into<String>) {
        self.detailed_text = text.into();
        self.update_size();
        self.dialog.widget_base_mut().update();
    }

    /// Get the standard buttons.
    pub fn standard_buttons(&self) -> StandardButton {
        self.dialog.standard_buttons()
    }

    /// Set the standard buttons.
    pub fn set_standard_buttons(&mut self, buttons: StandardButton) {
        self.dialog.set_standard_buttons(buttons);
    }

    /// Get the default button.
    pub fn default_button(&self) -> StandardButton {
        self.default_button
    }

    /// Set the default button.
    pub fn set_default_button(&mut self, button: StandardButton) {
        self.default_button = button;
    }

    /// Get the escape button.
    pub fn escape_button(&self) -> StandardButton {
        self.escape_button
    }

    /// Set the escape button.
    pub fn set_escape_button(&mut self, button: StandardButton) {
        self.escape_button = button;
    }

    /// Get the button that was clicked to close the dialog.
    pub fn clicked_button(&self) -> Option<StandardButton> {
        self.clicked_button
    }

    /// Get the custom button ID that was clicked (if any).
    pub fn clicked_custom_button(&self) -> Option<u32> {
        self.clicked_custom_button
    }

    /// Get the dialog result.
    pub fn result(&self) -> DialogResult {
        self.dialog.result()
    }

    /// Check if the dialog is currently open.
    pub fn is_open(&self) -> bool {
        self.dialog.is_open()
    }

    // =========================================================================
    // Custom Buttons
    // =========================================================================

    /// Add a custom button with the specified text and role.
    ///
    /// Returns the button ID which can be used to identify which button was clicked.
    pub fn add_button(&mut self, text: impl Into<String>, role: ButtonRole) -> u32 {
        let id = self.next_button_id;
        self.next_button_id += 1;

        self.custom_buttons.push(CustomButtonInfo {
            id,
            text: text.into(),
            role,
        });

        id
    }

    /// Remove a custom button by ID.
    pub fn remove_button(&mut self, id: u32) {
        self.custom_buttons.retain(|b| b.id != id);
    }

    /// Get all custom buttons.
    pub fn custom_buttons(&self) -> &[CustomButtonInfo] {
        &self.custom_buttons
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the message box (non-blocking modal).
    ///
    /// Shows the message box as a modal and returns immediately. Connect to
    /// the `button_clicked` or dialog signals to handle the result.
    ///
    /// If `use_native_dialog` is enabled and native dialogs are available,
    /// a native system alert will be shown instead.
    pub fn open(&mut self) {
        self.clicked_button = None;
        self.clicked_custom_button = None;

        // Try native dialog if preferred and available
        if self.use_native_dialog && native_dialogs::is_available() {
            // Only use native dialog for simple standard button configurations
            // Custom buttons require the custom implementation
            if self.custom_buttons.is_empty() {
                let buttons = self.dialog.standard_buttons();

                // Convert MessageIcon to NativeMessageLevel
                let level = match self.icon {
                    MessageIcon::NoIcon | MessageIcon::Question | MessageIcon::Information => {
                        NativeMessageLevel::Info
                    }
                    MessageIcon::Warning => NativeMessageLevel::Warning,
                    MessageIcon::Critical => NativeMessageLevel::Error,
                };

                // Convert StandardButton to NativeMessageButtons
                let native_buttons = if buttons.has(StandardButton::YES)
                    && buttons.has(StandardButton::NO)
                    && buttons.has(StandardButton::CANCEL)
                {
                    NativeMessageButtons::YesNoCancel
                } else if buttons.has(StandardButton::YES) && buttons.has(StandardButton::NO) {
                    NativeMessageButtons::YesNo
                } else if buttons.has(StandardButton::OK) && buttons.has(StandardButton::CANCEL)
                {
                    NativeMessageButtons::OkCancel
                } else {
                    NativeMessageButtons::Ok
                };

                let mut options = NativeMessageOptions::new(&self.text)
                    .title(self.dialog.title())
                    .level(level)
                    .buttons(native_buttons);

                if !self.informative_text.is_empty() {
                    options = options.detail(&self.informative_text);
                }

                if let Some(result) = native_dialogs::show_message(options) {
                    // Convert native result to StandardButton and emit signal
                    let clicked = match result {
                        NativeMessageResult::Ok => StandardButton::OK,
                        NativeMessageResult::Cancel => StandardButton::CANCEL,
                        NativeMessageResult::Yes => StandardButton::YES,
                        NativeMessageResult::No => StandardButton::NO,
                    };
                    self.clicked_button = Some(clicked);
                    self.button_clicked.emit(clicked);
                    return;
                }
                // Native dialog cancelled - don't show custom dialog
                return;
            }
        }

        // Use custom dialog
        self.dialog.open();
    }

    /// Accept the dialog.
    pub fn accept(&mut self) {
        self.dialog.accept();
    }

    /// Reject the dialog.
    pub fn reject(&mut self) {
        self.dialog.reject();
    }

    /// Close the dialog.
    pub fn close(&mut self) {
        self.dialog.close();
    }

    /// Handle a standard button click.
    pub fn handle_button_click(&mut self, button: StandardButton) {
        self.clicked_button = Some(button);
        self.button_clicked.emit(button);
        self.dialog.handle_button_click(button);
    }

    /// Handle a custom button click.
    pub fn handle_custom_button_click(&mut self, id: u32) {
        self.clicked_custom_button = Some(id);
        self.custom_button_clicked.emit(id);

        // Find the button's role and close appropriately
        if let Some(button_info) = self.custom_buttons.iter().find(|b| b.id == id) {
            match button_info.role {
                ButtonRole::Accept => self.dialog.accept(),
                ButtonRole::Reject | ButtonRole::Destructive => self.dialog.reject(),
                _ => {}
            }
        }
    }

    // =========================================================================
    // Signal Access (delegated from dialog)
    // =========================================================================

    /// Get a reference to the accepted signal.
    pub fn accepted(&self) -> &Signal<()> {
        &self.dialog.accepted
    }

    /// Get a reference to the rejected signal.
    pub fn rejected(&self) -> &Signal<()> {
        &self.dialog.rejected
    }

    /// Get a reference to the finished signal.
    pub fn finished(&self) -> &Signal<DialogResult> {
        &self.dialog.finished
    }

    // =========================================================================
    // Size Calculation
    // =========================================================================

    /// Update the dialog size based on content.
    fn update_size(&mut self) {
        let min_width: f32 = 350.0;
        let max_width: f32 = 500.0;

        // Calculate content height
        let mut content_height = self.content_padding * 2.0;

        // Icon and text area
        let text_area_start = if self.icon.is_visible() {
            self.content_padding + self.icon_size + self.icon_text_spacing
        } else {
            self.content_padding
        };
        let text_width = max_width - text_area_start - self.content_padding;

        // Estimate text heights (assuming ~20 chars per line at 14px font)
        let chars_per_line = (text_width / 7.0) as usize;
        let main_text_lines = (self.text.len() / chars_per_line).max(1);
        content_height += main_text_lines as f32 * 20.0;

        if !self.informative_text.is_empty() {
            let info_lines = (self.informative_text.len() / chars_per_line).max(1);
            content_height += 8.0 + info_lines as f32 * 18.0;
        }

        // Show Details button if we have detailed text
        if !self.detailed_text.is_empty() {
            content_height += 32.0; // Space for "Show Details" button

            if self.detail_button_state.expanded {
                content_height += self.detail_section_height;
            }
        }

        // Title bar + button row
        let title_bar_height = 28.0;
        let total_height = title_bar_height + content_height + self.button_row_height;

        let width = min_width.max(max_width.min(400.0));
        let height = total_height.max(160.0);

        self.dialog.widget_base_mut().set_size(Size::new(width, height));
    }

    // =========================================================================
    // Geometry
    // =========================================================================

    /// Get the icon rectangle.
    fn icon_rect(&self) -> Rect {
        let title_bar_height = 28.0;
        Rect::new(
            self.content_padding,
            title_bar_height + self.content_padding,
            self.icon_size,
            self.icon_size,
        )
    }

    /// Get the text area rectangle.
    fn text_rect(&self) -> Rect {
        let title_bar_height = 28.0;
        let rect = self.dialog.widget_base().rect();

        let left = if self.icon.is_visible() {
            self.content_padding + self.icon_size + self.icon_text_spacing
        } else {
            self.content_padding
        };

        Rect::new(
            left,
            title_bar_height + self.content_padding,
            rect.width() - left - self.content_padding,
            rect.height() - title_bar_height - self.button_row_height - self.content_padding * 2.0,
        )
    }

    /// Get the "Show Details" button rectangle (if detailed text exists).
    fn detail_button_rect(&self) -> Option<Rect> {
        if self.detailed_text.is_empty() {
            return None;
        }

        let title_bar_height = 28.0;
        let rect = self.dialog.widget_base().rect();

        // Position just above the button row
        let y = rect.height() - self.button_row_height - 36.0;
        if self.detail_button_state.expanded {
            // Move up to account for detail section
            Some(Rect::new(
                self.content_padding,
                title_bar_height + self.content_padding + 60.0,
                120.0,
                24.0,
            ))
        } else {
            Some(Rect::new(
                self.content_padding,
                y,
                120.0,
                24.0,
            ))
        }
    }

    /// Get the detail text section rectangle (when expanded).
    fn detail_section_rect(&self) -> Option<Rect> {
        if self.detailed_text.is_empty() || !self.detail_button_state.expanded {
            return None;
        }

        let rect = self.dialog.widget_base().rect();

        // Position below the detail button, above the button row
        if let Some(button_rect) = self.detail_button_rect() {
            Some(Rect::new(
                self.content_padding,
                button_rect.origin.y + button_rect.height() + 8.0,
                rect.width() - self.content_padding * 2.0,
                self.detail_section_height,
            ))
        } else {
            None
        }
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Check detail button click
        if let Some(detail_rect) = self.detail_button_rect() {
            if detail_rect.contains(event.local_pos) {
                self.detail_button_state.pressed = true;
                self.dialog.widget_base_mut().update();
                return true;
            }
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Check detail button release
        if self.detail_button_state.pressed {
            self.detail_button_state.pressed = false;

            if let Some(detail_rect) = self.detail_button_rect() {
                if detail_rect.contains(event.local_pos) {
                    self.detail_button_state.expanded = !self.detail_button_state.expanded;
                    self.update_size();
                }
            }
            self.dialog.widget_base_mut().update();
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        // Update detail button hover state
        if let Some(detail_rect) = self.detail_button_rect() {
            let new_hover = detail_rect.contains(event.local_pos);
            if self.detail_button_state.hovered != new_hover {
                self.detail_button_state.hovered = new_hover;
                self.dialog.widget_base_mut().update();
                return true;
            }
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Enter to activate default button
        if event.key == Key::Enter && !event.is_repeat {
            if self.default_button != StandardButton::NONE {
                self.handle_button_click(self.default_button);
                return true;
            }
        }

        // Escape to activate escape button
        if event.key == Key::Escape {
            if self.escape_button != StandardButton::NONE {
                self.handle_button_click(self.escape_button);
                return true;
            }
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_icon(&self, ctx: &mut PaintContext<'_>) {
        if !self.icon.is_visible() {
            return;
        }

        let rect = self.icon_rect();
        let center = Point::new(
            rect.origin.x + rect.width() / 2.0,
            rect.origin.y + rect.height() / 2.0,
        );
        let radius = self.icon_size / 2.0 - 2.0;
        let color = self.icon.color();

        match self.icon {
            MessageIcon::Information => {
                // Blue circle with "i"
                self.draw_circle(ctx, center, radius, color);
                self.draw_info_symbol(ctx, center, Color::WHITE);
            }
            MessageIcon::Warning => {
                // Orange triangle with "!"
                self.draw_warning_triangle(ctx, rect, color);
                self.draw_exclamation(ctx, center, Color::from_rgb8(40, 40, 40));
            }
            MessageIcon::Critical => {
                // Red circle with "X"
                self.draw_circle(ctx, center, radius, color);
                self.draw_x_symbol(ctx, center, radius * 0.5, Color::WHITE);
            }
            MessageIcon::Question => {
                // Blue circle with "?"
                self.draw_circle(ctx, center, radius, color);
                self.draw_question_symbol(ctx, center, Color::WHITE);
            }
            MessageIcon::NoIcon => {}
        }
    }

    fn draw_circle(&self, ctx: &mut PaintContext<'_>, center: Point, radius: f32, color: Color) {
        // Approximate circle with a rounded rect
        let rect = Rect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        );
        ctx.renderer().fill_rounded_rect(RoundedRect::new(rect, radius), color);
    }

    fn draw_info_symbol(&self, ctx: &mut PaintContext<'_>, center: Point, color: Color) {
        let stroke = Stroke::new(color, 2.5);

        // Dot at top
        let dot_center = Point::new(center.x, center.y - 6.0);
        let dot_rect = Rect::new(dot_center.x - 2.0, dot_center.y - 2.0, 4.0, 4.0);
        ctx.renderer().fill_rounded_rect(RoundedRect::new(dot_rect, 2.0), color);

        // Vertical line below
        ctx.renderer().draw_line(
            Point::new(center.x, center.y - 1.0),
            Point::new(center.x, center.y + 8.0),
            &stroke,
        );
    }

    fn draw_warning_triangle(&self, ctx: &mut PaintContext<'_>, rect: Rect, color: Color) {
        // Draw a filled triangle using lines (simplified approximation)
        let cx = rect.origin.x + rect.width() / 2.0;
        let top = rect.origin.y + 2.0;
        let bottom = rect.origin.y + rect.height() - 2.0;

        // Fill with horizontal lines (simple triangle fill)
        let height = bottom - top;
        let stroke = Stroke::new(color, 1.0);

        for i in 0..=(height as i32) {
            let y = top + i as f32;
            let progress = i as f32 / height;
            let half_width = progress * (rect.width() / 2.0 - 2.0);
            ctx.renderer().draw_line(
                Point::new(cx - half_width, y),
                Point::new(cx + half_width, y),
                &stroke,
            );
        }
    }

    fn draw_exclamation(&self, ctx: &mut PaintContext<'_>, center: Point, color: Color) {
        let stroke = Stroke::new(color, 2.5);

        // Vertical line
        ctx.renderer().draw_line(
            Point::new(center.x, center.y - 6.0),
            Point::new(center.x, center.y + 2.0),
            &stroke,
        );

        // Dot at bottom
        let dot_center = Point::new(center.x, center.y + 7.0);
        let dot_rect = Rect::new(dot_center.x - 2.0, dot_center.y - 2.0, 4.0, 4.0);
        ctx.renderer().fill_rounded_rect(RoundedRect::new(dot_rect, 2.0), color);
    }

    fn draw_x_symbol(&self, ctx: &mut PaintContext<'_>, center: Point, half_size: f32, color: Color) {
        let stroke = Stroke::new(color, 2.5);

        ctx.renderer().draw_line(
            Point::new(center.x - half_size, center.y - half_size),
            Point::new(center.x + half_size, center.y + half_size),
            &stroke,
        );
        ctx.renderer().draw_line(
            Point::new(center.x + half_size, center.y - half_size),
            Point::new(center.x - half_size, center.y + half_size),
            &stroke,
        );
    }

    fn draw_question_symbol(&self, ctx: &mut PaintContext<'_>, center: Point, color: Color) {
        let stroke = Stroke::new(color, 2.5);

        // Curved part of ? (simplified as lines)
        // Arc from top-left to right
        ctx.renderer().draw_line(
            Point::new(center.x - 4.0, center.y - 8.0),
            Point::new(center.x + 2.0, center.y - 10.0),
            &stroke,
        );
        ctx.renderer().draw_line(
            Point::new(center.x + 2.0, center.y - 10.0),
            Point::new(center.x + 5.0, center.y - 6.0),
            &stroke,
        );
        ctx.renderer().draw_line(
            Point::new(center.x + 5.0, center.y - 6.0),
            Point::new(center.x + 2.0, center.y - 2.0),
            &stroke,
        );
        ctx.renderer().draw_line(
            Point::new(center.x + 2.0, center.y - 2.0),
            Point::new(center.x, center.y + 1.0),
            &stroke,
        );

        // Dot at bottom
        let dot_center = Point::new(center.x, center.y + 7.0);
        let dot_rect = Rect::new(dot_center.x - 2.0, dot_center.y - 2.0, 4.0, 4.0);
        ctx.renderer().fill_rounded_rect(RoundedRect::new(dot_rect, 2.0), color);
    }

    fn paint_detail_button(&self, ctx: &mut PaintContext<'_>) {
        if let Some(rect) = self.detail_button_rect() {
            // Background
            let bg_color = if self.detail_button_state.pressed {
                Color::from_rgb8(200, 200, 200)
            } else if self.detail_button_state.hovered {
                Color::from_rgb8(230, 230, 230)
            } else {
                Color::from_rgb8(240, 240, 240)
            };

            let rrect = RoundedRect::new(rect, 4.0);
            ctx.renderer().fill_rounded_rect(rrect, bg_color);

            // Border
            let stroke = Stroke::new(Color::from_rgb8(180, 180, 180), 1.0);
            let rrect_border = RoundedRect::new(rect, 4.0);
            ctx.renderer().stroke_rounded_rect(rrect_border, &stroke);

            // Text is handled by the text rendering system
            // For now, we just draw the button background
        }
    }

    fn paint_detail_section(&self, ctx: &mut PaintContext<'_>) {
        if let Some(rect) = self.detail_section_rect() {
            // Background
            let rrect = RoundedRect::new(rect, 4.0);
            ctx.renderer().fill_rounded_rect(rrect, self.detailed_text_background);

            // Border
            let stroke = Stroke::new(Color::from_rgb8(200, 200, 200), 1.0);
            let rrect_border = RoundedRect::new(rect, 4.0);
            ctx.renderer().stroke_rounded_rect(rrect_border, &stroke);
        }
    }
}

impl Object for MessageBox {
    fn object_id(&self) -> ObjectId {
        self.dialog.object_id()
    }
}

impl Widget for MessageBox {
    fn widget_base(&self) -> &WidgetBase {
        self.dialog.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.dialog.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        self.dialog.size_hint()
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint the dialog base
        self.dialog.paint(ctx);

        if !self.dialog.is_open() {
            return;
        }

        // Paint MessageBox-specific content
        self.paint_icon(ctx);

        // Paint detail button and section if applicable
        if !self.detailed_text.is_empty() {
            self.paint_detail_button(ctx);
            if self.detail_button_state.expanded {
                self.paint_detail_section(ctx);
            }
        }

        // Note: Text rendering would be done here with the text rendering system
        // For now, the text fields are stored and would be rendered by the
        // actual rendering implementation
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle our own events first
        let handled = match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            _ => false,
        };

        if handled {
            return true;
        }

        // Delegate to dialog
        self.dialog.event(event)
    }
}

impl Default for MessageBox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::Arc;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_message_icon_colors() {
        assert_eq!(MessageIcon::NoIcon.color(), Color::TRANSPARENT);
        // Information is blue (0, 120, 215)
        assert!(MessageIcon::Information.color().b > 0.0);
        // Warning is orange/yellow (255, 185, 0)
        assert!(MessageIcon::Warning.color().r > 0.0);
        assert!(MessageIcon::Warning.color().g > 0.0);
        // Critical is red (232, 17, 35)
        assert!(MessageIcon::Critical.color().r > 0.0);
        // Question is blue (0, 120, 215)
        assert!(MessageIcon::Question.color().b > 0.0);
    }

    #[test]
    fn test_message_icon_visibility() {
        assert!(!MessageIcon::NoIcon.is_visible());
        assert!(MessageIcon::Information.is_visible());
        assert!(MessageIcon::Warning.is_visible());
        assert!(MessageIcon::Critical.is_visible());
        assert!(MessageIcon::Question.is_visible());
    }

    #[test]
    fn test_message_box_creation() {
        setup();
        let msg = MessageBox::new();
        assert_eq!(msg.icon(), MessageIcon::NoIcon);
        assert!(msg.text().is_empty());
        assert!(!msg.is_open());
    }

    #[test]
    fn test_message_box_builder_pattern() {
        setup();
        let msg = MessageBox::new()
            .with_icon(MessageIcon::Warning)
            .with_title("Test Title")
            .with_text("Test message")
            .with_informative_text("Additional info")
            .with_standard_buttons(StandardButton::YES | StandardButton::NO);

        assert_eq!(msg.icon(), MessageIcon::Warning);
        assert_eq!(msg.title(), "Test Title");
        assert_eq!(msg.text(), "Test message");
        assert_eq!(msg.informative_text(), "Additional info");
        assert!(msg.standard_buttons().has(StandardButton::YES));
        assert!(msg.standard_buttons().has(StandardButton::NO));
    }

    #[test]
    fn test_information_helper() {
        setup();
        let msg = MessageBox::information("Info", "This is information.");
        assert_eq!(msg.icon(), MessageIcon::Information);
        assert_eq!(msg.title(), "Info");
        assert_eq!(msg.text(), "This is information.");
        assert!(msg.standard_buttons().has(StandardButton::OK));
    }

    #[test]
    fn test_warning_helper() {
        setup();
        let msg = MessageBox::warning("Warning", "This is a warning.");
        assert_eq!(msg.icon(), MessageIcon::Warning);
        assert_eq!(msg.title(), "Warning");
        assert_eq!(msg.text(), "This is a warning.");
    }

    #[test]
    fn test_critical_helper() {
        setup();
        let msg = MessageBox::critical("Error", "Critical error occurred.");
        assert_eq!(msg.icon(), MessageIcon::Critical);
        assert_eq!(msg.title(), "Error");
        assert_eq!(msg.text(), "Critical error occurred.");
    }

    #[test]
    fn test_question_helper() {
        setup();
        let msg = MessageBox::question("Confirm", "Are you sure?");
        assert_eq!(msg.icon(), MessageIcon::Question);
        assert_eq!(msg.title(), "Confirm");
        assert_eq!(msg.text(), "Are you sure?");
        assert!(msg.standard_buttons().has(StandardButton::YES));
        assert!(msg.standard_buttons().has(StandardButton::NO));
    }

    #[test]
    fn test_custom_buttons() {
        setup();
        let mut msg = MessageBox::new();
        let id1 = msg.add_button("Custom 1", ButtonRole::Accept);
        let id2 = msg.add_button("Custom 2", ButtonRole::Reject);

        assert_eq!(msg.custom_buttons().len(), 2);
        assert_eq!(msg.custom_buttons()[0].id, id1);
        assert_eq!(msg.custom_buttons()[1].id, id2);

        msg.remove_button(id1);
        assert_eq!(msg.custom_buttons().len(), 1);
        assert_eq!(msg.custom_buttons()[0].id, id2);
    }

    #[test]
    fn test_default_and_escape_buttons() {
        setup();
        let msg = MessageBox::new()
            .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL);

        assert_eq!(msg.default_button(), StandardButton::OK);
        assert_eq!(msg.escape_button(), StandardButton::CANCEL);
    }

    #[test]
    fn test_detailed_text() {
        setup();
        let msg = MessageBox::new()
            .with_text("Main message")
            .with_detailed_text("This is detailed information that can be expanded.");

        assert_eq!(msg.text(), "Main message");
        assert_eq!(msg.detailed_text(), "This is detailed information that can be expanded.");
    }

    #[test]
    fn test_dialog_lifecycle() {
        setup();
        let mut msg = MessageBox::information("Test", "Test message");

        assert!(!msg.is_open());
        msg.open();
        assert!(msg.is_open());
        msg.close();
        assert!(!msg.is_open());
    }

    #[test]
    fn test_button_clicked_signal() {
        setup();
        let mut msg = MessageBox::new()
            .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL);

        let clicked = Arc::new(std::sync::Mutex::new(StandardButton::NONE));
        let clicked_clone = clicked.clone();

        msg.button_clicked.connect(move |button| {
            *clicked_clone.lock().unwrap() = *button;
        });

        msg.open();
        msg.handle_button_click(StandardButton::OK);

        assert_eq!(*clicked.lock().unwrap(), StandardButton::OK);
    }

    #[test]
    fn test_with_buttons_variants() {
        setup();

        let msg = MessageBox::information_with_buttons(
            "Title",
            "Text",
            StandardButton::YES | StandardButton::NO,
        );
        assert!(msg.standard_buttons().has(StandardButton::YES));
        assert!(msg.standard_buttons().has(StandardButton::NO));

        let msg = MessageBox::warning_with_buttons(
            "Title",
            "Text",
            StandardButton::RETRY | StandardButton::CANCEL,
        );
        assert!(msg.standard_buttons().has(StandardButton::RETRY));

        let msg = MessageBox::critical_with_buttons(
            "Title",
            "Text",
            StandardButton::ABORT | StandardButton::IGNORE,
        );
        assert!(msg.standard_buttons().has(StandardButton::ABORT));

        let msg = MessageBox::question_with_buttons(
            "Title",
            "Text",
            StandardButton::SAVE | StandardButton::DISCARD,
        );
        assert!(msg.standard_buttons().has(StandardButton::SAVE));
    }
}
