//! GroupBox widget implementation.
//!
//! This module provides [`GroupBox`], a container widget with a title label
//! and optional checkbox for enabling/disabling the group content.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::GroupBox;
//! use horizon_lattice::widget::layout::LayoutKind;
//!
//! // Create a simple group box with a title
//! let mut group = GroupBox::new("Settings")
//!     .with_layout(LayoutKind::vertical());
//!
//! // Create a checkable group box (content hidden when unchecked)
//! let mut checkable = GroupBox::new("Advanced Options")
//!     .with_checkable(true)
//!     .with_checked(false); // Start unchecked
//!
//! checkable.toggled.connect(|&checked| {
//!     println!("Group is now {}", if checked { "enabled" } else { "disabled" });
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontSystem, Path, Point, Rect, Renderer, RoundedRect, Size, Stroke, TextLayout,
    TextRenderer,
};

use super::frame::{FrameShadow, FrameShape};
use crate::widget::dispatcher::WidgetAccess;
use crate::widget::layout::{ContentMargins, LayoutItem, LayoutKind};
use crate::widget::{PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase, WidgetEvent};

/// A container widget with a title and optional checkbox.
///
/// `GroupBox` provides a frame around a group of widgets with a title label
/// at the top. It can optionally include a checkbox that controls the visibility
/// of the group's content.
///
/// # Features
///
/// - Title label with customizable font and color
/// - Frame decoration with shape and shadow options
/// - Optional checkbox mode for enabling/disabling content
/// - Layout support for child positioning
/// - Content margins for padding
///
/// # Checkbox Mode
///
/// When checkbox mode is enabled:
/// - A checkbox appears in the title area before the title text
/// - When unchecked, child widgets are hidden (not just disabled)
/// - The `toggled` signal is emitted when the checkbox state changes
///
/// # Signals
///
/// - `toggled(bool)`: Emitted when the checkbox state changes (checkable mode only)
/// - `children_changed()`: Emitted when children are added or removed
pub struct GroupBox {
    /// Widget base.
    base: WidgetBase,

    /// Title text displayed at the top.
    title: String,

    /// Font for title rendering.
    title_font: Font,

    /// Title text color.
    title_color: Color,

    /// The frame shape.
    shape: FrameShape,

    /// The frame shadow effect.
    shadow: FrameShadow,

    /// Border width in pixels.
    line_width: f32,

    /// Border color.
    border_color: Color,

    /// Content margins (padding inside the frame below the title).
    content_margins: ContentMargins,

    /// Background color (if any).
    background_color: Option<Color>,

    /// Whether the group box has a checkbox.
    checkable: bool,

    /// Whether the checkbox is checked (content visible).
    checked: bool,

    /// Size of the checkbox indicator.
    checkbox_size: f32,

    /// Spacing between checkbox and title.
    checkbox_spacing: f32,

    /// Child widget IDs.
    children: Vec<ObjectId>,

    /// Optional layout for child positioning.
    layout: Option<LayoutKind>,

    /// Signal emitted when the checkbox is toggled.
    pub toggled: Signal<bool>,

    /// Signal emitted when children are added or removed.
    pub children_changed: Signal<()>,
}

