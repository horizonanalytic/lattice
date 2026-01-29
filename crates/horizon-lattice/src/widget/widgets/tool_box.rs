//! ToolBox container implementation.
//!
//! This module provides [`ToolBox`], an accordion-style container that displays
//! a column of pages with clickable headers, where only one page is visible at a time.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::ToolBox;
//!
//! // Create a toolbox
//! let mut toolbox = ToolBox::new();
//!
//! // Add pages
//! toolbox.add_item(settings_widget_id, "Settings");
//! toolbox.add_item(preferences_widget_id, "Preferences");
//! toolbox.add_item(advanced_widget_id, "Advanced");
//!
//! // Switch to a specific page
//! toolbox.set_current_index(1);
//!
//! // Connect to page changes
//! toolbox.current_changed.connect(|&index| {
//!     println!("Switched to page: {}", index);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, FillRule, Font, FontSystem, Path, Point, Rect, Renderer, Stroke, TextLayout,
};

use crate::widget::events::{Key, MouseButton};
use crate::widget::{
    PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

/// Default header height for toolbox items.
const DEFAULT_HEADER_HEIGHT: f32 = 28.0;

/// Metadata for a single toolbox page.
#[derive(Clone)]
struct ToolBoxItem {
    /// The display text for this item's header.
    text: String,
    /// Optional icon path.
    icon_path: Option<String>,
    /// Optional tooltip text.
    tool_tip: Option<String>,
    /// Whether this item is enabled.
    enabled: bool,
}

impl ToolBoxItem {
    fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            icon_path: None,
            tool_tip: None,
            enabled: true,
        }
    }
}

/// A single page in the toolbox.
struct ToolBoxPage {
    /// The widget ID for this page's content.
    widget_id: ObjectId,
    /// Item metadata.
    item: ToolBoxItem,
}

/// An accordion-style container widget.
///
/// ToolBox displays a column of pages with clickable headers. Each page has
/// a header button that can be clicked to expand/show that page. Only one
/// page is visible at a time.
///
/// # Features
///
/// - Multiple pages with clickable headers
/// - Only one page visible at a time
/// - Optional icons on headers
/// - Individual page enable/disable
/// - Keyboard navigation support
///
/// # Signals
///
/// - `current_changed(i32)`: Emitted when the current page changes
pub struct ToolBox {
    /// Widget base.
    base: WidgetBase,

    /// Pages in the toolbox.
    pages: Vec<ToolBoxPage>,

    /// Current page index (-1 if no pages).
    current_index: i32,

    /// Header height for each item.
    header_height: f32,

    /// Header background color.
    header_color: Color,

    /// Header background color when hovered.
    header_hover_color: Color,

    /// Header background color when selected/active.
    header_selected_color: Color,

    /// Header text color.
    header_text_color: Color,

    /// Content background color.
    content_background_color: Color,

    /// Border color.
    border_color: Color,

    /// Border width.
    border_width: f32,

    /// Index of header currently being hovered (-1 for none).
    hovered_header: i32,

    /// Signal emitted when current page changes.
    pub current_changed: Signal<i32>,
}

impl ToolBox {
    /// Create a new empty toolbox.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));
        base.set_focusable(true);

