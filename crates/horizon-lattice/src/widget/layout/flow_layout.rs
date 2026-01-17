//! Flow layout for wrapping horizontal arrangement.
//!
//! `FlowLayout` arranges items horizontally from left to right, wrapping to
//! the next row when the available width is exceeded. This is similar to how
//! text flows in a paragraph or how items wrap in a CSS flexbox with wrap.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::layout::*;
//!
//! // Create a flow layout for a toolbar or tag list
//! let mut layout = FlowLayout::new();
//! layout.set_horizontal_spacing(8.0);
//! layout.set_vertical_spacing(4.0);
//! layout.add_widget(tag1_id);
//! layout.add_widget(tag2_id);
//! layout.add_widget(tag3_id);
//! // Items will wrap to new rows as needed
//! ```

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

use super::base::LayoutBase;
use super::box_layout::Alignment;
use super::item::LayoutItem;
use super::traits::Layout;
use super::ContentMargins;
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::geometry::{SizeHint, SizePolicy, SizePolicyPair};

/// A flow layout that wraps items horizontally.
///
/// Items are placed left to right until the available width is exceeded,
/// then the layout continues on the next row. This is ideal for:
///
/// - Tag lists and chip displays
/// - Button groups that adapt to width
/// - Image galleries
/// - Any content that should wrap like text
///
/// # Features
///
/// - Left-to-right, top-to-bottom flow
/// - Independent horizontal and vertical spacing
/// - Configurable alignment for incomplete rows
/// - Height depends on available width (height-for-width)
#[derive(Debug, Clone)]
pub struct FlowLayout {
    /// Common layout functionality.
    base: LayoutBase,
    /// Horizontal spacing between items in a row.
    horizontal_spacing: f32,
    /// Vertical spacing between rows.
    vertical_spacing: f32,
    /// How to align items in the last (incomplete) row.
    alignment: Alignment,
}

