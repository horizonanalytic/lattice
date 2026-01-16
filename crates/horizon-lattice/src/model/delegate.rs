//! Item delegates for custom rendering and editing in views.
//!
//! The delegate system provides a way to customize how items are displayed
//! and edited in item views (ListView, TableView, TreeView). This follows
//! the Model/View/Delegate pattern.
//!
//! # Architecture
//!
//! - **Model**: Provides the data through the `ItemModel` trait
//! - **View**: Manages layout, scrolling, and selection
//! - **Delegate**: Handles rendering and editing of individual items
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice::model::{ItemDelegate, StyleOptionViewItem, DefaultItemDelegate};
//!
//! // Use the default delegate
//! let delegate = DefaultItemDelegate::new();
//!
//! // Or implement a custom delegate
//! struct MyDelegate;
//!
//! impl ItemDelegate for MyDelegate {
//!     fn paint(&self, ctx: &mut DelegatePaintContext<'_>, option: &StyleOptionViewItem) {
//!         // Custom painting logic
//!     }
//!
//!     fn size_hint(&self, option: &StyleOptionViewItem) -> (f32, f32) {
//!         (200.0, 24.0)
//!     }
//! }
//! ```
//!
//! # Style Options
//!
//! The [`StyleOptionViewItem`] struct provides all the information needed
//! to render an item correctly, including:
//! - The item's rectangle
//! - Visual state (selected, focused, hovered, etc.)
//! - Data roles (display text, icon, check state)
//! - View-specific settings (alternating row colors, etc.)

use horizon_lattice_render::{
    Color, Font, FontFamily, FontSystem, GpuRenderer, HorizontalAlign, Icon, Point, Rect,
    Renderer, Size, TextLayout, TextLayoutOptions, TextRenderer, VerticalAlign,
};

use super::index::ModelIndex;
use super::role::{CheckState, ItemData, TextAlignment};
use super::traits::ItemFlags;

/// Visual state flags for an item being rendered.
///
/// These flags indicate the current visual state of the item, which the
/// delegate uses to determine how to render it.
#[derive(Debug, Clone, Copy, Default)]
pub struct ViewItemState {
    /// The item is currently selected.
    pub selected: bool,
    /// The item has keyboard focus.
    pub focused: bool,
    /// The mouse is hovering over the item.
    pub hovered: bool,
    /// The item is being pressed (mouse down).
    pub pressed: bool,
    /// The item is enabled for interaction.
    pub enabled: bool,
    /// The item is being edited.
    pub editing: bool,
    /// The item is in an alternate row (for alternating row colors).
    pub alternate: bool,
    /// The item is expanded (for tree items).
    pub expanded: bool,
    /// The item has children (for tree items).
    pub has_children: bool,
    /// The item's siblings have children (affects branch display).
    pub siblings_have_children: bool,
    /// This is the first item in its parent.
    pub first: bool,
    /// This is the last item in its parent.
    pub last: bool,
}

impl ViewItemState {
    /// Creates a new state with default values (enabled only).
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Sets the selected state.
    pub fn with_selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets the focused state.
    pub fn with_focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// Sets the hovered state.
    pub fn with_hovered(mut self, hovered: bool) -> Self {
        self.hovered = hovered;
        self
    }

    /// Sets the pressed state.
    pub fn with_pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    /// Sets the enabled state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets the alternate row state.
    pub fn with_alternate(mut self, alternate: bool) -> Self {
        self.alternate = alternate;
        self
    }

    /// Sets the expanded state (for tree items).
    pub fn with_expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Sets whether the item has children.
    pub fn with_has_children(mut self, has_children: bool) -> Self {
        self.has_children = has_children;
        self
    }
}

/// Display features for the delegate.
#[derive(Debug, Clone, Copy, Default)]
pub struct ViewItemFeatures {
    /// Show the selection indicator.
    pub show_selection: bool,
    /// Show the focus indicator.
    pub show_focus: bool,
    /// Show the decoration (icon).
    pub show_decoration: bool,
    /// Show the check indicator.
    pub show_check: bool,
    /// Wrap text if it doesn't fit.
    pub wrap_text: bool,
    /// Allow text elision with ellipsis.
    pub elide_text: bool,
}

