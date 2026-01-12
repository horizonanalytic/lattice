//! Damage tracking for efficient partial rendering.
//!
//! This module provides [`DamageTracker`] for tracking which regions of the screen
//! have changed and need to be repainted. This enables optimizations like:
//!
//! - Only re-rendering areas that have changed
//! - Using scissor rectangles to limit GPU work
//! - Skipping untouched regions entirely
//!
//! # Architecture
//!
//! The damage tracking system uses a simple union-based approach where dirty regions
//! are accumulated into a single bounding rectangle. This is efficient for typical
//! UI patterns and integrates well with GPU scissor-based rendering.
//!
//! More sophisticated approaches (region lists, R-trees) can be added if profiling
//! shows they would provide benefit.

use crate::types::Rect;

/// Tracks damaged (dirty) regions that need repainting.
///
/// The damage tracker accumulates dirty rectangles and provides the combined
/// damage region for efficient partial rendering. All damage is accumulated
/// into a single bounding rectangle for simplicity and GPU efficiency.
///
/// # Example
///
/// ```
/// use horizon_lattice_render::damage::DamageTracker;
/// use horizon_lattice_render::Rect;
///
/// let mut tracker = DamageTracker::new();
///
/// // Mark regions as dirty
/// tracker.add_damage(Rect::new(10.0, 10.0, 50.0, 30.0));
/// tracker.add_damage(Rect::new(100.0, 100.0, 20.0, 20.0));
///
/// // Get the combined damage region
/// if let Some(damage) = tracker.damage_region() {
///     println!("Need to repaint: {:?}", damage);
/// }
///
/// // Clear damage after rendering
/// tracker.clear();
/// ```
#[derive(Debug, Clone, Default)]
pub struct DamageTracker {
    /// The accumulated damage region (union of all dirty rects).
    damage: Option<Rect>,
    /// Total number of damage regions added this frame.
    damage_count: u32,
    /// Whether full repaint is needed (damage exceeds threshold).
    full_repaint: bool,
    /// Viewport bounds for damage validation.
    viewport: Option<Rect>,
}

impl DamageTracker {
    /// Create a new damage tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a damage tracker with a specific viewport.
    ///
    /// The viewport is used to clip damage regions and determine
    /// when full repaint is more efficient.
    pub fn with_viewport(viewport: Rect) -> Self {
        Self {
            damage: None,
            damage_count: 0,
            full_repaint: false,
            viewport: Some(viewport),
        }
    }

    /// Set the viewport bounds.
    ///
    /// This should be called when the window/surface is resized.
    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = Some(viewport);
        // Check if current damage exceeds the threshold for full repaint
        self.check_full_repaint_threshold();
    }

    /// Add a damaged region that needs repainting.
    ///
    /// The region is unioned with any existing damage to create
    /// a combined bounding rectangle.
    pub fn add_damage(&mut self, rect: Rect) {
        // Skip empty rects
        if rect.width() <= 0.0 || rect.height() <= 0.0 {
            return;
        }

        // Clip to viewport if set
        let rect = if let Some(viewport) = &self.viewport {
            match rect.intersect(viewport) {
                Some(clipped) => clipped,
                None => return, // Damage outside viewport
            }
        } else {
            rect
        };

        self.damage_count += 1;

        self.damage = Some(match self.damage {
            Some(existing) => existing.union(&rect),
            None => rect,
        });

        // Check if we should switch to full repaint
        self.check_full_repaint_threshold();
    }

    /// Mark a region as damaged by its bounds.
    ///
    /// Convenience method that creates a rect from position and size.
    #[inline]
    pub fn add_damage_bounds(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.add_damage(Rect::new(x, y, width, height));
    }

    /// Mark the entire viewport as damaged (full repaint needed).
    pub fn invalidate_all(&mut self) {
        self.full_repaint = true;
        self.damage = self.viewport;
        self.damage_count = 1;
    }

    /// Get the current damage region.
    ///
    /// Returns `None` if no damage has been recorded.
    /// Returns the viewport if full repaint is flagged.
    pub fn damage_region(&self) -> Option<Rect> {
        if self.full_repaint {
            self.viewport
        } else {
            self.damage
        }
    }

    /// Check if any damage has been recorded.
    #[inline]
    pub fn has_damage(&self) -> bool {
        self.damage.is_some() || self.full_repaint
    }

    /// Check if a full repaint is needed.
    ///
    /// Full repaint is triggered when:
    /// - `invalidate_all()` was called
    /// - Damage area exceeds 90% of viewport
    /// - Too many fragmented damage regions were added
    #[inline]
    pub fn needs_full_repaint(&self) -> bool {
        self.full_repaint
    }

    /// Get the number of damage regions added this frame.
    #[inline]
    pub fn damage_count(&self) -> u32 {
        self.damage_count
    }

    /// Clear all damage.
    ///
    /// Should be called after rendering the damaged regions.
    pub fn clear(&mut self) {
        self.damage = None;
        self.damage_count = 0;
        self.full_repaint = false;
    }

    /// Get the viewport bounds.
    pub fn viewport(&self) -> Option<Rect> {
        self.viewport
    }

    /// Calculate the damage ratio (damage area / viewport area).
    ///
    /// Returns 0.0 if no damage or no viewport, 1.0 for full damage.
    pub fn damage_ratio(&self) -> f32 {
        match (&self.damage, &self.viewport) {
            (Some(damage), Some(viewport)) => {
                let damage_area = damage.width() * damage.height();
                let viewport_area = viewport.width() * viewport.height();
                if viewport_area > 0.0 {
                    (damage_area / viewport_area).min(1.0)
                } else {
                    0.0
                }
            }
            _ => 0.0,
        }
    }

    /// Check if damage exceeds threshold for full repaint.
    fn check_full_repaint_threshold(&mut self) {
        // If damage covers more than 90% of viewport, just do full repaint
        const FULL_REPAINT_THRESHOLD: f32 = 0.9;

        // If we have too many fragmented regions, also switch to full repaint
        const MAX_DAMAGE_COUNT: u32 = 100;

        if self.damage_count > MAX_DAMAGE_COUNT {
            self.full_repaint = true;
            self.damage = self.viewport;
            return;
        }

        if self.damage_ratio() > FULL_REPAINT_THRESHOLD {
            self.full_repaint = true;
            self.damage = self.viewport;
        }
    }
}

