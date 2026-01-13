//! Common layout implementation.
//!
//! LayoutBase provides shared functionality for all layout implementations.

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

use super::{ContentMargins, DEFAULT_MARGINS, DEFAULT_SPACING};
use super::item::LayoutItem;
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::geometry::{SizeHint, SizePolicy, SizePolicyPair};

/// Common base for layout implementations.
///
/// This struct provides shared state and functionality used by all layout types:
/// - Geometry and margins
/// - Item storage
/// - Invalidation tracking
/// - Parent widget reference
///
/// Layout implementations typically include this as a field and delegate
/// common operations to it.
#[derive(Debug, Clone)]
pub struct LayoutBase {
    /// Items managed by the layout.
    items: Vec<LayoutItem>,

    /// Calculated geometries for each item (same indices as items).
    item_geometries: Vec<Rect>,

    /// The layout's geometry (position and size).
    geometry: Rect,

    /// Content margins around the layout.
    content_margins: ContentMargins,

    /// Spacing between items.
    spacing: f32,

    /// Whether the layout needs recalculation.
    dirty: bool,

    /// The parent widget that owns this layout.
    parent_widget: Option<ObjectId>,

    /// Cached size hint (invalidated when dirty).
    cached_size_hint: Option<SizeHint>,

    /// Cached minimum size.
    cached_minimum_size: Option<Size>,
}

