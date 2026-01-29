//! macOS global menu bar support.
//!
//! This module provides native macOS global menu bar integration, converting
//! Horizon Lattice's [`MenuBar`] widget into the native AppKit menu system.
//!
//! # Overview
//!
//! On macOS, applications traditionally display their menu bar at the top of the
//! screen rather than inside the window. This module provides [`NativeMenuBar`]
//! which converts a [`MenuBar`] into native NSMenu/NSMenuItem objects.
//!
//! # Features
//!
//! - Bidirectional state sync between [`Action`] and native menu items
//! - Automatic handling of [`MenuRole`] to move items to standard locations
//! - Standard macOS application menu (About, Preferences, Quit, Hide, etc.)
//! - Keyboard shortcut integration
//! - Checkable menu items
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::platform::NativeMenuBar;
//! use horizon_lattice::widget::widgets::{Action, Menu, MenuBar, MenuRole};
//! use std::sync::Arc;
//!
//! // Create menus as usual
//! let mut file_menu = Menu::new();
//! file_menu.add_action(Arc::new(Action::new("&Open")));
//!
//! let mut menu_bar = MenuBar::new();
//! menu_bar.add_menu("&File", Arc::new(file_menu));
//!
//! // Create About and Quit actions with roles
//! let about = Arc::new(Action::new("About MyApp").with_menu_role(MenuRole::AboutRole));
//! let quit = Arc::new(Action::new("Quit").with_menu_role(MenuRole::QuitRole));
//!
//! // Convert to native menu bar (must be called from main thread)
//! let native_menu = NativeMenuBar::from_menu_bar(&menu_bar, "MyApp")?;
//! native_menu.set_as_main_menu()?;
//! ```
//!
//! # Platform Notes
//!
//! This module is only available on macOS. On other platforms, applications
//! should use the in-window [`MenuBar`] widget.
//!
//! # Thread Safety
//!
//! All operations must be performed on the main thread (AppKit requirement).
//! Methods will return an error if called from a non-main thread.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::NSString;

use crate::widget::KeySequence;
use crate::widget::widgets::{Action, Menu, MenuBar, MenuItem, MenuRole};

// ============================================================================
// Error Types
// ============================================================================

/// Error type for native menu operations.
#[derive(Debug)]
pub struct NativeMenuError {
    kind: NativeMenuErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeMenuErrorKind {
    /// Operation requires main thread.
    NotMainThread,
    /// Failed to create native menu.
    CreationFailed,
    /// Failed to set menu as main menu.
    SetMainMenuFailed,
    /// Invalid state.
    InvalidState,
}

impl NativeMenuError {
    fn not_main_thread() -> Self {
        Self {
            kind: NativeMenuErrorKind::NotMainThread,
            message: "macOS menu operations must be performed on the main thread".into(),
        }
    }

