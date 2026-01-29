//! DockWidget implementation.
//!
//! This module provides [`DockWidget`], a dockable panel that can be placed in
//! dock areas of a [`MainWindow`] or floated as an independent panel.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{DockWidget, DockArea, DockWidgetFeatures};
//!
//! // Create a dock widget
//! let mut dock = DockWidget::new("Properties")
//!     .with_features(DockWidgetFeatures::all())
//!     .with_allowed_areas(DockArea::Left | DockArea::Right);
//!
//! // Set the content widget
//! dock.set_widget(properties_panel_id);
//!
//! // Connect to signals
//! dock.top_level_changed.connect(|&floating| {
//!     println!("Dock is floating: {}", floating);
//! });
//! ```

use std::ops::{BitAnd, BitOr, BitOrAssign};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Size, Stroke};

use crate::widget::layout::ContentMargins;
use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseDoubleClickEvent, MouseMoveEvent,
    MousePressEvent, MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget,
    WidgetBase, WidgetEvent,
};

/// Dock widget areas within a MainWindow.
///
/// These define the regions where dock widgets can be placed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DockArea {
    /// Left dock area.
    #[default]
    Left,
    /// Right dock area.
    Right,
    /// Top dock area.
    Top,
    /// Bottom dock area.
    Bottom,
}

impl DockArea {
    /// Returns all dock areas as an iterator.
    pub fn all() -> impl Iterator<Item = DockArea> {
        [
            DockArea::Left,
            DockArea::Right,
            DockArea::Top,
            DockArea::Bottom,
        ]
        .into_iter()
    }

    /// Check if this is a horizontal dock area (left or right).
    pub fn is_horizontal(&self) -> bool {
        matches!(self, DockArea::Left | DockArea::Right)
    }

    /// Check if this is a vertical dock area (top or bottom).
    pub fn is_vertical(&self) -> bool {
        matches!(self, DockArea::Top | DockArea::Bottom)
    }
}

/// A set of dock areas represented as bit flags.
///
/// # Example
///
/// ```ignore
/// let areas = DockAreas::LEFT | DockAreas::RIGHT;
/// assert!(areas.contains(DockAreas::LEFT));
/// assert!(!areas.contains(DockAreas::TOP));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DockAreas(u8);

impl DockAreas {
    /// No dock areas.
    pub const NONE: DockAreas = DockAreas(0);
    /// Left dock area.
    pub const LEFT: DockAreas = DockAreas(1 << 0);
    /// Right dock area.
    pub const RIGHT: DockAreas = DockAreas(1 << 1);
    /// Top dock area.
    pub const TOP: DockAreas = DockAreas(1 << 2);
    /// Bottom dock area.
    pub const BOTTOM: DockAreas = DockAreas(1 << 3);
    /// All dock areas.
    pub const ALL: DockAreas = DockAreas(0b1111);

    /// Create from a single dock area.
    pub fn from_area(area: DockArea) -> Self {
        match area {
            DockArea::Left => Self::LEFT,
            DockArea::Right => Self::RIGHT,
            DockArea::Top => Self::TOP,
            DockArea::Bottom => Self::BOTTOM,
        }
    }

    /// Check if this set contains the specified area.
    pub fn contains(&self, area: DockArea) -> bool {
        let area_flag = Self::from_area(area);
        (self.0 & area_flag.0) != 0
    }

    /// Check if this set contains the specified areas.
    pub fn contains_areas(&self, areas: DockAreas) -> bool {
        (self.0 & areas.0) == areas.0
    }

    /// Check if this set is empty.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Get all areas as an iterator.
    pub fn iter(&self) -> impl Iterator<Item = DockArea> + '_ {
        DockArea::all().filter(|&area| self.contains(area))
    }
}

impl BitOr for DockAreas {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        DockAreas(self.0 | rhs.0)
    }
}

impl BitOrAssign for DockAreas {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for DockAreas {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        DockAreas(self.0 & rhs.0)
    }
}

impl BitOr<DockArea> for DockAreas {
    type Output = Self;

