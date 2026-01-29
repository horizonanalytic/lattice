//! Anchor-based layout for constraint-based widget positioning.
//!
//! `AnchorLayout` allows positioning widgets relative to parent edges or sibling
//! edges using anchor constraints. This provides flexible, responsive layouts
//! where widgets maintain relationships as the container resizes.
//!
//! # Anchor Lines
//!
//! Each item has 6 anchor lines:
//! - **Left**, **Right**: Horizontal edges
//! - **Top**, **Bottom**: Vertical edges
//! - **HorizontalCenter**, **VerticalCenter**: Center points
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::layout::*;
//!
//! let mut layout = AnchorLayout::new();
//!
//! // Add widget anchored to fill parent
//! layout.add_widget(widget_id);
//! layout.anchor_to_parent(0, AnchorLine::Left, AnchorLine::Left, 10.0);
//! layout.anchor_to_parent(0, AnchorLine::Right, AnchorLine::Right, 10.0);
//! layout.anchor_to_parent(0, AnchorLine::Top, AnchorLine::Top, 10.0);
//! layout.anchor_to_parent(0, AnchorLine::Bottom, AnchorLine::Bottom, 10.0);
//!
//! // Add widget centered in parent
//! layout.add_widget(centered_widget_id);
//! layout.anchor_to_parent(1, AnchorLine::HorizontalCenter, AnchorLine::HorizontalCenter, 0.0);
//! layout.anchor_to_parent(1, AnchorLine::VerticalCenter, AnchorLine::VerticalCenter, 0.0);
//!
//! // Add widget anchored to sibling
//! layout.add_widget(sibling_widget_id);
//! layout.anchor_to_sibling(2, AnchorLine::Left, 0, AnchorLine::Right, 8.0);
//! ```

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

use super::ContentMargins;
use super::base::LayoutBase;
use super::item::LayoutItem;
use super::traits::Layout;
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::geometry::{SizeHint, SizePolicyPair};

/// Anchor lines available on each layout item.
///
/// These represent the edges and center points that can be used
/// to create anchor relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnchorLine {
    /// Left edge.
    Left,
    /// Right edge.
    Right,
    /// Top edge.
    Top,
    /// Bottom edge.
    Bottom,
    /// Horizontal center point.
    HorizontalCenter,
    /// Vertical center point.
    VerticalCenter,
}

impl AnchorLine {
    /// Check if this is a horizontal anchor line.
    #[inline]
    pub fn is_horizontal(&self) -> bool {
        matches!(
            self,
            AnchorLine::Left | AnchorLine::Right | AnchorLine::HorizontalCenter
        )
    }

    /// Check if this is a vertical anchor line.
    #[inline]
    pub fn is_vertical(&self) -> bool {
        !self.is_horizontal()
    }

    /// Get the opposite anchor line (Left<->Right, Top<->Bottom).
    /// Center lines return themselves.
    pub fn opposite(&self) -> Self {
        match self {
            AnchorLine::Left => AnchorLine::Right,
            AnchorLine::Right => AnchorLine::Left,
            AnchorLine::Top => AnchorLine::Bottom,
            AnchorLine::Bottom => AnchorLine::Top,
            AnchorLine::HorizontalCenter => AnchorLine::HorizontalCenter,
            AnchorLine::VerticalCenter => AnchorLine::VerticalCenter,
        }
    }
}

/// Target for an anchor constraint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnchorTarget {
    /// Anchor to the parent layout's edge.
    Parent(AnchorLine),
    /// Anchor to a sibling item's edge (by item index).
    Sibling {
        /// Index of the sibling item in the layout.
        index: usize,
        /// Which anchor line of the sibling to attach to.
        line: AnchorLine,
    },
}

/// An anchor constraint binding a source line to a target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    /// The anchor line on the source item.
    pub source_line: AnchorLine,
    /// The target to anchor to.
    pub target: AnchorTarget,
    /// Margin/offset from the target position.
    /// Positive values move inward (for edge anchors) or offset (for center anchors).
    pub margin: f32,
}

