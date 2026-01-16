//! Dialog button box widget implementation.
//!
//! This module provides [`DialogButtonBox`], a container for managing standard
//! dialog buttons with proper roles and platform-specific button ordering.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{DialogButtonBox, StandardButton};
//!
//! // Create a button box with standard OK and Cancel buttons
//! let mut button_box = DialogButtonBox::new()
//!     .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL);
//!
//! // Connect to signals
//! button_box.accepted.connect(|()| {
//!     println!("Dialog accepted");
//! });
//!
//! button_box.rejected.connect(|()| {
//!     println!("Dialog rejected");
//! });
//! ```

use std::ops::{BitAnd, BitOr, BitOrAssign};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Rect, Renderer, Size};

use crate::widget::layout::ContentMargins;
use crate::widget::{
    FocusPolicy, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

use super::push_button::PushButton;
use super::abstract_button::ButtonVariant;

// ============================================================================
// Standard Buttons
// ============================================================================

/// Standard buttons that can be used in a [`DialogButtonBox`].
///
/// These flags can be combined using bitwise OR operations.
///
/// # Example
///
/// ```ignore
/// let buttons = StandardButton::OK | StandardButton::CANCEL;
/// let yes_no = StandardButton::YES | StandardButton::NO;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StandardButton(u32);

impl StandardButton {
    /// No buttons.
    pub const NONE: StandardButton = StandardButton(0);

    /// An "OK" button.
    pub const OK: StandardButton = StandardButton(1 << 0);

    /// A "Cancel" button.
    pub const CANCEL: StandardButton = StandardButton(1 << 1);

    /// A "Yes" button.
    pub const YES: StandardButton = StandardButton(1 << 2);

    /// A "No" button.
    pub const NO: StandardButton = StandardButton(1 << 3);

    /// An "Apply" button.
    pub const APPLY: StandardButton = StandardButton(1 << 4);

    /// A "Close" button.
    pub const CLOSE: StandardButton = StandardButton(1 << 5);

    /// A "Help" button.
    pub const HELP: StandardButton = StandardButton(1 << 6);

    /// A "Save" button.
    pub const SAVE: StandardButton = StandardButton(1 << 7);

    /// A "Discard" / "Don't Save" button.
    pub const DISCARD: StandardButton = StandardButton(1 << 8);

    /// A "Reset" button.
    pub const RESET: StandardButton = StandardButton(1 << 9);

    /// A "Restore Defaults" button.
    pub const RESTORE_DEFAULTS: StandardButton = StandardButton(1 << 10);

    /// An "Abort" button.
    pub const ABORT: StandardButton = StandardButton(1 << 11);

    /// A "Retry" button.
    pub const RETRY: StandardButton = StandardButton(1 << 12);

    /// An "Ignore" button.
    pub const IGNORE: StandardButton = StandardButton(1 << 13);

    /// A "Save All" button.
    pub const SAVE_ALL: StandardButton = StandardButton(1 << 14);

    /// A "Yes to All" button.
    pub const YES_TO_ALL: StandardButton = StandardButton(1 << 15);

    /// A "No to All" button.
    pub const NO_TO_ALL: StandardButton = StandardButton(1 << 16);

    /// An "Open" button.
    pub const OPEN: StandardButton = StandardButton(1 << 17);

    /// Check if a button flag is set.
    pub fn has(&self, button: StandardButton) -> bool {
        (self.0 & button.0) == button.0 && button.0 != 0
    }

    /// Check if no buttons are set.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Get the display text for this standard button.
    pub fn text(&self) -> &'static str {
        match self.0 {
            x if x == Self::OK.0 => "&OK",
            x if x == Self::CANCEL.0 => "&Cancel",
            x if x == Self::YES.0 => "&Yes",
            x if x == Self::NO.0 => "&No",
            x if x == Self::APPLY.0 => "&Apply",
            x if x == Self::CLOSE.0 => "&Close",
            x if x == Self::HELP.0 => "&Help",
            x if x == Self::SAVE.0 => "&Save",
            x if x == Self::DISCARD.0 => "&Discard",
            x if x == Self::RESET.0 => "Re&set",
            x if x == Self::RESTORE_DEFAULTS.0 => "Restore &Defaults",
            x if x == Self::ABORT.0 => "&Abort",
            x if x == Self::RETRY.0 => "&Retry",
            x if x == Self::IGNORE.0 => "&Ignore",
            x if x == Self::SAVE_ALL.0 => "Save &All",
            x if x == Self::YES_TO_ALL.0 => "Yes to &All",
            x if x == Self::NO_TO_ALL.0 => "N&o to All",
            x if x == Self::OPEN.0 => "&Open",
            _ => "",
        }
    }

    /// Get the button role for this standard button.
    pub fn role(&self) -> ButtonRole {
        match self.0 {
            x if x == Self::OK.0 => ButtonRole::Accept,
            x if x == Self::CANCEL.0 => ButtonRole::Reject,
            x if x == Self::YES.0 => ButtonRole::Accept,
            x if x == Self::NO.0 => ButtonRole::Reject,
            x if x == Self::APPLY.0 => ButtonRole::Apply,
            x if x == Self::CLOSE.0 => ButtonRole::Reject,
            x if x == Self::HELP.0 => ButtonRole::Help,
            x if x == Self::SAVE.0 => ButtonRole::Accept,
            x if x == Self::DISCARD.0 => ButtonRole::Destructive,
            x if x == Self::RESET.0 => ButtonRole::Reset,
            x if x == Self::RESTORE_DEFAULTS.0 => ButtonRole::Reset,
            x if x == Self::ABORT.0 => ButtonRole::Reject,
            x if x == Self::RETRY.0 => ButtonRole::Accept,
            x if x == Self::IGNORE.0 => ButtonRole::Accept,
            x if x == Self::SAVE_ALL.0 => ButtonRole::Accept,
            x if x == Self::YES_TO_ALL.0 => ButtonRole::Accept,
            x if x == Self::NO_TO_ALL.0 => ButtonRole::Reject,
            x if x == Self::OPEN.0 => ButtonRole::Accept,
            _ => ButtonRole::Invalid,
        }
    }

    /// Get an iterator over all button flags that are set.
    pub fn iter(&self) -> impl Iterator<Item = StandardButton> {
        let value = self.0;
        (0..18).filter_map(move |i| {
            let flag = 1u32 << i;
            if (value & flag) != 0 {
                Some(StandardButton(flag))
            } else {
                None
            }
        })
    }
}