impl GroupBox {
    /// Create a new group box with the specified title.
    pub fn new(title: impl Into<String>) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred));

        Self {
            base,
            title: title.into(),
            title_font: Font::default(),
            title_color: Color::from_rgb8(33, 33, 33),
            shape: FrameShape::Box,
            shadow: FrameShadow::Plain,
            line_width: 1.0,
            border_color: Color::from_rgb8(200, 200, 200),
            content_margins: ContentMargins::uniform(8.0),
            background_color: None,
            checkable: false,
            checked: true,
            checkbox_size: 16.0,
            checkbox_spacing: 6.0,
            children: Vec::new(),
            layout: None,
            toggled: Signal::new(),
            children_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Title
    // =========================================================================

    /// Get the title text.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the title text.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.base.update();
    }

    /// Set title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Get the title font.
    pub fn title_font(&self) -> &Font {
        &self.title_font
    }

    /// Set the title font.
    pub fn set_title_font(&mut self, font: Font) {
        self.title_font = font;
        self.base.update();
    }

    /// Set title font using builder pattern.
    pub fn with_title_font(mut self, font: Font) -> Self {
        self.title_font = font;
        self
    }

    /// Get the title color.
    pub fn title_color(&self) -> Color {
        self.title_color
    }

    /// Set the title color.
    pub fn set_title_color(&mut self, color: Color) {
        self.title_color = color;
        self.base.update();
    }

    /// Set title color using builder pattern.
    pub fn with_title_color(mut self, color: Color) -> Self {
        self.title_color = color;
        self
    }

    // =========================================================================
    // Frame Style
    // =========================================================================

    /// Get the frame shape.
    pub fn shape(&self) -> FrameShape {
        self.shape
    }

    /// Set the frame shape.
    pub fn set_shape(&mut self, shape: FrameShape) {
        if self.shape != shape {
            self.shape = shape;
            self.base.update();
        }
    }

    /// Set shape using builder pattern.
    pub fn with_shape(mut self, shape: FrameShape) -> Self {
        self.shape = shape;
        self
    }

    /// Get the frame shadow.
    pub fn shadow(&self) -> FrameShadow {
        self.shadow
    }

    /// Set the frame shadow.
    pub fn set_shadow(&mut self, shadow: FrameShadow) {
        if self.shadow != shadow {
            self.shadow = shadow;
            self.base.update();
        }
    }

    /// Set shadow using builder pattern.
    pub fn with_shadow(mut self, shadow: FrameShadow) -> Self {
        self.shadow = shadow;
        self
    }

    /// Get the border line width.
    pub fn line_width(&self) -> f32 {
        self.line_width
    }

    /// Set the border line width.
    pub fn set_line_width(&mut self, width: f32) {
        if (self.line_width - width).abs() > f32::EPSILON {
            self.line_width = width.max(0.0);
            self.base.update();
        }
    }

    /// Set line width using builder pattern.
    pub fn with_line_width(mut self, width: f32) -> Self {
        self.line_width = width.max(0.0);
        self
    }

    /// Get the border color.
    pub fn border_color(&self) -> Color {
        self.border_color
    }

    /// Set the border color.
    pub fn set_border_color(&mut self, color: Color) {
        if self.border_color != color {
            self.border_color = color;
            self.base.update();
        }
    }

    /// Set border color using builder pattern.
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    // =========================================================================
    // Checkbox Mode
    // =========================================================================

    /// Check if the group box has a checkbox.
    pub fn is_checkable(&self) -> bool {
        self.checkable
    }

    /// Enable or disable checkbox mode.
    ///
    /// When enabled, a checkbox appears in the title area.
    /// Child widgets are hidden when the checkbox is unchecked.
    pub fn set_checkable(&mut self, checkable: bool) {
        if self.checkable != checkable {
            self.checkable = checkable;
            self.base.update();
        }
    }

    /// Set checkable using builder pattern.
    pub fn with_checkable(mut self, checkable: bool) -> Self {
        self.checkable = checkable;
        self
    }

    /// Check if the checkbox is checked (content visible).
    ///
    /// Returns `true` if not in checkable mode.
    pub fn is_checked(&self) -> bool {
        if self.checkable {
            self.checked
        } else {
            true
        }
    }

    /// Set the checked state.
    ///
    /// Only has effect if checkable mode is enabled.
    pub fn set_checked(&mut self, checked: bool) {
        if self.checkable && self.checked != checked {
            self.checked = checked;
            self.toggled.emit(checked);
            self.base.update();
        }
    }

    /// Set checked state using builder pattern.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Toggle the checked state.
    ///
    /// Only has effect if checkable mode is enabled.
    pub fn toggle(&mut self) {
        if self.checkable {
            self.set_checked(!self.checked);
        }
    }

    // =========================================================================
    // Content Margins
    // =========================================================================

    /// Get the content margins.
    pub fn content_margins(&self) -> ContentMargins {
        self.content_margins
    }

    /// Set the content margins.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        if self.content_margins != margins {
            self.content_margins = margins;
            if let Some(layout) = &mut self.layout {
                layout.set_content_margins(margins);
            }
            self.base.update();
        }
    }

    /// Set uniform content margins.
    pub fn set_content_margin(&mut self, margin: f32) {
        self.set_content_margins(ContentMargins::uniform(margin));
    }

    /// Set content margins using builder pattern.
    pub fn with_content_margins(mut self, margins: ContentMargins) -> Self {
        self.set_content_margins(margins);
        self
    }

    /// Set uniform content margins using builder pattern.
    pub fn with_content_margin(mut self, margin: f32) -> Self {
        self.set_content_margin(margin);
        self
    }

    // =========================================================================
    // Background Color
    // =========================================================================

    /// Get the background color.
    pub fn background_color(&self) -> Option<Color> {
        self.background_color
    }

    /// Set the background color.
    pub fn set_background_color(&mut self, color: Option<Color>) {
        if self.background_color != color {
            self.background_color = color;
            self.base.update();
        }
    }

    /// Set background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = Some(color);
        self
    }

    // =========================================================================
    // Layout Management
    // =========================================================================

    /// Get the layout, if any.
    pub fn layout(&self) -> Option<&LayoutKind> {
        self.layout.as_ref()
    }

    /// Get a mutable reference to the layout.
    pub fn layout_mut(&mut self) -> Option<&mut LayoutKind> {
        self.layout.as_mut()
    }

    /// Set the layout for child positioning.
    ///
    /// Existing children will be added to the new layout.
    pub fn set_layout(&mut self, layout: LayoutKind) {
        let mut new_layout = layout;
        new_layout.set_parent_widget(Some(self.base.object_id()));

        // Add existing children to the new layout
        for &child_id in &self.children {
            new_layout.add_widget(child_id);
        }

        self.layout = Some(new_layout);
        self.base.update();
    }

    /// Set layout using builder pattern.
    pub fn with_layout(mut self, layout: LayoutKind) -> Self {
        self.set_layout(layout);
        self
    }

    /// Check if the group has a layout.
    #[inline]
    pub fn has_layout(&self) -> bool {
        self.layout.is_some()
    }

    // =========================================================================
    // Child Management
    // =========================================================================

    /// Add a child widget to this group box.
    ///
    /// If a layout is set, the widget is also added to the layout.
    /// Returns the index of the new child.
    pub fn add_child(&mut self, widget_id: ObjectId) -> usize {
        self.children.push(widget_id);

        if let Some(layout) = &mut self.layout {
            layout.add_widget(widget_id);
        }

        self.base.update();
        self.children_changed.emit(());
        self.children.len() - 1
    }

    /// Insert a child widget at the specified index.
    ///
    /// Returns the actual index where the widget was inserted.
    pub fn insert_child(&mut self, index: usize, widget_id: ObjectId) -> usize {
        let insert_pos = index.min(self.children.len());
        self.children.insert(insert_pos, widget_id);

        if let Some(layout) = &mut self.layout {
            layout.insert_item(insert_pos, LayoutItem::Widget(widget_id));
        }

        self.base.update();
        self.children_changed.emit(());
        insert_pos
    }

    /// Remove the child widget at the specified index.
    ///
    /// Returns the widget ID of the removed child, if any.
    pub fn remove_child(&mut self, index: usize) -> Option<ObjectId> {
        if index >= self.children.len() {
            return None;
        }

        let widget_id = self.children.remove(index);

        if let Some(layout) = &mut self.layout {
            layout.remove_item(index);
        }

        self.base.update();
        self.children_changed.emit(());
        Some(widget_id)
    }

    /// Remove a child widget by its ID.
    ///
    /// Returns `true` if the widget was found and removed.
    pub fn remove_child_by_id(&mut self, widget_id: ObjectId) -> bool {
        if let Some(index) = self.children.iter().position(|&id| id == widget_id) {
            self.remove_child(index);
            true
        } else {
            false
        }
    }

    /// Remove all children from the group box.
    pub fn clear(&mut self) {
        self.children.clear();

        if let Some(layout) = &mut self.layout {
            layout.clear();
        }

        self.base.update();
        self.children_changed.emit(());
    }

    /// Get the number of children.
    #[inline]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Check if the group box has no children.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Get the child widget IDs.
    #[inline]
    pub fn children(&self) -> &[ObjectId] {
        &self.children
    }

    /// Get the child at the specified index.
    #[inline]
    pub fn child_at(&self, index: usize) -> Option<ObjectId> {
        self.children.get(index).copied()
    }

    /// Find the index of a child widget.
    pub fn index_of(&self, widget_id: ObjectId) -> Option<usize> {
        self.children.iter().position(|&id| id == widget_id)
    }

    // =========================================================================
    // Content Area
    // =========================================================================

    /// Calculate the title height including spacing.
    fn title_height(&self) -> f32 {
        if self.title.is_empty() && !self.checkable {
            0.0
        } else {
            let mut font_system = FontSystem::new();
            let text = if self.title.is_empty() { "Xg" } else { &self.title };
            let layout = TextLayout::new(&mut font_system, text, &self.title_font);
            let text_height = layout.height().max(self.checkbox_size);
            text_height + 8.0 // Add spacing below title
        }
    }

    /// Calculate the effective frame width.
    fn frame_width(&self) -> f32 {
        match self.shape {
            FrameShape::NoFrame => 0.0,
            _ => self.line_width,
        }
    }

    /// Get the content area rectangle (inside the frame and below the title).
    pub fn contents_rect(&self) -> Rect {
        let rect = self.base.rect();
        let frame_width = self.frame_width();
        let title_h = self.title_height();

        Rect::new(
            frame_width + self.content_margins.left,
            frame_width + title_h + self.content_margins.top,
            (rect.width() - 2.0 * frame_width - self.content_margins.horizontal()).max(0.0),
            (rect.height() - 2.0 * frame_width - title_h - self.content_margins.vertical()).max(0.0),
        )
    }

    // =========================================================================
    // Layout Operations
    // =========================================================================

    /// Calculate and apply the layout using the provided widget storage.
    pub fn do_layout<S: WidgetAccess>(&mut self, storage: &mut S) {
        // Don't layout if unchecked (children are hidden)
        if !self.is_checked() {
            return;
        }

        let content_rect = self.contents_rect();
        let geo = self.base.geometry();

        let layout_rect = Rect::new(
            geo.origin.x + content_rect.origin.x,
            geo.origin.y + content_rect.origin.y,
            content_rect.width(),
            content_rect.height(),
        );

        if let Some(layout) = &mut self.layout {
            layout.set_geometry(layout_rect);
            layout.calculate(storage, layout_rect.size);
            layout.apply(storage);
        }
    }

    /// Invalidate the layout for recalculation.
    pub fn invalidate_layout(&mut self) {
        if let Some(layout) = &mut self.layout {
            layout.invalidate();
        }
        self.base.update();
    }

    // =========================================================================
    // Painting Helpers
    // =========================================================================

    /// Get light and dark colors for 3D effects.
    fn get_3d_colors(&self) -> (Color, Color) {
        let base = self.border_color;

        let light = Color::from_rgba(
            (base.r * 1.3).min(1.0),
            (base.g * 1.3).min(1.0),
            (base.b * 1.3).min(1.0),
            base.a,
        );
        let dark = Color::from_rgba(
            base.r * 0.6,
            base.g * 0.6,
            base.b * 0.6,
            base.a,
        );

        (light, dark)
    }

    /// Paint the frame with gap for title.
    fn paint_frame(&self, ctx: &mut PaintContext<'_>, title_rect: Option<Rect>) {
        let rect = ctx.rect();

        if rect.width() <= 0.0 || rect.height() <= 0.0 || self.shape == FrameShape::NoFrame {
            return;
        }

        if self.line_width <= 0.0 {
            return;
        }

        let (light, dark) = self.get_3d_colors();
        let (top_left_color, bottom_right_color) = match self.shadow {
            FrameShadow::Plain => (self.border_color, self.border_color),
            FrameShadow::Raised => (light, dark),
            FrameShadow::Sunken => (dark, light),
        };

        let lw = self.line_width;
        let inset = lw / 2.0;

        // Calculate frame rect (title sits on the top edge)
        let title_h = self.title_height();
        let frame_top = rect.origin.y + title_h / 2.0;
        let frame_rect = Rect::new(
            rect.origin.x + inset,
            frame_top,
            rect.width() - lw,
            rect.height() - title_h / 2.0 - lw / 2.0,
        );

        // Draw bottom edge
        let stroke_br = Stroke::new(bottom_right_color, lw);
        ctx.renderer().draw_line(
            Point::new(frame_rect.origin.x, frame_rect.origin.y + frame_rect.height()),
            Point::new(frame_rect.origin.x + frame_rect.width(), frame_rect.origin.y + frame_rect.height()),
            &stroke_br,
        );

        // Draw right edge
        ctx.renderer().draw_line(
            Point::new(frame_rect.origin.x + frame_rect.width(), frame_rect.origin.y),
            Point::new(frame_rect.origin.x + frame_rect.width(), frame_rect.origin.y + frame_rect.height()),
            &stroke_br,
        );

        // Draw left edge
        let stroke_tl = Stroke::new(top_left_color, lw);
        ctx.renderer().draw_line(
            Point::new(frame_rect.origin.x, frame_rect.origin.y),
            Point::new(frame_rect.origin.x, frame_rect.origin.y + frame_rect.height()),
            &stroke_tl,
        );

        // Draw top edge with gap for title
        if let Some(title_r) = title_rect {
            let title_gap_start = title_r.origin.x - 4.0;
            let title_gap_end = title_r.origin.x + title_r.width() + 4.0;

            // Left portion of top edge
            if title_gap_start > frame_rect.origin.x {
                ctx.renderer().draw_line(
                    Point::new(frame_rect.origin.x, frame_rect.origin.y),
                    Point::new(title_gap_start, frame_rect.origin.y),
                    &stroke_tl,
                );
            }

            // Right portion of top edge
            if title_gap_end < frame_rect.origin.x + frame_rect.width() {
                ctx.renderer().draw_line(
                    Point::new(title_gap_end, frame_rect.origin.y),
                    Point::new(frame_rect.origin.x + frame_rect.width(), frame_rect.origin.y),
                    &stroke_tl,
                );
            }
        } else {
            // No title, draw complete top edge
            ctx.renderer().draw_line(
                Point::new(frame_rect.origin.x, frame_rect.origin.y),
                Point::new(frame_rect.origin.x + frame_rect.width(), frame_rect.origin.y),
                &stroke_tl,
            );
        }
    }

    /// Paint the checkbox indicator.
    fn paint_checkbox(&self, ctx: &mut PaintContext<'_>, checkbox_rect: Rect) {
        let is_enabled = self.base.is_effectively_enabled();
        let is_hovered = self.base.is_hovered();

        // Background color
        let bg_color = if self.checked {
            if !is_enabled {
                Color::from_rgb8(189, 189, 189)
            } else if is_hovered {
                Color::from_rgb8(41, 163, 255)
            } else {
                Color::from_rgb8(33, 150, 243)
            }
        } else {
            Color::TRANSPARENT
        };

        // Border color
        let border_color = if !is_enabled {
            Color::from_rgb8(189, 189, 189)
        } else if self.checked {
            bg_color
        } else if is_hovered {
            Color::from_rgb8(117, 117, 117)
        } else {
            Color::from_rgb8(158, 158, 158)
        };

        let rrect = RoundedRect::new(checkbox_rect, 3.0);

        // Fill background if checked
        if bg_color != Color::TRANSPARENT {
            ctx.renderer().fill_rounded_rect(rrect, bg_color);
        }

        // Draw border
        let border_stroke = Stroke::new(border_color, 1.5);
        ctx.renderer().stroke_rounded_rect(rrect, &border_stroke);

        // Draw checkmark if checked
        if self.checked {
            let check_color = if is_enabled {
                Color::WHITE
            } else {
                Color::from_rgb8(158, 158, 158)
            };

            let padding = checkbox_rect.width() * 0.2;
            let inner_rect = Rect::new(
                checkbox_rect.origin.x + padding,
                checkbox_rect.origin.y + padding,
                checkbox_rect.width() - padding * 2.0,
                checkbox_rect.height() - padding * 2.0,
            );

            let start = Point::new(
                inner_rect.origin.x,
                inner_rect.origin.y + inner_rect.height() * 0.5,
            );
            let middle = Point::new(
                inner_rect.origin.x + inner_rect.width() * 0.35,
                inner_rect.origin.y + inner_rect.height() * 0.75,
            );
            let end = Point::new(
                inner_rect.origin.x + inner_rect.width(),
                inner_rect.origin.y + inner_rect.height() * 0.15,
            );

            let mut path = Path::new();
            path.move_to(start);
            path.line_to(middle);
            path.line_to(end);

            let stroke = Stroke::new(check_color, 2.0);
            ctx.renderer().stroke_path(&path, &stroke);
        }
    }

    /// Calculate the title area rect (checkbox + text).
    fn calculate_title_rect(&self, rect: Rect) -> Option<Rect> {
        if self.title.is_empty() && !self.checkable {
            return None;
        }

        let mut font_system = FontSystem::new();
        let title_margin_left = self.frame_width() + 8.0;
        let x = rect.origin.x + title_margin_left;
        let mut total_width = 0.0f32;

        if self.checkable {
            total_width += self.checkbox_size + self.checkbox_spacing;
        }

        if !self.title.is_empty() {
            let layout = TextLayout::new(&mut font_system, &self.title, &self.title_font);
            total_width += layout.width();
        }

        let title_height = self.title_height();

        Some(Rect::new(x, rect.origin.y, total_width, title_height))
    }

    /// Check if a point is within the checkbox area.
    fn is_point_in_checkbox(&self, local_pos: Point) -> bool {
        if !self.checkable {
            return false;
        }

        let title_margin_left = self.frame_width() + 8.0;
        let title_height = self.title_height();

        let checkbox_x = title_margin_left;
        let checkbox_y = (title_height - self.checkbox_size) / 2.0;

        let checkbox_rect = Rect::new(
            checkbox_x,
            checkbox_y,
            self.checkbox_size,
            self.checkbox_size,
        );

        // Expand hit area slightly for easier clicking
        let hit_rect = checkbox_rect.inflate(4.0);
        hit_rect.contains(local_pos)
    }
}