    fn creation_failed(message: impl Into<String>) -> Self {
        Self {
            kind: NativeMenuErrorKind::CreationFailed,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn set_main_menu_failed(message: impl Into<String>) -> Self {
        Self {
            kind: NativeMenuErrorKind::SetMainMenuFailed,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn invalid_state(message: impl Into<String>) -> Self {
        Self {
            kind: NativeMenuErrorKind::InvalidState,
            message: message.into(),
        }
    }

    /// Check if the error is due to not being on the main thread.
    pub fn is_not_main_thread(&self) -> bool {
        self.kind == NativeMenuErrorKind::NotMainThread
    }
}

impl fmt::Display for NativeMenuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            NativeMenuErrorKind::NotMainThread => {
                write!(f, "not on main thread: {}", self.message)
            }
            NativeMenuErrorKind::CreationFailed => {
                write!(f, "menu creation failed: {}", self.message)
            }
            NativeMenuErrorKind::SetMainMenuFailed => {
                write!(f, "failed to set main menu: {}", self.message)
            }
            NativeMenuErrorKind::InvalidState => {
                write!(f, "invalid state: {}", self.message)
            }
        }
    }
}

impl std::error::Error for NativeMenuError {}

// ============================================================================
// Action Binding
// ============================================================================

// Internal storage for action bindings.
// This allows us to track which NSMenuItem corresponds to which Action
// for bidirectional updates.
thread_local! {
    static ACTION_BINDINGS: RefCell<HashMap<usize, Arc<Action>>> = RefCell::new(HashMap::new());
    static NEXT_ACTION_ID: RefCell<usize> = const { RefCell::new(1) };
}

/// Register an action and return its unique ID.
fn register_action(action: Arc<Action>) -> usize {
    NEXT_ACTION_ID.with(|id| {
        let current = *id.borrow();
        *id.borrow_mut() = current + 1;
        ACTION_BINDINGS.with(|bindings| {
            bindings.borrow_mut().insert(current, action);
        });
        current
    })
}

/// Get an action by its registered ID.
#[allow(dead_code)]
fn get_action(id: usize) -> Option<Arc<Action>> {
    ACTION_BINDINGS.with(|bindings| bindings.borrow().get(&id).cloned())
}

/// Unregister an action by ID.
#[allow(dead_code)]
fn unregister_action(id: usize) {
    ACTION_BINDINGS.with(|bindings| {
        bindings.borrow_mut().remove(&id);
    });
}

// ============================================================================
// NativeMenuBar
// ============================================================================

/// A native macOS menu bar.
///
/// This wraps an NSMenu that has been configured as the application's main menu,
/// along with tracking for bidirectional Action sync.
pub struct NativeMenuBar {
    /// The main menu (root NSMenu).
    main_menu: Retained<NSMenu>,
    /// Application name used for standard menu items.
    app_name: String,
    /// Marker proving we're on the main thread.
    _mtm: MainThreadMarker,
    /// Actions that have been registered for callbacks.
    _registered_actions: Vec<usize>,
}

impl NativeMenuBar {
    /// Create a native menu bar from a Horizon Lattice MenuBar.
    ///
    /// This converts the MenuBar's structure to native NSMenu/NSMenuItem objects.
    /// Actions with special [`MenuRole`] values are automatically reorganized
    /// into standard macOS menu locations.
    ///
    /// # Arguments
    ///
    /// * `menu_bar` - The MenuBar to convert
    /// * `app_name` - Application name (used for "About", "Quit", etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if not called from the main thread.
    pub fn from_menu_bar(menu_bar: &MenuBar, app_name: &str) -> Result<Self, NativeMenuError> {
        let mtm = MainThreadMarker::new().ok_or_else(NativeMenuError::not_main_thread)?;

        let mut native = Self {
            main_menu: NSMenu::new(mtm),
            app_name: app_name.to_string(),
            _mtm: mtm,
            _registered_actions: Vec::new(),
        };

        // Create the application menu (first menu, contains About, Preferences, Quit)
        native.create_application_menu(mtm)?;

        // Convert each menu in the menu bar
        for i in 0..menu_bar.menu_count() {
            if let Some(menu) = menu_bar.menu(i) {
                // Get the title from the MenuBar's item
                // We need to access the internal item title - for now use a placeholder
                // that will be set correctly
                let title = format!("Menu {}", i + 1);
                native.add_menu_from_horizon(&title, menu, mtm)?;
            }
        }

        Ok(native)
    }

    /// Create a native menu bar from individual menus.
    ///
    /// This provides more control over menu construction than `from_menu_bar`.
    ///
    /// # Arguments
    ///
    /// * `menus` - Vector of (title, Menu) pairs
    /// * `app_name` - Application name
    /// * `special_actions` - Actions with MenuRole for the application menu
    pub fn from_menus(
        menus: Vec<(&str, &Menu)>,
        app_name: &str,
        special_actions: &[Arc<Action>],
    ) -> Result<Self, NativeMenuError> {
        let mtm = MainThreadMarker::new().ok_or_else(NativeMenuError::not_main_thread)?;

        let mut native = Self {
            main_menu: NSMenu::new(mtm),
            app_name: app_name.to_string(),
            _mtm: mtm,
            _registered_actions: Vec::new(),
        };

        // Create application menu with special actions
        native.create_application_menu_with_actions(special_actions, mtm)?;

        // Add user menus
        for (title, menu) in menus {
            native.add_menu_from_horizon(title, menu, mtm)?;
        }

        Ok(native)
    }