impl BitOr for StandardButton {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        StandardButton(self.0 | rhs.0)
    }
}

impl BitOrAssign for StandardButton {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for StandardButton {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        StandardButton(self.0 & rhs.0)
    }
}

// ============================================================================
// Button Role
// ============================================================================

/// The role of a button in a dialog.
///
/// Button roles determine how buttons are ordered in the dialog and
/// what action they trigger when clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonRole {
    /// Invalid role - button does nothing special.
    #[default]
    Invalid,

    /// Clicking the button causes the dialog to be accepted (e.g., OK, Yes, Save).
    Accept,

    /// Clicking the button causes the dialog to be rejected (e.g., Cancel, No, Close).
    Reject,

    /// Clicking the button causes a destructive action (e.g., Discard).
    Destructive,

    /// Clicking the button applies changes without closing the dialog.
    Apply,

    /// Clicking the button resets the dialog's fields to default values.
    Reset,

    /// Clicking the button opens help information.
    Help,

    /// Clicking the button performs a custom action.
    Action,
}

impl ButtonRole {
    /// Check if this role accepts the dialog.
    pub fn is_accept(&self) -> bool {
        matches!(self, ButtonRole::Accept)
    }

    /// Check if this role rejects the dialog.
    pub fn is_reject(&self) -> bool {
        matches!(self, ButtonRole::Reject | ButtonRole::Destructive)
    }
}

