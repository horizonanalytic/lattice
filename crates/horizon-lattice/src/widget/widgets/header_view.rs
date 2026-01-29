//! HeaderView widget for displaying section headers.
//!
//! This module provides [`HeaderView`], a widget that displays horizontal or
//! vertical headers for table and tree views. It supports:
//!
//! - Section resizing (interactive or automatic)
//! - Section reordering via drag
//! - Section hiding
//! - Sort indicator display
//! - Click-to-sort functionality
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{HeaderView, ResizeMode, SortOrder};
//! use horizon_lattice::model::Orientation;
//!
//! let mut header = HeaderView::new(Orientation::Horizontal);
//! header.set_section_count(5);
//! header.set_section_size(0, 150.0);
//! header.set_resize_mode(0, ResizeMode::Stretch);
//!
//! header.section_clicked.connect(|section| {
//!     println!("Header section {} clicked", section);
//! });
//! ```

use std::sync::Arc;

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Stroke};

use crate::model::{ItemModel, ItemRole, Orientation};
use crate::widget::{
    ContextMenuEvent, FocusPolicy, MouseButton, MouseMoveEvent, MousePressEvent, MouseReleaseEvent,
    PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

/// Resize mode for header sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizeMode {
    /// User can resize section by dragging the edge.
    #[default]
    Interactive,
    /// Section has a fixed size.
    Fixed,
    /// Section stretches to fill available space.
    Stretch,
    /// Section automatically sizes to fit its contents.
    ResizeToContents,
}

/// Sort order for header sort indicator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// Ascending order (A-Z, 0-9).
    #[default]
    Ascending,
    /// Descending order (Z-A, 9-0).
    Descending,
}

const DEFAULT_SECTION_SIZE: f32 = 100.0;
const MINIMUM_SECTION_SIZE: f32 = 20.0;
const HEADER_HEIGHT: f32 = 24.0;
const RESIZE_HANDLE_WIDTH: f32 = 5.0;

/// A header view widget for table and tree views.
///
/// HeaderView displays section headers that can be resized, reordered, hidden,
/// and clicked. It supports both horizontal (column) and vertical (row) headers.
///
/// # Signals
///
/// - `section_clicked(usize)`: Emitted when a section is clicked
/// - `section_double_clicked(usize)`: Emitted when a section is double-clicked
/// - `section_resized((usize, f32, f32))`: Emitted when a section is resized (section, old, new)
/// - `section_moved((usize, usize, usize))`: Emitted when a section is moved (logical, old_visual, new_visual)
/// - `sort_indicator_changed((usize, SortOrder))`: Emitted when sort indicator changes
pub struct HeaderView {
    base: WidgetBase,

    /// Header orientation (horizontal for columns, vertical for rows).
    orientation: Orientation,

    /// Number of sections.
    section_count: usize,

    /// Size of each section (width for horizontal, height for vertical).
    section_sizes: Vec<f32>,

    /// Cached cumulative positions for fast lookup.
    section_positions: Vec<f32>,

    /// Resize mode for each section.
    section_resize_modes: Vec<ResizeMode>,

    /// Which sections are hidden.
    section_hidden: Vec<bool>,

    /// Maps logical index to visual index.
    section_visual_indices: Vec<usize>,

    /// Maps visual index to logical index.
    section_logical_indices: Vec<usize>,

    /// Default size for new sections.
    default_section_size: f32,

    /// Minimum allowed section size.
    minimum_section_size: f32,

    /// Whether the last section stretches to fill remaining space.
    stretch_last_section: bool,

    /// Section currently being resized (logical index).
    resize_section: Option<usize>,
    resize_start_pos: f32,
    resize_start_size: f32,

    /// Section currently being moved (logical index).
    move_section: Option<usize>,
    move_start_pos: f32,
    move_target_visual: Option<usize>,

    /// Whether sections can be moved by dragging.
    sections_movable: bool,

    /// Currently hovered section (logical index).
    hover_section: Option<usize>,