impl FlowLayout {
    /// Create a new flow layout with default settings.
    pub fn new() -> Self {
        Self {
            base: LayoutBase::new(),
            horizontal_spacing: super::DEFAULT_SPACING,
            vertical_spacing: super::DEFAULT_SPACING,
            alignment: Alignment::Start,
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

    /// Get the horizontal spacing between items.
    #[inline]
    pub fn horizontal_spacing(&self) -> f32 {
        self.horizontal_spacing
    }

    /// Set the horizontal spacing between items.
    pub fn set_horizontal_spacing(&mut self, spacing: f32) {
        if (self.horizontal_spacing - spacing).abs() > f32::EPSILON {
            self.horizontal_spacing = spacing;
            self.base.invalidate();
        }
    }

    /// Get the vertical spacing between rows.
    #[inline]
    pub fn vertical_spacing(&self) -> f32 {
        self.vertical_spacing
    }

    /// Set the vertical spacing between rows.
    pub fn set_vertical_spacing(&mut self, spacing: f32) {
        if (self.vertical_spacing - spacing).abs() > f32::EPSILON {
            self.vertical_spacing = spacing;
            self.base.invalidate();
        }
    }

    /// Get the alignment for items in incomplete rows.
    #[inline]
    pub fn alignment(&self) -> Alignment {
        self.alignment
    }

    /// Set the alignment for items in incomplete rows.
    ///
    /// - `Start`: Items align to the left (default)
    /// - `Center`: Items are centered
    /// - `End`: Items align to the right
    /// - `Stretch`: Items are spaced evenly
    pub fn set_alignment(&mut self, alignment: Alignment) {
        if self.alignment != alignment {
            self.alignment = alignment;
            self.base.invalidate();
        }
    }

    /// Perform the layout calculation, optionally just measuring.
    ///
    /// This is the core flow algorithm that positions items left-to-right,
    /// wrapping to new rows when the width is exceeded.
    ///
    /// # Arguments
    /// * `storage` - Access to widgets for size hints
    /// * `rect` - The bounding rectangle for layout
    /// * `test_only` - If true, only calculate height without storing positions
    ///
    /// # Returns
    /// The height required for the layout at the given width.
    fn do_layout<S: WidgetAccess>(
        &mut self,
        storage: &S,
        rect: Rect,
        test_only: bool,
    ) -> f32 {
        let content_x = rect.origin.x;
        let content_y = rect.origin.y;
        let content_width = rect.width();

        if content_width <= 0.0 {
            return 0.0;
        }

        let mut x = 0.0;
        let mut y = 0.0;
        let mut row_height: f32 = 0.0;
        let mut row_items: Vec<(usize, f32, f32, f32)> = Vec::new(); // (index, x_pos, width, height)

        // Collect visible items with their sizes
        let mut visible_items: Vec<(usize, Size)> = Vec::new();
        for (i, item) in self.base.items().iter().enumerate() {
            if !self.base.is_item_visible(storage, item) {
                continue;
            }
            let hint = self.base.get_item_size_hint(storage, item);
            visible_items.push((i, hint.preferred));
        }

        if visible_items.is_empty() {
            return 0.0;
        }

        // Helper to finalize a row
        let finalize_row = |items: &mut Vec<(usize, f32, f32, f32)>,
                            row_height: f32,
                            row_y: f32,
                            content_x: f32,
                            content_width: f32,
                            alignment: Alignment,
                            horizontal_spacing: f32,
                            base: &mut LayoutBase,
                            test_only: bool,
                            is_rtl: bool| {
            if items.is_empty() || test_only {
                return;
            }

            // For RTL, reverse the order of items in the row
            if is_rtl {
                items.reverse();
            }

            // Calculate total row width
            let total_width: f32 = items.iter().map(|(_, _, w, _)| *w).sum::<f32>()
                + (items.len().saturating_sub(1) as f32) * horizontal_spacing;

            // Calculate offset based on alignment (swap Start/End for RTL)
            let effective_alignment = if is_rtl {
                match alignment {
                    Alignment::Start => Alignment::End,
                    Alignment::End => Alignment::Start,
                    other => other,
                }
            } else {
                alignment
            };

            let offset = match effective_alignment {
                Alignment::Start => 0.0,
                Alignment::Center => (content_width - total_width) / 2.0,
                Alignment::End => content_width - total_width,
                Alignment::Stretch => 0.0, // Will distribute space
            };

            // Calculate per-item extra space for stretch alignment
            let extra_per_item = if alignment == Alignment::Stretch && items.len() > 1 {
                (content_width - total_width) / (items.len() - 1) as f32
            } else {
                0.0
            };

            // Apply positions
            let mut current_x = offset;
            for (idx, (item_idx, _, item_width, item_height)) in items.iter().enumerate() {
                let item_y = row_y + (row_height - item_height) / 2.0; // Center vertically in row
                let rect = Rect::new(
                    content_x + current_x,
                    item_y,
                    *item_width,
                    *item_height,
                );
                base.set_item_geometry(*item_idx, rect);

                current_x += item_width;
                if idx < items.len() - 1 {
                    current_x += horizontal_spacing + extra_per_item;
                }
            }
        };

        // Layout items with wrapping
        for &(item_idx, item_size) in visible_items.iter() {
            let item_width = item_size.width;
            let item_height = item_size.height;

            // Check if we need to wrap to next row
            let next_x = if row_items.is_empty() {
                x + item_width
            } else {
                x + self.horizontal_spacing + item_width
            };

            if !row_items.is_empty() && next_x > content_width {
                // Finalize current row
                let is_rtl = self.base.is_rtl();
                finalize_row(
                    &mut row_items,
                    row_height,
                    content_y + y,
                    content_x,
                    content_width,
                    self.alignment,
                    self.horizontal_spacing,
                    &mut self.base,
                    test_only,
                    is_rtl,
                );

                // Start new row
                y += row_height + self.vertical_spacing;
                x = 0.0;
                row_height = 0.0;
                row_items.clear();
            }

            // Add item to current row
            row_items.push((item_idx, x, item_width, item_height));
            row_height = row_height.max(item_height);

            x = if row_items.len() == 1 {
                item_width
            } else {
                x + self.horizontal_spacing + item_width
            };
        }

        // Finalize last row
        let is_rtl = self.base.is_rtl();
        finalize_row(
            &mut row_items,
            row_height,
            content_y + y,
            content_x,
            content_width,
            self.alignment,
            self.horizontal_spacing,
            &mut self.base,
            test_only,
            is_rtl,
        );

        // Return total height
        y + row_height
    }

    /// Calculate the minimum size (based on largest single item).
    fn calculate_minimum_size<S: WidgetAccess>(&self, storage: &S) -> Size {
        let mut max_width: f32 = 0.0;
        let mut max_height: f32 = 0.0;

        for item in self.base.items() {
            if !self.base.is_item_visible(storage, item) {
                continue;
            }
            let hint = self.base.get_item_size_hint(storage, item);
            let min = hint.effective_minimum();
            max_width = max_width.max(min.width);
            max_height = max_height.max(min.height);
        }

        let margins = self.base.content_margins();
        Size::new(
            max_width + margins.horizontal(),
            max_height + margins.vertical(),
        )
    }

    /// Calculate the preferred size (all items in one row).
    fn calculate_preferred_size<S: WidgetAccess>(&self, storage: &S) -> Size {
        let mut total_width: f32 = 0.0;
        let mut max_height: f32 = 0.0;
        let mut visible_count = 0;

        for item in self.base.items() {
            if !self.base.is_item_visible(storage, item) {
                continue;
            }
            let hint = self.base.get_item_size_hint(storage, item);
            total_width += hint.preferred.width;
            max_height = max_height.max(hint.preferred.height);
            visible_count += 1;
        }

        // Add spacing between items
        if visible_count > 1 {
            total_width += self.horizontal_spacing * (visible_count - 1) as f32;
        }

        let margins = self.base.content_margins();
        Size::new(
            total_width + margins.horizontal(),
            max_height + margins.vertical(),
        )
    }
}

impl Layout for FlowLayout {
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

        let preferred = self.calculate_preferred_size(storage);
        let minimum = self.calculate_minimum_size(storage);

        SizeHint {
            preferred,
            minimum: Some(minimum),
            maximum: None, // Flow layouts can be any size
        }
    }

    fn minimum_size<S: WidgetAccess>(&self, storage: &S) -> Size {
        if let Some(cached) = self.base.cached_minimum_size() {
            return cached;
        }
        self.calculate_minimum_size(storage)
    }

    fn size_policy(&self) -> SizePolicyPair {
        // Flow layouts prefer to fill horizontal space and adjust height
        SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Preferred)
            .with_height_for_width()
    }

