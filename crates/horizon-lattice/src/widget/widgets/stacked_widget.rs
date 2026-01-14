//! StackedWidget container implementation.
//!
//! This module provides [`StackedWidget`], a container that displays one child
//! widget at a time, with support for animated transitions between pages.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::StackedWidget;
//!
//! // Create a stacked widget
//! let mut stack = StackedWidget::new();
//!
//! // Add pages (each page is a widget ID)
//! stack.add_widget(page1_id);
//! stack.add_widget(page2_id);
//! stack.add_widget(page3_id);
//!
//! // Switch to page 2
//! stack.set_current_index(1);
//!
//! // Connect to page changes
//! stack.current_changed.connect(|&index| {
//!     println!("Switched to page: {}", index);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Rect, Renderer, Stroke};

use crate::widget::animation::{Easing, TransitionType};
use crate::widget::layout::{ContentMargins, Layout, LayoutItem, StackLayout, StackSizeMode};
use crate::widget::{PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent};

use std::time::Duration;

/// A container widget that displays one child widget at a time.
///
/// StackedWidget manages multiple child widgets (pages) where only one is
/// visible at a time. This is useful for wizards, multi-page dialogs, or
/// any interface where you want to switch between different views.
///
/// # Features
///
/// - Multiple child widgets with only one visible
/// - Animated transitions between pages (fade, slide)
/// - Two size modes: based on all widgets or current widget only
/// - Simple index-based navigation
///
/// # Signals
///
/// - `current_changed(i32)`: Emitted when the current page changes
pub struct StackedWidget {
    /// Widget base.
    base: WidgetBase,

    /// Child widget IDs (pages).
    pages: Vec<ObjectId>,

    /// Stack layout for page management.
    stack_layout: StackLayout,

    /// Content margins around the page area.
    content_margins: ContentMargins,

    /// Background color.
    background_color: Color,

    /// Border color.
    border_color: Color,

    /// Border width.
    border_width: f32,

    /// Signal emitted when current page changes.
    pub current_changed: Signal<i32>,
}

impl StackedWidget {
    /// Create a new stacked widget.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Expanding));

        Self {
            base,
            pages: Vec::new(),
            stack_layout: StackLayout::new(),
            content_margins: ContentMargins::uniform(0.0),
            background_color: Color::TRANSPARENT,
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
            current_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Page Management
    // =========================================================================

    /// Add a widget to the stack.
    ///
    /// Returns the index of the new page.
    pub fn add_widget(&mut self, widget_id: ObjectId) -> i32 {
        self.pages.push(widget_id);
        self.stack_layout.add_widget(widget_id);
        self.base.update();
        (self.pages.len() - 1) as i32
    }

    /// Insert a widget at the specified index.
    ///
    /// Returns the actual index where the widget was inserted.
    pub fn insert_widget(&mut self, index: i32, widget_id: ObjectId) -> i32 {
        let insert_pos = if index < 0 {
            0
        } else {
            (index as usize).min(self.pages.len())
        };

        self.pages.insert(insert_pos, widget_id);
        self.stack_layout.insert_item(insert_pos, LayoutItem::Widget(widget_id));
        self.base.update();
        insert_pos as i32
    }

    /// Remove the widget at the specified index.
    ///
    /// Returns the widget ID of the removed page, if any.
    pub fn remove_widget(&mut self, index: i32) -> Option<ObjectId> {
        if index < 0 || index as usize >= self.pages.len() {
            return None;
        }

        let old_current = self.current_index();
        let widget_id = self.pages.remove(index as usize);
        self.stack_layout.remove_item(index as usize);
        self.base.update();

        // Emit signal if current index changed
        let new_current = self.current_index();
        if new_current != old_current {
            self.current_changed.emit(new_current);
        }

        Some(widget_id)
    }

    /// Remove a widget by its ID.
    ///
    /// Returns `true` if the widget was found and removed.
    pub fn remove_widget_by_id(&mut self, widget_id: ObjectId) -> bool {
        let index = self.index_of(widget_id);
        if index >= 0 {
            self.remove_widget(index).is_some()
        } else {
            false
        }
    }

    /// Remove all widgets from the stack.
    pub fn clear(&mut self) {
        self.pages.clear();
        self.stack_layout.clear();
        self.base.update();
    }

    /// Get the number of pages.
    pub fn count(&self) -> i32 {
        self.pages.len() as i32
    }

    /// Check if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty()
    }