impl ViewItemFeatures {
    /// Creates features with typical defaults for list/table views.
    pub fn default_for_view() -> Self {
        Self {
            show_selection: true,
            show_focus: true,
            show_decoration: true,
            show_check: true,
            wrap_text: false,
            elide_text: true,
        }
    }
}

/// Position of the decoration (icon) relative to text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DecorationPosition {
    /// Decoration on the left of text.
    #[default]
    Left,
    /// Decoration on the right of text.
    Right,
    /// Decoration above text.
    Top,
    /// Decoration below text.
    Bottom,
}

/// Style information for rendering a view item.
///
/// This struct contains all the information a delegate needs to render
/// an item correctly, including geometry, state, and cached data values.
#[derive(Debug, Clone)]
pub struct StyleOptionViewItem {
    /// The bounding rectangle for the item.
    pub rect: Rect,
    /// The model index being rendered.
    pub index: ModelIndex,
    /// The visual state of the item.
    pub state: ViewItemState,
    /// Display features to use.
    pub features: ViewItemFeatures,
    /// Item flags from the model.
    pub flags: ItemFlags,
    /// The decoration position.
    pub decoration_position: DecorationPosition,
    /// Spacing between decoration and text.
    pub decoration_spacing: f32,
    /// Size for decoration icons.
    pub decoration_size: Size,
    /// Check indicator size.
    pub check_size: Size,
    /// The display text (if any).
    pub text: Option<String>,
    /// The decoration icon (if any).
    pub icon: Option<Icon>,
    /// The check state (if checkable).
    pub check_state: Option<CheckState>,
    /// Custom font override.
    pub font: Option<Font>,
    /// Custom background color.
    pub background_color: Option<Color>,
    /// Custom foreground/text color.
    pub foreground_color: Option<Color>,
    /// Text alignment.
    pub text_alignment: TextAlignment,
    /// Tooltip text.
    pub tooltip: Option<String>,
}

impl Default for StyleOptionViewItem {
    fn default() -> Self {
        Self {
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            index: ModelIndex::invalid(),
            state: ViewItemState::new(),
            features: ViewItemFeatures::default_for_view(),
            flags: ItemFlags::new(),
            decoration_position: DecorationPosition::Left,
            decoration_spacing: 4.0,
            decoration_size: Size::new(16.0, 16.0),
            check_size: Size::new(16.0, 16.0),
            text: None,
            icon: None,
            check_state: None,
            font: None,
            background_color: None,
            foreground_color: None,
            text_alignment: TextAlignment::left(),
            tooltip: None,
        }
    }
}

impl StyleOptionViewItem {
    /// Creates a new style option with the given rectangle and index.
    pub fn new(rect: Rect, index: ModelIndex) -> Self {
        Self {
            rect,
            index,
            ..Default::default()
        }
    }

    /// Sets the state.
    pub fn with_state(mut self, state: ViewItemState) -> Self {
        self.state = state;
        self
    }

    /// Sets the item flags.
    pub fn with_flags(mut self, flags: ItemFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Sets the display text.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Sets the decoration icon.
    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Sets the check state.
    pub fn with_check_state(mut self, check_state: CheckState) -> Self {
        self.check_state = Some(check_state);
        self
    }

    /// Sets the text alignment.
    pub fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.text_alignment = alignment;
        self
    }

    /// Sets the font.
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets the background color.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    /// Sets the foreground color.
    pub fn with_foreground_color(mut self, color: Color) -> Self {
        self.foreground_color = Some(color);
        self
    }
}

/// Context for delegate painting operations.
///
/// Wraps a GPU renderer and provides the current item's rect.
pub struct DelegatePaintContext<'a> {
    renderer: &'a mut GpuRenderer,
    rect: Rect,
}

