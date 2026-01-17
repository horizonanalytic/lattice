//! Stack layout for displaying one widget at a time.
//!
//! `StackLayout` manages multiple widgets where only one is visible at a time,
//! similar to a tab widget or wizard. It supports animated transitions between
//! pages.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::layout::*;
//! use horizon_lattice::widget::animation::TransitionType;
//!
//! // Create a stack layout with fade transitions
//! let mut layout = StackLayout::new();
//! layout.set_transition_type(TransitionType::Fade);
//!
//! // Add pages
//! layout.add_widget(page1_id);
//! layout.add_widget(page2_id);
//! layout.add_widget(page3_id);
//!
//! // Switch to page 2
//! layout.set_current_index(1);
//! ```

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

use super::base::LayoutBase;
use super::item::LayoutItem;
use super::traits::Layout;
use super::ContentMargins;
use crate::widget::animation::{Easing, Transition, TransitionState, TransitionType};
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::geometry::{SizeHint, SizePolicy, SizePolicyPair};

use std::time::Duration;

/// How the stack layout determines its size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StackSizeMode {
    /// Size based on all widgets (prevents layout jumping).
    #[default]
    AllWidgets,
    /// Size based only on the current widget.
    CurrentWidget,
}

/// A layout that displays one widget at a time.
///
/// `StackLayout` is useful for:
/// - Tab widgets where content changes based on the selected tab
/// - Wizards or step-by-step interfaces
/// - Any UI where you want to switch between multiple views
///
/// # Size Calculation
///
/// By default, the layout sizes itself based on the maximum size of all
/// widgets to prevent layout jumping when switching pages. This can be
/// changed with [`set_size_mode`](StackLayout::set_size_mode).
///
/// # Transitions
///
/// The layout supports animated transitions between pages:
/// - `None`: Instant switch (default)
/// - `Fade`: Cross-fade between pages
/// - `SlideHorizontal`: Slide left/right
/// - `SlideVertical`: Slide up/down
///
/// # Widget Visibility
///
/// All widgets in the stack are positioned at the same location. Only the
/// current widget (and the transitioning widget during animations) receives
/// paint events. The layout sets the geometry of non-visible widgets to
/// zero size to prevent them from receiving input events.
#[derive(Debug, Clone)]
pub struct StackLayout {
    /// Common layout functionality.
    base: LayoutBase,
    /// The currently visible widget index.
    current_index: usize,
    /// How to determine the layout's size.
    size_mode: StackSizeMode,
    /// Transition controller.
    transition: Transition,
}

impl StackLayout {
    /// Create a new stack layout.
    pub fn new() -> Self {
        Self {
            base: LayoutBase::new(),
            current_index: 0,
            size_mode: StackSizeMode::default(),
            transition: Transition::new(),
        }
    }

    /// Create a stack layout with a specific transition type.
    pub fn with_transition(transition_type: TransitionType) -> Self {
        let mut layout = Self::new();
        layout.transition.set_transition_type(transition_type);
        layout
    }

    /// Get a reference to the underlying layout base.
    #[inline]
    pub fn base(&self) -> &LayoutBase {
        &self.base
    }

    /// Get a mutable reference to the underlying layout base.
    #[inline]
    pub fn base_mut(&mut self) -> &mut LayoutBase {
        &mut self.base
    }

    /// Get the current widget index.
    #[inline]
    pub fn current_index(&self) -> usize {
        self.current_index
    }

    /// Set the current widget index.
    ///
    /// If a transition type is set, this will animate to the new index.
    /// If the index is out of bounds or the same as the current index,
    /// no change occurs.
    ///
    /// # Returns
    ///
    /// `true` if the index was changed, `false` otherwise.
    pub fn set_current_index(&mut self, index: usize) -> bool {
        if index >= self.base.item_count() || index == self.current_index {
            return false;
        }

        let old_index = self.current_index;
        self.current_index = index;

        // Start transition animation
        self.transition.start(old_index, index);

        self.base.invalidate();
        true
    }

    /// Get the current widget if any.
    pub fn current_widget(&self) -> Option<ObjectId> {
        self.base.item_at(self.current_index).and_then(|item| {
            if let LayoutItem::Widget(id) = item {
                Some(*id)
            } else {
                None
            }
        })
    }

    /// Get the size mode.
    #[inline]
    pub fn size_mode(&self) -> StackSizeMode {
        self.size_mode
    }

