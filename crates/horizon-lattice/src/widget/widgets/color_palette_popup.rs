//! Color palette popup widget implementation.
//!
//! This module provides [`ColorPalettePopup`], a popup containing a recent colors
//! palette for quick color selection from toolbar buttons.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{ColorPalettePopup, ColorButton};
//! use horizon_lattice_render::Color;
//!
//! let mut popup = ColorPalettePopup::new();
//!
//! // Add some recent colors
//! popup.add_color(Color::RED);
//! popup.add_color(Color::GREEN);
//!
//! // Connect to color selection
//! popup.color_selected.connect(|&color| {
//!     println!("Selected: {:?}", color);
//! });
//!
//! // Connect to "More Colors..." request
//! popup.more_colors_requested.connect(|()| {
//!     // Open full ColorDialog
//! });
//!
//! // Show the popup below a button
//! popup.popup_below_rect(button_rect);
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontSystem, Point, Rect, Renderer, Size, Stroke, TextLayout, TextRenderer,
};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MousePressEvent, MouseReleaseEvent, PaintContext,
    SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent,
};

// ============================================================================
// Constants
// ============================================================================

const DEFAULT_COLUMNS: usize = 8;
const DEFAULT_MAX_COLORS: usize = 16;
const DEFAULT_SWATCH_SIZE: f32 = 20.0;
const SWATCH_GAP: f32 = 4.0;
const PADDING: f32 = 8.0;
const ACTION_HEIGHT: f32 = 24.0;

// ============================================================================
// ColorPalettePopup
// ============================================================================

/// A popup widget containing a recent colors palette.
///
/// ColorPalettePopup provides a floating popup with recent colors for quick
/// selection, commonly used with toolbar color buttons.
///
/// # Signals
///
/// - `color_selected(Color)`: Emitted when a color is selected
/// - `more_colors_requested()`: Emitted when "More Colors..." is clicked
/// - `closed()`: Emitted when the popup is closed
pub struct ColorPalettePopup {
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

    /// Whether the popup is currently visible.
    visible: bool,

    /// Border color.
    border_color: Color,

    /// Background color.
    background_color: Color,

    /// Currently hovered swatch index.
    hovered_index: Option<usize>,

    /// Currently selected swatch index.
    selected_index: Option<usize>,

    /// Whether the action area is hovered.
    action_hovered: bool,

    /// Whether to show the "More Colors..." action.
    show_more_colors_action: bool,

    /// Signal emitted when a color is selected.
    pub color_selected: Signal<Color>,

    /// Signal emitted when "More Colors..." is clicked.
    pub more_colors_requested: Signal<()>,

    /// Signal emitted when the popup is closed.
    pub closed: Signal<()>,
}

impl ColorPalettePopup {
    /// Create a new color palette popup.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Fixed, SizePolicy::Fixed));
        base.hide(); // Start hidden

        Self {
            base,
            colors: Vec::new(),
            max_colors: DEFAULT_MAX_COLORS,
            columns: DEFAULT_COLUMNS,
            swatch_size: DEFAULT_SWATCH_SIZE,
            visible: false,
            border_color: Color::from_rgb8(180, 180, 180),
            background_color: Color::WHITE,
            hovered_index: None,
            selected_index: None,
            action_hovered: false,
            show_more_colors_action: true,
            color_selected: Signal::new(),
            more_colors_requested: Signal::new(),
            closed: Signal::new(),
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
    }

    /// Add a color to the palette (at the front).
    pub fn add_color(&mut self, color: Color) {
        self.colors.retain(|&c| c != color);
        self.colors.insert(0, color);
        if self.colors.len() > self.max_colors {
            self.colors.pop();
        }
    }

    /// Clear all colors from the palette.
    pub fn clear(&mut self) {
        self.colors.clear();
        self.selected_index = None;
        self.hovered_index = None;
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
    }

    /// Get the number of columns.
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Set the number of columns.
    pub fn set_columns(&mut self, columns: usize) {
        self.columns = columns.max(1);
    }

    // =========================================================================
    // Visibility
    // =========================================================================

    /// Check if the popup is visible.
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Show the popup at the specified position.
    pub fn popup_at(&mut self, x: f32, y: f32) {
        self.base.set_pos(Point::new(x, y));
        self.update_size();
        self.visible = true;
        self.base.show();
        self.base.update();
    }

    /// Show the popup below the specified rectangle (anchor).
    pub fn popup_below_rect(&mut self, anchor: Rect) {
        self.update_size();
        let x = anchor.left();
        let y = anchor.bottom();

        self.base.set_pos(Point::new(x, y));
        self.visible = true;
        self.base.show();
        self.base.update();
    }

