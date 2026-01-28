//! Action system for menus, toolbars, and keyboard shortcuts.
//!
//! This module provides [`Action`], a non-visual object that represents a user command.
//! Actions can be added to menus, toolbars, and assigned keyboard shortcuts. They provide
//! a central point for managing command properties like text, icon, and enabled state.
//!
//! # Overview
//!
//! An `Action` encapsulates:
//! - Text label (with optional mnemonic using `&`)
//! - Icon for visual representation
//! - Keyboard shortcut for direct activation
//! - Tooltip for user guidance
//! - Enabled/disabled state
//! - Checkable/toggle behavior
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Action, ShortcutContext};
//! use horizon_lattice::widget::KeySequence;
//!
//! // Create a simple action
//! let save_action = Action::new("&Save")
//!     .with_shortcut("Ctrl+S".parse().unwrap())
//!     .with_tooltip("Save the current document");
//!
//! // Connect to the triggered signal
//! save_action.triggered.connect(|_| {
//!     println!("Save triggered!");
//! });
//!
//! // Create a checkable action (toggle)
//! let bold_action = Action::new("&Bold")
//!     .with_checkable(true)
//!     .with_shortcut("Ctrl+B".parse().unwrap());
//!
//! bold_action.toggled.connect(|&checked| {
//!     println!("Bold is now: {}", if checked { "on" } else { "off" });
//! });
//! ```
//!
//! # Shortcut Context
//!
//! The [`ShortcutContext`] determines when a shortcut is active:
//!
//! - **Widget**: Only when the action's associated widget has focus
//! - **Window**: Active anywhere within the window
//! - **Application**: Global shortcut across all windows
//!
//! # Menu Roles (macOS)
//!
//! On macOS, certain actions have special meaning and are placed in standard
//! locations (like the application menu). Use [`MenuRole`] to indicate these:
//!
//! - `AboutRole`: "About" menu item (application menu)
//! - `PreferencesRole`: "Preferences" (application menu)
//! - `QuitRole`: "Quit" (application menu)
//! - etc.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use horizon_lattice_core::{Object, ObjectBase, ObjectId, Signal};
use horizon_lattice_render::Icon;
use parking_lot::RwLock;

use crate::widget::shortcut::{parse_mnemonic, KeySequence, MnemonicText};

// ============================================================================
// Shortcut Context
// ============================================================================

/// Determines the scope in which a keyboard shortcut is active.
///
/// This controls when the action's shortcut can be triggered based on
/// the current focus and window state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ShortcutContext {
    /// Shortcut is only active when the widget associated with the action has focus.
    ///
    /// Use this for widget-specific commands that shouldn't interfere with
    /// other widgets that might use the same shortcut.
    Widget,

    /// Shortcut is active anywhere within the parent window (default).
    ///
    /// This is the most common setting for application commands like Save, Open, etc.
    /// The shortcut works regardless of which widget has focus, as long as the
    /// window is active.
    #[default]
    Window,

    /// Shortcut is active globally across all windows in the application.
    ///
    /// Use sparingly, as this can conflict with system-wide shortcuts or
    /// shortcuts in other windows.
    Application,

    /// Shortcut is active in the window and all its child windows.
    ///
    /// Similar to Window, but also triggers in modal dialogs and other
    /// child windows.
    WindowWithChildren,
}

impl ShortcutContext {
    /// Check if this context requires widget focus.
    pub fn requires_focus(self) -> bool {
        matches!(self, ShortcutContext::Widget)
    }

    /// Check if this context is window-scoped.
    pub fn is_window_scoped(self) -> bool {
        matches!(
            self,
            ShortcutContext::Window | ShortcutContext::WindowWithChildren
        )
    }

    /// Check if this context is application-wide.
    pub fn is_application_wide(self) -> bool {
        matches!(self, ShortcutContext::Application)
    }
}

// ============================================================================
// Menu Role
// ============================================================================