    fn has_height_for_width(&self) -> bool {
        true
    }

    fn height_for_width<S: WidgetAccess>(&self, storage: &S, width: f32) -> Option<f32> {
        let margins = self.base.content_margins();
        let content_width = (width - margins.horizontal()).max(0.0);

        // Use a temporary mutable copy for measurement
        let mut temp = self.clone();
        let content_height = temp.do_layout(
            storage,
            Rect::new(0.0, 0.0, content_width, f32::MAX),
            true, // test_only
        );

        Some(content_height + margins.vertical())
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
        self.horizontal_spacing
    }

    fn set_spacing(&mut self, spacing: f32) {
        self.set_horizontal_spacing(spacing);
    }

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    fn calculate<S: WidgetAccess>(&mut self, storage: &S, available: Size) -> Size {
        let content_rect = self.base.content_rect();

        // Perform the actual layout
        let content_height = self.do_layout(storage, content_rect, false);

        // Cache the calculated size hint
        let size_hint = SizeHint {
            preferred: self.calculate_preferred_size(storage),
            minimum: Some(self.calculate_minimum_size(storage)),
            maximum: None,
        };
        self.base.set_cached_size_hint(size_hint);
        self.base.set_cached_minimum_size(self.calculate_minimum_size(storage));

        self.base.mark_valid();

        // Return the actual size used
        let margins = self.base.content_margins();
        Size::new(
            available.width,
            content_height + margins.vertical(),
        )
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

impl Default for FlowLayout {
    fn default() -> Self {
        Self::new()
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
    use horizon_lattice_core::{init_global_registry, Object, ObjectId};
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

    #[test]
    fn test_flow_layout_creation() {
        init_global_registry();

        let layout = FlowLayout::new();
        assert_eq!(layout.item_count(), 0);
        assert_eq!(layout.horizontal_spacing(), super::super::DEFAULT_SPACING);
        assert_eq!(layout.vertical_spacing(), super::super::DEFAULT_SPACING);
        assert_eq!(layout.alignment(), Alignment::Start);
    }

    #[test]
    fn test_flow_layout_spacing() {
        init_global_registry();

        let mut layout = FlowLayout::new();
        layout.set_horizontal_spacing(10.0);
        layout.set_vertical_spacing(5.0);

        assert_eq!(layout.horizontal_spacing(), 10.0);
        assert_eq!(layout.vertical_spacing(), 5.0);
    }

    #[test]
    fn test_flow_layout_single_row() {
        init_global_registry();

        let mut storage = MockStorage::new();

        // Three 50px wide widgets should fit in 200px without wrapping
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));
        let id3 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));

        let mut layout = FlowLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_horizontal_spacing(10.0);
        layout.set_vertical_spacing(5.0);
        layout.add_widget(id1);
        layout.add_widget(id2);
        layout.add_widget(id3);

        // 200px wide: 50 + 10 + 50 + 10 + 50 = 170px, fits in one row
        layout.set_geometry(Rect::new(0.0, 0.0, 200.0, 100.0));
        layout.calculate(&storage, Size::new(200.0, 100.0));
        layout.apply(&mut storage);

        // All widgets should be on the same row (y=0)
        let w1 = storage.widgets.get(&id1).unwrap();
        let w2 = storage.widgets.get(&id2).unwrap();
        let w3 = storage.widgets.get(&id3).unwrap();