// ============================================================================
// Button Layout Order
// ============================================================================

/// Button layout orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonBoxOrientation {
    /// Buttons arranged horizontally (default).
    #[default]
    Horizontal,
    /// Buttons arranged vertically.
    Vertical,
}

/// Platform-specific button ordering convention.
///
/// Different platforms have different conventions for ordering dialog buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonOrder {
    /// Windows/KDE style: Help | Stretch | Accept | Reject | Apply
    Windows,
    /// macOS style: Help | Stretch | Reject | Accept | Apply
    MacOS,
    /// GNOME/GTK style: Help | Stretch | Reject | Accept | Apply (same as macOS)
    Gnome,
}

impl Default for ButtonOrder {
    fn default() -> Self {
        // Default to Windows style; could be made platform-specific
        #[cfg(target_os = "macos")]
        return ButtonOrder::MacOS;

        #[cfg(not(target_os = "macos"))]
        ButtonOrder::Windows
    }
}

impl ButtonOrder {
    /// Get the sorting key for a button role.
    ///
    /// Lower values appear first (left in horizontal layout).
    pub fn sort_key(&self, role: ButtonRole) -> i32 {
        match self {
            ButtonOrder::Windows => match role {
                ButtonRole::Help => 0,
                ButtonRole::Reset => 1,
                ButtonRole::Accept => 100,
                ButtonRole::Reject => 101,
                ButtonRole::Destructive => 102,
                ButtonRole::Apply => 200,
                ButtonRole::Action => 50,
                ButtonRole::Invalid => 999,
            },
            ButtonOrder::MacOS | ButtonOrder::Gnome => match role {
                ButtonRole::Help => 0,
                ButtonRole::Reset => 1,
                ButtonRole::Reject => 100,
                ButtonRole::Destructive => 101,
                ButtonRole::Accept => 102,
                ButtonRole::Apply => 200,
                ButtonRole::Action => 50,
                ButtonRole::Invalid => 999,
            },
        }
    }
}

// ============================================================================
// Button Info
// ============================================================================

/// Information about a button in the button box.
#[derive(Debug, Clone)]
struct ButtonInfo {
    /// The button's object ID.
    button_id: ObjectId,
    /// The button's role.
    role: ButtonRole,
    /// The standard button type (if any).
    standard_button: Option<StandardButton>,
}

// ============================================================================
// Dialog Button Box
// ============================================================================

/// A container for managing dialog buttons.
///
/// `DialogButtonBox` provides a standardized way to handle dialog buttons
/// with proper roles and platform-specific button ordering. It automatically
/// arranges buttons according to the platform's conventions.
///
/// # Features
///
/// - Standard button presets (OK, Cancel, Yes, No, etc.)
/// - Platform-specific button ordering
/// - Automatic signal connection for accept/reject roles
/// - Custom button support
///
/// # Signals
///
/// - `accepted()`: Emitted when a button with Accept role is clicked
/// - `rejected()`: Emitted when a button with Reject role is clicked
/// - `clicked(StandardButton)`: Emitted when any standard button is clicked
/// - `help_requested()`: Emitted when the Help button is clicked
pub struct DialogButtonBox {
    /// Widget base.
    base: WidgetBase,

    /// Layout orientation.
    orientation: ButtonBoxOrientation,

    /// Button ordering convention.
    button_order: ButtonOrder,

    /// Standard buttons to display.
    standard_buttons: StandardButton,

    /// Information about buttons in the box.
    buttons: Vec<ButtonInfo>,

    /// Spacing between buttons.
    spacing: f32,

    /// Content margins.
    margins: ContentMargins,

    /// Whether to center reject buttons between help and accept.
    center_buttons: bool,

    /// Background color.
    background_color: Color,