    /// Set the size mode.
    pub fn set_size_mode(&mut self, mode: StackSizeMode) {
        if self.size_mode != mode {
            self.size_mode = mode;
            self.base.invalidate();
        }
    }

    /// Get the transition type.
    #[inline]
    pub fn transition_type(&self) -> TransitionType {
        self.transition.transition_type()
    }

    /// Set the transition type.
    pub fn set_transition_type(&mut self, transition_type: TransitionType) {
        self.transition.set_transition_type(transition_type);
    }

    /// Get the transition easing function.
    #[inline]
    pub fn transition_easing(&self) -> Easing {
        self.transition.easing()
    }

    /// Set the transition easing function.
    pub fn set_transition_easing(&mut self, easing: Easing) {
        self.transition.set_easing(easing);
    }

    /// Get the transition duration.
    #[inline]
    pub fn transition_duration(&self) -> Duration {
        self.transition.duration()
    }

    /// Set the transition duration.
    pub fn set_transition_duration(&mut self, duration: Duration) {
        self.transition.set_duration(duration);
    }

    /// Check if a transition is currently in progress.
    #[inline]
    pub fn is_transitioning(&self) -> bool {
        self.transition.is_running()
    }

    /// Update the transition state and return it.
    ///
    /// This should be called during the animation loop to advance the transition.
    pub fn update_transition(&mut self) -> TransitionState {
        self.transition.update()
    }

    /// Get a reference to the transition controller.
    #[inline]
    pub fn transition(&self) -> &Transition {
        &self.transition
    }

    /// Get a mutable reference to the transition controller.
    #[inline]
    pub fn transition_mut(&mut self) -> &mut Transition {
        &mut self.transition
    }

    /// Go to the next page.
    ///
    /// Wraps around to the first page if at the end.
    ///
    /// # Returns
    ///
    /// `true` if the page changed.
    pub fn next(&mut self) -> bool {
        if self.base.is_empty() {
            return false;
        }
        let next_index = (self.current_index + 1) % self.base.item_count();
        self.set_current_index(next_index)
    }

    /// Go to the previous page.
    ///
    /// Wraps around to the last page if at the beginning.
    ///
    /// # Returns
    ///
    /// `true` if the page changed.
    pub fn previous(&mut self) -> bool {
        if self.base.is_empty() {
            return false;
        }
        let prev_index = if self.current_index == 0 {
            self.base.item_count() - 1
        } else {
            self.current_index - 1
        };
        self.set_current_index(prev_index)
    }

    /// Calculate the size hint for the layout.
    fn calculate_size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        let margins = self.base.content_margins();

        match self.size_mode {
            StackSizeMode::AllWidgets => {
                // Size based on maximum of all widgets
                let mut max_width: f32 = 0.0;
                let mut max_height: f32 = 0.0;
                let mut max_min_width: f32 = 0.0;
                let mut max_min_height: f32 = 0.0;

                for item in self.base.items() {
                    let hint = self.base.get_item_size_hint(storage, item);
                    max_width = max_width.max(hint.preferred.width);
                    max_height = max_height.max(hint.preferred.height);

                    let min = hint.effective_minimum();
                    max_min_width = max_min_width.max(min.width);
                    max_min_height = max_min_height.max(min.height);
                }

                SizeHint {
                    preferred: Size::new(
                        max_width + margins.horizontal(),
                        max_height + margins.vertical(),
                    ),
                    minimum: Some(Size::new(
                        max_min_width + margins.horizontal(),
                        max_min_height + margins.vertical(),
                    )),
                    maximum: None,
                }
            }
            StackSizeMode::CurrentWidget => {
                // Size based only on current widget
                if let Some(item) = self.base.item_at(self.current_index) {
                    let hint = self.base.get_item_size_hint(storage, item);
                    let min = hint.effective_minimum();

                    SizeHint {
                        preferred: Size::new(
                            hint.preferred.width + margins.horizontal(),
                            hint.preferred.height + margins.vertical(),
                        ),
                        minimum: Some(Size::new(
                            min.width + margins.horizontal(),
                            min.height + margins.vertical(),
                        )),
                        maximum: None,
                    }
                } else {
                    SizeHint::default()
                }
            }
        }
    }

    /// Check if a widget at the given index should be visible.
    fn is_index_visible(&self, index: usize) -> bool {
        self.transition.is_index_visible(index, self.current_index)
    }
}