    /// Hide the popup.
    pub fn hide(&mut self) {
        if self.visible {
            self.visible = false;
            self.base.hide();
        }
    }

    /// Close the popup (hide and emit closed signal).
    pub fn close(&mut self) {
        if self.visible {
            self.visible = false;
            self.base.hide();
            self.closed.emit(());
        }
    }

    // =========================================================================
    // Size Calculations
    // =========================================================================

    fn calculate_size(&self) -> Size {
        let grid_width =
            self.columns as f32 * self.swatch_size + (self.columns - 1).max(0) as f32 * SWATCH_GAP;
        let grid_height = self.grid_height();

        let action_height = if self.show_more_colors_action {
            ACTION_HEIGHT + SWATCH_GAP
        } else {
            0.0
        };

        Size::new(
            grid_width + PADDING * 2.0,
            grid_height + action_height + PADDING * 2.0,
        )
    }

    fn update_size(&mut self) {
        let size = self.calculate_size();
        self.base.set_size(size);
    }

    fn row_count(&self) -> usize {
        if self.colors.is_empty() {
            return 1; // At least one row for empty state
        }
        self.colors.len().div_ceil(self.columns)
    }

    fn grid_height(&self) -> f32 {
        let rows = self.row_count();
        rows as f32 * self.swatch_size + (rows - 1).max(0) as f32 * SWATCH_GAP
    }

    fn swatch_rect(&self, index: usize) -> Option<Rect> {
        if index >= self.colors.len() {
            return None;
        }

        let row = index / self.columns;
        let col = index % self.columns;

        let x = PADDING + col as f32 * (self.swatch_size + SWATCH_GAP);
        let y = PADDING + row as f32 * (self.swatch_size + SWATCH_GAP);

        Some(Rect::new(x, y, self.swatch_size, self.swatch_size))
    }

    fn action_rect(&self) -> Option<Rect> {
        if !self.show_more_colors_action {
            return None;
        }

        let grid_width =
            self.columns as f32 * self.swatch_size + (self.columns - 1).max(0) as f32 * SWATCH_GAP;
        let y = PADDING + self.grid_height() + SWATCH_GAP;

        Some(Rect::new(PADDING, y, grid_width, ACTION_HEIGHT))
    }

    fn swatch_at_point(&self, point: Point) -> Option<usize> {
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
        let rect = self.base.rect();
        let local_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());

        // Click outside closes the popup
        if !local_rect.contains(pos) {
            self.close();
            return true;
        }

        // Check swatch click
        if let Some(index) = self.swatch_at_point(pos) {
            self.selected_index = Some(index);
            self.base.update();
            return true;
        }

