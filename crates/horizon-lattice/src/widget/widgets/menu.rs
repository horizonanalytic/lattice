//! Menu widget implementation.
//!
//! This module provides [`Menu`], a popup widget that displays a list of actions,
//! separators, and submenus.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Action, Menu};
//! use std::sync::Arc;
//!
//! // Create a menu
//! let mut menu = Menu::new();
//!
//! // Add actions
//! let open_action = Arc::new(Action::new("&Open"));
//! let save_action = Arc::new(Action::new("&Save"));
//! let quit_action = Arc::new(Action::new("&Quit"));
//!
//! menu.add_action(open_action.clone());
//! menu.add_action(save_action.clone());
//! menu.add_separator();
//! menu.add_action(quit_action.clone());
//!
//! // Connect to triggered signal
//! menu.triggered.connect(|action| {
//!     println!("Action triggered: {}", action.display_text());
//! });
//!
//! // Show the menu
//! menu.popup_at(100.0, 100.0);
//! ```

use std::sync::Arc;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, Size, Stroke, TextLayout,
    TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent, PaintContext,
    SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

use super::{Action, Popup, PopupFlags, PopupPlacement};

// ============================================================================
// MenuItem
// ============================================================================

/// An item in a menu.
///
/// Menu items can be actions (clickable items), separators (visual dividers),
/// or submenus (nested menus).
#[derive(Clone)]
pub enum MenuItem {
    /// A clickable action item.
    Action(Arc<Action>),
    /// A visual separator line.
    Separator,
    /// A submenu that opens on hover.
    Submenu {
        /// The title of the submenu.
        title: String,
        /// The icon for the submenu (optional).
        icon: Option<horizon_lattice_render::Icon>,
        /// The submenu itself.
        menu: Arc<Menu>,
    },
}

impl MenuItem {
    /// Create an action item.
    pub fn action(action: Arc<Action>) -> Self {
        MenuItem::Action(action)
    }

    /// Create a separator item.
    pub fn separator() -> Self {
        MenuItem::Separator
    }

    /// Create a submenu item.
    pub fn submenu(title: impl Into<String>, menu: Arc<Menu>) -> Self {
        MenuItem::Submenu {
            title: title.into(),
            icon: None,
            menu,
        }
    }

    /// Create a submenu item with an icon.
    pub fn submenu_with_icon(
        title: impl Into<String>,
        icon: horizon_lattice_render::Icon,
        menu: Arc<Menu>,
    ) -> Self {
        MenuItem::Submenu {
            title: title.into(),
            icon: Some(icon),
            menu,
        }
    }

    /// Check if this item is a separator.
    pub fn is_separator(&self) -> bool {
        matches!(self, MenuItem::Separator)
    }

    /// Check if this item is an action.
    pub fn is_action(&self) -> bool {
        matches!(self, MenuItem::Action(_))
    }

    /// Check if this item is a submenu.
    pub fn is_submenu(&self) -> bool {
        matches!(self, MenuItem::Submenu { .. })
    }

    /// Check if this item is enabled.
    pub fn is_enabled(&self) -> bool {
        match self {
            MenuItem::Action(action) => action.is_enabled(),
            MenuItem::Separator => false, // Separators are not interactive
            MenuItem::Submenu { menu, .. } => menu.has_enabled_items(),
        }
    }

    /// Check if this item is visible.
    pub fn is_visible(&self) -> bool {
        match self {
            MenuItem::Action(action) => action.is_visible(),
            MenuItem::Separator => true,
            MenuItem::Submenu { menu, .. } => menu.has_visible_items(),
        }
    }

    /// Get the display text for this item.
    pub fn display_text(&self) -> Option<String> {
        match self {
            MenuItem::Action(action) => Some(action.display_text()),
            MenuItem::Separator => None,
            MenuItem::Submenu { title, .. } => Some(title.clone()),
        }
    }

    /// Get the mnemonic character for this item.
    pub fn mnemonic(&self) -> Option<char> {
        match self {
            MenuItem::Action(action) => action.mnemonic(),
            MenuItem::Separator => None,
            MenuItem::Submenu { title, .. } => {
                // Parse mnemonic from title
                crate::widget::shortcut::parse_mnemonic(title).mnemonic
            }
        }
    }
}

// ============================================================================
// Menu Style
// ============================================================================