/// Special menu roles for platform integration.
///
/// On macOS, certain menu items have special meaning and are automatically
/// placed in standard locations (like the application menu). On other platforms,
/// these may affect behavior or appearance.
///
/// # macOS Behavior
///
/// - `AboutRole`: Placed in application menu as "About {AppName}"
/// - `PreferencesRole`: Placed in application menu as "Preferences..."
/// - `QuitRole`: Placed in application menu as "Quit {AppName}"
/// - `HideRole`: Placed in application menu as "Hide {AppName}"
/// - `HideOthersRole`: Placed in application menu as "Hide Others"
/// - `ShowAllRole`: Placed in application menu as "Show All"
/// - `MinimizeRole`: Placed in Window menu as "Minimize"
/// - `ZoomRole`: Placed in Window menu as "Zoom"
/// - `FullScreenRole`: Placed in View menu as "Enter Full Screen"
/// - `BringAllToFrontRole`: Placed in Window menu as "Bring All to Front"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MenuRole {
    /// No special role (default).
    ///
    /// The action appears where it is added, with no special platform handling.
    #[default]
    NoRole,

    /// About application menu item.
    ///
    /// On macOS, this is moved to the application menu as "About {AppName}".
    /// The action's text may be replaced with the standard text.
    AboutRole,

    /// Application preferences/settings.
    ///
    /// On macOS, this is moved to the application menu as "Preferences..."
    /// or "Settings..." depending on the OS version.
    PreferencesRole,

    /// Quit/exit application.
    ///
    /// On macOS, this is moved to the application menu as "Quit {AppName}".
    /// The standard Cmd+Q shortcut is typically used.
    QuitRole,

    /// Hide application windows (macOS).
    HideRole,

    /// Hide other applications (macOS).
    HideOthersRole,

    /// Show all hidden windows (macOS).
    ShowAllRole,

    /// Minimize window (macOS Window menu).
    MinimizeRole,

    /// Zoom window (macOS Window menu).
    ZoomRole,

    /// Toggle full screen mode.
    FullScreenRole,

    /// Bring all windows to front (macOS Window menu).
    BringAllToFrontRole,

    /// Cut text/selection to clipboard.
    CutRole,
    /// Copy text/selection to clipboard.
    CopyRole,
    /// Paste from clipboard.
    PasteRole,
    /// Select all text/content.
    SelectAllRole,
    /// Undo last action.
    UndoRole,
    /// Redo previously undone action.
    RedoRole,
}

impl MenuRole {
    /// Check if this role causes the action to be moved to the application menu on macOS.
    pub fn is_application_menu_role(self) -> bool {
        matches!(
            self,
            MenuRole::AboutRole
                | MenuRole::PreferencesRole
                | MenuRole::QuitRole
                | MenuRole::HideRole
                | MenuRole::HideOthersRole
                | MenuRole::ShowAllRole
        )
    }

    /// Check if this role causes the action to be moved to the Window menu on macOS.
    pub fn is_window_menu_role(self) -> bool {
        matches!(
            self,
            MenuRole::MinimizeRole | MenuRole::ZoomRole | MenuRole::BringAllToFrontRole
        )
    }

    /// Check if this is a text editing role.
    pub fn is_text_role(self) -> bool {
        matches!(
            self,
            MenuRole::CutRole
                | MenuRole::CopyRole
                | MenuRole::PasteRole
                | MenuRole::SelectAllRole
                | MenuRole::UndoRole
                | MenuRole::RedoRole
        )
    }

    /// Get the standard shortcut for this role, if any.
    ///
    /// Returns the platform-appropriate shortcut (Cmd on macOS, Ctrl elsewhere).
    pub fn standard_shortcut(self) -> Option<&'static str> {
        match self {
            MenuRole::QuitRole => Some("Ctrl+Q"),
            MenuRole::PreferencesRole => Some("Ctrl+,"),
            MenuRole::CutRole => Some("Ctrl+X"),
            MenuRole::CopyRole => Some("Ctrl+C"),
            MenuRole::PasteRole => Some("Ctrl+V"),
            MenuRole::SelectAllRole => Some("Ctrl+A"),
            MenuRole::UndoRole => Some("Ctrl+Z"),
            MenuRole::RedoRole => Some("Ctrl+Shift+Z"),
            MenuRole::MinimizeRole => Some("Ctrl+M"),
            MenuRole::FullScreenRole => Some("F11"),
            _ => None,
        }
    }
}

// ============================================================================
// Action Priority
// ============================================================================

/// Priority level for action display in toolbars.
///
/// When a toolbar has limited space, actions with lower priority may be
/// moved to an overflow menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum ActionPriority {
    /// Low priority - first to be moved to overflow menu.
    Low = 0,
    /// Normal priority (default).
    #[default]
    Normal = 1,
    /// High priority - last to be moved to overflow menu.
    High = 2,
}