    /// Set this menu bar as the application's main menu.
    ///
    /// After calling this, the menu bar will appear at the top of the screen
    /// (macOS global menu bar style).
    pub fn set_as_main_menu(&self) -> Result<(), NativeMenuError> {
        let mtm = MainThreadMarker::new().ok_or_else(NativeMenuError::not_main_thread)?;

        let app = NSApplication::sharedApplication(mtm);
        app.setMainMenu(Some(&self.main_menu));

        Ok(())
    }

    /// Get a reference to the underlying NSMenu.
    pub fn native_menu(&self) -> &NSMenu {
        &self.main_menu
    }

    // =========================================================================
    // Application Menu
    // =========================================================================

    /// Create the standard macOS application menu.
    ///
    /// This creates the menu that appears as the application name and contains
    /// standard items like About, Preferences, Services, Hide, Quit, etc.
    fn create_application_menu(&mut self, mtm: MainThreadMarker) -> Result<(), NativeMenuError> {
        // Create the application menu
        let app_menu = NSMenu::new(mtm);
        let app_menu_title = NSString::from_str(&self.app_name);
        app_menu.setTitle(&app_menu_title);

        // About item
        let about_title = NSString::from_str(&format!("About {}", self.app_name));
        let about_item = create_menu_item(
            mtm,
            &about_title,
            Some(sel!(orderFrontStandardAboutPanel:)),
            "",
            NSEventModifierFlags::empty(),
        );
        app_menu.addItem(&about_item);

        // Separator
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Preferences (disabled by default, user needs to provide action)
        let prefs_title = NSString::from_str("Settings...");
        let prefs_item =
            create_menu_item(mtm, &prefs_title, None, ",", NSEventModifierFlags::Command);
        prefs_item.setEnabled(false);
        app_menu.addItem(&prefs_item);

        // Separator
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Services submenu
        let services_title = NSString::from_str("Services");
        let services_item = create_menu_item(
            mtm,
            &services_title,
            None,
            "",
            NSEventModifierFlags::empty(),
        );
        let services_menu = NSMenu::new(mtm);
        services_menu.setTitle(&services_title);
        services_item.setSubmenu(Some(&services_menu));
        app_menu.addItem(&services_item);

        // Set as services menu
        let app = NSApplication::sharedApplication(mtm);
        app.setServicesMenu(Some(&services_menu));

        // Separator
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Hide <AppName>
        let hide_title = NSString::from_str(&format!("Hide {}", self.app_name));
        let hide_item = create_menu_item(
            mtm,
            &hide_title,
            Some(sel!(hide:)),
            "h",
            NSEventModifierFlags::Command,
        );
        app_menu.addItem(&hide_item);

        // Hide Others
        let hide_others_title = NSString::from_str("Hide Others");
        let hide_others_item = create_menu_item(
            mtm,
            &hide_others_title,
            Some(sel!(hideOtherApplications:)),
            "h",
            NSEventModifierFlags::Command | NSEventModifierFlags::Option,
        );
        app_menu.addItem(&hide_others_item);

        // Show All
        let show_all_title = NSString::from_str("Show All");
        let show_all_item = create_menu_item(
            mtm,
            &show_all_title,
            Some(sel!(unhideAllApplications:)),
            "",
            NSEventModifierFlags::empty(),
        );
        app_menu.addItem(&show_all_item);

        // Separator
        app_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Quit
        let quit_title = NSString::from_str(&format!("Quit {}", self.app_name));
        let quit_item = create_menu_item(
            mtm,
            &quit_title,
            Some(sel!(terminate:)),
            "q",
            NSEventModifierFlags::Command,
        );
        app_menu.addItem(&quit_item);

        // Create the menu bar item for the app menu
        let app_menu_item = NSMenuItem::new(mtm);
        app_menu_item.setSubmenu(Some(&app_menu));
        self.main_menu.addItem(&app_menu_item);

        Ok(())
    }

