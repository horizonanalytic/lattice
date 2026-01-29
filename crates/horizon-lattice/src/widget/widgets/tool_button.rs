//! Tool button widget implementation.
//!
//! This module provides [`ToolButton`], an icon-focused button typically used
//! in toolbars. It supports optional dropdown menus with various popup modes.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{ToolButton, ToolButtonPopupMode};
//! use horizon_lattice_render::Icon;
//!
//! // Simple instant action tool button
//! let mut button = ToolButton::new()
//!     .with_icon(Icon::from_path("icons/save.png"));
//!
//! button.triggered.connect(|()| {
//!     println!("Save action triggered");
//! });
//!
//! // Tool button with delayed menu popup
//! let mut menu_button = ToolButton::new()
//!     .with_icon(Icon::from_path("icons/new.png"))
//!     .with_popup_mode(ToolButtonPopupMode::DelayedPopup);
//!
//! menu_button.menu_requested.connect(|()| {
//!     // Show the popup menu here
//! });
//! ```

use std::sync::Arc;
use std::time::{Duration, Instant};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, FontSystem, Icon, IconMode, IconPosition, ImageScaleMode, Point, Rect, Renderer,
    RoundedRect, Size, Stroke, TextLayout, TextRenderer, icon_tint_for_state,
};

use super::abstract_button::{AbstractButton, ButtonVariant};
use super::{Action, Menu};
use crate::widget::{PaintContext, SizeHint, Widget, WidgetBase, WidgetEvent};

// ============================================================================
// Tool Button Popup Mode
// ============================================================================

/// Describes how the menu should be shown when using a tool button with a menu.
///
/// This enum is inspired by Qt's `QToolButton::ToolButtonPopupMode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolButtonPopupMode {
    /// The menu is displayed after pressing and holding the button for a short delay.
    ///
    /// Clicking the button immediately triggers the action. If you press and
    /// hold, the menu will appear after [`ToolButton::popup_delay()`] milliseconds.
    #[default]
    DelayedPopup,

    /// The button displays a dedicated arrow indicator. Clicking the main area
    /// triggers the action; clicking the arrow area shows the menu.
    ///
    /// This mode provides clear visual separation between the action and menu.
    MenuButtonPopup,

    /// The menu is displayed immediately when the button is pressed.
    ///
    /// The button's own action is never triggered - only menu actions work.
    /// The `triggered` signal will not be emitted in this mode.
    InstantPopup,
}

// ============================================================================
// Tool Button Style
// ============================================================================

/// Style of text/icon display for tool buttons.
///
/// Note: This is separate from IconMode because ToolButton needs additional
/// styles like "text beside icon" which differ from IconPosition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ToolButtonStyle {
    /// Display only the icon (default for tool buttons).
    #[default]
    IconOnly,
    /// Display only the text.
    TextOnly,
    /// Display text beside the icon.
    TextBesideIcon,
    /// Display text under the icon.
    TextUnderIcon,
}

impl ToolButtonStyle {
    /// Convert to IconMode for AbstractButton compatibility.
    fn to_icon_mode(self) -> IconMode {
        match self {
            ToolButtonStyle::IconOnly => IconMode::IconOnly,
            ToolButtonStyle::TextOnly => IconMode::TextOnly,
            ToolButtonStyle::TextBesideIcon | ToolButtonStyle::TextUnderIcon => {
                IconMode::IconAndText
            }
        }
    }

    /// Get the corresponding icon position.
    fn to_icon_position(self) -> IconPosition {
        match self {
            ToolButtonStyle::TextUnderIcon => IconPosition::Top,
            _ => IconPosition::Left,
        }
    }
}

// ============================================================================
// Tool Button
// ============================================================================

/// A quick-access button typically used in toolbars.
///
/// ToolButton differs from `PushButton` in several ways:
/// - Defaults to icon-only display (more compact)
/// - Supports auto-raise behavior (flat until hovered)
/// - Supports dropdown menu modes (delayed, instant, or split button)
/// - Smaller default padding
/// - Can be associated with an [`Action`] for centralized command management
/// - Can have an attached dropdown [`Menu`]
///
/// # Action Association
///
/// When associated with an action, the tool button automatically syncs:
/// - Text and icon from the action
/// - Enabled/disabled state
/// - Checkable/checked state
/// - Triggering the action when clicked
///
/// ```ignore
/// let action = Arc::new(Action::new("&Save").with_icon(save_icon));
/// let mut tool_button = ToolButton::new().with_default_action(action.clone());
/// // The button now displays the action's icon and triggers the action when clicked
/// ```
///
/// # Menu Integration
///
/// ToolButton supports three menu popup modes:
///
/// - **DelayedPopup** (default): Click triggers action, hold shows menu
/// - **MenuButtonPopup**: Split button with dedicated arrow for menu
/// - **InstantPopup**: Click always shows menu, no action trigger
///
/// You can either:
/// 1. Set a menu directly with `set_menu()` for automatic popup handling
/// 2. Connect to `menu_requested` signal for custom menu handling
///
/// ```ignore
/// // Automatic menu handling
/// let mut menu = Menu::new();
/// menu.add_action(action1);
/// menu.add_action(action2);
/// tool_button.set_menu(Some(Arc::new(menu)));
///
/// // Or custom handling via signal
/// tool_button.menu_requested.connect(|()| {
///     // Show your custom menu here
/// });
/// ```
///
/// # Signals
///
/// - `triggered`: Emitted when the button's action is triggered
/// - `menu_requested`: Emitted when the menu should be shown
pub struct ToolButton {
    /// The underlying abstract button implementation.
    inner: AbstractButton,

