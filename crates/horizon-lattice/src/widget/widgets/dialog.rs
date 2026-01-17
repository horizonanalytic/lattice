//! Dialog widget implementation.
//!
//! This module provides [`Dialog`], a modal window for user interaction with
//! standard accept/reject semantics.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Dialog, DialogResult, StandardButton};
//!
//! // Create a dialog with a title
//! let mut dialog = Dialog::new("Confirm Action")
//!     .with_size(400.0, 200.0)
//!     .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL);
//!
//! // Connect to signals
//! dialog.accepted.connect(|()| {
//!     println!("User accepted the dialog");
//! });
//!
//! dialog.rejected.connect(|()| {
//!     println!("User rejected the dialog");
//! });
//!
//! // Show the dialog
//! dialog.open();
//! ```

use std::collections::HashMap;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Size, Stroke};

use crate::widget::layout::ContentMargins;
use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, KeyReleaseEvent, MouseButton,
    MouseMoveEvent, MousePressEvent, MouseReleaseEvent, PaintContext, SizeHint, SizePolicy,
    SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

use super::dialog_button_box::{ButtonRole, StandardButton};
use super::window::{WindowFlags, WindowModality};
use crate::widget::ModalManager;

// ============================================================================
// Dialog Result
// ============================================================================

/// The result of a dialog execution.
///
/// Indicates whether the user accepted or rejected the dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogResult {
    /// The dialog was rejected (e.g., user clicked Cancel or closed the window).
    #[default]
    Rejected,

    /// The dialog was accepted (e.g., user clicked OK or Yes).
    Accepted,
}

impl DialogResult {
    /// Check if the dialog was accepted.
    pub fn is_accepted(&self) -> bool {
        matches!(self, DialogResult::Accepted)
    }

    /// Check if the dialog was rejected.
    pub fn is_rejected(&self) -> bool {
        matches!(self, DialogResult::Rejected)
    }
}

impl From<bool> for DialogResult {
    fn from(accepted: bool) -> Self {
        if accepted {
            DialogResult::Accepted
        } else {
            DialogResult::Rejected
        }
    }
}

impl From<DialogResult> for bool {
    fn from(result: DialogResult) -> Self {
        result.is_accepted()
    }
}

// ============================================================================
// Title Bar Button State
// ============================================================================

/// State of a title bar button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ButtonState {
    hovered: bool,
    pressed: bool,
}

// ============================================================================
// Dialog
// ============================================================================

/// A modal dialog window.
///
/// Dialog provides a modal window for user interaction with standard
/// accept/reject semantics. It supports:
///
/// - Modal operation that blocks input to parent windows
/// - Standard button box with OK, Cancel, etc.
/// - Accept/reject result codes
/// - Escape key to reject
/// - Enter key to accept (when default button is set)
///
/// # Modal Execution
///
/// Dialogs can be shown in two ways:
///
/// 1. **Non-blocking (`open()`)**: The dialog is shown and control returns
///    immediately. Use signals to handle the result.
///
/// 2. **Callback-based**: Connect to `finished` signal for result notification.
///
/// Note: Traditional blocking `exec()` is not directly supported in Rust due to
/// ownership constraints. Use `open()` with signals instead.
///
/// # Signals
///
/// - `accepted()`: Emitted when the dialog is accepted
/// - `rejected()`: Emitted when the dialog is rejected
/// - `finished(DialogResult)`: Emitted when the dialog is closed (with result)
pub struct Dialog {
    /// Widget base.
    base: WidgetBase,

    /// The dialog title.
    title: String,

    /// The content widget ID.
    content_widget: Option<ObjectId>,

    /// The dialog result.
    result: DialogResult,

    /// Window modality.
    modality: WindowModality,

    /// Window flags.
    flags: WindowFlags,

    /// Standard buttons to display.
    standard_buttons: StandardButton,

    /// Minimum dialog size.
    min_size: Size,

    /// Maximum dialog size (None means no maximum).
    max_size: Option<Size>,

    /// Title bar height.
    title_bar_height: f32,

    /// Button size.
    button_size: f32,

    /// Border width.
    border_width: f32,

    /// Content margins inside the dialog.
    content_margins: ContentMargins,

    /// Button box height (at bottom of dialog).
    button_box_height: f32,

    // Visual styling
    /// Title bar background color.
    title_bar_color: Color,
    /// Title bar background when active.
    title_bar_active_color: Color,
    /// Title text color.
    title_text_color: Color,
    /// Background color of content area.
    content_background: Color,
    /// Border color.
    border_color: Color,
    /// Button background color.
    button_color: Color,
    /// Button hover color.
    button_hover_color: Color,
    /// Button pressed color.
    button_pressed_color: Color,
    /// Close button hover color.
    close_button_hover_color: Color,
    /// Backdrop color for modal dialogs.
    backdrop_color: Color,

    // Interaction state
    /// Close button state.
    close_button_state: ButtonState,
    /// Whether dragging the title bar to move.
    dragging: bool,
    /// Drag start position (in global coordinates).
    drag_start: Point,
    /// Widget geometry at drag start.
    drag_start_geometry: Rect,
    /// Whether the dialog is currently active.
    active: bool,
    /// Whether the dialog is currently visible/open.
    is_open: bool,

    // Default button state
    /// The ObjectId of the current default button in this dialog.
    ///
    /// This may be the explicitly set default, or temporarily an auto-default
    /// button that has focus.
    default_button: Option<ObjectId>,

