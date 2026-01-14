//! Popup widget implementation.
//!
//! This module provides [`Popup`], a temporary floating container that displays
//! content and can auto-close based on user interaction.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Popup, PopupFlags, PopupPlacement};
//!
//! // Create a basic popup
//! let mut popup = Popup::new()
//!     .with_size(200.0, 150.0)
//!     .with_flags(PopupFlags::DEFAULT);
//!
//! // Connect to signals
//! popup.closed.connect(|()| {
//!     println!("Popup closed");
//! });
//!
//! // Show the popup at a specific position
//! popup.popup_at(100.0, 100.0);
//!
//! // Or show relative to an anchor widget
//! popup.popup_relative_to(anchor_widget_id, PopupPlacement::Below);
//! ```

use std::ops::{BitAnd, BitOr, BitOrAssign};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Size, Stroke};

use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseMoveEvent, MousePressEvent,
    MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
    WidgetEvent,
};

// ============================================================================
// Popup Flags
// ============================================================================

/// Flags that control popup appearance and behavior.
///
/// These flags can be combined using bitwise OR operations.
///
/// # Example
///
/// ```ignore
/// let flags = PopupFlags::STAYS_ON_TOP | PopupFlags::AUTO_CLOSE_ON_CLICK_OUTSIDE;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PopupFlags(u16);

impl PopupFlags {
    /// No special flags.
    pub const NONE: PopupFlags = PopupFlags(0);

    /// Popup has no frame or border.
    pub const FRAMELESS: PopupFlags = PopupFlags(1 << 0);

    /// Popup stays on top of other widgets.
    pub const STAYS_ON_TOP: PopupFlags = PopupFlags(1 << 1);

    /// Popup has a border.
    pub const BORDER: PopupFlags = PopupFlags(1 << 2);

    /// Popup has a close button.
    pub const CLOSE_BUTTON: PopupFlags = PopupFlags(1 << 3);

    /// Popup receives focus when shown.
    pub const FOCUS_ON_SHOW: PopupFlags = PopupFlags(1 << 4);

    /// Popup closes when clicking outside its bounds.
    pub const AUTO_CLOSE_ON_CLICK_OUTSIDE: PopupFlags = PopupFlags(1 << 5);

    /// Popup closes when it loses focus.
    pub const AUTO_CLOSE_ON_FOCUS_LOSS: PopupFlags = PopupFlags(1 << 6);

    /// Popup closes when Escape key is pressed.
    pub const CLOSE_ON_ESCAPE: PopupFlags = PopupFlags(1 << 7);

    /// Popup is modal (blocks input to other widgets).
    pub const MODAL: PopupFlags = PopupFlags(1 << 8);

    /// Show a backdrop behind the popup when modal.
    pub const SHOW_BACKDROP: PopupFlags = PopupFlags(1 << 9);

    /// Default flags for a standard popup.
    pub const DEFAULT: PopupFlags = PopupFlags(
        Self::STAYS_ON_TOP.0
            | Self::BORDER.0
            | Self::FOCUS_ON_SHOW.0
            | Self::AUTO_CLOSE_ON_CLICK_OUTSIDE.0
            | Self::CLOSE_ON_ESCAPE.0,
    );

    /// Flags for a modal popup (blocks other widgets).
    pub const MODAL_DEFAULT: PopupFlags = PopupFlags(
        Self::STAYS_ON_TOP.0
            | Self::BORDER.0
            | Self::FOCUS_ON_SHOW.0
            | Self::CLOSE_ON_ESCAPE.0
            | Self::MODAL.0
            | Self::SHOW_BACKDROP.0,
    );

    /// Flags for a tooltip-style popup (auto-close, no chrome).
    pub const TOOLTIP: PopupFlags = PopupFlags(
        Self::STAYS_ON_TOP.0
            | Self::BORDER.0
            | Self::AUTO_CLOSE_ON_CLICK_OUTSIDE.0
            | Self::AUTO_CLOSE_ON_FOCUS_LOSS.0,
    );

    /// Check if a flag is set.
    pub fn has(&self, flag: PopupFlags) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Check if the popup is frameless.
    pub fn is_frameless(&self) -> bool {
        self.has(Self::FRAMELESS)
    }

    /// Check if the popup stays on top.
    pub fn stays_on_top(&self) -> bool {
        self.has(Self::STAYS_ON_TOP)
    }