    fn bitor(self, rhs: DockArea) -> Self::Output {
        self | Self::from_area(rhs)
    }
}

impl BitOr<DockAreas> for DockArea {
    type Output = DockAreas;

    fn bitor(self, rhs: DockAreas) -> Self::Output {
        DockAreas::from_area(self) | rhs
    }
}

impl BitOr for DockArea {
    type Output = DockAreas;

    fn bitor(self, rhs: Self) -> Self::Output {
        DockAreas::from_area(self) | DockAreas::from_area(rhs)
    }
}

/// Feature flags for dock widgets.
///
/// These flags control what operations the user can perform on a dock widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DockWidgetFeatures(u8);

impl DockWidgetFeatures {
    /// No features enabled.
    pub const NONE: DockWidgetFeatures = DockWidgetFeatures(0);
    /// The dock widget can be closed.
    pub const CLOSABLE: DockWidgetFeatures = DockWidgetFeatures(1 << 0);
    /// The dock widget can be moved between dock areas.
    pub const MOVABLE: DockWidgetFeatures = DockWidgetFeatures(1 << 1);
    /// The dock widget can float as an independent panel.
    pub const FLOATABLE: DockWidgetFeatures = DockWidgetFeatures(1 << 2);
    /// The title bar is displayed vertically on the left side.
    pub const VERTICAL_TITLE_BAR: DockWidgetFeatures = DockWidgetFeatures(1 << 3);

    /// All standard features (closable, movable, floatable).
    pub fn all() -> Self {
        Self::CLOSABLE | Self::MOVABLE | Self::FLOATABLE
    }

    /// Check if a feature is enabled.
    pub fn has(&self, feature: DockWidgetFeatures) -> bool {
        (self.0 & feature.0) == feature.0
    }

    /// Check if closable.
    pub fn is_closable(&self) -> bool {
        self.has(Self::CLOSABLE)
    }

    /// Check if movable.
    pub fn is_movable(&self) -> bool {
        self.has(Self::MOVABLE)
    }

    /// Check if floatable.
    pub fn is_floatable(&self) -> bool {
        self.has(Self::FLOATABLE)
    }

    /// Check if vertical title bar.
    pub fn has_vertical_title_bar(&self) -> bool {
        self.has(Self::VERTICAL_TITLE_BAR)
    }
}

impl BitOr for DockWidgetFeatures {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        DockWidgetFeatures(self.0 | rhs.0)
    }
}

impl BitOrAssign for DockWidgetFeatures {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for DockWidgetFeatures {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        DockWidgetFeatures(self.0 & rhs.0)
    }
}

// ============================================================================
// Title Bar Button
// ============================================================================

/// Type of title bar button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TitleBarButton {
    Float,
    Close,
}

/// State of a title bar button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ButtonState {
    hovered: bool,
    pressed: bool,
}

// ============================================================================
// DockWidget
// ============================================================================

/// A dockable panel widget.
///
/// DockWidget provides a panel that can be docked in a `MainWindow`'s dock
/// areas or floated as an independent panel. It consists of a title bar with
/// controls and a content area.
///
/// # Features
///
/// - Title bar with window title, float button, and close button
/// - Dockable to left, right, top, or bottom areas of MainWindow
/// - Floatable as an independent panel within the window
/// - Configurable features (closable, movable, floatable)
/// - Vertical title bar option for space-efficient layouts
///
/// # Signals
///
/// - `top_level_changed(bool)`: Emitted when floating state changes
/// - `visibility_changed(bool)`: Emitted when visibility changes
/// - `dock_location_changed(DockArea)`: Emitted when dock area changes
/// - `features_changed(DockWidgetFeatures)`: Emitted when features change
/// - `close_requested()`: Emitted when close button is clicked
pub struct DockWidget {
    /// Widget base.
    base: WidgetBase,

    /// The title displayed in the title bar.
    title: String,

    /// The content widget ID.
    widget: Option<ObjectId>,

    /// Which dock areas this widget can be docked in.
    allowed_areas: DockAreas,

    /// The current dock area (if docked).
    dock_area: Option<DockArea>,

