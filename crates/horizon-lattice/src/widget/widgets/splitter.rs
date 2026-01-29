//! Splitter widget implementation.
//!
//! This module provides [`Splitter`], a container widget that allows resizing
//! of child panes by dragging handles between them.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Splitter, Orientation};
//!
//! // Create a horizontal splitter
//! let mut splitter = Splitter::new(Orientation::Horizontal);
//!
//! // Add panes
//! splitter.add_widget(left_panel_id);
//! splitter.add_widget(right_panel_id);
//!
//! // Set initial sizes
//! splitter.set_sizes(vec![200, 400]);
//!
//! // Connect to resize events
//! splitter.splitter_moved.connect(|&(index, position)| {
//!     println!("Handle {} moved to {}", index, position);
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer};

use crate::widget::{
    FocusPolicy, MouseButton, MouseDoubleClickEvent, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

use super::Orientation;

/// A container widget with resizable panes.
///
/// Splitter provides a way to divide an area into resizable panes separated
/// by draggable handles. It supports both horizontal and vertical orientations.
///
/// # Features
///
/// - Multiple child panes with user-resizable boundaries
/// - Horizontal or vertical orientation
/// - Minimum pane sizes
/// - Collapsible panes (double-click to collapse/restore)
/// - Save/restore sizes for session persistence
///
/// # Signals
///
/// - `splitter_moved(i32, i32)`: Emitted when a handle is dragged (handle index, new position)
pub struct Splitter {
    /// Widget base.
    base: WidgetBase,

    /// Child widget IDs (panes).
    children: Vec<ObjectId>,

    /// Sizes of each pane (in pixels).
    sizes: Vec<i32>,

    /// Splitter orientation.
    orientation: Orientation,

    /// Handle width in pixels.
    handle_width: f32,

    /// Minimum size for each pane.
    minimum_sizes: Vec<i32>,

    /// Whether each pane is collapsible.
    collapsible: Vec<bool>,

    /// Collapsed state for each pane (stores size before collapse).
    collapsed_sizes: Vec<Option<i32>>,

    /// Currently dragging handle index.
    dragging_handle: Option<usize>,

    /// Position where drag started.
    drag_start_pos: f32,

    /// Sizes when drag started.
    drag_start_sizes: Vec<i32>,

    /// Currently hovered handle index.
    hover_handle: Option<usize>,

    /// Default minimum pane size.
    default_minimum_size: i32,

    /// Handle color.
    handle_color: Color,

    /// Handle hover color.
    handle_hover_color: Color,

    /// Handle pressed color.
    handle_pressed_color: Color,

    /// Signal emitted when a handle is moved.
    pub splitter_moved: Signal<(i32, i32)>,
}

impl Splitter {
    /// Create a new splitter with the given orientation.
    pub fn new(orientation: Orientation) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::NoFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Expanding,
            SizePolicy::Expanding,
        ));

        Self {
            base,
            children: Vec::new(),
            sizes: Vec::new(),
            orientation,
            handle_width: 5.0,
            minimum_sizes: Vec::new(),
            collapsible: Vec::new(),
            collapsed_sizes: Vec::new(),
            dragging_handle: None,
            drag_start_pos: 0.0,
            drag_start_sizes: Vec::new(),
            hover_handle: None,
            default_minimum_size: 30,
            handle_color: Color::from_rgb8(200, 200, 200),
            handle_hover_color: Color::from_rgb8(160, 160, 160),
            handle_pressed_color: Color::from_rgb8(120, 120, 120),
            splitter_moved: Signal::new(),
        }
    }

    // =========================================================================
    // Orientation
    // =========================================================================

    /// Get the orientation.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Set the orientation.
    pub fn set_orientation(&mut self, orientation: Orientation) {
        if self.orientation != orientation {
            self.orientation = orientation;
            self.distribute_sizes();
            self.base.update();
        }
    }

    /// Set orientation using builder pattern.
    pub fn with_orientation(mut self, orientation: Orientation) -> Self {
        self.set_orientation(orientation);
        self
    }

    // =========================================================================
    // Child Widget Management
    // =========================================================================

    /// Add a widget as a pane.
    ///
    /// Returns the index of the new pane.
    pub fn add_widget(&mut self, widget_id: ObjectId) -> usize {
        let index = self.children.len();
        self.children.push(widget_id);
        self.sizes.push(0); // Will be distributed
        self.minimum_sizes.push(self.default_minimum_size);
        self.collapsible.push(false);
        self.collapsed_sizes.push(None);

        self.distribute_sizes();
        self.base.update();
        index
    }

    /// Insert a widget at the specified index.
    ///
    /// Returns the actual index where the widget was inserted.
    pub fn insert_widget(&mut self, index: usize, widget_id: ObjectId) -> usize {
        let insert_pos = index.min(self.children.len());

        self.children.insert(insert_pos, widget_id);
        self.sizes.insert(insert_pos, 0);
        self.minimum_sizes
            .insert(insert_pos, self.default_minimum_size);
        self.collapsible.insert(insert_pos, false);
        self.collapsed_sizes.insert(insert_pos, None);

        self.distribute_sizes();
        self.base.update();
        insert_pos
    }

    /// Remove the widget at the specified index.
    ///
    /// Returns the widget ID of the removed pane, if any.
    pub fn remove_widget(&mut self, index: usize) -> Option<ObjectId> {
        if index >= self.children.len() {
            return None;
        }

        let widget_id = self.children.remove(index);
        self.sizes.remove(index);
        self.minimum_sizes.remove(index);
        self.collapsible.remove(index);
        self.collapsed_sizes.remove(index);

        self.distribute_sizes();
        self.base.update();
        Some(widget_id)
    }

    /// Get the number of panes.
    pub fn count(&self) -> usize {
        self.children.len()
    }

    /// Check if the splitter has no panes.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Get the widget ID at a specific index.
    pub fn widget(&self, index: usize) -> Option<ObjectId> {
        self.children.get(index).copied()
    }

    /// Find the index of a widget.
    ///
    /// Returns `None` if the widget is not in the splitter.
    pub fn index_of(&self, widget_id: ObjectId) -> Option<usize> {
        self.children.iter().position(|&id| id == widget_id)
    }

    // =========================================================================
    // Sizes
    // =========================================================================

    /// Get the current sizes of all panes.
    pub fn sizes(&self) -> Vec<i32> {
        self.sizes.clone()
    }

    /// Set the sizes of all panes.
    ///
    /// If the number of sizes doesn't match the number of panes, extra sizes
    /// are ignored or missing sizes are filled with equal distribution.
    pub fn set_sizes(&mut self, sizes: Vec<i32>) {
        if self.children.is_empty() {
            return;
        }

        // Apply provided sizes, respecting minimums
        for (i, &size) in sizes.iter().enumerate() {
            if i < self.sizes.len() {
                let min = self.minimum_size(i);
                self.sizes[i] = size.max(min);
                // Clear collapsed state if size is being set explicitly
                self.collapsed_sizes[i] = None;
            }
        }

        // If fewer sizes provided, distribute remaining space
        if sizes.len() < self.sizes.len() {
            self.distribute_remaining_sizes(sizes.len());
        }

        self.base.update();
    }

    /// Set sizes using builder pattern.
    pub fn with_sizes(mut self, sizes: Vec<i32>) -> Self {
        self.set_sizes(sizes);
        self
    }

    /// Get the size of a specific pane.
    pub fn size(&self, index: usize) -> Option<i32> {
        self.sizes.get(index).copied()
    }

    /// Distribute sizes equally among panes.
    fn distribute_sizes(&mut self) {
        if self.children.is_empty() {
            return;
        }

        let rect = self.base.rect();
        let available = match self.orientation {
            Orientation::Horizontal => rect.width(),
            Orientation::Vertical => rect.height(),
        };

        // Calculate total handle space
        let handle_space = if self.children.len() > 1 {
            (self.children.len() - 1) as f32 * self.handle_width
        } else {
            0.0
        };

        let content_space = (available - handle_space).max(0.0);
        let count = self.children.len() as f32;
        let equal_size = (content_space / count) as i32;

        for i in 0..self.children.len() {
            let min = self.minimum_size(i);
            self.sizes[i] = equal_size.max(min);
        }
    }

    /// Distribute remaining space to panes after index.
    fn distribute_remaining_sizes(&mut self, start_index: usize) {
        if start_index >= self.sizes.len() {
            return;
        }

        let rect = self.base.rect();
        let available = match self.orientation {
            Orientation::Horizontal => rect.width(),
            Orientation::Vertical => rect.height(),
        };

        // Calculate used space
        let handle_space = if self.children.len() > 1 {
            (self.children.len() - 1) as f32 * self.handle_width
        } else {
            0.0
        };

        let used: i32 = self.sizes[..start_index].iter().sum();
        let remaining = (available - handle_space) as i32 - used;
        let count = self.sizes.len() - start_index;

        if count > 0 && remaining > 0 {
            let equal_size = remaining / count as i32;
            for i in start_index..self.sizes.len() {
                let min = self.minimum_size(i);
                self.sizes[i] = equal_size.max(min);
            }
        }
    }

    // =========================================================================
    // Minimum Sizes
    // =========================================================================

    /// Get the minimum size of a pane.
    pub fn minimum_size(&self, index: usize) -> i32 {
        self.minimum_sizes
            .get(index)
            .copied()
            .unwrap_or(self.default_minimum_size)
    }

    /// Set the minimum size of a pane.
    pub fn set_minimum_size(&mut self, index: usize, min_size: i32) {
        if index < self.minimum_sizes.len() {
            self.minimum_sizes[index] = min_size.max(0);
            // Ensure current size respects minimum
            if self.sizes[index] < min_size && self.collapsed_sizes[index].is_none() {
                self.sizes[index] = min_size;
                self.base.update();
            }
        }
    }

    /// Get the default minimum pane size.
    pub fn default_minimum_size(&self) -> i32 {
        self.default_minimum_size
    }

    /// Set the default minimum pane size.
    pub fn set_default_minimum_size(&mut self, size: i32) {
        self.default_minimum_size = size.max(0);
    }

    /// Set default minimum size using builder pattern.
    pub fn with_default_minimum_size(mut self, size: i32) -> Self {
        self.set_default_minimum_size(size);
        self
    }

    // =========================================================================
    // Collapsible Panes
    // =========================================================================

    /// Check if a pane is collapsible.
    pub fn is_collapsible(&self, index: usize) -> bool {
        self.collapsible.get(index).copied().unwrap_or(false)
    }

    /// Set whether a pane is collapsible.
    pub fn set_collapsible(&mut self, index: usize, collapsible: bool) {
        if index < self.collapsible.len() {
            self.collapsible[index] = collapsible;
        }
    }

    /// Check if a pane is currently collapsed.
    pub fn is_collapsed(&self, index: usize) -> bool {
        self.collapsed_sizes.get(index).and_then(|s| *s).is_some()
    }

    /// Collapse a pane.
    ///
    /// Returns `true` if the pane was collapsed.
    pub fn collapse(&mut self, index: usize) -> bool {
        if index >= self.children.len() || !self.is_collapsible(index) {
            return false;
        }

        // Don't collapse if already collapsed
        if self.is_collapsed(index) {
            return false;
        }

        // Store current size for restoration
        self.collapsed_sizes[index] = Some(self.sizes[index]);

        // Set size to 0 (collapsed)
        self.sizes[index] = 0;

        // Redistribute the freed space to adjacent pane
        self.redistribute_collapsed_space(index, true);

        self.base.update();
        true
    }

    /// Restore a collapsed pane.
    ///
    /// Returns `true` if the pane was restored.
    pub fn restore(&mut self, index: usize) -> bool {
        if index >= self.children.len() {
            return false;
        }

        // Get the stored size
        let stored_size = match self.collapsed_sizes[index] {
            Some(size) => size,
            None => return false,
        };

        // Clear collapsed state
        self.collapsed_sizes[index] = None;

        // Restore the size
        self.sizes[index] = stored_size;

        // Take space from adjacent pane
        self.redistribute_collapsed_space(index, false);

        self.base.update();
        true
    }

    /// Toggle collapse state of a pane.
    pub fn toggle_collapse(&mut self, index: usize) -> bool {
        if self.is_collapsed(index) {
            self.restore(index)
        } else {
            self.collapse(index)
        }
    }

    /// Redistribute space when collapsing/restoring.
    fn redistribute_collapsed_space(&mut self, index: usize, collapsing: bool) {
        // Find an adjacent pane that can receive/give space
        let adjacent = if index + 1 < self.children.len() {
            index + 1
        } else if index > 0 {
            index - 1
        } else {
            return;
        };

        if collapsing {
            // Give space from collapsed pane to adjacent
            if let Some(collapsed_size) = self.collapsed_sizes[index] {
                self.sizes[adjacent] += collapsed_size;
            }
        } else {
            // Take space from adjacent to restore pane
            let restored_size = self.sizes[index];
            let min_adjacent = self.minimum_size(adjacent);
            let available = self.sizes[adjacent] - min_adjacent;

            if available >= restored_size {
                self.sizes[adjacent] -= restored_size;
            } else {
                // Not enough space, just use what's available
                self.sizes[adjacent] = min_adjacent;
                self.sizes[index] = available.max(0);
            }
        }
    }

    // =========================================================================
    // Handle Properties
    // =========================================================================

    /// Get the handle width.
    pub fn handle_width(&self) -> f32 {
        self.handle_width
    }

    /// Set the handle width.
    pub fn set_handle_width(&mut self, width: f32) {
        if (self.handle_width - width).abs() > f32::EPSILON {
            self.handle_width = width.max(1.0);
            self.base.update();
        }
    }

    /// Set handle width using builder pattern.
    pub fn with_handle_width(mut self, width: f32) -> Self {
        self.set_handle_width(width);
        self
    }

    // =========================================================================
    // Appearance
    // =========================================================================

    /// Get the handle color.
    pub fn handle_color(&self) -> Color {
        self.handle_color
    }

    /// Set the handle color.
    pub fn set_handle_color(&mut self, color: Color) {
        if self.handle_color != color {
            self.handle_color = color;
            self.base.update();
        }
    }

    /// Set handle color using builder pattern.
    pub fn with_handle_color(mut self, color: Color) -> Self {
        self.handle_color = color;
        self
    }

    /// Get the handle hover color.
    pub fn handle_hover_color(&self) -> Color {
        self.handle_hover_color
    }

    /// Set the handle hover color.
    pub fn set_handle_hover_color(&mut self, color: Color) {
        if self.handle_hover_color != color {
            self.handle_hover_color = color;
            self.base.update();
        }
    }

    /// Set handle hover color using builder pattern.
    pub fn with_handle_hover_color(mut self, color: Color) -> Self {
        self.handle_hover_color = color;
        self
    }

    // =========================================================================
    // Save/Restore State
    // =========================================================================

    /// Get the splitter state as a string for persistence.
    ///
    /// The format is a comma-separated list of sizes.
    pub fn save_state(&self) -> String {
        self.sizes
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Restore the splitter state from a saved string.
    ///
    /// Returns `true` if the state was successfully restored.
    pub fn restore_state(&mut self, state: &str) -> bool {
        let sizes: Vec<i32> = state
            .split(',')
            .filter_map(|s| s.trim().parse().ok())
            .collect();

        if sizes.is_empty() {
            return false;
        }

        self.set_sizes(sizes);
        true
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    /// Get the rectangle for a specific handle.
    fn handle_rect(&self, index: usize) -> Option<Rect> {
        if index >= self.children.len().saturating_sub(1) {
            return None;
        }

        let rect = self.base.rect();

        // Calculate position of handle
        let mut pos: f32 = 0.0;
        for i in 0..=index {
            pos += self.sizes[i] as f32;
            if i < index {
                pos += self.handle_width;
            }
        }

        match self.orientation {
            Orientation::Horizontal => Some(Rect::new(pos, 0.0, self.handle_width, rect.height())),
            Orientation::Vertical => Some(Rect::new(0.0, pos, rect.width(), self.handle_width)),
        }
    }

    /// Get the rectangle for a specific pane.
    #[allow(dead_code)]
    fn pane_rect(&self, index: usize) -> Option<Rect> {
        if index >= self.children.len() {
            return None;
        }

        let rect = self.base.rect();

        // Calculate position of pane
        let mut pos: f32 = 0.0;
        for i in 0..index {
            pos += self.sizes[i] as f32 + self.handle_width;
        }

        let size = self.sizes[index] as f32;

        match self.orientation {
            Orientation::Horizontal => Some(Rect::new(pos, 0.0, size, rect.height())),
            Orientation::Vertical => Some(Rect::new(0.0, pos, rect.width(), size)),
        }
    }

    /// Hit test to find which handle is at a point.
    fn hit_test_handle(&self, pos: Point) -> Option<usize> {
        for i in 0..self.children.len().saturating_sub(1) {
            if let Some(rect) = self.handle_rect(i) {
                // Expand hit area slightly for easier grabbing
                let expanded = Rect::new(
                    rect.origin.x - 2.0,
                    rect.origin.y - 2.0,
                    rect.width() + 4.0,
                    rect.height() + 4.0,
                );
                if expanded.contains(pos) {
                    return Some(i);
                }
            }
        }
        None
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if let Some(handle_index) = self.hit_test_handle(event.local_pos) {
            self.dragging_handle = Some(handle_index);
            self.drag_start_pos = match self.orientation {
                Orientation::Horizontal => event.local_pos.x,
                Orientation::Vertical => event.local_pos.y,
            };
            self.drag_start_sizes = self.sizes.clone();
            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if self.dragging_handle.is_some() {
            self.dragging_handle = None;
            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        // Update hover state
        let new_hover = self.hit_test_handle(event.local_pos);
        if self.hover_handle != new_hover {
            self.hover_handle = new_hover;
            self.base.update();
        }

        // Handle dragging
        if let Some(handle_index) = self.dragging_handle {
            let current_pos = match self.orientation {
                Orientation::Horizontal => event.local_pos.x,
                Orientation::Vertical => event.local_pos.y,
            };

            let delta = (current_pos - self.drag_start_pos) as i32;

            // Calculate new sizes
            let left_index = handle_index;
            let right_index = handle_index + 1;

            let left_min = self.minimum_size(left_index);
            let right_min = self.minimum_size(right_index);

            let left_start = self.drag_start_sizes[left_index];
            let right_start = self.drag_start_sizes[right_index];

            // Calculate new sizes while respecting minimums
            let mut new_left = left_start + delta;
            let mut new_right = right_start - delta;

            // Clamp to minimums
            if new_left < left_min {
                let adjust = left_min - new_left;
                new_left = left_min;
                new_right -= adjust;
            }

            if new_right < right_min {
                let adjust = right_min - new_right;
                new_right = right_min;
                new_left -= adjust;
            }

            // Final clamp
            new_left = new_left.max(left_min);
            new_right = new_right.max(right_min);

            // Apply if changed
            if self.sizes[left_index] != new_left || self.sizes[right_index] != new_right {
                self.sizes[left_index] = new_left;
                self.sizes[right_index] = new_right;

                // Calculate handle position for signal
                let handle_pos: i32 = self.sizes[..=handle_index].iter().sum();

                self.splitter_moved.emit((handle_index as i32, handle_pos));
                self.base.update();
            }

            return true;
        }

        false
    }

    fn handle_leave(&mut self) {
        if self.hover_handle.is_some() {
            self.hover_handle = None;
            self.base.update();
        }
    }

    fn handle_double_click(&mut self, event: &MouseDoubleClickEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        if let Some(handle_index) = self.hit_test_handle(event.local_pos) {
            // Double-click toggles collapse of the left pane
            if self.is_collapsible(handle_index) {
                self.toggle_collapse(handle_index);
                return true;
            }
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_handles(&self, ctx: &mut PaintContext<'_>) {
        for i in 0..self.children.len().saturating_sub(1) {
            if let Some(rect) = self.handle_rect(i) {
                let color = if self.dragging_handle == Some(i) {
                    self.handle_pressed_color
                } else if self.hover_handle == Some(i) {
                    self.handle_hover_color
                } else {
                    self.handle_color
                };

                ctx.renderer().fill_rect(rect, color);
            }
        }
    }
}

impl Default for Splitter {
    fn default() -> Self {
        Self::new(Orientation::Horizontal)
    }
}

impl Object for Splitter {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Splitter {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Calculate based on children minimum sizes
        let handle_space = if self.children.len() > 1 {
            (self.children.len() - 1) as f32 * self.handle_width
        } else {
            0.0
        };

        let min_content: i32 = self.minimum_sizes.iter().sum();
        let min_total = min_content as f32 + handle_space;

        match self.orientation {
            Orientation::Horizontal => SizeHint::from_dimensions(200.0, 100.0)
                .with_minimum_dimensions(min_total.max(50.0), 50.0),
            Orientation::Vertical => SizeHint::from_dimensions(100.0, 200.0)
                .with_minimum_dimensions(50.0, min_total.max(50.0)),
        }
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint handles between panes
        self.paint_handles(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                // Check for double-click first (click count handling would be in event)
                if self.handle_mouse_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if self.handle_mouse_release(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseMove(e) => {
                if self.handle_mouse_move(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::DoubleClick(e) => {
                if self.handle_double_click(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::Leave(_) => {
                self.handle_leave();
            }
            WidgetEvent::Resize(_) => {
                // Redistribute sizes when widget is resized
                self.distribute_sizes();
            }
            _ => {}
        }
        false
    }
}

// Ensure Splitter is Send + Sync
static_assertions::assert_impl_all!(Splitter: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::widget::base::WidgetBase;
    use crate::widget::traits::{PaintContext, Widget};
    use horizon_lattice_core::{Object, init_global_registry};

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
    fn test_splitter_creation() {
        setup();
        let splitter = Splitter::new(Orientation::Horizontal);
        assert_eq!(splitter.orientation(), Orientation::Horizontal);
        assert_eq!(splitter.count(), 0);
        assert!(splitter.is_empty());
        assert_eq!(splitter.handle_width(), 5.0);
    }

    #[test]
    fn test_splitter_builder_pattern() {
        setup();
        let splitter = Splitter::new(Orientation::Vertical)
            .with_handle_width(8.0)
            .with_default_minimum_size(50)
            .with_handle_color(Color::from_rgb8(100, 100, 100));

        assert_eq!(splitter.orientation(), Orientation::Vertical);
        assert_eq!(splitter.handle_width(), 8.0);
        assert_eq!(splitter.default_minimum_size(), 50);
        assert_eq!(splitter.handle_color(), Color::from_rgb8(100, 100, 100));
    }

    #[test]
    fn test_add_widgets() {
        setup();
        let mut splitter = Splitter::new(Orientation::Horizontal);

        let pane1 = MockWidget::new();
        let pane1_id = pane1.object_id();
        let pane2 = MockWidget::new();
        let pane2_id = pane2.object_id();

        let idx0 = splitter.add_widget(pane1_id);
        assert_eq!(idx0, 0);
        assert_eq!(splitter.count(), 1);
        assert_eq!(splitter.widget(0), Some(pane1_id));

        let idx1 = splitter.add_widget(pane2_id);
        assert_eq!(idx1, 1);
        assert_eq!(splitter.count(), 2);
        assert_eq!(splitter.widget(1), Some(pane2_id));
    }

    #[test]
    fn test_remove_widget() {
        setup();
        let mut splitter = Splitter::new(Orientation::Horizontal);

        let pane1 = MockWidget::new();
        let pane1_id = pane1.object_id();
        let pane2 = MockWidget::new();
        let pane2_id = pane2.object_id();

        splitter.add_widget(pane1_id);
        splitter.add_widget(pane2_id);

        let removed = splitter.remove_widget(0);
        assert_eq!(removed, Some(pane1_id));
        assert_eq!(splitter.count(), 1);
        assert_eq!(splitter.widget(0), Some(pane2_id));
    }

    #[test]
    fn test_set_sizes() {
        setup();
        let mut splitter = Splitter::new(Orientation::Horizontal);

        let pane1 = MockWidget::new();
        let pane1_id = pane1.object_id();
        let pane2 = MockWidget::new();
        let pane2_id = pane2.object_id();

        splitter.add_widget(pane1_id);
        splitter.add_widget(pane2_id);

        splitter.set_sizes(vec![200, 300]);
        let sizes = splitter.sizes();
        assert_eq!(sizes.len(), 2);
        assert_eq!(sizes[0], 200);
        assert_eq!(sizes[1], 300);
    }

    #[test]
    fn test_minimum_sizes() {
        setup();
        let mut splitter = Splitter::new(Orientation::Horizontal);

        let pane1 = MockWidget::new();
        let pane1_id = pane1.object_id();

        splitter.add_widget(pane1_id);
        splitter.set_minimum_size(0, 100);

        assert_eq!(splitter.minimum_size(0), 100);

        // Setting size below minimum should clamp
        splitter.set_sizes(vec![50]);
        assert_eq!(splitter.size(0), Some(100));
    }

    #[test]
    fn test_collapsible() {
        setup();
        let mut splitter = Splitter::new(Orientation::Horizontal);

        let pane1 = MockWidget::new();
        let pane1_id = pane1.object_id();
        let pane2 = MockWidget::new();
        let pane2_id = pane2.object_id();

        splitter.add_widget(pane1_id);
        splitter.add_widget(pane2_id);

        assert!(!splitter.is_collapsible(0));
        splitter.set_collapsible(0, true);
        assert!(splitter.is_collapsible(0));

        splitter.set_sizes(vec![200, 300]);
        assert!(!splitter.is_collapsed(0));

        // Collapse pane 0
        assert!(splitter.collapse(0));
        assert!(splitter.is_collapsed(0));
        assert_eq!(splitter.size(0), Some(0));

        // Restore pane 0
        assert!(splitter.restore(0));
        assert!(!splitter.is_collapsed(0));
        assert!(splitter.size(0).unwrap() > 0);
    }

    #[test]
    fn test_save_restore_state() {
        setup();
        let mut splitter = Splitter::new(Orientation::Horizontal);

        let pane1 = MockWidget::new();
        let pane1_id = pane1.object_id();
        let pane2 = MockWidget::new();
        let pane2_id = pane2.object_id();
        let pane3 = MockWidget::new();
        let pane3_id = pane3.object_id();

        splitter.add_widget(pane1_id);
        splitter.add_widget(pane2_id);
        splitter.add_widget(pane3_id);

        splitter.set_sizes(vec![100, 200, 300]);

        let state = splitter.save_state();
        assert_eq!(state, "100,200,300");

        // Change sizes and restore
        splitter.set_sizes(vec![50, 50, 50]);
        assert!(splitter.restore_state(&state));

        let sizes = splitter.sizes();
        assert_eq!(sizes[0], 100);
        assert_eq!(sizes[1], 200);
        assert_eq!(sizes[2], 300);
    }

    #[test]
    fn test_index_of() {
        setup();
        let mut splitter = Splitter::new(Orientation::Horizontal);

        let pane1 = MockWidget::new();
        let pane1_id = pane1.object_id();
        let pane2 = MockWidget::new();
        let pane2_id = pane2.object_id();
        let pane3 = MockWidget::new();
        let pane3_id = pane3.object_id();

        splitter.add_widget(pane1_id);
        splitter.add_widget(pane2_id);

        assert_eq!(splitter.index_of(pane1_id), Some(0));
        assert_eq!(splitter.index_of(pane2_id), Some(1));
        assert_eq!(splitter.index_of(pane3_id), None);
    }

    #[test]
    fn test_insert_widget() {
        setup();
        let mut splitter = Splitter::new(Orientation::Horizontal);

        let pane1 = MockWidget::new();
        let pane1_id = pane1.object_id();
        let pane2 = MockWidget::new();
        let pane2_id = pane2.object_id();
        let pane3 = MockWidget::new();
        let pane3_id = pane3.object_id();

        splitter.add_widget(pane1_id);
        splitter.add_widget(pane3_id);

        // Insert in the middle
        let idx = splitter.insert_widget(1, pane2_id);
        assert_eq!(idx, 1);
        assert_eq!(splitter.count(), 3);
        assert_eq!(splitter.widget(0), Some(pane1_id));
        assert_eq!(splitter.widget(1), Some(pane2_id));
        assert_eq!(splitter.widget(2), Some(pane3_id));
    }
}