// ============================================================================
// Action
// ============================================================================

/// Internal mutable state for an Action.
struct ActionState {
    text: String,
    mnemonic_cache: MnemonicText,
    icon: Option<Icon>,
    shortcut: Option<KeySequence>,
    tooltip: String,
    status_tip: String,
    whats_this: String,
    enabled: bool,
    visible: bool,
    checkable: bool,
    checked: bool,
    shortcut_context: ShortcutContext,
    menu_role: MenuRole,
    priority: ActionPriority,
    auto_repeat: bool,
    icon_visible_in_menu: bool,
    shortcut_visible_in_context_menu: bool,
}

/// A non-visual object representing a user action.
///
/// Actions are the central point for managing commands in an application.
/// They can be added to menus, toolbars, and assigned keyboard shortcuts,
/// providing consistent behavior across all access points.
///
/// # Thread Safety
///
/// `Action` is `Send + Sync` and can be safely shared between threads.
/// All state modifications are protected by internal synchronization.
///
/// # Signals
///
/// - [`triggered`](Action::triggered): Emitted when the action is activated
/// - [`toggled`](Action::toggled): Emitted when a checkable action's state changes
/// - [`changed`](Action::changed): Emitted when any property changes
/// - [`hovered`](Action::hovered): Emitted when the action is highlighted (e.g., in a menu)
pub struct Action {
    /// Object system integration.
    object_base: ObjectBase,

    /// Internal mutable state.
    state: RwLock<ActionState>,

    /// Generation counter for detecting changes (used by `changed` signal).
    generation: AtomicU64,

    /// Signal emitted when the action is activated.
    ///
    /// For checkable actions, this is emitted after the checked state changes.
    /// The parameter is the checked state (always false for non-checkable actions).
    pub triggered: Signal<bool>,

    /// Signal emitted when a checkable action's state changes.
    ///
    /// Only emitted for checkable actions. The parameter is the new checked state.
    pub toggled: Signal<bool>,

    /// Signal emitted when any action property changes.
    ///
    /// This is useful for updating UI elements that display the action's state.
    pub changed: Signal<()>,

    /// Signal emitted when the action is hovered (highlighted in a menu).
    pub hovered: Signal<()>,
}

impl Action {
    /// Create a new action with the given text.
    ///
    /// The text can include a mnemonic indicator using `&`:
    /// - `"&File"` - 'F' is the mnemonic, displayed with underline
    /// - `"E&xit"` - 'x' is the mnemonic
    /// - `"Fish && Chips"` - literal '&', no mnemonic
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let mnemonic_cache = parse_mnemonic(&text);