impl LayoutBase {
    /// Create a new layout base with default settings.
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            item_geometries: Vec::new(),
            geometry: Rect::ZERO,
            content_margins: DEFAULT_MARGINS,
            spacing: DEFAULT_SPACING,
            dirty: true,
            parent_widget: None,
            cached_size_hint: None,
            cached_minimum_size: None,
        }
    }

    // =========================================================================
    // Item Management
    // =========================================================================

    /// Add an item to the layout.
    pub fn add_item(&mut self, item: LayoutItem) {
        self.items.push(item);
        self.item_geometries.push(Rect::ZERO);
        self.invalidate();
    }

    /// Insert an item at a specific index.
    pub fn insert_item(&mut self, index: usize, item: LayoutItem) {
        self.items.insert(index, item);
        self.item_geometries.insert(index, Rect::ZERO);
        self.invalidate();
    }

    /// Remove an item at the specified index.
    pub fn remove_item(&mut self, index: usize) -> Option<LayoutItem> {
        if index < self.items.len() {
            self.item_geometries.remove(index);
            let item = self.items.remove(index);
            self.invalidate();
            Some(item)
        } else {
            None
        }
    }

    /// Remove a widget by its ObjectId.
    pub fn remove_widget(&mut self, widget: ObjectId) -> bool {
        if let Some(index) = self.items.iter().position(|item| {
            matches!(item, LayoutItem::Widget(id) if *id == widget)
        }) {
            self.remove_item(index);
            true
        } else {
            false
        }
    }

    /// Get the number of items.
    #[inline]
    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Get an item by index.
    #[inline]
    pub fn item_at(&self, index: usize) -> Option<&LayoutItem> {
        self.items.get(index)
    }

    /// Get a mutable reference to an item.
    #[inline]
    pub fn item_at_mut(&mut self, index: usize) -> Option<&mut LayoutItem> {
        self.invalidate();
        self.items.get_mut(index)
    }

    /// Get all items.
    #[inline]
    pub fn items(&self) -> &[LayoutItem] {
        &self.items
    }

    /// Clear all items.
    pub fn clear(&mut self) {
        self.items.clear();
        self.item_geometries.clear();
        self.invalidate();
    }

    /// Check if empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    // =========================================================================
    // Geometry & Margins
    // =========================================================================

    /// Get the layout's geometry.
    #[inline]
    pub fn geometry(&self) -> Rect {
        self.geometry
    }

    /// Set the layout's geometry.
    pub fn set_geometry(&mut self, rect: Rect) {
        if self.geometry != rect {
            self.geometry = rect;
            self.invalidate();
        }
    }

    /// Get content margins.
    #[inline]
    pub fn content_margins(&self) -> ContentMargins {
        self.content_margins
    }

    /// Set content margins.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        if self.content_margins != margins {
            self.content_margins = margins;
            self.invalidate();
        }
    }

    /// Get spacing.
    #[inline]
    pub fn spacing(&self) -> f32 {
        self.spacing
    }

    /// Set spacing.
    pub fn set_spacing(&mut self, spacing: f32) {
        if (self.spacing - spacing).abs() > f32::EPSILON {
            self.spacing = spacing;
            self.invalidate();
        }
    }

    /// Get the content area (geometry minus margins).
    pub fn content_rect(&self) -> Rect {
        Rect::new(
            self.geometry.origin.x + self.content_margins.left,
            self.geometry.origin.y + self.content_margins.top,
            (self.geometry.width() - self.content_margins.horizontal()).max(0.0),
            (self.geometry.height() - self.content_margins.vertical()).max(0.0),
        )
    }

    // =========================================================================
    // Calculated Geometries
    // =========================================================================

    /// Get the calculated geometry for an item.
    #[inline]
    pub fn item_geometry(&self, index: usize) -> Option<Rect> {
        self.item_geometries.get(index).copied()
    }

    /// Set the calculated geometry for an item.
    pub fn set_item_geometry(&mut self, index: usize, rect: Rect) {
        if index < self.item_geometries.len() {
            self.item_geometries[index] = rect;
        }
    }

    /// Get all item geometries.
    #[inline]
    pub fn item_geometries(&self) -> &[Rect] {
        &self.item_geometries
    }

    // =========================================================================
    // Invalidation
    // =========================================================================

    /// Invalidate the layout.
    pub fn invalidate(&mut self) {
        self.dirty = true;
        self.cached_size_hint = None;
        self.cached_minimum_size = None;
    }

    /// Check if the layout needs recalculation.
    #[inline]
    pub fn needs_recalculation(&self) -> bool {
        self.dirty
    }

    /// Mark the layout as valid (after recalculation).
    pub fn mark_valid(&mut self) {
        self.dirty = false;
    }

    // =========================================================================
    // Parent Widget
    // =========================================================================

    /// Get the parent widget.
    #[inline]
    pub fn parent_widget(&self) -> Option<ObjectId> {
        self.parent_widget
    }

    /// Set the parent widget.
    pub fn set_parent_widget(&mut self, parent: Option<ObjectId>) {
        self.parent_widget = parent;
    }

    // =========================================================================
    // Cache Management
    // =========================================================================

    /// Get cached size hint.
    #[inline]
    pub fn cached_size_hint(&self) -> Option<SizeHint> {
        self.cached_size_hint
    }

    /// Set cached size hint.
    pub fn set_cached_size_hint(&mut self, hint: SizeHint) {
        self.cached_size_hint = Some(hint);
    }

    /// Get cached minimum size.
    #[inline]
    pub fn cached_minimum_size(&self) -> Option<Size> {
        self.cached_minimum_size
    }

    /// Set cached minimum size.
    pub fn set_cached_minimum_size(&mut self, size: Size) {
        self.cached_minimum_size = Some(size);
    }

    // =========================================================================
    // Helpers for Layout Algorithms
    // =========================================================================

    /// Get the size hint for a layout item.
    pub fn get_item_size_hint<S: WidgetAccess>(&self, storage: &S, item: &LayoutItem) -> SizeHint {
        match item {
            LayoutItem::Widget(id) => {
                if let Some(widget) = storage.get_widget(*id) {
                    // Skip hidden widgets
                    if !widget.is_visible() {
                        return SizeHint::fixed(Size::ZERO);
                    }
                    widget.size_hint()
                } else {
                    SizeHint::default()
                }
            }
            LayoutItem::Spacer(spacer) => spacer.size_hint(),
            LayoutItem::Layout(layout) => layout.size_hint(),
        }
    }

    /// Get the size policy for a layout item.
    pub fn get_item_size_policy<S: WidgetAccess>(
        &self,
        storage: &S,
        item: &LayoutItem,
    ) -> SizePolicyPair {
        match item {
            LayoutItem::Widget(id) => {
                if let Some(widget) = storage.get_widget(*id) {
                    widget.size_policy()
                } else {
                    SizePolicyPair::default()
                }
            }
            LayoutItem::Spacer(spacer) => spacer.size_policy(),
            LayoutItem::Layout(layout) => layout.size_policy(),
        }
    }

    /// Check if an item is visible (for widgets) or non-empty (for others).
    pub fn is_item_visible<S: WidgetAccess>(&self, storage: &S, item: &LayoutItem) -> bool {
        match item {
            LayoutItem::Widget(id) => storage
                .get_widget(*id)
                .is_some_and(|w| w.is_visible()),
            LayoutItem::Spacer(_) => true,
            LayoutItem::Layout(layout) => !layout.is_empty(),
        }
    }

    /// Count visible items.
    pub fn visible_item_count<S: WidgetAccess>(&self, storage: &S) -> usize {
        self.items
            .iter()
            .filter(|item| self.is_item_visible(storage, item))
            .count()
    }

    /// Apply geometry to a widget item.
    pub fn apply_item_geometry<S: WidgetAccess>(
        storage: &mut S,
        item: &LayoutItem,
        geometry: Rect,
    ) {
        if let LayoutItem::Widget(id) = item {
            if let Some(widget) = storage.get_widget_mut(*id) {
                widget.set_geometry(geometry);
            }
        }
    }

    /// Distribute space among items based on their policies and stretch factors.
    ///
    /// This is a helper for box layouts to distribute extra or deficit space.
    ///
    /// # Arguments
    /// * `items` - Information about each item (size hint, policy, stretch)
    /// * `available` - Total available space
    /// * `total_hint` - Sum of all preferred sizes
    /// * `total_min` - Sum of all minimum sizes
    /// * `total_max` - Sum of all maximum sizes
    ///
    /// # Returns
    /// A vector of sizes to assign to each item.
    pub fn distribute_space(
        items: &[(SizeHint, SizePolicy, u8)], // (hint, policy, stretch)
        available: f32,
        total_hint: f32,
        total_min: f32,
        total_max: f32,
    ) -> Vec<f32> {
        let n = items.len();
        if n == 0 {
            return Vec::new();
        }

        // Start with preferred sizes
        let mut sizes: Vec<f32> = items
            .iter()
            .map(|(hint, _, _)| hint.preferred.width.max(hint.preferred.height))
            .collect();

        let extra = available - total_hint;

        if extra > 0.0 {
            // We have extra space to distribute
            distribute_extra_space(&mut sizes, items, extra, total_max - total_hint);
        } else if extra < 0.0 {
            // We need to shrink items
            distribute_deficit_space(&mut sizes, items, -extra, total_hint - total_min);
        }

        sizes
    }
}