/// Extension trait for accumulating damage from multiple sources.
pub trait DamageSource {
    /// Report damage to the tracker.
    fn report_damage(&self, tracker: &mut DamageTracker);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_tracker() {
        let tracker = DamageTracker::new();
        assert!(!tracker.has_damage());
        assert!(tracker.damage_region().is_none());
        assert_eq!(tracker.damage_count(), 0);
    }

    #[test]
    fn test_single_damage() {
        let mut tracker = DamageTracker::new();
        tracker.add_damage(Rect::new(10.0, 20.0, 100.0, 50.0));

        assert!(tracker.has_damage());
        assert_eq!(tracker.damage_count(), 1);

        let damage = tracker.damage_region().unwrap();
        assert_eq!(damage.left(), 10.0);
        assert_eq!(damage.top(), 20.0);
        assert_eq!(damage.width(), 100.0);
        assert_eq!(damage.height(), 50.0);
    }

    #[test]
    fn test_damage_union() {
        let mut tracker = DamageTracker::new();
        tracker.add_damage(Rect::new(10.0, 10.0, 50.0, 50.0));
        tracker.add_damage(Rect::new(100.0, 100.0, 30.0, 30.0));

        let damage = tracker.damage_region().unwrap();
        // Union should encompass both rects
        assert_eq!(damage.left(), 10.0);
        assert_eq!(damage.top(), 10.0);
        assert_eq!(damage.right(), 130.0); // 100 + 30
        assert_eq!(damage.bottom(), 130.0); // 100 + 30
    }

    #[test]
    fn test_viewport_clipping() {
        let mut tracker = DamageTracker::with_viewport(Rect::new(0.0, 0.0, 800.0, 600.0));

        // Add damage partially outside viewport
        tracker.add_damage(Rect::new(-50.0, -50.0, 100.0, 100.0));

        let damage = tracker.damage_region().unwrap();
        // Should be clipped to viewport
        assert_eq!(damage.left(), 0.0);
        assert_eq!(damage.top(), 0.0);
        assert_eq!(damage.right(), 50.0);
        assert_eq!(damage.bottom(), 50.0);
    }

    #[test]
    fn test_damage_outside_viewport() {
        let mut tracker = DamageTracker::with_viewport(Rect::new(0.0, 0.0, 800.0, 600.0));

        // Add damage completely outside viewport
        tracker.add_damage(Rect::new(1000.0, 1000.0, 100.0, 100.0));

        // Should have no damage
        assert!(!tracker.has_damage());
    }

    #[test]
    fn test_invalidate_all() {
        let mut tracker = DamageTracker::with_viewport(Rect::new(0.0, 0.0, 800.0, 600.0));

        tracker.invalidate_all();

        assert!(tracker.has_damage());
        assert!(tracker.needs_full_repaint());

        let damage = tracker.damage_region().unwrap();
        assert_eq!(damage.width(), 800.0);
        assert_eq!(damage.height(), 600.0);
    }

    #[test]
    fn test_clear() {
        let mut tracker = DamageTracker::new();
        tracker.add_damage(Rect::new(10.0, 10.0, 50.0, 50.0));
        tracker.clear();

        assert!(!tracker.has_damage());
        assert_eq!(tracker.damage_count(), 0);
    }

    #[test]
    fn test_damage_ratio() {
        let mut tracker = DamageTracker::with_viewport(Rect::new(0.0, 0.0, 100.0, 100.0));

        // 25% damage
        tracker.add_damage(Rect::new(0.0, 0.0, 50.0, 50.0));
        assert!((tracker.damage_ratio() - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_full_repaint_threshold() {
        let mut tracker = DamageTracker::with_viewport(Rect::new(0.0, 0.0, 100.0, 100.0));

        // Add damage covering 95% of viewport
        tracker.add_damage(Rect::new(0.0, 0.0, 98.0, 98.0));

        // Should trigger full repaint
        assert!(tracker.needs_full_repaint());
    }

    #[test]
    fn test_empty_rect_ignored() {
        let mut tracker = DamageTracker::new();
        tracker.add_damage(Rect::new(10.0, 10.0, 0.0, 50.0)); // zero width
        tracker.add_damage(Rect::new(10.0, 10.0, 50.0, 0.0)); // zero height
        tracker.add_damage(Rect::new(10.0, 10.0, -10.0, 50.0)); // negative width

        assert!(!tracker.has_damage());
        assert_eq!(tracker.damage_count(), 0);
    }
}