        Self {
            object_base: ObjectBase::new::<Self>(),
            state: RwLock::new(ActionState {
                text,
                mnemonic_cache,
                icon: None,
                shortcut: None,
                tooltip: String::new(),
                status_tip: String::new(),
                whats_this: String::new(),
                enabled: true,
                visible: true,
                checkable: false,
                checked: false,
                shortcut_context: ShortcutContext::default(),
                menu_role: MenuRole::default(),
                priority: ActionPriority::default(),
                auto_repeat: true,
                icon_visible_in_menu: true,
                shortcut_visible_in_context_menu: true,
            }),
            generation: AtomicU64::new(0),
            triggered: Signal::new(),
            toggled: Signal::new(),
            changed: Signal::new(),
            hovered: Signal::new(),
        }
    }

    /// Create a new action with text and icon.
    pub fn with_icon(text: impl Into<String>, icon: Icon) -> Self {
        let action = Self::new(text);
        action.set_icon(Some(icon));
        action
    }

    /// Create a new action with text and shortcut.
    pub fn with_shortcut(text: impl Into<String>, shortcut: KeySequence) -> Self {
        let action = Self::new(text);
        action.set_shortcut(Some(shortcut));
        action
    }

    // ========================================================================
    // Text
    // ========================================================================

    /// Get the action's text.
    pub fn text(&self) -> String {
        self.state.read().text.clone()
    }

    /// Set the action's text.
    ///
    /// The text can include a mnemonic indicator using `&`.
    pub fn set_text(&self, text: impl Into<String>) {
        let text = text.into();
        let mnemonic_cache = parse_mnemonic(&text);
        {
            let mut state = self.state.write();
            if state.text == text {
                return;
            }
            state.text = text;
            state.mnemonic_cache = mnemonic_cache;
        }
        self.emit_changed();
    }

    /// Get the display text (with mnemonic marker processed).
    pub fn display_text(&self) -> String {
        self.state.read().mnemonic_cache.display_text.clone()
    }

    /// Get the mnemonic character, if any.
    pub fn mnemonic(&self) -> Option<char> {
        self.state.read().mnemonic_cache.mnemonic
    }

    /// Get the mnemonic index in display text.
    pub fn mnemonic_index(&self) -> Option<usize> {
        self.state.read().mnemonic_cache.mnemonic_index
    }

    // ========================================================================
    // Icon
    // ========================================================================

    /// Get the action's icon.
    pub fn icon(&self) -> Option<Icon> {
        self.state.read().icon.clone()
    }

    /// Set the action's icon.
    pub fn set_icon(&self, icon: Option<Icon>) {
        {
            let mut state = self.state.write();
            state.icon = icon;
        }
        self.emit_changed();
    }

    /// Builder pattern for setting icon.
    pub fn with_icon_builder(self, icon: Icon) -> Self {
        self.set_icon(Some(icon));
        self
    }

    // ========================================================================
    // Shortcut
    // ========================================================================

    /// Get the action's keyboard shortcut.
    pub fn shortcut(&self) -> Option<KeySequence> {
        self.state.read().shortcut.clone()
    }

    /// Set the action's keyboard shortcut.
    pub fn set_shortcut(&self, shortcut: Option<KeySequence>) {
        {
            let mut state = self.state.write();
            state.shortcut = shortcut;
        }
        self.emit_changed();
    }

    /// Builder pattern for setting shortcut.
    pub fn with_shortcut_builder(self, shortcut: KeySequence) -> Self {
        self.set_shortcut(Some(shortcut));
        self
    }

    /// Builder pattern for setting shortcut from string.
    pub fn with_shortcut_str(self, shortcut: &str) -> Self {
        if let Ok(seq) = shortcut.parse() {
            self.set_shortcut(Some(seq));
        }
        self
    }

    /// Get the shortcut context.
    pub fn shortcut_context(&self) -> ShortcutContext {
        self.state.read().shortcut_context
    }

    /// Set the shortcut context.
    pub fn set_shortcut_context(&self, context: ShortcutContext) {
        {
            let mut state = self.state.write();
            if state.shortcut_context == context {
                return;
            }
            state.shortcut_context = context;
        }
        self.emit_changed();
    }

    /// Builder pattern for setting shortcut context.
    pub fn with_shortcut_context(self, context: ShortcutContext) -> Self {
        self.set_shortcut_context(context);
        self
    }

    // ========================================================================
    // Tooltip and Status
    // ========================================================================

    /// Get the tooltip text.
    pub fn tooltip(&self) -> String {
        self.state.read().tooltip.clone()
    }

    /// Set the tooltip text.
    pub fn set_tooltip(&self, tooltip: impl Into<String>) {
        {
            let mut state = self.state.write();
            state.tooltip = tooltip.into();
        }
        self.emit_changed();
    }

    /// Builder pattern for setting tooltip.
    pub fn with_tooltip(self, tooltip: impl Into<String>) -> Self {
        self.set_tooltip(tooltip);
        self
    }

    /// Get the status bar tip text.
    pub fn status_tip(&self) -> String {
        self.state.read().status_tip.clone()
    }

    /// Set the status bar tip text.
    pub fn set_status_tip(&self, tip: impl Into<String>) {
        {
            let mut state = self.state.write();
            state.status_tip = tip.into();
        }
        self.emit_changed();
    }

    /// Get the "What's This?" help text.
    pub fn whats_this(&self) -> String {
        self.state.read().whats_this.clone()
    }

    /// Set the "What's This?" help text.
    pub fn set_whats_this(&self, text: impl Into<String>) {
        {
            let mut state = self.state.write();
            state.whats_this = text.into();
        }
        self.emit_changed();
    }

    // ========================================================================
    // Enabled / Visible
    // ========================================================================

    /// Check if the action is enabled.
    pub fn is_enabled(&self) -> bool {
        self.state.read().enabled
    }

    /// Set whether the action is enabled.
    pub fn set_enabled(&self, enabled: bool) {
        {
            let mut state = self.state.write();
            if state.enabled == enabled {
                return;
            }
            state.enabled = enabled;
        }
        self.emit_changed();
    }

    /// Builder pattern for setting enabled state.
    pub fn with_enabled(self, enabled: bool) -> Self {
        self.set_enabled(enabled);
        self
    }

    /// Check if the action is visible.
    pub fn is_visible(&self) -> bool {
        self.state.read().visible
    }

    /// Set whether the action is visible.
    pub fn set_visible(&self, visible: bool) {
        {
            let mut state = self.state.write();
            if state.visible == visible {
                return;
            }
            state.visible = visible;
        }
        self.emit_changed();
    }

    /// Builder pattern for setting visibility.
    pub fn with_visible(self, visible: bool) -> Self {
        self.set_visible(visible);
        self
    }

    // ========================================================================
    // Checkable State
    // ========================================================================

    /// Check if the action is checkable.
    pub fn is_checkable(&self) -> bool {
        self.state.read().checkable
    }

    /// Set whether the action is checkable.
    pub fn set_checkable(&self, checkable: bool) {
        let should_uncheck;
        {
            let mut state = self.state.write();
            if state.checkable == checkable {
                return;
            }
            state.checkable = checkable;
            should_uncheck = !checkable && state.checked;
            if should_uncheck {
                state.checked = false;
            }
        }
        if should_uncheck {
            self.toggled.emit(false);
        }
        self.emit_changed();
    }

    /// Builder pattern for setting checkable state.
    pub fn with_checkable(self, checkable: bool) -> Self {
        self.set_checkable(checkable);
        self
    }

    /// Check if the action is currently checked.
    pub fn is_checked(&self) -> bool {
        self.state.read().checked
    }

    /// Set the checked state.
    ///
    /// Only has effect if the action is checkable.
    pub fn set_checked(&self, checked: bool) {
        let should_emit;
        {
            let mut state = self.state.write();
            if !state.checkable || state.checked == checked {
                return;
            }
            state.checked = checked;
            should_emit = true;
        }
        if should_emit {
            self.toggled.emit(checked);
            self.emit_changed();
        }
    }

    /// Builder pattern for setting checked state.
    pub fn with_checked(self, checked: bool) -> Self {
        self.set_checked(checked);
        self
    }

    /// Toggle the checked state.
    ///
    /// Only has effect if the action is checkable.
    pub fn toggle(&self) {
        let new_checked;
        {
            let mut state = self.state.write();
            if !state.checkable {
                return;
            }
            state.checked = !state.checked;
            new_checked = state.checked;
        }
        self.toggled.emit(new_checked);
        self.emit_changed();
    }

    // ========================================================================
    // Menu Role
    // ========================================================================

    /// Get the menu role.
    pub fn menu_role(&self) -> MenuRole {
        self.state.read().menu_role
    }

    /// Set the menu role.
    pub fn set_menu_role(&self, role: MenuRole) {
        {
            let mut state = self.state.write();
            if state.menu_role == role {
                return;
            }
            state.menu_role = role;
        }
        self.emit_changed();
    }

    /// Builder pattern for setting menu role.
    pub fn with_menu_role(self, role: MenuRole) -> Self {
        self.set_menu_role(role);
        self
    }

    // ========================================================================
    // Priority
    // ========================================================================

    /// Get the action priority for toolbar overflow.
    pub fn priority(&self) -> ActionPriority {
        self.state.read().priority
    }

    /// Set the action priority.
    pub fn set_priority(&self, priority: ActionPriority) {
        {
            let mut state = self.state.write();
            if state.priority == priority {
                return;
            }
            state.priority = priority;
        }
        self.emit_changed();
    }

    /// Builder pattern for setting priority.
    pub fn with_priority(self, priority: ActionPriority) -> Self {
        self.set_priority(priority);
        self
    }

    // ========================================================================
    // Auto-repeat
    // ========================================================================

    /// Check if auto-repeat is enabled for keyboard shortcuts.
    pub fn auto_repeat(&self) -> bool {
        self.state.read().auto_repeat
    }

    /// Set whether auto-repeat is enabled for keyboard shortcuts.
    pub fn set_auto_repeat(&self, auto_repeat: bool) {
        {
            let mut state = self.state.write();
            state.auto_repeat = auto_repeat;
        }
        self.emit_changed();
    }

    /// Builder pattern for setting auto-repeat.
    pub fn with_auto_repeat(self, auto_repeat: bool) -> Self {
        self.set_auto_repeat(auto_repeat);
        self
    }

    // ========================================================================
    // Display Options
    // ========================================================================

    /// Check if the icon should be shown in menus.
    pub fn is_icon_visible_in_menu(&self) -> bool {
        self.state.read().icon_visible_in_menu
    }

    /// Set whether the icon should be shown in menus.
    pub fn set_icon_visible_in_menu(&self, visible: bool) {
        {
            let mut state = self.state.write();
            state.icon_visible_in_menu = visible;
        }
        self.emit_changed();
    }

    /// Check if the shortcut should be shown in context menus.
    pub fn is_shortcut_visible_in_context_menu(&self) -> bool {
        self.state.read().shortcut_visible_in_context_menu
    }

    /// Set whether the shortcut should be shown in context menus.
    pub fn set_shortcut_visible_in_context_menu(&self, visible: bool) {
        {
            let mut state = self.state.write();
            state.shortcut_visible_in_context_menu = visible;
        }
        self.emit_changed();
    }

    // ========================================================================
    // Activation
    // ========================================================================

    /// Trigger the action programmatically.
    ///
    /// This behaves as if the user activated the action. For checkable actions,
    /// this toggles the checked state before emitting `triggered`.
    pub fn trigger(&self) {
        if !self.is_enabled() {
            return;
        }

        let checked;
        {
            let mut state = self.state.write();
            if state.checkable {
                state.checked = !state.checked;
                checked = state.checked;
            } else {
                checked = false;
            }
        }

        if self.is_checkable() {
            self.toggled.emit(checked);
        }
        self.triggered.emit(checked);
    }

    /// Emit the hovered signal.
    ///
    /// Called when the action is highlighted in a menu.
    pub fn hover(&self) {
        self.hovered.emit(());
    }

    // ========================================================================
    // Change Detection
    // ========================================================================

    /// Get the current generation counter.
    ///
    /// This increments each time a property changes, useful for detecting
    /// when UI elements need to be updated.
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Emit the changed signal and increment generation.
    fn emit_changed(&self) {
        self.generation.fetch_add(1, Ordering::Release);
        self.changed.emit(());
    }
}