impl Anchor {
    /// Create a new anchor to parent.
    pub fn to_parent(source: AnchorLine, target: AnchorLine, margin: f32) -> Self {
        Self {
            source_line: source,
            target: AnchorTarget::Parent(target),
            margin,
        }
    }

    /// Create a new anchor to a sibling.
    pub fn to_sibling(
        source: AnchorLine,
        sibling_index: usize,
        target: AnchorLine,
        margin: f32,
    ) -> Self {
        Self {
            source_line: source,
            target: AnchorTarget::Sibling {
                index: sibling_index,
                line: target,
            },
            margin,
        }
    }
}

/// Resolved anchor values for an item during calculation.
#[derive(Debug, Clone, Copy, Default)]
struct ResolvedAnchors {
    left: Option<f32>,
    right: Option<f32>,
    top: Option<f32>,
    bottom: Option<f32>,
    h_center: Option<f32>,
    v_center: Option<f32>,
}

impl ResolvedAnchors {
    fn get(&self, line: AnchorLine) -> Option<f32> {
        match line {
            AnchorLine::Left => self.left,
            AnchorLine::Right => self.right,
            AnchorLine::Top => self.top,
            AnchorLine::Bottom => self.bottom,
            AnchorLine::HorizontalCenter => self.h_center,
            AnchorLine::VerticalCenter => self.v_center,
        }
    }

    fn set(&mut self, line: AnchorLine, value: f32) {
        match line {
            AnchorLine::Left => self.left = Some(value),
            AnchorLine::Right => self.right = Some(value),
            AnchorLine::Top => self.top = Some(value),
            AnchorLine::Bottom => self.bottom = Some(value),
            AnchorLine::HorizontalCenter => self.h_center = Some(value),
            AnchorLine::VerticalCenter => self.v_center = Some(value),
        }
    }
}

/// A constraint-based layout that positions items using anchor relationships.
///
/// Each item can have multiple anchors that constrain its position relative to
/// the parent layout edges or sibling items. The layout solver resolves these
/// constraints to determine final positions.
///
/// # Anchor Resolution Rules
///
/// - If both opposing edges are anchored (left+right or top+bottom), the item
///   is resized to fit between them.
/// - If only one edge is anchored, the item keeps its preferred size and is
///   positioned at that edge.
/// - Center anchors position the item's center point; size comes from hints.
/// - Margins push the item inward from the anchor target.
///
/// # Anchor Compatibility
///
/// - Horizontal anchors (Left, Right, HorizontalCenter) must anchor to horizontal targets.
/// - Vertical anchors (Top, Bottom, VerticalCenter) must anchor to vertical targets.
/// - Mixing horizontal/vertical anchors in a single constraint is not allowed.
#[derive(Debug, Clone)]
pub struct AnchorLayout {
    /// Common layout functionality.
    base: LayoutBase,
    /// Anchors for each item (indices match base.items()).
    item_anchors: Vec<Vec<Anchor>>,
}