        Self {
            base,
            pages: Vec::new(),
            current_index: -1,
            header_height: DEFAULT_HEADER_HEIGHT,
            header_color: Color::from_rgb8(240, 240, 240),
            header_hover_color: Color::from_rgb8(230, 230, 230),
            header_selected_color: Color::from_rgb8(66, 133, 244),
            header_text_color: Color::from_rgb8(33, 33, 33),
            content_background_color: Color::WHITE,
            border_color: Color::from_rgb8(200, 200, 200),
            border_width: 1.0,
            hovered_header: -1,
            current_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Item Management
    // =========================================================================

    /// Add a widget to the toolbox.
    ///
    /// Returns the index of the new item.
    pub fn add_item(&mut self, widget_id: ObjectId, text: impl Into<String>) -> i32 {
        self.insert_item(self.pages.len() as i32, widget_id, text)
    }

    /// Add a widget with an icon to the toolbox.
    ///
    /// Returns the index of the new item.
    pub fn add_item_with_icon(
        &mut self,
        widget_id: ObjectId,
        icon_path: impl Into<String>,
        text: impl Into<String>,
    ) -> i32 {
        let index = self.add_item(widget_id, text);
        self.set_item_icon(index, Some(icon_path.into()));
        index
    }

    /// Insert a widget at the specified index.
    ///
    /// Returns the actual index where the widget was inserted.
    pub fn insert_item(&mut self, index: i32, widget_id: ObjectId, text: impl Into<String>) -> i32 {
        let insert_pos = if index < 0 {
            0
        } else {
            (index as usize).min(self.pages.len())
        };

        let page = ToolBoxPage {
            widget_id,
            item: ToolBoxItem::new(text),
        };

        self.pages.insert(insert_pos, page);

        // If this is the first page, make it current
        if self.pages.len() == 1 {
            self.current_index = 0;
            self.current_changed.emit(0);
        } else if insert_pos as i32 <= self.current_index {
            // Adjust current index if we inserted before it
            self.current_index += 1;
        }

        self.base.update();
        insert_pos as i32
    }

    /// Remove the item at the specified index.
    ///
    /// Returns the widget ID of the removed item, if any.
    pub fn remove_item(&mut self, index: i32) -> Option<ObjectId> {
        if index < 0 || index as usize >= self.pages.len() {
            return None;
        }

        let page = self.pages.remove(index as usize);
        let old_current = self.current_index;

        // Adjust current index
        if self.pages.is_empty() {
            self.current_index = -1;
        } else if index < self.current_index {
            self.current_index -= 1;
        } else if index == self.current_index {
            // Current item was removed, select the previous one or stay at same index
            self.current_index = self.current_index.min(self.pages.len() as i32 - 1);
        }

        self.base.update();

        // Emit signal if current changed
        if self.current_index != old_current {
            self.current_changed.emit(self.current_index);
        }

        Some(page.widget_id)
    }

    /// Get the number of items.
    pub fn count(&self) -> i32 {
        self.pages.len() as i32
    }

    /// Check if the toolbox is empty.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    // =========================================================================
    // Current Item
    // =========================================================================

    /// Get the current item index.
    ///
    /// Returns -1 if there are no items.
    pub fn current_index(&self) -> i32 {
        self.current_index
    }

    /// Set the current item index.
    ///
    /// Returns `true` if the index changed.
    pub fn set_current_index(&mut self, index: i32) -> bool {
        if index < 0 || index as usize >= self.pages.len() {
            return false;
        }

        // Check if the item is enabled
        if !self.pages[index as usize].item.enabled {
            return false;
        }

        if index == self.current_index {
            return false;
        }

        self.current_index = index;
        self.base.update();
        self.current_changed.emit(index);
        true
    }

    /// Get the widget ID of the current item.
    pub fn current_widget(&self) -> Option<ObjectId> {
        if self.current_index >= 0 {
            self.pages
                .get(self.current_index as usize)
                .map(|p| p.widget_id)
        } else {
            None
        }
    }

    /// Set the current item by widget ID.
    ///
    /// Returns `true` if the widget was found and is now current.
    pub fn set_current_widget(&mut self, widget_id: ObjectId) -> bool {
        let index = self.index_of(widget_id);
        if index >= 0 {
            self.set_current_index(index)
        } else {
            false
        }
    }

    /// Get the widget ID at a specific index.
    pub fn widget(&self, index: i32) -> Option<ObjectId> {
        if index < 0 {
            None
        } else {
            self.pages.get(index as usize).map(|p| p.widget_id)
        }
    }

    /// Find the index of a widget.
    ///
    /// Returns -1 if the widget is not in the toolbox.
    pub fn index_of(&self, widget_id: ObjectId) -> i32 {
        self.pages
            .iter()
            .position(|p| p.widget_id == widget_id)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    // =========================================================================
    // Item Properties
    // =========================================================================

    /// Get the text for an item.
    pub fn item_text(&self, index: i32) -> Option<&str> {
        if index < 0 {
            None
        } else {
            self.pages.get(index as usize).map(|p| p.item.text.as_str())
        }
    }

    /// Set the text for an item.
    pub fn set_item_text(&mut self, index: i32, text: impl Into<String>) {
        if index >= 0
            && let Some(page) = self.pages.get_mut(index as usize) {
                page.item.text = text.into();
                self.base.update();
            }
    }

    /// Get the icon path for an item.
    pub fn item_icon(&self, index: i32) -> Option<&str> {
        if index < 0 {
            None
        } else {
            self.pages
                .get(index as usize)
                .and_then(|p| p.item.icon_path.as_deref())
        }
    }

    /// Set the icon path for an item.
    pub fn set_item_icon(&mut self, index: i32, icon_path: Option<String>) {
        if index >= 0
            && let Some(page) = self.pages.get_mut(index as usize) {
                page.item.icon_path = icon_path;
                self.base.update();
            }
    }

    /// Check if an item is enabled.
    pub fn is_item_enabled(&self, index: i32) -> bool {
        if index < 0 {
            false
        } else {
            self.pages
                .get(index as usize)
                .map(|p| p.item.enabled)
                .unwrap_or(false)
        }
    }

    /// Set whether an item is enabled.
    pub fn set_item_enabled(&mut self, index: i32, enabled: bool) {
        if index >= 0
            && let Some(page) = self.pages.get_mut(index as usize) {
                page.item.enabled = enabled;
                self.base.update();
            }
    }

    /// Get the tooltip for an item.
    pub fn item_tool_tip(&self, index: i32) -> Option<&str> {
        if index < 0 {
            None
        } else {
            self.pages
                .get(index as usize)
                .and_then(|p| p.item.tool_tip.as_deref())
        }
    }

    /// Set the tooltip for an item.
    pub fn set_item_tool_tip(&mut self, index: i32, tool_tip: Option<String>) {
        if index >= 0
            && let Some(page) = self.pages.get_mut(index as usize) {
                page.item.tool_tip = tool_tip;
            }
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the header height.
    pub fn header_height(&self) -> f32 {
        self.header_height
    }

    /// Set the header height.
    pub fn set_header_height(&mut self, height: f32) {
        self.header_height = height.max(16.0);
        self.base.update();
    }

    /// Set header height using builder pattern.
    pub fn with_header_height(mut self, height: f32) -> Self {
        self.set_header_height(height);
        self
    }

    /// Get the header background color.
    pub fn header_color(&self) -> Color {
        self.header_color
    }

    /// Set the header background color.
    pub fn set_header_color(&mut self, color: Color) {
        self.header_color = color;
        self.base.update();
    }

    /// Set header color using builder pattern.
    pub fn with_header_color(mut self, color: Color) -> Self {
        self.set_header_color(color);
        self
    }

    /// Get the selected header background color.
    pub fn header_selected_color(&self) -> Color {
        self.header_selected_color
    }

    /// Set the selected header background color.
    pub fn set_header_selected_color(&mut self, color: Color) {
        self.header_selected_color = color;
        self.base.update();
    }

    /// Set selected header color using builder pattern.
    pub fn with_header_selected_color(mut self, color: Color) -> Self {
        self.set_header_selected_color(color);
        self
    }

    /// Get the content background color.
    pub fn content_background_color(&self) -> Color {
        self.content_background_color
    }

    /// Set the content background color.
    pub fn set_content_background_color(&mut self, color: Color) {
        self.content_background_color = color;
        self.base.update();
    }

    /// Set content background color using builder pattern.
    pub fn with_content_background_color(mut self, color: Color) -> Self {
        self.set_content_background_color(color);
        self
    }

    /// Get the border color.
    pub fn border_color(&self) -> Color {
        self.border_color
    }

    /// Set the border color.
    pub fn set_border_color(&mut self, color: Color) {
        self.border_color = color;
        self.base.update();
    }

    /// Set border color using builder pattern.
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.set_border_color(color);
        self
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    /// Get the header rectangle for a specific index.
    fn header_rect(&self, index: i32) -> Option<Rect> {
        if index < 0 || index as usize >= self.pages.len() {
            return None;
        }

        let rect = self.base.rect();
        let mut y = self.border_width;

        for i in 0..=index as usize {
            if i == index as usize {
                return Some(Rect::new(
                    self.border_width,
                    y,
                    (rect.width() - 2.0 * self.border_width).max(0.0),
                    self.header_height,
                ));
            }

            y += self.header_height;

            // If this is the current page, add content height
            if i as i32 == self.current_index {
                y += self.content_height();
            }
        }

        None
    }

    /// Get the content rectangle (for the current page).
    fn content_rect(&self) -> Option<Rect> {
        if self.current_index < 0 || self.current_index as usize >= self.pages.len() {
            return None;
        }

        let rect = self.base.rect();
        let mut y = self.border_width;

        // Sum heights of headers before the current one, plus the current header
        for _ in 0..=self.current_index as usize {
            y += self.header_height;
        }

        let content_h = self.content_height();

        Some(Rect::new(
            self.border_width,
            y,
            (rect.width() - 2.0 * self.border_width).max(0.0),
            content_h,
        ))
    }

    /// Calculate available content height.
    fn content_height(&self) -> f32 {
        let rect = self.base.rect();
        let total_header_height = self.pages.len() as f32 * self.header_height;
        let available = rect.height() - 2.0 * self.border_width - total_header_height;
        available.max(0.0)
    }

    /// Find which header index contains a point.
    fn header_at(&self, pos: Point) -> i32 {
        for i in 0..self.pages.len() as i32 {
            if let Some(rect) = self.header_rect(i)
                && rect.contains(pos) {
                    return i;
                }
        }
        -1
    }

    /// Find next enabled index in a direction.
    fn find_enabled_index(&self, from: i32, forward: bool) -> Option<i32> {
        let count = self.pages.len() as i32;
        if count == 0 {
            return None;
        }

        let mut index = from;
        for _ in 0..count {
            index = if forward {
                (index + 1) % count
            } else {
                (index - 1 + count) % count
            };

            if self.pages[index as usize].item.enabled {
                return Some(index);
            }
        }

        None
    }
}

impl Default for ToolBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ToolBox {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ToolBox {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Calculate minimum height based on headers
        let header_total = self.pages.len() as f32 * self.header_height;
        let min_content = 50.0; // Minimum content area
        let extra = 2.0 * self.border_width;

        let min_height = header_total + min_content + extra;
        let min_width = 100.0 + extra;

        let preferred_height = header_total + 200.0 + extra;
        let preferred_width = 200.0 + extra;

        SizeHint::from_dimensions(preferred_width, preferred_height)
            .with_minimum_dimensions(min_width, min_height)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let widget_rect = ctx.rect();

        // Background
        if self.content_background_color.a > 0.0 {
            ctx.renderer()
                .fill_rect(widget_rect, self.content_background_color);
        }

        // Border
        if self.border_width > 0.0 && self.border_color.a > 0.0 {
            let stroke = Stroke::new(self.border_color, self.border_width);
            ctx.renderer().stroke_rect(widget_rect, &stroke);
        }

        // Paint headers
        for i in 0..self.pages.len() as i32 {
            if let Some(header_rect) = self.header_rect(i) {
                let page = &self.pages[i as usize];

                // Choose header color based on state
                let bg_color = if !page.item.enabled {
                    // Disabled - gray out
                    Color::from_rgba8(200, 200, 200, 128)
                } else if i == self.current_index {
                    self.header_selected_color
                } else if i == self.hovered_header {
                    self.header_hover_color
                } else {
                    self.header_color
                };

                // Draw header background
                ctx.renderer().fill_rect(header_rect, bg_color);

                // Draw header bottom border
                let border_y = header_rect.top() + header_rect.height();
                ctx.renderer().draw_line(
                    Point::new(header_rect.left(), border_y),
                    Point::new(header_rect.left() + header_rect.width(), border_y),
                    &Stroke::new(self.border_color, 1.0),
                );

                // Draw expand indicator (triangle) using Path
                let indicator_size = 8.0;
                let indicator_x = header_rect.left() + 8.0;
                let indicator_y = header_rect.top() + (header_rect.height() - indicator_size) / 2.0;

                let indicator_color = if i == self.current_index {
                    Color::WHITE
                } else if !page.item.enabled {
                    Color::from_rgb8(150, 150, 150)
                } else {
                    self.header_text_color
                };

                let mut indicator_path = Path::new();
                if i == self.current_index {
                    // Down-pointing triangle (expanded)
                    indicator_path.move_to(Point::new(indicator_x, indicator_y));
                    indicator_path.line_to(Point::new(indicator_x + indicator_size, indicator_y));
                    indicator_path.line_to(Point::new(
                        indicator_x + indicator_size / 2.0,
                        indicator_y + indicator_size,
                    ));
                    indicator_path.close();
                } else {
                    // Right-pointing triangle (collapsed)
                    indicator_path.move_to(Point::new(indicator_x, indicator_y));
                    indicator_path.line_to(Point::new(indicator_x, indicator_y + indicator_size));
                    indicator_path.line_to(Point::new(
                        indicator_x + indicator_size,
                        indicator_y + indicator_size / 2.0,
                    ));
                    indicator_path.close();
                }
                ctx.renderer()
                    .fill_path(&indicator_path, indicator_color, FillRule::NonZero);

                // Draw text using TextLayout (text rendering infrastructure)
                let _text_color = if i == self.current_index {
                    Color::WHITE
                } else if !page.item.enabled {
                    Color::from_rgb8(150, 150, 150)
                } else {
                    self.header_text_color
                };

                let text_x = header_rect.left() + 24.0; // After indicator
                let text_y = header_rect.top() + (header_rect.height() - 14.0) / 2.0;
                let _text_pos = Point::new(text_x, text_y);

                // Text rendering requires integration with the app's render pass system.
                // The FontSystem and TextRenderer would be used here in a full implementation.
                // For now, the layout calculation is set up but actual glyph rendering
                // is deferred to the framework's text render pass integration.
                if !page.item.text.is_empty() {
                    let mut font_system = FontSystem::new();
                    let layout =
                        TextLayout::new(&mut font_system, &page.item.text, &Font::default());
                    let _ = layout; // Layout prepared for text rendering
                }
            }
        }

        // Paint content area background (if there's a current page)
        if let Some(content_rect) = self.content_rect() {
            // The actual content widget would be painted by the parent/framework
            // Here we just draw the content area background
            ctx.renderer()
                .fill_rect(content_rect, self.content_background_color);
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                if e.button == MouseButton::Left {
                    let header_index = self.header_at(e.local_pos);
                    if header_index >= 0
                        && self.set_current_index(header_index) {
                            return true;
                        }
                }
                false
            }

            WidgetEvent::MouseMove(e) => {
                let header_index = self.header_at(e.local_pos);
                if header_index != self.hovered_header {
                    self.hovered_header = header_index;
                    self.base.update();
                }
                false
            }

            WidgetEvent::Leave(_) => {
                if self.hovered_header >= 0 {
                    self.hovered_header = -1;
                    self.base.update();
                }
                false
            }

            WidgetEvent::KeyPress(e) => {
                if !self.base.has_focus() {
                    return false;
                }

                match e.key {
                    Key::ArrowUp => {
                        if let Some(next) = self.find_enabled_index(self.current_index, false) {
                            self.set_current_index(next);
                            return true;
                        }
                    }
                    Key::ArrowDown => {
                        if let Some(next) = self.find_enabled_index(self.current_index, true) {
                            self.set_current_index(next);
                            return true;
                        }
                    }
                    Key::Home => {
                        if let Some(first) = self.find_enabled_index(-1, true) {
                            self.set_current_index(first);
                            return true;
                        }
                    }
                    Key::End => {
                        if let Some(last) = self.find_enabled_index(0, false) {
                            self.set_current_index(last);
                            return true;
                        }
                    }
                    _ => {}
                }
                false
            }

            _ => false,
        }
    }
}

// Ensure ToolBox is Send + Sync
static_assertions::assert_impl_all!(ToolBox: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::base::WidgetBase;
    use crate::widget::traits::{PaintContext, Widget};
    use horizon_lattice_core::{Object, init_global_registry};

    /// Mock widget for testing.
    struct MockWidget {
        base: WidgetBase,
    }

    impl MockWidget {
        fn new() -> Self {
            Self {
                base: WidgetBase::new::<Self>(),
            }
        }
    }

    impl Object for MockWidget {
        fn object_id(&self) -> ObjectId {
            self.base.object_id()
        }
    }

    impl Widget for MockWidget {
        fn widget_base(&self) -> &WidgetBase {
            &self.base
        }

        fn widget_base_mut(&mut self) -> &mut WidgetBase {
            &mut self.base
        }

        fn size_hint(&self) -> SizeHint {
            SizeHint::default()
        }

        fn paint(&self, _ctx: &mut PaintContext<'_>) {}
    }

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_toolbox_creation() {
        setup();
        let toolbox = ToolBox::new();
        assert_eq!(toolbox.count(), 0);
        assert_eq!(toolbox.current_index(), -1);
        assert!(toolbox.is_empty());
        assert!(toolbox.current_widget().is_none());
    }

    #[test]
    fn test_add_items() {
        setup();
        let mut toolbox = ToolBox::new();

        // Create mock widgets to get valid ObjectIds
        let mock1 = MockWidget::new();
        let widget1 = mock1.object_id();
        let mock2 = MockWidget::new();
        let widget2 = mock2.object_id();

        let idx0 = toolbox.add_item(widget1, "Page 1");
        assert_eq!(idx0, 0);
        assert_eq!(toolbox.count(), 1);
        assert_eq!(toolbox.current_index(), 0);
        assert_eq!(toolbox.current_widget(), Some(widget1));
        assert!(!toolbox.is_empty());

        let idx1 = toolbox.add_item(widget2, "Page 2");
        assert_eq!(idx1, 1);
        assert_eq!(toolbox.count(), 2);
        assert_eq!(toolbox.current_index(), 0); // Still on first page
    }

    #[test]
    fn test_set_current_index() {
        setup();
        let mut toolbox = ToolBox::new();

        let mock1 = MockWidget::new();
        let widget1 = mock1.object_id();
        let mock2 = MockWidget::new();
        let widget2 = mock2.object_id();

        toolbox.add_item(widget1, "Page 1");
        toolbox.add_item(widget2, "Page 2");

        assert!(toolbox.set_current_index(1));
        assert_eq!(toolbox.current_index(), 1);
        assert_eq!(toolbox.current_widget(), Some(widget2));

        // Same index should return false
        assert!(!toolbox.set_current_index(1));

        // Out of bounds should return false
        assert!(!toolbox.set_current_index(10));
        assert!(!toolbox.set_current_index(-1));
    }

    #[test]
    fn test_remove_item() {
        setup();
        let mut toolbox = ToolBox::new();

        let mock1 = MockWidget::new();
        let widget1 = mock1.object_id();
        let mock2 = MockWidget::new();
        let widget2 = mock2.object_id();

        toolbox.add_item(widget1, "Page 1");
        toolbox.add_item(widget2, "Page 2");

        let removed = toolbox.remove_item(0);
        assert_eq!(removed, Some(widget1));
        assert_eq!(toolbox.count(), 1);
        assert_eq!(toolbox.current_widget(), Some(widget2));
        assert_eq!(toolbox.current_index(), 0);
    }

    #[test]
    fn test_item_properties() {
        setup();
        let mut toolbox = ToolBox::new();

        let mock1 = MockWidget::new();
        let widget1 = mock1.object_id();
        toolbox.add_item(widget1, "Original");

        assert_eq!(toolbox.item_text(0), Some("Original"));

        toolbox.set_item_text(0, "Updated");
        assert_eq!(toolbox.item_text(0), Some("Updated"));

        assert!(toolbox.is_item_enabled(0));
        toolbox.set_item_enabled(0, false);
        assert!(!toolbox.is_item_enabled(0));

        // Disabled item cannot become current
        let mock2 = MockWidget::new();
        let widget2 = mock2.object_id();
        toolbox.add_item(widget2, "Page 2");
        toolbox.set_current_index(1);

        assert!(!toolbox.set_current_index(0)); // Should fail - item 0 is disabled
        assert_eq!(toolbox.current_index(), 1);
    }

    #[test]
    fn test_index_of() {
        setup();
        let mut toolbox = ToolBox::new();

        let mock1 = MockWidget::new();
        let widget1 = mock1.object_id();
        let mock2 = MockWidget::new();
        let widget2 = mock2.object_id();
        let mock3 = MockWidget::new();
        let widget3 = mock3.object_id();

        toolbox.add_item(widget1, "Page 1");
        toolbox.add_item(widget2, "Page 2");

        assert_eq!(toolbox.index_of(widget1), 0);
        assert_eq!(toolbox.index_of(widget2), 1);
        assert_eq!(toolbox.index_of(widget3), -1); // Not found
    }

    #[test]
    fn test_insert_item() {
        setup();
        let mut toolbox = ToolBox::new();

        let mock1 = MockWidget::new();
        let widget1 = mock1.object_id();
        let mock2 = MockWidget::new();
        let widget2 = mock2.object_id();
        let mock3 = MockWidget::new();
        let widget3 = mock3.object_id();

        toolbox.add_item(widget1, "Page 1");
        toolbox.add_item(widget3, "Page 3");

        // Insert in the middle
        let idx = toolbox.insert_item(1, widget2, "Page 2");
        assert_eq!(idx, 1);
        assert_eq!(toolbox.count(), 3);
        assert_eq!(toolbox.widget(0), Some(widget1));
        assert_eq!(toolbox.widget(1), Some(widget2));
        assert_eq!(toolbox.widget(2), Some(widget3));
    }

    #[test]
    fn test_builder_pattern() {
        setup();
        let toolbox = ToolBox::new()
            .with_header_height(36.0)
            .with_header_color(Color::from_rgb8(100, 100, 100))
            .with_border_color(Color::BLACK);

        assert_eq!(toolbox.header_height(), 36.0);
        assert_eq!(toolbox.header_color(), Color::from_rgb8(100, 100, 100));
        assert_eq!(toolbox.border_color(), Color::BLACK);
    }
}
