//! Menu bar widget implementation.
//!
//! This module provides [`MenuBar`], a horizontal menu bar widget that displays
//! menu titles and opens dropdown menus when clicked.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Action, Menu, MenuBar};
//! use std::sync::Arc;
//!
//! // Create menus
//! let mut file_menu = Menu::with_title("&File");
//! file_menu.add_action(Arc::new(Action::new("&Open")));
//! file_menu.add_action(Arc::new(Action::new("&Save")));
//!
//! let mut edit_menu = Menu::with_title("&Edit");
//! edit_menu.add_action(Arc::new(Action::new("&Undo")));
//! edit_menu.add_action(Arc::new(Action::new("&Redo")));
//!
//! // Create menu bar and add menus
//! let mut menu_bar = MenuBar::new();
//! menu_bar.add_menu("&File", Arc::new(file_menu));
//! menu_bar.add_menu("&Edit", Arc::new(edit_menu));
//!
//! // Connect to triggered signal
//! menu_bar.triggered.connect(|action| {
//!     println!("Action triggered: {}", action.display_text());
//! });
//! ```

use std::sync::Arc;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, Point, Rect, Renderer, Size, Stroke, TextLayout,
    TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

use super::{Action, Menu};

// ============================================================================
// MenuBarItem
// ============================================================================

/// An item in the menu bar (a menu with its title).
struct MenuBarItem {
    /// The title text (may contain '&' for mnemonic).
    title: String,
    /// The display text (with '&' markers removed).
    display_text: String,
    /// The mnemonic character (lowercase), if any.
    mnemonic: Option<char>,
    /// The index in display_text where the mnemonic is located.
    mnemonic_index: Option<usize>,
    /// The associated menu.
    menu: Arc<Menu>,
    /// Whether this menu is visible.
    visible: bool,
    /// Whether this menu is enabled.
    enabled: bool,
}

impl MenuBarItem {
    /// Create a new menu bar item.
    fn new(title: impl Into<String>, menu: Arc<Menu>) -> Self {
        let title = title.into();
        let parsed = crate::widget::shortcut::parse_mnemonic(&title);

        Self {
            title,
            display_text: parsed.display_text,
            mnemonic: parsed.mnemonic,
            mnemonic_index: parsed.mnemonic_index,
            menu,
            visible: true,
            enabled: true,
        }
    }
}

// ============================================================================
// MenuBarStyle
// ============================================================================

/// Style configuration for menu bar appearance.
#[derive(Clone)]
pub struct MenuBarStyle {
    /// Background color.
    pub background_color: Color,
    /// Text color.
    pub text_color: Color,
    /// Disabled text color.
    pub disabled_text_color: Color,
    /// Highlight background color (when menu is open or hovered).
    pub highlight_color: Color,
    /// Highlight text color.
    pub highlight_text_color: Color,
    /// Border color (bottom border).
    pub border_color: Color,
    /// Menu bar height.
    pub height: f32,
    /// Horizontal padding for each menu title.
    pub item_padding: f32,
    /// Border width.
    pub border_width: f32,
    /// Font for menu titles.
    pub font: Font,
}

impl Default for MenuBarStyle {
    fn default() -> Self {
        Self {
            background_color: Color::from_rgb8(240, 240, 240),
            text_color: Color::BLACK,
            disabled_text_color: Color::from_rgb8(128, 128, 128),
            highlight_color: Color::from_rgb8(0, 120, 215),
            highlight_text_color: Color::WHITE,
            border_color: Color::from_rgb8(200, 200, 200),
            height: 24.0,
            item_padding: 12.0,
            border_width: 1.0,
            font: Font::new(FontFamily::SansSerif, 13.0),
        }
    }
}

// ============================================================================
// MenuBar
// ============================================================================

