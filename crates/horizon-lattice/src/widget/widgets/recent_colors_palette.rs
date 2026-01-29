//! Recent colors palette widget implementation.
//!
//! This module provides [`RecentColorsPalette`], a widget that displays a grid of
//! recently used colors with a "More Colors..." action for quick color selection.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::RecentColorsPalette;
//! use horizon_lattice_render::Color;
//!
//! let mut palette = RecentColorsPalette::new();
//!
//! // Add some recent colors
//! palette.add_color(Color::RED);
//! palette.add_color(Color::GREEN);
//! palette.add_color(Color::BLUE);
//!
//! // Connect to color selection
//! palette.color_selected.connect(|&color| {
//!     println!("Selected color: {:?}", color);
//! });
//!
//! // Connect to "More Colors..." click
//! palette.more_colors_requested.connect(|()| {
//!     // Open ColorDialog here
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontSystem, Point, Rect, Renderer, RoundedRect, Size, Stroke, TextLayout,
    TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

// ============================================================================
// Constants
// ============================================================================

/// Default maximum number of colors in the palette.
const DEFAULT_MAX_COLORS: usize = 16;

/// Default swatch size.
const DEFAULT_SWATCH_SIZE: f32 = 20.0;

/// Default gap between swatches.
const DEFAULT_SWATCH_GAP: f32 = 4.0;

/// Default number of columns.
const DEFAULT_COLUMNS: usize = 8;

// ============================================================================
// RecentColorsPalette
// ============================================================================

/// A widget displaying a grid of recent colors for quick selection.
///
/// RecentColorsPalette provides:
/// - A grid of color swatches showing recently used colors
/// - Keyboard navigation for accessibility
/// - A "More Colors..." action to open a full color picker
/// - Configurable layout (columns, swatch size, max colors)
///
/// # Signals
///
/// - `color_selected(Color)`: Emitted when a color swatch is clicked
/// - `more_colors_requested()`: Emitted when "More Colors..." is clicked
pub struct RecentColorsPalette {
    /// Widget base.
    base: WidgetBase,

    /// Recent colors list (most recent first).
    colors: Vec<Color>,

    /// Maximum number of colors to display.
    max_colors: usize,

    /// Number of columns in the grid.
    columns: usize,

    /// Size of each swatch.
    swatch_size: f32,

    /// Gap between swatches.
    swatch_gap: f32,

    /// Padding around the content.
    padding: f32,

    /// Height of the "More Colors..." action area.
    action_height: f32,

    /// Border color for swatches.
    border_color: Color,

    /// Hover border color.
    hover_border_color: Color,

    /// Selected swatch index.
    selected_index: Option<usize>,

    /// Hovered swatch index.
    hovered_index: Option<usize>,

    /// Whether "More Colors..." action is hovered.
    action_hovered: bool,

    /// Whether "More Colors..." action is pressed.
    action_pressed: bool,

    /// Whether to show the "More Colors..." action.
    show_more_colors_action: bool,

    /// Signal emitted when a color is selected from the palette.
    pub color_selected: Signal<Color>,

    /// Signal emitted when "More Colors..." is clicked.
    pub more_colors_requested: Signal<()>,
}