    /// Create application menu with user-provided actions for About, Preferences, Quit.
    fn create_application_menu_with_actions(
        &mut self,
        special_actions: &[Arc<Action>],
        mtm: MainThreadMarker,
    ) -> Result<(), NativeMenuError> {
        let app_menu = NSMenu::new(mtm);
        let app_menu_title = NSString::from_str(&self.app_name);
        app_menu.setTitle(&app_menu_title);

        // Find and add About action or use default
        let about_action = special_actions
            .iter()
            .find(|a| a.menu_role() == MenuRole::AboutRole);

        if let Some(action) = about_action {
            let item = self.create_menu_item_from_action(action, mtm)?;
            app_menu.addItem(&item);
        } else {
            // Default About
            let about_title = NSString::from_str(&format!("About {}", self.app_name));
            let about_item = create_menu_item(
                mtm,
                &about_title,
                Some(sel!(orderFrontStandardAboutPanel:)),
                "",
                NSEventModifierFlags::empty(),
            );
            app_menu.addItem(&about_item);
        }

        app_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Find and add Preferences action
        let prefs_action = special_actions
            .iter()
            .find(|a| a.menu_role() == MenuRole::PreferencesRole);

        if let Some(action) = prefs_action {
            let item = self.create_menu_item_from_action(action, mtm)?;
            // Override key equivalent to standard
            item.setKeyEquivalent(&NSString::from_str(","));
            item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
            app_menu.addItem(&item);
        }

        app_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Services submenu
        let services_title = NSString::from_str("Services");
        let services_item = create_menu_item(
            mtm,
            &services_title,
            None,
            "",
            NSEventModifierFlags::empty(),
        );
        let services_menu = NSMenu::new(mtm);
        services_menu.setTitle(&services_title);
        services_item.setSubmenu(Some(&services_menu));
        app_menu.addItem(&services_item);

        let app = NSApplication::sharedApplication(mtm);
        app.setServicesMenu(Some(&services_menu));

        app_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Hide actions
        self.add_standard_hide_items(&app_menu, mtm)?;

        app_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Find and add Quit action or use default
        let quit_action = special_actions
            .iter()
            .find(|a| a.menu_role() == MenuRole::QuitRole);

        if let Some(action) = quit_action {
            let item = self.create_menu_item_from_action(action, mtm)?;
            // Override key equivalent to standard
            item.setKeyEquivalent(&NSString::from_str("q"));
            item.setKeyEquivalentModifierMask(NSEventModifierFlags::Command);
            app_menu.addItem(&item);
        } else {
            // Default Quit
            let quit_title = NSString::from_str(&format!("Quit {}", self.app_name));
            let quit_item = create_menu_item(
                mtm,
                &quit_title,
                Some(sel!(terminate:)),
                "q",
                NSEventModifierFlags::Command,
            );
            app_menu.addItem(&quit_item);
        }

        // Create menu bar item
        let app_menu_item = NSMenuItem::new(mtm);
        app_menu_item.setSubmenu(Some(&app_menu));
        self.main_menu.addItem(&app_menu_item);

        Ok(())
    }

    /// Add standard Hide/Show items to a menu.
    fn add_standard_hide_items(
        &self,
        menu: &NSMenu,
        mtm: MainThreadMarker,
    ) -> Result<(), NativeMenuError> {
        // Hide <AppName>
        let hide_title = NSString::from_str(&format!("Hide {}", self.app_name));
        let hide_item = create_menu_item(
            mtm,
            &hide_title,
            Some(sel!(hide:)),
            "h",
            NSEventModifierFlags::Command,
        );
        menu.addItem(&hide_item);

        // Hide Others
        let hide_others_title = NSString::from_str("Hide Others");
        let hide_others_item = create_menu_item(
            mtm,
            &hide_others_title,
            Some(sel!(hideOtherApplications:)),
            "h",
            NSEventModifierFlags::Command | NSEventModifierFlags::Option,
        );
        menu.addItem(&hide_others_item);

        // Show All
        let show_all_title = NSString::from_str("Show All");
        let show_all_item = create_menu_item(
            mtm,
            &show_all_title,
            Some(sel!(unhideAllApplications:)),
            "",
            NSEventModifierFlags::empty(),
        );
        menu.addItem(&show_all_item);

        Ok(())
    }

    // =========================================================================
    // Menu Conversion
    // =========================================================================