impl Default for GroupBox {
    fn default() -> Self {
        Self::new("")
    }
}

impl Object for GroupBox {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for GroupBox {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let frame_width = self.frame_width();
        let title_h = self.title_height();

        let min_width = 2.0 * frame_width + self.content_margins.horizontal();
        let min_height = 2.0 * frame_width + title_h + self.content_margins.vertical();

        let preferred = Size::new(
            min_width.max(150.0),
            min_height.max(50.0),
        );

        SizeHint::new(preferred).with_minimum(Size::new(min_width, min_height))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Draw background if set
        if let Some(bg_color) = self.background_color {
            ctx.renderer().fill_rect(rect, bg_color);
        }

        // Calculate title rect for frame gap
        let title_rect = self.calculate_title_rect(rect);

        // Draw frame with gap for title
        self.paint_frame(ctx, title_rect);

        // Draw title area (checkbox + text)
        if self.title.is_empty() && !self.checkable {
            return;
        }

        let title_margin_left = self.frame_width() + 8.0;
        let title_height = self.title_height();
        let mut x = rect.origin.x + title_margin_left;

        // Draw checkbox if checkable
        if self.checkable {
            let checkbox_y = rect.origin.y + (title_height - self.checkbox_size) / 2.0;
            let checkbox_rect = Rect::new(x, checkbox_y, self.checkbox_size, self.checkbox_size);
            self.paint_checkbox(ctx, checkbox_rect);
            x += self.checkbox_size + self.checkbox_spacing;
        }

