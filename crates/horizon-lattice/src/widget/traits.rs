//! Core widget trait definitions.
//!
//! This module defines the [`Widget`] trait which is the foundation for all
//! UI elements in Horizon Lattice.
//!
//! # Key Types
//!
//! - [`Widget`] - Base trait for all UI elements
//! - [`PaintContext`] - Rendering context passed to [`Widget::paint`]
//! - [`AsWidget`] - Helper trait for widget references
//!
//! # Related Types
//!
//! - [`super::WidgetBase`] - Common implementation for widgets
//! - [`super::SizeHint`] - Layout size hints
//! - [`super::SizePolicy`] - Layout sizing behavior
//! - [`super::WidgetEvent`] - Events handled by widgets
//! - [`super::Layout`] - Layout management for child widgets

use horizon_lattice_core::{Object, ObjectId};
use horizon_lattice_render::{GpuRenderer, Point, Rect, Renderer, Size};

use super::base::WidgetBase;
use super::events::WidgetEvent;
use super::geometry::{SizeHint, SizePolicyPair};

/// Context provided during widget painting.
///
/// This wraps a renderer and provides the widget's geometry information
/// for convenient access during the paint operation. Passed to [`Widget::paint`].
///
/// # Related
///
/// - [`Widget::paint`] - Receives this context
/// - [`GpuRenderer`](horizon_lattice_render::GpuRenderer) - The underlying renderer
pub struct PaintContext<'a> {
    /// The renderer to draw with.
    renderer: &'a mut GpuRenderer,
    /// The widget's local rectangle (origin always 0,0).
    widget_rect: Rect,
    /// Whether the Alt key is currently held (for mnemonic underline display).
    alt_held: bool,
    /// Whether to show focus indicator (widget has focus and window is active).
    show_focus: bool,
}

impl<'a> PaintContext<'a> {
    /// Create a new paint context.
    pub fn new(renderer: &'a mut GpuRenderer, widget_rect: Rect) -> Self {
        Self {
            renderer,
            widget_rect,
            alt_held: false,
            show_focus: false,
        }
    }

    /// Set the Alt held state (builder pattern).
    #[inline]
    pub fn with_alt_held(mut self, alt_held: bool) -> Self {
        self.alt_held = alt_held;
        self
    }

    /// Set whether to show focus indicator (builder pattern).
    #[inline]
    pub fn with_show_focus(mut self, show_focus: bool) -> Self {
        self.show_focus = show_focus;
        self
    }

    /// Check if the Alt key is currently held.
    ///
    /// This is used by widgets like Label to determine whether to display
    /// mnemonic underlines.
    #[inline]
    pub fn is_alt_held(&self) -> bool {
        self.alt_held
    }

    /// Check if the focus indicator should be shown.
    ///
    /// Returns `true` when the widget has focus and should display a visual
    /// indicator. Widgets can check this in their `paint()` method to draw
    /// focus rectangles or other focus visualization.
    #[inline]
    pub fn should_show_focus(&self) -> bool {
        self.show_focus
    }

    /// Get the renderer.
    #[inline]
    pub fn renderer(&mut self) -> &mut GpuRenderer {
        self.renderer
    }

    /// Get the widget's local rectangle.
    #[inline]
    pub fn rect(&self) -> Rect {
        self.widget_rect
    }

    /// Get the widget's width.
    #[inline]
    pub fn width(&self) -> f32 {
        self.widget_rect.width()
    }

    /// Get the widget's height.
    #[inline]
    pub fn height(&self) -> f32 {
        self.widget_rect.height()
    }

    /// Get the widget's size.
    #[inline]
    pub fn size(&self) -> Size {
        self.widget_rect.size
    }