/// Style configuration for menu appearance.
#[derive(Clone)]
pub struct MenuStyle {
    /// Background color.
    pub background_color: Color,
    /// Border color.
    pub border_color: Color,
    /// Text color.
    pub text_color: Color,
    /// Disabled text color.
    pub disabled_text_color: Color,
    /// Highlight background color.
    pub highlight_color: Color,
    /// Highlight text color.
    pub highlight_text_color: Color,
    /// Separator color.
    pub separator_color: Color,
    /// Checkmark color.
    pub checkmark_color: Color,
    /// Shortcut text color.
    pub shortcut_color: Color,
    /// Item height.
    pub item_height: f32,
    /// Separator height.
    pub separator_height: f32,
    /// Left padding (for icons/checkmarks).
    pub left_padding: f32,
    /// Right padding (for shortcuts/arrows).
    pub right_padding: f32,
    /// Text left margin (after icon area).
    pub text_margin: f32,
    /// Icon size.
    pub icon_size: f32,
    /// Border width.
    pub border_width: f32,
    /// Menu padding (around all items).
    pub padding: f32,
    /// Submenu arrow width.
    pub arrow_width: f32,
    /// Font for menu text.
    pub font: Font,
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self {
            background_color: Color::WHITE,
            border_color: Color::from_rgb8(180, 180, 180),
            text_color: Color::BLACK,
            disabled_text_color: Color::from_rgb8(128, 128, 128),
            highlight_color: Color::from_rgb8(0, 120, 215),
            highlight_text_color: Color::WHITE,
            separator_color: Color::from_rgb8(200, 200, 200),
            checkmark_color: Color::BLACK,
            shortcut_color: Color::from_rgb8(100, 100, 100),
            item_height: 24.0,
            separator_height: 9.0,
            left_padding: 28.0,
            right_padding: 16.0,
            text_margin: 4.0,
            icon_size: 16.0,
            border_width: 1.0,
            padding: 4.0,
            arrow_width: 8.0,
            font: Font::new(FontFamily::SansSerif, 13.0),
        }
    }
}

// ============================================================================
// Menu
// ============================================================================

/// A popup menu widget.
///
/// Menu displays a list of actions, separators, and submenus. It supports
/// keyboard navigation, mnemonics (accelerator keys), and nested submenus.
///
/// # Features
///
/// - Action items with text, icons, shortcuts, and checkable state
/// - Separator items for visual grouping
/// - Nested submenus
/// - Keyboard navigation (arrow keys, Enter, Escape)
/// - Mnemonic keys (Alt+letter when menu is open)
/// - Auto-close when clicking outside or losing focus
///
/// # Signals
///
/// - [`triggered`](Menu::triggered): Emitted when an action is triggered
/// - [`about_to_show`](Menu::about_to_show): Emitted before the menu is shown
/// - [`about_to_hide`](Menu::about_to_hide): Emitted before the menu is hidden
pub struct Menu {
    /// Widget base.
    base: WidgetBase,

    /// Menu items.
    items: Vec<MenuItem>,

    /// Currently selected item index (None if no selection).
    selected_index: Option<usize>,

    /// Currently open submenu index (None if no submenu open).
    open_submenu_index: Option<usize>,

    /// Parent menu (for submenu chains).
    parent_menu: Option<ObjectId>,

    /// Menu title (optional, used for menu bar integration).
    title: String,

    /// Menu style.
    style: MenuStyle,

    /// Whether mnemonics are active (show underlines).
    mnemonics_active: bool,

    /// Internal popup for positioning/display.
    popup: Popup,

    // Signals
    /// Signal emitted when an action is triggered.
    pub triggered: Signal<Arc<Action>>,
    /// Signal emitted before the menu is shown.
    pub about_to_show: Signal<()>,
    /// Signal emitted before the menu is hidden.
    pub about_to_hide: Signal<()>,
}

