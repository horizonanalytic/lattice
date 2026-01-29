//! TabBar widget implementation.
//!
//! This module provides [`TabBar`], a standalone tab bar widget that can be used
//! independently or as part of a [`TabWidget`].
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{TabBar, TabPosition};
//!
//! // Create a horizontal tab bar
//! let mut tab_bar = TabBar::new()
//!     .with_tab_position(TabPosition::Top)
//!     .with_tabs_closable(true);
//!
//! // Add some tabs
//! tab_bar.add_tab("Home");
//! tab_bar.add_tab("Settings");
//! tab_bar.add_tab("Help");
//!
//! // Connect to tab changes
//! tab_bar.current_changed.connect(|&index| {
//!     println!("Switched to tab: {}", index);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, CornerRadii, Font, FontFamily, FontSystem, Icon, ImageScaleMode, Point, Rect, Renderer,
    RoundedRect, Stroke, TextLayout, TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

/// Tab position relative to the content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabPosition {
    /// Tabs at the top (horizontal layout).
    #[default]
    Top,
    /// Tabs at the bottom (horizontal layout).
    Bottom,
    /// Tabs on the left (vertical layout).
    Left,
    /// Tabs on the right (vertical layout).
    Right,
}

impl TabPosition {
    /// Returns true if the tab position results in a horizontal tab layout.
    pub fn is_horizontal(&self) -> bool {
        matches!(self, TabPosition::Top | TabPosition::Bottom)
    }

    /// Returns true if the tab position results in a vertical tab layout.
    pub fn is_vertical(&self) -> bool {
        matches!(self, TabPosition::Left | TabPosition::Right)
    }
}

/// Information about a single tab.
#[derive(Clone)]
struct TabItem {
    /// Tab label text.
    label: String,
    /// Optional icon for the tab.
    icon: Option<Icon>,
    /// Whether the tab is enabled.
    enabled: bool,
    /// Whether this tab can be closed.
    closable: bool,
    /// Custom tooltip text.
    tooltip: Option<String>,
}

impl TabItem {
    fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            enabled: true,
            closable: false,
            tooltip: None,
        }
    }
}

/// Which part of the tab bar was hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TabBarPart {
    #[default]
    None,
    /// A tab at the given index.
    Tab(usize),
    /// The close button on a tab.
    CloseButton(usize),
    /// The scroll left/up button.
    ScrollDecrease,
    /// The scroll right/down button.
    ScrollIncrease,
}

/// A standalone tab bar widget.
///
/// TabBar displays a row or column of tabs that can be clicked to switch between pages.
/// It supports:
/// - Horizontal (top/bottom) and vertical (left/right) orientations
/// - Closable tabs with close buttons
/// - Keyboard navigation
/// - Tab reordering via drag and drop
/// - Overflow scrolling when there are too many tabs
///
/// # Signals
///
/// - `current_changed(i32)`: Emitted when the current tab changes
/// - `tab_close_requested(i32)`: Emitted when a tab's close button is clicked
/// - `tab_moved(i32, i32)`: Emitted when a tab is moved (from_index, to_index)
pub struct TabBar {
    /// Widget base.
    base: WidgetBase,

    /// List of tabs.
    tabs: Vec<TabItem>,

    /// Currently selected tab index (-1 if no tabs).
    current_index: i32,

    /// Tab position (affects layout direction).
    tab_position: TabPosition,

    /// Whether tabs can be closed by default.
    tabs_closable: bool,

    /// Whether tabs can be reordered by dragging.
    tabs_movable: bool,

    /// Whether to expand tabs to fill available space.
    expanding: bool,

    /// Scroll offset for overflow handling.
    scroll_offset: f32,

    /// Whether scroll buttons are needed.
    overflow: bool,

    /// Hovered tab part.
    hover_part: TabBarPart,

    /// Pressed tab part.
    pressed_part: TabBarPart,

    /// Drag state for tab reordering.
    dragging: Option<DragState>,

    /// Font for tab labels.
    font: Font,

    /// Tab dimensions.
    tab_height: f32,
    tab_min_width: f32,
    tab_max_width: f32,
    tab_padding: f32,

    /// Icon size for tab icons.
    icon_size: f32,
    /// Spacing between icon and text.
    icon_spacing: f32,

    /// Close button size.
    close_button_size: f32,

    /// Scroll button size.
    scroll_button_size: f32,

    /// Colors.
    background_color: Color,
    tab_color: Color,
    tab_hover_color: Color,
    tab_selected_color: Color,
    tab_disabled_color: Color,
    text_color: Color,
    text_disabled_color: Color,
    close_button_color: Color,
    close_button_hover_color: Color,
    border_color: Color,

    /// Border radius.
    border_radius: f32,

    /// Signal emitted when current tab changes.
    pub current_changed: Signal<i32>,

    /// Signal emitted when a tab close is requested.
    pub tab_close_requested: Signal<i32>,

    /// Signal emitted when a tab is moved.
    pub tab_moved: Signal<(i32, i32)>,
}

/// State for drag-to-reorder.
#[derive(Debug, Clone)]
struct DragState {
    /// Index of tab being dragged.
    tab_index: usize,
    /// Original position when drag started.
    start_pos: Point,
    /// Current drag position.
    current_pos: Point,
    /// Whether we've moved enough to start reordering.
    active: bool,
}