impl<'a> DelegatePaintContext<'a> {
    /// Creates a new paint context.
    pub fn new(renderer: &'a mut GpuRenderer, rect: Rect) -> Self {
        Self { renderer, rect }
    }

    /// Gets the renderer for drawing.
    #[inline]
    pub fn renderer(&mut self) -> &mut GpuRenderer {
        self.renderer
    }

    /// Gets the item's bounding rectangle.
    #[inline]
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// Gets the width of the item rect.
    #[inline]
    pub fn width(&self) -> f32 {
        self.rect.width()
    }

    /// Gets the height of the item rect.
    #[inline]
    pub fn height(&self) -> f32 {
        self.rect.height()
    }
}

/// The delegate trait for rendering and editing items in views.
///
/// Delegates are responsible for:
/// - Painting items with their current visual state
/// - Providing size hints for layout
/// - Creating and managing editors for in-place editing
///
/// # Default Implementation
///
/// Use [`DefaultItemDelegate`] for standard text/icon/checkbox rendering.
/// Implement this trait for custom rendering needs.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::model::{ItemDelegate, StyleOptionViewItem, DelegatePaintContext};
///
/// struct ProgressBarDelegate;
///
/// impl ItemDelegate for ProgressBarDelegate {
///     fn paint(&self, ctx: &mut DelegatePaintContext<'_>, option: &StyleOptionViewItem) {
///         // Draw a progress bar based on the item's data
///         let rect = option.rect;
///         let progress = option.text.as_ref()
///             .and_then(|s| s.parse::<f32>().ok())
///             .unwrap_or(0.0) / 100.0;
///
///         // Background
///         ctx.renderer().fill_rect(rect, Color::LIGHT_GRAY);
///
///         // Progress fill
///         let fill_width = rect.width() * progress;
///         let fill_rect = Rect::new(rect.origin.x, rect.origin.y, fill_width, rect.height());
///         ctx.renderer().fill_rect(fill_rect, Color::from_rgb8(76, 175, 80));
///     }
///
///     fn size_hint(&self, option: &StyleOptionViewItem) -> (f32, f32) {
///         (100.0, 20.0)
///     }
/// }
/// ```
pub trait ItemDelegate: Send + Sync {
    /// Paint the item.
    ///
    /// This is called to render the item into the provided paint context.
    /// The delegate should use the information in `option` to determine
    /// how to render the item (colors, state indicators, etc.).
    ///
    /// # Arguments
    ///
    /// * `ctx` - The paint context with renderer access
    /// * `option` - Style information about the item being painted
    fn paint(&self, ctx: &mut DelegatePaintContext<'_>, option: &StyleOptionViewItem);

    /// Returns the size hint for the item.
    ///
    /// Views use this to determine item sizes for layout purposes.
    /// Returns `(width, height)` in pixels.
    ///
    /// # Arguments
    ///
    /// * `option` - Style information about the item
    fn size_hint(&self, option: &StyleOptionViewItem) -> (f32, f32);

    /// Called when editing should start for an item.
    ///
    /// Return `true` if editing was successfully started.
    /// The default returns `false` (editing not supported).
    ///
    /// # Arguments
    ///
    /// * `option` - Style information about the item to edit
    fn start_editing(&self, _option: &StyleOptionViewItem) -> bool {
        false
    }

    /// Called when editing should commit.
    ///
    /// Return the edited value to set on the model, or `None` to cancel.
    /// The default returns `None`.
    fn commit_editing(&self) -> Option<ItemData> {
        None
    }

    /// Called when editing should be cancelled.
    fn cancel_editing(&self) {}

    /// Returns `true` if the delegate is currently editing.
    fn is_editing(&self) -> bool {
        false
    }

    /// Returns the editor widget's rectangle if editing.
    fn editor_rect(&self) -> Option<Rect> {
        None
    }

    /// Updates the editor size if the item rect changes during editing.
    fn update_editor_geometry(&self, _rect: Rect) {}

    /// Handles a click event on the item.
    ///
    /// Returns `true` if the delegate handled the event (e.g., toggled a checkbox).
    /// The default returns `false`.
    ///
    /// # Arguments
    ///
    /// * `option` - Style information about the item
    /// * `pos` - Click position relative to item rect
    fn handle_click(&self, _option: &StyleOptionViewItem, _pos: Point) -> bool {
        false
    }

    /// Returns the click region for a specific feature.
    ///
    /// This allows views to determine what was clicked (checkbox, icon, text).
    fn click_region(&self, _option: &StyleOptionViewItem, _pos: Point) -> ClickRegion {
        ClickRegion::None
    }
}

/// Region within an item that was clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickRegion {
    /// No specific region or outside the item.
    None,
    /// The checkbox/check indicator.
    CheckIndicator,
    /// The decoration (icon).
    Decoration,
    /// The text content.
    Text,
    /// The expand/collapse indicator (tree items).
    ExpandIndicator,
    /// The entire item (general click).
    Item,
}