    // =========================================================================
    // Current Page
    // =========================================================================

    /// Get the current page index.
    ///
    /// Returns -1 if there are no pages.
    pub fn current_index(&self) -> i32 {
        if self.pages.is_empty() {
            -1
        } else {
            self.stack_layout.current_index() as i32
        }
    }

    /// Set the current page index.
    ///
    /// Returns `true` if the index changed.
    pub fn set_current_index(&mut self, index: i32) -> bool {
        if index < 0 || index as usize >= self.pages.len() {
            return false;
        }

        if self.stack_layout.set_current_index(index as usize) {
            self.base.update();
            self.current_changed.emit(index);
            true
        } else {
            false
        }
    }

    /// Get the widget ID of the current page.
    pub fn current_widget(&self) -> Option<ObjectId> {
        let index = self.current_index();
        if index >= 0 {
            self.pages.get(index as usize).copied()
        } else {
            None
        }
    }

    /// Set the current page by widget ID.
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
            self.pages.get(index as usize).copied()
        }
    }

    /// Find the index of a widget.
    ///
    /// Returns -1 if the widget is not in the stack.
    pub fn index_of(&self, widget_id: ObjectId) -> i32 {
        self.pages
            .iter()
            .position(|&id| id == widget_id)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    // =========================================================================
    // Size Mode
    // =========================================================================

    /// Get the size mode.
    pub fn size_mode(&self) -> StackSizeMode {
        self.stack_layout.size_mode()
    }

    /// Set the size mode.
    ///
    /// - `AllWidgets`: Size based on maximum of all widgets (prevents layout jumping)
    /// - `CurrentWidget`: Size based only on the current widget
    pub fn set_size_mode(&mut self, mode: StackSizeMode) {
        self.stack_layout.set_size_mode(mode);
        self.base.update();
    }

    /// Set size mode using builder pattern.
    pub fn with_size_mode(mut self, mode: StackSizeMode) -> Self {
        self.set_size_mode(mode);
        self
    }

    // =========================================================================
    // Transitions
    // =========================================================================

    /// Get the transition type.
    pub fn transition_type(&self) -> TransitionType {
        self.stack_layout.transition_type()
    }

    /// Set the transition type for page switches.
    pub fn set_transition_type(&mut self, transition_type: TransitionType) {
        self.stack_layout.set_transition_type(transition_type);
    }

    /// Set transition type using builder pattern.
    pub fn with_transition_type(mut self, transition_type: TransitionType) -> Self {
        self.set_transition_type(transition_type);
        self
    }

    /// Get the transition easing function.
    pub fn transition_easing(&self) -> Easing {
        self.stack_layout.transition_easing()
    }

    /// Set the transition easing function.
    pub fn set_transition_easing(&mut self, easing: Easing) {
        self.stack_layout.set_transition_easing(easing);
    }

    /// Set transition easing using builder pattern.
    pub fn with_transition_easing(mut self, easing: Easing) -> Self {
        self.set_transition_easing(easing);
        self
    }

    /// Get the transition duration.
    pub fn transition_duration(&self) -> Duration {
        self.stack_layout.transition_duration()
    }

    /// Set the transition duration.
    pub fn set_transition_duration(&mut self, duration: Duration) {
        self.stack_layout.set_transition_duration(duration);
    }

    /// Set transition duration using builder pattern.
    pub fn with_transition_duration(mut self, duration: Duration) -> Self {
        self.set_transition_duration(duration);
        self
    }

    /// Check if a transition is currently in progress.
    pub fn is_transitioning(&self) -> bool {
        self.stack_layout.is_transitioning()
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the content margins.
    pub fn content_margins(&self) -> ContentMargins {
        self.content_margins
    }

    /// Set the content margins around the page area.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        self.content_margins = margins;
        self.stack_layout.set_content_margins(margins);
        self.base.update();
    }

    /// Set content margins using builder pattern.
    pub fn with_content_margins(mut self, margins: ContentMargins) -> Self {
        self.set_content_margins(margins);
        self
    }

    /// Get the background color.
    pub fn background_color(&self) -> Color {
        self.background_color
    }

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Color) {
        self.background_color = color;
        self.base.update();
    }

    /// Set background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.set_background_color(color);
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

    /// Get the border width.
    pub fn border_width(&self) -> f32 {
        self.border_width
    }

    /// Set the border width.
    pub fn set_border_width(&mut self, width: f32) {
        self.border_width = width;
        self.base.update();
    }

    /// Set border width using builder pattern.
    pub fn with_border_width(mut self, width: f32) -> Self {
        self.set_border_width(width);
        self
    }

    // =========================================================================
    // Layout Access
    // =========================================================================

    /// Get a reference to the underlying stack layout.
    ///
    /// This allows advanced configuration of the layout.
    pub fn stack_layout(&self) -> &StackLayout {
        &self.stack_layout
    }

    /// Get a mutable reference to the underlying stack layout.
    pub fn stack_layout_mut(&mut self) -> &mut StackLayout {
        &mut self.stack_layout
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    /// Get the content rectangle (page area).
    fn content_rect(&self) -> Rect {
        let rect = self.base.rect();
        let m = &self.content_margins;
        let bw = self.border_width;

        Rect::new(
            m.left + bw,
            m.top + bw,
            (rect.width() - m.left - m.right - 2.0 * bw).max(0.0),
            (rect.height() - m.top - m.bottom - 2.0 * bw).max(0.0),
        )
    }
}