    /// Check if the popup has a border.
    pub fn has_border(&self) -> bool {
        self.has(Self::BORDER) && !self.is_frameless()
    }

    /// Check if the popup has a close button.
    pub fn has_close_button(&self) -> bool {
        self.has(Self::CLOSE_BUTTON)
    }

    /// Check if the popup should receive focus when shown.
    pub fn focus_on_show(&self) -> bool {
        self.has(Self::FOCUS_ON_SHOW)
    }

    /// Check if the popup should close on outside click.
    pub fn auto_close_on_click_outside(&self) -> bool {
        self.has(Self::AUTO_CLOSE_ON_CLICK_OUTSIDE)
    }

    /// Check if the popup should close on focus loss.
    pub fn auto_close_on_focus_loss(&self) -> bool {
        self.has(Self::AUTO_CLOSE_ON_FOCUS_LOSS)
    }

    /// Check if the popup should close on Escape key.
    pub fn close_on_escape(&self) -> bool {
        self.has(Self::CLOSE_ON_ESCAPE)
    }

    /// Check if the popup is modal.
    pub fn is_modal(&self) -> bool {
        self.has(Self::MODAL)
    }

    /// Check if the popup shows a backdrop.
    pub fn shows_backdrop(&self) -> bool {
        self.has(Self::SHOW_BACKDROP) && self.is_modal()
    }
}

impl BitOr for PopupFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        PopupFlags(self.0 | rhs.0)
    }
}

impl BitOrAssign for PopupFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for PopupFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        PopupFlags(self.0 & rhs.0)
    }
}

// ============================================================================
// Popup Placement
// ============================================================================

/// Placement strategy for positioning a popup relative to an anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PopupPlacement {
    /// Position below the anchor widget.
    #[default]
    Below,
    /// Position above the anchor widget.
    Above,
    /// Position to the left of the anchor widget.
    Left,
    /// Position to the right of the anchor widget.
    Right,
    /// Center over the anchor widget.
    Center,
    /// Align the left edge with the anchor's left edge, below.
    BelowAlignLeft,
    /// Align the right edge with the anchor's right edge, below.
    BelowAlignRight,
    /// Align the left edge with the anchor's left edge, above.
    AboveAlignLeft,
    /// Align the right edge with the anchor's right edge, above.
    AboveAlignRight,
}

impl PopupPlacement {
    /// Calculate the popup position given anchor geometry and popup size.
    ///
    /// The `available_bounds` parameter is used for flip/shift logic when
    /// the popup would go off-screen.
    pub fn calculate_position(
        &self,
        anchor_rect: Rect,
        popup_size: Size,
        available_bounds: Option<Rect>,
    ) -> Point {
        let mut pos = self.calculate_initial_position(anchor_rect, popup_size);

        // Apply flip/shift if bounds are provided
        if let Some(bounds) = available_bounds {
            pos = Self::apply_flip_shift(pos, popup_size, bounds, *self, anchor_rect);
        }

        pos
    }

    fn calculate_initial_position(&self, anchor_rect: Rect, popup_size: Size) -> Point {
        let anchor_center_x = anchor_rect.origin.x + anchor_rect.size.width / 2.0;
        let anchor_center_y = anchor_rect.origin.y + anchor_rect.size.height / 2.0;

        match self {
            PopupPlacement::Below => Point::new(
                anchor_center_x - popup_size.width / 2.0,
                anchor_rect.origin.y + anchor_rect.size.height,
            ),
            PopupPlacement::Above => Point::new(
                anchor_center_x - popup_size.width / 2.0,
                anchor_rect.origin.y - popup_size.height,
            ),
            PopupPlacement::Left => Point::new(
                anchor_rect.origin.x - popup_size.width,
                anchor_center_y - popup_size.height / 2.0,
            ),
            PopupPlacement::Right => Point::new(
                anchor_rect.origin.x + anchor_rect.size.width,
                anchor_center_y - popup_size.height / 2.0,
            ),
            PopupPlacement::Center => Point::new(
                anchor_center_x - popup_size.width / 2.0,
                anchor_center_y - popup_size.height / 2.0,
            ),
            PopupPlacement::BelowAlignLeft => Point::new(
                anchor_rect.origin.x,
                anchor_rect.origin.y + anchor_rect.size.height,
            ),
            PopupPlacement::BelowAlignRight => Point::new(
                anchor_rect.origin.x + anchor_rect.size.width - popup_size.width,
                anchor_rect.origin.y + anchor_rect.size.height,
            ),
            PopupPlacement::AboveAlignLeft => {
                Point::new(anchor_rect.origin.x, anchor_rect.origin.y - popup_size.height)
            }
            PopupPlacement::AboveAlignRight => Point::new(
                anchor_rect.origin.x + anchor_rect.size.width - popup_size.width,
                anchor_rect.origin.y - popup_size.height,
            ),
        }
    }