impl Layout for StackLayout {
    // =========================================================================
    // Item Management
    // =========================================================================

    fn add_item(&mut self, item: LayoutItem) {
        self.base.add_item(item);
    }

    fn insert_item(&mut self, index: usize, item: LayoutItem) {
        self.base.insert_item(index, item);

        // Adjust current index if needed
        if index <= self.current_index && !self.base.is_empty() {
            self.current_index = (self.current_index + 1).min(self.base.item_count() - 1);
        }
    }

    fn remove_item(&mut self, index: usize) -> Option<LayoutItem> {
        let item = self.base.remove_item(index);

        // Adjust current index if needed
        if item.is_some() {
            if self.base.is_empty() {
                self.current_index = 0;
            } else if index < self.current_index {
                self.current_index = self.current_index.saturating_sub(1);
            } else if index == self.current_index {
                self.current_index = self.current_index.min(self.base.item_count().saturating_sub(1));
            }
        }

        item
    }

    fn remove_widget(&mut self, widget: ObjectId) -> bool {
        // Find the index first
        let index = self.base.items().iter().position(|item| {
            matches!(item, LayoutItem::Widget(id) if *id == widget)
        });

        if let Some(idx) = index {
            self.remove_item(idx).is_some()
        } else {
            false
        }
    }

    fn item_count(&self) -> usize {
        self.base.item_count()
    }

    fn item_at(&self, index: usize) -> Option<&LayoutItem> {
        self.base.item_at(index)
    }

    fn item_at_mut(&mut self, index: usize) -> Option<&mut LayoutItem> {
        self.base.item_at_mut(index)
    }

    fn clear(&mut self) {
        self.base.clear();
        self.current_index = 0;
        self.transition.stop();
    }

    // =========================================================================
    // Size Hints & Policies
    // =========================================================================

    fn size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        if let Some(cached) = self.base.cached_size_hint() {
            return cached;
        }
        self.calculate_size_hint(storage)
    }

    fn minimum_size<S: WidgetAccess>(&self, storage: &S) -> Size {
        if let Some(cached) = self.base.cached_minimum_size() {
            return cached;
        }
        self.size_hint(storage).effective_minimum()
    }

    fn size_policy(&self) -> SizePolicyPair {
        // Stack layouts prefer their size but can expand
        SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred)
    }

    // =========================================================================
    // Geometry & Margins
    // =========================================================================

    fn geometry(&self) -> Rect {
        self.base.geometry()
    }

    fn set_geometry(&mut self, rect: Rect) {
        self.base.set_geometry(rect);
    }

    fn content_margins(&self) -> ContentMargins {
        self.base.content_margins()
    }

    fn set_content_margins(&mut self, margins: ContentMargins) {
        self.base.set_content_margins(margins);
    }

    fn spacing(&self) -> f32 {
        // Stack layout doesn't use spacing between items
        0.0
    }

    fn set_spacing(&mut self, _spacing: f32) {
        // No-op: spacing is not applicable for stack layouts
    }

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    fn calculate<S: WidgetAccess>(&mut self, storage: &S, _available: Size) -> Size {
        let content_rect = self.base.content_rect();

        // Update transition state
        let transition_state = self.transition.update();

        // Calculate geometries for all items
        for i in 0..self.base.item_count() {
            let is_visible = self.is_index_visible(i);

            if is_visible {
                // Visible items get the full content rect, potentially with offset
                let mut rect = content_rect;

                // Apply transition offset if transitioning
                if let TransitionState::Running { progress, .. } = transition_state {
                    match self.transition.transition_type() {
                        TransitionType::SlideHorizontal => {
                            let offset = self.transition.slide_horizontal_offset(
                                i,
                                progress,
                                content_rect.width(),
                            );
                            rect = Rect::new(
                                rect.origin.x + offset,
                                rect.origin.y,
                                rect.width(),
                                rect.height(),
                            );
                        }
                        TransitionType::SlideVertical => {
                            let offset = self.transition.slide_vertical_offset(
                                i,
                                progress,
                                content_rect.height(),
                            );
                            rect = Rect::new(
                                rect.origin.x,
                                rect.origin.y + offset,
                                rect.width(),
                                rect.height(),
                            );
                        }
                        TransitionType::Fade | TransitionType::None => {
                            // Fade and None don't use position offset
                        }
                    }
                }

                self.base.set_item_geometry(i, rect);
            } else {
                // Non-visible items get zero size (positioned off-screen)
                self.base.set_item_geometry(i, Rect::ZERO);
            }
        }

        // Cache the calculated size hint
        let size_hint = self.calculate_size_hint(storage);
        self.base.set_cached_size_hint(size_hint);
        self.base.set_cached_minimum_size(size_hint.effective_minimum());

        self.base.mark_valid();

        // Return the actual size used (the geometry size)
        self.base.geometry().size
    }

    fn apply<S: WidgetAccess>(&self, storage: &mut S) {
        for (i, item) in self.base.items().iter().enumerate() {
            if let Some(geometry) = self.base.item_geometry(i) {
                // Only apply geometry to visible items or items being transitioned
                if self.is_index_visible(i) || geometry.width() > 0.0 {
                    LayoutBase::apply_item_geometry(storage, item, geometry);
                }
            }
        }
    }

    // =========================================================================
    // Invalidation
    // =========================================================================

    fn invalidate(&mut self) {
        self.base.invalidate();
    }

    fn needs_recalculation(&self) -> bool {
        self.base.needs_recalculation() || self.transition.is_running()
    }

    // =========================================================================
    // Ownership
    // =========================================================================

    fn parent_widget(&self) -> Option<ObjectId> {
        self.base.parent_widget()
    }

    fn set_parent_widget(&mut self, parent: Option<ObjectId>) {
        self.base.set_parent_widget(parent);
    }
}