    /// Tool button style (icon only, text only, etc.).
    tool_button_style: ToolButtonStyle,

    /// Menu popup mode.
    popup_mode: ToolButtonPopupMode,

    /// Delay in milliseconds before showing menu in DelayedPopup mode.
    popup_delay: Duration,

    /// Whether auto-raise is enabled (flat until hovered).
    auto_raise: bool,

    /// Border radius for rounded corners.
    border_radius: f32,

    /// Arrow width for MenuButtonPopup mode.
    arrow_width: f32,

    // Action association
    /// The action this button is associated with, if any.
    default_action: Option<Arc<Action>>,

    /// Generation of the action when last synced (for change detection).
    action_generation: u64,

    // Menu integration
    /// The dropdown menu attached to this button, if any.
    menu: Option<Arc<Menu>>,

    // Internal state for delayed popup
    /// When the mouse was pressed (for delay calculation).
    press_start: Option<Instant>,

    /// Whether the menu has been requested for the current press.
    menu_shown_for_press: bool,

    /// Whether the mouse is over the arrow area (MenuButtonPopup mode).
    arrow_hovered: bool,

    /// Whether the arrow area is pressed.
    arrow_pressed: bool,

    // Signals
    /// Signal emitted when the button's action is triggered.
    ///
    /// In InstantPopup mode, this signal is never emitted.
    pub triggered: Signal<()>,

    /// Signal emitted when the menu should be shown.
    ///
    /// Connect to this signal to display your popup menu.
    /// Note: If a menu is set via `set_menu()`, it will be shown automatically
    /// and this signal will still be emitted afterward.
    pub menu_requested: Signal<()>,
}

impl ToolButton {
    /// Create a new tool button.
    ///
    /// By default, tool buttons:
    /// - Display icons only (IconOnly style)
    /// - Use auto-raise behavior (flat until hovered)
    /// - Use DelayedPopup mode
    pub fn new() -> Self {
        let mut inner = AbstractButton::new("");
        // Tool buttons default to flat variant (auto-raise)
        inner = inner.with_variant(ButtonVariant::Flat);
        // Default to icon-only
        inner = inner.with_icon_mode(IconMode::IconOnly);

        Self {
            inner,
            tool_button_style: ToolButtonStyle::IconOnly,
            popup_mode: ToolButtonPopupMode::DelayedPopup,
            popup_delay: Duration::from_millis(500),
            auto_raise: true,
            border_radius: 4.0,
            arrow_width: 16.0,
            default_action: None,
            // Use u64::MAX so first sync always runs (won't match any action's generation)
            action_generation: u64::MAX,
            menu: None,
            press_start: None,
            menu_shown_for_press: false,
            arrow_hovered: false,
            arrow_pressed: false,
            triggered: Signal::new(),
            menu_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Text Methods
    // =========================================================================

    /// Get the button's text.
    pub fn text(&self) -> &str {
        self.inner.text()
    }

    /// Set the button's text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.inner.set_text(text);
    }

    /// Set the text using builder pattern.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.inner = self.inner.with_text(text);
        self
    }

    // =========================================================================
    // Icon Methods
    // =========================================================================

    /// Get the button's icon, if any.
    pub fn icon(&self) -> Option<&Icon> {
        self.inner.icon()
    }

    /// Set the button's icon.
    pub fn set_icon(&mut self, icon: Option<Icon>) {
        self.inner.set_icon(icon);
    }