    fn apply_flip_shift(
        pos: Point,
        popup_size: Size,
        bounds: Rect,
        placement: PopupPlacement,
        anchor_rect: Rect,
    ) -> Point {
        let mut result = pos;

        // Check if popup goes outside bounds and needs flipping
        let popup_rect = Rect::new(pos.x, pos.y, popup_size.width, popup_size.height);

        // Flip vertically if needed
        if matches!(
            placement,
            PopupPlacement::Below
                | PopupPlacement::BelowAlignLeft
                | PopupPlacement::BelowAlignRight
        ) {
            if popup_rect.bottom() > bounds.bottom() {
                // Flip to above
                result.y = anchor_rect.origin.y - popup_size.height;
            }
        } else if matches!(
            placement,
            PopupPlacement::Above
                | PopupPlacement::AboveAlignLeft
                | PopupPlacement::AboveAlignRight
        ) {
            if popup_rect.origin.y < bounds.origin.y {
                // Flip to below
                result.y = anchor_rect.origin.y + anchor_rect.size.height;
            }
        }

        // Flip horizontally if needed
        if matches!(placement, PopupPlacement::Left) {
            if popup_rect.origin.x < bounds.origin.x {
                result.x = anchor_rect.origin.x + anchor_rect.size.width;
            }
        } else if matches!(placement, PopupPlacement::Right) {
            if popup_rect.right() > bounds.right() {
                result.x = anchor_rect.origin.x - popup_size.width;
            }
        }

        // Shift to stay within bounds (after flipping)
        if result.x < bounds.origin.x {
            result.x = bounds.origin.x;
        } else if result.x + popup_size.width > bounds.right() {
            result.x = bounds.right() - popup_size.width;
        }

        if result.y < bounds.origin.y {
            result.y = bounds.origin.y;
        } else if result.y + popup_size.height > bounds.bottom() {
            result.y = bounds.bottom() - popup_size.height;
        }

        result
    }
}

// ============================================================================
// Button State
// ============================================================================

/// State of the close button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ButtonState {
    hovered: bool,
    pressed: bool,
}

// ============================================================================
// Popup
// ============================================================================

/// A temporary floating container widget.
///
/// Popup provides a floating container that displays content and can auto-close
/// based on various conditions (click outside, focus loss, Escape key). It supports
/// both absolute positioning and positioning relative to an anchor widget.
///
/// # Features
///
/// - Positioning relative to anchor widgets with automatic flip/shift
/// - Auto-close on click outside
/// - Auto-close on focus loss
/// - Escape key to close
/// - Modal mode with optional backdrop
/// - Optional close button
///
/// # Signals
///
/// - `about_to_show()`: Emitted before the popup becomes visible
/// - `about_to_hide()`: Emitted before the popup is hidden
/// - `closed()`: Emitted when the popup is closed
pub struct Popup {
    /// Widget base.
    base: WidgetBase,

    /// The content widget ID.
    content_widget: Option<ObjectId>,

    /// Popup flags controlling behavior.
    flags: PopupFlags,

    /// Anchor widget ID (for relative positioning).
    anchor_widget: Option<ObjectId>,

    /// Current placement strategy.
    placement: PopupPlacement,

    /// Available bounds for flip/shift calculations.
    available_bounds: Option<Rect>,

    /// Minimum popup size.
    min_size: Size,

    /// Maximum popup size (None means no maximum).
    max_size: Option<Size>,

    /// Border width.
    border_width: f32,

    /// Close button size.
    button_size: f32,

    /// Padding inside the popup.
    padding: f32,