    /// Currently pressed section (logical index).
    pressed_section: Option<usize>,

    /// Section showing sort indicator (logical index).
    sort_indicator_section: Option<usize>,

    /// Sort order for the indicator.
    sort_indicator_order: SortOrder,

    /// Whether to show sort indicator.
    sort_indicator_shown: bool,

    /// Scroll offset (synced with parent view).
    offset: i32,

    /// Header height (for horizontal) or width (for vertical).
    header_size: f32,

    /// Background color.
    background_color: Color,

    /// Border color.
    border_color: Color,

    /// Text color.
    text_color: Color,

    /// Highlight color for hover/press.
    highlight_color: Color,

    /// Connected model for header text.
    model: Option<Arc<dyn ItemModel>>,

    /// Emitted when a section header is clicked.
    pub section_clicked: Signal<usize>,

    /// Emitted when a section header is double-clicked.
    pub section_double_clicked: Signal<usize>,

    /// Emitted when a section is resized.
    pub section_resized: Signal<(usize, f32, f32)>,

    /// Emitted when a section is moved.
    pub section_moved: Signal<(usize, usize, usize)>,

    /// Emitted when sort indicator changes.
    pub sort_indicator_changed: Signal<(usize, SortOrder)>,

    /// Emitted when a context menu is requested on a section.
    ///
    /// The tuple contains (section logical index or None, position in widget coords).
    /// If the context menu was requested over a section, the index will be Some.
    /// If requested over empty space, the index will be None.
    pub context_menu_requested: Signal<(Option<usize>, Point)>,
}

impl Default for HeaderView {
    fn default() -> Self {
        Self::new(Orientation::Horizontal)
    }
}