/// A horizontal menu bar widget.
///
/// MenuBar displays a row of menu titles that open dropdown menus when clicked.
/// It supports keyboard navigation using Alt+letter mnemonics and arrow keys.
///
/// # Features
///
/// - Click a menu title to open its dropdown menu
/// - Hover to switch between menus while one is open
/// - Keyboard navigation with Alt+letter, arrow keys, and Escape
/// - Optional corner widget for extra controls
///
/// # Signals
///
/// - [`triggered`](MenuBar::triggered): Emitted when an action is triggered
/// - [`hovered`](MenuBar::hovered): Emitted when an action is hovered
pub struct MenuBar {
    /// Widget base.
    base: WidgetBase,

    /// Menu items.
    items: Vec<MenuBarItem>,

    /// Currently highlighted menu index (when navigating with keyboard).
    highlighted_index: Option<usize>,

    /// Currently open menu index.
    open_menu_index: Option<usize>,

    /// Whether a menu is currently open.
    menu_open: bool,

    /// Whether mnemonics should be shown (Alt key held or menu bar activated).
    mnemonics_visible: bool,

    /// Corner widget (optional, placed at the right side).
    corner_widget: Option<ObjectId>,

    /// Menu bar style.
    style: MenuBarStyle,

    // Signals
    /// Signal emitted when an action is triggered.
    pub triggered: Signal<Arc<Action>>,
    /// Signal emitted when an action is hovered.
    pub hovered: Signal<Arc<Action>>,
}