impl Menu {
    /// Create a new empty menu.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Preferred,
        ));
        base.hide();

        let popup = Popup::new().with_flags(
            PopupFlags::STAYS_ON_TOP
                | PopupFlags::BORDER
                | PopupFlags::FOCUS_ON_SHOW
                | PopupFlags::AUTO_CLOSE_ON_CLICK_OUTSIDE
                | PopupFlags::CLOSE_ON_ESCAPE,
        );

        Self {
            base,
            items: Vec::new(),
            selected_index: None,
            open_submenu_index: None,
            parent_menu: None,
            title: String::new(),
            style: MenuStyle::default(),
            mnemonics_active: false,
            popup,
            triggered: Signal::new(),
            about_to_show: Signal::new(),
            about_to_hide: Signal::new(),
        }
    }

    /// Create a new menu with a title.
    pub fn with_title(title: impl Into<String>) -> Self {
        let mut menu = Self::new();
        menu.title = title.into();
        menu
    }

    // =========================================================================
    // Builder Pattern
    // =========================================================================

    /// Set the menu style using builder pattern.
    pub fn with_style(mut self, style: MenuStyle) -> Self {
        self.style = style;
        self
    }

    // =========================================================================
    // Items
    // =========================================================================

    /// Add an action to the menu.
    pub fn add_action(&mut self, action: Arc<Action>) {
        self.items.push(MenuItem::Action(action));
        self.update_size();
    }

    /// Add a separator to the menu.
    pub fn add_separator(&mut self) {
        self.items.push(MenuItem::Separator);
        self.update_size();
    }

    /// Add a submenu to the menu.
    pub fn add_submenu(&mut self, title: impl Into<String>, menu: Arc<Menu>) {
        self.items.push(MenuItem::Submenu {
            title: title.into(),
            icon: None,
            menu,
        });
        self.update_size();
    }

    /// Add a menu item.
    pub fn add_item(&mut self, item: MenuItem) {
        self.items.push(item);
        self.update_size();
    }

    /// Insert an action at a specific index.
    pub fn insert_action(&mut self, index: usize, action: Arc<Action>) {
        let index = index.min(self.items.len());
        self.items.insert(index, MenuItem::Action(action));
        self.update_size();
    }

    /// Insert a separator at a specific index.
    pub fn insert_separator(&mut self, index: usize) {
        let index = index.min(self.items.len());
        self.items.insert(index, MenuItem::Separator);
        self.update_size();
    }

    /// Remove an item at a specific index.
    pub fn remove_item(&mut self, index: usize) -> Option<MenuItem> {
        if index < self.items.len() {
            let item = self.items.remove(index);
            self.update_size();
            Some(item)
        } else {
            None
        }
    }

    /// Clear all items from the menu.
    pub fn clear(&mut self) {
        self.items.clear();
        self.selected_index = None;
        self.open_submenu_index = None;
        self.update_size();
    }

    /// Get the number of items in the menu.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Get the items in the menu.
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    /// Check if the menu is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Check if the menu has any enabled items.
    pub fn has_enabled_items(&self) -> bool {
        self.items.iter().any(|item| item.is_enabled())
    }

    /// Check if the menu has any visible items.
    pub fn has_visible_items(&self) -> bool {
        self.items.iter().any(|item| item.is_visible())
    }

    // =========================================================================
    // Title
    // =========================================================================

    /// Get the menu title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the menu title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
    }

    // =========================================================================
    // Style
    // =========================================================================

    /// Get the menu style.
    pub fn style(&self) -> &MenuStyle {
        &self.style
    }

    /// Set the menu style.
    pub fn set_style(&mut self, style: MenuStyle) {
        self.style = style;
        self.update_size();
        self.base.update();
    }

    // =========================================================================
    // Selection
    // =========================================================================

    /// Get the currently selected item index.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Set the selected item index.
    pub fn set_selected_index(&mut self, index: Option<usize>) {
        let valid_index = index.filter(|&i| i < self.items.len());
        if self.selected_index != valid_index {
            // Close any open submenu if selection changes
            if self.open_submenu_index.is_some() && self.open_submenu_index != valid_index {
                self.close_submenu();
            }
            self.selected_index = valid_index;
            self.base.update();
        }
    }

    /// Select the next enabled item.
    pub fn select_next(&mut self) {
        let start = self.selected_index.map(|i| i + 1).unwrap_or(0);
        self.select_next_from(start);
    }

    /// Select the previous enabled item.
    pub fn select_previous(&mut self) {
        let start = self.selected_index.unwrap_or(self.items.len());
        self.select_previous_from(start);
    }

    /// Select the first enabled item.
    pub fn select_first(&mut self) {
        self.select_next_from(0);
    }

    /// Select the last enabled item.
    pub fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.select_previous_from(self.items.len());
        }
    }

    fn select_next_from(&mut self, start: usize) {
        let count = self.items.len();
        if count == 0 {
            return;
        }

        for offset in 0..count {
            let index = (start + offset) % count;
            if self.is_item_selectable(index) {
                self.set_selected_index(Some(index));
                return;
            }
        }
    }

    fn select_previous_from(&mut self, start: usize) {
        let count = self.items.len();
        if count == 0 {
            return;
        }

        for offset in 1..=count {
            let index = (start + count - offset) % count;
            if self.is_item_selectable(index) {
                self.set_selected_index(Some(index));
                return;
            }
        }
    }

    fn is_item_selectable(&self, index: usize) -> bool {
        self.items
            .get(index)
            .map(|item| !item.is_separator() && item.is_visible() && item.is_enabled())
            .unwrap_or(false)
    }

    // =========================================================================
    // Submenu Management
    // =========================================================================

    /// Open the submenu at the given index.
    pub fn open_submenu(&mut self, index: usize) {
        if let Some(MenuItem::Submenu { .. }) = self.items.get(index) {
            self.open_submenu_index = Some(index);
            self.base.update();
        }
    }

    /// Close any open submenu.
    pub fn close_submenu(&mut self) {
        if self.open_submenu_index.is_some() {
            self.open_submenu_index = None;
            self.base.update();
        }
    }

    /// Check if a submenu is open.
    pub fn is_submenu_open(&self) -> bool {
        self.open_submenu_index.is_some()
    }

    // =========================================================================
    // Show/Hide
    // =========================================================================

    /// Show the menu at the specified position.
    pub fn popup_at(&mut self, x: f32, y: f32) {
        self.about_to_show.emit(());
        self.base.set_pos(Point::new(x, y));
        self.base.show();
        self.selected_index = None;
        self.mnemonics_active = true;
        self.base.update();
    }

    /// Show the menu relative to an anchor rectangle.
    pub fn popup_relative_to(&mut self, anchor_rect: Rect, placement: PopupPlacement) {
        self.about_to_show.emit(());
        let size = self.base.size();
        let pos = placement.calculate_position(anchor_rect, size, None);
        self.base.set_pos(pos);
        self.base.show();
        self.selected_index = None;
        self.mnemonics_active = true;
        self.base.update();
    }

    /// Hide the menu.
    pub fn hide(&mut self) {
        if self.base.is_visible() {
            self.about_to_hide.emit(());
            self.close_submenu();
            self.base.hide();
            self.selected_index = None;
        }
    }

    /// Close the menu (same as hide but also closes parent menus).
    pub fn close(&mut self) {
        self.hide();
    }

    /// Check if the menu is visible.
    pub fn is_visible(&self) -> bool {
        self.base.is_visible()
    }

    // =========================================================================
    // Action Triggering
    // =========================================================================

    /// Trigger the currently selected action.
    pub fn trigger_selected(&mut self) {
        if let Some(index) = self.selected_index {
            self.trigger_item(index);
        }
    }

    /// Trigger the action at the given index.
    pub fn trigger_item(&mut self, index: usize) {
        if let Some(item) = self.items.get(index) {
            match item {
                MenuItem::Action(action) if action.is_enabled() => {
                    action.trigger();
                    self.triggered.emit(action.clone());
                    self.close();
                }
                MenuItem::Submenu { .. } => {
                    // Open the submenu
                    self.open_submenu(index);
                }
                _ => {}
            }
        }
    }

    // =========================================================================
    // Size Calculation
    // =========================================================================

    fn update_size(&mut self) {
        let size = self.calculate_preferred_size();
        self.base.set_size(size);
    }

    fn calculate_preferred_size(&self) -> Size {
        let mut width = 0.0f32;
        let mut height = self.style.padding * 2.0;

        for item in &self.items {
            match item {
                MenuItem::Separator => {
                    height += self.style.separator_height;
                }
                MenuItem::Action(action) => {
                    if !action.is_visible() {
                        continue;
                    }
                    height += self.style.item_height;

                    // Calculate width needed for this item
                    let text_width = self.estimate_text_width(&action.display_text());
                    let shortcut_width = action
                        .shortcut()
                        .map(|s| self.estimate_text_width(&s.to_string()) + 20.0)
                        .unwrap_or(0.0);
                    let item_width = self.style.left_padding
                        + text_width
                        + self.style.text_margin * 2.0
                        + shortcut_width
                        + self.style.right_padding;
                    width = width.max(item_width);
                }
                MenuItem::Submenu { title, .. } => {
                    height += self.style.item_height;
                    let text = crate::widget::shortcut::parse_mnemonic(title).display_text;
                    let text_width = self.estimate_text_width(&text);
                    let item_width = self.style.left_padding
                        + text_width
                        + self.style.text_margin * 2.0
                        + self.style.arrow_width
                        + self.style.right_padding;
                    width = width.max(item_width);
                }
            }
        }

        // Add border
        width += self.style.border_width * 2.0;
        height += self.style.border_width * 2.0;

        // Minimum width
        width = width.max(100.0);

        Size::new(width, height)
    }

    fn estimate_text_width(&self, text: &str) -> f32 {
        // Simple estimation: ~7 pixels per character
        text.len() as f32 * 7.0
    }

    // =========================================================================
    // Geometry
    // =========================================================================

    fn item_rect(&self, index: usize) -> Option<Rect> {
        let mut y = self.style.border_width + self.style.padding;

        for (i, item) in self.items.iter().enumerate() {
            let height = match item {
                MenuItem::Separator => self.style.separator_height,
                _ => self.style.item_height,
            };

            if i == index {
                let rect = self.base.rect();
                return Some(Rect::new(
                    self.style.border_width,
                    y,
                    rect.width() - self.style.border_width * 2.0,
                    height,
                ));
            }

            y += height;
        }

        None
    }

    fn item_at_position(&self, pos: Point) -> Option<usize> {
        let mut y = self.style.border_width + self.style.padding;
        let rect = self.base.rect();
        let content_width = rect.width() - self.style.border_width * 2.0;

        for (i, item) in self.items.iter().enumerate() {
            let height = match item {
                MenuItem::Separator => self.style.separator_height,
                _ => self.style.item_height,
            };

            let item_rect = Rect::new(self.style.border_width, y, content_width, height);
            if item_rect.contains(pos) {
                return Some(i);
            }

            y += height;
        }

        None
    }

    // =========================================================================
    // Mnemonic Handling
    // =========================================================================

    fn find_mnemonic_item(&self, key: Key) -> Option<usize> {
        let target_char = key.to_ascii_char()?;

        for (i, item) in self.items.iter().enumerate() {
            if !item.is_visible() || item.is_separator() {
                continue;
            }

            if let Some(mnemonic) = item.mnemonic()
                && mnemonic == target_char
                && item.is_enabled()
            {
                return Some(i);
            }
        }

        None
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;
        let rect = self.base.rect();
        let local_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());

        // Click outside closes the menu
        if !local_rect.contains(pos) {
            self.close();
            return true;
        }

        // Click on an item
        if let Some(index) = self.item_at_position(pos) {
            self.set_selected_index(Some(index));
            self.trigger_item(index);
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;

        if let Some(index) = self.item_at_position(pos)
            && self.is_item_selectable(index)
        {
            self.set_selected_index(Some(index));

            // Open submenu on hover (with a delay in a full implementation)
            if let Some(MenuItem::Submenu { .. }) = self.items.get(index) {
                self.open_submenu(index);
            }

            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::ArrowDown => {
                self.select_next();
                true
            }
            Key::ArrowUp => {
                self.select_previous();
                true
            }
            Key::ArrowRight => {
                // Open submenu if selected item is a submenu
                if let Some(index) = self.selected_index
                    && let Some(MenuItem::Submenu { .. }) = self.items.get(index)
                {
                    self.open_submenu(index);
                    return true;
                }
                false
            }
            Key::ArrowLeft => {
                // Close submenu and return to parent
                if self.open_submenu_index.is_some() {
                    self.close_submenu();
                    return true;
                }
                // If we have a parent menu, close this menu
                if self.parent_menu.is_some() {
                    self.close();
                    return true;
                }
                false
            }
            Key::Enter | Key::Space => {
                self.trigger_selected();
                true
            }
            Key::Escape => {
                self.close();
                true
            }
            Key::Home => {
                self.select_first();
                true
            }
            Key::End => {
                self.select_last();
                true
            }
            _ => {
                // Check for mnemonic
                if self.mnemonics_active
                    && let Some(index) = self.find_mnemonic_item(event.key)
                {
                    self.set_selected_index(Some(index));
                    self.trigger_item(index);
                    return true;
                }
                false
            }
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let local_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());
        ctx.renderer()
            .fill_rect(local_rect, self.style.background_color);
    }

    fn paint_border(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let border_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());
        let stroke = Stroke::new(self.style.border_color, self.style.border_width);
        ctx.renderer().stroke_rect(border_rect, &stroke);
    }

    fn paint_items(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let mut y = self.style.border_width + self.style.padding;

        for (i, item) in self.items.iter().enumerate() {
            match item {
                MenuItem::Separator => {
                    self.paint_separator(ctx, y);
                    y += self.style.separator_height;
                }
                MenuItem::Action(action) => {
                    if !action.is_visible() {
                        continue;
                    }
                    let item_rect = Rect::new(
                        self.style.border_width,
                        y,
                        rect.width() - self.style.border_width * 2.0,
                        self.style.item_height,
                    );
                    let selected = self.selected_index == Some(i);
                    self.paint_action_item(ctx, action, item_rect, selected);
                    y += self.style.item_height;
                }
                MenuItem::Submenu { title, icon, .. } => {
                    let item_rect = Rect::new(
                        self.style.border_width,
                        y,
                        rect.width() - self.style.border_width * 2.0,
                        self.style.item_height,
                    );
                    let selected = self.selected_index == Some(i);
                    self.paint_submenu_item(ctx, title, icon.as_ref(), item_rect, selected);
                    y += self.style.item_height;
                }
            }
        }
    }

    fn paint_separator(&self, ctx: &mut PaintContext<'_>, y: f32) {
        let rect = self.base.rect();
        let line_y = y + self.style.separator_height / 2.0;
        let x1 = self.style.border_width + self.style.padding;
        let x2 = rect.width() - self.style.border_width - self.style.padding;

        let stroke = Stroke::new(self.style.separator_color, 1.0);
        ctx.renderer()
            .draw_line(Point::new(x1, line_y), Point::new(x2, line_y), &stroke);
    }

    fn paint_action_item(
        &self,
        ctx: &mut PaintContext<'_>,
        action: &Action,
        rect: Rect,
        selected: bool,
    ) {
        let enabled = action.is_enabled();

        // Highlight background
        if selected && enabled {
            ctx.renderer().fill_rect(rect, self.style.highlight_color);
        }

        // Text color
        let text_color = if !enabled {
            self.style.disabled_text_color
        } else if selected {
            self.style.highlight_text_color
        } else {
            self.style.text_color
        };

        // Checkmark for checkable actions
        if action.is_checkable() && action.is_checked() {
            let check_x = rect.origin.x + (self.style.left_padding - self.style.icon_size) / 2.0;
            let check_y = rect.origin.y + (rect.height() - self.style.icon_size) / 2.0;
            let check_color = if selected {
                self.style.highlight_text_color
            } else {
                self.style.checkmark_color
            };
            self.paint_checkmark(ctx, check_x, check_y, check_color);
        }

        // Icon - draw a placeholder rectangle if icon exists
        // (Full icon rendering requires image handling)
        if action.is_icon_visible_in_menu() && action.icon().is_some() {
            let icon_x = rect.origin.x + (self.style.left_padding - self.style.icon_size) / 2.0;
            let icon_y = rect.origin.y + (rect.height() - self.style.icon_size) / 2.0;
            let icon_rect = Rect::new(icon_x, icon_y, self.style.icon_size, self.style.icon_size);
            let icon_color = if enabled {
                Color::from_rgb8(100, 100, 100)
            } else {
                Color::from_rgb8(180, 180, 180)
            };
            ctx.renderer().fill_rect(icon_rect, icon_color);
        }

        // Text position
        let text_x = rect.origin.x + self.style.left_padding + self.style.text_margin;
        let text_y = rect.origin.y + (rect.height() - self.style.font.size()) / 2.0;
        let display_text = action.display_text();

        // Draw text using TextLayout
        self.paint_text_layout(ctx, &display_text, text_x, text_y, text_color);

        // Draw mnemonic underline if active
        if self.mnemonics_active
            && let Some(mnemonic_index) = action.mnemonic_index()
        {
            self.paint_mnemonic_underline(ctx, mnemonic_index, text_x, text_y, text_color);
        }

        // Shortcut text
        if let Some(shortcut) = action.shortcut() {
            let shortcut_text = shortcut.to_string();
            let shortcut_width = self.estimate_text_width(&shortcut_text);
            let shortcut_x =
                rect.origin.x + rect.width() - self.style.right_padding - shortcut_width;
            let shortcut_color = if selected && enabled {
                self.style.highlight_text_color
            } else {
                self.style.shortcut_color
            };
            self.paint_text_layout(ctx, &shortcut_text, shortcut_x, text_y, shortcut_color);
        }
    }

    fn paint_submenu_item(
        &self,
        ctx: &mut PaintContext<'_>,
        title: &str,
        _icon: Option<&horizon_lattice_render::Icon>,
        rect: Rect,
        selected: bool,
    ) {
        // Highlight background
        if selected {
            ctx.renderer().fill_rect(rect, self.style.highlight_color);
        }

        let text_color = if selected {
            self.style.highlight_text_color
        } else {
            self.style.text_color
        };

        // Text with mnemonic
        let parsed = crate::widget::shortcut::parse_mnemonic(title);
        let text_x = rect.origin.x + self.style.left_padding + self.style.text_margin;
        let text_y = rect.origin.y + (rect.height() - self.style.font.size()) / 2.0;

        self.paint_text_layout(ctx, &parsed.display_text, text_x, text_y, text_color);

        if self.mnemonics_active
            && let Some(mnemonic_index) = parsed.mnemonic_index
        {
            self.paint_mnemonic_underline(ctx, mnemonic_index, text_x, text_y, text_color);
        }

        // Submenu arrow
        let arrow_x =
            rect.origin.x + rect.width() - self.style.right_padding - self.style.arrow_width;
        let arrow_y = rect.origin.y + rect.height() / 2.0;
        self.paint_submenu_arrow(ctx, arrow_x, arrow_y, text_color);
    }

    fn paint_checkmark(&self, ctx: &mut PaintContext<'_>, x: f32, y: f32, color: Color) {
        let size = self.style.icon_size;
        let stroke = Stroke::new(color, 2.0);

        // Draw a simple checkmark
        let p1 = Point::new(x + size * 0.2, y + size * 0.5);
        let p2 = Point::new(x + size * 0.4, y + size * 0.7);
        let p3 = Point::new(x + size * 0.8, y + size * 0.3);

        ctx.renderer().draw_line(p1, p2, &stroke);
        ctx.renderer().draw_line(p2, p3, &stroke);
    }

    fn paint_submenu_arrow(&self, ctx: &mut PaintContext<'_>, x: f32, y: f32, color: Color) {
        let stroke = Stroke::new(color, 1.5);

        // Draw a simple arrow pointing right
        let p1 = Point::new(x, y - 4.0);
        let p2 = Point::new(x + 4.0, y);
        let p3 = Point::new(x, y + 4.0);

        ctx.renderer().draw_line(p1, p2, &stroke);
        ctx.renderer().draw_line(p2, p3, &stroke);
    }

    fn paint_text_layout(
        &self,
        _ctx: &mut PaintContext<'_>,
        text: &str,
        x: f32,
        y: f32,
        color: Color,
    ) {
        // Text rendering uses the TextLayout and TextRenderer system.
        // Create layout and prepare for rendering.
        let mut font_system = FontSystem::new();
        let layout = TextLayout::new(&mut font_system, text, &self.style.font);

        // Prepare glyphs for rendering
        let position = Point::new(x, y);
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(&mut font_system, &layout, position, color);
            // Note: Actual glyph rendering requires integration with the render pass.
            // The prepared glyphs are submitted during the frame render.
        }
    }

    fn paint_mnemonic_underline(
        &self,
        ctx: &mut PaintContext<'_>,
        char_index: usize,
        text_x: f32,
        text_y: f32,
        color: Color,
    ) {
        // Estimate character position for underline
        let char_width = self.style.font.size() * 0.6; // Approximate
        let underline_x = text_x + char_index as f32 * char_width;
        let underline_y = text_y + self.style.font.size() + 2.0;

        let stroke = Stroke::new(color, 1.0);
        ctx.renderer().draw_line(
            Point::new(underline_x, underline_y),
            Point::new(underline_x + char_width, underline_y),
            &stroke,
        );
    }
}