    // Signals
    /// Signal emitted when a button with Accept role is clicked.
    pub accepted: Signal<()>,
    /// Signal emitted when a button with Reject role is clicked.
    pub rejected: Signal<()>,
    /// Signal emitted when any standard button is clicked.
    pub clicked: Signal<StandardButton>,
    /// Signal emitted when the Help button is clicked.
    pub help_requested: Signal<()>,
}

impl DialogButtonBox {
    /// Create a new dialog button box.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::NoFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Fixed));

        Self {
            base,
            orientation: ButtonBoxOrientation::Horizontal,
            button_order: ButtonOrder::default(),
            standard_buttons: StandardButton::NONE,
            buttons: Vec::new(),
            spacing: 8.0,
            margins: ContentMargins::uniform(8.0),
            center_buttons: false,
            background_color: Color::TRANSPARENT,
            accepted: Signal::new(),
            rejected: Signal::new(),
            clicked: Signal::new(),
            help_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the standard buttons using builder pattern.
    pub fn with_standard_buttons(mut self, buttons: StandardButton) -> Self {
        self.standard_buttons = buttons;
        self.rebuild_buttons();
        self
    }

    /// Set the orientation using builder pattern.
    pub fn with_orientation(mut self, orientation: ButtonBoxOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Set the button order using builder pattern.
    pub fn with_button_order(mut self, order: ButtonOrder) -> Self {
        self.button_order = order;
        self
    }

    /// Set the spacing using builder pattern.
    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set the margins using builder pattern.
    pub fn with_margins(mut self, margins: ContentMargins) -> Self {
        self.margins = margins;
        self
    }

    /// Set whether to center buttons using builder pattern.
    pub fn with_center_buttons(mut self, center: bool) -> Self {
        self.center_buttons = center;
        self
    }

    /// Set the background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = color;
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
    ///
    /// This clears any existing buttons and creates new buttons based
    /// on the specified flags.
    pub fn set_standard_buttons(&mut self, buttons: StandardButton) {
        if self.standard_buttons != buttons {
            self.standard_buttons = buttons;
            self.rebuild_buttons();
            self.base.update();
        }
    }

    /// Add a standard button.
    pub fn add_standard_button(&mut self, button: StandardButton) {
        if !self.standard_buttons.has(button) {
            self.standard_buttons |= button;
            self.rebuild_buttons();
            self.base.update();
        }
    }

    /// Remove a standard button.
    pub fn remove_standard_button(&mut self, button: StandardButton) {
        if self.standard_buttons.has(button) {
            self.standard_buttons = StandardButton(self.standard_buttons.0 & !button.0);
            self.rebuild_buttons();
            self.base.update();
        }
    }

    /// Create a button widget for a standard button.
    fn create_standard_button(&self, button: StandardButton) -> PushButton {
        let mut btn = PushButton::new(button.text());

        // Apply variant based on role
        let variant = match button.role() {
            ButtonRole::Accept => ButtonVariant::Primary,
            ButtonRole::Reject => ButtonVariant::Secondary,
            ButtonRole::Destructive => ButtonVariant::Danger,
            ButtonRole::Apply | ButtonRole::Reset => ButtonVariant::Outlined,
            ButtonRole::Help | ButtonRole::Action | ButtonRole::Invalid => ButtonVariant::Flat,
        };
        btn.set_variant(variant);

        btn
    }

    /// Rebuild the buttons list based on standard buttons.
    fn rebuild_buttons(&mut self) {
        self.buttons.clear();

        for button in self.standard_buttons.iter() {
            let btn = self.create_standard_button(button);
            let button_id = btn.object_id();

            self.buttons.push(ButtonInfo {
                button_id,
                role: button.role(),
                standard_button: Some(button),
            });
        }

        // Sort buttons by platform-specific order
        self.buttons.sort_by_key(|b| self.button_order.sort_key(b.role));
    }

    // =========================================================================
    // Custom Buttons
    // =========================================================================

    /// Add a custom button with a specific role.
    ///
    /// Returns the ObjectId of the created button.
    pub fn add_button(&mut self, text: &str, role: ButtonRole) -> ObjectId {
        let mut btn = PushButton::new(text);

        // Apply variant based on role
        let variant = match role {
            ButtonRole::Accept => ButtonVariant::Primary,
            ButtonRole::Reject => ButtonVariant::Secondary,
            ButtonRole::Destructive => ButtonVariant::Danger,
            ButtonRole::Apply | ButtonRole::Reset => ButtonVariant::Outlined,
            ButtonRole::Help | ButtonRole::Action | ButtonRole::Invalid => ButtonVariant::Flat,
        };
        btn.set_variant(variant);

        let button_id = btn.object_id();

        self.buttons.push(ButtonInfo {
            button_id,
            role,
            standard_button: None,
        });

        // Re-sort buttons
        self.buttons.sort_by_key(|b| self.button_order.sort_key(b.role));

        self.base.update();
        button_id
    }

    /// Remove a custom button by its ObjectId.
    pub fn remove_button(&mut self, button_id: ObjectId) {
        self.buttons.retain(|b| b.button_id != button_id);
        self.base.update();
    }

    // =========================================================================
    // Button Access
    // =========================================================================

    /// Get the button IDs in display order.
    pub fn button_ids(&self) -> Vec<ObjectId> {
        self.buttons.iter().map(|b| b.button_id).collect()
    }

    /// Get the button ObjectId for a standard button.
    pub fn button(&self, standard_button: StandardButton) -> Option<ObjectId> {
        self.buttons
            .iter()
            .find(|b| b.standard_button == Some(standard_button))
            .map(|b| b.button_id)
    }

    /// Get the role of a button by its ObjectId.
    pub fn button_role(&self, button_id: ObjectId) -> ButtonRole {
        self.buttons
            .iter()
            .find(|b| b.button_id == button_id)
            .map(|b| b.role)
            .unwrap_or(ButtonRole::Invalid)
    }

    /// Get the standard button type for a button ObjectId.
    pub fn standard_button_for_id(&self, button_id: ObjectId) -> StandardButton {
        self.buttons
            .iter()
            .find(|b| b.button_id == button_id)
            .and_then(|b| b.standard_button)
            .unwrap_or(StandardButton::NONE)
    }

    // =========================================================================
    // Orientation & Ordering
    // =========================================================================

    /// Get the orientation.
    pub fn orientation(&self) -> ButtonBoxOrientation {
        self.orientation
    }

    /// Set the orientation.
    pub fn set_orientation(&mut self, orientation: ButtonBoxOrientation) {
        if self.orientation != orientation {
            self.orientation = orientation;
            self.base.update();
        }
    }

    /// Get the button order.
    pub fn button_order(&self) -> ButtonOrder {
        self.button_order
    }

    /// Set the button order.
    pub fn set_button_order(&mut self, order: ButtonOrder) {
        if self.button_order != order {
            self.button_order = order;
            // Re-sort buttons
            self.buttons.sort_by_key(|b| self.button_order.sort_key(b.role));
            self.base.update();
        }
    }

    // =========================================================================
    // Spacing & Margins
    // =========================================================================

    /// Get the spacing between buttons.
    pub fn spacing(&self) -> f32 {
        self.spacing
    }

    /// Set the spacing between buttons.
    pub fn set_spacing(&mut self, spacing: f32) {
        if (self.spacing - spacing).abs() > f32::EPSILON {
            self.spacing = spacing;
            self.base.update();
        }
    }

    /// Get the content margins.
    pub fn margins(&self) -> ContentMargins {
        self.margins
    }

    /// Set the content margins.
    pub fn set_margins(&mut self, margins: ContentMargins) {
        if self.margins != margins {
            self.margins = margins;
            self.base.update();
        }
    }

    // =========================================================================
    // Click Handling
    // =========================================================================

    /// Handle a button click by its ObjectId.
    ///
    /// This emits the appropriate signals based on the button's role.
    pub fn handle_button_click(&mut self, button_id: ObjectId) {
        let Some(info) = self.buttons.iter().find(|b| b.button_id == button_id) else {
            return;
        };

        // Emit the standard button clicked signal
        if let Some(std_button) = info.standard_button {
            self.clicked.emit(std_button);

            // Emit help signal for help button
            if std_button == StandardButton::HELP {
                self.help_requested.emit(());
            }
        }

        // Emit role-based signals
        match info.role {
            ButtonRole::Accept => self.accepted.emit(()),
            ButtonRole::Reject | ButtonRole::Destructive => self.rejected.emit(()),
            _ => {}
        }
    }

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    /// Calculate the content rect (inside margins).
    fn content_rect(&self) -> Rect {
        let rect = self.base.rect();
        Rect::new(
            self.margins.left,
            self.margins.top,
            rect.width() - self.margins.horizontal(),
            rect.height() - self.margins.vertical(),
        )
    }

    /// Calculate button layout positions.
    ///
    /// Returns Vec of (button_id, rect) pairs.
    pub fn calculate_button_rects(&self, button_size: Size) -> Vec<(ObjectId, Rect)> {
        let content_rect = self.content_rect();
        let mut result = Vec::new();

        if self.buttons.is_empty() {
            return result;
        }

        let total_buttons = self.buttons.len();
        let total_spacing = self.spacing * (total_buttons.saturating_sub(1)) as f32;

        match self.orientation {
            ButtonBoxOrientation::Horizontal => {
                let total_width = button_size.width * total_buttons as f32 + total_spacing;

                // Start from the right side (standard dialog convention)
                let start_x = content_rect.origin.x + content_rect.width() - total_width;
                let y = content_rect.origin.y
                    + (content_rect.height() - button_size.height) / 2.0;

                for (i, info) in self.buttons.iter().enumerate() {
                    let x = start_x + (button_size.width + self.spacing) * i as f32;
                    result.push((
                        info.button_id,
                        Rect::new(x, y, button_size.width, button_size.height),
                    ));
                }
            }
            ButtonBoxOrientation::Vertical => {
                let total_height = button_size.height * total_buttons as f32 + total_spacing;

                // Center horizontally
                let x = content_rect.origin.x
                    + (content_rect.width() - button_size.width) / 2.0;
                let start_y = content_rect.origin.y
                    + (content_rect.height() - total_height) / 2.0;

                for (i, info) in self.buttons.iter().enumerate() {
                    let y = start_y + (button_size.height + self.spacing) * i as f32;
                    result.push((
                        info.button_id,
                        Rect::new(x, y, button_size.width, button_size.height),
                    ));
                }
            }
        }

        result
    }
}

impl Object for DialogButtonBox {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for DialogButtonBox {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Calculate preferred size based on buttons
        let button_count = self.buttons.len().max(1);
        let button_size = Size::new(80.0, 32.0);

        let (width, height) = match self.orientation {
            ButtonBoxOrientation::Horizontal => {
                let w = button_size.width * button_count as f32
                    + self.spacing * (button_count.saturating_sub(1)) as f32
                    + self.margins.horizontal();
                let h = button_size.height + self.margins.vertical();
                (w, h)
            }
            ButtonBoxOrientation::Vertical => {
                let w = button_size.width + self.margins.horizontal();
                let h = button_size.height * button_count as f32
                    + self.spacing * (button_count.saturating_sub(1)) as f32
                    + self.margins.vertical();
                (w, h)
            }
        };

        let preferred = Size::new(width, height);
        let minimum = Size::new(80.0, button_size.height + self.margins.vertical());

        SizeHint::new(preferred).with_minimum(minimum)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Draw background if set
        if self.background_color.a > 0.0 {
            ctx.renderer().fill_rect(rect, self.background_color);
        }

        // Note: The actual buttons are painted separately as child widgets.
        // This widget just provides layout and signal management.
    }

    fn event(&mut self, _event: &mut WidgetEvent) -> bool {
        // Button box itself doesn't handle events - the buttons do
        false
    }
}

impl Default for DialogButtonBox {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_standard_button_flags() {
        let buttons = StandardButton::OK | StandardButton::CANCEL;
        assert!(buttons.has(StandardButton::OK));
        assert!(buttons.has(StandardButton::CANCEL));
        assert!(!buttons.has(StandardButton::YES));
    }

    #[test]
    fn test_standard_button_text() {
        assert_eq!(StandardButton::OK.text(), "&OK");
        assert_eq!(StandardButton::CANCEL.text(), "&Cancel");
        assert_eq!(StandardButton::YES.text(), "&Yes");
        assert_eq!(StandardButton::NO.text(), "&No");
    }

    #[test]
    fn test_standard_button_roles() {
        assert!(StandardButton::OK.role().is_accept());
        assert!(StandardButton::YES.role().is_accept());
        assert!(StandardButton::SAVE.role().is_accept());

        assert!(StandardButton::CANCEL.role().is_reject());
        assert!(StandardButton::NO.role().is_reject());
        assert!(StandardButton::CLOSE.role().is_reject());

        assert_eq!(StandardButton::DISCARD.role(), ButtonRole::Destructive);
        assert_eq!(StandardButton::HELP.role(), ButtonRole::Help);
    }

    #[test]
    fn test_standard_button_iter() {
        let buttons = StandardButton::OK | StandardButton::CANCEL | StandardButton::HELP;
        let collected: Vec<_> = buttons.iter().collect();

        assert_eq!(collected.len(), 3);
        assert!(collected.contains(&StandardButton::OK));
        assert!(collected.contains(&StandardButton::CANCEL));
        assert!(collected.contains(&StandardButton::HELP));
    }

    #[test]
    fn test_button_role_is_accept() {
        assert!(ButtonRole::Accept.is_accept());
        assert!(!ButtonRole::Reject.is_accept());
        assert!(!ButtonRole::Help.is_accept());
    }

    #[test]
    fn test_button_role_is_reject() {
        assert!(ButtonRole::Reject.is_reject());
        assert!(ButtonRole::Destructive.is_reject());
        assert!(!ButtonRole::Accept.is_reject());
    }

    #[test]
    fn test_button_order_windows() {
        let order = ButtonOrder::Windows;
        // Help comes first
        assert!(order.sort_key(ButtonRole::Help) < order.sort_key(ButtonRole::Accept));
        // Accept comes before Reject
        assert!(order.sort_key(ButtonRole::Accept) < order.sort_key(ButtonRole::Reject));
    }

    #[test]
    fn test_button_order_macos() {
        let order = ButtonOrder::MacOS;
        // Help comes first
        assert!(order.sort_key(ButtonRole::Help) < order.sort_key(ButtonRole::Reject));
        // Reject comes before Accept (macOS style)
        assert!(order.sort_key(ButtonRole::Reject) < order.sort_key(ButtonRole::Accept));
    }

    #[test]
    fn test_dialog_button_box_creation() {
        setup();
        let button_box = DialogButtonBox::new();
        assert!(button_box.standard_buttons().is_empty());
        assert_eq!(button_box.orientation(), ButtonBoxOrientation::Horizontal);
    }

    #[test]
    fn test_dialog_button_box_with_standard_buttons() {
        setup();
        let button_box = DialogButtonBox::new()
            .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL);

        assert!(button_box.standard_buttons().has(StandardButton::OK));
        assert!(button_box.standard_buttons().has(StandardButton::CANCEL));
    }

    #[test]
    fn test_dialog_button_box_size_hint() {
        setup();
        let button_box = DialogButtonBox::new()
            .with_standard_buttons(StandardButton::OK | StandardButton::CANCEL);

        let hint = button_box.size_hint();
        assert!(hint.preferred.width > 100.0);
        assert!(hint.preferred.height > 30.0);
    }
}