    /// Draw a focus indicator around the widget.
    ///
    /// This draws a standard focus rectangle around the widget's bounds,
    /// using the platform's focus indicator style. Widgets should call this
    /// method in their `paint()` implementation when `should_show_focus()`
    /// returns `true`.
    ///
    /// # Arguments
    ///
    /// * `inset` - How much to inset the focus rectangle from the widget bounds.
    ///   Use 0.0 for a focus ring around the entire widget, or a positive value
    ///   to draw the indicator inside the widget's border.
    ///
    /// # Example
    ///
    /// ```ignore
    /// fn paint(&self, ctx: &mut PaintContext<'_>) {
    ///     // Paint widget content...
    ///
    ///     // Draw focus indicator if this widget has focus
    ///     if ctx.should_show_focus() {
    ///         ctx.draw_focus_indicator(1.0);
    ///     }
    /// }
    /// ```
    pub fn draw_focus_indicator(&mut self, inset: f32) {
        use horizon_lattice_render::{Color, Stroke};

        // Standard focus indicator color - platform-appropriate blue
        let focus_color = Color::from_rgb8(0, 120, 215);

        let rect = if inset > 0.0 {
            Rect::new(
                inset,
                inset,
                self.widget_rect.width() - inset * 2.0,
                self.widget_rect.height() - inset * 2.0,
            )
        } else {
            self.widget_rect
        };

        // Draw a 2-pixel focus outline
        let stroke = Stroke::new(focus_color, 2.0);
        self.renderer.stroke_rect(rect, &stroke);
    }

    /// Draw a focus indicator with custom color and width.
    ///
    /// Like `draw_focus_indicator` but allows customization of the appearance.
    pub fn draw_focus_indicator_styled(
        &mut self,
        inset: f32,
        color: horizon_lattice_render::Color,
        width: f32,
    ) {
        use horizon_lattice_render::Stroke;

        let rect = if inset > 0.0 {
            Rect::new(
                inset,
                inset,
                self.widget_rect.width() - inset * 2.0,
                self.widget_rect.height() - inset * 2.0,
            )
        } else {
            self.widget_rect
        };

        let stroke = Stroke::new(color, width);
        self.renderer.stroke_rect(rect, &stroke);
    }
}

/// The core trait for all widgets.
///
/// `Widget` extends [`Object`] to provide the fundamental interface for all
/// UI elements in Horizon Lattice. This is similar to Qt's `QWidget`.
///
/// # Required Methods
///
/// Implementors must provide:
/// - [`widget_base()`](Self::widget_base) / [`widget_base_mut()`](Self::widget_base_mut): Access to the underlying [`WidgetBase`]
/// - [`size_hint()`](Self::size_hint): The widget's preferred size for layout (see [`SizeHint`])
/// - [`paint()`](Self::paint): How to render the widget (see [`PaintContext`])
///
/// # Default Implementations
///
/// Many methods have default implementations that delegate to [`WidgetBase`]:
/// - Geometry accessors and mutators
/// - Visibility and enabled state
/// - Event handling (returns `false` by default)
///
/// # Related Types
///
/// - [`WidgetBase`] - Common widget implementation
/// - [`SizeHint`] - Layout size preferences
/// - [`SizePolicyPair`] - How the widget grows/shrinks
/// - [`PaintContext`] - Rendering context
/// - [`WidgetEvent`] - Input and lifecycle events
/// - [`super::Layout`] - Managing child widget positions
///
/// # Implementing Object
///
/// Widgets must also implement the [`Object`] trait. The simplest way is to
/// delegate to the [`WidgetBase`]:
///
/// ```ignore
/// impl Object for MyWidget {
///     fn object_id(&self) -> ObjectId {
///         self.base.object_id()
///     }
/// }
/// ```
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::widget::*;
/// use horizon_lattice::render::Color;
/// use horizon_lattice_core::{Object, ObjectId};
///
/// struct ColorBox {
///     base: WidgetBase,
///     color: Color,
/// }
///
/// impl ColorBox {
///     pub fn new(color: Color) -> Self {
///         Self {
///             base: WidgetBase::new::<Self>(),
///             color,
///         }
///     }
/// }
///
/// impl Object for ColorBox {
///     fn object_id(&self) -> ObjectId {
///         self.base.object_id()
///     }
/// }
///
/// impl Widget for ColorBox {
///     fn widget_base(&self) -> &WidgetBase { &self.base }
///     fn widget_base_mut(&mut self) -> &mut WidgetBase { &mut self.base }
///
///     fn size_hint(&self) -> SizeHint {
///         SizeHint::from_dimensions(100.0, 100.0)
///     }
///
///     fn paint(&self, ctx: &mut PaintContext<'_>) {
///         ctx.renderer().fill_rect(ctx.rect(), self.color);
///     }
/// }
/// ```
pub trait Widget: Object + Send + Sync {
    // =========================================================================
    // Required Methods
    // =========================================================================