impl TabBar {
    /// Create a new tab bar.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Fixed,
        ));

        Self {
            base,
            tabs: Vec::new(),
            current_index: -1,
            tab_position: TabPosition::Top,
            tabs_closable: false,
            tabs_movable: true,
            expanding: false,
            scroll_offset: 0.0,
            overflow: false,
            hover_part: TabBarPart::None,
            pressed_part: TabBarPart::None,
            dragging: None,
            font: Font::new(FontFamily::SansSerif, 13.0),
            tab_height: 32.0,
            tab_min_width: 80.0,
            tab_max_width: 200.0,
            tab_padding: 12.0,
            icon_size: 16.0,
            icon_spacing: 6.0,
            close_button_size: 16.0,
            scroll_button_size: 20.0,
            background_color: Color::from_rgb8(245, 245, 245),
            tab_color: Color::from_rgb8(245, 245, 245),
            tab_hover_color: Color::from_rgb8(230, 230, 230),
            tab_selected_color: Color::WHITE,
            tab_disabled_color: Color::from_rgb8(220, 220, 220),
            text_color: Color::from_rgb8(50, 50, 50),
            text_disabled_color: Color::from_rgb8(150, 150, 150),
            close_button_color: Color::from_rgb8(120, 120, 120),
            close_button_hover_color: Color::from_rgb8(180, 60, 60),
            border_color: Color::from_rgb8(200, 200, 200),
            border_radius: 4.0,
            current_changed: Signal::new(),
            tab_close_requested: Signal::new(),
            tab_moved: Signal::new(),
        }
    }

    // =========================================================================
    // Tab Management
    // =========================================================================

    /// Add a new tab with the given label.
    ///
    /// Returns the index of the new tab.
    pub fn add_tab(&mut self, label: impl Into<String>) -> i32 {
        let index = self.tabs.len() as i32;
        let mut item = TabItem::new(label);
        item.closable = self.tabs_closable;
        self.tabs.push(item);

        // Select first tab automatically
        if self.current_index < 0 {
            self.current_index = 0;
            self.current_changed.emit(0);
        }

        self.update_overflow();
        self.base.update();
        index
    }

    /// Add a new tab with label and icon.
    pub fn add_tab_with_icon(&mut self, label: impl Into<String>, icon: Icon) -> i32 {
        let index = self.add_tab(label);
        if let Some(tab) = self.tabs.get_mut(index as usize) {
            tab.icon = Some(icon);
        }
        index
    }

    /// Insert a tab at the specified index.
    pub fn insert_tab(&mut self, index: i32, label: impl Into<String>) -> i32 {
        let index = (index as usize).min(self.tabs.len());
        let mut item = TabItem::new(label);
        item.closable = self.tabs_closable;
        self.tabs.insert(index, item);

        // Adjust current index if needed
        if self.current_index >= index as i32 {
            self.current_index += 1;
        }

        // Select first tab automatically
        if self.current_index < 0 && !self.tabs.is_empty() {
            self.current_index = 0;
            self.current_changed.emit(0);
        }

        self.update_overflow();
        self.base.update();
        index as i32
    }

    /// Remove the tab at the specified index.
    pub fn remove_tab(&mut self, index: i32) {
        if index < 0 || index as usize >= self.tabs.len() {
            return;
        }

        self.tabs.remove(index as usize);

        // Adjust current index
        if self.tabs.is_empty() {
            self.current_index = -1;
            self.current_changed.emit(-1);
        } else if index <= self.current_index {
            let new_index = (self.current_index - 1)
                .max(0)
                .min(self.tabs.len() as i32 - 1);
            if new_index != self.current_index {
                self.current_index = new_index;
                self.current_changed.emit(new_index);
            }
        }

        self.update_overflow();
        self.base.update();
    }

    /// Move a tab from one index to another.
    pub fn move_tab(&mut self, from: i32, to: i32) {
        if from < 0 || from as usize >= self.tabs.len() {
            return;
        }
        if to < 0 || to as usize >= self.tabs.len() {
            return;
        }
        if from == to {
            return;
        }

        let tab = self.tabs.remove(from as usize);
        self.tabs.insert(to as usize, tab);

        // Adjust current index to follow the selected tab
        if self.current_index == from {
            self.current_index = to;
        } else if from < self.current_index && to >= self.current_index {
            self.current_index -= 1;
        } else if from > self.current_index && to <= self.current_index {
            self.current_index += 1;
        }

        self.tab_moved.emit((from, to));
        self.base.update();
    }

    /// Clear all tabs.
    pub fn clear(&mut self) {
        self.tabs.clear();
        self.current_index = -1;
        self.scroll_offset = 0.0;
        self.overflow = false;
        self.current_changed.emit(-1);
        self.base.update();
    }

    /// Get the number of tabs.
    pub fn count(&self) -> i32 {
        self.tabs.len() as i32
    }

    // =========================================================================
    // Tab Properties
    // =========================================================================

    /// Get the text of a tab.
    pub fn tab_text(&self, index: i32) -> Option<&str> {
        self.tabs.get(index as usize).map(|t| t.label.as_str())
    }

    /// Set the text of a tab.
    pub fn set_tab_text(&mut self, index: i32, text: impl Into<String>) {
        if let Some(tab) = self.tabs.get_mut(index as usize) {
            tab.label = text.into();
            self.update_overflow();
            self.base.update();
        }
    }

    /// Get whether a tab is enabled.
    pub fn is_tab_enabled(&self, index: i32) -> bool {
        self.tabs
            .get(index as usize)
            .map(|t| t.enabled)
            .unwrap_or(false)
    }

    /// Set whether a tab is enabled.
    pub fn set_tab_enabled(&mut self, index: i32, enabled: bool) {
        if let Some(tab) = self.tabs.get_mut(index as usize) {
            tab.enabled = enabled;
            self.base.update();
        }
    }

    /// Get the tooltip for a tab.
    pub fn tab_tooltip(&self, index: i32) -> Option<&str> {
        self.tabs
            .get(index as usize)
            .and_then(|t| t.tooltip.as_deref())
    }

    /// Set the tooltip for a tab.
    pub fn set_tab_tooltip(&mut self, index: i32, tooltip: impl Into<String>) {
        if let Some(tab) = self.tabs.get_mut(index as usize) {
            tab.tooltip = Some(tooltip.into());
        }
    }

    /// Get whether a specific tab is closable.
    pub fn is_tab_closable(&self, index: i32) -> bool {
        self.tabs
            .get(index as usize)
            .map(|t| t.closable)
            .unwrap_or(false)
    }

    /// Set whether a specific tab is closable.
    pub fn set_tab_closable(&mut self, index: i32, closable: bool) {
        if let Some(tab) = self.tabs.get_mut(index as usize) {
            tab.closable = closable;
            self.update_overflow();
            self.base.update();
        }
    }

    /// Get the icon for a tab.
    pub fn tab_icon(&self, index: i32) -> Option<&Icon> {
        self.tabs.get(index as usize).and_then(|t| t.icon.as_ref())
    }

    /// Set the icon for a tab.
    pub fn set_tab_icon(&mut self, index: i32, icon: Option<Icon>) {
        if let Some(tab) = self.tabs.get_mut(index as usize) {
            tab.icon = icon;
            self.update_overflow();
            self.base.update();
        }
    }

    // =========================================================================
    // Icon Size Configuration
    // =========================================================================

    /// Get the icon size used for tab icons.
    pub fn icon_size(&self) -> f32 {
        self.icon_size
    }

    /// Set the icon size used for tab icons.
    pub fn set_icon_size(&mut self, size: f32) {
        if self.icon_size != size {
            self.icon_size = size.max(8.0);
            self.update_overflow();
            self.base.update();
        }
    }

    /// Set icon size using builder pattern.
    pub fn with_icon_size(mut self, size: f32) -> Self {
        self.icon_size = size.max(8.0);
        self
    }

    /// Get the spacing between icon and text.
    pub fn icon_spacing(&self) -> f32 {
        self.icon_spacing
    }

    /// Set the spacing between icon and text.
    pub fn set_icon_spacing(&mut self, spacing: f32) {
        if self.icon_spacing != spacing {
            self.icon_spacing = spacing.max(0.0);
            self.update_overflow();
            self.base.update();
        }
    }

    /// Set icon spacing using builder pattern.
    pub fn with_icon_spacing(mut self, spacing: f32) -> Self {
        self.icon_spacing = spacing.max(0.0);
        self
    }

    // =========================================================================
    // Current Tab
    // =========================================================================

    /// Get the current tab index.
    pub fn current_index(&self) -> i32 {
        self.current_index
    }

    /// Set the current tab index.
    pub fn set_current_index(&mut self, index: i32) {
        if index < 0 || index as usize >= self.tabs.len() {
            return;
        }

        // Check if tab is enabled
        if !self.tabs[index as usize].enabled {
            return;
        }

        if self.current_index != index {
            self.current_index = index;
            self.ensure_tab_visible(index);
            self.current_changed.emit(index);
            self.base.update();
        }
    }

    // =========================================================================
    // Tab Bar Properties
    // =========================================================================

    /// Get the tab position.
    pub fn tab_position(&self) -> TabPosition {
        self.tab_position
    }

    /// Set the tab position.
    pub fn set_tab_position(&mut self, position: TabPosition) {
        if self.tab_position != position {
            self.tab_position = position;
            // Update size policy based on orientation
            let policy = if position.is_horizontal() {
                SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Fixed)
            } else {
                SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Expanding)
            };
            self.base.set_size_policy(policy);
            self.update_overflow();
            self.base.update();
        }
    }

    /// Set tab position using builder pattern.
    pub fn with_tab_position(mut self, position: TabPosition) -> Self {
        self.set_tab_position(position);
        self
    }

    /// Check if tabs are closable by default.
    pub fn tabs_closable(&self) -> bool {
        self.tabs_closable
    }

    /// Set whether new tabs are closable by default.
    pub fn set_tabs_closable(&mut self, closable: bool) {
        self.tabs_closable = closable;
    }

    /// Set tabs closable using builder pattern.
    pub fn with_tabs_closable(mut self, closable: bool) -> Self {
        self.tabs_closable = closable;
        self
    }

    /// Check if tabs are movable.
    pub fn tabs_movable(&self) -> bool {
        self.tabs_movable
    }

    /// Set whether tabs can be reordered by dragging.
    pub fn set_tabs_movable(&mut self, movable: bool) {
        self.tabs_movable = movable;
    }

    /// Set tabs movable using builder pattern.
    pub fn with_tabs_movable(mut self, movable: bool) -> Self {
        self.tabs_movable = movable;
        self
    }

    /// Check if tabs expand to fill space.
    pub fn expanding(&self) -> bool {
        self.expanding
    }

    /// Set whether tabs expand to fill available space.
    pub fn set_expanding(&mut self, expanding: bool) {
        if self.expanding != expanding {
            self.expanding = expanding;
            self.update_overflow();
            self.base.update();
        }
    }

    /// Set expanding using builder pattern.
    pub fn with_expanding(mut self, expanding: bool) -> Self {
        self.set_expanding(expanding);
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the font.
    pub fn font(&self) -> &Font {
        &self.font
    }

    /// Set the font.
    pub fn set_font(&mut self, font: Font) {
        self.font = font;
        self.update_overflow();
        self.base.update();
    }

    /// Set font using builder pattern.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = font;
        self
    }

    /// Set the tab height.
    pub fn set_tab_height(&mut self, height: f32) {
        self.tab_height = height.max(20.0);
        self.base.update();
    }

    /// Set tab height using builder pattern.
    pub fn with_tab_height(mut self, height: f32) -> Self {
        self.tab_height = height.max(20.0);
        self
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    /// Get the content area (excluding scroll buttons if present).
    fn content_rect(&self) -> Rect {
        let rect = self.base.rect();
        if !self.overflow {
            return rect;
        }

        let btn_size = self.scroll_button_size;
        if self.tab_position.is_horizontal() {
            Rect::new(btn_size, 0.0, rect.width() - 2.0 * btn_size, rect.height())
        } else {
            Rect::new(0.0, btn_size, rect.width(), rect.height() - 2.0 * btn_size)
        }
    }

    /// Calculate the width needed for a tab.
    fn tab_width(&self, index: usize) -> f32 {
        let tab = &self.tabs[index];
        let mut font_system = FontSystem::new();
        let layout = TextLayout::new(&mut font_system, &tab.label, &self.font);
        let text_width = layout.width();

        let icon_width = if tab.icon.is_some() {
            self.icon_size + self.icon_spacing
        } else {
            0.0
        };

        let close_width = if tab.closable {
            self.close_button_size + self.tab_padding / 2.0
        } else {
            0.0
        };

        (text_width + icon_width + 2.0 * self.tab_padding + close_width)
            .clamp(self.tab_min_width, self.tab_max_width)
    }

    /// Calculate the total width/height of all tabs.
    fn total_tabs_size(&self) -> f32 {
        if self.tabs.is_empty() {
            return 0.0;
        }

        let is_horizontal = self.tab_position.is_horizontal();
        let mut total: f32 = 0.0;

        for i in 0..self.tabs.len() {
            if is_horizontal {
                total += self.tab_width(i);
            } else {
                total += self.tab_height;
            }
        }

        total
    }

    /// Get the rectangle for a specific tab (in widget coordinates).
    fn tab_rect(&self, index: usize) -> Option<Rect> {
        if index >= self.tabs.len() {
            return None;
        }

        let content = self.content_rect();
        let is_horizontal = self.tab_position.is_horizontal();

        // Calculate position
        let mut pos = -self.scroll_offset;
        for i in 0..index {
            if is_horizontal {
                pos += self.tab_width(i);
            } else {
                pos += self.tab_height;
            }
        }

        let rect = if is_horizontal {
            let width = if self.expanding && !self.overflow && !self.tabs.is_empty() {
                content.width() / self.tabs.len() as f32
            } else {
                self.tab_width(index)
            };
            Rect::new(
                content.origin.x + pos,
                content.origin.y,
                width,
                self.tab_height,
            )
        } else {
            let height = if self.expanding && !self.overflow && !self.tabs.is_empty() {
                content.height() / self.tabs.len() as f32
            } else {
                self.tab_height
            };
            Rect::new(
                content.origin.x,
                content.origin.y + pos,
                self.tab_height,
                height,
            )
        };

        Some(rect)
    }

    /// Get the rectangle for a tab's close button.
    fn close_button_rect(&self, tab_rect: &Rect) -> Rect {
        let is_horizontal = self.tab_position.is_horizontal();
        let size = self.close_button_size;
        let padding = 4.0;

        if is_horizontal {
            Rect::new(
                tab_rect.origin.x + tab_rect.width() - size - padding,
                tab_rect.origin.y + (tab_rect.height() - size) / 2.0,
                size,
                size,
            )
        } else {
            Rect::new(
                tab_rect.origin.x + (tab_rect.width() - size) / 2.0,
                tab_rect.origin.y + tab_rect.height() - size - padding,
                size,
                size,
            )
        }
    }

    /// Get the scroll decrease button rectangle.
    fn scroll_decrease_rect(&self) -> Option<Rect> {
        if !self.overflow {
            return None;
        }
        let rect = self.base.rect();
        let size = self.scroll_button_size;

        Some(if self.tab_position.is_horizontal() {
            Rect::new(0.0, 0.0, size, rect.height())
        } else {
            Rect::new(0.0, 0.0, rect.width(), size)
        })
    }

    /// Get the scroll increase button rectangle.
    fn scroll_increase_rect(&self) -> Option<Rect> {
        if !self.overflow {
            return None;
        }
        let rect = self.base.rect();
        let size = self.scroll_button_size;

        Some(if self.tab_position.is_horizontal() {
            Rect::new(rect.width() - size, 0.0, size, rect.height())
        } else {
            Rect::new(0.0, rect.height() - size, rect.width(), size)
        })
    }

    /// Hit test to find which part of the tab bar is at a point.
    fn hit_test(&self, pos: Point) -> TabBarPart {
        // Check scroll buttons first
        if let Some(rect) = self.scroll_decrease_rect()
            && rect.contains(pos)
        {
            return TabBarPart::ScrollDecrease;
        }
        if let Some(rect) = self.scroll_increase_rect()
            && rect.contains(pos)
        {
            return TabBarPart::ScrollIncrease;
        }

        // Check tabs
        for i in 0..self.tabs.len() {
            if let Some(tab_rect) = self.tab_rect(i)
                && tab_rect.contains(pos)
            {
                // Check close button
                if self.tabs[i].closable {
                    let close_rect = self.close_button_rect(&tab_rect);
                    if close_rect.contains(pos) {
                        return TabBarPart::CloseButton(i);
                    }
                }
                return TabBarPart::Tab(i);
            }
        }

        TabBarPart::None
    }

    /// Update overflow state based on available space.
    fn update_overflow(&mut self) {
        let rect = self.base.rect();
        let total = self.total_tabs_size();
        let available = if self.tab_position.is_horizontal() {
            rect.width()
        } else {
            rect.height()
        };

        self.overflow = total > available;

        // Clamp scroll offset
        if self.overflow {
            let max_scroll = (total - available + 2.0 * self.scroll_button_size).max(0.0);
            self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);
        } else {
            self.scroll_offset = 0.0;
        }
    }

    /// Scroll to ensure a tab is visible.
    fn ensure_tab_visible(&mut self, index: i32) {
        if !self.overflow || index < 0 || index as usize >= self.tabs.len() {
            return;
        }

        let content = self.content_rect();
        let is_horizontal = self.tab_position.is_horizontal();

        // Calculate tab position
        let mut tab_start = 0.0;
        for i in 0..index as usize {
            if is_horizontal {
                tab_start += self.tab_width(i);
            } else {
                tab_start += self.tab_height;
            }
        }

        let tab_size = if is_horizontal {
            self.tab_width(index as usize)
        } else {
            self.tab_height
        };

        let tab_end = tab_start + tab_size;
        let visible_start = self.scroll_offset;
        let visible_end = self.scroll_offset
            + if is_horizontal {
                content.width()
            } else {
                content.height()
            };

        if tab_start < visible_start {
            self.scroll_offset = tab_start;
        } else if tab_end > visible_end {
            self.scroll_offset = tab_end
                - if is_horizontal {
                    content.width()
                } else {
                    content.height()
                };
        }
    }

    /// Find the tab index at a drag position for reordering.
    fn tab_index_at_drag_pos(&self, pos: Point) -> Option<usize> {
        let is_horizontal = self.tab_position.is_horizontal();
        let coord = if is_horizontal { pos.x } else { pos.y };

        let mut offset = -self.scroll_offset
            + if is_horizontal {
                self.content_rect().origin.x
            } else {
                self.content_rect().origin.y
            };

        for i in 0..self.tabs.len() {
            let size = if is_horizontal {
                self.tab_width(i)
            } else {
                self.tab_height
            };

            if coord < offset + size / 2.0 {
                return Some(i);
            }
            offset += size;
        }

        Some(self.tabs.len().saturating_sub(1))
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
            TabBarPart::Tab(index) => {
                if self.tabs[index].enabled {
                    // Start potential drag
                    if self.tabs_movable {
                        self.dragging = Some(DragState {
                            tab_index: index,
                            start_pos: event.local_pos,
                            current_pos: event.local_pos,
                            active: false,
                        });
                    }
                    self.base.update();
                    return true;
                }
            }
            TabBarPart::CloseButton(_) => {
                self.base.update();
                return true;
            }
            TabBarPart::ScrollDecrease | TabBarPart::ScrollIncrease => {
                self.handle_scroll_click(part);
                return true;
            }
            TabBarPart::None => {}
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let part = self.hit_test(event.local_pos);

        // Handle drag end
        if let Some(drag) = self.dragging.take()
            && drag.active
        {
            // Drag finished - tab was already moved during drag
            self.base.update();
            return true;
        }

        // Handle click
        if self.pressed_part == part {
            match part {
                TabBarPart::Tab(index) => {
                    if self.tabs[index].enabled {
                        self.set_current_index(index as i32);
                    }
                }
                TabBarPart::CloseButton(index) => {
                    self.tab_close_requested.emit(index as i32);
                }
                _ => {}
            }
        }

        self.pressed_part = TabBarPart::None;
        self.base.update();
        true
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        // Update hover state
        let new_hover = self.hit_test(event.local_pos);
        if self.hover_part != new_hover {
            self.hover_part = new_hover;
            self.base.update();
        }

        // Handle dragging
        if self.dragging.is_some() {
            // Extract drag info to avoid borrow issues
            let (current_tab_index, is_active) = {
                let drag = self.dragging.as_mut().unwrap();
                drag.current_pos = event.local_pos;

                // Check if we should start active dragging
                if !drag.active {
                    let is_horizontal = self.tab_position.is_horizontal();
                    let delta = if is_horizontal {
                        (drag.current_pos.x - drag.start_pos.x).abs()
                    } else {
                        (drag.current_pos.y - drag.start_pos.y).abs()
                    };

                    if delta > 5.0 {
                        drag.active = true;
                    }
                }

                (drag.tab_index, drag.active)
            };

            // Perform reordering (self is no longer borrowed)
            if is_active
                && let Some(new_index) = self.tab_index_at_drag_pos(event.local_pos)
                && new_index != current_tab_index
            {
                self.move_tab(current_tab_index as i32, new_index as i32);

                // Update drag state with new index
                if let Some(ref mut drag) = self.dragging {
                    drag.tab_index = new_index;
                }
            }

            self.base.update();
            return true;
        }

        false
    }

    fn handle_scroll_click(&mut self, part: TabBarPart) {
        let scroll_step = 50.0;

        match part {
            TabBarPart::ScrollDecrease => {
                self.scroll_offset = (self.scroll_offset - scroll_step).max(0.0);
            }
            TabBarPart::ScrollIncrease => {
                let max_scroll = self.total_tabs_size()
                    - if self.tab_position.is_horizontal() {
                        self.content_rect().width()
                    } else {
                        self.content_rect().height()
                    };
                self.scroll_offset = (self.scroll_offset + scroll_step).min(max_scroll.max(0.0));
            }
            _ => {}
        }

        self.base.update();
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        if self.tabs.is_empty() {
            return false;
        }

        let (prev_key, next_key) = if self.tab_position.is_horizontal() {
            (Key::ArrowLeft, Key::ArrowRight)
        } else {
            (Key::ArrowUp, Key::ArrowDown)
        };

        match event.key {
            key if key == prev_key => {
                self.select_previous_enabled();
                true
            }
            key if key == next_key => {
                self.select_next_enabled();
                true
            }
            Key::Home => {
                self.select_first_enabled();
                true
            }
            Key::End => {
                self.select_last_enabled();
                true
            }
            _ => false,
        }
    }

    fn handle_leave(&mut self) -> bool {
        if self.hover_part != TabBarPart::None {
            self.hover_part = TabBarPart::None;
            self.base.update();
        }
        false
    }

    // =========================================================================
    // Navigation Helpers
    // =========================================================================

    fn select_next_enabled(&mut self) {
        let start = (self.current_index + 1) as usize;
        for i in start..self.tabs.len() {
            if self.tabs[i].enabled {
                self.set_current_index(i as i32);
                return;
            }
        }
    }

    fn select_previous_enabled(&mut self) {
        if self.current_index <= 0 {
            return;
        }
        for i in (0..self.current_index as usize).rev() {
            if self.tabs[i].enabled {
                self.set_current_index(i as i32);
                return;
            }
        }
    }

    fn select_first_enabled(&mut self) {
        for i in 0..self.tabs.len() {
            if self.tabs[i].enabled {
                self.set_current_index(i as i32);
                return;
            }
        }
    }

    fn select_last_enabled(&mut self) {
        for i in (0..self.tabs.len()).rev() {
            if self.tabs[i].enabled {
                self.set_current_index(i as i32);
                return;
            }
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();
        ctx.renderer().fill_rect(rect, self.background_color);
    }

    fn paint_scroll_buttons(&self, ctx: &mut PaintContext<'_>) {
        if !self.overflow {
            return;
        }

        let is_horizontal = self.tab_position.is_horizontal();

        // Decrease button
        if let Some(rect) = self.scroll_decrease_rect() {
            let color = if self.hover_part == TabBarPart::ScrollDecrease {
                self.tab_hover_color
            } else {
                self.tab_color
            };
            ctx.renderer().fill_rect(rect, color);
            self.paint_scroll_arrow(ctx, rect, false, is_horizontal);
        }

        // Increase button
        if let Some(rect) = self.scroll_increase_rect() {
            let color = if self.hover_part == TabBarPart::ScrollIncrease {
                self.tab_hover_color
            } else {
                self.tab_color
            };
            ctx.renderer().fill_rect(rect, color);
            self.paint_scroll_arrow(ctx, rect, true, is_horizontal);
        }
    }

    fn paint_scroll_arrow(
        &self,
        ctx: &mut PaintContext<'_>,
        rect: Rect,
        increase: bool,
        horizontal: bool,
    ) {
        let center_x = rect.origin.x + rect.width() / 2.0;
        let center_y = rect.origin.y + rect.height() / 2.0;
        let arrow_size = 4.0;
        let stroke = Stroke::new(self.text_color, 1.5);

        match (horizontal, increase) {
            (true, false) => {
                // Left arrow
                let p1 = Point::new(center_x + arrow_size / 2.0, center_y - arrow_size);
                let p2 = Point::new(center_x - arrow_size / 2.0, center_y);
                let p3 = Point::new(center_x + arrow_size / 2.0, center_y + arrow_size);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
            (true, true) => {
                // Right arrow
                let p1 = Point::new(center_x - arrow_size / 2.0, center_y - arrow_size);
                let p2 = Point::new(center_x + arrow_size / 2.0, center_y);
                let p3 = Point::new(center_x - arrow_size / 2.0, center_y + arrow_size);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
            (false, false) => {
                // Up arrow
                let p1 = Point::new(center_x - arrow_size, center_y + arrow_size / 2.0);
                let p2 = Point::new(center_x, center_y - arrow_size / 2.0);
                let p3 = Point::new(center_x + arrow_size, center_y + arrow_size / 2.0);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
            (false, true) => {
                // Down arrow
                let p1 = Point::new(center_x - arrow_size, center_y - arrow_size / 2.0);
                let p2 = Point::new(center_x, center_y + arrow_size / 2.0);
                let p3 = Point::new(center_x + arrow_size, center_y - arrow_size / 2.0);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
        }
    }

    fn paint_tabs(&self, ctx: &mut PaintContext<'_>) {
        for i in 0..self.tabs.len() {
            if let Some(tab_rect) = self.tab_rect(i) {
                // Skip tabs outside visible area
                let content = self.content_rect();
                if self.tab_position.is_horizontal() {
                    if tab_rect.origin.x + tab_rect.width() < content.origin.x
                        || tab_rect.origin.x > content.origin.x + content.width()
                    {
                        continue;
                    }
                } else if tab_rect.origin.y + tab_rect.height() < content.origin.y
                    || tab_rect.origin.y > content.origin.y + content.height()
                {
                    continue;
                }

                self.paint_tab(ctx, i, &tab_rect);
            }
        }
    }

    fn paint_tab(&self, ctx: &mut PaintContext<'_>, index: usize, rect: &Rect) {
        let tab = &self.tabs[index];
        let is_selected = index as i32 == self.current_index;
        let is_hovered = matches!(self.hover_part, TabBarPart::Tab(i) | TabBarPart::CloseButton(i) if i == index);
        let is_pressed = matches!(self.pressed_part, TabBarPart::Tab(i) if i == index);

        // Background color
        let bg_color = if !tab.enabled {
            self.tab_disabled_color
        } else if is_selected {
            self.tab_selected_color
        } else if is_pressed {
            darken_color(self.tab_hover_color, 0.1)
        } else if is_hovered {
            self.tab_hover_color
        } else {
            self.tab_color
        };

        // Draw tab background with rounded corners on the appropriate side
        let rrect = self.rounded_tab_rect(rect, is_selected);
        ctx.renderer().fill_rounded_rect(rrect, bg_color);

        // Draw bottom border for selected tab (creates visual connection to content)
        if is_selected {
            let border_rect = match self.tab_position {
                TabPosition::Top => Rect::new(
                    rect.origin.x,
                    rect.origin.y + rect.height() - 2.0,
                    rect.width(),
                    2.0,
                ),
                TabPosition::Bottom => Rect::new(rect.origin.x, rect.origin.y, rect.width(), 2.0),
                TabPosition::Left => Rect::new(
                    rect.origin.x + rect.width() - 2.0,
                    rect.origin.y,
                    2.0,
                    rect.height(),
                ),
                TabPosition::Right => Rect::new(rect.origin.x, rect.origin.y, 2.0, rect.height()),
            };
            ctx.renderer()
                .fill_rect(border_rect, self.tab_selected_color);
        }

        // Draw icon and text
        let text_color = if tab.enabled {
            self.text_color
        } else {
            self.text_disabled_color
        };

        let mut font_system = FontSystem::new();
        let layout = TextLayout::new(&mut font_system, &tab.label, &self.font);

        // Calculate content width (icon + text)
        let icon_space = if tab.icon.is_some() {
            self.icon_size + self.icon_spacing
        } else {
            0.0
        };

        let close_space = if tab.closable {
            self.close_button_size + self.tab_padding / 2.0
        } else {
            0.0
        };

        let content_width = icon_space + layout.width();

        // Calculate starting position for content (centered, accounting for close button)
        let content_start_x = if self.tab_position.is_horizontal() {
            rect.origin.x + (rect.width() - content_width - close_space) / 2.0
        } else {
            rect.origin.x + (rect.width() - content_width) / 2.0
        };

        let content_y = rect.origin.y + (rect.height() - layout.height()) / 2.0;

        // Draw icon if present
        let mut current_x = content_start_x;
        if let Some(icon) = &tab.icon {
            let icon_y = rect.origin.y + (rect.height() - self.icon_size) / 2.0;

            // Get the appropriate image based on enabled state
            let image = if !tab.enabled {
                icon.disabled_image().or_else(|| icon.image())
            } else {
                icon.image()
            };

            if let Some(img) = image {
                let icon_rect = Rect::new(current_x, icon_y, self.icon_size, self.icon_size);
                ctx.renderer()
                    .draw_image(img, icon_rect, ImageScaleMode::Fit);
            }

            current_x += self.icon_size + self.icon_spacing;
        }

        // Draw text
        let text_x = current_x;
        let text_y = content_y;

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                text_color,
            );
        }

        // Draw close button
        if tab.closable {
            self.paint_close_button(ctx, index, rect);
        }
    }

    fn rounded_tab_rect(&self, rect: &Rect, is_selected: bool) -> RoundedRect {
        let r = if is_selected {
            self.border_radius
        } else {
            self.border_radius / 2.0
        };

        // Round corners based on tab position
        let radii = match self.tab_position {
            TabPosition::Top => CornerRadii {
                top_left: r,
                top_right: r,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
            TabPosition::Bottom => CornerRadii {
                top_left: 0.0,
                top_right: 0.0,
                bottom_right: r,
                bottom_left: r,
            },
            TabPosition::Left => CornerRadii {
                top_left: r,
                top_right: 0.0,
                bottom_right: 0.0,
                bottom_left: r,
            },
            TabPosition::Right => CornerRadii {
                top_left: 0.0,
                top_right: r,
                bottom_right: r,
                bottom_left: 0.0,
            },
        };
        RoundedRect::with_radii(*rect, radii)
    }

    fn paint_close_button(&self, ctx: &mut PaintContext<'_>, index: usize, tab_rect: &Rect) {
        let rect = self.close_button_rect(tab_rect);
        let is_hovered = matches!(self.hover_part, TabBarPart::CloseButton(i) if i == index);

        // Background on hover
        if is_hovered {
            let rrect = RoundedRect::new(rect, 3.0);
            ctx.renderer()
                .fill_rounded_rect(rrect, self.close_button_hover_color);
        }

        // Draw X
        let padding = 4.0;
        let color = if is_hovered {
            Color::WHITE
        } else {
            self.close_button_color
        };
        let stroke = Stroke::new(color, 1.5);

        let p1 = Point::new(rect.origin.x + padding, rect.origin.y + padding);
        let p2 = Point::new(
            rect.origin.x + rect.width() - padding,
            rect.origin.y + rect.height() - padding,
        );
        let p3 = Point::new(
            rect.origin.x + rect.width() - padding,
            rect.origin.y + padding,
        );
        let p4 = Point::new(
            rect.origin.x + padding,
            rect.origin.y + rect.height() - padding,
        );

        ctx.renderer().draw_line(p1, p2, &stroke);
        ctx.renderer().draw_line(p3, p4, &stroke);
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for TabBar {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for TabBar {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let content_size = self.total_tabs_size();
        let is_horizontal = self.tab_position.is_horizontal();

        if is_horizontal {
            SizeHint::from_dimensions(content_size.max(100.0), self.tab_height)
                .with_minimum_dimensions(100.0, self.tab_height)
        } else {
            SizeHint::from_dimensions(self.tab_height, content_size.max(100.0))
                .with_minimum_dimensions(self.tab_height, 100.0)
        }
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_background(ctx);
        self.paint_scroll_buttons(ctx);
        self.paint_tabs(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Update overflow state on resize
        if let WidgetEvent::Resize(_) = event {
            self.update_overflow();
        }

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
            WidgetEvent::KeyPress(e) => {
                if self.handle_key_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::Leave(_) => {
                self.handle_leave();
            }
            _ => {}
        }
        false
    }
}

// Color helper
fn darken_color(color: Color, factor: f32) -> Color {
    let factor = 1.0 - factor.clamp(0.0, 1.0);
    Color::new(
        color.r * factor,
        color.g * factor,
        color.b * factor,
        color.a,
    )
}

// Ensure TabBar is Send + Sync
static_assertions::assert_impl_all!(TabBar: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        Arc,
        atomic::{AtomicI32, Ordering},
    };

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_tab_bar_creation() {
        setup();
        let bar = TabBar::new();
        assert_eq!(bar.count(), 0);
        assert_eq!(bar.current_index(), -1);
        assert_eq!(bar.tab_position(), TabPosition::Top);
    }

    #[test]
    fn test_add_tabs() {
        setup();
        let mut bar = TabBar::new();

        let idx0 = bar.add_tab("Tab 1");
        assert_eq!(idx0, 0);
        assert_eq!(bar.count(), 1);
        assert_eq!(bar.current_index(), 0); // Auto-selected

        let idx1 = bar.add_tab("Tab 2");
        assert_eq!(idx1, 1);
        assert_eq!(bar.count(), 2);
        assert_eq!(bar.current_index(), 0); // Still first tab
    }

    #[test]
    fn test_remove_tab() {
        setup();
        let mut bar = TabBar::new();
        bar.add_tab("Tab 1");
        bar.add_tab("Tab 2");
        bar.add_tab("Tab 3");

        bar.set_current_index(2);
        assert_eq!(bar.current_index(), 2);

        bar.remove_tab(0);
        assert_eq!(bar.count(), 2);
        assert_eq!(bar.current_index(), 1); // Adjusted
    }

    #[test]
    fn test_current_changed_signal() {
        setup();
        let mut bar = TabBar::new();
        let last_index = Arc::new(AtomicI32::new(-99));
        let last_index_clone = last_index.clone();

        bar.current_changed.connect(move |&index| {
            last_index_clone.store(index, Ordering::SeqCst);
        });

        bar.add_tab("Tab 1"); // Triggers current_changed(0)
        assert_eq!(last_index.load(Ordering::SeqCst), 0);

        bar.add_tab("Tab 2");
        bar.set_current_index(1);
        assert_eq!(last_index.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_tab_position() {
        setup();
        let bar = TabBar::new().with_tab_position(TabPosition::Left);
        assert_eq!(bar.tab_position(), TabPosition::Left);
        assert!(bar.tab_position().is_vertical());
    }

    #[test]
    fn test_move_tab() {
        setup();
        let mut bar = TabBar::new();
        bar.add_tab("A");
        bar.add_tab("B");
        bar.add_tab("C");

        bar.set_current_index(0);
        bar.move_tab(0, 2);

        assert_eq!(bar.tab_text(0), Some("B"));
        assert_eq!(bar.tab_text(1), Some("C"));
        assert_eq!(bar.tab_text(2), Some("A"));
        assert_eq!(bar.current_index(), 2); // Follows the moved tab
    }

    #[test]
    fn test_tab_enabled() {
        setup();
        let mut bar = TabBar::new();
        bar.add_tab("Tab 1");
        bar.add_tab("Tab 2");

        bar.set_tab_enabled(1, false);
        assert!(!bar.is_tab_enabled(1));

        // Can't select disabled tab
        bar.set_current_index(1);
        assert_eq!(bar.current_index(), 0); // Still 0
    }

    #[test]
    fn test_builder_pattern() {
        setup();
        let bar = TabBar::new()
            .with_tab_position(TabPosition::Bottom)
            .with_tabs_closable(true)
            .with_tabs_movable(false)
            .with_expanding(true);

        assert_eq!(bar.tab_position(), TabPosition::Bottom);
        assert!(bar.tabs_closable());
        assert!(!bar.tabs_movable());
        assert!(bar.expanding());
    }

    #[test]
    fn test_tab_icon() {
        setup();
        let mut bar = TabBar::new();
        bar.add_tab("Tab 1");
        bar.add_tab("Tab 2");

        // Initially no icon
        assert!(bar.tab_icon(0).is_none());
        assert!(bar.tab_icon(1).is_none());

        // Set an icon using a path (lazy loading)
        let icon = Icon::from_path("test_icon.png");
        bar.set_tab_icon(0, Some(icon));

        // Icon is now set
        assert!(bar.tab_icon(0).is_some());
        assert!(bar.tab_icon(1).is_none());

        // Clear the icon
        bar.set_tab_icon(0, None);
        assert!(bar.tab_icon(0).is_none());
    }

    #[test]
    fn test_icon_size_configuration() {
        setup();
        let bar = TabBar::new().with_icon_size(24.0).with_icon_spacing(8.0);

        assert_eq!(bar.icon_size(), 24.0);
        assert_eq!(bar.icon_spacing(), 8.0);
    }

    #[test]
    fn test_add_tab_with_icon() {
        setup();
        let mut bar = TabBar::new();

        let icon = Icon::from_path("home.png");
        let index = bar.add_tab_with_icon("Home", icon);

        assert_eq!(index, 0);
        assert!(bar.tab_icon(0).is_some());
    }
}