    /// Feature flags controlling user interactions.
    features: DockWidgetFeatures,

    /// Whether the widget is currently floating.
    floating: bool,

    /// Position when floating (relative to parent).
    float_position: Point,

    /// Size when floating.
    float_size: Size,

    /// Title bar height.
    title_bar_height: f32,

    /// Title bar button size.
    button_size: f32,

    /// Content margins inside the dock widget.
    content_margins: ContentMargins,

    // Visual styling
    /// Title bar background color.
    title_bar_color: Color,
    /// Title bar background when active/focused.
    title_bar_active_color: Color,
    /// Title text color.
    title_text_color: Color,
    /// Background color of content area.
    content_background: Color,
    /// Border color.
    border_color: Color,
    /// Border width.
    border_width: f32,
    /// Button background color.
    button_color: Color,
    /// Button hover color.
    button_hover_color: Color,
    /// Button pressed color.
    button_pressed_color: Color,

    // Interaction state
    /// Button states.
    float_button_state: ButtonState,
    close_button_state: ButtonState,
    /// Whether dragging the title bar.
    dragging: bool,
    /// Drag start position (in parent coordinates).
    drag_start: Point,
    /// Widget position at drag start.
    drag_start_pos: Point,

    // Signals
    /// Signal emitted when floating state changes.
    pub top_level_changed: Signal<bool>,
    /// Signal emitted when visibility changes.
    pub visibility_changed: Signal<bool>,
    /// Signal emitted when dock area changes.
    pub dock_location_changed: Signal<DockArea>,
    /// Signal emitted when features change.
    pub features_changed: Signal<DockWidgetFeatures>,
    /// Signal emitted when close is requested.
    pub close_requested: Signal<()>,
}