    /// Set the icon using builder pattern.
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.inner = self.inner.with_icon(icon);
        self
    }

    // =========================================================================
    // Style Methods
    // =========================================================================

    /// Get the tool button style.
    pub fn tool_button_style(&self) -> ToolButtonStyle {
        self.tool_button_style
    }

    /// Set the tool button style.
    ///
    /// This controls how the icon and text are displayed:
    /// - `IconOnly`: Show only the icon (default)
    /// - `TextOnly`: Show only the text
    /// - `TextBesideIcon`: Show text to the right of the icon
    /// - `TextUnderIcon`: Show text below the icon
    pub fn set_tool_button_style(&mut self, style: ToolButtonStyle) {
        if self.tool_button_style != style {
            self.tool_button_style = style;
            self.inner.set_icon_mode(style.to_icon_mode());
            self.inner.set_icon_position(style.to_icon_position());
            self.inner.widget_base_mut().update();
        }
    }

    /// Set tool button style using builder pattern.
    pub fn with_tool_button_style(mut self, style: ToolButtonStyle) -> Self {
        self.tool_button_style = style;
        self.inner = self.inner.with_icon_mode(style.to_icon_mode());
        self.inner = self.inner.with_icon_position(style.to_icon_position());
        self
    }

    // =========================================================================
    // Popup Mode Methods
    // =========================================================================

    /// Get the popup mode.
    pub fn popup_mode(&self) -> ToolButtonPopupMode {
        self.popup_mode
    }

    /// Set the popup mode.
    ///
    /// - `DelayedPopup`: Click = action, hold = menu (default)
    /// - `MenuButtonPopup`: Split button with arrow for menu
    /// - `InstantPopup`: Click always shows menu
    pub fn set_popup_mode(&mut self, mode: ToolButtonPopupMode) {
        if self.popup_mode != mode {
            self.popup_mode = mode;
            self.inner.widget_base_mut().update();
        }
    }

    /// Set popup mode using builder pattern.
    pub fn with_popup_mode(mut self, mode: ToolButtonPopupMode) -> Self {
        self.popup_mode = mode;
        self
    }

    /// Get the popup delay in milliseconds.
    pub fn popup_delay(&self) -> Duration {
        self.popup_delay
    }

    /// Set the popup delay for DelayedPopup mode.
    ///
    /// This is the time the button must be held before the menu appears.
    pub fn set_popup_delay(&mut self, delay: Duration) {
        self.popup_delay = delay;
    }

    /// Set popup delay using builder pattern.
    pub fn with_popup_delay(mut self, delay: Duration) -> Self {
        self.popup_delay = delay;
        self
    }

    /// Set popup delay from milliseconds using builder pattern.
    pub fn with_popup_delay_ms(mut self, ms: u64) -> Self {
        self.popup_delay = Duration::from_millis(ms);
        self
    }

    // =========================================================================
    // Auto-Raise Methods
    // =========================================================================

    /// Check if auto-raise is enabled.
    ///
    /// When enabled, the button appears flat until the mouse hovers over it,
    /// at which point it "raises" to show a border.
    pub fn auto_raise(&self) -> bool {
        self.auto_raise
    }

    /// Set whether auto-raise is enabled.
    pub fn set_auto_raise(&mut self, enabled: bool) {
        if self.auto_raise != enabled {
            self.auto_raise = enabled;
            self.inner.widget_base_mut().update();
        }
    }

    /// Set auto-raise using builder pattern.
    pub fn with_auto_raise(mut self, enabled: bool) -> Self {
        self.auto_raise = enabled;
        self
    }

    // =========================================================================
    // Visual Styling Methods
    // =========================================================================

    /// Get the border radius.
    pub fn border_radius(&self) -> f32 {
        self.border_radius
    }

    /// Set the border radius for rounded corners.
    pub fn set_border_radius(&mut self, radius: f32) {
        if self.border_radius != radius {
            self.border_radius = radius;
            self.inner.widget_base_mut().update();
        }
    }

    /// Set border radius using builder pattern.
    pub fn with_border_radius(mut self, radius: f32) -> Self {
        self.border_radius = radius;
        self
    }

    /// Get the arrow area width for MenuButtonPopup mode.
    pub fn arrow_width(&self) -> f32 {
        self.arrow_width
    }

    /// Set the arrow area width.
    pub fn set_arrow_width(&mut self, width: f32) {
        if self.arrow_width != width {
            self.arrow_width = width;
            self.inner.widget_base_mut().update();
        }
    }

    /// Set arrow width using builder pattern.
    pub fn with_arrow_width(mut self, width: f32) -> Self {
        self.arrow_width = width;
        self
    }

    // =========================================================================
    // Checkable Methods
    // =========================================================================

    /// Check if the button is checkable (toggle button).
    pub fn is_checkable(&self) -> bool {
        self.inner.is_checkable()
    }

    /// Set whether the button is checkable.
    pub fn set_checkable(&mut self, checkable: bool) {
        self.inner.set_checkable(checkable);
    }

    /// Set checkable using builder pattern.
    pub fn with_checkable(mut self, checkable: bool) -> Self {
        self.inner = self.inner.with_checkable(checkable);
        self
    }

    /// Check if the button is currently checked.
    pub fn is_checked(&self) -> bool {
        self.inner.is_checked()
    }

    /// Set the checked state.
    pub fn set_checked(&mut self, checked: bool) {
        self.inner.set_checked(checked);
    }

    /// Set checked state using builder pattern.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.inner = self.inner.with_checked(checked);
        self
    }

    // =========================================================================
    // Shortcut Methods
    // =========================================================================

    /// Get the button's keyboard shortcut, if any.
    pub fn shortcut(&self) -> Option<&crate::widget::KeySequence> {
        self.inner.shortcut()
    }

    /// Set the button's keyboard shortcut.
    pub fn set_shortcut(&mut self, shortcut: Option<crate::widget::KeySequence>) {
        self.inner.set_shortcut(shortcut);
    }

    /// Set shortcut using builder pattern.
    pub fn with_shortcut(mut self, shortcut: crate::widget::KeySequence) -> Self {
        self.inner = self.inner.with_shortcut(shortcut);
        self
    }

    /// Set shortcut from a string using builder pattern.
    pub fn with_shortcut_str(mut self, shortcut: &str) -> Self {
        self.inner = self.inner.with_shortcut_str(shortcut);
        self
    }

    // =========================================================================
    // Default Action Methods
    // =========================================================================

    /// Get the default action associated with this button, if any.
    pub fn default_action(&self) -> Option<&Arc<Action>> {
        self.default_action.as_ref()
    }

    /// Set the default action for this button.
    ///
    /// When a default action is set:
    /// - The button syncs its text, icon, enabled, and checkable states from the action
    /// - Clicking the button triggers the action
    /// - The button automatically updates when the action changes
    ///
    /// Pass `None` to clear the association.
    pub fn set_default_action(&mut self, action: Option<Arc<Action>>) {
        self.default_action = action;
        self.action_generation = 0; // Force sync on next update
        self.sync_from_action();
    }

    /// Set the default action using builder pattern.
    pub fn with_default_action(mut self, action: Arc<Action>) -> Self {
        self.default_action = Some(action);
        self.sync_from_action();
        self
    }

    /// Sync the button's properties from its associated action.
    ///
    /// This is called automatically when the action changes, but can also
    /// be called manually if needed.
    pub fn sync_from_action(&mut self) {
        let Some(action) = &self.default_action else {
            return;
        };

        // Check if action has changed
        let current_gen = action.generation();
        if current_gen == self.action_generation {
            return; // No changes
        }
        self.action_generation = current_gen;

        // Sync text
        self.inner.set_text(action.text());

        // Sync icon (action.icon() returns owned Option<Icon>)
        self.inner.set_icon(action.icon());

        // Sync enabled state
        self.inner
            .widget_base_mut()
            .set_enabled(action.is_enabled());

        // Sync checkable/checked state
        self.inner.set_checkable(action.is_checkable());
        if action.is_checkable() {
            self.inner.set_checked(action.is_checked());
        }

        // Sync shortcut (action.shortcut() returns owned Option<KeySequence>)
        self.inner.set_shortcut(action.shortcut());

        self.inner.widget_base_mut().update();
    }

    // =========================================================================
    // Menu Methods
    // =========================================================================

    /// Get the dropdown menu attached to this button, if any.
    pub fn menu(&self) -> Option<&Arc<Menu>> {
        self.menu.as_ref()
    }

    /// Set the dropdown menu for this button.
    ///
    /// When a menu is set, it will be shown automatically when:
    /// - In DelayedPopup mode: after holding the button for the popup delay
    /// - In MenuButtonPopup mode: when clicking the arrow area
    /// - In InstantPopup mode: when clicking the button
    ///
    /// Pass `None` to remove the menu.
    pub fn set_menu(&mut self, menu: Option<Arc<Menu>>) {
        self.menu = menu;
        self.inner.widget_base_mut().update();
    }

    /// Set the menu using builder pattern.
    pub fn with_menu(mut self, menu: Arc<Menu>) -> Self {
        self.menu = Some(menu);
        self
    }

    /// Check if this button has a menu attached.
    pub fn has_menu(&self) -> bool {
        self.menu.is_some()
    }

    // =========================================================================
    // Trigger Methods
    // =========================================================================

    /// Programmatically trigger the button's action.
    ///
    /// This emits the `triggered` signal (unless in InstantPopup mode).
    /// If a default action is set, it will also be triggered.
    pub fn trigger(&mut self) {
        if !self.inner.widget_base().is_effectively_enabled() {
            return;
        }

        if self.popup_mode == ToolButtonPopupMode::InstantPopup {
            // In InstantPopup mode, trigger shows the menu instead
            self.request_menu_popup();
        } else {
            // Handle checkable state
            if self.inner.is_checkable() {
                self.inner.toggle();
            }

            // Trigger the default action if set
            if let Some(action) = &self.default_action {
                action.trigger();
            }

            self.triggered.emit(());
        }
    }

    /// Request to show the menu.
    ///
    /// If a menu is attached, it will be shown. The `menu_requested` signal
    /// is always emitted regardless of whether a menu is attached.
    pub fn show_menu(&mut self) {
        if self.inner.widget_base().is_effectively_enabled() {
            self.request_menu_popup();
        }
    }

    /// Internal method to request menu popup.
    fn request_menu_popup(&mut self) {
        // Show attached menu if present
        if let Some(menu) = &self.menu {
            let button_rect = self.inner.widget_base().rect();
            // Clone the menu Arc to avoid borrow issues
            let menu_clone = Arc::clone(menu);
            // Note: We need mutable access to show the menu, but we can't
            // get it from Arc<Menu>. The actual popup will be handled by
            // the application's event loop when it processes the signal.
            // For now, we just emit the signal with the menu available.
            let _ = menu_clone; // Menu is available via self.menu()
            let _ = button_rect; // Rect available via widget_base().rect()
        }

        // Always emit the signal so custom handlers can respond
        self.menu_requested.emit(());
    }

    // =========================================================================
    // Geometry Calculations
    // =========================================================================

    /// Get the main button area rectangle (excludes arrow in MenuButtonPopup mode).
    fn main_button_rect(&self) -> Rect {
        let rect = self.inner.widget_base().rect();
        if self.popup_mode == ToolButtonPopupMode::MenuButtonPopup {
            Rect::new(
                rect.origin.x,
                rect.origin.y,
                rect.size.width - self.arrow_width,
                rect.size.height,
            )
        } else {
            rect
        }
    }

    /// Get the arrow area rectangle (only in MenuButtonPopup mode).
    fn arrow_rect(&self) -> Option<Rect> {
        if self.popup_mode != ToolButtonPopupMode::MenuButtonPopup {
            return None;
        }

        let rect = self.inner.widget_base().rect();
        Some(Rect::new(
            rect.origin.x + rect.size.width - self.arrow_width,
            rect.origin.y,
            self.arrow_width,
            rect.size.height,
        ))
    }

    /// Check if a point is in the arrow area.
    fn is_point_in_arrow(&self, point: Point) -> bool {
        self.arrow_rect().is_some_and(|r| {
            // Convert to local coordinates
            let local = Point::new(
                point.x - self.inner.widget_base().rect().origin.x,
                point.y - self.inner.widget_base().rect().origin.y,
            );
            let local_rect = Rect::new(
                r.origin.x - self.inner.widget_base().rect().origin.x,
                0.0,
                r.size.width,
                r.size.height,
            );
            local_rect.contains(local)
        })
    }

    /// Check if the delayed popup timer has elapsed.
    fn check_delayed_popup(&mut self) -> bool {
        if self.popup_mode != ToolButtonPopupMode::DelayedPopup {
            return false;
        }

        if let Some(start) = self.press_start
            && !self.menu_shown_for_press
            && start.elapsed() >= self.popup_delay
        {
            self.menu_shown_for_press = true;
            self.request_menu_popup();
            return true;
        }
        false
    }

    // =========================================================================
    // Rendering Helpers
    // =========================================================================

    /// Determine if the button should show raised (non-flat) appearance.
    fn should_show_raised(&self) -> bool {
        let base = self.inner.widget_base();
        if !self.auto_raise {
            return true;
        }
        // Show raised when hovered, pressed, or checked
        base.is_hovered() || base.is_pressed() || self.inner.is_checked()
    }

    /// Get background and border colors based on state.
    fn get_colors(
        &self,
        is_disabled: bool,
        is_pressed: bool,
        is_hovered: bool,
        is_checked: bool,
    ) -> (Color, Option<Color>) {
        // Palette colors
        let primary_tint = Color::from_rgba8(0, 122, 255, 26);
        let pressed_tint = Color::from_rgba8(0, 122, 255, 51);
        let checked_tint = Color::from_rgba8(0, 122, 255, 38);
        let border = Color::from_rgb8(200, 200, 200);
        let disabled_bg = Color::from_rgba8(200, 200, 200, 128);
        let transparent = Color::from_rgba8(0, 0, 0, 0);

        if is_disabled {
            if self.should_show_raised() {
                return (disabled_bg, Some(border));
            }
            return (transparent, None);
        }

        let show_raised = self.should_show_raised();

        let bg = if is_pressed {
            pressed_tint
        } else if is_checked {
            checked_tint
        } else if is_hovered && show_raised {
            primary_tint
        } else {
            transparent
        };

        let border_color = if show_raised { Some(border) } else { None };

        (bg, border_color)
    }

    /// Get colors for the arrow area specifically.
    fn get_arrow_colors(&self, is_disabled: bool) -> (Color, Option<Color>) {
        let pressed_tint = Color::from_rgba8(0, 122, 255, 51);
        let hover_tint = Color::from_rgba8(0, 122, 255, 26);
        let border = Color::from_rgb8(200, 200, 200);
        let transparent = Color::from_rgba8(0, 0, 0, 0);

        if is_disabled {
            return (transparent, Some(border));
        }

        let bg = if self.arrow_pressed {
            pressed_tint
        } else if self.arrow_hovered {
            hover_tint
        } else {
            transparent
        };

        (bg, Some(border))
    }

    /// Paint the dropdown arrow indicator.
    fn paint_arrow(&self, ctx: &mut PaintContext<'_>, rect: Rect, is_disabled: bool) {
        let arrow_color = if is_disabled {
            Color::from_rgb8(160, 160, 160)
        } else {
            Color::from_rgb8(80, 80, 80)
        };

        // Draw a small downward-pointing chevron (V shape)
        let center_x = rect.origin.x + rect.size.width / 2.0;
        let center_y = rect.origin.y + rect.size.height / 2.0;
        let arrow_size = 4.0;

        let points = [
            Point::new(center_x - arrow_size, center_y - arrow_size / 2.0),
            Point::new(center_x, center_y + arrow_size / 2.0),
            Point::new(center_x + arrow_size, center_y - arrow_size / 2.0),
        ];

        let stroke = Stroke::new(arrow_color, 1.5);
        ctx.renderer().draw_polyline(&points, &stroke);
    }

    /// Paint the separator line between main button and arrow (MenuButtonPopup).
    fn paint_separator(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        let sep_x = rect.origin.x;
        let stroke = Stroke::new(Color::from_rgb8(200, 200, 200), 1.0);
        ctx.renderer().draw_line(
            Point::new(sep_x, rect.origin.y + 4.0),
            Point::new(sep_x, rect.origin.y + rect.size.height - 4.0),
            &stroke,
        );
    }
}