    /// Get a reference to the widget's base.
    fn widget_base(&self) -> &WidgetBase;

    /// Get a mutable reference to the widget's base.
    fn widget_base_mut(&mut self) -> &mut WidgetBase;

    /// Get the widget's size hint for layout purposes.
    ///
    /// This tells layout managers what size the widget prefers. The actual
    /// size assigned may differ based on the layout and size policy.
    fn size_hint(&self) -> SizeHint;

    /// Paint the widget.
    ///
    /// This is called when the widget needs to be rendered. The paint context
    /// provides access to the renderer and the widget's geometry.
    ///
    /// # Coordinate System
    ///
    /// The renderer is already translated so that (0, 0) is the top-left
    /// corner of the widget. Use `ctx.rect()` to get the full bounds.
    fn paint(&self, ctx: &mut PaintContext<'_>);

    // =========================================================================
    // Geometry (default implementations delegate to WidgetBase)
    // =========================================================================

    /// Get the widget's geometry (position and size).
    fn geometry(&self) -> Rect {
        self.widget_base().geometry()
    }

    /// Set the widget's geometry.
    fn set_geometry(&mut self, rect: Rect) {
        self.widget_base_mut().set_geometry(rect);
    }

    /// Get the widget's position relative to its parent.
    fn pos(&self) -> Point {
        self.widget_base().pos()
    }

    /// Set the widget's position relative to its parent.
    fn set_pos(&mut self, pos: Point) {
        self.widget_base_mut().set_pos(pos);
    }

    /// Get the widget's size.
    fn size(&self) -> Size {
        self.widget_base().size()
    }

    /// Set the widget's size.
    fn set_size(&mut self, size: Size) {
        self.widget_base_mut().set_size(size);
    }

    /// Get the widget's local rectangle (origin at 0,0).
    fn rect(&self) -> Rect {
        self.widget_base().rect()
    }

    /// Get the widget's width.
    fn width(&self) -> f32 {
        self.widget_base().width()
    }

    /// Get the widget's height.
    fn height(&self) -> f32 {
        self.widget_base().height()
    }

    // =========================================================================
    // Size Policy
    // =========================================================================

    /// Get the widget's size policy.
    fn size_policy(&self) -> SizePolicyPair {
        self.widget_base().size_policy()
    }

    /// Set the widget's size policy.
    fn set_size_policy(&mut self, policy: SizePolicyPair) {
        self.widget_base_mut().set_size_policy(policy);
    }

    /// Calculate height for a given width (for widgets with height-for-width).
    ///
    /// Override this for widgets that need to adjust their height based on
    /// their width, such as text that wraps.
    fn height_for_width(&self, _width: f32) -> Option<f32> {
        None
    }

    /// Calculate width for a given height (for widgets with width-for-height).
    ///
    /// Override this for widgets that need to adjust their width based on
    /// their height.
    fn width_for_height(&self, _height: f32) -> Option<f32> {
        None
    }

    // =========================================================================
    // Visibility
    // =========================================================================

    /// Check if the widget is visible.
    fn is_visible(&self) -> bool {
        self.widget_base().is_visible()
    }

    /// Set whether the widget is visible.
    fn set_visible(&mut self, visible: bool) {
        self.widget_base_mut().set_visible(visible);
    }

    /// Show the widget.
    fn show(&mut self) {
        self.widget_base_mut().show();
    }

    /// Hide the widget.
    fn hide(&mut self) {
        self.widget_base_mut().hide();
    }

    /// Check if the widget is effectively visible (considering ancestors).
    ///
    /// Returns `true` only if this widget AND all its ancestors are visible.
    /// A widget with `is_visible() == true` may still be effectively hidden
    /// if any ancestor is hidden.
    fn is_effectively_visible(&self) -> bool {
        self.widget_base().is_effectively_visible()
    }

    // =========================================================================
    // Enabled State
    // =========================================================================

    /// Check if the widget is enabled.
    fn is_enabled(&self) -> bool {
        self.widget_base().is_enabled()
    }

    /// Set whether the widget is enabled.
    fn set_enabled(&mut self, enabled: bool) {
        self.widget_base_mut().set_enabled(enabled);
    }

    /// Check if the widget is effectively enabled (considering ancestors).
    ///
    /// Returns `true` only if this widget AND all its ancestors are enabled.
    /// A widget with `is_enabled() == true` may still be effectively disabled
    /// if any ancestor is disabled.
    fn is_effectively_enabled(&self) -> bool {
        self.widget_base().is_effectively_enabled()
    }

    // =========================================================================
    // Focus
    // =========================================================================

    /// Get the widget's focus policy.
    fn focus_policy(&self) -> super::base::FocusPolicy {
        self.widget_base().focus_policy()
    }

    /// Set the widget's focus policy.
    ///
    /// The focus policy determines how a widget can receive keyboard focus.
    /// See [`FocusPolicy`](super::base::FocusPolicy) for available options.
    fn set_focus_policy(&mut self, policy: super::base::FocusPolicy) {
        self.widget_base_mut().set_focus_policy(policy);
    }

    /// Check if the widget can receive keyboard focus.
    fn is_focusable(&self) -> bool {
        self.widget_base().is_focusable()
    }

    /// Check if the widget accepts focus via Tab/Shift+Tab navigation.
    fn accepts_tab_focus(&self) -> bool {
        self.widget_base().accepts_tab_focus()
    }

    /// Check if the widget accepts focus via mouse click.
    fn accepts_click_focus(&self) -> bool {
        self.widget_base().accepts_click_focus()
    }

    /// Set whether the widget can receive keyboard focus.
    ///
    /// This is a convenience method that sets the focus policy to `StrongFocus`
    /// if `focusable` is `true`, or `NoFocus` if `false`.
    fn set_focusable(&mut self, focusable: bool) {
        self.widget_base_mut().set_focusable(focusable);
    }

    /// Check if the widget currently has keyboard focus.
    fn has_focus(&self) -> bool {
        self.widget_base().has_focus()
    }

    // =========================================================================
    // Pressed State
    // =========================================================================

    /// Check if the widget is currently pressed.
    ///
    /// A widget is considered pressed when a mouse button is held down on it.
    /// This is typically used for visual feedback (e.g., button appears pushed).
    fn is_pressed(&self) -> bool {
        self.widget_base().is_pressed()
    }

    /// Check if the mouse is currently hovering over this widget.
    fn is_hovered(&self) -> bool {
        self.widget_base().is_hovered()
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    /// Handle a widget event.
    ///
    /// This is the main event dispatch method. The default implementation
    /// returns `false` to indicate the event was not handled. Override this
    /// to handle events specific to your widget.
    ///
    /// Return `true` if the event was handled and should not propagate further.
    fn event(&mut self, _event: &mut WidgetEvent) -> bool {
        false
    }

    /// Filter an event destined for another widget.
    ///
    /// This method is called when this widget is installed as an event filter
    /// on another widget. It allows this widget to intercept and optionally
    /// consume events before they reach their target.
    ///
    /// # Arguments
    ///
    /// * `event` - The event to filter.
    /// * `target` - The ObjectId of the widget the event was originally sent to.
    ///
    /// # Returns
    ///
    /// * `true` if the event was handled and should not reach the target widget.
    /// * `false` if the event should continue to the target widget.
    ///
    /// # Example
    ///
    /// ```ignore
    /// impl Widget for MyFilter {
    ///     fn event_filter(&mut self, event: &mut WidgetEvent, target: ObjectId) -> bool {
    ///         // Log all events
    ///         println!("Event {:?} for widget {:?}", event, target);
    ///
    ///         // Block Escape key
    ///         if let WidgetEvent::KeyPress(e) = event {
    ///             if e.key == Key::Escape {
    ///                 return true; // Consume the event
    ///             }
    ///         }
    ///
    ///         false // Let other events through
    ///     }
    ///     // ... other methods
    /// }
    /// ```
    fn event_filter(&mut self, _event: &mut WidgetEvent, _target: ObjectId) -> bool {
        false
    }

    // =========================================================================
    // Coordinate Mapping
    // =========================================================================

    /// Map a point from widget-local coordinates to parent coordinates.
    fn map_to_parent(&self, point: Point) -> Point {
        self.widget_base().map_to_parent(point)
    }

    /// Map a point from parent coordinates to widget-local coordinates.
    fn map_from_parent(&self, point: Point) -> Point {
        self.widget_base().map_from_parent(point)
    }

    /// Check if a point (in local coordinates) is inside the widget.
    fn contains_point(&self, point: Point) -> bool {
        self.widget_base().contains_point(point)
    }

    // =========================================================================
    // Opaque Widget
    // =========================================================================

    /// Check if this widget is opaque.
    ///
    /// Opaque widgets paint all their pixels, allowing the painting system
    /// to skip painting parent regions that would be completely covered.
    fn is_opaque(&self) -> bool {
        self.widget_base().is_opaque()
    }

    /// Set whether this widget is opaque.
    ///
    /// Set to `true` if this widget always paints all its pixels with opaque
    /// colors.
    fn set_opaque(&mut self, opaque: bool) {
        self.widget_base_mut().set_opaque(opaque);
    }

    // =========================================================================
    // Update / Repaint
    // =========================================================================

    /// Request a full repaint of the widget.
    ///
    /// This schedules a repaint for the next frame. Multiple calls before
    /// the next paint are coalesced.
    fn update(&mut self) {
        self.widget_base_mut().update();
    }

    /// Request a partial repaint of a specific region.
    ///
    /// This schedules a repaint of only the specified region for the next frame.
    /// The region is in widget-local coordinates.
    fn update_rect(&mut self, rect: Rect) {
        self.widget_base_mut().update_rect(rect);
    }

    /// Check if the widget needs to be repainted.
    fn needs_repaint(&self) -> bool {
        self.widget_base().needs_repaint()
    }

    /// Get the dirty region that needs repainting.
    ///
    /// Returns `None` if no repaint is needed, or `Some(rect)` with the
    /// region in widget-local coordinates that needs repainting.
    fn dirty_region(&self) -> Option<Rect> {
        self.widget_base().dirty_region()
    }

    /// Request an immediate repaint of the widget.
    ///
    /// Unlike `update()` which schedules a repaint for the next frame,
    /// this signals that the widget should be repainted immediately.
    fn repaint(&mut self) -> Rect {
        self.widget_base_mut().repaint()
    }

    /// Request an immediate repaint of a specific region.
    fn repaint_rect(&mut self, rect: Rect) -> Option<Rect> {
        self.widget_base_mut().repaint_rect(rect)
    }

    // =========================================================================
    // Mnemonic Support
    // =========================================================================

    /// Check if this widget has a mnemonic that matches the given key.
    ///
    /// The key should be a lowercase character. Override this in widgets
    /// that support mnemonics (like Label).
    ///
    /// # Returns
    ///
    /// `true` if this widget has a mnemonic matching the given key.
    fn matches_mnemonic_key(&self, _key: char) -> bool {
        false
    }

    /// Activate this widget's mnemonic.
    ///
    /// This is called when the user presses Alt+key where key matches
    /// this widget's mnemonic. Override this in widgets that support
    /// mnemonics to emit signals and return the buddy widget ID.
    ///
    /// # Returns
    ///
    /// The ObjectId of the buddy widget to receive focus, or `None` if
    /// this widget doesn't have a buddy or doesn't support mnemonics.
    fn activate_mnemonic(&self) -> Option<ObjectId> {
        None
    }
}

/// Extension trait for converting to `&dyn Widget`.
pub trait AsWidget {
    /// Get a reference to self as a widget.
    fn as_widget(&self) -> &dyn Widget;
    /// Get a mutable reference to self as a widget.
    fn as_widget_mut(&mut self) -> &mut dyn Widget;
}

impl<W: Widget> AsWidget for W {
    fn as_widget(&self) -> &dyn Widget {
        self
    }

    fn as_widget_mut(&mut self) -> &mut dyn Widget {
        self
    }
}
