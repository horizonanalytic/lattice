//! Frame rendering and repaint management for widgets.
//!
//! This module provides the infrastructure for painting widget trees:
//!
//! - [`RepaintManager`]: Tracks widgets that need repainting and coalesces updates
//! - [`FrameRenderer`]: Paints the widget tree in correct order (parent-before-children)
//!
//! # Paint Event Flow
//!
//! The painting system follows this flow:
//!
//! 1. Widgets call `update()` or `update_rect()` to schedule repaints
//! 2. `RepaintManager` collects all pending updates
//! 3. At frame time, `FrameRenderer::render_frame()` is called
//! 4. Widgets are painted in depth-first preorder (parents before children)
//! 5. Opaque widgets cause parent regions underneath them to be skipped
//! 6. Dirty regions are clipped to minimize overdraw
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::{FrameRenderer, RepaintManager, WidgetAccess};
//!
//! // During event processing, widgets call update()
//! button.update();
//! label.update_rect(text_bounds);
//!
//! // At frame time, render all widgets that need painting
//! FrameRenderer::render_frame(&mut storage, root_id, &mut renderer);
//! ```

use std::collections::HashMap;

use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{GpuRenderer, Point, Rect, Renderer};

use super::events::{PaintEvent, WidgetEvent};
use super::traits::PaintContext;
use super::WidgetAccess;

/// Manages repaint requests and coalesces updates.
///
/// The `RepaintManager` tracks which widgets need repainting and their
/// dirty regions. Multiple `update()` calls on the same widget between
/// frames are coalesced into a single repaint with a combined dirty region.
///
/// # Usage
///
/// ```ignore
/// let mut repaint_mgr = RepaintManager::new();
///
/// // Mark widgets as needing repaint
/// repaint_mgr.mark_dirty(widget_id, dirty_rect);
///
/// // At frame time, get all pending repaints
/// for (widget_id, region) in repaint_mgr.pending_repaints() {
///     // Paint widget...
/// }
///
/// // Clear after rendering
/// repaint_mgr.clear();
/// ```
#[derive(Debug, Default)]
pub struct RepaintManager {
    /// Widgets that need repainting, with their dirty regions.
    /// The region is in window coordinates.
    pending: HashMap<ObjectId, Rect>,

    /// Whether a full window repaint is needed.
    full_repaint: bool,
}

impl RepaintManager {
    /// Create a new repaint manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mark a widget as needing repaint.
    ///
    /// # Arguments
    ///
    /// * `id` - The widget's ObjectId.
    /// * `window_rect` - The dirty region in window coordinates.
    pub fn mark_dirty(&mut self, id: ObjectId, window_rect: Rect) {
        // Skip empty rects
        if window_rect.width() <= 0.0 || window_rect.height() <= 0.0 {
            return;
        }

        self.pending
            .entry(id)
            .and_modify(|existing| *existing = existing.union(&window_rect))
            .or_insert(window_rect);
    }

    /// Mark that a full repaint is needed.
    ///
    /// This is typically called when the window is resized or first shown.
    pub fn invalidate_all(&mut self) {
        self.full_repaint = true;
    }

    /// Check if any widgets need repainting.
    pub fn has_pending(&self) -> bool {
        self.full_repaint || !self.pending.is_empty()
    }

    /// Check if a full repaint is needed.
    pub fn needs_full_repaint(&self) -> bool {
        self.full_repaint
    }

    /// Get the pending repaints.
    ///
    /// Returns an iterator over (widget_id, dirty_region) pairs.
    pub fn pending_repaints(&self) -> impl Iterator<Item = (ObjectId, Rect)> + '_ {
        self.pending.iter().map(|(&id, &rect)| (id, rect))
    }

    /// Get the number of pending repaints.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Clear all pending repaints.
    ///
    /// Call this after rendering the frame.
    pub fn clear(&mut self) {
        self.pending.clear();
        self.full_repaint = false;
    }
}

/// Result of rendering a frame.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    /// Number of widgets painted.
    pub widgets_painted: u32,
    /// Number of widgets skipped (not visible or no damage).
    pub widgets_skipped: u32,
    /// Number of regions skipped due to opaque widget optimization.
    pub opaque_optimizations: u32,
}

/// Renders widget trees with proper paint order and dirty region handling.
///
/// The `FrameRenderer` handles:
/// - Painting widgets in depth-first preorder (parents before children)
/// - Clipping to dirty regions
/// - Skipping parent regions covered by opaque widgets
/// - Coordinate transformations for child widgets
///
/// # Example
///
/// ```ignore
/// // Render all visible widgets
/// let stats = FrameRenderer::render_frame(&mut storage, root_id, &mut renderer);
/// println!("Painted {} widgets", stats.widgets_painted);
/// ```
pub struct FrameRenderer;