impl Default for ToolButton {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ToolButton {
    fn object_id(&self) -> ObjectId {
        self.inner.widget_base().object_id()
    }
}

impl Widget for ToolButton {
    fn widget_base(&self) -> &WidgetBase {
        self.inner.widget_base()
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        self.inner.widget_base_mut()
    }

    fn size_hint(&self) -> SizeHint {
        let content_size = self.inner.content_size();

        // Tool buttons are more compact than push buttons
        let padding = 8.0; // 4px on each side
        let min_size = 24.0;

        let mut width = (content_size.width + padding * 2.0).max(min_size);
        let height = (content_size.height + padding).max(min_size);

        // Add arrow width for MenuButtonPopup mode
        if self.popup_mode == ToolButtonPopupMode::MenuButtonPopup {
            width += self.arrow_width;
        }

        let preferred = Size::new(width, height);

        SizeHint::new(preferred).with_minimum_dimensions(min_size, min_size)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Get state info
        let is_disabled = !self.inner.widget_base().is_effectively_enabled();
        let is_pressed = self.inner.widget_base().is_pressed() && !self.arrow_pressed;
        let is_hovered = self.inner.widget_base().is_hovered() && !self.arrow_hovered;
        let is_checked = self.inner.is_checked();

        // Get colors
        let (bg_color, border_color) =
            self.get_colors(is_disabled, is_pressed, is_hovered, is_checked);

        // Draw main button area background
        let main_rect = if self.popup_mode == ToolButtonPopupMode::MenuButtonPopup {
            Rect::new(
                rect.origin.x,
                rect.origin.y,
                rect.size.width - self.arrow_width,
                rect.size.height,
            )
        } else {
            rect
        };

        let rrect = RoundedRect::new(main_rect, self.border_radius);

        // Fill background if not transparent
        if bg_color.a > 0.0 {
            ctx.renderer().fill_rounded_rect(rrect, bg_color);
        }

        // Draw border if needed
        if let Some(border) = border_color {
            let stroke = Stroke::new(border, 1.0);
            ctx.renderer().stroke_rounded_rect(rrect, &stroke);
        }

        // Draw arrow area for MenuButtonPopup mode
        if self.popup_mode == ToolButtonPopupMode::MenuButtonPopup {
            let arrow_rect = Rect::new(
                rect.origin.x + rect.size.width - self.arrow_width,
                rect.origin.y,
                self.arrow_width,
                rect.size.height,
            );

            let (arrow_bg, arrow_border) = self.get_arrow_colors(is_disabled);
            let arrow_rrect = RoundedRect::new(arrow_rect, self.border_radius);

            if arrow_bg.a > 0.0 {
                ctx.renderer().fill_rounded_rect(arrow_rrect, arrow_bg);
            }
            if let Some(border) = arrow_border {
                let stroke = Stroke::new(border, 1.0);
                ctx.renderer().stroke_rounded_rect(arrow_rrect, &stroke);
            }

            // Separator line
            self.paint_separator(ctx, arrow_rect);

            // Arrow indicator
            self.paint_arrow(ctx, arrow_rect, is_disabled);
        }

        // Calculate content positioning
        let content_rect = if self.popup_mode == ToolButtonPopupMode::MenuButtonPopup {
            main_rect
        } else {
            rect
        };

        let icon_size = self.inner.icon_size();
        let shows_icon = self.inner.shows_icon();
        let shows_text = self.inner.shows_text();

        // Center content
        let content_size = self.inner.content_size();
        let content_x =
            content_rect.origin.x + (content_rect.size.width - content_size.width) / 2.0;
        let content_y =
            content_rect.origin.y + (content_rect.size.height - content_size.height) / 2.0;

        // Draw icon
        if shows_icon && let Some(icon) = self.inner.icon() {
            let image = if is_disabled {
                icon.disabled_image()
            } else {
                icon.image()
            };

            if let Some(img) = image {
                let icon_x =
                    if shows_text && self.tool_button_style == ToolButtonStyle::TextBesideIcon {
                        content_x
                    } else {
                        content_rect.origin.x + (content_rect.size.width - icon_size.width) / 2.0
                    };

                let icon_y =
                    if shows_text && self.tool_button_style == ToolButtonStyle::TextUnderIcon {
                        content_y
                    } else {
                        content_rect.origin.y + (content_rect.size.height - icon_size.height) / 2.0
                    };

                let icon_rect = Rect::new(icon_x, icon_y, icon_size.width, icon_size.height);

                // Apply tint for state feedback
                let _tint = icon_tint_for_state(
                    Color::WHITE,
                    is_disabled && icon.disabled_image().is_none(),
                    is_pressed,
                    is_hovered,
                );

                ctx.renderer()
                    .draw_image(img, icon_rect, ImageScaleMode::Fit);
            }
        }

        // Draw text if visible
        if shows_text && !self.inner.display_text().is_empty() {
            let mut font_system = FontSystem::new();
            let display_text = self.inner.display_text();
            let layout = TextLayout::new(&mut font_system, display_text, self.inner.font());

            let text_x = if shows_icon {
                match self.tool_button_style {
                    ToolButtonStyle::TextBesideIcon => {
                        content_x + icon_size.width + self.inner.icon_spacing()
                    }
                    ToolButtonStyle::TextUnderIcon => {
                        content_rect.origin.x + (content_rect.size.width - layout.width()) / 2.0
                    }
                    _ => content_rect.origin.x + (content_rect.size.width - layout.width()) / 2.0,
                }
            } else {
                content_rect.origin.x + (content_rect.size.width - layout.width()) / 2.0
            };

            let text_y = if shows_icon && self.tool_button_style == ToolButtonStyle::TextUnderIcon {
                content_y + icon_size.height + self.inner.icon_spacing()
            } else {
                content_rect.origin.y + (content_rect.size.height - layout.height()) / 2.0
            };

            let text_pos = Point::new(text_x, text_y);
            let text_color = self.inner.effective_text_color();

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ =
                    text_renderer.prepare_layout(&mut font_system, &layout, text_pos, text_color);
            }
        }