impl Default for StackLayout {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::base::WidgetBase;
    use crate::widget::traits::{PaintContext, Widget};
    use horizon_lattice_core::{init_global_registry, Object, ObjectId};
    use std::collections::HashMap;

    /// Mock widget for testing.
    struct MockWidget {
        base: WidgetBase,
        mock_size_hint: SizeHint,
    }

    impl MockWidget {
        fn new(size_hint: SizeHint) -> Self {
            Self {
                base: WidgetBase::new::<Self>(),
                mock_size_hint: size_hint,
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
            self.mock_size_hint
        }

        fn paint(&self, _ctx: &mut PaintContext<'_>) {}
    }

    /// Mock widget storage for testing.
    struct MockStorage {
        widgets: HashMap<ObjectId, MockWidget>,
    }

    impl MockStorage {
        fn new() -> Self {
            Self {
                widgets: HashMap::new(),
            }
        }

        fn add(&mut self, widget: MockWidget) -> ObjectId {
            let id = widget.object_id();
            self.widgets.insert(id, widget);
            id
        }
    }

    impl WidgetAccess for MockStorage {
        fn get_widget(&self, id: ObjectId) -> Option<&dyn Widget> {
            self.widgets.get(&id).map(|w| w as &dyn Widget)
        }

        fn get_widget_mut(&mut self, id: ObjectId) -> Option<&mut dyn Widget> {
            self.widgets.get_mut(&id).map(|w| w as &mut dyn Widget)
        }
    }

    #[test]
    fn test_stack_layout_creation() {
        init_global_registry();

        let layout = StackLayout::new();
        assert_eq!(layout.current_index(), 0);
        assert_eq!(layout.item_count(), 0);
        assert!(layout.is_empty());
    }

    #[test]
    fn test_stack_layout_add_widgets() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 50.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 75.0))));
        let id3 = storage.add(MockWidget::new(SizeHint::new(Size::new(120.0, 60.0))));

        let mut layout = StackLayout::new();
        layout.add_widget(id1);
        layout.add_widget(id2);
        layout.add_widget(id3);

        assert_eq!(layout.item_count(), 3);
        assert_eq!(layout.current_index(), 0);
        assert_eq!(layout.current_widget(), Some(id1));
    }

    #[test]
    fn test_stack_layout_set_current_index() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 50.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 75.0))));

        let mut layout = StackLayout::new();
        layout.add_widget(id1);
        layout.add_widget(id2);

        assert!(layout.set_current_index(1));
        assert_eq!(layout.current_index(), 1);
        assert_eq!(layout.current_widget(), Some(id2));

        // Same index should return false
        assert!(!layout.set_current_index(1));

        // Out of bounds should return false
        assert!(!layout.set_current_index(10));
    }

    #[test]
    fn test_stack_layout_next_previous() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 50.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 75.0))));
        let id3 = storage.add(MockWidget::new(SizeHint::new(Size::new(120.0, 60.0))));

        let mut layout = StackLayout::new();
        layout.add_widget(id1);
        layout.add_widget(id2);
        layout.add_widget(id3);

        assert_eq!(layout.current_index(), 0);

        assert!(layout.next());
        assert_eq!(layout.current_index(), 1);

        assert!(layout.next());
        assert_eq!(layout.current_index(), 2);

        // Wrap around
        assert!(layout.next());
        assert_eq!(layout.current_index(), 0);

        // Go back
        assert!(layout.previous());
        assert_eq!(layout.current_index(), 2);
    }

    #[test]
    fn test_stack_layout_size_hint_all_widgets() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 50.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 75.0))));
        let id3 = storage.add(MockWidget::new(SizeHint::new(Size::new(120.0, 60.0))));

        let mut layout = StackLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.add_widget(id1);
        layout.add_widget(id2);
        layout.add_widget(id3);

        let hint = layout.size_hint(&storage);
        // Should be maximum of all widgets
        assert_eq!(hint.preferred.width, 150.0);
        assert_eq!(hint.preferred.height, 75.0);
    }

    #[test]
    fn test_stack_layout_size_hint_current_widget() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 50.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 75.0))));

        let mut layout = StackLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_size_mode(StackSizeMode::CurrentWidget);
        layout.add_widget(id1);
        layout.add_widget(id2);

        // Should be size of first widget
        let hint = layout.size_hint(&storage);
        assert_eq!(hint.preferred.width, 100.0);
        assert_eq!(hint.preferred.height, 50.0);

        // Switch to second widget
        layout.set_current_index(1);
        layout.invalidate(); // Clear cache

        let hint = layout.size_hint(&storage);
        assert_eq!(hint.preferred.width, 150.0);
        assert_eq!(hint.preferred.height, 75.0);
    }

    #[test]
    fn test_stack_layout_calculate_and_apply() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 50.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 75.0))));

        let mut layout = StackLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.add_widget(id1);
        layout.add_widget(id2);

        layout.set_geometry(Rect::new(0.0, 0.0, 200.0, 100.0));
        layout.calculate(&storage, Size::new(200.0, 100.0));
        layout.apply(&mut storage);

        // First widget should have full geometry
        let w1 = storage.widgets.get(&id1).unwrap();
        assert_eq!(w1.geometry().origin.x, 0.0);
        assert_eq!(w1.geometry().origin.y, 0.0);
        assert_eq!(w1.geometry().width(), 200.0);
        assert_eq!(w1.geometry().height(), 100.0);

        // Second widget should have zero geometry (not visible)
        let w2 = storage.widgets.get(&id2).unwrap();
        assert_eq!(w2.geometry().width(), 0.0);
    }

    #[test]
    fn test_stack_layout_remove_adjusts_index() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 50.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(150.0, 75.0))));
        let id3 = storage.add(MockWidget::new(SizeHint::new(Size::new(120.0, 60.0))));

        let mut layout = StackLayout::new();
        layout.add_widget(id1);
        layout.add_widget(id2);
        layout.add_widget(id3);

        layout.set_current_index(2);
        assert_eq!(layout.current_index(), 2);

        // Remove item before current
        layout.remove_item(0);
        assert_eq!(layout.current_index(), 1); // Should decrement

        // Remove current item
        layout.remove_item(1);
        assert_eq!(layout.current_index(), 0); // Should adjust to last valid
    }

    #[test]
    fn test_stack_layout_transition_settings() {
        init_global_registry();

        let mut layout = StackLayout::new();

        layout.set_transition_type(TransitionType::Fade);
        assert_eq!(layout.transition_type(), TransitionType::Fade);

        layout.set_transition_easing(Easing::EaseOutCubic);
        assert_eq!(layout.transition_easing(), Easing::EaseOutCubic);

        layout.set_transition_duration(Duration::from_millis(500));
        assert_eq!(layout.transition_duration(), Duration::from_millis(500));
    }

    #[test]
    fn test_stack_layout_with_transition() {
        init_global_registry();

        let layout = StackLayout::with_transition(TransitionType::SlideHorizontal);
        assert_eq!(layout.transition_type(), TransitionType::SlideHorizontal);
    }
}