impl Object for Action {
    fn object_id(&self) -> ObjectId {
        self.object_base.id()
    }
}

// Action is Send + Sync
unsafe impl Send for Action {}
unsafe impl Sync for Action {}

// ============================================================================
// ActionGroup
// ============================================================================

/// Internal state for ActionGroup.
struct ActionGroupState {
    actions: Vec<Arc<Action>>,
    exclusive: bool,
    enabled: bool,
    visible: bool,
}

/// A group of actions with optional mutual exclusivity.
///
/// `ActionGroup` is used to manage a set of related actions, typically
/// checkable actions where only one should be checked at a time (like
/// radio buttons in a menu).
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::widgets::{Action, ActionGroup};
/// use std::sync::Arc;
///
/// let group = ActionGroup::new();
///
/// let left_align = Arc::new(Action::new("&Left").with_checkable(true));
/// let center_align = Arc::new(Action::new("&Center").with_checkable(true));
/// let right_align = Arc::new(Action::new("&Right").with_checkable(true));
///
/// group.add_action(left_align.clone());
/// group.add_action(center_align.clone());
/// group.add_action(right_align.clone());
///
/// // Group is exclusive by default, so checking one unchecks others
/// left_align.set_checked(true);  // center and right are now unchecked
/// ```
pub struct ActionGroup {
    /// Object system integration.
    object_base: ObjectBase,