/// Default theme colors for item delegates.
#[derive(Debug, Clone)]
pub struct DelegateTheme {
    /// Background color for normal items.
    pub background: Color,
    /// Background color for selected items.
    pub selection_background: Color,
    /// Background color for alternate rows.
    pub alternate_background: Color,
    /// Background color for hovered items.
    pub hover_background: Color,
    /// Text color for normal items.
    pub text: Color,
    /// Text color for selected items.
    pub selection_text: Color,
    /// Text color for disabled items.
    pub disabled_text: Color,
    /// Focus indicator color.
    pub focus_border: Color,
    /// Check indicator color.
    pub check_color: Color,
}

impl Default for DelegateTheme {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            selection_background: Color::from_rgb8(51, 153, 255),
            alternate_background: Color::from_rgba8(0, 0, 0, 8),
            hover_background: Color::from_rgba8(0, 0, 0, 15),
            text: Color::from_rgb8(33, 33, 33),
            selection_text: Color::WHITE,
            disabled_text: Color::from_rgb8(160, 160, 160),
            focus_border: Color::from_rgb8(51, 153, 255),
            check_color: Color::from_rgb8(51, 153, 255),
        }
    }
}

/// Default delegate for standard item rendering.
///
/// This delegate handles:
/// - Text rendering with alignment and elision
/// - Icon/decoration display
/// - Checkbox indicators
/// - Selection highlighting
/// - Focus indicators
/// - Alternate row coloring
/// - Disabled state rendering
#[derive(Debug, Clone)]
pub struct DefaultItemDelegate {
    /// Theme colors.
    theme: DelegateTheme,
    /// Default font for text rendering.
    default_font: Font,
    /// Padding inside the item rect.
    padding: f32,
}

impl Default for DefaultItemDelegate {
    fn default() -> Self {
        Self::new()
    }
}

impl DefaultItemDelegate {
    /// Creates a new default delegate with standard settings.
    pub fn new() -> Self {
        Self {
            theme: DelegateTheme::default(),
            default_font: Font::builder()
                .family(FontFamily::SansSerif)
                .size(13.0)
                .build(),
            padding: 4.0,
        }
    }