impl MenuBar {
    /// Create a new empty menu bar.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Fixed,
        ));

        Self {
            base,
            items: Vec::new(),
            highlighted_index: None,
            open_menu_index: None,
            menu_open: false,
            mnemonics_visible: false,
            corner_widget: None,
            style: MenuBarStyle::default(),
            triggered: Signal::new(),
            hovered: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Pattern
    // =========================================================================

    /// Set the menu bar style using builder pattern.
    pub fn with_style(mut self, style: MenuBarStyle) -> Self {
        self.style = style;
        self
    }

    /// Set the corner widget using builder pattern.
    pub fn with_corner_widget(mut self, widget_id: ObjectId) -> Self {
        self.corner_widget = Some(widget_id);
        self
    }

    // =========================================================================
    // Menu Management
    // =========================================================================

    /// Add a menu to the menu bar.
    ///
    /// Note: The menu's `triggered` signal is not automatically connected.
    /// Connect to the menu bar's `triggered` signal to receive action events,
    /// or connect directly to each menu's `triggered` signal.
    pub fn add_menu(&mut self, title: impl Into<String>, menu: Arc<Menu>) {
        let item = MenuBarItem::new(title, menu);
        self.items.push(item);
        self.base.update();
    }

    /// Insert a menu at a specific index.
    pub fn insert_menu(&mut self, index: usize, title: impl Into<String>, menu: Arc<Menu>) {
        let insert_index = index.min(self.items.len());
        let item = MenuBarItem::new(title, menu);
        self.items.insert(insert_index, item);
        self.base.update();
    }

    /// Remove a menu at a specific index.
    pub fn remove_menu(&mut self, index: usize) -> Option<Arc<Menu>> {
        if index < self.items.len() {
            let item = self.items.remove(index);
            self.base.update();
            Some(item.menu)
        } else {
            None
        }
    }

    /// Clear all menus from the menu bar.
    pub fn clear(&mut self) {
        self.items.clear();
        self.highlighted_index = None;
        self.close_menu();
        self.base.update();
    }

    /// Get the number of menus in the menu bar.
    pub fn menu_count(&self) -> usize {
        self.items.len()
    }

    /// Check if the menu bar is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get a menu by index.
    pub fn menu(&self, index: usize) -> Option<&Arc<Menu>> {
        self.items.get(index).map(|item| &item.menu)
    }

    /// Set visibility of a menu item.
    pub fn set_menu_visible(&mut self, index: usize, visible: bool) {
        if let Some(item) = self.items.get_mut(index)
            && item.visible != visible {
                item.visible = visible;
                self.base.update();
            }
    }

    /// Set enabled state of a menu item.
    pub fn set_menu_enabled(&mut self, index: usize, enabled: bool) {
        if let Some(item) = self.items.get_mut(index)
            && item.enabled != enabled {
                item.enabled = enabled;
                self.base.update();
            }
    }

    // =========================================================================
    // Corner Widget
    // =========================================================================

    /// Get the corner widget ID, if any.
    pub fn corner_widget(&self) -> Option<ObjectId> {
        self.corner_widget
    }

    /// Set the corner widget (displayed at the right side of the menu bar).
    pub fn set_corner_widget(&mut self, widget_id: Option<ObjectId>) {
        self.corner_widget = widget_id;
        self.base.update();
    }

    // =========================================================================
    // Style
    // =========================================================================

    /// Get the menu bar style.
    pub fn style(&self) -> &MenuBarStyle {
        &self.style
    }

    /// Set the menu bar style.
    pub fn set_style(&mut self, style: MenuBarStyle) {
        self.style = style;
        self.base.update();
    }

    // =========================================================================
    // Menu Opening/Closing
    // =========================================================================

    /// Open the menu at the given index.
    pub fn open_menu(&mut self, index: usize) {
        if index >= self.items.len() {
            return;
        }

        let item = &self.items[index];
        if !item.visible || !item.enabled {
            return;
        }

        // Close any currently open menu
        if self.open_menu_index != Some(index) {
            self.close_current_menu();
        }

        // Calculate the position for the popup menu
        if let Some(_rect) = self.item_rect(index) {
            // In a full implementation, the popup menu would be shown at the calculated position.
            // The anchor rect would be:
            // let menu_bar_pos = self.base.geometry().origin;
            // let anchor = Rect::new(
            //     menu_bar_pos.x + rect.origin.x,
            //     menu_bar_pos.y + rect.origin.y,
            //     rect.width(),
            //     rect.height(),
            // );
            // menu.popup_relative_to(anchor, PopupPlacement::Below);

            // For now, we just track the state
            self.open_menu_index = Some(index);
            self.highlighted_index = Some(index);
            self.menu_open = true;
            self.mnemonics_visible = true;
        }

        self.base.update();
    }

    /// Close the currently open menu.
    pub fn close_menu(&mut self) {
        self.close_current_menu();
        self.menu_open = false;
        self.mnemonics_visible = false;
        self.highlighted_index = None;
        self.base.update();
    }

    fn close_current_menu(&mut self) {
        if self.open_menu_index.is_some() {
            // In a full implementation, we'd call menu.hide() here
            self.open_menu_index = None;
        }
    }

    /// Check if any menu is currently open.
    pub fn is_menu_open(&self) -> bool {
        self.menu_open
    }

    // =========================================================================
    // Navigation
    // =========================================================================

    /// Highlight the next visible and enabled menu.
    pub fn highlight_next(&mut self) {
        let start = self.highlighted_index.map(|i| i + 1).unwrap_or(0);
        self.highlight_from(start, true);
    }

    /// Highlight the previous visible and enabled menu.
    pub fn highlight_previous(&mut self) {
        let start = self.highlighted_index.unwrap_or(self.items.len());
        self.highlight_from(start, false);
    }

    fn highlight_from(&mut self, start: usize, forward: bool) {
        let count = self.items.len();
        if count == 0 {
            return;
        }

        for offset in 0..count {
            let index = if forward {
                (start + offset) % count
            } else {
                (start + count - offset - 1) % count
            };

            if self.is_item_highlightable(index) {
                self.set_highlighted_index(Some(index));
                return;
            }
        }
    }

    fn is_item_highlightable(&self, index: usize) -> bool {
        self.items
            .get(index)
            .map(|item| item.visible && item.enabled)
            .unwrap_or(false)
    }

    fn set_highlighted_index(&mut self, index: Option<usize>) {
        if self.highlighted_index != index {
            self.highlighted_index = index;

            // If a menu is open, switch to the newly highlighted menu
            if self.menu_open
                && let Some(idx) = index {
                    self.open_menu(idx);
                }

            self.base.update();
        }
    }

    // =========================================================================
    // Geometry
    // =========================================================================

    fn item_rect(&self, index: usize) -> Option<Rect> {
        let mut x = 0.0;

        for (i, item) in self.items.iter().enumerate() {
            if !item.visible {
                continue;
            }

            let width =
                self.estimate_text_width(&item.display_text) + self.style.item_padding * 2.0;

            if i == index {
                return Some(Rect::new(x, 0.0, width, self.style.height));
            }

            x += width;
        }

        None
    }

    fn item_at_position(&self, pos: Point) -> Option<usize> {
        let mut x = 0.0;

        for (i, item) in self.items.iter().enumerate() {
            if !item.visible {
                continue;
            }

            let width =
                self.estimate_text_width(&item.display_text) + self.style.item_padding * 2.0;
            let rect = Rect::new(x, 0.0, width, self.style.height);

            if rect.contains(pos) {
                return Some(i);
            }

            x += width;
        }

        None
    }

    fn estimate_text_width(&self, text: &str) -> f32 {
        // Simple estimation: ~7 pixels per character at font size 13
        text.len() as f32 * (self.style.font.size() * 0.6)
    }

    fn calculate_preferred_width(&self) -> f32 {
        let mut width = 0.0;

        for item in &self.items {
            if !item.visible {
                continue;
            }
            width += self.estimate_text_width(&item.display_text) + self.style.item_padding * 2.0;
        }

        width.max(100.0)
    }

    // =========================================================================
    // Mnemonic Handling
    // =========================================================================

    fn find_mnemonic_item(&self, key: Key) -> Option<usize> {
        let target_char = key.to_ascii_char()?;

        for (i, item) in self.items.iter().enumerate() {
            if !item.visible || !item.enabled {
                continue;
            }

            if let Some(mnemonic) = item.mnemonic
                && mnemonic == target_char {
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

        if let Some(index) = self.item_at_position(pos) {
            if self.is_item_highlightable(index) {
                if self.menu_open && self.open_menu_index == Some(index) {
                    // Clicking the same menu title closes it
                    self.close_menu();
                } else {
                    // Open the clicked menu
                    self.open_menu(index);
                }
                return true;
            }
        } else if self.menu_open {
            // Clicking outside the menu bar closes the menu
            self.close_menu();
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;

        if let Some(index) = self.item_at_position(pos)
            && self.is_item_highlightable(index) {
                self.set_highlighted_index(Some(index));
                return true;
            }

        false
    }

    fn handle_mouse_release(&mut self, _event: &MouseReleaseEvent) -> bool {
        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::ArrowLeft => {
                if self.menu_open || self.highlighted_index.is_some() {
                    self.highlight_previous();
                    return true;
                }
            }
            Key::ArrowRight => {
                if self.menu_open || self.highlighted_index.is_some() {
                    self.highlight_next();
                    return true;
                }
            }
            Key::ArrowDown => {
                // Open the currently highlighted menu
                if let Some(index) = self.highlighted_index {
                    if !self.menu_open {
                        self.open_menu(index);
                    }
                    return true;
                }
            }
            Key::Enter | Key::Space => {
                if let Some(index) = self.highlighted_index {
                    if self.menu_open {
                        // Let the menu handle it
                    } else {
                        self.open_menu(index);
                    }
                    return true;
                }
            }
            Key::Escape => {
                if self.menu_open {
                    self.close_menu();
                    return true;
                }
                // Clear keyboard navigation state
                if self.highlighted_index.is_some() {
                    self.highlighted_index = None;
                    self.mnemonics_visible = false;
                    self.base.update();
                    return true;
                }
            }
            Key::AltLeft | Key::AltRight => {
                // Toggle mnemonic visibility and keyboard navigation mode
                if !self.menu_open {
                    self.mnemonics_visible = !self.mnemonics_visible;
                    if self.mnemonics_visible && self.highlighted_index.is_none() {
                        // Highlight the first menu
                        self.highlight_from(0, true);
                    } else if !self.mnemonics_visible {
                        self.highlighted_index = None;
                    }
                    self.base.update();
                    return true;
                }
            }
            _ => {
                // Check for mnemonic key (with Alt held or when mnemonics are visible)
                if (event.modifiers.alt || self.mnemonics_visible)
                    && let Some(index) = self.find_mnemonic_item(event.key) {
                        self.open_menu(index);
                        return true;
                    }
            }
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        ctx.renderer().fill_rect(rect, self.style.background_color);
    }

    fn paint_border(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        // Draw bottom border
        let stroke = Stroke::new(self.style.border_color, self.style.border_width);
        let y = rect.height() - self.style.border_width / 2.0;
        ctx.renderer()
            .draw_line(Point::new(0.0, y), Point::new(rect.width(), y), &stroke);
    }

    fn paint_items(&self, ctx: &mut PaintContext<'_>) {
        let mut x = 0.0;

        for (i, item) in self.items.iter().enumerate() {
            if !item.visible {
                continue;
            }

            let width =
                self.estimate_text_width(&item.display_text) + self.style.item_padding * 2.0;
            let item_rect = Rect::new(x, 0.0, width, self.style.height);

            let is_highlighted = self.highlighted_index == Some(i);
            let is_open = self.open_menu_index == Some(i);

            self.paint_item(ctx, item, item_rect, is_highlighted || is_open);

            x += width;
        }
    }

    fn paint_item(
        &self,
        ctx: &mut PaintContext<'_>,
        item: &MenuBarItem,
        rect: Rect,
        highlighted: bool,
    ) {
        let enabled = item.enabled;

        // Highlight background
        if highlighted && enabled {
            ctx.renderer().fill_rect(rect, self.style.highlight_color);
        }

        // Text color
        let text_color = if !enabled {
            self.style.disabled_text_color
        } else if highlighted {
            self.style.highlight_text_color
        } else {
            self.style.text_color
        };

        // Text position (centered vertically)
        let text_x = rect.origin.x + self.style.item_padding;
        let text_y = rect.origin.y + (rect.height() - self.style.font.size()) / 2.0;

        // Draw text
        self.paint_text(ctx, &item.display_text, text_x, text_y, text_color);

        // Draw mnemonic underline if visible
        if (self.mnemonics_visible || ctx.is_alt_held())
            && let Some(mnemonic_index) = item.mnemonic_index {
                self.paint_mnemonic_underline(ctx, mnemonic_index, text_x, text_y, text_color);
            }
    }

    fn paint_text(&self, _ctx: &mut PaintContext<'_>, text: &str, x: f32, y: f32, color: Color) {
        // Text rendering using TextLayout and TextRenderer
        let mut font_system = FontSystem::new();
        let layout = TextLayout::new(&mut font_system, text, &self.style.font);

        let position = Point::new(x, y);
        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(&mut font_system, &layout, position, color);
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
        let char_width = self.style.font.size() * 0.6;
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

impl Widget for MenuBar {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let preferred = Size::new(self.calculate_preferred_width(), self.style.height);
        SizeHint::new(preferred).with_minimum(Size::new(50.0, self.style.height))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);
        self.paint_items(ctx);
        self.paint_border(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::Leave(_) => {
                // Only clear highlight if menu is not open
                if !self.menu_open {
                    self.highlighted_index = None;
                    self.base.update();
                }
                false
            }
            WidgetEvent::FocusOut(_) => {
                // Close menu and clear state when focus is lost
                self.close_menu();
                self.highlighted_index = None;
                self.base.update();
                false
            }
            _ => false,
        }
    }
}

impl Object for MenuBar {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Default for MenuBar {
    fn default() -> Self {
        Self::new()
    }
}

// MenuBar is Send + Sync (all fields are thread-safe)
unsafe impl Send for MenuBar {}
unsafe impl Sync for MenuBar {}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    #[test]
    fn test_menu_bar_new() {
        init_global_registry();
        let menu_bar = MenuBar::new();
        assert!(menu_bar.is_empty());
        assert_eq!(menu_bar.menu_count(), 0);
    }

    #[test]
    fn test_menu_bar_add_menu() {
        init_global_registry();
        let mut menu_bar = MenuBar::new();
        let menu = Arc::new(Menu::new());

        menu_bar.add_menu("&File", menu.clone());

        assert_eq!(menu_bar.menu_count(), 1);
        assert!(!menu_bar.is_empty());
    }

    #[test]
    fn test_menu_bar_insert_menu() {
        init_global_registry();
        let mut menu_bar = MenuBar::new();
        let file_menu = Arc::new(Menu::new());
        let edit_menu = Arc::new(Menu::new());
        let view_menu = Arc::new(Menu::new());

        menu_bar.add_menu("&File", file_menu);
        menu_bar.add_menu("&View", view_menu);
        menu_bar.insert_menu(1, "&Edit", edit_menu);

        assert_eq!(menu_bar.menu_count(), 3);
    }

    #[test]
    fn test_menu_bar_remove_menu() {
        init_global_registry();
        let mut menu_bar = MenuBar::new();
        let menu = Arc::new(Menu::new());

        menu_bar.add_menu("&File", menu);
        assert_eq!(menu_bar.menu_count(), 1);

        menu_bar.remove_menu(0);
        assert_eq!(menu_bar.menu_count(), 0);
    }

    #[test]
    fn test_menu_bar_clear() {
        init_global_registry();
        let mut menu_bar = MenuBar::new();

        menu_bar.add_menu("&File", Arc::new(Menu::new()));
        menu_bar.add_menu("&Edit", Arc::new(Menu::new()));
        assert_eq!(menu_bar.menu_count(), 2);

        menu_bar.clear();
        assert_eq!(menu_bar.menu_count(), 0);
        assert!(menu_bar.is_empty());
    }

    #[test]
    fn test_menu_bar_mnemonic_parsing() {
        init_global_registry();
        let item = MenuBarItem::new("&File", Arc::new(Menu::new()));

        assert_eq!(item.display_text, "File");
        assert_eq!(item.mnemonic, Some('f'));
        assert_eq!(item.mnemonic_index, Some(0));
    }

    #[test]
    fn test_menu_bar_mnemonic_middle() {
        init_global_registry();
        let item = MenuBarItem::new("Hel&p", Arc::new(Menu::new()));

        assert_eq!(item.display_text, "Help");
        assert_eq!(item.mnemonic, Some('p'));
        assert_eq!(item.mnemonic_index, Some(3));
    }

    #[test]
    fn test_menu_bar_no_mnemonic() {
        init_global_registry();
        let item = MenuBarItem::new("File", Arc::new(Menu::new()));

        assert_eq!(item.display_text, "File");
        assert_eq!(item.mnemonic, None);
        assert_eq!(item.mnemonic_index, None);
    }

    #[test]
    fn test_menu_bar_style() {
        init_global_registry();
        let style = MenuBarStyle {
            height: 30.0,
            ..Default::default()
        };
        let menu_bar = MenuBar::new().with_style(style);

        assert_eq!(menu_bar.style().height, 30.0);
    }

    #[test]
    fn test_menu_bar_visibility() {
        init_global_registry();
        let mut menu_bar = MenuBar::new();
        menu_bar.add_menu("&File", Arc::new(Menu::new()));

        menu_bar.set_menu_visible(0, false);
        // The menu is still in the list, just not visible
        assert_eq!(menu_bar.menu_count(), 1);
    }

    #[test]
    fn test_menu_bar_enabled() {
        init_global_registry();
        let mut menu_bar = MenuBar::new();
        menu_bar.add_menu("&File", Arc::new(Menu::new()));

        menu_bar.set_menu_enabled(0, false);
        // The menu should not be highlightable when disabled
        assert!(!menu_bar.is_item_highlightable(0));
    }
}