    /// Internal state.
    state: RwLock<ActionGroupState>,

    /// Signal emitted when an action in the group is triggered.
    pub triggered: Signal<Arc<Action>>,

    /// Signal emitted when the checked action changes (exclusive groups).
    pub checked_changed: Signal<Option<Arc<Action>>>,
}

impl ActionGroup {
    /// Create a new action group with exclusive selection.
    pub fn new() -> Self {
        Self {
            object_base: ObjectBase::new::<Self>(),
            state: RwLock::new(ActionGroupState {
                actions: Vec::new(),
                exclusive: true,
                enabled: true,
                visible: true,
            }),
            triggered: Signal::new(),
            checked_changed: Signal::new(),
        }
    }

    /// Create a non-exclusive action group.
    pub fn non_exclusive() -> Self {
        let group = Self::new();
        group.set_exclusive(false);
        group
    }

    // ========================================================================
    // Actions
    // ========================================================================

    /// Add an action to the group.
    pub fn add_action(&self, action: Arc<Action>) {
        {
            let mut state = self.state.write();

            // Check if already in group
            if state
                .actions
                .iter()
                .any(|a| a.object_id() == action.object_id())
            {
                return;
            }

            // Apply group's enabled/visible state
            if !state.enabled {
                action.set_enabled(false);
            }
            if !state.visible {
                action.set_visible(false);
            }

            state.actions.push(action);
        }
    }