    /// Sets the theme colors.
    pub fn with_theme(mut self, theme: DelegateTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Sets the default font.
    pub fn with_font(mut self, font: Font) -> Self {
        self.default_font = font;
        self
    }

    /// Sets the padding.
    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Gets the theme.
    pub fn theme(&self) -> &DelegateTheme {
        &self.theme
    }

    /// Gets the default font.
    pub fn font(&self) -> &Font {
        &self.default_font
    }

    /// Calculate the background color for an item.
    fn background_color(&self, option: &StyleOptionViewItem) -> Color {
        if let Some(bg) = option.background_color {
            return bg;
        }

        if !option.state.enabled {
            return self.theme.background;
        }

        if option.state.selected {
            return self.theme.selection_background;
        }

        if option.state.pressed {
            return self.theme.selection_background.with_alpha(0.3);
        }

        if option.state.hovered {
            return self.theme.hover_background;
        }

        if option.state.alternate {
            return self.theme.alternate_background;
        }

        self.theme.background
    }

    /// Calculate the text color for an item.
    fn text_color(&self, option: &StyleOptionViewItem) -> Color {
        if let Some(fg) = option.foreground_color {
            return fg;
        }

        if !option.state.enabled {
            return self.theme.disabled_text;
        }

        if option.state.selected {
            return self.theme.selection_text;
        }

        self.theme.text
    }

    /// Paint the background.
    fn paint_background(&self, ctx: &mut DelegatePaintContext<'_>, option: &StyleOptionViewItem) {
        let bg = self.background_color(option);
        if bg.a > 0.0 {
            ctx.renderer().fill_rect(option.rect, bg);
        }
    }

    /// Paint the focus indicator.
    fn paint_focus(&self, ctx: &mut DelegatePaintContext<'_>, option: &StyleOptionViewItem) {
        if !option.state.focused || !option.features.show_focus {
            return;
        }

        let rect = option.rect.deflate(1.0);
        let stroke = horizon_lattice_render::Stroke::new(self.theme.focus_border, 1.0);
        ctx.renderer().stroke_rect(rect, &stroke);
    }

    /// Calculate the content rect (after padding and features).
    fn content_rect(&self, option: &StyleOptionViewItem) -> Rect {
        let mut rect = option.rect.deflate(self.padding);

        // Reserve space for check indicator
        if option.features.show_check && option.flags.checkable {
            rect = Rect::new(
                rect.origin.x + option.check_size.width + option.decoration_spacing,
                rect.origin.y,
                rect.width() - option.check_size.width - option.decoration_spacing,
                rect.height(),
            );
        }

        rect
    }

    /// Paint the check indicator.
    fn paint_check(&self, ctx: &mut DelegatePaintContext<'_>, option: &StyleOptionViewItem) {
        if !option.features.show_check || !option.flags.checkable {
            return;
        }

        let check_state = option.check_state.unwrap_or(CheckState::Unchecked);
        let rect = option.rect.deflate(self.padding);

        // Center the check box vertically
        let check_y = rect.origin.y + (rect.height() - option.check_size.height) / 2.0;
        let check_rect = Rect::new(
            rect.origin.x,
            check_y,
            option.check_size.width,
            option.check_size.height,
        );

        // Draw checkbox background
        let check_bg = if option.state.enabled {
            Color::WHITE
        } else {
            Color::from_rgb8(240, 240, 240)
        };
        ctx.renderer().fill_rect(check_rect, check_bg);

        // Draw checkbox border
        let border_color = if option.state.enabled {
            Color::from_rgb8(180, 180, 180)
        } else {
            Color::from_rgb8(200, 200, 200)
        };
        let stroke = horizon_lattice_render::Stroke::new(border_color, 1.0);
        ctx.renderer().stroke_rect(check_rect, &stroke);

        // Draw check mark if checked
        let check_color = if option.state.enabled {
            self.theme.check_color
        } else {
            self.theme.disabled_text
        };

        match check_state {
            CheckState::Checked => {
                // Draw checkmark
                self.draw_checkmark(ctx, check_rect.deflate(3.0), check_color);
            }
            CheckState::PartiallyChecked => {
                // Draw partial indicator (horizontal line)
                let line_rect = Rect::new(
                    check_rect.origin.x + 3.0,
                    check_rect.center().y - 1.0,
                    check_rect.width() - 6.0,
                    2.0,
                );
                ctx.renderer().fill_rect(line_rect, check_color);
            }
            CheckState::Unchecked => {}
        }
    }

    /// Draw a checkmark in the given rect.
    fn draw_checkmark(&self, ctx: &mut DelegatePaintContext<'_>, rect: Rect, color: Color) {
        let stroke = horizon_lattice_render::Stroke::new(color, 2.0);

        // Checkmark path: from bottom-left corner, down to bottom, then up to top-right
        let points = [
            Point::new(rect.left(), rect.center().y),
            Point::new(rect.left() + rect.width() * 0.35, rect.bottom()),
            Point::new(rect.right(), rect.top()),
        ];

        ctx.renderer().draw_polyline(&points, &stroke);
    }

    /// Paint the text content.
    ///
    /// Note: Full text rendering requires integration with the view's render pass system.
    /// This implementation prepares text layout and stores glyphs for the view to render.
    /// For now, text is prepared but actual glyph rendering must be integrated by the view.
    fn paint_text(&self, _ctx: &mut DelegatePaintContext<'_>, option: &StyleOptionViewItem) {
        let text = match &option.text {
            Some(t) if !t.is_empty() => t,
            _ => return,
        };

        let content_rect = self.content_rect(option);
        let text_color = self.text_color(option);
        let font = option.font.as_ref().unwrap_or(&self.default_font);

        // Calculate text position based on alignment
        let text_x = content_rect.origin.x;
        let text_y = content_rect.center().y;
        let position = Point::new(text_x, text_y);

        // Create text layout
        let mut font_system = FontSystem::new();
        let layout_options = TextLayoutOptions::default()
            .max_width(content_rect.width())
            .horizontal_align(HorizontalAlign::Left)
            .vertical_align(VerticalAlign::Top);
        let layout = TextLayout::with_options(&mut font_system, text, font, layout_options);

        // Prepare glyphs for rendering
        // Note: In a full implementation, these glyphs would be submitted to a TextRenderPass.
        // The view that owns this delegate should handle the actual glyph rendering.
        if let Ok(mut text_renderer) = TextRenderer::new() {
            if let Ok(_prepared_glyphs) =
                text_renderer.prepare_layout(&mut font_system, &layout, position, text_color)
            {
                // The prepared glyphs would be rendered by the view's text render pass.
                // For now, this is a placeholder showing the pattern.
            }
        }
    }

    /// Returns layout information for text at the given index.
    ///
    /// This can be used by views to get text metrics for layout calculations.
    pub fn text_layout(
        &self,
        option: &StyleOptionViewItem,
    ) -> Option<(TextLayout, Point, Color)> {
        let text = option.text.as_ref()?;
        if text.is_empty() {
            return None;
        }

        let content_rect = self.content_rect(option);
        let text_color = self.text_color(option);
        let font = option.font.as_ref().unwrap_or(&self.default_font);

        let text_x = content_rect.origin.x;
        let text_y = content_rect.center().y;
        let position = Point::new(text_x, text_y);

        let mut font_system = FontSystem::new();
        let layout_options = TextLayoutOptions::default()
            .max_width(content_rect.width())
            .horizontal_align(HorizontalAlign::Left)
            .vertical_align(VerticalAlign::Top);
        let layout = TextLayout::with_options(&mut font_system, text, font, layout_options);

        Some((layout, position, text_color))
    }
}

impl ItemDelegate for DefaultItemDelegate {
    fn paint(&self, ctx: &mut DelegatePaintContext<'_>, option: &StyleOptionViewItem) {
        self.paint_background(ctx, option);
        self.paint_check(ctx, option);
        self.paint_text(ctx, option);
        self.paint_focus(ctx, option);
    }