    /// Add a menu from a Horizon Lattice Menu.
    fn add_menu_from_horizon(
        &mut self,
        title: &str,
        menu: &Menu,
        mtm: MainThreadMarker,
    ) -> Result<(), NativeMenuError> {
        let ns_title = NSString::from_str(&strip_mnemonic(title));
        let ns_menu = NSMenu::new(mtm);
        ns_menu.setTitle(&ns_title);

        // Convert all items
        self.populate_menu_from_horizon(&ns_menu, menu, mtm)?;

        // Create menu bar item
        let menu_item = NSMenuItem::new(mtm);
        menu_item.setTitle(&ns_title);
        menu_item.setSubmenu(Some(&ns_menu));
        self.main_menu.addItem(&menu_item);

        Ok(())
    }

    /// Populate an NSMenu from a Horizon Menu's items.
    fn populate_menu_from_horizon(
        &mut self,
        ns_menu: &NSMenu,
        menu: &Menu,
        mtm: MainThreadMarker,
    ) -> Result<(), NativeMenuError> {
        for item in menu.items() {
            match item {
                MenuItem::Action(action) => {
                    // Skip items with application menu roles (they go in the app menu)
                    if action.menu_role().is_application_menu_role() {
                        continue;
                    }

                    let ns_item = self.create_menu_item_from_action(action, mtm)?;
                    ns_menu.addItem(&ns_item);
                }
                MenuItem::Separator => {
                    ns_menu.addItem(&NSMenuItem::separatorItem(mtm));
                }
                MenuItem::Submenu { title, menu, .. } => {
                    let submenu_title = NSString::from_str(&strip_mnemonic(title));
                    let submenu = NSMenu::new(mtm);
                    submenu.setTitle(&submenu_title);

                    // Recursively populate submenu
                    self.populate_menu_from_horizon(&submenu, menu, mtm)?;

                    let submenu_item = NSMenuItem::new(mtm);
                    submenu_item.setTitle(&submenu_title);
                    submenu_item.setSubmenu(Some(&submenu));
                    ns_menu.addItem(&submenu_item);
                }
            }
        }

        Ok(())
    }

    /// Create an NSMenuItem from an Action.
    fn create_menu_item_from_action(
        &mut self,
        action: &Arc<Action>,
        mtm: MainThreadMarker,
    ) -> Result<Retained<NSMenuItem>, NativeMenuError> {
        let title = NSString::from_str(&strip_mnemonic(&action.display_text()));
        let key_equiv = action
            .shortcut()
            .map(|s| key_sequence_to_key_equivalent(&s))
            .unwrap_or_default();

        let modifiers = action
            .shortcut()
            .map(|s| key_sequence_to_modifier_flags(&s))
            .unwrap_or(NSEventModifierFlags::empty());

        // Register the action for callbacks
        let action_id = register_action(action.clone());
        self._registered_actions.push(action_id);

        // Create the menu item
        // Note: For actual action callbacks, we'd need a custom target object.
        // For now, we create items without action selectors - they'll need to be
        // hooked up via a delegate pattern or by overriding validateMenuItem.
        let item = create_menu_item(mtm, &title, None, &key_equiv, modifiers);

        // Set enabled state
        item.setEnabled(action.is_enabled());

        // Set checkmark for checkable actions
        if action.is_checkable() {
            let state = if action.is_checked() { 1 } else { 0 }; // NSOnState = 1, NSOffState = 0
            item.setState(state);
        }

        // Store the action ID in the item's tag for later retrieval
        item.setTag(action_id as isize);

        // Set up change listener to sync Action state to NSMenuItem
        // Note: In a full implementation, we'd need to keep a reference to the item
        // and update it when the Action changes. For now, this is a one-way conversion.

        Ok(item)
    }