        assert_eq!(w1.geometry().origin.y, 0.0);
        assert_eq!(w2.geometry().origin.y, 0.0);
        assert_eq!(w3.geometry().origin.y, 0.0);

        // Check horizontal positions
        assert_eq!(w1.geometry().origin.x, 0.0);
        assert_eq!(w2.geometry().origin.x, 60.0); // 50 + 10
        assert_eq!(w3.geometry().origin.x, 120.0); // 50 + 10 + 50 + 10
    }

    #[test]
    fn test_flow_layout_wrapping() {
        init_global_registry();

        let mut storage = MockStorage::new();

        // Four 50px widgets won't fit in 150px, should wrap
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));
        let id3 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));
        let id4 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));

        let mut layout = FlowLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_horizontal_spacing(10.0);
        layout.set_vertical_spacing(5.0);
        layout.add_widget(id1);
        layout.add_widget(id2);
        layout.add_widget(id3);
        layout.add_widget(id4);

        // 150px wide: 50 + 10 + 50 = 110px fits 2, then wrap
        layout.set_geometry(Rect::new(0.0, 0.0, 150.0, 100.0));
        layout.calculate(&storage, Size::new(150.0, 100.0));
        layout.apply(&mut storage);

        let w1 = storage.widgets.get(&id1).unwrap();
        let w2 = storage.widgets.get(&id2).unwrap();
        let w3 = storage.widgets.get(&id3).unwrap();
        let w4 = storage.widgets.get(&id4).unwrap();

        // First row
        assert_eq!(w1.geometry().origin.y, 0.0);
        assert_eq!(w2.geometry().origin.y, 0.0);

        // Second row (30 + 5 = 35)
        assert_eq!(w3.geometry().origin.y, 35.0);
        assert_eq!(w4.geometry().origin.y, 35.0);
    }

    #[test]
    fn test_flow_layout_height_for_width() {
        init_global_registry();

        let mut storage = MockStorage::new();

        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));
        let id3 = storage.add(MockWidget::new(SizeHint::new(Size::new(100.0, 30.0))));

        let mut layout = FlowLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_horizontal_spacing(10.0);
        layout.set_vertical_spacing(5.0);
        layout.add_widget(id1);
        layout.add_widget(id2);
        layout.add_widget(id3);

        // Wide enough for all in one row
        let h1 = layout.height_for_width(&storage, 500.0).unwrap();
        assert!((h1 - 30.0).abs() < 0.01); // One row height

        // Narrow - forces wrapping (each on its own row)
        let h2 = layout.height_for_width(&storage, 120.0).unwrap();
        // 3 rows: 30 + 5 + 30 + 5 + 30 = 100
        assert!((h2 - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_flow_layout_alignment() {
        init_global_registry();

        let mut storage = MockStorage::new();

        // Two items that don't fill the row
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(40.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(40.0, 30.0))));

        let mut layout = FlowLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.set_horizontal_spacing(10.0);
        layout.set_alignment(Alignment::Center);
        layout.add_widget(id1);
        layout.add_widget(id2);

        // Total width used: 40 + 10 + 40 = 90
        // Available: 200
        // Centered offset: (200 - 90) / 2 = 55
        layout.set_geometry(Rect::new(0.0, 0.0, 200.0, 100.0));
        layout.calculate(&storage, Size::new(200.0, 100.0));
        layout.apply(&mut storage);

        let w1 = storage.widgets.get(&id1).unwrap();
        assert!((w1.geometry().origin.x - 55.0).abs() < 0.01);
    }

    #[test]
    fn test_flow_layout_has_height_for_width() {
        init_global_registry();

        let layout = FlowLayout::new();
        assert!(layout.has_height_for_width());
    }

    #[test]
    fn test_flow_layout_minimum_size() {
        init_global_registry();

        let mut storage = MockStorage::new();

        let id1 = storage.add(MockWidget::new(
            SizeHint::new(Size::new(100.0, 30.0)).with_minimum(Size::new(50.0, 20.0)),
        ));
        let id2 = storage.add(MockWidget::new(
            SizeHint::new(Size::new(80.0, 40.0)).with_minimum(Size::new(40.0, 25.0)),
        ));

        let mut layout = FlowLayout::new();
        layout.set_content_margins(ContentMargins::uniform(10.0));
        layout.add_widget(id1);
        layout.add_widget(id2);

        let min = layout.minimum_size(&storage);
        // Minimum width = max single item minimum (50) + margins (20) = 70
        assert!((min.width - 70.0).abs() < 0.01);
        // Minimum height = max single item minimum (25) + margins (20) = 45
        assert!((min.height - 45.0).abs() < 0.01);
    }
}