impl Widget for Menu {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let preferred = self.calculate_preferred_size();
        SizeHint::new(preferred).with_minimum(Size::new(50.0, 20.0))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        if !self.base.is_visible() {
            return;
        }

        self.paint_background(ctx);
        self.paint_border(ctx);
        self.paint_items(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::MouseRelease(_) => false,
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::Leave(_) => {
                // Don't clear selection on leave to allow submenu hover
                false
            }
            WidgetEvent::FocusOut(_) => {
                self.close();
                true
            }
            _ => false,
        }
    }
}

impl Object for Menu {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

// Menu is Send + Sync (all fields are thread-safe)
unsafe impl Send for Menu {}
unsafe impl Sync for Menu {}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    #[test]
    fn test_menu_new() {
        init_global_registry();
        let menu = Menu::new();
        assert!(menu.is_empty());
        assert_eq!(menu.item_count(), 0);
    }

    #[test]
    fn test_menu_add_action() {
        init_global_registry();
        let mut menu = Menu::new();
        let action = Arc::new(Action::new("&Open"));

        menu.add_action(action.clone());

        assert_eq!(menu.item_count(), 1);
        assert!(!menu.is_empty());
    }

    #[test]
    fn test_menu_add_separator() {
        init_global_registry();
        let mut menu = Menu::new();

        menu.add_separator();

        assert_eq!(menu.item_count(), 1);
        assert!(menu.items()[0].is_separator());
    }

