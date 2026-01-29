//! Box layout for arranging widgets in a row or column.
//!
//! `BoxLayout` arranges items either horizontally or vertically. It provides
//! spacing between items, content margins, and alignment options.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::layout::{BoxLayout, VBoxLayout, Layout};
//!
//! // Create a horizontal box layout
//! let mut layout = BoxLayout::horizontal();
//! layout.set_spacing(10.0);
//!
//! // Add widgets (widget IDs come from the widget system)
//! layout.add_widget(button1.id());
//! layout.add_widget(button2.id());
//! layout.add_stretch(1); // Expanding spacer
//! layout.add_widget(button3.id());
//!
//! // Or use the type alias for vertical layout
//! let mut vbox = VBoxLayout::new();
//! vbox.add_widget(label.id());
//! vbox.add_widget(input.id());
//! ```

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

use super::ContentMargins;
use super::base::LayoutBase;
use super::item::{LayoutItem, SpacerItem, SpacerType};
use super::traits::Layout;
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::geometry::{SizeHint, SizePolicy, SizePolicyPair};

/// Layout orientation for box layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Items are arranged left to right.
    #[default]
    Horizontal,
    /// Items are arranged top to bottom.
    Vertical,
}

impl Orientation {
    /// Get the cross (perpendicular) orientation.
    #[inline]
    pub fn cross(self) -> Self {
        match self {
            Orientation::Horizontal => Orientation::Vertical,
            Orientation::Vertical => Orientation::Horizontal,
        }
    }
}

/// Alignment of items within the layout.
///
/// For horizontal layouts, this affects vertical positioning of items.
/// For vertical layouts, this affects horizontal positioning of items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    /// Align items at the start (left/top).
    Start,
    /// Center items.
    Center,
    /// Align items at the end (right/bottom).
    End,
    /// Stretch items to fill available space (default).
    #[default]
    Stretch,
}

/// A box layout that arranges items horizontally or vertically.
///
/// `BoxLayout` is the foundation for `HBoxLayout` and `VBoxLayout`. It distributes
/// space among items based on their size hints, size policies, and stretch factors.
///
/// # Features
///
/// - Left-to-right (horizontal) or top-to-bottom (vertical) arrangement
/// - Configurable spacing between items
/// - Content margins around all items
/// - Cross-axis alignment (stretch, start, center, end)
/// - Stretch factors for proportional space distribution
/// - Support for spacers (fixed and expanding)
///
/// # Layout Algorithm
///
/// 1. Calculate total preferred and minimum sizes along the main axis
/// 2. Distribute available space using [`LayoutBase::distribute_space`]
/// 3. Position items along the main axis
/// 4. Size and position items on the cross axis based on alignment
#[derive(Debug, Clone)]
pub struct BoxLayout {
    /// Common layout functionality.
    base: LayoutBase,
    /// Whether items are arranged horizontally or vertically.
    orientation: Orientation,
    /// How items are aligned on the cross axis.
    alignment: Alignment,
}

impl BoxLayout {
    /// Create a new box layout with the specified orientation.
    pub fn new(orientation: Orientation) -> Self {
        Self {
            base: LayoutBase::new(),
            orientation,
            alignment: Alignment::default(),
        }
    }

    /// Create a horizontal box layout.
    pub fn horizontal() -> Self {
        Self::new(Orientation::Horizontal)
    }

    /// Create a vertical box layout.
    pub fn vertical() -> Self {
        Self::new(Orientation::Vertical)
    }