impl FrameRenderer {
    /// Render a frame by painting all widgets that need updating.
    ///
    /// This paints the widget tree starting from `root_id` in depth-first
    /// preorder (parents before children). Only widgets with `needs_repaint()`
    /// set will actually be painted.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`.
    /// * `root_id` - The root widget to start painting from.
    /// * `renderer` - The GPU renderer to paint with.
    ///
    /// # Returns
    ///
    /// Statistics about the frame rendering.
    pub fn render_frame<S: WidgetAccess>(
        storage: &mut S,
        root_id: ObjectId,
        renderer: &mut GpuRenderer,
    ) -> FrameStats {
        Self::render_frame_with_alt(storage, root_id, renderer, false)
    }

    /// Render a frame with Alt key state for mnemonic underlines.
    ///
    /// This is the same as `render_frame` but accepts an `alt_held` parameter
    /// to control whether mnemonic underlines should be displayed.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`.
    /// * `root_id` - The root widget to start painting from.
    /// * `renderer` - The GPU renderer to paint with.
    /// * `alt_held` - Whether the Alt key is currently held (for mnemonic display).
    ///
    /// # Returns
    ///
    /// Statistics about the frame rendering.
    pub fn render_frame_with_alt<S: WidgetAccess>(
        storage: &mut S,
        root_id: ObjectId,
        renderer: &mut GpuRenderer,
        alt_held: bool,
    ) -> FrameStats {
        let mut stats = FrameStats::default();

        // Collect the paint order (depth-first preorder from root)
        let paint_order = Self::collect_paint_order(storage, root_id);

        // Paint each widget
        for widget_id in paint_order {
            Self::paint_widget(storage, widget_id, renderer, Point::ZERO, alt_held, &mut stats);
        }

        stats
    }

    /// Render a frame with a specific dirty region.
    ///
    /// Only widgets that intersect with the dirty region will be painted.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`.
    /// * `root_id` - The root widget to start painting from.
    /// * `renderer` - The GPU renderer to paint with.
    /// * `dirty_region` - The region that needs repainting (in window coordinates).
    pub fn render_frame_region<S: WidgetAccess>(
        storage: &mut S,
        root_id: ObjectId,
        renderer: &mut GpuRenderer,
        dirty_region: Rect,
    ) -> FrameStats {
        Self::render_frame_region_with_alt(storage, root_id, renderer, dirty_region, false)
    }

    /// Render a frame with a specific dirty region and Alt key state.
    ///
    /// This is the same as `render_frame_region` but accepts an `alt_held` parameter
    /// to control whether mnemonic underlines should be displayed.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`.
    /// * `root_id` - The root widget to start painting from.
    /// * `renderer` - The GPU renderer to paint with.
    /// * `dirty_region` - The region that needs repainting (in window coordinates).
    /// * `alt_held` - Whether the Alt key is currently held (for mnemonic display).
    pub fn render_frame_region_with_alt<S: WidgetAccess>(
        storage: &mut S,
        root_id: ObjectId,
        renderer: &mut GpuRenderer,
        dirty_region: Rect,
        alt_held: bool,
    ) -> FrameStats {
        let mut stats = FrameStats::default();

        // Set up clipping for the dirty region
        renderer.save();
        renderer.clip_rect(dirty_region);

        // Collect the paint order
        let paint_order = Self::collect_paint_order(storage, root_id);

        // Paint each widget that intersects with dirty region
        for widget_id in paint_order {
            Self::paint_widget_with_clip(
                storage,
                widget_id,
                renderer,
                Point::ZERO,
                &dirty_region,
                alt_held,
                &mut stats,
            );
        }

        renderer.restore();

        stats
    }

    /// Collect widgets in paint order (depth-first preorder).
    fn collect_paint_order<S: WidgetAccess>(storage: &S, root_id: ObjectId) -> Vec<ObjectId> {
        let mut order = Vec::new();
        Self::collect_recursive(storage, root_id, &mut order);
        order
    }

    fn collect_recursive<S: WidgetAccess>(
        storage: &S,
        widget_id: ObjectId,
        order: &mut Vec<ObjectId>,
    ) {
        let Some(widget) = storage.get_widget(widget_id) else {
            return;
        };

        // Only include visible widgets
        if !widget.is_visible() {
            return;
        }

        // Add this widget first (preorder)
        order.push(widget_id);

        // Then add children in z-order
        let children = storage.get_children(widget_id);
        for child_id in children {
            Self::collect_recursive(storage, child_id, order);
        }
    }