    fn size_hint(&self, option: &StyleOptionViewItem) -> (f32, f32) {
        let mut width = self.padding * 2.0;
        let mut height = self.default_font.size() + self.padding * 2.0;

        // Add space for check indicator
        if option.features.show_check && option.flags.checkable {
            width += option.check_size.width + option.decoration_spacing;
        }

        // Add space for decoration
        if option.features.show_decoration && option.icon.is_some() {
            width += option.decoration_size.width + option.decoration_spacing;
            height = height.max(option.decoration_size.height + self.padding * 2.0);
        }

        // Estimate text width (simplified)
        if let Some(text) = &option.text {
            // Rough estimate: average character width * count
            let char_width = self.default_font.size() * 0.5;
            width += text.chars().count() as f32 * char_width;
        }

        (width, height.max(24.0)) // Minimum height
    }

    fn handle_click(&self, option: &StyleOptionViewItem, pos: Point) -> bool {
        // Check if click is on checkbox
        if option.features.show_check && option.flags.checkable {
            let rect = option.rect.deflate(self.padding);
            let check_y = rect.origin.y + (rect.height() - option.check_size.height) / 2.0;
            let check_rect = Rect::new(
                rect.origin.x,
                check_y,
                option.check_size.width,
                option.check_size.height,
            );

            if check_rect.contains(pos) {
                return true; // Signal that checkbox was clicked
            }
        }

        false
    }