    /// Get the layout orientation.
    #[inline]
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Set the layout orientation.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        if self.orientation != orientation {
            self.orientation = orientation;
            self.base.invalidate();
        }
    }

    /// Get the cross-axis alignment.
    #[inline]
    pub fn alignment(&self) -> Alignment {
        self.alignment
    }

    /// Set the cross-axis alignment.
    pub fn set_alignment(&mut self, alignment: Alignment) {
        if self.alignment != alignment {
            self.alignment = alignment;
            self.base.invalidate();
        }
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

    /// Add an expanding spacer with the given stretch factor.
    ///
    /// This is equivalent to adding a spacer that grows to fill available space.
    /// Note: Stretch factors are handled via size policies. For explicit stretch
    /// control, set stretch values on widgets via their size policy.
    pub fn add_stretch(&mut self, _stretch: u8) {
        let spacer = match self.orientation {
            Orientation::Horizontal => {
                SpacerItem::new(Size::ZERO, SpacerType::Expanding, SpacerType::Fixed)
            }
            Orientation::Vertical => {
                SpacerItem::new(Size::ZERO, SpacerType::Fixed, SpacerType::Expanding)
            }
        };
        self.base.add_item(LayoutItem::Spacer(spacer));
    }

    /// Add fixed spacing (non-expanding spacer).
    pub fn add_spacing(&mut self, size: f32) {
        let spacer = match self.orientation {
            Orientation::Horizontal => SpacerItem::horizontal_fixed(size),
            Orientation::Vertical => SpacerItem::vertical_fixed(size),
        };
        self.base.add_item(LayoutItem::Spacer(spacer));
    }

    /// Insert a widget at the specified index.
    pub fn insert_widget(&mut self, index: usize, widget: ObjectId) {
        self.base.insert_item(index, LayoutItem::Widget(widget));
    }

    /// Insert an expanding spacer at the specified index.
    pub fn insert_stretch(&mut self, index: usize) {
        let spacer = match self.orientation {
            Orientation::Horizontal => SpacerItem::horizontal_expanding(),
            Orientation::Vertical => SpacerItem::vertical_expanding(),
        };
        self.base.insert_item(index, LayoutItem::Spacer(spacer));
    }

    // =========================================================================
    // Size Calculation Helpers
    // =========================================================================

    /// Get the main axis component of a size.
    #[inline]
    fn main_axis(&self, size: Size) -> f32 {
        match self.orientation {
            Orientation::Horizontal => size.width,
            Orientation::Vertical => size.height,
        }
    }

    /// Get the cross axis component of a size.
    #[inline]
    fn cross_axis(&self, size: Size) -> f32 {
        match self.orientation {
            Orientation::Horizontal => size.height,
            Orientation::Vertical => size.width,
        }
    }

    /// Create a size from main and cross axis values.
    #[inline]
    fn make_size(&self, main: f32, cross: f32) -> Size {
        match self.orientation {
            Orientation::Horizontal => Size::new(main, cross),
            Orientation::Vertical => Size::new(cross, main),
        }
    }

    /// Get the main axis policy from a size policy pair.
    #[inline]
    fn main_policy(&self, policy: SizePolicyPair) -> SizePolicy {
        match self.orientation {
            Orientation::Horizontal => policy.horizontal,
            Orientation::Vertical => policy.vertical,
        }
    }

    /// Get the main axis stretch from a size policy pair.
    #[inline]
    fn main_stretch(&self, policy: SizePolicyPair) -> u8 {
        match self.orientation {
            Orientation::Horizontal => policy.horizontal_stretch,
            Orientation::Vertical => policy.vertical_stretch,
        }
    }

    /// Calculate the aggregate size hint for the layout.
    fn calculate_size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        let mut total_main_pref: f32 = 0.0;
        let mut total_main_min: f32 = 0.0;
        let mut total_main_max: f32 = 0.0;
        let mut max_cross_pref: f32 = 0.0;
        let mut max_cross_min: f32 = 0.0;
        let mut max_cross_max: f32 = f32::MAX;
        let mut visible_count = 0;

        for item in self.base.items() {
            if !self.base.is_item_visible(storage, item) {
                continue;
            }

            let hint = self.base.get_item_size_hint(storage, item);
            visible_count += 1;

            // Main axis: sum up
            total_main_pref += self.main_axis(hint.preferred);
            total_main_min += self.main_axis(hint.effective_minimum());
            let item_max = self.main_axis(hint.effective_maximum());
            if item_max < f32::MAX - total_main_max {
                total_main_max += item_max;
            } else {
                total_main_max = f32::MAX;
            }

            // Cross axis: take maximum
            max_cross_pref = max_cross_pref.max(self.cross_axis(hint.preferred));
            max_cross_min = max_cross_min.max(self.cross_axis(hint.effective_minimum()));
            max_cross_max = max_cross_max.min(self.cross_axis(hint.effective_maximum()));
        }

        // Add spacing
        if visible_count > 1 {
            let total_spacing = self.base.spacing() * (visible_count - 1) as f32;
            total_main_pref += total_spacing;
            total_main_min += total_spacing;
            if total_main_max < f32::MAX - total_spacing {
                total_main_max += total_spacing;
            }
        }

        // Add margins
        let margins = self.base.content_margins();
        let main_margin = match self.orientation {
            Orientation::Horizontal => margins.horizontal(),
            Orientation::Vertical => margins.vertical(),
        };
        let cross_margin = match self.orientation {
            Orientation::Horizontal => margins.vertical(),
            Orientation::Vertical => margins.horizontal(),
        };

        total_main_pref += main_margin;
        total_main_min += main_margin;
        if total_main_max < f32::MAX - main_margin {
            total_main_max += main_margin;
        }

        max_cross_pref += cross_margin;
        max_cross_min += cross_margin;
        if max_cross_max < f32::MAX - cross_margin {
            max_cross_max += cross_margin;
        }

        // Ensure max >= min
        max_cross_max = max_cross_max.max(max_cross_min);

        SizeHint {
            preferred: self.make_size(total_main_pref, max_cross_pref),
            minimum: Some(self.make_size(total_main_min, max_cross_min)),
            maximum: if total_main_max < f32::MAX && max_cross_max < f32::MAX {
                Some(self.make_size(total_main_max, max_cross_max))
            } else {
                None
            },
        }
    }
}

