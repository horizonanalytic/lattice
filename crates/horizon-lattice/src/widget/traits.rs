//! Core widget trait definitions.
//!
//! This module defines the `Widget` trait which is the foundation for all
//! UI elements in Horizon Lattice.

use horizon_lattice_core::Object;
use horizon_lattice_render::{GpuRenderer, Point, Rect, Size};

use super::base::WidgetBase;
use super::events::WidgetEvent;
use super::geometry::{SizeHint, SizePolicyPair};

/// Context provided during widget painting.
///
/// This wraps a renderer and provides the widget's geometry information
/// for convenient access during the paint operation.
pub struct PaintContext<'a> {
    /// The renderer to draw with.
    renderer: &'a mut GpuRenderer,
    /// The widget's local rectangle (origin always 0,0).
    widget_rect: Rect,
}

impl<'a> PaintContext<'a> {
    /// Create a new paint context.
    pub fn new(renderer: &'a mut GpuRenderer, widget_rect: Rect) -> Self {
        Self {
            renderer,
            widget_rect,
        }
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
}

/// The core trait for all widgets.
///
/// `Widget` extends `Object` to provide the fundamental interface for all
/// UI elements in Horizon Lattice. This is similar to Qt's `QWidget`.
///
/// # Required Methods
///
/// Implementors must provide:
/// - `widget_base()` / `widget_base_mut()`: Access to the underlying WidgetBase
/// - `size_hint()`: The widget's preferred size for layout
/// - `paint()`: How to render the widget
///
/// # Default Implementations
///
/// Many methods have default implementations that delegate to `WidgetBase`:
/// - Geometry accessors and mutators
/// - Visibility and enabled state
/// - Event handling (returns `false` by default)
///
/// # Implementing Object
///
/// Widgets must also implement the `Object` trait. The simplest way is to
/// delegate to the `WidgetBase`:
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

    // =========================================================================
    // Focus
    // =========================================================================

    /// Check if the widget can receive keyboard focus.
    fn is_focusable(&self) -> bool {
        self.widget_base().is_focusable()
    }

    /// Set whether the widget can receive keyboard focus.
    fn set_focusable(&mut self, focusable: bool) {
        self.widget_base_mut().set_focusable(focusable);
    }

    /// Check if the widget currently has keyboard focus.
    fn has_focus(&self) -> bool {
        self.widget_base().has_focus()
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
    // Update
    // =========================================================================

    /// Request a repaint of the widget.
    fn update(&mut self) {
        self.widget_base_mut().update();
    }

    /// Check if the widget needs to be repainted.
    fn needs_repaint(&self) -> bool {
        self.widget_base().needs_repaint()
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