impl DockWidget {
    /// Create a new dock widget with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::ClickFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Preferred,
        ));

        Self {
            base,
            title: title.into(),
            widget: None,
            allowed_areas: DockAreas::ALL,
            dock_area: None,
            features: DockWidgetFeatures::all(),
            floating: false,
            float_position: Point::ZERO,
            float_size: Size::new(200.0, 150.0),
            title_bar_height: 24.0,
            button_size: 16.0,
            content_margins: ContentMargins::uniform(1.0),
            title_bar_color: Color::from_rgb8(240, 240, 240),
            title_bar_active_color: Color::from_rgb8(200, 220, 240),
            title_text_color: Color::from_rgb8(40, 40, 40),
            content_background: Color::WHITE,
            border_color: Color::from_rgb8(180, 180, 180),
            border_width: 1.0,
            button_color: Color::from_rgb8(220, 220, 220),
            button_hover_color: Color::from_rgb8(200, 200, 200),
            button_pressed_color: Color::from_rgb8(180, 180, 180),
            float_button_state: ButtonState::default(),
            close_button_state: ButtonState::default(),
            dragging: false,
            drag_start: Point::ZERO,
            drag_start_pos: Point::ZERO,
            top_level_changed: Signal::new(),
            visibility_changed: Signal::new(),
            dock_location_changed: Signal::new(),
            features_changed: Signal::new(),
            close_requested: Signal::new(),
        }
    }

    // =========================================================================
    // Title
    // =========================================================================

    /// Get the window title.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Set the window title.
    pub fn set_title(&mut self, title: impl Into<String>) {
        self.title = title.into();
        self.base.update();
    }

    /// Set title using builder pattern.
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    // =========================================================================
    // Content Widget
    // =========================================================================

    /// Get the content widget ID.
    pub fn widget(&self) -> Option<ObjectId> {
        self.widget
    }

    /// Set the content widget.
    pub fn set_widget(&mut self, widget_id: ObjectId) {
        self.widget = Some(widget_id);
        self.base.update();
    }

    /// Set widget using builder pattern.
    pub fn with_widget(mut self, widget_id: ObjectId) -> Self {
        self.widget = Some(widget_id);
        self
    }

    // =========================================================================
    // Allowed Areas
    // =========================================================================

    /// Get the allowed dock areas.
    pub fn allowed_areas(&self) -> DockAreas {
        self.allowed_areas
    }

    /// Set the allowed dock areas.
    pub fn set_allowed_areas(&mut self, areas: DockAreas) {
        self.allowed_areas = areas;
    }

    /// Set allowed areas using builder pattern.
    pub fn with_allowed_areas(mut self, areas: DockAreas) -> Self {
        self.allowed_areas = areas;
        self
    }

    /// Check if a specific area is allowed.
    pub fn is_area_allowed(&self, area: DockArea) -> bool {
        self.allowed_areas.contains(area)
    }

    // =========================================================================
    // Dock Location
    // =========================================================================

    /// Get the current dock area (None if floating).
    pub fn dock_area(&self) -> Option<DockArea> {
        self.dock_area
    }

    /// Set the dock area. This will dock the widget to the specified area.
    ///
    /// If the area is not allowed, this has no effect.
    pub fn set_dock_area(&mut self, area: DockArea) {
        if !self.is_area_allowed(area) {
            return;
        }

        let was_floating = self.floating;
        self.dock_area = Some(area);
        self.floating = false;

        if was_floating {
            self.top_level_changed.emit(false);
        }
        self.dock_location_changed.emit(area);
        self.base.update();
    }

    // =========================================================================
    // Features
    // =========================================================================

    /// Get the enabled features.
    pub fn features(&self) -> DockWidgetFeatures {
        self.features
    }

    /// Set the enabled features.
    pub fn set_features(&mut self, features: DockWidgetFeatures) {
        if self.features != features {
            self.features = features;
            self.features_changed.emit(features);
            self.base.update();
        }
    }

    /// Set features using builder pattern.
    pub fn with_features(mut self, features: DockWidgetFeatures) -> Self {
        self.features = features;
        self
    }

    // =========================================================================
    // Floating
    // =========================================================================

    /// Check if the widget is floating.
    pub fn is_floating(&self) -> bool {
        self.floating
    }

    /// Set the floating state.
    pub fn set_floating(&mut self, floating: bool) {
        if !self.features.is_floatable() && floating {
            return;
        }

        if self.floating != floating {
            self.floating = floating;
            if floating {
                self.dock_area = None;
                // Set initial float position to current position if not set
                if self.float_position == Point::ZERO {
                    self.float_position = self.base.pos();
                }
            }
            self.top_level_changed.emit(floating);
            self.base.update();
        }
    }

    /// Toggle floating state.
    pub fn toggle_floating(&mut self) {
        self.set_floating(!self.floating);
    }

    /// Get the floating position.
    pub fn float_position(&self) -> Point {
        self.float_position
    }

    /// Set the floating position.
    pub fn set_float_position(&mut self, position: Point) {
        self.float_position = position;
        if self.floating {
            self.base.set_pos(position);
        }
    }

    /// Get the floating size.
    pub fn float_size(&self) -> Size {
        self.float_size
    }

    /// Set the floating size.
    pub fn set_float_size(&mut self, size: Size) {
        self.float_size = size;
        if self.floating {
            self.base.set_size(size);
        }
    }

    // =========================================================================
    // Styling
    // =========================================================================

    /// Set the title bar height.
    pub fn set_title_bar_height(&mut self, height: f32) {
        self.title_bar_height = height;
        self.base.update();
    }

    /// Set title bar height using builder pattern.
    pub fn with_title_bar_height(mut self, height: f32) -> Self {
        self.title_bar_height = height;
        self
    }

    /// Set the title bar color.
    pub fn set_title_bar_color(&mut self, color: Color) {
        self.title_bar_color = color;
        self.base.update();
    }

    /// Set title bar color using builder pattern.
    pub fn with_title_bar_color(mut self, color: Color) -> Self {
        self.title_bar_color = color;
        self
    }

    /// Set content margins.
    pub fn set_content_margins(&mut self, margins: ContentMargins) {
        self.content_margins = margins;
        self.base.update();
    }

    /// Set content margins using builder pattern.
    pub fn with_content_margins(mut self, margins: ContentMargins) -> Self {
        self.content_margins = margins;
        self
    }

    // =========================================================================
    // Geometry Calculations
    // =========================================================================

    /// Get the title bar rectangle.
    pub fn title_bar_rect(&self) -> Rect {
        let rect = self.base.rect();
        if self.features.has_vertical_title_bar() {
            Rect::new(0.0, 0.0, self.title_bar_height, rect.height())
        } else {
            Rect::new(0.0, 0.0, rect.width(), self.title_bar_height)
        }
    }

    /// Get the content area rectangle (excluding title bar and margins).
    pub fn content_rect(&self) -> Rect {
        let rect = self.base.rect();
        let title_bar = self.title_bar_rect();

        if self.features.has_vertical_title_bar() {
            Rect::new(
                title_bar.width() + self.content_margins.left,
                self.content_margins.top,
                rect.width() - title_bar.width() - self.content_margins.horizontal(),
                rect.height() - self.content_margins.vertical(),
            )
        } else {
            Rect::new(
                self.content_margins.left,
                title_bar.height() + self.content_margins.top,
                rect.width() - self.content_margins.horizontal(),
                rect.height() - title_bar.height() - self.content_margins.vertical(),
            )
        }
    }

    /// Get the float button rectangle (if visible).
    fn float_button_rect(&self) -> Option<Rect> {
        if !self.features.is_floatable() {
            return None;
        }

        let title_rect = self.title_bar_rect();
        let padding = (self.title_bar_height - self.button_size) / 2.0;

        // Position buttons from the right
        let close_offset = if self.features.is_closable() {
            self.button_size + padding
        } else {
            0.0
        };

        if self.features.has_vertical_title_bar() {
            Some(Rect::new(
                padding,
                padding + close_offset,
                self.button_size,
                self.button_size,
            ))
        } else {
            Some(Rect::new(
                title_rect.width() - padding - self.button_size - close_offset,
                padding,
                self.button_size,
                self.button_size,
            ))
        }
    }

    /// Get the close button rectangle (if visible).
    fn close_button_rect(&self) -> Option<Rect> {
        if !self.features.is_closable() {
            return None;
        }

        let title_rect = self.title_bar_rect();
        let padding = (self.title_bar_height - self.button_size) / 2.0;

        if self.features.has_vertical_title_bar() {
            Some(Rect::new(
                padding,
                padding,
                self.button_size,
                self.button_size,
            ))
        } else {
            Some(Rect::new(
                title_rect.width() - padding - self.button_size,
                padding,
                self.button_size,
                self.button_size,
            ))
        }
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    /// Check which button is at the given position.
    fn hit_test_button(&self, pos: Point) -> Option<TitleBarButton> {
        if let Some(close_rect) = self.close_button_rect()
            && close_rect.contains(pos)
        {
            return Some(TitleBarButton::Close);
        }
        if let Some(float_rect) = self.float_button_rect()
            && float_rect.contains(pos)
        {
            return Some(TitleBarButton::Float);
        }
        None
    }

    /// Check if the position is in the title bar drag area.
    fn is_in_title_bar_drag_area(&self, pos: Point) -> bool {
        let title_rect = self.title_bar_rect();
        if !title_rect.contains(pos) {
            return false;
        }
        // Not over any button
        self.hit_test_button(pos).is_none()
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check button clicks
        if let Some(button) = self.hit_test_button(pos) {
            match button {
                TitleBarButton::Close => {
                    self.close_button_state.pressed = true;
                    self.base.update();
                    return true;
                }
                TitleBarButton::Float => {
                    self.float_button_state.pressed = true;
                    self.base.update();
                    return true;
                }
            }
        }

        // Check title bar drag (only if movable or floatable)
        if (self.features.is_movable() || self.features.is_floatable())
            && self.is_in_title_bar_drag_area(pos)
        {
            self.dragging = true;
            self.drag_start = event.global_pos;
            self.drag_start_pos = self.base.pos();
            return true;
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check button releases
        if self.close_button_state.pressed {
            self.close_button_state.pressed = false;
            if let Some(rect) = self.close_button_rect()
                && rect.contains(pos)
            {
                self.close_requested.emit(());
            }
            self.base.update();
            return true;
        }

        if self.float_button_state.pressed {
            self.float_button_state.pressed = false;
            if let Some(rect) = self.float_button_rect()
                && rect.contains(pos)
            {
                self.toggle_floating();
            }
            self.base.update();
            return true;
        }

        // End drag
        if self.dragging {
            self.dragging = false;
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;

        // Update button hover states
        let new_float_hover = self.float_button_rect().is_some_and(|r| r.contains(pos));
        let new_close_hover = self.close_button_rect().is_some_and(|r| r.contains(pos));

        let hover_changed = self.float_button_state.hovered != new_float_hover
            || self.close_button_state.hovered != new_close_hover;

        self.float_button_state.hovered = new_float_hover;
        self.close_button_state.hovered = new_close_hover;

        if hover_changed {
            self.base.update();
        }

        // Handle dragging
        if self.dragging {
            let delta = Point::new(
                event.global_pos.x - self.drag_start.x,
                event.global_pos.y - self.drag_start.y,
            );

            // If not floating and we've dragged far enough, float the widget
            if !self.floating && self.features.is_floatable() {
                let drag_distance = (delta.x * delta.x + delta.y * delta.y).sqrt();
                if drag_distance > 10.0 {
                    self.set_floating(true);
                    self.float_position = self.drag_start_pos;
                }
            }

            if self.floating {
                let new_pos = Point::new(
                    self.drag_start_pos.x + delta.x,
                    self.drag_start_pos.y + delta.y,
                );
                self.float_position = new_pos;
                self.base.set_pos(new_pos);
            }

            return true;
        }

        hover_changed
    }

    fn handle_double_click(&mut self, event: &MouseDoubleClickEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Double-click on title bar toggles floating
        if self.features.is_floatable() && self.is_in_title_bar_drag_area(event.local_pos) {
            self.toggle_floating();
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Escape to close if focused and closable
        if event.key == Key::Escape && self.features.is_closable() {
            self.close_requested.emit(());
            return true;
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_title_bar(&self, ctx: &mut PaintContext<'_>) {
        let title_rect = self.title_bar_rect();

        // Background
        let bg_color = if self.base.has_focus() {
            self.title_bar_active_color
        } else {
            self.title_bar_color
        };
        ctx.renderer().fill_rect(title_rect, bg_color);

        // Note: Text rendering requires the full TextRenderer system.
        // For now, the title is stored but not rendered visually.
        // A full implementation would use TextRenderer here.

        // Draw buttons
        self.paint_buttons(ctx);
    }

    fn paint_buttons(&self, ctx: &mut PaintContext<'_>) {
        // Float button
        if let Some(rect) = self.float_button_rect() {
            let bg = if self.float_button_state.pressed {
                self.button_pressed_color
            } else if self.float_button_state.hovered {
                self.button_hover_color
            } else {
                self.button_color
            };
            ctx.renderer().fill_rect(rect, bg);

            // Draw float/dock icon (simple rectangle for floating, docked indicator)
            let icon_margin = 3.0;
            let icon_rect = Rect::new(
                rect.origin.x + icon_margin,
                rect.origin.y + icon_margin,
                rect.width() - icon_margin * 2.0,
                rect.height() - icon_margin * 2.0,
            );
            let icon_color = Color::from_rgb8(80, 80, 80);
            let stroke = Stroke::new(icon_color, 1.0);

            if self.floating {
                // Draw dock icon (rectangle)
                ctx.renderer().stroke_rect(icon_rect, &stroke);
            } else {
                // Draw float icon (offset rectangles)
                let small_rect = Rect::new(
                    icon_rect.origin.x + 2.0,
                    icon_rect.origin.y,
                    icon_rect.width() - 2.0,
                    icon_rect.height() - 2.0,
                );
                ctx.renderer().stroke_rect(small_rect, &stroke);
            }
        }

        // Close button
        if let Some(rect) = self.close_button_rect() {
            let bg = if self.close_button_state.pressed {
                self.button_pressed_color
            } else if self.close_button_state.hovered {
                Color::from_rgb8(232, 17, 35) // Red hover for close
            } else {
                self.button_color
            };
            ctx.renderer().fill_rect(rect, bg);

            // Draw X icon
            let icon_margin = 4.0;
            let x1 = rect.origin.x + icon_margin;
            let y1 = rect.origin.y + icon_margin;
            let x2 = rect.origin.x + rect.width() - icon_margin;
            let y2 = rect.origin.y + rect.height() - icon_margin;

            let icon_color = if self.close_button_state.hovered {
                Color::WHITE
            } else {
                Color::from_rgb8(80, 80, 80)
            };
            let stroke = Stroke::new(icon_color, 1.5);

            ctx.renderer()
                .draw_line(Point::new(x1, y1), Point::new(x2, y2), &stroke);
            ctx.renderer()
                .draw_line(Point::new(x2, y1), Point::new(x1, y2), &stroke);
        }
    }

    fn paint_content_area(&self, ctx: &mut PaintContext<'_>) {
        let content_rect = self.content_rect();
        ctx.renderer()
            .fill_rect(content_rect, self.content_background);
    }

    fn paint_border(&self, ctx: &mut PaintContext<'_>) {
        if self.border_width > 0.0 {
            let rect = self.base.rect();
            let border_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());
            let stroke = Stroke::new(self.border_color, self.border_width);
            ctx.renderer().stroke_rect(border_rect, &stroke);
        }
    }
}

impl Widget for DockWidget {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Minimum size is title bar + some content space
        let min_width = if self.features.has_vertical_title_bar() {
            self.title_bar_height + 50.0
        } else {
            100.0
        };
        let min_height = if self.features.has_vertical_title_bar() {
            50.0
        } else {
            self.title_bar_height + 50.0
        };

        let preferred = Size::new(200.0, 150.0);
        SizeHint::new(preferred).with_minimum(Size::new(min_width, min_height))
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Paint in order: content background, content, title bar, border
        self.paint_content_area(ctx);
        self.paint_title_bar(ctx);
        self.paint_border(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::DoubleClick(e) => self.handle_double_click(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::Leave(_) => {
                // Clear hover states
                let changed = self.float_button_state.hovered || self.close_button_state.hovered;
                self.float_button_state.hovered = false;
                self.close_button_state.hovered = false;
                if changed {
                    self.base.update();
                }
                false
            }
            _ => false,
        }
    }
}

impl Object for DockWidget {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Default for DockWidget {
    fn default() -> Self {
        Self::new("Dock Widget")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_areas_bitflags() {
        let areas = DockAreas::LEFT | DockAreas::RIGHT;
        assert!(areas.contains(DockArea::Left));
        assert!(areas.contains(DockArea::Right));
        assert!(!areas.contains(DockArea::Top));
        assert!(!areas.contains(DockArea::Bottom));
    }

    #[test]
    fn test_dock_area_or() {
        let areas = DockArea::Left | DockArea::Top;
        assert!(areas.contains(DockArea::Left));
        assert!(areas.contains(DockArea::Top));
        assert!(!areas.contains(DockArea::Right));
    }

    #[test]
    fn test_dock_widget_features() {
        let features = DockWidgetFeatures::CLOSABLE | DockWidgetFeatures::MOVABLE;
        assert!(features.is_closable());
        assert!(features.is_movable());
        assert!(!features.is_floatable());
    }

    #[test]
    fn test_dock_widget_features_all() {
        let features = DockWidgetFeatures::all();
        assert!(features.is_closable());
        assert!(features.is_movable());
        assert!(features.is_floatable());
        assert!(!features.has_vertical_title_bar());
    }

    #[test]
    fn test_dock_areas_iter() {
        let areas = DockAreas::LEFT | DockAreas::BOTTOM;
        let collected: Vec<_> = areas.iter().collect();
        assert_eq!(collected.len(), 2);
        assert!(collected.contains(&DockArea::Left));
        assert!(collected.contains(&DockArea::Bottom));
    }
}