impl Layout for BoxLayout {
    // =========================================================================
    // Item Management - Delegate to LayoutBase
    // =========================================================================

    fn add_item(&mut self, item: LayoutItem) {
        self.base.add_item(item);
    }

    fn insert_item(&mut self, index: usize, item: LayoutItem) {
        self.base.insert_item(index, item);
    }

    fn remove_item(&mut self, index: usize) -> Option<LayoutItem> {
        self.base.remove_item(index)
    }

    fn remove_widget(&mut self, widget: ObjectId) -> bool {
        self.base.remove_widget(widget)
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
        // Box layouts are generally expanding in their main direction
        match self.orientation {
            Orientation::Horizontal => {
                SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred)
            }
            Orientation::Vertical => {
                SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred)
            }
        }
    }

    // =========================================================================
    // Geometry & Margins - Delegate to LayoutBase
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
        self.base.spacing()
    }

    fn set_spacing(&mut self, spacing: f32) {
        self.base.set_spacing(spacing);
    }

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    fn calculate<S: WidgetAccess>(&mut self, storage: &S, available: Size) -> Size {
        let content_rect = self.base.content_rect();
        let content_main = self.main_axis(content_rect.size);
        let content_cross = self.cross_axis(content_rect.size);

        // Collect item information
        let mut items_info: Vec<(SizeHint, SizePolicy, u8, usize)> = Vec::new();
        let mut visible_indices: Vec<usize> = Vec::new();

        for (i, item) in self.base.items().iter().enumerate() {
            if !self.base.is_item_visible(storage, item) {
                continue;
            }

            let hint = self.base.get_item_size_hint(storage, item);
            let policy = self.base.get_item_size_policy(storage, item);
            let main_policy = self.main_policy(policy);
            let main_stretch = self.main_stretch(policy);

            // Create a main-axis-only hint for distribution
            let main_hint = SizeHint {
                preferred: self.make_size(self.main_axis(hint.preferred), 0.0),
                minimum: hint.minimum.map(|s| self.make_size(self.main_axis(s), 0.0)),
                maximum: hint.maximum.map(|s| self.make_size(self.main_axis(s), 0.0)),
            };

            items_info.push((main_hint, main_policy, main_stretch, i));
            visible_indices.push(i);
        }

        if items_info.is_empty() {
            self.base.mark_valid();
            return available;
        }

        // Calculate total spacing
        let total_spacing = self.base.spacing() * (items_info.len().saturating_sub(1)) as f32;
        let available_for_items = (content_main - total_spacing).max(0.0);

        // Calculate totals for distribution
        let mut total_hint: f32 = 0.0;
        let mut total_min: f32 = 0.0;
        let mut total_max: f32 = 0.0;

        let dist_items: Vec<(SizeHint, SizePolicy, u8)> = items_info
            .iter()
            .map(|(hint, policy, stretch, _)| {
                let pref = self.main_axis(hint.preferred);
                let min = self.main_axis(hint.effective_minimum());
                let max = self.main_axis(hint.effective_maximum());
                total_hint += pref;
                total_min += min;
                if max < f32::MAX - total_max {
                    total_max += max;
                } else {
                    total_max = f32::MAX;
                }
                (*hint, *policy, *stretch)
            })
            .collect();

        // Distribute space
        let sizes = LayoutBase::distribute_space(
            &dist_items,
            available_for_items,
            total_hint,
            total_min,
            total_max,
        );

        // Position items
        let content_x = content_rect.origin.x;
        let content_y = content_rect.origin.y;
        let mut main_pos: f32 = 0.0;

        for (idx, &(_, _, _, item_index)) in items_info.iter().enumerate() {
            let item_main_size = sizes[idx];
            let item = &self.base.items()[item_index];
            let item_hint = self.base.get_item_size_hint(storage, item);

            // Calculate cross-axis size based on alignment
            let (cross_pos, cross_size) = match self.alignment {
                Alignment::Stretch => (0.0, content_cross),
                Alignment::Start => {
                    let size = self.cross_axis(item_hint.preferred).min(content_cross);
                    (0.0, size)
                }
                Alignment::Center => {
                    let size = self.cross_axis(item_hint.preferred).min(content_cross);
                    ((content_cross - size) / 2.0, size)
                }
                Alignment::End => {
                    let size = self.cross_axis(item_hint.preferred).min(content_cross);
                    (content_cross - size, size)
                }
            };

            // Create item rect (with RTL mirroring for horizontal layouts)
            let rect = match self.orientation {
                Orientation::Horizontal => {
                    // Mirror x position for RTL layouts
                    let x_pos = self.base.mirror_x(main_pos, item_main_size, content_main);
                    Rect::new(
                        content_x + x_pos,
                        content_y + cross_pos,
                        item_main_size,
                        cross_size,
                    )
                }
                Orientation::Vertical => Rect::new(
                    content_x + cross_pos,
                    content_y + main_pos,
                    cross_size,
                    item_main_size,
                ),
            };

            self.base.set_item_geometry(item_index, rect);
            main_pos += item_main_size + self.base.spacing();
        }

        // Cache the calculated size hint
        let size_hint = self.calculate_size_hint(storage);
        self.base.set_cached_size_hint(size_hint);
        self.base
            .set_cached_minimum_size(size_hint.effective_minimum());

        self.base.mark_valid();
        available
    }

    fn apply<S: WidgetAccess>(&self, storage: &mut S) {
        for (i, item) in self.base.items().iter().enumerate() {
            if let Some(geometry) = self.base.item_geometry(i) {
                LayoutBase::apply_item_geometry(storage, item, geometry);
            }
        }
    }

    // =========================================================================
    // Invalidation - Delegate to LayoutBase
    // =========================================================================

    fn invalidate(&mut self) {
        self.base.invalidate();
    }

    fn needs_recalculation(&self) -> bool {
        self.base.needs_recalculation()
    }

    // =========================================================================
    // Ownership - Delegate to LayoutBase
    // =========================================================================

    fn parent_widget(&self) -> Option<ObjectId> {
        self.base.parent_widget()
    }

    fn set_parent_widget(&mut self, parent: Option<ObjectId>) {
        self.base.set_parent_widget(parent);
    }
}