    /// The ObjectId of the explicitly set default button.
    ///
    /// This is the "original" default that gets restored when an auto-default
    /// button loses focus.
    explicit_default_button: Option<ObjectId>,

    /// Whether the current default button was set via auto-default.
    ///
    /// Used to track when to restore the explicit default.
    is_auto_default_active: bool,

    // Focus restoration state
    /// The ObjectId of the widget that had focus before the dialog opened.
    ///
    /// This is used to restore focus when the dialog closes.
    previously_focused_widget: Option<ObjectId>,

    /// The ObjectId of the parent window for window-modal dialogs.
    ///
    /// Used for modal input blocking and determining which window to restore focus to.
    parent_window: Option<ObjectId>,

    // Mnemonic state
    /// Whether the Alt key is currently held.
    alt_held: bool,
    /// Current mnemonic cycling state.
    mnemonic_cycle_state: HashMap<char, usize>,
    /// The last mnemonic key pressed.
    last_mnemonic_key: Option<char>,

    // Signals
    /// Signal emitted when the dialog is accepted.
    pub accepted: Signal<()>,
    /// Signal emitted when the dialog is rejected.
    pub rejected: Signal<()>,
    /// Signal emitted when the dialog is finished (closed).
    pub finished: Signal<DialogResult>,
    /// Signal emitted when the dialog becomes visible.
    pub about_to_show: Signal<()>,
    /// Signal emitted when the dialog is about to hide.
    pub about_to_hide: Signal<()>,
    /// Signal emitted when the title changes.
    pub title_changed: Signal<String>,
    /// Signal emitted when the dialog becomes active.
    pub activated: Signal<()>,
    /// Signal emitted when the dialog is deactivated.
    pub deactivated: Signal<()>,
    /// Signal emitted when a standard button is clicked.
    pub button_clicked: Signal<StandardButton>,
    /// Signal emitted when Enter is pressed to activate the default button.
    pub default_button_activated: Signal<ObjectId>,
    /// Signal emitted when an Alt+key mnemonic combination is pressed.
    pub mnemonic_key_pressed: Signal<char>,
    /// Signal emitted when the default button changes (including via auto-default).
    ///
    /// The parameter is the new default button ID (if any).
    pub default_button_changed: Signal<Option<ObjectId>>,
    /// Signal emitted when a widget receives focus inside the dialog.
    ///
    /// Used for auto-default button handling. The parameter is the focused widget ID.
    pub focus_changed: Signal<Option<ObjectId>>,

    /// Signal emitted when the dialog is about to close and focus should be restored.
    ///
    /// The parameter is the ObjectId of the widget that should receive focus,
    /// or `None` if there was no previously focused widget.
    ///
    /// Connect to this signal to restore focus to the appropriate widget when
    /// the dialog closes.
    pub focus_restore_requested: Signal<Option<ObjectId>>,
}

impl Dialog {
    /// Create a new dialog with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred));
        // Dialogs start hidden
        base.hide();

        Self {
            base,
            title: title.into(),
            content_widget: None,
            result: DialogResult::Rejected,
            modality: WindowModality::ApplicationModal,
            flags: WindowFlags::DIALOG,
            standard_buttons: StandardButton::NONE,
            min_size: Size::new(200.0, 100.0),
            max_size: None,
            title_bar_height: 28.0,
            button_size: 20.0,
            border_width: 1.0,
            content_margins: ContentMargins::uniform(12.0),
            button_box_height: 48.0,
            title_bar_color: Color::from_rgb8(240, 240, 240),
            title_bar_active_color: Color::from_rgb8(200, 220, 240),
            title_text_color: Color::from_rgb8(40, 40, 40),
            content_background: Color::WHITE,
            border_color: Color::from_rgb8(160, 160, 160),
            button_color: Color::from_rgb8(240, 240, 240),
            button_hover_color: Color::from_rgb8(220, 220, 220),
            button_pressed_color: Color::from_rgb8(200, 200, 200),
            close_button_hover_color: Color::from_rgb8(232, 17, 35),
            backdrop_color: Color::from_rgba8(0, 0, 0, 80),
            close_button_state: ButtonState::default(),
            dragging: false,
            drag_start: Point::ZERO,
            drag_start_geometry: Rect::ZERO,
            active: false,
            is_open: false,
            default_button: None,
            explicit_default_button: None,
            is_auto_default_active: false,
            previously_focused_widget: None,
            parent_window: None,
            alt_held: false,
            mnemonic_cycle_state: HashMap::new(),
            last_mnemonic_key: None,
            accepted: Signal::new(),
            rejected: Signal::new(),
            finished: Signal::new(),
            about_to_show: Signal::new(),
            about_to_hide: Signal::new(),
            title_changed: Signal::new(),
            activated: Signal::new(),
            deactivated: Signal::new(),
            button_clicked: Signal::new(),
            default_button_activated: Signal::new(),
            mnemonic_key_pressed: Signal::new(),
            default_button_changed: Signal::new(),
            focus_changed: Signal::new(),
            focus_restore_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the dialog size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.base.set_size(Size::new(width, height));
        self
    }