        // Check action click
        if let Some(action_rect) = self.action_rect()
            && action_rect.contains(pos)
        {
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check swatch release
        if let Some(index) = self.swatch_at_point(pos)
            && let Some(&color) = self.colors.get(index)
        {
            self.color_selected.emit(color);
            self.close();
            return true;
        }

        // Check action release
        if let Some(action_rect) = self.action_rect()
            && action_rect.contains(pos)
        {
            self.more_colors_requested.emit(());
            self.close();
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        match event.key {
            Key::Escape => {
                self.close();
                true
            }
            Key::Enter | Key::Space => {
                if let Some(index) = self.selected_index
                    && let Some(&color) = self.colors.get(index)
                {
                    self.color_selected.emit(color);
                    self.close();
                    return true;
                }
                false
            }
            Key::ArrowLeft => self.move_selection(-1, 0),
            Key::ArrowRight => self.move_selection(1, 0),
            Key::ArrowUp => self.move_selection(0, -(self.columns as i32)),
            Key::ArrowDown => self.move_selection(0, self.columns as i32),
            _ => false,
        }
    }

    fn move_selection(&mut self, dx: i32, dy: i32) -> bool {
        if self.colors.is_empty() {
            return false;
        }

        let current = self.selected_index.unwrap_or(0) as i32;
        let new_index = (current + dx + dy).clamp(0, self.colors.len() as i32 - 1) as usize;

        if self.selected_index != Some(new_index) {
            self.selected_index = Some(new_index);
            self.base.update();
        }
        true
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
        let border_color = Color::from_rgb8(180, 180, 180);
        let hover_color = Color::from_rgb8(0, 122, 255);

        for (i, &color) in self.colors.iter().enumerate() {
            let Some(rect) = self.swatch_rect(i) else {
                continue;
            };

            // Checkerboard for alpha
            if color.a < 1.0 {
                self.paint_checkerboard(ctx, rect);
            }

            // Color fill
            ctx.renderer().fill_rect(rect, color);

            // Border
            let is_selected = self.selected_index == Some(i);
            let is_hovered = self.hovered_index == Some(i);
            let stroke_color = if is_selected || is_hovered {
                hover_color
            } else {
                border_color
            };
            let stroke_width = if is_selected { 2.0 } else { 1.0 };
            let stroke = Stroke::new(stroke_color, stroke_width);
            ctx.renderer().stroke_rect(rect, &stroke);
        }
    }

    fn paint_action(&self, ctx: &mut PaintContext<'_>) {
        let Some(rect) = self.action_rect() else {
            return;
        };

        // Background on hover
        if self.action_hovered {
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

        let text_color = Color::from_rgb8(0, 122, 255);

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

        let text = "No recent colors";
        let mut font_system = FontSystem::new();
        let font = Font::default();
        let layout = TextLayout::new(&mut font_system, text, &font);

        let rect = self.base.rect();
        let text_x = (rect.width() - layout.width()) / 2.0;
        let text_y = PADDING + (self.swatch_size - layout.height()) / 2.0;

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

impl Default for ColorPalettePopup {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for ColorPalettePopup {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for ColorPalettePopup {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let size = self.calculate_size();
        SizeHint::from_dimensions(size.width, size.height)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        if !self.visible {
            return;
        }

        let rect = ctx.rect();

        // Shadow
        let shadow_rect = Rect::new(2.0, 2.0, rect.width(), rect.height());
        ctx.renderer()
            .fill_rect(shadow_rect, Color::from_rgba8(0, 0, 0, 30));

        // Background
        ctx.renderer().fill_rect(rect, self.background_color);

        // Border
        let stroke = Stroke::new(self.border_color, 1.0);
        ctx.renderer().stroke_rect(rect, &stroke);

        // Content
        if self.colors.is_empty() {
            self.paint_empty_state(ctx);
        } else {
            self.paint_swatches(ctx);
        }
        self.paint_action(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        if !self.visible {
            return false;
        }

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
                let pos = e.local_pos;
                let new_hover = self.swatch_at_point(pos);
                let new_action_hover = self.action_rect().map(|r| r.contains(pos)).unwrap_or(false);

                if new_hover != self.hovered_index || new_action_hover != self.action_hovered {
                    self.hovered_index = new_hover;
                    self.action_hovered = new_action_hover;
                    self.base.update();
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
            WidgetEvent::FocusOut(_) => {
                self.close();
                return true;
            }
            _ => {}
        }

        false
    }
}

// Thread safety assertion
static_assertions::assert_impl_all!(ColorPalettePopup: Send, Sync);

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
    fn test_popup_creation() {
        setup();
        let popup = ColorPalettePopup::new();
        assert!(!popup.is_visible());
        assert!(popup.colors().is_empty());
    }

    #[test]
    fn test_popup_colors() {
        setup();
        let mut popup = ColorPalettePopup::new();

        popup.add_color(Color::RED);
        popup.add_color(Color::GREEN);

        assert_eq!(popup.colors().len(), 2);
        assert_eq!(popup.colors()[0], Color::GREEN); // Most recent first
    }

    #[test]
    fn test_popup_visibility() {
        setup();
        let mut popup = ColorPalettePopup::new();

        assert!(!popup.is_visible());

        popup.popup_at(100.0, 100.0);
        assert!(popup.is_visible());

        popup.hide();
        assert!(!popup.is_visible());
    }

    #[test]
    fn test_popup_close_emits_signal() {
        setup();
        let mut popup = ColorPalettePopup::new();

        let closed = Arc::new(AtomicBool::new(false));
        let closed_clone = closed.clone();

        popup.closed.connect(move |()| {
            closed_clone.store(true, Ordering::SeqCst);
        });

        popup.popup_at(100.0, 100.0);
        popup.close();

        assert!(closed.load(Ordering::SeqCst));
        assert!(!popup.is_visible());
    }

    #[test]
    fn test_color_selected_signal() {
        setup();
        let popup = ColorPalettePopup::new().with_colors(vec![Color::RED]);

        let selected = Arc::new(Mutex::new(Color::TRANSPARENT));
        let selected_clone = selected.clone();

        popup.color_selected.connect(move |color| {
            *selected_clone.lock().unwrap() = *color;
        });

        popup.color_selected.emit(Color::RED);

        let result = *selected.lock().unwrap();
        assert_eq!(result, Color::RED);
    }

    #[test]
    fn test_builder_pattern() {
        setup();
        let popup = ColorPalettePopup::new()
            .with_max_colors(8)
            .with_columns(4)
            .with_colors(vec![Color::RED, Color::BLUE])
            .with_more_colors_action(true);

        assert_eq!(popup.max_colors(), 8);
        assert_eq!(popup.columns(), 4);
        assert_eq!(popup.colors().len(), 2);
    }
}