impl HeaderView {
    /// Creates a new HeaderView with the specified orientation.
    pub fn new(orientation: Orientation) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::NoFocus);

        let size_policy = match orientation {
            Orientation::Horizontal => {
                SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Fixed)
            }
            Orientation::Vertical => SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Expanding),
        };
        base.set_size_policy(size_policy);

        Self {
            base,
            orientation,
            section_count: 0,
            section_sizes: Vec::new(),
            section_positions: Vec::new(),
            section_resize_modes: Vec::new(),
            section_hidden: Vec::new(),
            section_visual_indices: Vec::new(),
            section_logical_indices: Vec::new(),
            default_section_size: DEFAULT_SECTION_SIZE,
            minimum_section_size: MINIMUM_SECTION_SIZE,
            stretch_last_section: false,
            resize_section: None,
            resize_start_pos: 0.0,
            resize_start_size: 0.0,
            move_section: None,
            move_start_pos: 0.0,
            move_target_visual: None,
            sections_movable: false,
            hover_section: None,
            pressed_section: None,
            sort_indicator_section: None,
            sort_indicator_order: SortOrder::Ascending,
            sort_indicator_shown: true,
            offset: 0,
            header_size: HEADER_HEIGHT,
            background_color: Color::from_rgb8(240, 240, 240),
            border_color: Color::from_rgb8(200, 200, 200),
            text_color: Color::BLACK,
            highlight_color: Color::from_rgb8(220, 220, 220),
            model: None,
            section_clicked: Signal::new(),
            section_double_clicked: Signal::new(),
            section_resized: Signal::new(),
            section_moved: Signal::new(),
            sort_indicator_changed: Signal::new(),
            context_menu_requested: Signal::new(),
        }
    }

    /// Sets the connected model for header text.
    pub fn with_model(mut self, model: Arc<dyn ItemModel>) -> Self {
        self.model = Some(model);
        self
    }

    /// Sets the connected model for header text.
    pub fn set_model(&mut self, model: Option<Arc<dyn ItemModel>>) {
        self.model = model;
        self.base.update();
    }

    // =========================================================================
    // Section Count
    // =========================================================================

    /// Returns the number of sections.
    pub fn section_count(&self) -> usize {
        self.section_count
    }

    /// Sets the number of sections.
    pub fn set_section_count(&mut self, count: usize) {
        if count == self.section_count {
            return;
        }

        self.section_count = count;

        // Resize vectors
        self.section_sizes.resize(count, self.default_section_size);
        self.section_resize_modes
            .resize(count, ResizeMode::Interactive);
        self.section_hidden.resize(count, false);

        // Reset visual/logical mappings to identity
        self.section_visual_indices = (0..count).collect();
        self.section_logical_indices = (0..count).collect();

        self.update_section_positions();
        self.base.update();
    }

    // =========================================================================
    // Section Sizes
    // =========================================================================

    /// Returns the size of a section (width for horizontal, height for vertical).
    pub fn section_size(&self, logical_index: usize) -> f32 {
        self.section_sizes
            .get(logical_index)
            .copied()
            .unwrap_or(self.default_section_size)
    }

    /// Sets the size of a section.
    pub fn set_section_size(&mut self, logical_index: usize, size: f32) {
        if logical_index >= self.section_count {
            return;
        }

        let clamped_size = size.max(self.minimum_section_size);
        let old_size = self.section_sizes[logical_index];

        if (clamped_size - old_size).abs() > 0.5 {
            self.section_sizes[logical_index] = clamped_size;
            self.update_section_positions();
            self.section_resized
                .emit((logical_index, old_size, clamped_size));
            self.base.update();
        }
    }

    /// Resizes a section (alias for set_section_size).
    pub fn resize_section(&mut self, logical_index: usize, size: f32) {
        self.set_section_size(logical_index, size);
    }

    /// Returns the position of a section's start edge.
    pub fn section_position(&self, logical_index: usize) -> f32 {
        if logical_index >= self.section_count {
            return 0.0;
        }

        let visual_index = self.visual_index(logical_index);
        self.section_positions
            .get(visual_index)
            .copied()
            .unwrap_or(0.0)
    }

    /// Returns the logical index of the section at the given position.
    pub fn section_at(&self, position: f32) -> Option<usize> {
        let adjusted_pos = position + self.offset as f32;

        for visual in 0..self.section_count {
            let logical = self.logical_index(visual);
            if self.section_hidden.get(logical).copied().unwrap_or(false) {
                continue;
            }

            let start = self.section_positions.get(visual).copied().unwrap_or(0.0);
            let size = self.section_sizes.get(logical).copied().unwrap_or(0.0);
            let end = start + size;

            if adjusted_pos >= start && adjusted_pos < end {
                return Some(logical);
            }
        }

        None
    }

    /// Returns the default section size.
    pub fn default_section_size(&self) -> f32 {
        self.default_section_size
    }

    /// Sets the default section size for new sections.
    pub fn set_default_section_size(&mut self, size: f32) {
        self.default_section_size = size.max(self.minimum_section_size);
    }

    /// Returns the minimum section size.
    pub fn minimum_section_size(&self) -> f32 {
        self.minimum_section_size
    }

    /// Sets the minimum section size.
    pub fn set_minimum_section_size(&mut self, size: f32) {
        self.minimum_section_size = size.max(1.0);
    }

    // =========================================================================
    // Section Visibility
    // =========================================================================

    /// Returns whether a section is hidden.
    pub fn is_section_hidden(&self, logical_index: usize) -> bool {
        self.section_hidden
            .get(logical_index)
            .copied()
            .unwrap_or(false)
    }

    /// Sets whether a section is hidden.
    pub fn set_section_hidden(&mut self, logical_index: usize, hidden: bool) {
        if logical_index >= self.section_count {
            return;
        }

        if self.section_hidden[logical_index] != hidden {
            self.section_hidden[logical_index] = hidden;
            self.update_section_positions();
            self.base.update();
        }
    }

    /// Shows a hidden section.
    pub fn show_section(&mut self, logical_index: usize) {
        self.set_section_hidden(logical_index, false);
    }

    /// Hides a section.
    pub fn hide_section(&mut self, logical_index: usize) {
        self.set_section_hidden(logical_index, true);
    }

    /// Returns the number of hidden sections.
    pub fn hidden_section_count(&self) -> usize {
        self.section_hidden.iter().filter(|&&h| h).count()
    }

    // =========================================================================
    // Section Ordering (Visual vs Logical)
    // =========================================================================

    /// Returns the visual index for a logical index.
    pub fn visual_index(&self, logical_index: usize) -> usize {
        self.section_visual_indices
            .get(logical_index)
            .copied()
            .unwrap_or(logical_index)
    }

    /// Returns the logical index for a visual index.
    pub fn logical_index(&self, visual_index: usize) -> usize {
        self.section_logical_indices
            .get(visual_index)
            .copied()
            .unwrap_or(visual_index)
    }

    /// Moves a section from one visual position to another.
    pub fn move_section(&mut self, from_visual: usize, to_visual: usize) {
        if from_visual >= self.section_count || to_visual >= self.section_count {
            return;
        }
        if from_visual == to_visual {
            return;
        }

        let logical = self.section_logical_indices[from_visual];

        // Remove from old position
        self.section_logical_indices.remove(from_visual);

        // Insert at new position
        let insert_pos = if to_visual > from_visual {
            to_visual
        } else {
            to_visual
        };
        self.section_logical_indices
            .insert(insert_pos.min(self.section_logical_indices.len()), logical);

        // Rebuild visual indices
        for (visual, &log) in self.section_logical_indices.iter().enumerate() {
            if log < self.section_visual_indices.len() {
                self.section_visual_indices[log] = visual;
            }
        }

        self.update_section_positions();
        self.section_moved.emit((logical, from_visual, to_visual));
        self.base.update();
    }

    /// Swaps two sections by their logical indices.
    pub fn swap_sections(&mut self, first: usize, second: usize) {
        if first >= self.section_count || second >= self.section_count {
            return;
        }
        if first == second {
            return;
        }

        let first_visual = self.visual_index(first);
        let second_visual = self.visual_index(second);

        // Swap in logical indices array
        self.section_logical_indices[first_visual] = second;
        self.section_logical_indices[second_visual] = first;

        // Update visual indices
        self.section_visual_indices[first] = second_visual;
        self.section_visual_indices[second] = first_visual;

        self.update_section_positions();
        self.base.update();
    }

    /// Returns whether sections can be moved by dragging.
    pub fn sections_movable(&self) -> bool {
        self.sections_movable
    }

    /// Sets whether sections can be moved by dragging.
    pub fn set_sections_movable(&mut self, movable: bool) {
        self.sections_movable = movable;
    }

    // =========================================================================
    // Resize Modes
    // =========================================================================

    /// Returns the resize mode for a section.
    pub fn resize_mode(&self, logical_index: usize) -> ResizeMode {
        self.section_resize_modes
            .get(logical_index)
            .copied()
            .unwrap_or(ResizeMode::Interactive)
    }

    /// Sets the resize mode for a section.
    pub fn set_resize_mode(&mut self, logical_index: usize, mode: ResizeMode) {
        if logical_index >= self.section_count {
            return;
        }
        self.section_resize_modes[logical_index] = mode;
    }

    /// Sets the resize mode for all sections.
    pub fn set_default_resize_mode(&mut self, mode: ResizeMode) {
        for m in &mut self.section_resize_modes {
            *m = mode;
        }
    }

    /// Returns whether the last section stretches to fill remaining space.
    pub fn stretch_last_section(&self) -> bool {
        self.stretch_last_section
    }

    /// Sets whether the last section stretches to fill remaining space.
    pub fn set_stretch_last_section(&mut self, stretch: bool) {
        if self.stretch_last_section != stretch {
            self.stretch_last_section = stretch;
            self.update_section_positions();
            self.base.update();
        }
    }

    // =========================================================================
    // Sort Indicator
    // =========================================================================

    /// Returns the section showing the sort indicator, if any.
    pub fn sort_indicator_section(&self) -> Option<usize> {
        if self.sort_indicator_shown {
            self.sort_indicator_section
        } else {
            None
        }
    }

    /// Returns the sort order.
    pub fn sort_indicator_order(&self) -> SortOrder {
        self.sort_indicator_order
    }

    /// Sets the sort indicator on a section.
    pub fn set_sort_indicator(&mut self, section: usize, order: SortOrder) {
        let changed =
            self.sort_indicator_section != Some(section) || self.sort_indicator_order != order;

        self.sort_indicator_section = Some(section);
        self.sort_indicator_order = order;

        if changed {
            self.sort_indicator_changed.emit((section, order));
            self.base.update();
        }
    }

    /// Clears the sort indicator.
    pub fn clear_sort_indicator(&mut self) {
        if self.sort_indicator_section.is_some() {
            self.sort_indicator_section = None;
            self.base.update();
        }
    }

    /// Returns whether the sort indicator is shown.
    pub fn sort_indicator_shown(&self) -> bool {
        self.sort_indicator_shown
    }

    /// Sets whether to show the sort indicator.
    pub fn set_sort_indicator_shown(&mut self, shown: bool) {
        if self.sort_indicator_shown != shown {
            self.sort_indicator_shown = shown;
            self.base.update();
        }
    }

    // =========================================================================
    // Scrolling
    // =========================================================================

    /// Returns the current scroll offset.
    pub fn offset(&self) -> i32 {
        self.offset
    }

    /// Sets the scroll offset (called by parent view).
    pub fn set_offset(&mut self, offset: i32) {
        if self.offset != offset {
            self.offset = offset;
            self.base.update();
        }
    }

    // =========================================================================
    // Size
    // =========================================================================

    /// Returns the total size of all visible sections.
    pub fn total_size(&self) -> f32 {
        let mut total = 0.0;
        for (logical, &size) in self.section_sizes.iter().enumerate() {
            if !self.is_section_hidden(logical) {
                total += size;
            }
        }
        total
    }

    /// Returns the header height (for horizontal) or width (for vertical).
    pub fn header_size(&self) -> f32 {
        self.header_size
    }

    /// Sets the header height (for horizontal) or width (for vertical).
    pub fn set_header_size(&mut self, size: f32) {
        if self.header_size != size {
            self.header_size = size;
            self.base.update();
        }
    }

    // =========================================================================
    // Internal Layout
    // =========================================================================

    fn update_section_positions(&mut self) {
        self.section_positions.clear();
        self.section_positions.reserve(self.section_count);

        let mut pos = 0.0;
        for visual in 0..self.section_count {
            let logical = self.logical_index(visual);
            self.section_positions.push(pos);

            if !self.is_section_hidden(logical) {
                pos += self.section_sizes.get(logical).copied().unwrap_or(0.0);
            }
        }
    }

    fn resize_handle_at(&self, pos: f32) -> Option<usize> {
        let adjusted_pos = pos + self.offset as f32;

        for visual in 0..self.section_count {
            let logical = self.logical_index(visual);
            if self.is_section_hidden(logical) {
                continue;
            }

            let start = self.section_positions.get(visual).copied().unwrap_or(0.0);
            let size = self.section_sizes.get(logical).copied().unwrap_or(0.0);
            let end = start + size;

            // Check if near the right edge (for horizontal) or bottom edge (for vertical)
            if (adjusted_pos - end).abs() < RESIZE_HANDLE_WIDTH {
                // Check if this section can be resized
                let mode = self.resize_mode(logical);
                if mode == ResizeMode::Interactive {
                    return Some(logical);
                }
            }
        }

        None
    }

    fn section_rect(&self, logical_index: usize) -> Rect {
        let visual = self.visual_index(logical_index);
        let start = self.section_positions.get(visual).copied().unwrap_or(0.0) - self.offset as f32;
        let size = self
            .section_sizes
            .get(logical_index)
            .copied()
            .unwrap_or(0.0);

        match self.orientation {
            Orientation::Horizontal => Rect::new(start, 0.0, size, self.header_size),
            Orientation::Vertical => Rect::new(0.0, start, self.header_size, size),
        }
    }

    fn header_text(&self, logical_index: usize) -> String {
        if let Some(model) = &self.model
            && let Some(text) = model
                .header_data(logical_index, self.orientation, ItemRole::Display)
                .as_string()
        {
            return text.to_string();
        }

        // Default: column/row number
        match self.orientation {
            Orientation::Horizontal => {
                // Column letters like Excel (A, B, C, ... Z, AA, AB, ...)
                column_to_letter(logical_index)
            }
            Orientation::Vertical => (logical_index + 1).to_string(),
        }
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_header(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();

        // Background
        ctx.renderer().fill_rect(rect, self.background_color);

        // Draw each visible section
        for visual in 0..self.section_count {
            let logical = self.logical_index(visual);
            if self.is_section_hidden(logical) {
                continue;
            }

            let section_rect = self.section_rect(logical);

            // Skip if off-screen
            if match self.orientation {
                Orientation::Horizontal => {
                    section_rect.origin.x + section_rect.width() < 0.0
                        || section_rect.origin.x > rect.width()
                }
                Orientation::Vertical => {
                    section_rect.origin.y + section_rect.height() < 0.0
                        || section_rect.origin.y > rect.height()
                }
            } {
                continue;
            }

            self.paint_section(ctx, logical, section_rect);
        }

        // Border at bottom (horizontal) or right (vertical)
        match self.orientation {
            Orientation::Horizontal => {
                ctx.renderer().fill_rect(
                    Rect::new(0.0, self.header_size - 1.0, rect.width(), 1.0),
                    self.border_color,
                );
            }
            Orientation::Vertical => {
                ctx.renderer().fill_rect(
                    Rect::new(self.header_size - 1.0, 0.0, 1.0, rect.height()),
                    self.border_color,
                );
            }
        }
    }

    fn paint_section(&self, ctx: &mut PaintContext<'_>, logical_index: usize, rect: Rect) {
        // Section background (highlight if hovered or pressed)
        let bg_color = if self.pressed_section == Some(logical_index) {
            Color::from_rgb8(200, 200, 200)
        } else if self.hover_section == Some(logical_index) {
            self.highlight_color
        } else {
            self.background_color
        };

        ctx.renderer().fill_rect(rect, bg_color);

        // Right/bottom border
        match self.orientation {
            Orientation::Horizontal => {
                ctx.renderer().fill_rect(
                    Rect::new(
                        rect.origin.x + rect.width() - 1.0,
                        rect.origin.y,
                        1.0,
                        rect.height(),
                    ),
                    self.border_color,
                );
            }
            Orientation::Vertical => {
                ctx.renderer().fill_rect(
                    Rect::new(
                        rect.origin.x,
                        rect.origin.y + rect.height() - 1.0,
                        rect.width(),
                        1.0,
                    ),
                    self.border_color,
                );
            }
        }

        // Header text
        let text = self.header_text(logical_index);
        let padding = 4.0;
        let text_rect = Rect::new(
            rect.origin.x + padding,
            rect.origin.y + padding,
            rect.width() - padding * 2.0 - 12.0, // Reserve space for sort indicator
            rect.height() - padding * 2.0,
        );

        // For now, we'll draw text using a simple approach
        // In a full implementation, we'd use TextLayout and TextRenderer
        // For headers, we can indicate text area with a subtle background
        let _ = text; // Text will be rendered when full text rendering is integrated
        let _ = text_rect;

        // Sort indicator
        if self.sort_indicator_shown && self.sort_indicator_section == Some(logical_index) {
            self.paint_sort_indicator(ctx, rect, self.sort_indicator_order);
        }
    }

    fn paint_sort_indicator(&self, ctx: &mut PaintContext<'_>, rect: Rect, order: SortOrder) {
        let indicator_size = 6.0;
        let x = rect.origin.x + rect.width() - indicator_size - 6.0;
        let y = rect.origin.y + (rect.height() - indicator_size) / 2.0;
        let stroke = Stroke::new(self.text_color, 1.5);

        match order {
            SortOrder::Ascending => {
                // Triangle pointing up (two lines forming a chevron)
                let p1 = Point::new(x, y + indicator_size);
                let p2 = Point::new(x + indicator_size / 2.0, y);
                let p3 = Point::new(x + indicator_size, y + indicator_size);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
            SortOrder::Descending => {
                // Triangle pointing down (two lines forming a chevron)
                let p1 = Point::new(x, y);
                let p2 = Point::new(x + indicator_size / 2.0, y + indicator_size);
                let p3 = Point::new(x + indicator_size, y);
                ctx.renderer().draw_line(p1, p2, &stroke);
                ctx.renderer().draw_line(p2, p3, &stroke);
            }
        }
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = match self.orientation {
            Orientation::Horizontal => event.local_pos.x,
            Orientation::Vertical => event.local_pos.y,
        };

        // Check for resize handle first
        if let Some(logical) = self.resize_handle_at(pos) {
            self.resize_section = Some(logical);
            self.resize_start_pos = pos;
            self.resize_start_size = self.section_size(logical);
            return true;
        }

        // Check for section click
        if let Some(logical) = self.section_at(pos) {
            self.pressed_section = Some(logical);

            if self.sections_movable {
                self.move_section = Some(logical);
                self.move_start_pos = pos;
            }

            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = match self.orientation {
            Orientation::Horizontal => event.local_pos.x,
            Orientation::Vertical => event.local_pos.y,
        };

        // End resize
        if self.resize_section.is_some() {
            self.resize_section = None;
            return true;
        }

        // End move
        if let Some(moving) = self.move_section.take()
            && let Some(target_visual) = self.move_target_visual.take()
        {
            let from_visual = self.visual_index(moving);
            if from_visual != target_visual {
                self.move_section(from_visual, target_visual);
            }
        }

        // Emit click if released on same section
        if let Some(pressed) = self.pressed_section.take() {
            if let Some(current) = self.section_at(pos)
                && current == pressed
            {
                self.section_clicked.emit(pressed);
            }
            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = match self.orientation {
            Orientation::Horizontal => event.local_pos.x,
            Orientation::Vertical => event.local_pos.y,
        };

        // Handle resize drag
        if let Some(logical) = self.resize_section {
            let delta = pos - self.resize_start_pos;
            let new_size = (self.resize_start_size + delta).max(self.minimum_section_size);
            self.set_section_size(logical, new_size);
            return true;
        }

        // Handle section move drag
        if let Some(_moving) = self.move_section {
            // Calculate target position based on mouse position
            if let Some(target_logical) = self.section_at(pos) {
                self.move_target_visual = Some(self.visual_index(target_logical));
            }
            self.base.update();
            return true;
        }

        // Update hover state
        let old_hover = self.hover_section;
        self.hover_section = self.section_at(pos);

        // Note: Cursor shape changes would be handled here in a full implementation
        // For now, we just track the hover state
        let _ = self.resize_handle_at(pos);

        if old_hover != self.hover_section {
            self.base.update();
        }

        false
    }

    fn handle_context_menu(&mut self, event: &ContextMenuEvent) -> bool {
        let pos = match self.orientation {
            Orientation::Horizontal => event.local_pos.x + self.offset as f32,
            Orientation::Vertical => event.local_pos.y + self.offset as f32,
        };

        // Find the section at the context menu position
        let section = self.section_at(pos);

        // Emit the context_menu_requested signal with the section and position
        self.context_menu_requested.emit((section, event.local_pos));

        true
    }
}

impl Object for HeaderView {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for HeaderView {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        match self.orientation {
            Orientation::Horizontal => {
                SizeHint::from_dimensions(self.total_size(), self.header_size)
                    .with_minimum_dimensions(0.0, self.header_size)
            }
            Orientation::Vertical => SizeHint::from_dimensions(self.header_size, self.total_size())
                .with_minimum_dimensions(self.header_size, 0.0),
        }
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        self.paint_header(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::ContextMenu(e) => self.handle_context_menu(e),
            _ => false,
        }
    }
}

/// Converts a column index to Excel-style letter (A, B, ... Z, AA, AB, ...).
fn column_to_letter(index: usize) -> String {
    let mut result = String::new();
    let mut n = index + 1;

    while n > 0 {
        n -= 1;
        let c = (b'A' + (n % 26) as u8) as char;
        result.insert(0, c);
        n /= 26;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_view_creation() {
        let header = HeaderView::new(Orientation::Horizontal);
        assert_eq!(header.section_count(), 0);
        assert_eq!(header.orientation, Orientation::Horizontal);
    }

    #[test]
    fn test_section_count() {
        let mut header = HeaderView::new(Orientation::Horizontal);
        header.set_section_count(5);
        assert_eq!(header.section_count(), 5);
        assert_eq!(header.section_sizes.len(), 5);
    }

    #[test]
    fn test_section_sizes() {
        let mut header = HeaderView::new(Orientation::Horizontal);
        header.set_section_count(3);

        header.set_section_size(1, 200.0);
        assert_eq!(header.section_size(1), 200.0);

        // Test minimum size enforcement
        header.set_section_size(1, 5.0);
        assert_eq!(header.section_size(1), header.minimum_section_size());
    }

    #[test]
    fn test_section_visibility() {
        let mut header = HeaderView::new(Orientation::Horizontal);
        header.set_section_count(3);

        assert!(!header.is_section_hidden(1));
        header.set_section_hidden(1, true);
        assert!(header.is_section_hidden(1));
        assert_eq!(header.hidden_section_count(), 1);

        header.show_section(1);
        assert!(!header.is_section_hidden(1));
    }

    #[test]
    fn test_visual_logical_mapping() {
        let mut header = HeaderView::new(Orientation::Horizontal);
        header.set_section_count(3);

        // Initially, visual == logical
        assert_eq!(header.visual_index(0), 0);
        assert_eq!(header.logical_index(0), 0);

        // Move section 0 to visual position 2
        header.move_section(0, 2);
        assert_eq!(header.logical_index(2), 0);
    }

    #[test]
    fn test_sort_indicator() {
        let mut header = HeaderView::new(Orientation::Horizontal);
        header.set_section_count(3);

        assert!(header.sort_indicator_section().is_none());

        header.set_sort_indicator(1, SortOrder::Descending);
        assert_eq!(header.sort_indicator_section(), Some(1));
        assert_eq!(header.sort_indicator_order(), SortOrder::Descending);

        header.clear_sort_indicator();
        assert!(header.sort_indicator_section().is_none());
    }

    #[test]
    fn test_column_to_letter() {
        assert_eq!(column_to_letter(0), "A");
        assert_eq!(column_to_letter(1), "B");
        assert_eq!(column_to_letter(25), "Z");
        assert_eq!(column_to_letter(26), "AA");
        assert_eq!(column_to_letter(27), "AB");
        assert_eq!(column_to_letter(51), "AZ");
        assert_eq!(column_to_letter(52), "BA");
    }

    #[test]
    fn test_context_menu_signal() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};

        let header = HeaderView::new(Orientation::Horizontal);
        let signal_received = Arc::new(AtomicBool::new(false));
        let received_clone = signal_received.clone();

        // Connect to the context menu signal
        header.context_menu_requested.connect(move |_| {
            received_clone.store(true, Ordering::SeqCst);
        });

        // Emit a test signal (simulating what handle_context_menu does)
        header
            .context_menu_requested
            .emit((Some(0), Point::new(10.0, 10.0)));

        assert!(signal_received.load(Ordering::SeqCst));
    }
}
