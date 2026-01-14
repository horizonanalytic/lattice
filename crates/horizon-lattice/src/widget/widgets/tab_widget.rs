//! TabWidget container implementation.
//!
//! This module provides [`TabWidget`], a tabbed page container that combines
//! a [`TabBar`] with a [`StackLayout`] to switch between child widgets.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{TabWidget, TabPosition};
//!
//! // Create a tab widget
//! let mut tabs = TabWidget::new()
//!     .with_tab_position(TabPosition::Top);
//!
//! // Add pages (each page is a widget)
//! tabs.add_tab(page1_id, "Home");
//! tabs.add_tab(page2_id, "Settings");
//! tabs.add_tab(page3_id, "Help");
//!
//! // Connect to tab changes
//! tabs.current_changed.connect(|&index| {
//!     println!("Switched to page: {}", index);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Rect, Renderer};

use super::tab_bar::{TabBar, TabPosition};
use crate::widget::layout::{ContentMargins, Layout, LayoutItem, StackLayout};
use crate::widget::{PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent};

/// A tabbed page container widget.
///
/// TabWidget manages multiple child widgets (pages) where only one is visible
/// at a time. Users switch between pages by clicking tabs in the integrated
/// tab bar.
///
/// # Features
///
/// - Integrated tab bar with customizable position (top, bottom, left, right)
/// - Support for closable tabs
/// - Tab icons (via TabBar)
/// - Keyboard navigation between tabs
/// - Tab reordering via drag and drop
///
/// # Layout
///
/// TabWidget uses a BoxLayout internally:
/// - For top/bottom tabs: vertical layout with tab bar and content area
/// - For left/right tabs: horizontal layout with tab bar and content area
///
/// # Signals
///
/// - `current_changed(i32)`: Emitted when the current tab changes
/// - `tab_close_requested(i32)`: Emitted when a tab's close button is clicked
pub struct TabWidget {
    /// Widget base.
    base: WidgetBase,

    /// The integrated tab bar.
    tab_bar: TabBar,

    /// Child widget IDs (pages).
    pages: Vec<ObjectId>,

    /// Stack layout for page management.
    stack_layout: StackLayout,

    /// Tab position.
    tab_position: TabPosition,

    /// Content margins around the page area.
    content_margins: ContentMargins,

    /// Background color for the content area.
    content_background: Color,

    /// Border color around content.
    border_color: Color,

    /// Border width.
    border_width: f32,

    /// Signal emitted when current tab changes.
    pub current_changed: Signal<i32>,

    /// Signal emitted when a tab close is requested.
    pub tab_close_requested: Signal<i32>,
}

impl TabWidget {
    /// Create a new tab widget.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Expanding));

        let tab_bar = TabBar::new();