    #[test]
    fn test_menu_add_submenu() {
        init_global_registry();
        let mut menu = Menu::new();
        let submenu = Arc::new(Menu::new());

        menu.add_submenu("&File", submenu);

        assert_eq!(menu.item_count(), 1);
        assert!(menu.items()[0].is_submenu());
    }

    #[test]
    fn test_menu_item_enum() {
        init_global_registry();
        let action = Arc::new(Action::new("Test"));

        let item_action = MenuItem::action(action.clone());
        assert!(item_action.is_action());
        assert!(!item_action.is_separator());
        assert!(!item_action.is_submenu());

        let item_sep = MenuItem::separator();
        assert!(!item_sep.is_action());
        assert!(item_sep.is_separator());
        assert!(!item_sep.is_submenu());

        let submenu = Arc::new(Menu::new());
        let item_submenu = MenuItem::submenu("Sub", submenu);
        assert!(!item_submenu.is_action());
        assert!(!item_submenu.is_separator());
        assert!(item_submenu.is_submenu());
    }

    #[test]
    fn test_menu_selection() {
        init_global_registry();
        let mut menu = Menu::new();
        let action1 = Arc::new(Action::new("Item 1"));
        let action2 = Arc::new(Action::new("Item 2"));
        let action3 = Arc::new(Action::new("Item 3"));

        menu.add_action(action1);
        menu.add_action(action2);
        menu.add_action(action3);

        assert_eq!(menu.selected_index(), None);

        menu.select_first();
        assert_eq!(menu.selected_index(), Some(0));

        menu.select_next();
        assert_eq!(menu.selected_index(), Some(1));

        menu.select_last();
        assert_eq!(menu.selected_index(), Some(2));

        menu.select_previous();
        assert_eq!(menu.selected_index(), Some(1));
    }