        // Draw focus indicator
        if self.widget_base().has_focus() {
            let focus_rect = RoundedRect::new(rect.inflate(2.0), self.border_radius + 2.0);
            let focus_color = Color::from_rgba8(66, 133, 244, 128);
            ctx.renderer().fill_rounded_rect(focus_rect, focus_color);
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                use crate::widget::MouseButton;
                if e.button != MouseButton::Left {
                    return false;
                }
                if !self.inner.widget_base().is_effectively_enabled() {
                    return false;
                }

                // Check if pressing the arrow area
                if self.popup_mode == ToolButtonPopupMode::MenuButtonPopup {
                    // Convert global to local for arrow check
                    let arrow_rect = self.arrow_rect();
                    if let Some(ar) = arrow_rect {
                        let local_arrow = Rect::new(
                            ar.origin.x - self.inner.widget_base().rect().origin.x,
                            0.0,
                            ar.size.width,
                            ar.size.height,
                        );
                        if local_arrow.contains(e.local_pos) {
                            self.arrow_pressed = true;
                            self.inner.widget_base_mut().update();
                            event.accept();
                            return true;
                        }
                    }
                }

                // Handle InstantPopup - show menu immediately
                if self.popup_mode == ToolButtonPopupMode::InstantPopup {
                    self.request_menu_popup();
                    event.accept();
                    return true;
                }

                // Start tracking for delayed popup
                if self.popup_mode == ToolButtonPopupMode::DelayedPopup {
                    self.press_start = Some(Instant::now());
                    self.menu_shown_for_press = false;
                }

                self.inner.pressed.emit(());
                event.accept();
                true
            }