impl AnchorLayout {
    /// Create a new empty anchor layout.
    pub fn new() -> Self {
        Self {
            base: LayoutBase::new(),
            item_anchors: Vec::new(),
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

    /// Add an anchor constraint to an item.
    ///
    /// The item must already exist in the layout (added via `add_item` or `add_widget`).
    /// Returns `true` if the anchor was added, `false` if the item index is invalid
    /// or the anchor lines are incompatible (mixing horizontal/vertical).
    pub fn add_anchor(&mut self, item_index: usize, anchor: Anchor) -> bool {
        if item_index >= self.item_anchors.len() {
            return false;
        }

        // Validate anchor line compatibility
        let target_line = match anchor.target {
            AnchorTarget::Parent(line) => line,
            AnchorTarget::Sibling { line, .. } => line,
        };

        if anchor.source_line.is_horizontal() != target_line.is_horizontal() {
            return false;
        }

        // Check for duplicate anchor on same source line
        let anchors = &mut self.item_anchors[item_index];
        if let Some(existing) = anchors
            .iter_mut()
            .find(|a| a.source_line == anchor.source_line)
        {
            *existing = anchor;
        } else {
            anchors.push(anchor);
        }

        self.base.invalidate();
        true
    }

    /// Convenience: anchor an item's line to the parent's line.
    pub fn anchor_to_parent(
        &mut self,
        item_index: usize,
        source: AnchorLine,
        target: AnchorLine,
        margin: f32,
    ) -> bool {
        self.add_anchor(item_index, Anchor::to_parent(source, target, margin))
    }

    /// Convenience: anchor an item's line to a sibling's line.
    pub fn anchor_to_sibling(
        &mut self,
        item_index: usize,
        source: AnchorLine,
        sibling_index: usize,
        target: AnchorLine,
        margin: f32,
    ) -> bool {
        if sibling_index == item_index {
            return false; // Can't anchor to self
        }
        self.add_anchor(
            item_index,
            Anchor::to_sibling(source, sibling_index, target, margin),
        )
    }

    /// Convenience: fill the parent (anchor all four edges).
    pub fn fill_parent(&mut self, item_index: usize, margin: f32) -> bool {
        self.anchor_to_parent(item_index, AnchorLine::Left, AnchorLine::Left, margin)
            && self.anchor_to_parent(item_index, AnchorLine::Right, AnchorLine::Right, margin)
            && self.anchor_to_parent(item_index, AnchorLine::Top, AnchorLine::Top, margin)
            && self.anchor_to_parent(item_index, AnchorLine::Bottom, AnchorLine::Bottom, margin)
    }

    /// Convenience: center in parent.
    pub fn center_in_parent(&mut self, item_index: usize) -> bool {
        self.anchor_to_parent(
            item_index,
            AnchorLine::HorizontalCenter,
            AnchorLine::HorizontalCenter,
            0.0,
        ) && self.anchor_to_parent(
            item_index,
            AnchorLine::VerticalCenter,
            AnchorLine::VerticalCenter,
            0.0,
        )
    }

    /// Remove all anchors from an item.
    pub fn clear_anchors(&mut self, item_index: usize) {
        if item_index < self.item_anchors.len() {
            self.item_anchors[item_index].clear();
            self.base.invalidate();
        }
    }

    /// Get the anchors for an item.
    pub fn anchors(&self, item_index: usize) -> Option<&[Anchor]> {
        self.item_anchors.get(item_index).map(|v| v.as_slice())
    }

    // =========================================================================
    // Private: Anchor Resolution
    // =========================================================================

    /// Get the position of a parent's anchor line.
    fn parent_anchor_position(&self, line: AnchorLine, content_rect: Rect) -> f32 {
        match line {
            AnchorLine::Left => content_rect.origin.x,
            AnchorLine::Right => content_rect.origin.x + content_rect.width(),
            AnchorLine::Top => content_rect.origin.y,
            AnchorLine::Bottom => content_rect.origin.y + content_rect.height(),
            AnchorLine::HorizontalCenter => content_rect.origin.x + content_rect.width() / 2.0,
            AnchorLine::VerticalCenter => content_rect.origin.y + content_rect.height() / 2.0,
        }
    }

    /// Get the position of a resolved item's anchor line.
    fn item_anchor_position(&self, rect: Rect, line: AnchorLine) -> f32 {
        match line {
            AnchorLine::Left => rect.origin.x,
            AnchorLine::Right => rect.origin.x + rect.width(),
            AnchorLine::Top => rect.origin.y,
            AnchorLine::Bottom => rect.origin.y + rect.height(),
            AnchorLine::HorizontalCenter => rect.origin.x + rect.width() / 2.0,
            AnchorLine::VerticalCenter => rect.origin.y + rect.height() / 2.0,
        }
    }

    /// Resolve anchor constraints and calculate item geometries.
    fn resolve_anchors<S: WidgetAccess>(&mut self, storage: &S, content_rect: Rect) {
        let item_count = self.base.item_count();
        if item_count == 0 {
            return;
        }

        // Get preferred sizes for all items
        let mut preferred_sizes: Vec<Size> = Vec::with_capacity(item_count);
        for item in self.base.items() {
            let hint = self.base.get_item_size_hint(storage, item);
            preferred_sizes.push(hint.preferred);
        }

        // Initialize resolved anchors
        let mut resolved: Vec<ResolvedAnchors> = vec![ResolvedAnchors::default(); item_count];

        // Resolve in multiple passes until stable
        // (needed for sibling dependencies)
        const MAX_ITERATIONS: usize = 10;
        for _ in 0..MAX_ITERATIONS {
            let mut changed = false;

            for item_idx in 0..item_count {
                let anchors = &self.item_anchors[item_idx];

                for anchor in anchors {
                    // Get target position
                    let target_pos = match anchor.target {
                        AnchorTarget::Parent(line) => {
                            Some(self.parent_anchor_position(line, content_rect))
                        }
                        AnchorTarget::Sibling { index, line } => {
                            // Can only resolve if sibling is already resolved
                            if let Some(sibling_rect) = self.base.item_geometry(index) {
                                if sibling_rect != Rect::ZERO {
                                    Some(self.item_anchor_position(sibling_rect, line))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                    };

                    if let Some(target_pos) = target_pos {
                        // Apply margin based on anchor direction
                        let source_pos = match anchor.source_line {
                            // For left/top anchors, margin pushes right/down (positive)
                            AnchorLine::Left | AnchorLine::Top => target_pos + anchor.margin,
                            // For right/bottom anchors, margin pushes left/up (negative)
                            AnchorLine::Right | AnchorLine::Bottom => target_pos - anchor.margin,
                            // For center anchors, margin is an offset
                            _ => target_pos + anchor.margin,
                        };

                        let old = resolved[item_idx].get(anchor.source_line);
                        if old != Some(source_pos) {
                            resolved[item_idx].set(anchor.source_line, source_pos);
                            changed = true;
                        }
                    }
                }

                // Calculate geometry from resolved anchors
                let r = &resolved[item_idx];
                let pref = preferred_sizes[item_idx];

                // Calculate horizontal position and width
                let (x, width) = match (r.left, r.right, r.h_center) {
                    (Some(left), Some(right), _) => {
                        // Both edges anchored - resize to fit
                        (left, (right - left).max(0.0))
                    }
                    (Some(left), None, _) => {
                        // Only left anchored
                        (left, pref.width)
                    }
                    (None, Some(right), _) => {
                        // Only right anchored
                        (right - pref.width, pref.width)
                    }
                    (None, None, Some(center)) => {
                        // Center anchored
                        (center - pref.width / 2.0, pref.width)
                    }
                    (None, None, None) => {
                        // No horizontal anchors - use content rect origin
                        (content_rect.origin.x, pref.width)
                    }
                };

                // Calculate vertical position and height
                let (y, height) = match (r.top, r.bottom, r.v_center) {
                    (Some(top), Some(bottom), _) => {
                        // Both edges anchored - resize to fit
                        (top, (bottom - top).max(0.0))
                    }
                    (Some(top), None, _) => {
                        // Only top anchored
                        (top, pref.height)
                    }
                    (None, Some(bottom), _) => {
                        // Only bottom anchored
                        (bottom - pref.height, pref.height)
                    }
                    (None, None, Some(center)) => {
                        // Center anchored
                        (center - pref.height / 2.0, pref.height)
                    }
                    (None, None, None) => {
                        // No vertical anchors - use content rect origin
                        (content_rect.origin.y, pref.height)
                    }
                };

                self.base
                    .set_item_geometry(item_idx, Rect::new(x, y, width, height));
            }

            if !changed {
                break;
            }
        }
    }

    /// Calculate the size hint based on item hints and anchors.
    fn calculate_size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        let mut max_width: f32 = 0.0;
        let mut max_height: f32 = 0.0;

        for (i, item) in self.base.items().iter().enumerate() {
            if !self.base.is_item_visible(storage, item) {
                continue;
            }

            let hint = self.base.get_item_size_hint(storage, item);
            let anchors = &self.item_anchors[i];

            // For each item, calculate the minimum space it needs based on anchors
            let mut item_width = hint.preferred.width;
            let mut item_height = hint.preferred.height;

            // Add margins from anchors
            for anchor in anchors {
                match anchor.target {
                    AnchorTarget::Parent(_) => match anchor.source_line {
                        AnchorLine::Left | AnchorLine::Right => {
                            item_width += anchor.margin.abs();
                        }
                        AnchorLine::Top | AnchorLine::Bottom => {
                            item_height += anchor.margin.abs();
                        }
                        _ => {}
                    },
                    AnchorTarget::Sibling { .. } => {
                        // Sibling anchors don't directly contribute to layout size
                    }
                }
            }

            max_width = max_width.max(item_width);
            max_height = max_height.max(item_height);
        }

        // Add content margins
        let margins = self.base.content_margins();
        max_width += margins.horizontal();
        max_height += margins.vertical();

        SizeHint::new(Size::new(max_width, max_height))
    }
}

impl Default for AnchorLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl Layout for AnchorLayout {
    // =========================================================================
    // Item Management
    // =========================================================================

    fn add_item(&mut self, item: LayoutItem) {
        self.base.add_item(item);
        self.item_anchors.push(Vec::new());
    }

    fn insert_item(&mut self, index: usize, item: LayoutItem) {
        self.base.insert_item(index, item);
        self.item_anchors.insert(index, Vec::new());

        // Update sibling anchor indices for items after the insertion
        for anchors in &mut self.item_anchors {
            for anchor in anchors {
                if let AnchorTarget::Sibling {
                    index: ref mut sib_idx,
                    ..
                } = anchor.target
                    && *sib_idx >= index
                {
                    *sib_idx += 1;
                }
            }
        }
    }

    fn remove_item(&mut self, index: usize) -> Option<LayoutItem> {
        if index >= self.base.item_count() {
            return None;
        }

        // Remove this item's anchors
        self.item_anchors.remove(index);

        // Update sibling anchor indices and remove invalid anchors
        for anchors in &mut self.item_anchors {
            anchors.retain_mut(|anchor| {
                if let AnchorTarget::Sibling {
                    index: ref mut sib_idx,
                    ..
                } = anchor.target
                {
                    if *sib_idx == index {
                        return false; // Remove anchor to deleted item
                    }
                    if *sib_idx > index {
                        *sib_idx -= 1;
                    }
                }
                true
            });
        }

        self.base.remove_item(index)
    }

    fn remove_widget(&mut self, widget: ObjectId) -> bool {
        if let Some(index) = self
            .base
            .items()
            .iter()
            .position(|item| matches!(item, LayoutItem::Widget(id) if *id == widget))
        {
            self.remove_item(index);
            true
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
        self.item_anchors.clear();
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
        // Anchor layouts are generally flexible in both directions
        SizePolicyPair::default()
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
        // Anchor layouts don't use uniform spacing
        self.base.spacing()
    }

    fn set_spacing(&mut self, spacing: f32) {
        self.base.set_spacing(spacing);
    }

    // =========================================================================
    // Layout Calculation
    // =========================================================================

    fn calculate<S: WidgetAccess>(&mut self, storage: &S, _available: Size) -> Size {
        let content_rect = self.base.content_rect();

        self.resolve_anchors(storage, content_rect);

        // Cache size hint
        let size_hint = self.calculate_size_hint(storage);
        self.base.set_cached_size_hint(size_hint);
        self.base
            .set_cached_minimum_size(size_hint.effective_minimum());

        self.base.mark_valid();
        self.base.geometry().size
    }

    fn apply<S: WidgetAccess>(&self, storage: &mut S) {
        for (i, item) in self.base.items().iter().enumerate() {
            if let Some(geometry) = self.base.item_geometry(i) {
                LayoutBase::apply_item_geometry(storage, item, geometry);
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
        self.base.needs_recalculation()
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
    fn test_anchor_layout_creation() {
        init_global_registry();

        let layout = AnchorLayout::new();
        assert_eq!(layout.item_count(), 0);
        assert!(layout.is_empty());
    }

    #[test]
    fn test_anchor_line_horizontal_vertical() {
        assert!(AnchorLine::Left.is_horizontal());
        assert!(AnchorLine::Right.is_horizontal());
        assert!(AnchorLine::HorizontalCenter.is_horizontal());
        assert!(AnchorLine::Top.is_vertical());
        assert!(AnchorLine::Bottom.is_vertical());
        assert!(AnchorLine::VerticalCenter.is_vertical());
    }

    #[test]
    fn test_anchor_line_opposite() {
        assert_eq!(AnchorLine::Left.opposite(), AnchorLine::Right);
        assert_eq!(AnchorLine::Right.opposite(), AnchorLine::Left);
        assert_eq!(AnchorLine::Top.opposite(), AnchorLine::Bottom);
        assert_eq!(AnchorLine::Bottom.opposite(), AnchorLine::Top);
    }

    #[test]
    fn test_anchor_to_parent_fill() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 50.0))));

        let mut layout = AnchorLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.add_widget(id);

        // Fill parent with 10px margin
        assert!(layout.fill_parent(0, 10.0));

        layout.set_geometry(Rect::new(0.0, 0.0, 200.0, 100.0));
        layout.calculate(&storage, Size::new(200.0, 100.0));
        layout.apply(&mut storage);

        let widget = storage.widgets.get(&id).unwrap();
        let geo = widget.geometry();

        // Should be inset by 10px on all sides
        assert_eq!(geo.origin.x, 10.0);
        assert_eq!(geo.origin.y, 10.0);
        assert_eq!(geo.width(), 180.0); // 200 - 10 - 10
        assert_eq!(geo.height(), 80.0); // 100 - 10 - 10
    }

    #[test]
    fn test_anchor_center_in_parent() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id = storage.add(MockWidget::new(SizeHint::new(Size::new(40.0, 20.0))));

        let mut layout = AnchorLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.add_widget(id);

        assert!(layout.center_in_parent(0));

        layout.set_geometry(Rect::new(0.0, 0.0, 100.0, 100.0));
        layout.calculate(&storage, Size::new(100.0, 100.0));
        layout.apply(&mut storage);

        let widget = storage.widgets.get(&id).unwrap();
        let geo = widget.geometry();

        // Centered: (100-40)/2 = 30, (100-20)/2 = 40
        assert_eq!(geo.origin.x, 30.0);
        assert_eq!(geo.origin.y, 40.0);
        assert_eq!(geo.width(), 40.0);
        assert_eq!(geo.height(), 20.0);
    }

    #[test]
    fn test_anchor_left_only() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id = storage.add(MockWidget::new(SizeHint::new(Size::new(60.0, 30.0))));

        let mut layout = AnchorLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.add_widget(id);

        // Only anchor left edge
        layout.anchor_to_parent(0, AnchorLine::Left, AnchorLine::Left, 15.0);
        layout.anchor_to_parent(0, AnchorLine::Top, AnchorLine::Top, 0.0);

        layout.set_geometry(Rect::new(0.0, 0.0, 200.0, 100.0));
        layout.calculate(&storage, Size::new(200.0, 100.0));
        layout.apply(&mut storage);

        let widget = storage.widgets.get(&id).unwrap();
        let geo = widget.geometry();

        // Left at 15, keeps preferred width
        assert_eq!(geo.origin.x, 15.0);
        assert_eq!(geo.width(), 60.0);
    }

    #[test]
    fn test_anchor_right_only() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));

        let mut layout = AnchorLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.add_widget(id);

        // Only anchor right edge
        layout.anchor_to_parent(0, AnchorLine::Right, AnchorLine::Right, 20.0);
        layout.anchor_to_parent(0, AnchorLine::Top, AnchorLine::Top, 0.0);

        layout.set_geometry(Rect::new(0.0, 0.0, 200.0, 100.0));
        layout.calculate(&storage, Size::new(200.0, 100.0));
        layout.apply(&mut storage);

        let widget = storage.widgets.get(&id).unwrap();
        let geo = widget.geometry();

        // Right edge at 200-20=180, so left = 180-50=130
        assert_eq!(geo.origin.x, 130.0);
        assert_eq!(geo.width(), 50.0);
    }

    #[test]
    fn test_anchor_to_sibling() {
        init_global_registry();

        let mut storage = MockStorage::new();
        let id1 = storage.add(MockWidget::new(SizeHint::new(Size::new(50.0, 30.0))));
        let id2 = storage.add(MockWidget::new(SizeHint::new(Size::new(40.0, 30.0))));

        let mut layout = AnchorLayout::new();
        layout.set_content_margins(ContentMargins::uniform(0.0));
        layout.add_widget(id1);
        layout.add_widget(id2);

        // First widget anchored to left of parent
        layout.anchor_to_parent(0, AnchorLine::Left, AnchorLine::Left, 0.0);
        layout.anchor_to_parent(0, AnchorLine::Top, AnchorLine::Top, 0.0);

        // Second widget anchored to right of first widget
        layout.anchor_to_sibling(1, AnchorLine::Left, 0, AnchorLine::Right, 10.0);
        layout.anchor_to_parent(1, AnchorLine::Top, AnchorLine::Top, 0.0);

        layout.set_geometry(Rect::new(0.0, 0.0, 200.0, 100.0));
        layout.calculate(&storage, Size::new(200.0, 100.0));
        layout.apply(&mut storage);

        let w1 = storage.widgets.get(&id1).unwrap();
        let w2 = storage.widgets.get(&id2).unwrap();

        // First widget at x=0
        assert_eq!(w1.geometry().origin.x, 0.0);
        // Second widget at x = 50 (first width) + 10 (margin) = 60
        assert_eq!(w2.geometry().origin.x, 60.0);
    }

    #[test]
    fn test_anchor_invalid_cross_axis() {
        init_global_registry();

        let mut layout = AnchorLayout::new();
        let ids = {
            use slotmap::SlotMap;
            let mut map: SlotMap<ObjectId, ()> = SlotMap::with_key();
            vec![map.insert(()), map.insert(())]
        };

        layout.add_widget(ids[0]);

        // This should fail: can't anchor horizontal to vertical
        assert!(!layout.anchor_to_parent(0, AnchorLine::Left, AnchorLine::Top, 0.0));
        assert!(!layout.anchor_to_parent(0, AnchorLine::Top, AnchorLine::Left, 0.0));
    }

    #[test]
    fn test_anchor_remove_item_updates_indices() {
        init_global_registry();

        let mut layout = AnchorLayout::new();
        let ids = {
            use slotmap::SlotMap;
            let mut map: SlotMap<ObjectId, ()> = SlotMap::with_key();
            vec![map.insert(()), map.insert(()), map.insert(())]
        };

        layout.add_widget(ids[0]);
        layout.add_widget(ids[1]);
        layout.add_widget(ids[2]);

        // Third widget anchored to second
        layout.anchor_to_sibling(2, AnchorLine::Left, 1, AnchorLine::Right, 0.0);

        // Remove second widget - anchor should be removed since target is gone
        layout.remove_item(1);

        // Third widget (now at index 1) should have no anchors
        assert_eq!(layout.anchors(1).map(|a| a.len()), Some(0));
    }
}