    #[test]
    fn test_menu_selection_skips_separators() {
        init_global_registry();
        let mut menu = Menu::new();
        let action1 = Arc::new(Action::new("Item 1"));
        let action2 = Arc::new(Action::new("Item 2"));

        menu.add_action(action1);
        menu.add_separator();
        menu.add_action(action2);

        menu.select_first();
        assert_eq!(menu.selected_index(), Some(0));

        menu.select_next();
        // Should skip separator at index 1, select item at index 2
        assert_eq!(menu.selected_index(), Some(2));
    }

    #[test]
    fn test_menu_selection_skips_disabled() {
        init_global_registry();
        let mut menu = Menu::new();
        let action1 = Arc::new(Action::new("Item 1"));
        let action2 = Arc::new(Action::new("Item 2").with_enabled(false));
        let action3 = Arc::new(Action::new("Item 3"));

        menu.add_action(action1);
        menu.add_action(action2);
        menu.add_action(action3);

        menu.select_first();
        assert_eq!(menu.selected_index(), Some(0));

        menu.select_next();
        // Should skip disabled item at index 1
        assert_eq!(menu.selected_index(), Some(2));
    }

    #[test]
    fn test_menu_mnemonic() {
        init_global_registry();
        let item = MenuItem::action(Arc::new(Action::new("&Open")));
        assert_eq!(item.mnemonic(), Some('o'));

        let item_no_mnemonic = MenuItem::action(Arc::new(Action::new("Save")));
        assert_eq!(item_no_mnemonic.mnemonic(), None);
    }

    #[test]
    fn test_menu_clear() {
        init_global_registry();
        let mut menu = Menu::new();
        menu.add_action(Arc::new(Action::new("Item 1")));
        menu.add_action(Arc::new(Action::new("Item 2")));

        assert_eq!(menu.item_count(), 2);

        menu.clear();

        assert_eq!(menu.item_count(), 0);
        assert!(menu.is_empty());
    }

    #[test]
    fn test_menu_style() {
        init_global_registry();
        let style = MenuStyle {
            item_height: 30.0,
            ..Default::default()
        };
        let menu = Menu::new().with_style(style.clone());

        assert_eq!(menu.style().item_height, 30.0);
    }
}