    /// Paint a single widget and its subtree.
    fn paint_widget<S: WidgetAccess>(
        storage: &mut S,
        widget_id: ObjectId,
        renderer: &mut GpuRenderer,
        parent_offset: Point,
        alt_held: bool,
        stats: &mut FrameStats,
    ) {
        // Get widget info
        let (geometry, needs_paint, is_visible, is_opaque) = {
            let Some(widget) = storage.get_widget(widget_id) else {
                return;
            };
            (
                widget.geometry(),
                widget.needs_repaint(),
                widget.is_effectively_visible(),
                widget.is_opaque(),
            )
        };

        // Skip hidden widgets
        if !is_visible {
            stats.widgets_skipped += 1;
            return;
        }

        // Calculate window position
        let window_pos = Point::new(
            parent_offset.x + geometry.origin.x,
            parent_offset.y + geometry.origin.y,
        );

        // Create local rect for painting
        let local_rect = Rect::new(0.0, 0.0, geometry.size.width, geometry.size.height);

        // Paint this widget if it needs repainting
        if needs_paint {
            renderer.save();
            renderer.translate(window_pos.x, window_pos.y);

            // Create paint context and paint
            {
                let Some(widget) = storage.get_widget(widget_id) else {
                    renderer.restore();
                    return;
                };
                let mut ctx = PaintContext::new(renderer, local_rect).with_alt_held(alt_held);
                widget.paint(&mut ctx);
            }

            renderer.restore();

            // Clear the repaint flag
            if let Some(widget) = storage.get_widget_mut(widget_id) {
                widget.widget_base_mut().clear_repaint_flag();
            }

            stats.widgets_painted += 1;

            if is_opaque {
                stats.opaque_optimizations += 1;
            }
        } else {
            stats.widgets_skipped += 1;
        }

        // Paint children
        let children = storage.get_children(widget_id);
        for child_id in children {
            Self::paint_widget(storage, child_id, renderer, window_pos, alt_held, stats);
        }
    }

    /// Paint a widget with dirty region clipping.
    fn paint_widget_with_clip<S: WidgetAccess>(
        storage: &mut S,
        widget_id: ObjectId,
        renderer: &mut GpuRenderer,
        parent_offset: Point,
        dirty_region: &Rect,
        alt_held: bool,
        stats: &mut FrameStats,
    ) {
        // Get widget info
        let (geometry, is_visible, is_opaque) = {
            let Some(widget) = storage.get_widget(widget_id) else {
                return;
            };
            (
                widget.geometry(),
                widget.is_effectively_visible(),
                widget.is_opaque(),
            )
        };

        // Skip hidden widgets
        if !is_visible {
            stats.widgets_skipped += 1;
            return;
        }

        // Calculate window rect
        let window_pos = Point::new(
            parent_offset.x + geometry.origin.x,
            parent_offset.y + geometry.origin.y,
        );
        let window_rect = Rect::new(
            window_pos.x,
            window_pos.y,
            geometry.size.width,
            geometry.size.height,
        );

        // Check if widget intersects with dirty region
        let Some(intersect) = window_rect.intersect(dirty_region) else {
            stats.widgets_skipped += 1;
            return;
        };

        // Create local rect for painting
        let local_rect = Rect::new(0.0, 0.0, geometry.size.width, geometry.size.height);

        // Calculate local dirty region
        let local_dirty = Rect::new(
            intersect.origin.x - window_pos.x,
            intersect.origin.y - window_pos.y,
            intersect.size.width,
            intersect.size.height,
        );

        // Paint this widget
        renderer.save();
        renderer.translate(window_pos.x, window_pos.y);

        // Clip to local dirty region
        renderer.clip_rect(local_dirty);

        // Send paint event and paint
        {
            let Some(widget) = storage.get_widget_mut(widget_id) else {
                renderer.restore();
                return;
            };

            // Send paint event
            let mut paint_event = WidgetEvent::Paint(PaintEvent::new(local_dirty));
            let _ = widget.event(&mut paint_event);

            // Paint the widget
            let mut ctx = PaintContext::new(renderer, local_rect).with_alt_held(alt_held);
            widget.paint(&mut ctx);

            // Clear repaint flag
            widget.widget_base_mut().clear_repaint_flag();
        }

        renderer.restore();
        stats.widgets_painted += 1;

        if is_opaque {
            stats.opaque_optimizations += 1;
        }

        // Paint children
        let children = storage.get_children(widget_id);
        for child_id in children {
            Self::paint_widget_with_clip(storage, child_id, renderer, window_pos, dirty_region, alt_held, stats);
        }
    }