impl Default for StackedWidget {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for StackedWidget {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for StackedWidget {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Base size from content margins and border
        let m = &self.content_margins;
        let extra_width = m.left + m.right + 2.0 * self.border_width;
        let extra_height = m.top + m.bottom + 2.0 * self.border_width;

        // Default minimum size for empty stack
        let min_width = 50.0 + extra_width;
        let min_height = 50.0 + extra_height;

        // Preferred size (would be calculated from pages in full implementation)
        let preferred_width = 200.0 + extra_width;
        let preferred_height = 150.0 + extra_height;

        SizeHint::from_dimensions(preferred_width, preferred_height)
            .with_minimum_dimensions(min_width, min_height)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Background
        if self.background_color.a > 0.0 {
            ctx.renderer().fill_rect(rect, self.background_color);
        }

        // Border
        if self.border_width > 0.0 && self.border_color.a > 0.0 {
            let stroke = Stroke::new(self.border_color, self.border_width);
            ctx.renderer().stroke_rect(rect, &stroke);
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        // Handle resize to update stack layout geometry
        if let WidgetEvent::Resize(_) = event {
            let content_rect = self.content_rect();
            self.stack_layout.set_geometry(content_rect);
            return true;
        }

        false
    }
}

// Ensure StackedWidget is Send + Sync
static_assertions::assert_impl_all!(StackedWidget: Send, Sync);

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
    fn test_stacked_widget_creation() {
        setup();
        let widget = StackedWidget::new();
        assert_eq!(widget.count(), 0);
        assert_eq!(widget.current_index(), -1);
        assert!(widget.is_empty());
        assert!(widget.current_widget().is_none());
    }

    #[test]
    fn test_add_widgets() {
        setup();
        let mut widget = StackedWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();

        let idx0 = widget.add_widget(page1_id);
        assert_eq!(idx0, 0);
        assert_eq!(widget.count(), 1);
        assert_eq!(widget.current_index(), 0);
        assert_eq!(widget.current_widget(), Some(page1_id));
        assert!(!widget.is_empty());

        let idx1 = widget.add_widget(page2_id);
        assert_eq!(idx1, 1);
        assert_eq!(widget.count(), 2);
        assert_eq!(widget.current_index(), 0); // Still on first page
    }

    #[test]
    fn test_set_current_index() {
        setup();
        let mut widget = StackedWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();

        widget.add_widget(page1_id);
        widget.add_widget(page2_id);

        assert!(widget.set_current_index(1));
        assert_eq!(widget.current_index(), 1);
        assert_eq!(widget.current_widget(), Some(page2_id));

        // Same index should return false
        assert!(!widget.set_current_index(1));

        // Out of bounds should return false
        assert!(!widget.set_current_index(10));
        assert!(!widget.set_current_index(-1));
    }