impl Default for LayoutBase {
    fn default() -> Self {
        Self::new()
    }
}

/// Distribute extra space among items that can grow.
fn distribute_extra_space(
    sizes: &mut [f32],
    items: &[(SizeHint, SizePolicy, u8)],
    mut extra: f32,
    max_growth: f32,
) {
    if extra <= 0.0 || max_growth <= 0.0 {
        return;
    }

    // Clamp extra to maximum possible growth
    extra = extra.min(max_growth);

    // First, identify which items can grow and their stretch factors
    let mut growable: Vec<(usize, u8, f32)> = Vec::new(); // (index, stretch, max_growth)

    for (i, (hint, policy, stretch)) in items.iter().enumerate() {
        if policy.can_grow() {
            let item_max = hint.effective_maximum().width.max(hint.effective_maximum().height);
            let current = hint.preferred.width.max(hint.preferred.height);
            let growth_room = (item_max - current).max(0.0);
            if growth_room > 0.0 {
                growable.push((i, *stretch, growth_room));
            }
        }
    }

    if growable.is_empty() {
        return;
    }

    // Calculate total stretch factor (0 means equal distribution)
    let total_stretch: u32 = growable.iter().map(|(_, s, _)| *s as u32).sum();

    if total_stretch == 0 {
        // Equal distribution among growable items
        let per_item = extra / growable.len() as f32;
        for (idx, _, max_growth) in &growable {
            sizes[*idx] += per_item.min(*max_growth);
        }
    } else {
        // Distribute by stretch factor
        for (idx, stretch, max_growth) in &growable {
            let share = extra * (*stretch as f32 / total_stretch as f32);
            sizes[*idx] += share.min(*max_growth);
        }
    }
}