        Self {
            base,
            tab_bar,
            pages: Vec::new(),
            stack_layout: StackLayout::new(),
            tab_position: TabPosition::Top,
            content_margins: ContentMargins::uniform(4.0),
            content_background: Color::WHITE,
            border_color: Color::from_rgb8(200, 200, 200),
            border_width: 1.0,
            current_changed: Signal::new(),
            tab_close_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Tab/Page Management
    // =========================================================================

    /// Add a new tab with the given widget and label.
    ///
    /// Returns the index of the new tab.
    pub fn add_tab(&mut self, widget_id: ObjectId, label: impl Into<String>) -> i32 {
        let index = self.tab_bar.add_tab(label);
        self.pages.push(widget_id);
        self.stack_layout.add_widget(widget_id);
        self.sync_current_index();
        self.base.update();
        index
    }

    /// Add a new tab with widget, label, and icon.
    pub fn add_tab_with_icon(
        &mut self,
        widget_id: ObjectId,
        label: impl Into<String>,
        icon: impl Into<String>,
    ) -> i32 {
        let index = self.tab_bar.add_tab_with_icon(label, icon);
        self.pages.push(widget_id);
        self.stack_layout.add_widget(widget_id);
        self.sync_current_index();
        self.base.update();
        index
    }

    /// Insert a tab at the specified index.
    pub fn insert_tab(
        &mut self,
        index: i32,
        widget_id: ObjectId,
        label: impl Into<String>,
    ) -> i32 {
        let result_index = self.tab_bar.insert_tab(index, label);
        let insert_pos = (index as usize).min(self.pages.len());
        self.pages.insert(insert_pos, widget_id);
        self.stack_layout.insert_item(insert_pos, LayoutItem::Widget(widget_id));
        self.sync_current_index();
        self.base.update();
        result_index
    }

    /// Remove the tab at the specified index.
    ///
    /// Returns the widget ID of the removed page, if any.
    pub fn remove_tab(&mut self, index: i32) -> Option<ObjectId> {
        if index < 0 || index as usize >= self.pages.len() {
            return None;
        }

        let widget_id = self.pages.remove(index as usize);
        self.stack_layout.remove_item(index as usize);
        self.tab_bar.remove_tab(index);
        self.sync_current_index();
        self.base.update();
        Some(widget_id)
    }

    /// Clear all tabs.
    pub fn clear(&mut self) {
        self.pages.clear();
        self.stack_layout.clear();
        self.tab_bar.clear();
        self.base.update();
    }

    /// Get the number of tabs.
    pub fn count(&self) -> i32 {
        self.pages.len() as i32
    }

    // =========================================================================
    // Current Tab
    // =========================================================================

    /// Get the current tab index.
    pub fn current_index(&self) -> i32 {
        self.tab_bar.current_index()
    }

    /// Set the current tab index.
    pub fn set_current_index(&mut self, index: i32) {
        self.tab_bar.set_current_index(index);
        self.sync_current_index();
    }

    /// Get the widget ID of the current page.
    pub fn current_widget(&self) -> Option<ObjectId> {
        let index = self.current_index();
        if index >= 0 && (index as usize) < self.pages.len() {
            Some(self.pages[index as usize])
        } else {
            None
        }
    }

    /// Get the widget ID at a specific index.
    pub fn widget(&self, index: i32) -> Option<ObjectId> {
        self.pages.get(index as usize).copied()
    }

    /// Find the index of a widget.
    pub fn index_of(&self, widget_id: ObjectId) -> i32 {
        self.pages
            .iter()
            .position(|&id| id == widget_id)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    /// Sync stack layout with tab bar current index.
    fn sync_current_index(&mut self) {
        let index = self.tab_bar.current_index();
        if index >= 0 {
            self.stack_layout.set_current_index(index as usize);
        }
    }

    // =========================================================================
    // Tab Properties
    // =========================================================================

    /// Get the text of a tab.
    pub fn tab_text(&self, index: i32) -> Option<&str> {
        self.tab_bar.tab_text(index)
    }

    /// Set the text of a tab.
    pub fn set_tab_text(&mut self, index: i32, text: impl Into<String>) {
        self.tab_bar.set_tab_text(index, text);
    }

    /// Get whether a tab is enabled.
    pub fn is_tab_enabled(&self, index: i32) -> bool {
        self.tab_bar.is_tab_enabled(index)
    }

    /// Set whether a tab is enabled.
    pub fn set_tab_enabled(&mut self, index: i32, enabled: bool) {
        self.tab_bar.set_tab_enabled(index, enabled);
    }

    /// Get the tooltip for a tab.
    pub fn tab_tooltip(&self, index: i32) -> Option<&str> {
        self.tab_bar.tab_tooltip(index)
    }

    /// Set the tooltip for a tab.
    pub fn set_tab_tooltip(&mut self, index: i32, tooltip: impl Into<String>) {
        self.tab_bar.set_tab_tooltip(index, tooltip);
    }

    /// Get whether a specific tab is closable.
    pub fn is_tab_closable(&self, index: i32) -> bool {
        self.tab_bar.is_tab_closable(index)
    }

    /// Set whether a specific tab is closable.
    pub fn set_tab_closable(&mut self, index: i32, closable: bool) {
        self.tab_bar.set_tab_closable(index, closable);
    }

    // =========================================================================
    // TabWidget Properties
    // =========================================================================

    /// Get the tab position.
    pub fn tab_position(&self) -> TabPosition {
        self.tab_position
    }

    /// Set the tab position.
    pub fn set_tab_position(&mut self, position: TabPosition) {
        if self.tab_position != position {
            self.tab_position = position;
            self.tab_bar.set_tab_position(position);
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
        self.tab_bar.tabs_closable()
    }

    /// Set whether new tabs are closable by default.
    pub fn set_tabs_closable(&mut self, closable: bool) {
        self.tab_bar.set_tabs_closable(closable);
    }

    /// Set tabs closable using builder pattern.
    pub fn with_tabs_closable(mut self, closable: bool) -> Self {
        self.tab_bar.set_tabs_closable(closable);
        self
    }

    /// Check if tabs are movable.
    pub fn tabs_movable(&self) -> bool {
        self.tab_bar.tabs_movable()
    }

    /// Set whether tabs can be reordered by dragging.
    pub fn set_tabs_movable(&mut self, movable: bool) {
        self.tab_bar.set_tabs_movable(movable);
    }

    /// Set tabs movable using builder pattern.
    pub fn with_tabs_movable(mut self, movable: bool) -> Self {
        self.tab_bar.set_tabs_movable(movable);
        self
    }

    /// Get the content margins.
    pub fn content_margins(&self) -> ContentMargins {
        self.content_margins
    }

    /// Set the content margins around the page area.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        self.content_margins = margins;
        self.base.update();
    }

    /// Set content margins using builder pattern.
    pub fn with_content_margins(mut self, margins: ContentMargins) -> Self {
        self.content_margins = margins;
        self
    }

    /// Get the content background color.
    pub fn content_background(&self) -> Color {
        self.content_background
    }

    /// Set the content background color.
    pub fn set_content_background(&mut self, color: Color) {
        self.content_background = color;
        self.base.update();
    }

    /// Set content background using builder pattern.
    pub fn with_content_background(mut self, color: Color) -> Self {
        self.content_background = color;
        self
    }

    /// Get access to the tab bar for advanced configuration.
    pub fn tab_bar(&self) -> &TabBar {
        &self.tab_bar
    }

    /// Get mutable access to the tab bar.
    pub fn tab_bar_mut(&mut self) -> &mut TabBar {
        &mut self.tab_bar
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    /// Get the tab bar rectangle.
    fn tab_bar_rect(&self) -> Rect {
        let rect = self.base.rect();
        let tab_height = self.tab_bar.size_hint().preferred.height;
        let tab_width = self.tab_bar.size_hint().preferred.width;

        match self.tab_position {
            TabPosition::Top => Rect::new(0.0, 0.0, rect.width(), tab_height),
            TabPosition::Bottom => {
                Rect::new(0.0, rect.height() - tab_height, rect.width(), tab_height)
            }
            TabPosition::Left => Rect::new(0.0, 0.0, tab_width, rect.height()),
            TabPosition::Right => Rect::new(rect.width() - tab_width, 0.0, tab_width, rect.height()),
        }
    }

    /// Get the content area rectangle.
    fn content_rect(&self) -> Rect {
        let rect = self.base.rect();
        let tab_bar_rect = self.tab_bar_rect();
        let m = &self.content_margins;
        let bw = self.border_width;

        match self.tab_position {
            TabPosition::Top => Rect::new(
                m.left + bw,
                tab_bar_rect.height() + m.top + bw,
                rect.width() - m.left - m.right - 2.0 * bw,
                rect.height() - tab_bar_rect.height() - m.top - m.bottom - 2.0 * bw,
            ),
            TabPosition::Bottom => Rect::new(
                m.left + bw,
                m.top + bw,
                rect.width() - m.left - m.right - 2.0 * bw,
                rect.height() - tab_bar_rect.height() - m.top - m.bottom - 2.0 * bw,
            ),
            TabPosition::Left => Rect::new(
                tab_bar_rect.width() + m.left + bw,
                m.top + bw,
                rect.width() - tab_bar_rect.width() - m.left - m.right - 2.0 * bw,
                rect.height() - m.top - m.bottom - 2.0 * bw,
            ),
            TabPosition::Right => Rect::new(
                m.left + bw,
                m.top + bw,
                rect.width() - tab_bar_rect.width() - m.left - m.right - 2.0 * bw,
                rect.height() - m.top - m.bottom - 2.0 * bw,
            ),
        }
    }

    /// Get the frame rectangle (for border).
    fn frame_rect(&self) -> Rect {
        let rect = self.base.rect();
        let tab_bar_rect = self.tab_bar_rect();

        match self.tab_position {
            TabPosition::Top => Rect::new(
                0.0,
                tab_bar_rect.height() - 1.0,
                rect.width(),
                rect.height() - tab_bar_rect.height() + 1.0,
            ),
            TabPosition::Bottom => Rect::new(0.0, 0.0, rect.width(), rect.height() - tab_bar_rect.height() + 1.0),
            TabPosition::Left => Rect::new(
                tab_bar_rect.width() - 1.0,
                0.0,
                rect.width() - tab_bar_rect.width() + 1.0,
                rect.height(),
            ),
            TabPosition::Right => Rect::new(0.0, 0.0, rect.width() - tab_bar_rect.width() + 1.0, rect.height()),
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_frame(&self, ctx: &mut PaintContext<'_>) {
        let frame = self.frame_rect();

        // Background
        ctx.renderer().fill_rect(frame, self.content_background);

        // Border
        if self.border_width > 0.0 {
            let stroke = horizon_lattice_render::Stroke::new(self.border_color, self.border_width);

            // Draw border lines
            let top_left = horizon_lattice_render::Point::new(frame.origin.x, frame.origin.y);
            let top_right =
                horizon_lattice_render::Point::new(frame.origin.x + frame.width(), frame.origin.y);
            let bottom_left =
                horizon_lattice_render::Point::new(frame.origin.x, frame.origin.y + frame.height());
            let bottom_right = horizon_lattice_render::Point::new(
                frame.origin.x + frame.width(),
                frame.origin.y + frame.height(),
            );

            ctx.renderer().draw_line(top_left, top_right, &stroke);
            ctx.renderer().draw_line(top_right, bottom_right, &stroke);
            ctx.renderer().draw_line(bottom_right, bottom_left, &stroke);
            ctx.renderer().draw_line(bottom_left, top_left, &stroke);
        }
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_tab_bar_event(&mut self, event: &mut WidgetEvent) -> bool {
        // Forward events to tab bar if within its bounds
        let tab_bar_rect = self.tab_bar_rect();

        let in_tab_bar = match event {
            WidgetEvent::MousePress(e) => tab_bar_rect.contains(e.local_pos),
            WidgetEvent::MouseRelease(e) => tab_bar_rect.contains(e.local_pos),
            WidgetEvent::MouseMove(e) => tab_bar_rect.contains(e.local_pos),
            WidgetEvent::KeyPress(_) => self.tab_bar.widget_base().has_focus(),
            _ => false,
        };

        if in_tab_bar || matches!(event, WidgetEvent::KeyPress(_)) {
            let old_index = self.tab_bar.current_index();
            let handled = self.tab_bar.event(event);

            // Check for tab change
            let new_index = self.tab_bar.current_index();
            if new_index != old_index {
                self.sync_current_index();
                self.current_changed.emit(new_index);
            }

            // Check for close request (would need to intercept tab_close_requested signal)
            return handled;
        }

        false
    }
}

impl Default for TabWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for TabWidget {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for TabWidget {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let tab_hint = self.tab_bar.size_hint();

        // Default page size hints (would iterate pages in full implementation)
        let max_page_width: f32 = 200.0;
        let max_page_height: f32 = 150.0;

        // Content margins add to size
        let margins = &self.content_margins;
        let extra_width = margins.left + margins.right + 2.0 * self.border_width;
        let extra_height = margins.top + margins.bottom + 2.0 * self.border_width;

        if self.tab_position.is_horizontal() {
            SizeHint::from_dimensions(
                (max_page_width + extra_width).max(tab_hint.preferred.width),
                max_page_height + tab_hint.preferred.height + extra_height,
            )
            .with_minimum_dimensions(
                tab_hint.effective_minimum().width,
                tab_hint.effective_minimum().height + 100.0,
            )
        } else {
            SizeHint::from_dimensions(
                max_page_width + tab_hint.preferred.width + extra_width,
                (max_page_height + extra_height).max(tab_hint.preferred.height),
            )
            .with_minimum_dimensions(
                tab_hint.effective_minimum().width + 100.0,
                tab_hint.effective_minimum().height,
            )
        }
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint frame/background first
        self.paint_frame(ctx);

        // Paint tab bar
        // Note: In a real implementation, the tab bar would be a child widget
        // and painted separately. For now, we delegate painting.
        // This would require setting up the proper transform/clip context.
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle resize to update internal geometry
        if let WidgetEvent::Resize(_) = event {
            // Update tab bar geometry
            let tab_bar_rect = self.tab_bar_rect();
            self.tab_bar.widget_base_mut().set_geometry(tab_bar_rect);

            // Update stack layout geometry
            let content_rect = self.content_rect();
            self.stack_layout.set_geometry(content_rect);
        }

        // Try tab bar first
        if self.handle_tab_bar_event(event) {
            return true;
        }

        false
    }
}

// Ensure TabWidget is Send + Sync
static_assertions::assert_impl_all!(TabWidget: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::base::WidgetBase;
    use crate::widget::traits::{PaintContext, Widget};
    use horizon_lattice_core::{init_global_registry, Object};

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
    fn test_tab_widget_creation() {
        setup();
        let widget = TabWidget::new();
        assert_eq!(widget.count(), 0);
        assert_eq!(widget.current_index(), -1);
        assert_eq!(widget.tab_position(), TabPosition::Top);
    }

    #[test]
    fn test_add_tabs() {
        setup();
        let mut widget = TabWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();

        let idx0 = widget.add_tab(page1_id, "Page 1");
        assert_eq!(idx0, 0);
        assert_eq!(widget.count(), 1);
        assert_eq!(widget.current_index(), 0);
        assert_eq!(widget.current_widget(), Some(page1_id));

        let idx1 = widget.add_tab(page2_id, "Page 2");
        assert_eq!(idx1, 1);
        assert_eq!(widget.count(), 2);
    }

    #[test]
    fn test_remove_tab() {
        setup();
        let mut widget = TabWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();

        widget.add_tab(page1_id, "Page 1");
        widget.add_tab(page2_id, "Page 2");

        let removed = widget.remove_tab(0);
        assert_eq!(removed, Some(page1_id));
        assert_eq!(widget.count(), 1);
        assert_eq!(widget.current_widget(), Some(page2_id));
    }

    #[test]
    fn test_tab_properties() {
        setup();
        let mut widget = TabWidget::new();

        let page = MockWidget::new();
        let page_id = page.object_id();
        widget.add_tab(page_id, "Test");

        widget.set_tab_text(0, "New Name");
        assert_eq!(widget.tab_text(0), Some("New Name"));

        widget.set_tab_enabled(0, false);
        assert!(!widget.is_tab_enabled(0));
    }

    #[test]
    fn test_builder_pattern() {
        setup();
        let widget = TabWidget::new()
            .with_tab_position(TabPosition::Left)
            .with_tabs_closable(true)
            .with_tabs_movable(false);

        assert_eq!(widget.tab_position(), TabPosition::Left);
        assert!(widget.tabs_closable());
        assert!(!widget.tabs_movable());
    }

    #[test]
    fn test_index_of() {
        setup();
        let mut widget = TabWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();
        let page3 = MockWidget::new();
        let page3_id = page3.object_id();

        widget.add_tab(page1_id, "Page 1");
        widget.add_tab(page2_id, "Page 2");

        assert_eq!(widget.index_of(page1_id), 0);
        assert_eq!(widget.index_of(page2_id), 1);
        assert_eq!(widget.index_of(page3_id), -1); // Not found
    }
}