    // Visual styling
    /// Background color.
    background_color: Color,
    /// Border color.
    border_color: Color,
    /// Close button color.
    button_color: Color,
    /// Close button hover color.
    button_hover_color: Color,
    /// Close button pressed color.
    button_pressed_color: Color,
    /// Backdrop color (for modal popups).
    backdrop_color: Color,

    // Interaction state
    /// Close button state.
    close_button_state: ButtonState,

    // Signals
    /// Signal emitted before the popup is shown.
    pub about_to_show: Signal<()>,
    /// Signal emitted before the popup is hidden.
    pub about_to_hide: Signal<()>,
    /// Signal emitted when the popup is closed.
    pub closed: Signal<()>,
}

impl Popup {
    /// Create a new popup.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred));
        // Start hidden
        base.hide();

        Self {
            base,
            content_widget: None,
            flags: PopupFlags::DEFAULT,
            anchor_widget: None,
            placement: PopupPlacement::Below,
            available_bounds: None,
            min_size: Size::new(50.0, 30.0),
            max_size: None,
            border_width: 1.0,
            button_size: 16.0,
            padding: 4.0,
            background_color: Color::WHITE,
            border_color: Color::from_rgb8(180, 180, 180),
            button_color: Color::from_rgb8(240, 240, 240),
            button_hover_color: Color::from_rgb8(220, 220, 220),
            button_pressed_color: Color::from_rgb8(200, 200, 200),
            backdrop_color: Color::from_rgba8(0, 0, 0, 100),
            close_button_state: ButtonState::default(),
            about_to_show: Signal::new(),
            about_to_hide: Signal::new(),
            closed: Signal::new(),
        }
    }

    // =========================================================================
    // Builder Pattern Methods
    // =========================================================================

    /// Set the popup size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.base.set_size(Size::new(width, height));
        self
    }

    /// Set the popup position using builder pattern.
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.base.set_pos(Point::new(x, y));
        self
    }

    /// Set the popup flags using builder pattern.
    pub fn with_flags(mut self, flags: PopupFlags) -> Self {
        self.flags = flags;
        self
    }

    /// Set the minimum size using builder pattern.
    pub fn with_min_size(mut self, width: f32, height: f32) -> Self {
        self.min_size = Size::new(width, height);
        self
    }

    /// Set the maximum size using builder pattern.
    pub fn with_max_size(mut self, width: f32, height: f32) -> Self {
        self.max_size = Some(Size::new(width, height));
        self
    }

    /// Set the placement strategy using builder pattern.
    pub fn with_placement(mut self, placement: PopupPlacement) -> Self {
        self.placement = placement;
        self
    }

    /// Set the available bounds for flip/shift using builder pattern.
    pub fn with_available_bounds(mut self, bounds: Rect) -> Self {
        self.available_bounds = Some(bounds);
        self
    }

    /// Set the background color using builder pattern.
    pub fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = color;
        self
    }

    /// Set the border color using builder pattern.
    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    // =========================================================================
    // Flags
    // =========================================================================

    /// Get the popup flags.
    pub fn flags(&self) -> PopupFlags {
        self.flags
    }

    /// Set the popup flags.
    pub fn set_flags(&mut self, flags: PopupFlags) {
        self.flags = flags;
        self.base.update();
    }

    /// Check if the popup is modal.
    pub fn is_modal(&self) -> bool {
        self.flags.is_modal()
    }

    /// Set whether the popup is modal.
    pub fn set_modal(&mut self, modal: bool) {
        if modal {
            self.flags |= PopupFlags::MODAL;
        } else {
            self.flags = PopupFlags(self.flags.0 & !PopupFlags::MODAL.0);
        }
    }

    // =========================================================================
    // Content Widget
    // =========================================================================

    /// Get the content widget ID.
    pub fn content_widget(&self) -> Option<ObjectId> {
        self.content_widget
    }

    /// Set the content widget.
    pub fn set_content_widget(&mut self, widget_id: ObjectId) {
        self.content_widget = Some(widget_id);
        self.base.update();
    }

    /// Set the content widget using builder pattern.
    pub fn with_content_widget(mut self, widget_id: ObjectId) -> Self {
        self.content_widget = Some(widget_id);
        self
    }

    // =========================================================================
    // Anchor and Positioning
    // =========================================================================

    /// Get the anchor widget ID.
    pub fn anchor_widget(&self) -> Option<ObjectId> {
        self.anchor_widget
    }

    /// Set the anchor widget.
    pub fn set_anchor_widget(&mut self, widget_id: Option<ObjectId>) {
        self.anchor_widget = widget_id;
    }

    /// Get the current placement strategy.
    pub fn placement(&self) -> PopupPlacement {
        self.placement
    }

    /// Set the placement strategy.
    pub fn set_placement(&mut self, placement: PopupPlacement) {
        self.placement = placement;
    }

    /// Get the available bounds for flip/shift.
    pub fn available_bounds(&self) -> Option<Rect> {
        self.available_bounds
    }

    /// Set the available bounds for flip/shift.
    pub fn set_available_bounds(&mut self, bounds: Option<Rect>) {
        self.available_bounds = bounds;
    }

    /// Position the popup relative to an anchor rectangle.
    ///
    /// This calculates the popup position based on the placement strategy
    /// and applies flip/shift if available bounds are set.
    pub fn position_relative_to_rect(&mut self, anchor_rect: Rect) {
        let popup_size = self.base.size();
        let pos =
            self.placement
                .calculate_position(anchor_rect, popup_size, self.available_bounds);
        self.base.set_pos(pos);
    }

    /// Move the popup to an absolute position.
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.base.move_to(x, y);
    }

    /// Resize the popup.
    pub fn resize(&mut self, width: f32, height: f32) {
        let clamped_width = width.max(self.min_size.width);
        let clamped_height = height.max(self.min_size.height);

        let (final_width, final_height) = if let Some(max) = self.max_size {
            (clamped_width.min(max.width), clamped_height.min(max.height))
        } else {
            (clamped_width, clamped_height)
        };

        self.base.resize(final_width, final_height);
    }

    // =========================================================================
    // Size Constraints
    // =========================================================================

    /// Get the minimum popup size.
    pub fn min_size(&self) -> Size {
        self.min_size
    }

    /// Set the minimum popup size.
    pub fn set_min_size(&mut self, size: Size) {
        self.min_size = size;
    }

    /// Get the maximum popup size.
    pub fn max_size(&self) -> Option<Size> {
        self.max_size
    }

    /// Set the maximum popup size.
    pub fn set_max_size(&mut self, size: Option<Size>) {
        self.max_size = size;
    }

    // =========================================================================
    // Show/Hide Operations
    // =========================================================================

    /// Show the popup at the current position.
    pub fn show(&mut self) {
        self.about_to_show.emit(());
        self.base.show();
        self.base.update();
    }

    /// Show the popup at the specified position.
    pub fn popup_at(&mut self, x: f32, y: f32) {
        self.base.set_pos(Point::new(x, y));
        self.show();
    }

    /// Show the popup relative to an anchor rectangle.
    pub fn popup_relative_to_rect(&mut self, anchor_rect: Rect, placement: PopupPlacement) {
        self.placement = placement;
        self.position_relative_to_rect(anchor_rect);
        self.show();
    }

    /// Hide the popup.
    pub fn hide(&mut self) {
        if self.base.is_visible() {
            self.about_to_hide.emit(());
            self.base.hide();
        }
    }

    /// Close the popup (hide and emit closed signal).
    pub fn close(&mut self) {
        if self.base.is_visible() {
            self.about_to_hide.emit(());
            self.base.hide();
            self.closed.emit(());
        }
    }

    /// Check if the popup is visible.
    pub fn is_visible(&self) -> bool {
        self.base.is_visible()
    }

    // =========================================================================
    // Geometry Calculations
    // =========================================================================

    /// Get the content area rectangle.
    pub fn content_rect(&self) -> Rect {
        let rect = self.base.rect();
        let border = if self.flags.has_border() {
            self.border_width
        } else {
            0.0
        };

        let top_padding = if self.flags.has_close_button() {
            self.button_size + self.padding
        } else {
            self.padding
        };

        Rect::new(
            border + self.padding,
            border + top_padding,
            rect.width() - border * 2.0 - self.padding * 2.0,
            rect.height() - border * 2.0 - self.padding - top_padding,
        )
    }

    /// Get the close button rectangle.
    fn close_button_rect(&self) -> Option<Rect> {
        if !self.flags.has_close_button() {
            return None;
        }

        let rect = self.base.rect();
        let border = if self.flags.has_border() {
            self.border_width
        } else {
            0.0
        };
        let padding = self.padding;

        Some(Rect::new(
            rect.width() - border - padding - self.button_size,
            border + padding,
            self.button_size,
            self.button_size,
        ))
    }

    // =========================================================================
    // Event Handlers
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;
        let rect = self.base.rect();
        let local_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());

        // Check if click is outside popup bounds (for auto-close)
        if self.flags.auto_close_on_click_outside() && !local_rect.contains(pos) {
            self.close();
            return true;
        }

        // Check close button
        if let Some(button_rect) = self.close_button_rect() {
            if button_rect.contains(pos) {
                self.close_button_state.pressed = true;
                self.base.update();
                return true;
            }
        }

        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        let pos = event.local_pos;

        // Check close button release
        if self.close_button_state.pressed {
            self.close_button_state.pressed = false;
            if let Some(button_rect) = self.close_button_rect() {
                if button_rect.contains(pos) {
                    self.close();
                    return true;
                }
            }
            self.base.update();
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;

        // Update close button hover state
        let new_hover = self.close_button_rect().is_some_and(|r| r.contains(pos));

        if new_hover != self.close_button_state.hovered {
            self.close_button_state.hovered = new_hover;
            self.base.update();
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Escape to close
        if event.key == Key::Escape && self.flags.close_on_escape() {
            self.close();
            return true;
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_backdrop(&self, ctx: &mut PaintContext<'_>) {
        if !self.flags.shows_backdrop() {
            return;
        }

        // Paint a semi-transparent backdrop
        // Note: In a full implementation, this would cover the entire parent/window area.
        // For now, we paint a slightly larger area around the popup.
        let rect = self.base.rect();
        let backdrop_rect = Rect::new(
            -rect.origin.x,
            -rect.origin.y,
            rect.origin.x * 2.0 + rect.width() + 1000.0,
            rect.origin.y * 2.0 + rect.height() + 1000.0,
        );
        ctx.renderer().fill_rect(backdrop_rect, self.backdrop_color);
    }

    fn paint_background(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let local_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());
        ctx.renderer().fill_rect(local_rect, self.background_color);
    }

    fn paint_border(&self, ctx: &mut PaintContext<'_>) {
        if !self.flags.has_border() {
            return;
        }

        let rect = self.base.rect();
        let border_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());
        let stroke = Stroke::new(self.border_color, self.border_width);
        ctx.renderer().stroke_rect(border_rect, &stroke);
    }

    fn paint_close_button(&self, ctx: &mut PaintContext<'_>) {
        let Some(rect) = self.close_button_rect() else {
            return;
        };

        // Button background
        let bg = if self.close_button_state.pressed {
            self.button_pressed_color
        } else if self.close_button_state.hovered {
            self.button_hover_color
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

        let icon_color = Color::from_rgb8(80, 80, 80);
        let stroke = Stroke::new(icon_color, 1.5);

        ctx.renderer()
            .draw_line(Point::new(x1, y1), Point::new(x2, y2), &stroke);
        ctx.renderer()
            .draw_line(Point::new(x2, y1), Point::new(x1, y2), &stroke);
    }
}

impl Widget for Popup {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let preferred = Size::new(200.0, 150.0);
        SizeHint::new(preferred).with_minimum(self.min_size)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        if !self.base.is_visible() {
            return;
        }

        // Paint in order: backdrop (if modal), background, border, close button
        self.paint_backdrop(ctx);
        self.paint_background(ctx);
        self.paint_border(ctx);
        self.paint_close_button(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => self.handle_mouse_press(e),
            WidgetEvent::MouseRelease(e) => self.handle_mouse_release(e),
            WidgetEvent::MouseMove(e) => self.handle_mouse_move(e),
            WidgetEvent::KeyPress(e) => self.handle_key_press(e),
            WidgetEvent::Leave(_) => {
                // Clear hover state
                if self.close_button_state.hovered {
                    self.close_button_state.hovered = false;
                    self.base.update();
                }
                false
            }
            WidgetEvent::FocusOut(_) => {
                if self.flags.auto_close_on_focus_loss() {
                    self.close();
                    return true;
                }
                false
            }
            _ => false,
        }
    }
}