/// Distribute deficit (shrink items that can shrink).
fn distribute_deficit_space(
    sizes: &mut [f32],
    items: &[(SizeHint, SizePolicy, u8)],
    mut deficit: f32,
    max_shrink: f32,
) {
    if deficit <= 0.0 || max_shrink <= 0.0 {
        return;
    }

    // Clamp deficit to maximum possible shrinkage
    deficit = deficit.min(max_shrink);

    // Identify which items can shrink and by how much
    let mut shrinkable: Vec<(usize, f32)> = Vec::new(); // (index, shrink_room)

    for (i, (hint, policy, _)) in items.iter().enumerate() {
        if policy.can_shrink() {
            let item_min = hint.effective_minimum().width.max(hint.effective_minimum().height);
            let current = hint.preferred.width.max(hint.preferred.height);
            let shrink_room = (current - item_min).max(0.0);
            if shrink_room > 0.0 {
                shrinkable.push((i, shrink_room));
            }
        }
    }

    if shrinkable.is_empty() {
        return;
    }

    // Distribute deficit proportionally to shrink room
    let total_shrink_room: f32 = shrinkable.iter().map(|(_, r)| *r).sum();
    for (idx, shrink_room) in &shrinkable {
        let share = deficit * (*shrink_room / total_shrink_room);
        sizes[*idx] -= share;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_base_creation() {
        let base = LayoutBase::new();
        assert!(base.is_empty());
        assert!(base.needs_recalculation());
        assert_eq!(base.spacing(), DEFAULT_SPACING);
    }

    #[test]
    fn test_layout_base_margins() {
        let mut base = LayoutBase::new();
        base.set_content_margins(ContentMargins::uniform(10.0));
        assert_eq!(base.content_margins().left, 10.0);

        base.set_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
        let content = base.content_rect();
        assert_eq!(content.width(), 80.0);
        assert_eq!(content.height(), 80.0);
    }

    #[test]
    fn test_distribute_space_extra() {
        // Two items with equal stretch, both can grow
        let items = vec![
            (
                SizeHint::new(Size::new(50.0, 20.0)),
                SizePolicy::Expanding,
                1,
            ),
            (
                SizeHint::new(Size::new(50.0, 20.0)),
                SizePolicy::Expanding,
                1,
            ),
        ];

        let sizes = LayoutBase::distribute_space(
            &items,
            200.0, // available
            100.0, // total_hint
            0.0,   // total_min
            400.0, // total_max
        );

        assert_eq!(sizes.len(), 2);
        // Extra 100 split equally
        assert!((sizes[0] - 100.0).abs() < 0.01);
        assert!((sizes[1] - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_distribute_space_deficit() {
        // Two items that can shrink
        let items = vec![
            (
                SizeHint::new(Size::new(100.0, 20.0)).with_minimum(Size::new(50.0, 20.0)),
                SizePolicy::Preferred,
                0,
            ),
            (
                SizeHint::new(Size::new(100.0, 20.0)).with_minimum(Size::new(50.0, 20.0)),
                SizePolicy::Preferred,
                0,
            ),
        ];

        let sizes = LayoutBase::distribute_space(
            &items,
            100.0, // available (need 200, only have 100)
            200.0, // total_hint
            100.0, // total_min
            400.0, // total_max
        );

        assert_eq!(sizes.len(), 2);
        // Each should shrink to minimum
        assert!((sizes[0] - 50.0).abs() < 0.01);
        assert!((sizes[1] - 50.0).abs() < 0.01);
    }
}