    /// Set the dialog position using builder pattern.
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.base.set_pos(Point::new(x, y));
        self
    }

    /// Set the title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set the standard buttons using builder pattern.
    pub fn with_standard_buttons(mut self, buttons: StandardButton) -> Self {
        self.standard_buttons = buttons;
        self
    }

    /// Set the modality using builder pattern.
    pub fn with_modality(mut self, modality: WindowModality) -> Self {
        self.modality = modality;
        self
    }

    /// Set the content margins using builder pattern.
    pub fn with_content_margins(mut self, margins: ContentMargins) -> Self {
        self.content_margins = margins;
        self
    }

    /// Set the minimum size using builder pattern.
    pub fn with_min_size(mut self, width: f32, height: f32) -> Self {
        self.min_size = Size::new(width, height);
        self
    }

    /// Set the maximum size using builder pattern.
    pub fn with_max_size(mut self, width: f32, height: f32) -> Self {
        self.max_size = Some(Size::new(width, height));
        self
    }

    // =========================================================================
    // Title
    // =========================================================================

    /// Get the dialog title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the dialog title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        let new_title = title.into();
        if self.title != new_title {
            self.title = new_title.clone();
            self.base.update();
            self.title_changed.emit(new_title);
        }
    }

    // =========================================================================
    // Content Widget
    // =========================================================================

    /// Get the content widget ID.
    pub fn content_widget(&self) -> Option<ObjectId> {
        self.content_widget
    }

    /// Set the content widget.
    pub fn set_content_widget(&mut self, widget_id: ObjectId) {
        self.content_widget = Some(widget_id);
        self.base.update();
    }

    /// Set content widget using builder pattern.
    pub fn with_content_widget(mut self, widget_id: ObjectId) -> Self {
        self.content_widget = Some(widget_id);
        self
    }

    // =========================================================================
    // Standard Buttons
    // =========================================================================

    /// Get the standard buttons.
    pub fn standard_buttons(&self) -> StandardButton {
        self.standard_buttons
    }

    /// Set the standard buttons to display.
    pub fn set_standard_buttons(&mut self, buttons: StandardButton) {
        if self.standard_buttons != buttons {
            self.standard_buttons = buttons;
            self.base.update();
        }
    }

    /// Add a standard button.
    pub fn add_standard_button(&mut self, button: StandardButton) {
        if !self.standard_buttons.has(button) {
            self.standard_buttons |= button;
            self.base.update();
        }
    }

    // =========================================================================
    // Result
    // =========================================================================

    /// Get the dialog result.
    pub fn result(&self) -> DialogResult {
        self.result
    }

    /// Set the dialog result.
    ///
    /// This does not close the dialog; call `accept()` or `reject()` instead.
    pub fn set_result(&mut self, result: DialogResult) {
        self.result = result;
    }

    // =========================================================================
    // Modality
    // =========================================================================

    /// Get the window modality.
    pub fn modality(&self) -> WindowModality {
        self.modality
    }

    /// Set the window modality.
    pub fn set_modality(&mut self, modality: WindowModality) {
        self.modality = modality;
    }

    /// Check if the dialog is modal.
    pub fn is_modal(&self) -> bool {
        self.modality.is_modal()
    }

    // =========================================================================
    // Parent Window
    // =========================================================================

    /// Get the parent window ID.
    ///
    /// This is used for window-modal dialogs to determine which window to block,
    /// and for focus restoration when the dialog closes.
    pub fn parent_window(&self) -> Option<ObjectId> {
        self.parent_window
    }

    /// Set the parent window.
    ///
    /// The parent window is blocked when using `WindowModality::WindowModal`.
    pub fn set_parent_window(&mut self, parent_id: Option<ObjectId>) {
        self.parent_window = parent_id;
    }

    /// Set parent window using builder pattern.
    pub fn with_parent_window(mut self, parent_id: ObjectId) -> Self {
        self.parent_window = Some(parent_id);
        self
    }

    // =========================================================================
    // Focus Restoration
    // =========================================================================

    /// Get the widget that had focus before the dialog was opened.
    ///
    /// This is used to restore focus when the dialog closes.
    pub fn previously_focused_widget(&self) -> Option<ObjectId> {
        self.previously_focused_widget
    }

    /// Set the previously focused widget.
    ///
    /// This should be called before opening the dialog to save the current
    /// focus state for restoration later.
    pub fn set_previously_focused_widget(&mut self, widget_id: Option<ObjectId>) {
        self.previously_focused_widget = widget_id;
    }

    // =========================================================================
    // Size Constraints
    // =========================================================================

    /// Get the minimum dialog size.
    pub fn min_size(&self) -> Size {
        self.min_size
    }

    /// Set the minimum dialog size.
    pub fn set_min_size(&mut self, size: Size) {
        self.min_size = size;
    }

    /// Get the maximum dialog size.
    pub fn max_size(&self) -> Option<Size> {
        self.max_size
    }

    /// Set the maximum dialog size.
    pub fn set_max_size(&mut self, size: Option<Size>) {
        self.max_size = size;
    }

    // =========================================================================
    // Default Button
    // =========================================================================

    /// Get the current default button's ObjectId, if one is set.
    ///
    /// This returns the currently active default button, which may be either:
    /// - The explicitly set default button, or
    /// - A temporarily promoted auto-default button that has focus
    pub fn default_button(&self) -> Option<ObjectId> {
        self.default_button
    }

    /// Get the explicitly set default button's ObjectId.
    ///
    /// This is the "original" default button, not affected by auto-default behavior.
    pub fn explicit_default_button(&self) -> Option<ObjectId> {
        self.explicit_default_button
    }

    /// Set the default button for this dialog.
    ///
    /// This sets both the current and explicit default button. The explicit
    /// default will be restored when an auto-default button loses focus.
    pub fn set_default_button(&mut self, button_id: Option<ObjectId>) {
        let old_default = self.default_button;
        self.default_button = button_id;
        self.explicit_default_button = button_id;
        self.is_auto_default_active = false;

        if old_default != self.default_button {
            self.default_button_changed.emit(self.default_button);
        }
    }

    /// Set default button using builder pattern.
    pub fn with_default_button(mut self, button_id: ObjectId) -> Self {
        self.default_button = Some(button_id);
        self.explicit_default_button = Some(button_id);
        self
    }

    /// Check if the current default was set via auto-default behavior.
    pub fn is_auto_default_active(&self) -> bool {
        self.is_auto_default_active
    }

    // =========================================================================
    // Auto-Default Handling
    // =========================================================================

    /// Activate auto-default for a button.
    ///
    /// This should be called when a button with `is_auto_default() == true`
    /// receives focus via Tab navigation. The button becomes the temporary
    /// default button, and the original default is saved for restoration.
    ///
    /// # Arguments
    ///
    /// * `button_id` - The ObjectId of the auto-default button receiving focus
    ///
    /// # Returns
    ///
    /// `true` if the auto-default was activated, `false` if it was already the default.
    pub fn activate_auto_default(&mut self, button_id: ObjectId) -> bool {
        if self.default_button == Some(button_id) {
            return false;
        }

        // Save the current explicit default if we haven't already
        if !self.is_auto_default_active {
            // The explicit_default_button is already set, so we just need to
            // mark that auto-default is now active
        }

        let old_default = self.default_button;
        self.default_button = Some(button_id);
        self.is_auto_default_active = true;
        self.base.update();

        if old_default != self.default_button {
            self.default_button_changed.emit(self.default_button);
        }

        true
    }

    /// Restore the explicit default button.
    ///
    /// This should be called when an auto-default button loses focus.
    /// The original explicit default button (if any) becomes the default again.
    ///
    /// # Returns
    ///
    /// `true` if the default was restored, `false` if auto-default wasn't active.
    pub fn restore_explicit_default(&mut self) -> bool {
        if !self.is_auto_default_active {
            return false;
        }

        let old_default = self.default_button;
        self.default_button = self.explicit_default_button;
        self.is_auto_default_active = false;
        self.base.update();

        if old_default != self.default_button {
            self.default_button_changed.emit(self.default_button);
        }

        true
    }

    /// Handle focus change for auto-default button behavior.
    ///
    /// This method should be called when focus changes within the dialog.
    /// It checks if the newly focused widget is an auto-default button and
    /// updates the default button accordingly.
    ///
    /// # Arguments
    ///
    /// * `focused_widget_id` - The ObjectId of the newly focused widget, or None
    /// * `is_auto_default_button` - Whether the focused widget is an auto-default button
    ///
    /// # Returns
    ///
    /// `true` if the default button state changed, `false` otherwise.
    pub fn handle_focus_for_auto_default(
        &mut self,
        focused_widget_id: Option<ObjectId>,
        is_auto_default_button: bool,
    ) -> bool {
        self.focus_changed.emit(focused_widget_id);

        match (focused_widget_id, is_auto_default_button) {
            (Some(id), true) => {
                // Auto-default button gained focus
                self.activate_auto_default(id)
            }
            (Some(_), false) | (None, _) => {
                // Non-auto-default widget gained focus, or focus cleared
                // Restore the explicit default
                self.restore_explicit_default()
            }
        }
    }

    // =========================================================================
    // Dialog Lifecycle
    // =========================================================================

    /// Open the dialog (non-blocking modal).
    ///
    /// Shows the dialog as a modal and returns immediately. Connect to
    /// the `finished`, `accepted`, or `rejected` signals to handle the result.
    ///
    /// # Example
    ///
    /// ```ignore
    /// dialog.finished.connect(|result| {
    ///     if result.is_accepted() {
    ///         println!("User accepted");
    ///     }
    /// });
    /// dialog.open();
    /// ```
    pub fn open(&mut self) {
        if self.is_open {
            return;
        }

        self.is_open = true;
        self.result = DialogResult::Rejected; // Default to rejected

        // Register with the modal manager if this is a modal dialog
        if self.modality.is_modal() {
            ModalManager::push_modal(
                self.base.object_id(),
                self.modality,
                self.parent_window,
            );
        }

        self.about_to_show.emit(());
        self.base.show();
        self.activate();
        self.base.update();
    }

    /// Open the dialog with focus restoration support.
    ///
    /// This is like `open()` but also saves the previously focused widget
    /// for automatic focus restoration when the dialog closes.
    ///
    /// # Arguments
    ///
    /// * `previous_focus` - The ObjectId of the widget that currently has focus
    pub fn open_with_focus(&mut self, previous_focus: Option<ObjectId>) {
        self.previously_focused_widget = previous_focus;
        self.open();
    }

    /// Show the dialog (alias for `open()`).
    pub fn show(&mut self) {
        self.open();
    }

    /// Hide the dialog without setting a result.
    ///
    /// Prefer using `accept()` or `reject()` to close with a proper result.
    pub fn hide(&mut self) {
        if !self.is_open {
            return;
        }

        self.about_to_hide.emit(());
        self.is_open = false;
        self.base.hide();
    }

    /// Accept the dialog.
    ///
    /// Sets the result to `Accepted`, emits `accepted()` and `finished(Accepted)`,
    /// then closes the dialog.
    pub fn accept(&mut self) {
        self.done(DialogResult::Accepted);
    }

    /// Reject the dialog.
    ///
    /// Sets the result to `Rejected`, emits `rejected()` and `finished(Rejected)`,
    /// then closes the dialog.
    pub fn reject(&mut self) {
        self.done(DialogResult::Rejected);
    }

    /// Close the dialog with the specified result.
    ///
    /// This is the main method for closing a dialog. It:
    /// 1. Sets the result code
    /// 2. Emits `accepted()` or `rejected()` based on the result
    /// 3. Emits `finished(result)`
    /// 4. Unregisters from the modal manager
    /// 5. Emits `focus_restore_requested` for focus restoration
    /// 6. Hides the dialog
    pub fn done(&mut self, result: DialogResult) {
        if !self.is_open {
            return;
        }

        self.result = result;

        // Emit result-specific signal
        match result {
            DialogResult::Accepted => self.accepted.emit(()),
            DialogResult::Rejected => self.rejected.emit(()),
        }

        // Emit finished signal
        self.finished.emit(result);

        // Unregister from modal manager
        if self.modality.is_modal() {
            ModalManager::pop_modal(self.base.object_id());
        }

        // Request focus restoration before hiding
        // This allows the application to restore focus to the previously focused widget
        self.focus_restore_requested.emit(self.previously_focused_widget);

        // Clear the previously focused widget
        self.previously_focused_widget = None;

        // Hide the dialog
        self.hide();
    }

    /// Check if the dialog is currently open.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Close the dialog by rejecting it.
    ///
    /// This is called when the close button is clicked or Escape is pressed.
    pub fn close(&mut self) {
        self.reject();
    }

    // =========================================================================
    // Button Click Handling
    // =========================================================================

    /// Handle a standard button click.
    ///
    /// This should be called when a button in the dialog's button box is clicked.
    pub fn handle_button_click(&mut self, button: StandardButton) {
        self.button_clicked.emit(button);

        // Close the dialog based on button role
        match button.role() {
            ButtonRole::Accept => self.accept(),
            ButtonRole::Reject | ButtonRole::Destructive => self.reject(),
            _ => {}
        }
    }

    // =========================================================================
    // Active State
    // =========================================================================

    /// Check if the dialog is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Activate the dialog.
    pub fn activate(&mut self) {
        if !self.active {
            self.active = true;
            self.base.update();
            self.activated.emit(());
        }
    }

    /// Deactivate the dialog.
    pub fn deactivate(&mut self) {
        if self.active {
            self.active = false;
            self.base.update();
            self.deactivated.emit(());
        }
    }

    // =========================================================================
    // Mnemonic State
    // =========================================================================

    /// Check if the Alt key is currently held.
    pub fn is_alt_held(&self) -> bool {
        self.alt_held
    }

    /// Clear the mnemonic cycle state.
    fn reset_mnemonic_cycle(&mut self) {
        self.mnemonic_cycle_state.clear();
        self.last_mnemonic_key = None;
    }

    // =========================================================================
    // Geometry Calculations
    // =========================================================================

    /// Get the title bar rectangle.
    fn title_bar_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(0.0, 0.0, rect.width(), self.title_bar_height)
    }

    /// Get the close button rectangle.
    fn close_button_rect(&self) -> Rect {
        let title_rect = self.title_bar_rect();
        let padding = (self.title_bar_height - self.button_size) / 2.0;

        Rect::new(
            title_rect.width() - padding - self.button_size,
            padding,
            self.button_size,
            self.button_size,
        )
    }

    /// Get the content area rectangle.
    pub fn content_rect(&self) -> Rect {
        let rect = self.base.rect();

        let top = self.title_bar_height + self.content_margins.top;
        let bottom = if self.standard_buttons.is_empty() {
            self.content_margins.bottom
        } else {
            self.button_box_height + self.content_margins.bottom
        };

        Rect::new(
            self.border_width + self.content_margins.left,
            top,
            rect.width() - self.border_width * 2.0 - self.content_margins.horizontal(),
            rect.height() - top - bottom - self.border_width,
        )
    }

    /// Get the button box rectangle.
    pub fn button_box_rect(&self) -> Rect {
        let rect = self.base.rect();

        Rect::new(
            self.border_width,
            rect.height() - self.button_box_height - self.border_width,
            rect.width() - self.border_width * 2.0,
            self.button_box_height,
        )
    }

    /// Check if the position is in the title bar drag area.
    fn is_in_title_bar_drag_area(&self, pos: Point) -> bool {
        let title_rect = self.title_bar_rect();
        if !title_rect.contains(pos) {
            return false;
        }
        // Not over the close button
        !self.close_button_rect().contains(pos)
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check close button click
        if self.close_button_rect().contains(pos) {
            self.close_button_state.pressed = true;
            self.base.update();
            return true;
        }

        // Check title bar drag
        if self.is_in_title_bar_drag_area(pos) {
            self.dragging = true;
            self.drag_start = event.global_pos;
            self.drag_start_geometry = self.base.geometry();
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check close button release
        if self.close_button_state.pressed {
            self.close_button_state.pressed = false;
            if self.close_button_rect().contains(pos) {
                self.close();
            }
            self.base.update();
            return true;
        }

        // End drag
        if self.dragging {
            self.dragging = false;
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;

        // Update close button hover state
        let new_close_hover = self.close_button_rect().contains(pos);

        if self.close_button_state.hovered != new_close_hover {
            self.close_button_state.hovered = new_close_hover;
            self.base.update();
        }

        // Handle dragging
        if self.dragging {
            let delta = Point::new(
                event.global_pos.x - self.drag_start.x,
                event.global_pos.y - self.drag_start.y,
            );

            let new_pos = Point::new(
                self.drag_start_geometry.origin.x + delta.x,
                self.drag_start_geometry.origin.y + delta.y,
            );
            self.base.set_pos(new_pos);
            return true;
        }

        new_close_hover != self.close_button_state.hovered
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Escape to reject
        if event.key == Key::Escape {
            self.reject();
            return true;
        }

        // Enter to accept (via default button)
        if event.key == Key::Enter && !event.is_repeat {
            if let Some(button_id) = self.default_button {
                self.default_button_activated.emit(button_id);
                return true;
            }
            // If no default button but we have OK, accept
            if self.standard_buttons.has(StandardButton::OK)
                || self.standard_buttons.has(StandardButton::YES)
                || self.standard_buttons.has(StandardButton::SAVE)
            {
                self.accept();
                return true;
            }
        }

        // Handle Alt key press - show mnemonic underlines
        if matches!(event.key, Key::AltLeft | Key::AltRight) {
            if !self.alt_held {
                self.alt_held = true;
                self.base.update();
            }
            return false;
        }

        // Handle Alt+key mnemonic activation
        if event.modifiers.alt {
            if let Some(key_char) = event.key.to_ascii_char() {
                self.mnemonic_key_pressed.emit(key_char);
                return true;
            }
        }

        false
    }

    fn handle_key_release(&mut self, event: &KeyReleaseEvent) -> bool {
        // Handle Alt key release
        if matches!(event.key, Key::AltLeft | Key::AltRight) {
            if !event.modifiers.alt && self.alt_held {
                self.alt_held = false;
                self.reset_mnemonic_cycle();
                self.base.update();
            }
            return false;
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_backdrop(&self, ctx: &mut PaintContext<'_>) {
        if !self.modality.is_modal() {
            return;
        }

        // Paint a semi-transparent backdrop
        let rect = self.base.rect();
        let backdrop_rect = Rect::new(
            -rect.origin.x,
            -rect.origin.y,
            rect.origin.x * 2.0 + rect.width() + 2000.0,
            rect.origin.y * 2.0 + rect.height() + 2000.0,
        );
        ctx.renderer().fill_rect(backdrop_rect, self.backdrop_color);
    }

    fn paint_border(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let border_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());
        let stroke = Stroke::new(self.border_color, self.border_width);
        ctx.renderer().stroke_rect(border_rect, &stroke);
    }

    fn paint_title_bar(&self, ctx: &mut PaintContext<'_>) {
        let title_rect = self.title_bar_rect();

        // Background
        let bg_color = if self.active {
            self.title_bar_active_color
        } else {
            self.title_bar_color
        };
        ctx.renderer().fill_rect(title_rect, bg_color);

        // Draw close button
        self.paint_close_button(ctx);
    }

    fn paint_close_button(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.close_button_rect();

        // Button background
        let bg = if self.close_button_state.pressed {
            self.button_pressed_color
        } else if self.close_button_state.hovered {
            self.close_button_hover_color
        } else {
            self.button_color
        };
        ctx.renderer().fill_rect(rect, bg);

        // Draw X icon
        let icon_margin = 5.0;
        let x1 = rect.origin.x + icon_margin;
        let y1 = rect.origin.y + icon_margin;
        let x2 = rect.origin.x + rect.width() - icon_margin;
        let y2 = rect.origin.y + rect.height() - icon_margin;

        let icon_color = if self.close_button_state.hovered {
            Color::WHITE
        } else {
            Color::from_rgb8(80, 80, 80)
        };
        let stroke = Stroke::new(icon_color, 1.5);

        ctx.renderer()
            .draw_line(Point::new(x1, y1), Point::new(x2, y2), &stroke);
        ctx.renderer()
            .draw_line(Point::new(x2, y1), Point::new(x1, y2), &stroke);
    }

    fn paint_content_area(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let content_rect = Rect::new(
            self.border_width,
            self.title_bar_height,
            rect.width() - self.border_width * 2.0,
            rect.height() - self.title_bar_height - self.border_width,
        );
        ctx.renderer().fill_rect(content_rect, self.content_background);
    }

    fn paint_button_box_separator(&self, ctx: &mut PaintContext<'_>) {
        if self.standard_buttons.is_empty() {
            return;
        }

        let button_box_rect = self.button_box_rect();

        // Draw a separator line above the button box
        let separator_y = button_box_rect.origin.y;
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().draw_line(
            Point::new(self.border_width, separator_y),
            Point::new(
                self.base.rect().width() - self.border_width,
                separator_y,
            ),
            &stroke,
        );
    }
}

impl Object for Dialog {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Dialog {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let preferred = Size::new(400.0, 200.0);
        SizeHint::new(preferred).with_minimum(self.min_size)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        if !self.is_open {
            return;
        }

        // Paint in order: backdrop (if modal), content background, title bar, border
        self.paint_backdrop(ctx);
        self.paint_content_area(ctx);
        self.paint_title_bar(ctx);
        self.paint_button_box_separator(ctx);
        self.paint_border(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::KeyRelease(e) => self.handle_key_release(e),
            WidgetEvent::Leave(_) => {
                // Clear hover state
                if self.close_button_state.hovered {
                    self.close_button_state.hovered = false;
                    self.base.update();
                }
                false
            }
            WidgetEvent::FocusIn(_) => {
                self.activate();
                true
            }
            WidgetEvent::FocusOut(_) => {
                self.deactivate();
                true
            }
            _ => false,
        }
    }
}

impl Default for Dialog {
    fn default() -> Self {
        Self::new("Dialog")
    }
}

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
    fn test_dialog_result() {
        assert!(DialogResult::Accepted.is_accepted());
        assert!(!DialogResult::Accepted.is_rejected());
        assert!(!DialogResult::Rejected.is_accepted());
        assert!(DialogResult::Rejected.is_rejected());
    }

    #[test]
    fn test_dialog_result_conversion() {
        assert_eq!(DialogResult::from(true), DialogResult::Accepted);
        assert_eq!(DialogResult::from(false), DialogResult::Rejected);
        assert!(bool::from(DialogResult::Accepted));
        assert!(!bool::from(DialogResult::Rejected));
    }

    #[test]
    fn test_dialog_creation() {
        setup();
        let dialog = Dialog::new("Test Dialog");
        assert_eq!(dialog.title(), "Test Dialog");
        assert!(dialog.modality().is_modal());
        assert!(!dialog.is_open());
    }

    #[test]
    fn test_dialog_builder_pattern() {
        setup();
        let dialog = Dialog::new("Test")
            .with_size(500.0, 300.0)
            .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL)
            .with_modality(WindowModality::WindowModal);

        assert!(dialog.standard_buttons().has(StandardButton::OK));
        assert!(dialog.standard_buttons().has(StandardButton::CANCEL));
        assert_eq!(dialog.modality(), WindowModality::WindowModal);
    }

    #[test]
    fn test_dialog_open_close() {
        setup();
        let mut dialog = Dialog::new("Test");
        assert!(!dialog.is_open());

        dialog.open();
        assert!(dialog.is_open());

        dialog.close();
        assert!(!dialog.is_open());
        assert_eq!(dialog.result(), DialogResult::Rejected);
    }

    #[test]
    fn test_dialog_accept() {
        setup();
        let mut dialog = Dialog::new("Test");
        let accepted = Arc::new(AtomicBool::new(false));
        let accepted_clone = accepted.clone();

        dialog.accepted.connect(move |()| {
            accepted_clone.store(true, Ordering::SeqCst);
        });

        dialog.open();
        dialog.accept();

        assert!(!dialog.is_open());
        assert_eq!(dialog.result(), DialogResult::Accepted);
        assert!(accepted.load(Ordering::SeqCst));
    }

    #[test]
    fn test_dialog_reject() {
        setup();
        let mut dialog = Dialog::new("Test");
        let rejected = Arc::new(AtomicBool::new(false));
        let rejected_clone = rejected.clone();

        dialog.rejected.connect(move |()| {
            rejected_clone.store(true, Ordering::SeqCst);
        });

        dialog.open();
        dialog.reject();

        assert!(!dialog.is_open());
        assert_eq!(dialog.result(), DialogResult::Rejected);
        assert!(rejected.load(Ordering::SeqCst));
    }

    #[test]
    fn test_dialog_finished_signal() {
        setup();
        let mut dialog = Dialog::new("Test");
        let result: Arc<std::sync::Mutex<Option<DialogResult>>> =
            Arc::new(std::sync::Mutex::new(None));
        let result_clone = result.clone();

        dialog.finished.connect(move |r| {
            *result_clone.lock().unwrap() = Some(*r);
        });

        dialog.open();
        dialog.accept();

        assert_eq!(*result.lock().unwrap(), Some(DialogResult::Accepted));
    }

    #[test]
    fn test_dialog_content_rect() {
        setup();
        let dialog = Dialog::new("Test").with_size(400.0, 300.0);
        let content_rect = dialog.content_rect();

        // Content rect should be inside the dialog
        assert!(content_rect.origin.x > 0.0);
        assert!(content_rect.origin.y > 0.0);
        assert!(content_rect.width() < 400.0);
        assert!(content_rect.height() < 300.0);
    }

    #[test]
    fn test_dialog_size_hint() {
        setup();
        let dialog = Dialog::new("Test");
        let hint = dialog.size_hint();

        assert!(hint.preferred.width > 100.0);
        assert!(hint.preferred.height > 50.0);
    }

    // =========================================================================
    // Auto-Default Button Tests
    // =========================================================================

    #[test]
    fn test_dialog_auto_default_activation() {
        setup();
        ModalManager::clear();

        let mut dialog = Dialog::new("Test");
        let button1_id = ObjectId::from_raw((1_u64 << 32) | 100).unwrap();
        let button2_id = ObjectId::from_raw((1_u64 << 32) | 101).unwrap();

        // Set an explicit default button
        dialog.set_default_button(Some(button1_id));
        assert_eq!(dialog.default_button(), Some(button1_id));
        assert_eq!(dialog.explicit_default_button(), Some(button1_id));
        assert!(!dialog.is_auto_default_active());

        // Activate auto-default for another button
        assert!(dialog.activate_auto_default(button2_id));
        assert_eq!(dialog.default_button(), Some(button2_id));
        assert_eq!(dialog.explicit_default_button(), Some(button1_id));
        assert!(dialog.is_auto_default_active());

        // Restore explicit default
        assert!(dialog.restore_explicit_default());
        assert_eq!(dialog.default_button(), Some(button1_id));
        assert!(!dialog.is_auto_default_active());
    }

    #[test]
    fn test_dialog_auto_default_signal() {
        setup();
        ModalManager::clear();

        let mut dialog = Dialog::new("Test");
        let button_id = ObjectId::from_raw((1_u64 << 32) | 100).unwrap();

        let changed_value = Arc::new(std::sync::Mutex::new(None));
        let changed_value_clone = changed_value.clone();

        dialog.default_button_changed.connect(move |new_default| {
            *changed_value_clone.lock().unwrap() = Some(*new_default);
        });

        dialog.activate_auto_default(button_id);

        assert_eq!(*changed_value.lock().unwrap(), Some(Some(button_id)));
    }

    #[test]
    fn test_dialog_handle_focus_for_auto_default() {
        setup();
        ModalManager::clear();

        let mut dialog = Dialog::new("Test");
        let explicit_id = ObjectId::from_raw((1_u64 << 32) | 100).unwrap();
        let auto_id = ObjectId::from_raw((1_u64 << 32) | 101).unwrap();
        let non_button_id = ObjectId::from_raw((1_u64 << 32) | 102).unwrap();

        dialog.set_default_button(Some(explicit_id));

        // Auto-default button gains focus
        assert!(dialog.handle_focus_for_auto_default(Some(auto_id), true));
        assert_eq!(dialog.default_button(), Some(auto_id));

        // Non-button gains focus - should restore explicit
        assert!(dialog.handle_focus_for_auto_default(Some(non_button_id), false));
        assert_eq!(dialog.default_button(), Some(explicit_id));
    }

    // =========================================================================
    // Modal Input Blocking Tests
    // =========================================================================

    #[test]
    fn test_dialog_modal_manager_registration() {
        setup();
        ModalManager::clear();

        let mut dialog = Dialog::new("Test");
        assert_eq!(dialog.modality(), WindowModality::ApplicationModal);

        // Before opening, no modals registered
        assert!(!ModalManager::has_modal());

        dialog.open();

        // After opening, dialog should be registered
        assert!(ModalManager::has_modal());
        assert_eq!(ModalManager::active_modal(), Some(dialog.base.object_id()));

        dialog.close();

        // After closing, dialog should be unregistered
        assert!(!ModalManager::has_modal());
    }

    #[test]
    fn test_dialog_modal_blocking() {
        setup();
        ModalManager::clear();

        let mut dialog = Dialog::new("Modal Dialog");
        let other_window_id = ObjectId::from_raw((1_u64 << 32) | 200).unwrap();

        dialog.open();

        // Other windows should be blocked
        assert!(ModalManager::is_blocked(other_window_id));

        // The dialog itself should not be blocked
        assert!(!ModalManager::is_blocked(dialog.base.object_id()));

        dialog.close();

        // After closing, nothing should be blocked
        assert!(!ModalManager::is_blocked(other_window_id));
    }

    #[test]
    fn test_non_modal_dialog_no_blocking() {
        setup();
        ModalManager::clear();

        let mut dialog = Dialog::new("Non-Modal")
            .with_modality(WindowModality::NonModal);

        let other_window_id = ObjectId::from_raw((1_u64 << 32) | 200).unwrap();

        dialog.open();

        // Non-modal dialog should not block other windows
        assert!(!ModalManager::is_blocked(other_window_id));
        assert!(!ModalManager::has_modal());

        dialog.close();
    }

    // =========================================================================
    // Focus Restoration Tests
    // =========================================================================

    #[test]
    fn test_dialog_focus_restoration_signal() {
        setup();
        ModalManager::clear();

        let mut dialog = Dialog::new("Test");
        let previous_focus_id = ObjectId::from_raw((1_u64 << 32) | 300).unwrap();

        let restore_id = Arc::new(std::sync::Mutex::new(None));
        let restore_id_clone = restore_id.clone();

        dialog.focus_restore_requested.connect(move |widget_id| {
            *restore_id_clone.lock().unwrap() = Some(*widget_id);
        });

        // Open with previous focus tracking
        dialog.open_with_focus(Some(previous_focus_id));
        assert_eq!(dialog.previously_focused_widget(), Some(previous_focus_id));

        // Close the dialog
        dialog.close();

        // Focus restore signal should have been emitted with the previous widget
        let restored = *restore_id.lock().unwrap();
        assert_eq!(restored, Some(Some(previous_focus_id)));
    }

    #[test]
    fn test_dialog_parent_window() {
        setup();
        ModalManager::clear();

        let parent_id = ObjectId::from_raw((1_u64 << 32) | 400).unwrap();

        let dialog = Dialog::new("Test")
            .with_parent_window(parent_id)
            .with_modality(WindowModality::WindowModal);

        assert_eq!(dialog.parent_window(), Some(parent_id));
        assert_eq!(dialog.modality(), WindowModality::WindowModal);
    }
}