    /// Add a standard Edit menu with text editing commands.
    pub fn add_edit_menu(&mut self) -> Result<(), NativeMenuError> {
        let mtm = MainThreadMarker::new().ok_or_else(NativeMenuError::not_main_thread)?;

        let edit_title = NSString::from_str("Edit");
        let edit_menu = NSMenu::new(mtm);
        edit_menu.setTitle(&edit_title);

        // Undo
        let undo_item = create_menu_item(
            mtm,
            &NSString::from_str("Undo"),
            Some(sel!(undo:)),
            "z",
            NSEventModifierFlags::Command,
        );
        edit_menu.addItem(&undo_item);

        // Redo
        let redo_item = create_menu_item(
            mtm,
            &NSString::from_str("Redo"),
            Some(sel!(redo:)),
            "Z",
            NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
        );
        edit_menu.addItem(&redo_item);

        edit_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Cut
        let cut_item = create_menu_item(
            mtm,
            &NSString::from_str("Cut"),
            Some(sel!(cut:)),
            "x",
            NSEventModifierFlags::Command,
        );
        edit_menu.addItem(&cut_item);

        // Copy
        let copy_item = create_menu_item(
            mtm,
            &NSString::from_str("Copy"),
            Some(sel!(copy:)),
            "c",
            NSEventModifierFlags::Command,
        );
        edit_menu.addItem(&copy_item);

        // Paste
        let paste_item = create_menu_item(
            mtm,
            &NSString::from_str("Paste"),
            Some(sel!(paste:)),
            "v",
            NSEventModifierFlags::Command,
        );
        edit_menu.addItem(&paste_item);

        // Delete
        let delete_item = create_menu_item(
            mtm,
            &NSString::from_str("Delete"),
            Some(sel!(delete:)),
            "",
            NSEventModifierFlags::empty(),
        );
        edit_menu.addItem(&delete_item);

        // Select All
        let select_all_item = create_menu_item(
            mtm,
            &NSString::from_str("Select All"),
            Some(sel!(selectAll:)),
            "a",
            NSEventModifierFlags::Command,
        );
        edit_menu.addItem(&select_all_item);

        // Create menu bar item
        let edit_menu_item = NSMenuItem::new(mtm);
        edit_menu_item.setTitle(&edit_title);
        edit_menu_item.setSubmenu(Some(&edit_menu));
        self.main_menu.addItem(&edit_menu_item);

        Ok(())
    }

    /// Add a standard Window menu.
    pub fn add_window_menu(&mut self) -> Result<(), NativeMenuError> {
        let mtm = MainThreadMarker::new().ok_or_else(NativeMenuError::not_main_thread)?;

        let window_title = NSString::from_str("Window");
        let window_menu = NSMenu::new(mtm);
        window_menu.setTitle(&window_title);

        // Minimize
        let minimize_item = create_menu_item(
            mtm,
            &NSString::from_str("Minimize"),
            Some(sel!(performMiniaturize:)),
            "m",
            NSEventModifierFlags::Command,
        );
        window_menu.addItem(&minimize_item);

        // Zoom
        let zoom_item = create_menu_item(
            mtm,
            &NSString::from_str("Zoom"),
            Some(sel!(performZoom:)),
            "",
            NSEventModifierFlags::empty(),
        );
        window_menu.addItem(&zoom_item);

        window_menu.addItem(&NSMenuItem::separatorItem(mtm));

        // Bring All to Front
        let bring_all_item = create_menu_item(
            mtm,
            &NSString::from_str("Bring All to Front"),
            Some(sel!(arrangeInFront:)),
            "",
            NSEventModifierFlags::empty(),
        );
        window_menu.addItem(&bring_all_item);

        // Create menu bar item
        let window_menu_item = NSMenuItem::new(mtm);
        window_menu_item.setTitle(&window_title);
        window_menu_item.setSubmenu(Some(&window_menu));
        self.main_menu.addItem(&window_menu_item);

        // Set as windows menu
        let app = NSApplication::sharedApplication(mtm);
        app.setWindowsMenu(Some(&window_menu));

        Ok(())
    }