    /// Remove an action from the group.
    pub fn remove_action(&self, action: &Arc<Action>) {
        let mut state = self.state.write();
        state
            .actions
            .retain(|a| a.object_id() != action.object_id());
    }

    /// Get all actions in the group.
    pub fn actions(&self) -> Vec<Arc<Action>> {
        self.state.read().actions.clone()
    }

    /// Get the currently checked action (for exclusive groups).
    pub fn checked_action(&self) -> Option<Arc<Action>> {
        let state = self.state.read();
        state.actions.iter().find(|a| a.is_checked()).cloned()
    }

    // ========================================================================
    // Exclusivity
    // ========================================================================

    /// Check if the group is exclusive.
    pub fn is_exclusive(&self) -> bool {
        self.state.read().exclusive
    }

    /// Set whether the group is exclusive.
    ///
    /// When exclusive, only one action in the group can be checked at a time.
    pub fn set_exclusive(&self, exclusive: bool) {
        let mut state = self.state.write();
        state.exclusive = exclusive;
    }

    /// Builder pattern for setting exclusivity.
    pub fn with_exclusive(self, exclusive: bool) -> Self {
        self.set_exclusive(exclusive);
        self
    }

    // ========================================================================
    // Group Enable/Visible
    // ========================================================================

    /// Check if the group is enabled.
    pub fn is_enabled(&self) -> bool {
        self.state.read().enabled
    }

    /// Set whether the group is enabled.
    ///
    /// This affects all actions in the group.
    pub fn set_enabled(&self, enabled: bool) {
        let actions;
        {
            let mut state = self.state.write();
            if state.enabled == enabled {
                return;
            }
            state.enabled = enabled;
            actions = state.actions.clone();
        }
        for action in actions {
            action.set_enabled(enabled);
        }
    }

    /// Check if the group is visible.
    pub fn is_visible(&self) -> bool {
        self.state.read().visible
    }

    /// Set whether the group is visible.
    ///
    /// This affects all actions in the group.
    pub fn set_visible(&self, visible: bool) {
        let actions;
        {
            let mut state = self.state.write();
            if state.visible == visible {
                return;
            }
            state.visible = visible;
            actions = state.actions.clone();
        }
        for action in actions {
            action.set_visible(visible);
        }
    }

    // ========================================================================
    // Exclusive Selection Handling
    // ========================================================================

    /// Handle an action being checked (for exclusive groups).
    ///
    /// Call this when an action in an exclusive group is checked to
    /// uncheck all other actions.
    pub fn handle_action_checked(&self, checked_action: &Arc<Action>) {
        let state = self.state.read();
        if !state.exclusive {
            return;
        }

        for action in &state.actions {
            if action.object_id() != checked_action.object_id() && action.is_checked() {
                action.set_checked(false);
            }
        }

        drop(state);
        self.checked_changed.emit(Some(checked_action.clone()));
    }
}

impl Default for ActionGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ActionGroup {
    fn object_id(&self) -> ObjectId {
        self.object_base.id()
    }
}

// ActionGroup is Send + Sync
unsafe impl Send for ActionGroup {}
unsafe impl Sync for ActionGroup {}

// ============================================================================
// Separator Action (convenience)
// ============================================================================

impl Action {
    /// Create a separator action.
    ///
    /// Separators are used to visually divide groups of actions in menus
    /// and toolbars. They have no text, icon, or shortcut.
    pub fn separator() -> Self {
        Self::new("")
    }