    #[test]
    fn test_set_current_widget() {
        setup();
        let mut widget = StackedWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();
        let page3 = MockWidget::new();
        let page3_id = page3.object_id();

        widget.add_widget(page1_id);
        widget.add_widget(page2_id);

        assert!(widget.set_current_widget(page2_id));
        assert_eq!(widget.current_index(), 1);

        // Widget not in stack
        assert!(!widget.set_current_widget(page3_id));
    }

    #[test]
    fn test_remove_widget() {
        setup();
        let mut widget = StackedWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();

        widget.add_widget(page1_id);
        widget.add_widget(page2_id);

        let removed = widget.remove_widget(0);
        assert_eq!(removed, Some(page1_id));
        assert_eq!(widget.count(), 1);
        assert_eq!(widget.current_widget(), Some(page2_id));
    }

    #[test]
    fn test_remove_widget_by_id() {
        setup();
        let mut widget = StackedWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();

        widget.add_widget(page1_id);
        widget.add_widget(page2_id);

        assert!(widget.remove_widget_by_id(page1_id));
        assert_eq!(widget.count(), 1);
        assert!(!widget.remove_widget_by_id(page1_id)); // Already removed
    }

    #[test]
    fn test_insert_widget() {
        setup();
        let mut widget = StackedWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();
        let page3 = MockWidget::new();
        let page3_id = page3.object_id();

        widget.add_widget(page1_id);
        widget.add_widget(page3_id);

        // Insert in the middle
        let idx = widget.insert_widget(1, page2_id);
        assert_eq!(idx, 1);
        assert_eq!(widget.count(), 3);
        assert_eq!(widget.widget(0), Some(page1_id));
        assert_eq!(widget.widget(1), Some(page2_id));
        assert_eq!(widget.widget(2), Some(page3_id));
    }

    #[test]
    fn test_clear() {
        setup();
        let mut widget = StackedWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();

        widget.add_widget(page1_id);
        widget.add_widget(page2_id);
        assert_eq!(widget.count(), 2);

        widget.clear();
        assert_eq!(widget.count(), 0);
        assert!(widget.is_empty());
        assert_eq!(widget.current_index(), -1);
    }

    #[test]
    fn test_index_of() {
        setup();
        let mut widget = StackedWidget::new();

        let page1 = MockWidget::new();
        let page1_id = page1.object_id();
        let page2 = MockWidget::new();
        let page2_id = page2.object_id();
        let page3 = MockWidget::new();
        let page3_id = page3.object_id();

        widget.add_widget(page1_id);
        widget.add_widget(page2_id);

        assert_eq!(widget.index_of(page1_id), 0);
        assert_eq!(widget.index_of(page2_id), 1);
        assert_eq!(widget.index_of(page3_id), -1); // Not found
    }

    #[test]
    fn test_builder_pattern() {
        setup();
        let widget = StackedWidget::new()
            .with_size_mode(StackSizeMode::CurrentWidget)
            .with_transition_type(TransitionType::Fade)
            .with_transition_duration(Duration::from_millis(300))
            .with_content_margins(ContentMargins::uniform(10.0))
            .with_background_color(Color::WHITE);

        assert_eq!(widget.size_mode(), StackSizeMode::CurrentWidget);
        assert_eq!(widget.transition_type(), TransitionType::Fade);
        assert_eq!(widget.transition_duration(), Duration::from_millis(300));
        assert_eq!(widget.content_margins(), ContentMargins::uniform(10.0));
        assert_eq!(widget.background_color(), Color::WHITE);
    }

    #[test]
    fn test_transition_settings() {
        setup();
        let mut widget = StackedWidget::new();

        widget.set_transition_type(TransitionType::SlideHorizontal);
        assert_eq!(widget.transition_type(), TransitionType::SlideHorizontal);

        widget.set_transition_easing(Easing::EaseOutCubic);
        assert_eq!(widget.transition_easing(), Easing::EaseOutCubic);

        widget.set_transition_duration(Duration::from_millis(500));
        assert_eq!(widget.transition_duration(), Duration::from_millis(500));
    }
}