impl Object for Popup {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Default for Popup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_popup_flags_default() {
        let flags = PopupFlags::DEFAULT;
        assert!(flags.stays_on_top());
        assert!(flags.has_border());
        assert!(flags.focus_on_show());
        assert!(flags.auto_close_on_click_outside());
        assert!(flags.close_on_escape());
        assert!(!flags.is_modal());
        assert!(!flags.is_frameless());
    }

    #[test]
    fn test_popup_flags_modal() {
        let flags = PopupFlags::MODAL_DEFAULT;
        assert!(flags.is_modal());
        assert!(flags.shows_backdrop());
        assert!(flags.close_on_escape());
        assert!(!flags.auto_close_on_click_outside());
    }

    #[test]
    fn test_popup_flags_tooltip() {
        let flags = PopupFlags::TOOLTIP;
        assert!(flags.auto_close_on_click_outside());
        assert!(flags.auto_close_on_focus_loss());
        assert!(!flags.has_close_button());
        assert!(!flags.is_modal());
    }

    #[test]
    fn test_popup_flags_bitwise() {
        let flags = PopupFlags::BORDER | PopupFlags::CLOSE_BUTTON;
        assert!(flags.has_border());
        assert!(flags.has_close_button());
        assert!(!flags.is_modal());
    }