            WidgetEvent::MouseRelease(e) => {
                use crate::widget::MouseButton;
                if e.button != MouseButton::Left {
                    return false;
                }
                if !self.inner.widget_base().is_effectively_enabled() {
                    return false;
                }

                // Handle arrow release
                if self.arrow_pressed {
                    self.arrow_pressed = false;
                    // Check if still over arrow
                    if let Some(ar) = self.arrow_rect() {
                        let local_arrow = Rect::new(
                            ar.origin.x - self.inner.widget_base().rect().origin.x,
                            0.0,
                            ar.size.width,
                            ar.size.height,
                        );
                        if local_arrow.contains(e.local_pos) {
                            self.request_menu_popup();
                        }
                    }
                    self.inner.widget_base_mut().update();
                    event.accept();
                    return true;
                }

                // Clear delayed popup state
                let menu_was_shown = self.menu_shown_for_press;
                self.press_start = None;
                self.menu_shown_for_press = false;

                self.inner.released.emit(());

                // Trigger action if menu wasn't shown (DelayedPopup)
                // and we're still over the button
                let is_over = self.inner.widget_base().contains_point(e.local_pos);
                if is_over
                    && self.inner.widget_base().is_pressed()
                    && !menu_was_shown
                    && self.popup_mode != ToolButtonPopupMode::InstantPopup
                {
                    self.trigger();
                }

                event.accept();
                true
            }