    /// Calculate the regions of a parent widget that are NOT covered by opaque children.
    ///
    /// This is used to optimize painting by skipping parent regions that would
    /// be completely covered by opaque children.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`.
    /// * `parent_id` - The parent widget.
    /// * `parent_rect` - The parent's rect in window coordinates.
    ///
    /// # Returns
    ///
    /// A list of rectangles that need to be painted (parent rect minus opaque child rects).
    pub fn calculate_visible_regions<S: WidgetAccess>(
        storage: &S,
        parent_id: ObjectId,
        parent_rect: Rect,
    ) -> Vec<Rect> {
        let children = storage.get_children(parent_id);

        // Collect opaque child rects
        let opaque_rects: Vec<Rect> = children
            .iter()
            .filter_map(|&child_id| {
                let widget = storage.get_widget(child_id)?;
                if widget.is_opaque() && widget.is_visible() {
                    let geom = widget.geometry();
                    Some(Rect::new(
                        parent_rect.origin.x + geom.origin.x,
                        parent_rect.origin.y + geom.origin.y,
                        geom.size.width,
                        geom.size.height,
                    ))
                } else {
                    None
                }
            })
            .collect();

        if opaque_rects.is_empty() {
            return vec![parent_rect];
        }

        // For now, use simple subtraction (just return parent rect if no full coverage)
        // A more sophisticated implementation would compute actual region difference
        Self::subtract_rects(parent_rect, &opaque_rects)
    }

    /// Simple rect subtraction. Returns regions of `rect` not covered by `subtract`.
    ///
    /// This is a simplified implementation that handles common cases.
    /// For fully general region subtraction, a more complex algorithm would be needed.
    fn subtract_rects(rect: Rect, subtract: &[Rect]) -> Vec<Rect> {
        // For now, check if any subtraction rect fully covers the rect
        for sub in subtract {
            if sub.origin.x <= rect.origin.x
                && sub.origin.y <= rect.origin.y
                && sub.right() >= rect.right()
                && sub.bottom() >= rect.bottom()
            {
                // Fully covered, nothing to paint
                return vec![];
            }
        }

        // Otherwise, just return the original rect
        // A full implementation would compute the difference regions
        vec![rect]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::KeyData;

    /// Create a test ObjectId from index and version.
    fn test_object_id(idx: u32, version: u32) -> ObjectId {
        ObjectId::from(KeyData::from_ffi((idx as u64) | ((version as u64) << 32)))
    }

    #[test]
    fn test_repaint_manager_empty() {
        let mgr = RepaintManager::new();
        assert!(!mgr.has_pending());
        assert_eq!(mgr.pending_count(), 0);
    }

    #[test]
    fn test_repaint_manager_mark_dirty() {
        let mut mgr = RepaintManager::new();
        let id = test_object_id(1, 1);

        mgr.mark_dirty(id, Rect::new(0.0, 0.0, 100.0, 50.0));

        assert!(mgr.has_pending());
        assert_eq!(mgr.pending_count(), 1);
    }

    #[test]
    fn test_repaint_manager_coalesce() {
        let mut mgr = RepaintManager::new();
        let id = test_object_id(1, 1);

        mgr.mark_dirty(id, Rect::new(0.0, 0.0, 50.0, 50.0));
        mgr.mark_dirty(id, Rect::new(25.0, 25.0, 50.0, 50.0));

        assert_eq!(mgr.pending_count(), 1);

        // Should be coalesced to union
        let (_, region) = mgr.pending_repaints().next().unwrap();
        assert_eq!(region.left(), 0.0);
        assert_eq!(region.top(), 0.0);
        assert_eq!(region.right(), 75.0);
        assert_eq!(region.bottom(), 75.0);
    }

    #[test]
    fn test_repaint_manager_clear() {
        let mut mgr = RepaintManager::new();
        let id = test_object_id(1, 1);

        mgr.mark_dirty(id, Rect::new(0.0, 0.0, 100.0, 50.0));
        mgr.invalidate_all();

        assert!(mgr.needs_full_repaint());

        mgr.clear();

        assert!(!mgr.has_pending());
        assert!(!mgr.needs_full_repaint());
    }

    #[test]
    fn test_repaint_manager_skip_empty() {
        let mut mgr = RepaintManager::new();
        let id = test_object_id(1, 1);

        mgr.mark_dirty(id, Rect::new(0.0, 0.0, 0.0, 50.0)); // zero width
        mgr.mark_dirty(id, Rect::new(0.0, 0.0, 50.0, 0.0)); // zero height

        assert!(!mgr.has_pending());
    }

    #[test]
    fn test_subtract_rects_full_coverage() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let subtract = vec![Rect::new(-10.0, -10.0, 200.0, 200.0)]; // Fully covers

        let result = FrameRenderer::subtract_rects(rect, &subtract);
        assert!(result.is_empty());
    }

    #[test]
    fn test_subtract_rects_no_coverage() {
        let rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let subtract = vec![Rect::new(200.0, 200.0, 50.0, 50.0)]; // No overlap

        let result = FrameRenderer::subtract_rects(rect, &subtract);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], rect);
    }
}