    fn click_region(&self, option: &StyleOptionViewItem, pos: Point) -> ClickRegion {
        if !option.rect.contains(pos) {
            return ClickRegion::None;
        }

        // Check if click is on checkbox
        if option.features.show_check && option.flags.checkable {
            let rect = option.rect.deflate(self.padding);
            let check_y = rect.origin.y + (rect.height() - option.check_size.height) / 2.0;
            let check_rect = Rect::new(
                rect.origin.x,
                check_y,
                option.check_size.width,
                option.check_size.height,
            );

            if check_rect.contains(pos) {
                return ClickRegion::CheckIndicator;
            }
        }

        // Default to item region
        ClickRegion::Item
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_view_item_state_default() {
        let state = ViewItemState::new();
        assert!(state.enabled);
        assert!(!state.selected);
        assert!(!state.focused);
        assert!(!state.hovered);
    }

    #[test]
    fn test_view_item_state_builder() {
        let state = ViewItemState::new()
            .with_selected(true)
            .with_focused(true)
            .with_hovered(true);

        assert!(state.selected);
        assert!(state.focused);
        assert!(state.hovered);
        assert!(state.enabled);
    }

    #[test]
    fn test_style_option_default() {
        let option = StyleOptionViewItem::default();
        assert!(option.state.enabled);
        assert!(option.features.show_selection);
        assert!(option.text.is_none());
        assert!(option.icon.is_none());
    }

    #[test]
    fn test_style_option_builder() {
        let option = StyleOptionViewItem::new(Rect::new(0.0, 0.0, 100.0, 24.0), ModelIndex::invalid())
            .with_text("Hello")
            .with_state(ViewItemState::new().with_selected(true));

        assert_eq!(option.rect.width(), 100.0);
        assert_eq!(option.text.as_deref(), Some("Hello"));
        assert!(option.state.selected);
    }

    #[test]
    fn test_delegate_theme_default() {
        let theme = DelegateTheme::default();
        assert_eq!(theme.selection_text, Color::WHITE);
        assert!(theme.disabled_text.r > 0.5); // Grayish
    }

    #[test]
    fn test_default_delegate_size_hint() {
        let delegate = DefaultItemDelegate::new();
        let option = StyleOptionViewItem::default().with_text("Test");

        let (width, height) = delegate.size_hint(&option);
        assert!(width > 0.0);
        assert!(height >= 24.0);
    }

    #[test]
    fn test_default_delegate_size_hint_with_check() {
        let delegate = DefaultItemDelegate::new();
        let mut option = StyleOptionViewItem::default().with_text("Test");
        option.flags.checkable = true;

        let (width_with_check, _) = delegate.size_hint(&option);

        option.flags.checkable = false;
        let (width_without_check, _) = delegate.size_hint(&option);

        assert!(width_with_check > width_without_check);
    }

    #[test]
    fn test_click_region_detection() {
        let delegate = DefaultItemDelegate::new();
        let mut option =
            StyleOptionViewItem::new(Rect::new(0.0, 0.0, 200.0, 24.0), ModelIndex::invalid());
        option.flags.checkable = true;

        // Click on checkbox area (left side after padding)
        let region = delegate.click_region(&option, Point::new(10.0, 12.0));
        assert_eq!(region, ClickRegion::CheckIndicator);

        // Click on text area (right side)
        let region = delegate.click_region(&option, Point::new(100.0, 12.0));
        assert_eq!(region, ClickRegion::Item);

        // Click outside
        let region = delegate.click_region(&option, Point::new(300.0, 12.0));
        assert_eq!(region, ClickRegion::None);
    }
}