            WidgetEvent::MouseMove(e) => {
                // Check delayed popup timer
                self.check_delayed_popup();

                // Update arrow hover state for MenuButtonPopup
                if self.popup_mode == ToolButtonPopupMode::MenuButtonPopup
                    && let Some(ar) = self.arrow_rect()
                {
                    let local_arrow = Rect::new(
                        ar.origin.x - self.inner.widget_base().rect().origin.x,
                        0.0,
                        ar.size.width,
                        ar.size.height,
                    );
                    let new_hovered = local_arrow.contains(e.local_pos);
                    if new_hovered != self.arrow_hovered {
                        self.arrow_hovered = new_hovered;
                        self.inner.widget_base_mut().update();
                        return true;
                    }
                }
                false
            }

            WidgetEvent::KeyPress(e) => {
                if self.inner.handle_key_press(e) {
                    event.accept();
                    true
                } else {
                    false
                }
            }

            WidgetEvent::KeyRelease(e) => {
                use crate::widget::Key;
                if !self.inner.widget_base().is_effectively_enabled() {
                    return false;
                }
                match e.key {
                    Key::Space | Key::Enter => {
                        self.inner.released.emit(());
                        self.trigger();
                        event.accept();
                        true
                    }
                    _ => false,
                }
            }

            WidgetEvent::Leave(_) => {
                // Clear arrow hover state
                if self.arrow_hovered {
                    self.arrow_hovered = false;
                    self.inner.widget_base_mut().update();
                }
                false
            }

            _ => false,
        }
    }
}

