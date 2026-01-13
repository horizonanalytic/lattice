//! Layout invalidation and deferred recalculation system.
//!
//! The invalidation system ensures that layout recalculations happen efficiently:
//! - Changes mark layouts as "dirty"
//! - Dirty state cascades up the layout tree
//! - Actual recalculation is deferred until needed (e.g., before paint)

use std::collections::HashSet;

use horizon_lattice_core::ObjectId;

/// Scope for layout invalidation.
///
/// Determines how invalidation propagates through the widget/layout hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationScope {
    /// Only invalidate the immediate layout.
    Local,
    /// Invalidate up to the parent layout (one level).
    Parent,
    /// Invalidate all ancestor layouts up to the root.
    All,
}

/// Manager for tracking layout invalidation.
///
/// This coordinates layout invalidation across the widget tree. When a widget's
/// size hint changes or a layout is modified, the invalidator tracks which
/// layouts need recalculation.
///
/// The invalidator supports deferred recalculation: layouts are marked dirty
/// but not immediately recalculated. The actual recalculation happens during
/// the layout activation phase (typically before painting).
#[derive(Debug, Default)]
pub struct LayoutInvalidator {
    /// Set of widget IDs whose layouts need recalculation.
    dirty_layouts: HashSet<ObjectId>,

    /// Whether a full layout pass is needed.
    full_layout_needed: bool,

    /// Whether layout processing is currently suspended.
    suspended: bool,

    /// Deferred invalidations while suspended.
    deferred: Vec<ObjectId>,
}

impl LayoutInvalidator {
    /// Create a new layout invalidator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a widget's layout as needing recalculation.
    ///
    /// If invalidation is suspended, this will be deferred until resumed.
    pub fn invalidate(&mut self, widget_id: ObjectId) {
        if self.suspended {
            self.deferred.push(widget_id);
        } else {
            self.dirty_layouts.insert(widget_id);
        }
    }

    /// Mark all layouts as needing recalculation.
    ///
    /// This is used when a global change affects all layouts (e.g., font change).
    pub fn invalidate_all(&mut self) {
        self.full_layout_needed = true;
    }

    /// Check if a widget's layout needs recalculation.
    #[inline]
    pub fn is_dirty(&self, widget_id: ObjectId) -> bool {
        self.full_layout_needed || self.dirty_layouts.contains(&widget_id)
    }

    /// Check if any layout needs recalculation.
    #[inline]
    pub fn has_dirty_layouts(&self) -> bool {
        self.full_layout_needed || !self.dirty_layouts.is_empty()
    }

    /// Get all widgets with dirty layouts.
    pub fn dirty_widgets(&self) -> impl Iterator<Item = &ObjectId> {
        self.dirty_layouts.iter()
    }

    /// Clear the dirty flag for a specific widget.
    pub fn clear(&mut self, widget_id: ObjectId) {
        self.dirty_layouts.remove(&widget_id);
    }

    /// Clear all dirty flags.
    pub fn clear_all(&mut self) {
        self.dirty_layouts.clear();
        self.full_layout_needed = false;
    }

    /// Suspend invalidation processing.
    ///
    /// While suspended, invalidations are queued but not processed.
    /// This is useful during batch operations to avoid repeated layouts.
    pub fn suspend(&mut self) {
        self.suspended = true;
    }

    /// Resume invalidation processing.
    ///
    /// Processes any deferred invalidations.
    pub fn resume(&mut self) {
        self.suspended = false;

        // Process deferred invalidations
        let deferred = std::mem::take(&mut self.deferred);
        for id in deferred {
            self.dirty_layouts.insert(id);
        }
    }

    /// Check if invalidation is currently suspended.
    #[inline]
    pub fn is_suspended(&self) -> bool {
        self.suspended
    }
}

/// Extension trait for widgets to participate in layout invalidation.
///
/// Widgets can implement this to notify parent layouts when their size
/// requirements change.
pub trait LayoutInvalidation {
    /// Notify that this widget's size hint has changed.
    ///
    /// This should cascade invalidation up to parent layouts.
    fn invalidate_size_hint(&mut self);

    /// Notify that this widget's layout needs recalculation.
    fn invalidate_layout(&mut self);
}

/// RAII guard for suspending layout invalidation.
///
/// Automatically resumes invalidation when dropped.
///
/// # Example
///
/// ```ignore
/// {
///     let _guard = invalidator.suspend_guard();
///     // Add many items without triggering multiple layout passes
///     for item in items {
///         layout.add_item(item);
///     }
/// } // Guard drops, invalidation resumes and processes all changes
/// ```
pub struct SuspendGuard<'a> {
    invalidator: &'a mut LayoutInvalidator,
}

impl<'a> SuspendGuard<'a> {
    /// Create a new suspend guard.
    pub fn new(invalidator: &'a mut LayoutInvalidator) -> Self {
        invalidator.suspend();
        Self { invalidator }
    }
}

impl Drop for SuspendGuard<'_> {
    fn drop(&mut self) {
        self.invalidator.resume();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use slotmap::SlotMap;

    // Helper to create test ObjectIds
    fn create_test_ids(count: usize) -> Vec<ObjectId> {
        let mut map: SlotMap<ObjectId, ()> = SlotMap::with_key();
        (0..count).map(|_| map.insert(())).collect()
    }

    #[test]
    fn test_invalidator_basic() {
        init_global_registry();
        let mut invalidator = LayoutInvalidator::new();
        let ids = create_test_ids(3);

        assert!(!invalidator.has_dirty_layouts());

        invalidator.invalidate(ids[0]);
        assert!(invalidator.has_dirty_layouts());
        assert!(invalidator.is_dirty(ids[0]));
        assert!(!invalidator.is_dirty(ids[1]));

        invalidator.clear(ids[0]);
        assert!(!invalidator.is_dirty(ids[0]));
    }

    #[test]
    fn test_invalidator_suspend_resume() {
        let mut invalidator = LayoutInvalidator::new();
        let ids = create_test_ids(3);

        invalidator.suspend();
        assert!(invalidator.is_suspended());

        // Invalidations while suspended are deferred
        invalidator.invalidate(ids[0]);
        invalidator.invalidate(ids[1]);
        assert!(!invalidator.has_dirty_layouts());

        // Resume processes deferred
        invalidator.resume();
        assert!(!invalidator.is_suspended());
        assert!(invalidator.is_dirty(ids[0]));
        assert!(invalidator.is_dirty(ids[1]));
    }

    #[test]
    fn test_invalidate_all() {
        let mut invalidator = LayoutInvalidator::new();
        let ids = create_test_ids(3);

        invalidator.invalidate_all();

        // All widgets should be considered dirty
        assert!(invalidator.is_dirty(ids[0]));
        assert!(invalidator.is_dirty(ids[1]));
        assert!(invalidator.is_dirty(ids[2]));
    }
}