    #[test]
    fn test_popup_placement_below() {
        let anchor = Rect::new(100.0, 100.0, 50.0, 30.0);
        let popup_size = Size::new(100.0, 80.0);
        let pos = PopupPlacement::Below.calculate_position(anchor, popup_size, None);

        // Should be centered horizontally below the anchor
        assert_eq!(pos.x, 100.0 + 25.0 - 50.0); // anchor_center_x - popup_width/2
        assert_eq!(pos.y, 130.0); // anchor_y + anchor_height
    }

    #[test]
    fn test_popup_placement_above() {
        let anchor = Rect::new(100.0, 100.0, 50.0, 30.0);
        let popup_size = Size::new(100.0, 80.0);
        let pos = PopupPlacement::Above.calculate_position(anchor, popup_size, None);

        // Should be centered horizontally above the anchor
        assert_eq!(pos.x, 100.0 + 25.0 - 50.0); // anchor_center_x - popup_width/2
        assert_eq!(pos.y, 20.0); // anchor_y - popup_height
    }

    #[test]
    fn test_popup_placement_below_align_left() {
        let anchor = Rect::new(100.0, 100.0, 50.0, 30.0);
        let popup_size = Size::new(100.0, 80.0);
        let pos = PopupPlacement::BelowAlignLeft.calculate_position(anchor, popup_size, None);

        assert_eq!(pos.x, 100.0); // aligned with anchor's left edge
        assert_eq!(pos.y, 130.0); // below anchor
    }