impl RecentColorsPalette {
    /// Create a new recent colors palette.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Preferred,
        ));

        Self {
            base,
            colors: Vec::new(),
            max_colors: DEFAULT_MAX_COLORS,
            columns: DEFAULT_COLUMNS,
            swatch_size: DEFAULT_SWATCH_SIZE,
            swatch_gap: DEFAULT_SWATCH_GAP,
            padding: 8.0,
            action_height: 24.0,
            border_color: Color::from_rgb8(180, 180, 180),
            hover_border_color: Color::from_rgb8(0, 122, 255),
            selected_index: None,
            hovered_index: None,
            action_hovered: false,
            action_pressed: false,
            show_more_colors_action: true,
            color_selected: Signal::new(),
            more_colors_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the maximum number of colors using builder pattern.
    pub fn with_max_colors(mut self, max: usize) -> Self {
        self.max_colors = max;
        self
    }

    /// Set the number of columns using builder pattern.
    pub fn with_columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    /// Set the swatch size using builder pattern.
    pub fn with_swatch_size(mut self, size: f32) -> Self {
        self.swatch_size = size.max(8.0);
        self
    }

    /// Set the initial colors using builder pattern.
    pub fn with_colors(mut self, colors: Vec<Color>) -> Self {
        self.colors = colors;
        if self.colors.len() > self.max_colors {
            self.colors.truncate(self.max_colors);
        }
        self
    }

    /// Set whether to show "More Colors..." action using builder pattern.
    pub fn with_more_colors_action(mut self, show: bool) -> Self {
        self.show_more_colors_action = show;
        self
    }

    // =========================================================================
    // Color Management
    // =========================================================================

    /// Get the current colors in the palette.
    pub fn colors(&self) -> &[Color] {
        &self.colors
    }

    /// Set the colors in the palette.
    pub fn set_colors(&mut self, colors: Vec<Color>) {
        self.colors = colors;
        if self.colors.len() > self.max_colors {
            self.colors.truncate(self.max_colors);
        }
        self.base.update();
    }

    /// Add a color to the palette (at the front).
    ///
    /// If the color already exists, it is moved to the front.
    /// If the palette is full, the oldest color is removed.
    pub fn add_color(&mut self, color: Color) {
        // Remove if already exists
        self.colors.retain(|&c| c != color);
        // Add to front
        self.colors.insert(0, color);
        // Enforce max
        if self.colors.len() > self.max_colors {
            self.colors.pop();
        }
        self.base.update();
    }

    /// Clear all colors from the palette.
    pub fn clear(&mut self) {
        self.colors.clear();
        self.selected_index = None;
        self.hovered_index = None;
        self.base.update();
    }

    /// Get the maximum number of colors.
    pub fn max_colors(&self) -> usize {
        self.max_colors
    }

    /// Set the maximum number of colors.
    pub fn set_max_colors(&mut self, max: usize) {
        self.max_colors = max;
        if self.colors.len() > max {
            self.colors.truncate(max);
        }
        self.base.update();
    }

    // =========================================================================
    // Layout Configuration
    // =========================================================================

    /// Get the number of columns.
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Set the number of columns.
    pub fn set_columns(&mut self, columns: usize) {
        self.columns = columns.max(1);
        self.base.update();
    }

    /// Get the swatch size.
    pub fn swatch_size(&self) -> f32 {
        self.swatch_size
    }

    /// Set the swatch size.
    pub fn set_swatch_size(&mut self, size: f32) {
        self.swatch_size = size.max(8.0);
        self.base.update();
    }

    /// Get whether "More Colors..." action is shown.
    pub fn shows_more_colors_action(&self) -> bool {
        self.show_more_colors_action
    }

    /// Set whether to show "More Colors..." action.
    pub fn set_show_more_colors_action(&mut self, show: bool) {
        if self.show_more_colors_action != show {
            self.show_more_colors_action = show;
            self.base.update();
        }
    }

    // =========================================================================
    // Selection
    // =========================================================================

    /// Get the currently selected index.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Set the selected index.
    pub fn set_selected_index(&mut self, index: Option<usize>) {
        if let Some(i) = index {
            if i < self.colors.len() {
                self.selected_index = Some(i);
            }
        } else {
            self.selected_index = None;
        }
        self.base.update();
    }

    /// Get the selected color, if any.
    pub fn selected_color(&self) -> Option<Color> {
        self.selected_index
            .and_then(|i| self.colors.get(i).copied())
    }

    // =========================================================================
    // Geometry Calculations
    // =========================================================================

    /// Calculate the number of rows needed.
    fn row_count(&self) -> usize {
        if self.colors.is_empty() {
            return 0;
        }
        self.colors.len().div_ceil(self.columns)
    }

    /// Calculate the grid height.
    fn grid_height(&self) -> f32 {
        let rows = self.row_count();
        if rows == 0 {
            0.0
        } else {
            rows as f32 * self.swatch_size + (rows - 1).max(0) as f32 * self.swatch_gap
        }
    }

    /// Calculate the total preferred size.
    fn calculate_preferred_size(&self) -> Size {
        let grid_width = self.columns as f32 * self.swatch_size
            + (self.columns - 1).max(0) as f32 * self.swatch_gap;
        let grid_height = self.grid_height();

        let action_height = if self.show_more_colors_action {
            self.action_height + self.swatch_gap
        } else {
            0.0
        };

        Size::new(
            grid_width + self.padding * 2.0,
            grid_height + action_height + self.padding * 2.0,
        )
    }

    /// Get the rectangle for a swatch at the given index.
    fn swatch_rect(&self, index: usize) -> Option<Rect> {
        if index >= self.colors.len() {
            return None;
        }

        let row = index / self.columns;
        let col = index % self.columns;

        let x = self.padding + col as f32 * (self.swatch_size + self.swatch_gap);
        let y = self.padding + row as f32 * (self.swatch_size + self.swatch_gap);

        Some(Rect::new(x, y, self.swatch_size, self.swatch_size))
    }

    /// Get the rectangle for the "More Colors..." action.
    fn action_rect(&self) -> Option<Rect> {
        if !self.show_more_colors_action {
            return None;
        }

        let grid_width = self.columns as f32 * self.swatch_size
            + (self.columns - 1).max(0) as f32 * self.swatch_gap;
        let y = self.padding + self.grid_height() + self.swatch_gap;

        Some(Rect::new(self.padding, y, grid_width, self.action_height))
    }

    /// Find which swatch (if any) contains the given point.
    ///
    /// Returns the index of the swatch containing the point, or `None` if
    /// the point is not over any swatch.
    pub fn swatch_at_point(&self, point: Point) -> Option<usize> {
        for i in 0..self.colors.len() {
            if let Some(rect) = self.swatch_rect(i)
                && rect.contains(point)
            {
                return Some(i);
            }
        }
        None
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check action area
        if let Some(action_rect) = self.action_rect()
            && action_rect.contains(pos)
        {
            self.action_pressed = true;
            self.base.update();
            return true;
        }

        // Check swatches
        if let Some(index) = self.swatch_at_point(pos) {
            self.selected_index = Some(index);
            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check action release
        if self.action_pressed {
            self.action_pressed = false;
            if let Some(action_rect) = self.action_rect()
                && action_rect.contains(pos)
            {
                self.more_colors_requested.emit(());
                self.base.update();
                return true;
            }
            self.base.update();
            return true;
        }

        // Check swatch release (emit color_selected)
        if let Some(index) = self.swatch_at_point(pos)
            && let Some(&color) = self.colors.get(index)
        {
            self.color_selected.emit(color);
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;
        let mut needs_update = false;

        // Update action hover
        let new_action_hover = self.action_rect().map(|r| r.contains(pos)).unwrap_or(false);
        if new_action_hover != self.action_hovered {
            self.action_hovered = new_action_hover;
            needs_update = true;
        }

        // Update swatch hover
        let new_hover = self.swatch_at_point(pos);
        if new_hover != self.hovered_index {
            self.hovered_index = new_hover;
            needs_update = true;
        }

        if needs_update {
            self.base.update();
        }

        needs_update
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        let current = self.selected_index.unwrap_or(0);
        let count = self.colors.len();

        if count == 0 {
            return false;
        }

        match event.key {
            Key::ArrowLeft => {
                if current > 0 {
                    self.selected_index = Some(current - 1);
                    self.base.update();
                    return true;
                }
            }
            Key::ArrowRight => {
                if current < count - 1 {
                    self.selected_index = Some(current + 1);
                    self.base.update();
                    return true;
                }
            }
            Key::ArrowUp => {
                if current >= self.columns {
                    self.selected_index = Some(current - self.columns);
                    self.base.update();
                    return true;
                }
            }
            Key::ArrowDown => {
                let next = current + self.columns;
                if next < count {
                    self.selected_index = Some(next);
                    self.base.update();
                    return true;
                }
            }
            Key::Enter | Key::Space => {
                if let Some(&color) = self.selected_index.and_then(|i| self.colors.get(i)) {
                    self.color_selected.emit(color);
                    return true;
                }
            }
            Key::Home => {
                self.selected_index = Some(0);
                self.base.update();
                return true;
            }
            Key::End => {
                self.selected_index = Some(count - 1);
                self.base.update();
                return true;
            }
            _ => {}
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_checkerboard(&self, ctx: &mut PaintContext<'_>, rect: Rect) {
        let checker_size = 4.0;
        let light = Color::from_rgb8(255, 255, 255);
        let dark = Color::from_rgb8(200, 200, 200);

        let cols = (rect.width() / checker_size).ceil() as i32;
        let rows = (rect.height() / checker_size).ceil() as i32;

        for row in 0..rows {
            for col in 0..cols {
                let color = if (row + col) % 2 == 0 { light } else { dark };
                let x = rect.left() + col as f32 * checker_size;
                let y = rect.top() + row as f32 * checker_size;
                let w = checker_size.min(rect.right() - x);
                let h = checker_size.min(rect.bottom() - y);
                ctx.renderer().fill_rect(Rect::new(x, y, w, h), color);
            }
        }
    }

    fn paint_swatches(&self, ctx: &mut PaintContext<'_>) {
        for (i, &color) in self.colors.iter().enumerate() {
            let Some(rect) = self.swatch_rect(i) else {
                continue;
            };

            // Paint checkerboard for alpha
            if color.a < 1.0 {
                self.paint_checkerboard(ctx, rect);
            }

            // Paint the color
            let rounded = RoundedRect::new(rect, 2.0);
            ctx.renderer().fill_rounded_rect(rounded, color);

            // Paint border
            let border_color = if self.selected_index == Some(i) || self.hovered_index == Some(i) {
                self.hover_border_color
            } else {
                self.border_color
            };

            let stroke_width = if self.selected_index == Some(i) {
                2.0
            } else {
                1.0
            };

            let stroke = Stroke::new(border_color, stroke_width);
            ctx.renderer().stroke_rounded_rect(rounded, &stroke);
        }
    }

    fn paint_action(&self, ctx: &mut PaintContext<'_>) {
        let Some(rect) = self.action_rect() else {
            return;
        };

        // Background on hover/press
        if self.action_pressed {
            ctx.renderer()
                .fill_rect(rect, Color::from_rgba8(0, 122, 255, 51));
        } else if self.action_hovered {
            ctx.renderer()
                .fill_rect(rect, Color::from_rgba8(0, 122, 255, 26));
        }

        // Draw "More Colors..." text
        let text = "More Colors...";
        let mut font_system = FontSystem::new();
        let font = Font::default();
        let layout = TextLayout::new(&mut font_system, text, &font);

        let text_x = rect.left() + (rect.width() - layout.width()) / 2.0;
        let text_y = rect.top() + (rect.height() - layout.height()) / 2.0;

        let text_color = if self.action_hovered || self.action_pressed {
            Color::from_rgb8(0, 102, 204)
        } else {
            Color::from_rgb8(0, 122, 255)
        };

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                text_color,
            );
        }
    }

    fn paint_empty_state(&self, _ctx: &mut PaintContext<'_>) {
        if !self.colors.is_empty() {
            return;
        }

        // Draw "No recent colors" text
        let rect = self.base.rect();
        let text = "No recent colors";
        let mut font_system = FontSystem::new();
        let font = Font::default();
        let layout = TextLayout::new(&mut font_system, text, &font);

        let text_x = (rect.width() - layout.width()) / 2.0;
        let text_y = self.padding + (self.swatch_size - layout.height()) / 2.0;

        let text_color = Color::from_rgb8(128, 128, 128);

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(
                &mut font_system,
                &layout,
                Point::new(text_x, text_y),
                text_color,
            );
        }
    }
}

impl Default for RecentColorsPalette {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for RecentColorsPalette {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for RecentColorsPalette {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let preferred = self.calculate_preferred_size();
        let min_height = self.padding * 2.0
            + self.swatch_size
            + if self.show_more_colors_action {
                self.action_height + self.swatch_gap
            } else {
                0.0
            };

        SizeHint::new(preferred).with_minimum_dimensions(preferred.width, min_height)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Background
        let rect = ctx.rect();
        ctx.renderer().fill_rect(rect, Color::WHITE);

        // Paint empty state or swatches
        if self.colors.is_empty() {
            self.paint_empty_state(ctx);
        } else {
            self.paint_swatches(ctx);
        }

        // Paint action
        self.paint_action(ctx);

        // Focus indicator
        if self.base.has_focus() {
            let focus_rect = rect.deflate(1.0);
            let stroke = Stroke::new(Color::from_rgba8(66, 133, 244, 128), 2.0);
            ctx.renderer().stroke_rect(focus_rect, &stroke);
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
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
                    return true;
                }
            }
            WidgetEvent::KeyPress(e) => {
                if self.handle_key_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::Leave(_) => {
                self.hovered_index = None;
                self.action_hovered = false;
                self.base.update();
            }
            _ => {}
        }
        false
    }
}

// Thread safety assertion
static_assertions::assert_impl_all!(RecentColorsPalette: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };

    fn setup() {
        let _ = init_global_registry();
    }

    #[test]
    fn test_palette_creation() {
        setup();
        let palette = RecentColorsPalette::new();
        assert!(palette.colors().is_empty());
        assert_eq!(palette.max_colors(), DEFAULT_MAX_COLORS);
        assert_eq!(palette.columns(), DEFAULT_COLUMNS);
    }

    #[test]
    fn test_add_color() {
        setup();
        let mut palette = RecentColorsPalette::new();

        palette.add_color(Color::RED);
        assert_eq!(palette.colors().len(), 1);
        assert_eq!(palette.colors()[0], Color::RED);

        palette.add_color(Color::GREEN);
        assert_eq!(palette.colors().len(), 2);
        // Most recent first
        assert_eq!(palette.colors()[0], Color::GREEN);
        assert_eq!(palette.colors()[1], Color::RED);
    }

    #[test]
    fn test_add_duplicate_color() {
        setup();
        let mut palette = RecentColorsPalette::new();

        palette.add_color(Color::RED);
        palette.add_color(Color::GREEN);
        palette.add_color(Color::RED); // Duplicate

        // Should have only 2 colors, with RED moved to front
        assert_eq!(palette.colors().len(), 2);
        assert_eq!(palette.colors()[0], Color::RED);
        assert_eq!(palette.colors()[1], Color::GREEN);
    }

    #[test]
    fn test_max_colors_limit() {
        setup();
        let mut palette = RecentColorsPalette::new().with_max_colors(3);

        palette.add_color(Color::RED);
        palette.add_color(Color::GREEN);
        palette.add_color(Color::BLUE);
        palette.add_color(Color::WHITE); // Should push out RED

        assert_eq!(palette.colors().len(), 3);
        assert_eq!(palette.colors()[0], Color::WHITE);
        assert_eq!(palette.colors()[2], Color::GREEN); // RED should be gone
    }

    #[test]
    fn test_builder_pattern() {
        setup();
        let palette = RecentColorsPalette::new()
            .with_max_colors(10)
            .with_columns(5)
            .with_swatch_size(24.0)
            .with_more_colors_action(false)
            .with_colors(vec![Color::RED, Color::GREEN]);

        assert_eq!(palette.max_colors(), 10);
        assert_eq!(palette.columns(), 5);
        assert_eq!(palette.swatch_size(), 24.0);
        assert!(!palette.shows_more_colors_action());
        assert_eq!(palette.colors().len(), 2);
    }

    #[test]
    fn test_color_selected_signal() {
        setup();
        let mut palette = RecentColorsPalette::new();
        palette.add_color(Color::RED);
        palette.set_selected_index(Some(0));

        let selected = Arc::new(Mutex::new(Color::TRANSPARENT));
        let selected_clone = selected.clone();

        palette.color_selected.connect(move |color| {
            *selected_clone.lock().unwrap() = *color;
        });

        // Emit via signal directly (simulating click)
        palette.color_selected.emit(Color::RED);

        let result = *selected.lock().unwrap();
        assert_eq!(result, Color::RED);
    }

    #[test]
    fn test_more_colors_signal() {
        setup();
        let palette = RecentColorsPalette::new();

        let requested = Arc::new(AtomicBool::new(false));
        let requested_clone = requested.clone();

        palette.more_colors_requested.connect(move |()| {
            requested_clone.store(true, Ordering::SeqCst);
        });

        palette.more_colors_requested.emit(());

        assert!(requested.load(Ordering::SeqCst));
    }

    #[test]
    fn test_clear() {
        setup();
        let mut palette = RecentColorsPalette::new();
        palette.add_color(Color::RED);
        palette.add_color(Color::GREEN);
        palette.set_selected_index(Some(0));

        palette.clear();

        assert!(palette.colors().is_empty());
        assert!(palette.selected_index().is_none());
    }

    #[test]
    fn test_selection() {
        setup();
        let mut palette = RecentColorsPalette::new();
        palette.add_color(Color::RED);
        palette.add_color(Color::GREEN);

        assert!(palette.selected_index().is_none());
        assert!(palette.selected_color().is_none());

        palette.set_selected_index(Some(1));
        assert_eq!(palette.selected_index(), Some(1));
        assert_eq!(palette.selected_color(), Some(Color::RED)); // Index 1 is RED (second added)
    }

    #[test]
    fn test_row_count() {
        setup();
        let palette = RecentColorsPalette::new().with_columns(4).with_colors(vec![
            Color::RED,
            Color::GREEN,
            Color::BLUE,
            Color::WHITE,
            Color::BLACK,
        ]);

        // 5 colors with 4 columns = 2 rows
        assert_eq!(palette.row_count(), 2);
    }
}