impl Default for BoxLayout {
    fn default() -> Self {
        Self::horizontal()
    }
}

// =============================================================================
// Type Aliases for Convenience
// =============================================================================

/// Horizontal box layout.
///
/// A convenience wrapper around `BoxLayout` with horizontal orientation.
/// Items are arranged left to right.
pub type HBoxLayout = BoxLayout;

/// Vertical box layout.
///
/// A convenience wrapper around `BoxLayout` with vertical orientation.
/// Items are arranged top to bottom.
pub type VBoxLayout = BoxLayout;

/// Convenient constructor functions for box layouts.
impl BoxLayout {
    /// Create a new HBoxLayout (alias for `BoxLayout::horizontal()`).
    pub fn hbox() -> Self {
        Self::horizontal()
    }

    /// Create a new VBoxLayout (alias for `BoxLayout::vertical()`).
    pub fn vbox() -> Self {
        Self::vertical()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::base::WidgetBase;
    use crate::widget::geometry::SizeHint;
    use crate::widget::traits::{PaintContext, Widget};
    use horizon_lattice_core::{Object, ObjectId, init_global_registry};
    use std::collections::HashMap;

    /// Mock widget for testing layouts.
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

        fn with_policy(mut self, policy: SizePolicyPair) -> Self {
            self.base.set_size_policy(policy);
            self
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

        fn paint(&self, _ctx: &mut PaintContext<'_>) {
            // No-op for tests
        }
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

    // Helper to create test ObjectIds using SlotMap
    fn create_test_ids(count: usize) -> Vec<ObjectId> {
        use slotmap::SlotMap;
        let mut map: SlotMap<ObjectId, ()> = SlotMap::with_key();
        (0..count).map(|_| map.insert(())).collect()
    }

    #[test]
    fn test_box_layout_creation() {
        init_global_registry();

        let hbox = BoxLayout::horizontal();
        assert_eq!(hbox.orientation(), Orientation::Horizontal);
        assert_eq!(hbox.item_count(), 0);

        let vbox = BoxLayout::vertical();
        assert_eq!(vbox.orientation(), Orientation::Vertical);
    }

    #[test]
    fn test_box_layout_add_items() {
        init_global_registry();

        let mut layout = BoxLayout::horizontal();
        let ids = create_test_ids(2);

        layout.add_widget(ids[0]);
        layout.add_widget(ids[1]);

        assert_eq!(layout.item_count(), 2);
        assert!(layout.item_at(0).unwrap().is_widget());
        assert!(layout.item_at(1).unwrap().is_widget());
    }

    #[test]
    fn test_box_layout_spacing() {
        init_global_registry();

        let mut layout = BoxLayout::horizontal();
        layout.add_spacing(20.0);

        assert_eq!(layout.item_count(), 1);
        assert!(layout.item_at(0).unwrap().is_spacer());
    }

    #[test]
    fn test_box_layout_stretch() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));