    #[test]
    fn test_popup_placement_with_flip() {
        let anchor = Rect::new(100.0, 900.0, 50.0, 30.0);
        let popup_size = Size::new(100.0, 80.0);
        let bounds = Rect::new(0.0, 0.0, 1000.0, 950.0);

        // Popup would go below y=950, so it should flip to above
        let pos = PopupPlacement::Below.calculate_position(anchor, popup_size, Some(bounds));

        // Should flip to above the anchor
        assert_eq!(pos.y, 820.0); // anchor_y - popup_height
    }

    #[test]
    fn test_popup_placement_center() {
        let anchor = Rect::new(100.0, 100.0, 50.0, 30.0);
        let popup_size = Size::new(100.0, 80.0);
        let pos = PopupPlacement::Center.calculate_position(anchor, popup_size, None);

        // Should be centered over the anchor
        let anchor_center_x = 100.0 + 25.0;
        let anchor_center_y = 100.0 + 15.0;
        assert_eq!(pos.x, anchor_center_x - 50.0); // center_x - popup_width/2
        assert_eq!(pos.y, anchor_center_y - 40.0); // center_y - popup_height/2
    }

    #[test]
    fn test_popup_placement_left_right() {
        let anchor = Rect::new(100.0, 100.0, 50.0, 30.0);
        let popup_size = Size::new(80.0, 60.0);

        let pos_left = PopupPlacement::Left.calculate_position(anchor, popup_size, None);
        assert_eq!(pos_left.x, 20.0); // anchor_x - popup_width

        let pos_right = PopupPlacement::Right.calculate_position(anchor, popup_size, None);
        assert_eq!(pos_right.x, 150.0); // anchor_x + anchor_width
    }
}