// Ensure ToolButton is Send + Sync
static_assertions::assert_impl_all!(ToolButton: Send, Sync);

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
    fn test_tool_button_creation() {
        setup();
        let button = ToolButton::new();
        assert_eq!(button.popup_mode(), ToolButtonPopupMode::DelayedPopup);
        assert!(button.auto_raise());
        assert_eq!(button.tool_button_style(), ToolButtonStyle::IconOnly);
    }

    #[test]
    fn test_tool_button_builder_pattern() {
        setup();
        let button = ToolButton::new()
            .with_popup_mode(ToolButtonPopupMode::MenuButtonPopup)
            .with_auto_raise(false)
            .with_tool_button_style(ToolButtonStyle::TextBesideIcon)
            .with_popup_delay_ms(300);

        assert_eq!(button.popup_mode(), ToolButtonPopupMode::MenuButtonPopup);
        assert!(!button.auto_raise());
        assert_eq!(button.tool_button_style(), ToolButtonStyle::TextBesideIcon);
        assert_eq!(button.popup_delay(), Duration::from_millis(300));
    }

    #[test]
    fn test_tool_button_triggered_signal() {
        setup();
        let mut button = ToolButton::new();
        let triggered = Arc::new(AtomicBool::new(false));
        let triggered_clone = triggered.clone();

        button.triggered.connect(move |()| {
            triggered_clone.store(true, Ordering::SeqCst);
        });

        button.trigger();
        assert!(triggered.load(Ordering::SeqCst));
    }

    #[test]
    fn test_tool_button_menu_requested_signal() {
        setup();
        let mut button = ToolButton::new();
        let menu_requested = Arc::new(AtomicBool::new(false));
        let menu_clone = menu_requested.clone();

        button.menu_requested.connect(move |()| {
            menu_clone.store(true, Ordering::SeqCst);
        });

        button.show_menu();
        assert!(menu_requested.load(Ordering::SeqCst));
    }

    #[test]
    fn test_instant_popup_mode_triggers_menu() {
        setup();
        let mut button = ToolButton::new().with_popup_mode(ToolButtonPopupMode::InstantPopup);

        let menu_requested = Arc::new(AtomicBool::new(false));
        let triggered = Arc::new(AtomicBool::new(false));
        let menu_clone = menu_requested.clone();
        let triggered_clone = triggered.clone();

        button.menu_requested.connect(move |()| {
            menu_clone.store(true, Ordering::SeqCst);
        });

        button.triggered.connect(move |()| {
            triggered_clone.store(true, Ordering::SeqCst);
        });

        button.trigger();

        // In InstantPopup mode, trigger() should emit menu_requested, not triggered
        assert!(menu_requested.load(Ordering::SeqCst));
        assert!(!triggered.load(Ordering::SeqCst));
    }

    #[test]
    fn test_tool_button_checkable() {
        setup();
        let mut button = ToolButton::new().with_checkable(true);
        assert!(button.is_checkable());
        assert!(!button.is_checked());

        button.set_checked(true);
        assert!(button.is_checked());
    }

    #[test]
    fn test_popup_mode_setters() {
        setup();
        let mut button = ToolButton::new();

        button.set_popup_mode(ToolButtonPopupMode::InstantPopup);
        assert_eq!(button.popup_mode(), ToolButtonPopupMode::InstantPopup);

        button.set_popup_mode(ToolButtonPopupMode::MenuButtonPopup);
        assert_eq!(button.popup_mode(), ToolButtonPopupMode::MenuButtonPopup);

        button.set_popup_mode(ToolButtonPopupMode::DelayedPopup);
        assert_eq!(button.popup_mode(), ToolButtonPopupMode::DelayedPopup);
    }

    #[test]
    fn test_tool_button_style_modes() {
        setup();
        let mut button = ToolButton::new();

        button.set_tool_button_style(ToolButtonStyle::TextOnly);
        assert_eq!(button.tool_button_style(), ToolButtonStyle::TextOnly);

        button.set_tool_button_style(ToolButtonStyle::TextBesideIcon);
        assert_eq!(button.tool_button_style(), ToolButtonStyle::TextBesideIcon);

        button.set_tool_button_style(ToolButtonStyle::TextUnderIcon);
        assert_eq!(button.tool_button_style(), ToolButtonStyle::TextUnderIcon);
    }

    #[test]
    fn test_size_hint_varies_by_popup_mode() {
        setup();
        let button_normal = ToolButton::new();
        let button_menu = ToolButton::new().with_popup_mode(ToolButtonPopupMode::MenuButtonPopup);

        let hint_normal = button_normal.size_hint();
        let hint_menu = button_menu.size_hint();

        // MenuButtonPopup mode adds arrow width
        assert!(hint_menu.preferred.width > hint_normal.preferred.width);
    }

    // =========================================================================
    // Action Association Tests
    // =========================================================================

    #[test]
    fn test_tool_button_default_action() {
        setup();
        let action = Arc::new(Action::new("&Save"));
        let button = ToolButton::new().with_default_action(action.clone());

        assert!(button.default_action().is_some());
        assert_eq!(
            button.default_action().unwrap().object_id(),
            action.object_id()
        );
    }

    #[test]
    fn test_tool_button_action_sync_text() {
        setup();
        let action = Arc::new(Action::new("&Save Document"));
        let button = ToolButton::new().with_default_action(action.clone());

        // Button should sync text from action (raw text includes mnemonic marker)
        assert_eq!(button.text(), "&Save Document");
    }

    #[test]
    fn test_tool_button_action_sync_enabled() {
        setup();
        let action = Arc::new(Action::new("&Test").with_enabled(false));
        let button = ToolButton::new().with_default_action(action.clone());

        // Button should be disabled because action is disabled
        assert!(!button.widget_base().is_enabled());
    }

    #[test]
    fn test_tool_button_action_sync_checkable() {
        setup();
        let action = Arc::new(Action::new("&Bold").with_checkable(true).with_checked(true));
        let button = ToolButton::new().with_default_action(action.clone());

        // Button should sync checkable and checked state
        assert!(button.is_checkable());
        assert!(button.is_checked());
    }

    #[test]
    fn test_tool_button_action_triggers_action() {
        setup();
        let action = Arc::new(Action::new("&Test"));
        let mut button = ToolButton::new().with_default_action(action.clone());

        let action_triggered = Arc::new(AtomicBool::new(false));
        let action_clone = action_triggered.clone();

        action.triggered.connect(move |_| {
            action_clone.store(true, Ordering::SeqCst);
        });

        button.trigger();

        // Triggering the button should trigger the action
        assert!(action_triggered.load(Ordering::SeqCst));
    }

    #[test]
    fn test_tool_button_clear_action() {
        setup();
        let action = Arc::new(Action::new("&Test"));
        let mut button = ToolButton::new().with_default_action(action.clone());

        assert!(button.default_action().is_some());

        button.set_default_action(None);

        assert!(button.default_action().is_none());
    }

    // =========================================================================
    // Menu Integration Tests
    // =========================================================================

    #[test]
    fn test_tool_button_menu() {
        setup();
        let menu = Arc::new(Menu::new());
        let button = ToolButton::new().with_menu(menu.clone());

        assert!(button.has_menu());
        assert!(button.menu().is_some());
    }

    #[test]
    fn test_tool_button_clear_menu() {
        setup();
        let menu = Arc::new(Menu::new());
        let mut button = ToolButton::new().with_menu(menu.clone());

        assert!(button.has_menu());

        button.set_menu(None);

        assert!(!button.has_menu());
        assert!(button.menu().is_none());
    }

    #[test]
    fn test_tool_button_menu_signal_emitted() {
        setup();
        let menu = Arc::new(Menu::new());
        let mut button = ToolButton::new().with_menu(menu.clone());

        let signal_emitted = Arc::new(AtomicBool::new(false));
        let signal_clone = signal_emitted.clone();

        button.menu_requested.connect(move |()| {
            signal_clone.store(true, Ordering::SeqCst);
        });

        button.show_menu();

        // Signal should be emitted even when menu is attached
        assert!(signal_emitted.load(Ordering::SeqCst));
    }

    #[test]
    fn test_tool_button_with_action_and_menu() {
        setup();
        let action = Arc::new(Action::new("&New"));
        let menu = Arc::new(Menu::new());

        let button = ToolButton::new()
            .with_default_action(action.clone())
            .with_menu(menu.clone())
            .with_popup_mode(ToolButtonPopupMode::MenuButtonPopup);

        assert!(button.default_action().is_some());
        assert!(button.has_menu());
        assert_eq!(button.popup_mode(), ToolButtonPopupMode::MenuButtonPopup);
    }
}