        let mut layout = BoxLayout::horizontal();
        layout.add_widget(id1);
        layout.add_stretch(1);
        layout.add_widget(id2);

        assert_eq!(layout.item_count(), 3);
    }

    #[test]
    fn test_box_layout_size_hint() {
        init_global_registry();

        let mut storage = MockStorage::new();

        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(80.0, 40.0))));

        let mut layout = BoxLayout::horizontal();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_spacing(0.0);
        layout.add_widget(id1);
        layout.add_widget(id2);

        let hint = layout.size_hint(&storage);
        // Horizontal: widths add up, heights take max
        assert_eq!(hint.preferred.width, 180.0); // 100 + 80
        assert_eq!(hint.preferred.height, 40.0); // max(30, 40)
    }

    #[test]
    fn test_box_layout_calculate_horizontal() {
        init_global_registry();

        let mut storage = MockStorage::new();

        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));

        let mut layout = BoxLayout::horizontal();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_spacing(10.0);
        layout.add_widget(id1);
        layout.add_widget(id2);

        // Set geometry and calculate
        layout.set_geometry(Rect::new(0.0, 0.0, 300.0, 50.0));
        layout.calculate(&storage, Size::new(300.0, 50.0));
        layout.apply(&mut storage);

        // Check widget geometries
        let w1 = storage.widgets.get(&id1).unwrap();
        let w2 = storage.widgets.get(&id2).unwrap();

        // First widget at x=0
        assert_eq!(w1.geometry().origin.x, 0.0);
        // Second widget after first + spacing
        assert_eq!(w2.geometry().origin.x, w1.geometry().width() + 10.0);
    }

    #[test]
    fn test_box_layout_calculate_vertical() {
        init_global_registry();

        let mut storage = MockStorage::new();

        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 40.0))));

        let mut layout = BoxLayout::vertical();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_spacing(5.0);
        layout.add_widget(id1);
        layout.add_widget(id2);

        layout.set_geometry(Rect::new(0.0, 0.0, 150.0, 200.0));
        layout.calculate(&storage, Size::new(150.0, 200.0));
        layout.apply(&mut storage);

        let w1 = storage.widgets.get(&id1).unwrap();
        let w2 = storage.widgets.get(&id2).unwrap();

        // First widget at y=0
        assert_eq!(w1.geometry().origin.y, 0.0);
        // Second widget after first + spacing
        assert_eq!(w2.geometry().origin.y, w1.geometry().height() + 5.0);
    }

    #[test]
    fn test_box_layout_alignment() {
        init_global_registry();

        let mut storage = MockStorage::new();

        let id = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 20.0))));

        let mut layout = BoxLayout::horizontal();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_spacing(0.0);
        layout.set_alignment(Alignment::Center);
        layout.add_widget(id);

        layout.set_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
        layout.calculate(&storage, Size::new(100.0, 100.0));
        layout.apply(&mut storage);

        let w = storage.widgets.get(&id).unwrap();
        // Cross-axis (vertical) should be centered
        // (100 - 20) / 2 = 40
        assert_eq!(w.geometry().origin.y, 40.0);
    }

    #[test]
    fn test_box_layout_margins() {
        init_global_registry();

        let mut storage = MockStorage::new();

        let id = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));

        let mut layout = BoxLayout::horizontal();
        layout.set_content_margins(ContentMargins::new(10.0, 20.0, 10.0, 20.0));
        layout.set_spacing(0.0);
        layout.add_widget(id);

        layout.set_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
        layout.calculate(&storage, Size::new(100.0, 100.0));
        layout.apply(&mut storage);

        let w = storage.widgets.get(&id).unwrap();
        // Widget should start after left margin
        assert_eq!(w.geometry().origin.x, 10.0);
        // And after top margin
        assert_eq!(w.geometry().origin.y, 20.0);
    }

    #[test]
    fn test_orientation_cross() {
        assert_eq!(Orientation::Horizontal.cross(), Orientation::Vertical);
        assert_eq!(Orientation::Vertical.cross(), Orientation::Horizontal);
    }

    #[test]
    fn test_remove_widget() {
        init_global_registry();

        let mut layout = BoxLayout::horizontal();
        let ids = create_test_ids(2);

        layout.add_widget(ids[0]);
        layout.add_widget(ids[1]);
        assert_eq!(layout.item_count(), 2);

        assert!(layout.remove_widget(ids[0]));
        assert_eq!(layout.item_count(), 1);
        assert!(!layout.remove_widget(ids[0])); // Already removed
    }

    #[test]
    fn test_rtl_horizontal_layout() {
        use crate::platform::TextDirection;

        init_global_registry();

        let mut storage = MockStorage::new();

        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));

        let mut layout = BoxLayout::horizontal();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_spacing(10.0);
        layout.base_mut().set_text_direction(TextDirection::Rtl);
        layout.add_widget(id1);
        layout.add_widget(id2);

        // Set geometry: 300 wide, 50 tall
        layout.set_geometry(Rect::new(0.0, 0.0, 300.0, 50.0));
        layout.calculate(&storage, Size::new(300.0, 50.0));
        layout.apply(&mut storage);

        let w1 = storage.widgets.get(&id1).unwrap();
        let w2 = storage.widgets.get(&id2).unwrap();

        // In RTL, first widget (id1) should be on the right, second (id2) on the left
        // w1.x should be greater than w2.x
        assert!(
            w1.geometry().origin.x > w2.geometry().origin.x,
            "In RTL, w1 (x={}) should be to the right of w2 (x={})",
            w1.geometry().origin.x,
            w2.geometry().origin.x
        );

        // Verify w1 is positioned from the right edge
        // w1 should end at or near the right edge (300)
        let w1_right = w1.geometry().origin.x + w1.geometry().width();
        assert!(
            (w1_right - 300.0).abs() < 1.0,
            "w1 right edge ({}) should be near 300",
            w1_right
        );
    }

    #[test]
    fn test_ltr_horizontal_layout() {
        use crate::platform::TextDirection;

        init_global_registry();

        let mut storage = MockStorage::new();

        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));

        let mut layout = BoxLayout::horizontal();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_spacing(10.0);
        layout.base_mut().set_text_direction(TextDirection::Ltr);
        layout.add_widget(id1);
        layout.add_widget(id2);

        layout.set_geometry(Rect::new(0.0, 0.0, 300.0, 50.0));
        layout.calculate(&storage, Size::new(300.0, 50.0));
        layout.apply(&mut storage);

        let w1 = storage.widgets.get(&id1).unwrap();
        let w2 = storage.widgets.get(&id2).unwrap();

        // In LTR, first widget at x=0, second widget after first + spacing
        assert_eq!(w1.geometry().origin.x, 0.0);
        // w2 should be after w1 + spacing
        assert_eq!(
            w2.geometry().origin.x,
            w1.geometry().width() + 10.0,
            "w2 should start after w1 ({}) + spacing (10)",
            w1.geometry().width()
        );
    }

    #[test]
    fn test_rtl_vertical_layout_no_change() {
        use crate::platform::TextDirection;

        init_global_registry();

        let mut storage = MockStorage::new();

        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 40.0))));

        let mut layout = BoxLayout::vertical();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_spacing(5.0);
        layout.base_mut().set_text_direction(TextDirection::Rtl);
        layout.add_widget(id1);
        layout.add_widget(id2);

        layout.set_geometry(Rect::new(0.0, 0.0, 150.0, 200.0));
        layout.calculate(&storage, Size::new(150.0, 200.0));
        layout.apply(&mut storage);

        let w1 = storage.widgets.get(&id1).unwrap();
        let w2 = storage.widgets.get(&id2).unwrap();

        // Vertical layout is not affected by RTL in terms of y positions
        // Both should start at y=0 with w2 after w1 + spacing
        assert_eq!(w1.geometry().origin.y, 0.0);
        // w2 should be after w1's height + spacing
        assert_eq!(
            w2.geometry().origin.y,
            w1.geometry().height() + 5.0,
            "w2 y ({}) should be w1 height ({}) + 5",
            w2.geometry().origin.y,
            w1.geometry().height()
        );
    }
}