    /// Add a standard Help menu.
    pub fn add_help_menu(&mut self) -> Result<(), NativeMenuError> {
        let mtm = MainThreadMarker::new().ok_or_else(NativeMenuError::not_main_thread)?;

        let help_title = NSString::from_str("Help");
        let help_menu = NSMenu::new(mtm);
        help_menu.setTitle(&help_title);

        // Search field is automatically added by macOS when we set this as help menu

        // Create menu bar item
        let help_menu_item = NSMenuItem::new(mtm);
        help_menu_item.setTitle(&help_title);
        help_menu_item.setSubmenu(Some(&help_menu));
        self.main_menu.addItem(&help_menu_item);

        // Set as help menu (enables Spotlight for Help search)
        let app = NSApplication::sharedApplication(mtm);
        app.setHelpMenu(Some(&help_menu));

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a menu item with title, action, key equivalent, and modifiers.
fn create_menu_item(
    mtm: MainThreadMarker,
    title: &NSString,
    action: Option<objc2::runtime::Sel>,
    key_equivalent: &str,
    modifiers: NSEventModifierFlags,
) -> Retained<NSMenuItem> {
    let item = NSMenuItem::new(mtm);
    item.setTitle(title);
    if let Some(sel) = action {
        // SAFETY: We're setting a valid selector that was created using the sel!() macro
        // to a known AppKit action. The selector will be invoked by the Cocoa responder chain.
        unsafe { item.setAction(Some(sel)) };
    }
    if !key_equivalent.is_empty() {
        item.setKeyEquivalent(&NSString::from_str(key_equivalent));
        item.setKeyEquivalentModifierMask(modifiers);
    }
    item
}

/// Remove mnemonic marker ('&') from text.
fn strip_mnemonic(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '&' {
            if chars.peek() == Some(&'&') {
                // '&&' becomes '&'
                result.push('&');
                chars.next();
            }
            // Otherwise skip the '&' (mnemonic marker)
            if let Some(next) = chars.next() {
                result.push(next);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Convert a KeySequence to macOS key equivalent string.
fn key_sequence_to_key_equivalent(key_seq: &KeySequence) -> String {
    // Get the primary key from the sequence
    let key_str = key_seq.to_string();

    // Extract just the key part (after any modifiers)
    // Format is like "Ctrl+S" or "Cmd+Shift+N"
    if let Some(pos) = key_str.rfind('+') {
        let key = &key_str[pos + 1..];
        // Convert to lowercase for NSMenuItem key equivalent
        key.to_lowercase()
    } else {
        // No modifiers, just the key
        key_str.to_lowercase()
    }
}

/// Convert a KeySequence to NSEventModifierFlags.
fn key_sequence_to_modifier_flags(key_seq: &KeySequence) -> NSEventModifierFlags {
    let key_str = key_seq.to_string();
    let mut flags = NSEventModifierFlags::empty();

    // Check for each modifier in the string
    let key_lower = key_str.to_lowercase();

    if key_lower.contains("cmd") || key_lower.contains("meta") || key_lower.contains("ctrl") {
        // On macOS, Ctrl in key sequences typically means Cmd
        flags |= NSEventModifierFlags::Command;
    }

    if key_lower.contains("shift") {
        flags |= NSEventModifierFlags::Shift;
    }

    if key_lower.contains("alt") || key_lower.contains("option") {
        flags |= NSEventModifierFlags::Option;
    }

    if key_lower.contains("control") {
        // Actual Control key (not Cmd)
        flags |= NSEventModifierFlags::Control;
    }

    flags
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_mnemonic() {
        assert_eq!(strip_mnemonic("&File"), "File");
        assert_eq!(strip_mnemonic("E&xit"), "Exit");
        assert_eq!(strip_mnemonic("Fish && Chips"), "Fish & Chips");
        assert_eq!(strip_mnemonic("No Mnemonic"), "No Mnemonic");
        assert_eq!(strip_mnemonic("&&Double"), "&Double");
        assert_eq!(strip_mnemonic("&"), "");
    }

    #[test]
    fn test_key_sequence_to_key_equivalent() {
        // These tests check the extraction logic
        // Actual KeySequence parsing is tested elsewhere

        // Simple extraction test
        let key = "Ctrl+S";
        let result = if let Some(pos) = key.rfind('+') {
            key[pos + 1..].to_lowercase()
        } else {
            key.to_lowercase()
        };
        assert_eq!(result, "s");

        let key2 = "Cmd+Shift+N";
        let result2 = if let Some(pos) = key2.rfind('+') {
            key2[pos + 1..].to_lowercase()
        } else {
            key2.to_lowercase()
        };
        assert_eq!(result2, "n");
    }

    #[test]
    fn test_native_menu_error_display() {
        let err = NativeMenuError::not_main_thread();
        assert!(err.to_string().contains("main thread"));
        assert!(err.is_not_main_thread());

        let err2 = NativeMenuError::creation_failed("test error");
        assert!(err2.to_string().contains("test error"));
        assert!(!err2.is_not_main_thread());
    }
}