        // Draw title text
        if !self.title.is_empty() {
            let mut font_system = FontSystem::new();
            let layout = TextLayout::new(&mut font_system, &self.title, &self.title_font);

            let text_y = rect.origin.y + (title_height - layout.height()) / 2.0;
            let text_pos = Point::new(x, text_y);

            let text_color = if self.base.is_effectively_enabled() {
                self.title_color
            } else {
                Color::from_rgb8(158, 158, 158)
            };

            if let Ok(mut text_renderer) = TextRenderer::new() {
                let _ = text_renderer.prepare_layout(
                    &mut font_system,
                    &layout,
                    text_pos,
                    text_color,
                );
            }
        }

        // Child widgets are painted separately by the paint system
        // Children should be hidden when unchecked - handled by visibility system
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MouseRelease(e) => {
                if !self.base.is_effectively_enabled() {
                    return false;
                }

                if e.button == crate::widget::MouseButton::Left {
                    if self.is_point_in_checkbox(e.local_pos) {
                        self.toggle();
                        event.accept();
                        return true;
                    }
                }
                false
            }
            _ => false,
        }
    }
}

// Ensure GroupBox is Send + Sync
static_assertions::assert_impl_all!(GroupBox: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_group_box_creation() {
        setup();
        let group = GroupBox::new("Test Group");
        assert_eq!(group.title(), "Test Group");
        assert!(!group.is_checkable());
        assert!(group.is_checked());
        assert!(group.is_empty());
    }

    #[test]
    fn test_group_box_builder_pattern() {
        setup();
        let group = GroupBox::new("Settings")
            .with_checkable(true)
            .with_checked(false)
            .with_content_margin(12.0)
            .with_shape(FrameShape::Panel)
            .with_shadow(FrameShadow::Sunken)
            .with_background_color(Color::WHITE);

        assert_eq!(group.title(), "Settings");
        assert!(group.is_checkable());
        assert!(!group.is_checked());
        assert_eq!(group.content_margins().left, 12.0);
        assert_eq!(group.shape(), FrameShape::Panel);
        assert_eq!(group.shadow(), FrameShadow::Sunken);
        assert_eq!(group.background_color(), Some(Color::WHITE));
    }

    #[test]
    fn test_checkbox_mode() {
        setup();
        let mut group = GroupBox::new("Options")
            .with_checkable(true);

        assert!(group.is_checkable());
        assert!(group.is_checked()); // Default checked

        group.set_checked(false);
        assert!(!group.is_checked());

        group.toggle();
        assert!(group.is_checked());
    }

    #[test]
    fn test_non_checkable_always_checked() {
        setup();
        let mut group = GroupBox::new("Static Group");

        // Non-checkable groups are always considered "checked"
        assert!(!group.is_checkable());
        assert!(group.is_checked());

        // set_checked has no effect when not checkable
        group.set_checked(false);
        assert!(group.is_checked());
    }

    #[test]
    fn test_toggled_signal() {
        setup();
        let mut group = GroupBox::new("Toggle Test")
            .with_checkable(true);

        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_clone = signal_received.clone();

        group.toggled.connect(move |&checked| {
            signal_clone.store(!checked, Ordering::SeqCst);
        });

        group.set_checked(false);
        assert!(signal_received.load(Ordering::SeqCst));
    }

    #[test]
    fn test_child_management() {
        setup();
        let mut group = GroupBox::new("Container");
        let container1 = super::super::ContainerWidget::new();
        let container2 = super::super::ContainerWidget::new();
        let id1 = container1.object_id();
        let id2 = container2.object_id();

        // Add children
        let index1 = group.add_child(id1);
        let index2 = group.add_child(id2);

        assert_eq!(index1, 0);
        assert_eq!(index2, 1);
        assert_eq!(group.child_count(), 2);
        assert_eq!(group.children(), &[id1, id2]);
        assert_eq!(group.child_at(0), Some(id1));
        assert_eq!(group.index_of(id2), Some(1));

        // Remove child
        let removed = group.remove_child(0);
        assert_eq!(removed, Some(id1));
        assert_eq!(group.child_count(), 1);

        // Clear
        group.clear();
        assert!(group.is_empty());
    }

    #[test]
    fn test_children_changed_signal() {
        setup();
        let mut group = GroupBox::new("Signal Test");
        let container = super::super::ContainerWidget::new();
        let id = container.object_id();

        let signal_count = Arc::new(AtomicU32::new(0));
        let signal_clone = signal_count.clone();

        group.children_changed.connect(move |_| {
            signal_clone.fetch_add(1, Ordering::SeqCst);
        });

        group.add_child(id);
        assert_eq!(signal_count.load(Ordering::SeqCst), 1);

        group.remove_child(0);
        assert_eq!(signal_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_layout_integration() {
        setup();
        let mut group = GroupBox::new("Layout Test")
            .with_layout(LayoutKind::vertical());

        assert!(group.has_layout());

        let container = super::super::ContainerWidget::new();
        let id = container.object_id();

        group.add_child(id);
        assert_eq!(group.layout().unwrap().item_count(), 1);
    }

    #[test]
    fn test_title_customization() {
        setup();
        let group = GroupBox::new("Custom")
            .with_title_color(Color::from_rgb8(255, 0, 0))
            .with_title_font(Font::default().with_size(14.0));

        assert_eq!(group.title_color(), Color::from_rgb8(255, 0, 0));
        assert_eq!(group.title_font().size(), 14.0);
    }

    #[test]
    fn test_frame_style() {
        setup();
        let group = GroupBox::new("Styled")
            .with_shape(FrameShape::StyledPanel)
            .with_shadow(FrameShadow::Raised)
            .with_line_width(2.0)
            .with_border_color(Color::from_rgb8(100, 100, 100));

        assert_eq!(group.shape(), FrameShape::StyledPanel);
        assert_eq!(group.shadow(), FrameShadow::Raised);
        assert_eq!(group.line_width(), 2.0);
        assert_eq!(group.border_color(), Color::from_rgb8(100, 100, 100));
    }

    #[test]
    fn test_contents_rect() {
        setup();
        let mut group = GroupBox::new("Content")
            .with_content_margin(10.0);

        group.widget_base_mut().set_geometry(Rect::new(0.0, 0.0, 200.0, 200.0));

        let content = group.contents_rect();
        // Content should be inset by frame_width (1) + margin (10) horizontally
        // And below title area + margin vertically
        assert!(content.origin.x >= 11.0);
        assert!(content.origin.y > 0.0); // Below title
        assert!(content.width() <= 178.0);
        assert!(content.height() < 200.0);
    }

    #[test]
    fn test_size_hint() {
        setup();
        let group = GroupBox::new("Size Test")
            .with_content_margin(20.0);

        let hint = group.size_hint();

        // Should have reasonable minimum and preferred sizes
        assert!(hint.effective_minimum().width > 0.0);
        assert!(hint.effective_minimum().height > 0.0);
        assert!(hint.preferred.width >= hint.effective_minimum().width);
        assert!(hint.preferred.height >= hint.effective_minimum().height);
    }
}