    /// Check if this action is a separator.
    pub fn is_separator(&self) -> bool {
        // Separators have no text, no shortcut, and no icon
        self.text().is_empty() && self.shortcut().is_none() && self.icon().is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    #[test]
    fn test_action_new() {
        init_global_registry();
        let action = Action::new("&Save");
        assert_eq!(action.text(), "&Save");
        assert_eq!(action.display_text(), "Save");
        assert_eq!(action.mnemonic(), Some('s'));
        assert!(action.is_enabled());
        assert!(action.is_visible());
        assert!(!action.is_checkable());
        assert!(!action.is_checked());
    }

    #[test]
    fn test_action_checkable() {
        init_global_registry();
        let action = Action::new("Bold").with_checkable(true);
        assert!(action.is_checkable());
        assert!(!action.is_checked());

        action.set_checked(true);
        assert!(action.is_checked());

        action.toggle();
        assert!(!action.is_checked());
    }

    #[test]
    fn test_action_trigger() {
        init_global_registry();
        let action = Action::new("Test").with_checkable(true);
        let triggered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let triggered_clone = triggered.clone();

        action.triggered.connect(move |_| {
            triggered_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        action.trigger();
        assert!(triggered.load(std::sync::atomic::Ordering::SeqCst));
        assert!(action.is_checked()); // Toggle happened
    }

    #[test]
    fn test_action_disabled_no_trigger() {
        init_global_registry();
        let action = Action::new("Test");
        action.set_enabled(false);

        let triggered = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let triggered_clone = triggered.clone();

        action.triggered.connect(move |_| {
            triggered_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        action.trigger();
        assert!(!triggered.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[test]
    fn test_action_shortcut() {
        init_global_registry();
        let action = Action::new("Save").with_shortcut_str("Ctrl+S");
        let shortcut = action.shortcut().expect("Should have shortcut");
        assert_eq!(shortcut.to_string(), "Ctrl+S");
    }

    #[test]
    fn test_action_menu_role() {
        init_global_registry();
        let action = Action::new("Quit").with_menu_role(MenuRole::QuitRole);
        assert_eq!(action.menu_role(), MenuRole::QuitRole);
        assert!(action.menu_role().is_application_menu_role());
    }

    #[test]
    fn test_shortcut_context() {
        assert!(ShortcutContext::Widget.requires_focus());
        assert!(!ShortcutContext::Window.requires_focus());
        assert!(ShortcutContext::Window.is_window_scoped());
        assert!(ShortcutContext::Application.is_application_wide());
    }

    #[test]
    fn test_action_group_exclusive() {
        init_global_registry();
        let group = ActionGroup::new();
        assert!(group.is_exclusive());

        let action1 = Arc::new(Action::new("Option 1").with_checkable(true));
        let action2 = Arc::new(Action::new("Option 2").with_checkable(true));
        let action3 = Arc::new(Action::new("Option 3").with_checkable(true));

        group.add_action(action1.clone());
        group.add_action(action2.clone());
        group.add_action(action3.clone());

        assert_eq!(group.actions().len(), 3);

        action1.set_checked(true);
        group.handle_action_checked(&action1);

        assert!(action1.is_checked());

        action2.set_checked(true);
        group.handle_action_checked(&action2);

        assert!(!action1.is_checked());
        assert!(action2.is_checked());
    }

    #[test]
    fn test_action_group_enable_disable() {
        init_global_registry();
        let group = ActionGroup::new();
        let action1 = Arc::new(Action::new("Action 1"));
        let action2 = Arc::new(Action::new("Action 2"));

        group.add_action(action1.clone());
        group.add_action(action2.clone());

        assert!(action1.is_enabled());
        assert!(action2.is_enabled());

        group.set_enabled(false);

        assert!(!action1.is_enabled());
        assert!(!action2.is_enabled());
    }

    #[test]
    fn test_action_separator() {
        init_global_registry();
        let sep = Action::separator();
        assert!(sep.is_separator());
        assert!(sep.text().is_empty());
    }

    #[test]
    fn test_action_changed_signal() {
        init_global_registry();
        let action = Action::new("Test");
        let changed_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let changed_clone = changed_count.clone();

        action.changed.connect(move |_| {
            changed_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        });

        let initial_gen = action.generation();

        action.set_text("New Text");
        assert!(action.generation() > initial_gen);
        assert_eq!(
            changed_count.load(std::sync::atomic::Ordering::SeqCst),
            1
        );

        action.set_enabled(false);
        assert_eq!(
            changed_count.load(std::sync::atomic::Ordering::SeqCst),
            2
        );
    }

    #[test]
    fn test_menu_role_standard_shortcut() {
        assert_eq!(MenuRole::QuitRole.standard_shortcut(), Some("Ctrl+Q"));
        assert_eq!(MenuRole::CopyRole.standard_shortcut(), Some("Ctrl+C"));
        assert_eq!(MenuRole::NoRole.standard_shortcut(), None);
    }
}
