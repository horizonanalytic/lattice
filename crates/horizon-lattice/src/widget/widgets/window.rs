//! Window widget implementation.
//!
//! This module provides [`Window`], a top-level window widget that represents
//! a self-contained window with title bar, decorations, and window management
//! capabilities.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::{Window, WindowFlags, WindowModality, WindowState};
//!
//! // Create a basic window
//! let mut window = Window::new("My Window")
//!     .with_size(800.0, 600.0)
//!     .with_flags(WindowFlags::DEFAULT);
//!
//! // Connect to signals
//! window.close_requested.connect(|()| {
//!     println!("Window close requested");
//! });
//!
//! window.state_changed.connect(|state| {
//!     println!("Window state changed to: {:?}", state);
//! });
//!
//! // Show the window
//! window.show();
//! ```

use std::ops::{BitAnd, BitOr, BitOrAssign};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Size, Stroke};

use crate::widget::layout::ContentMargins;
use crate::widget::{
    FocusPolicy, Key, KeyPressEvent, MouseButton, MouseDoubleClickEvent, MouseMoveEvent,
    MousePressEvent, MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair,
    Widget, WidgetBase, WidgetEvent,
};

// ============================================================================
// Window State
// ============================================================================

/// The state of a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowState {
    /// Normal window state (default size and position).
    #[default]
    Normal,
    /// Window is minimized (iconified).
    Minimized,
    /// Window is maximized (fills available space).
    Maximized,
    /// Window is fullscreen (covers entire screen, no decorations).
    Fullscreen,
}

impl WindowState {
    /// Check if the window is in a normal state.
    pub fn is_normal(&self) -> bool {
        matches!(self, WindowState::Normal)
    }

    /// Check if the window is minimized.
    pub fn is_minimized(&self) -> bool {
        matches!(self, WindowState::Minimized)
    }

    /// Check if the window is maximized.
    pub fn is_maximized(&self) -> bool {
        matches!(self, WindowState::Maximized)
    }

    /// Check if the window is fullscreen.
    pub fn is_fullscreen(&self) -> bool {
        matches!(self, WindowState::Fullscreen)
    }
}

// ============================================================================
// Window Flags
// ============================================================================

/// Flags that control window appearance and behavior.
///
/// These flags can be combined using bitwise OR operations.
///
/// # Example
///
/// ```ignore
/// let flags = WindowFlags::FRAMELESS | WindowFlags::STAYS_ON_TOP;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WindowFlags(u16);

impl WindowFlags {
    /// No special flags (standard window).
    pub const NONE: WindowFlags = WindowFlags(0);

    /// Window has no frame or title bar.
    pub const FRAMELESS: WindowFlags = WindowFlags(1 << 0);

    /// Window stays on top of other windows.
    pub const STAYS_ON_TOP: WindowFlags = WindowFlags(1 << 1);

    /// Window has a minimize button.
    pub const MINIMIZE_BUTTON: WindowFlags = WindowFlags(1 << 2);

    /// Window has a maximize button.
    pub const MAXIMIZE_BUTTON: WindowFlags = WindowFlags(1 << 3);

    /// Window has a close button.
    pub const CLOSE_BUTTON: WindowFlags = WindowFlags(1 << 4);

    /// Window is resizable.
    pub const RESIZABLE: WindowFlags = WindowFlags(1 << 5);

    /// Window is movable.
    pub const MOVABLE: WindowFlags = WindowFlags(1 << 6);

    /// Window has a title bar.
    pub const TITLE_BAR: WindowFlags = WindowFlags(1 << 7);

    /// Window has a border.
    pub const BORDER: WindowFlags = WindowFlags(1 << 8);

    /// Default flags for a standard window (title bar, all buttons, resizable, movable).
    pub const DEFAULT: WindowFlags = WindowFlags(
        Self::MINIMIZE_BUTTON.0
            | Self::MAXIMIZE_BUTTON.0
            | Self::CLOSE_BUTTON.0
            | Self::RESIZABLE.0
            | Self::MOVABLE.0
            | Self::TITLE_BAR.0
            | Self::BORDER.0,
    );

    /// Flags for a dialog-style window (title bar, close button, movable, not resizable).
    pub const DIALOG: WindowFlags = WindowFlags(
        Self::CLOSE_BUTTON.0 | Self::MOVABLE.0 | Self::TITLE_BAR.0 | Self::BORDER.0,
    );

    /// Flags for a tool window (small title bar, close button, movable).
    pub const TOOL: WindowFlags =
        WindowFlags(Self::CLOSE_BUTTON.0 | Self::MOVABLE.0 | Self::TITLE_BAR.0 | Self::BORDER.0);

    /// Check if a flag is set.
    pub fn has(&self, flag: WindowFlags) -> bool {
        (self.0 & flag.0) == flag.0
    }

    /// Check if the window is frameless.
    pub fn is_frameless(&self) -> bool {
        self.has(Self::FRAMELESS)
    }

    /// Check if the window stays on top.
    pub fn stays_on_top(&self) -> bool {
        self.has(Self::STAYS_ON_TOP)
    }

    /// Check if the window has a minimize button.
    pub fn has_minimize_button(&self) -> bool {
        self.has(Self::MINIMIZE_BUTTON)
    }

    /// Check if the window has a maximize button.
    pub fn has_maximize_button(&self) -> bool {
        self.has(Self::MAXIMIZE_BUTTON)
    }

    /// Check if the window has a close button.
    pub fn has_close_button(&self) -> bool {
        self.has(Self::CLOSE_BUTTON)
    }

    /// Check if the window is resizable.
    pub fn is_resizable(&self) -> bool {
        self.has(Self::RESIZABLE)
    }

    /// Check if the window is movable.
    pub fn is_movable(&self) -> bool {
        self.has(Self::MOVABLE)
    }

    /// Check if the window has a title bar.
    pub fn has_title_bar(&self) -> bool {
        self.has(Self::TITLE_BAR) && !self.is_frameless()
    }

    /// Check if the window has a border.
    pub fn has_border(&self) -> bool {
        self.has(Self::BORDER) && !self.is_frameless()
    }
}

impl BitOr for WindowFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        WindowFlags(self.0 | rhs.0)
    }
}

impl BitOrAssign for WindowFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for WindowFlags {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        WindowFlags(self.0 & rhs.0)
    }
}

// ============================================================================
// Window Modality
// ============================================================================

/// The modality of a window.
///
/// Modality determines how the window interacts with other windows in terms
/// of input focus and blocking behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowModality {
    /// The window is not modal and does not block other windows.
    #[default]
    NonModal,
    /// The window is modal to its parent window.
    ///
    /// The parent window is blocked from receiving input while this window
    /// is visible, but other windows in the application can still receive input.
    WindowModal,
    /// The window is application modal.
    ///
    /// All other windows in the application are blocked from receiving input
    /// while this window is visible.
    ApplicationModal,
}

impl WindowModality {
    /// Check if the window is non-modal.
    pub fn is_non_modal(&self) -> bool {
        matches!(self, WindowModality::NonModal)
    }

    /// Check if the window is window-modal.
    pub fn is_window_modal(&self) -> bool {
        matches!(self, WindowModality::WindowModal)
    }

    /// Check if the window is application-modal.
    pub fn is_application_modal(&self) -> bool {
        matches!(self, WindowModality::ApplicationModal)
    }

    /// Check if the window is any kind of modal.
    pub fn is_modal(&self) -> bool {
        !self.is_non_modal()
    }
}

// ============================================================================
// Title Bar Button
// ============================================================================

/// Type of title bar button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TitleBarButton {
    Minimize,
    Maximize,
    Close,
}

/// State of a title bar button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ButtonState {
    hovered: bool,
    pressed: bool,
}

// ============================================================================
// Resize Edge
// ============================================================================

/// Edge or corner used for resizing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeEdge {
    None,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeEdge {
    /// Check if this is a valid resize edge (not None).
    fn is_some(&self) -> bool {
        !matches!(self, ResizeEdge::None)
    }
}

// ============================================================================
// Window
// ============================================================================

/// A top-level window widget.
///
/// Window provides a self-contained window with optional title bar, borders,
/// and window management capabilities. It supports various window states
/// (normal, minimized, maximized, fullscreen) and modality options.
///
/// # Features
///
/// - Title bar with minimize, maximize, and close buttons
/// - Resizable borders (when enabled)
/// - Movable by dragging the title bar
/// - Multiple window states (normal, minimized, maximized, fullscreen)
/// - Modality support (non-modal, window-modal, application-modal)
/// - Customizable appearance through flags
///
/// # Signals
///
/// - `close_requested()`: Emitted when the close button is clicked or close is requested
/// - `state_changed(WindowState)`: Emitted when the window state changes
/// - `title_changed(String)`: Emitted when the window title changes
/// - `flags_changed(WindowFlags)`: Emitted when the window flags change
/// - `modality_changed(WindowModality)`: Emitted when the window modality changes
/// - `activated()`: Emitted when the window becomes the active window
/// - `deactivated()`: Emitted when the window is no longer the active window
pub struct Window {
    /// Widget base.
    base: WidgetBase,

    /// The window title.
    title: String,

    /// The content widget ID.
    content_widget: Option<ObjectId>,

    /// The window state.
    state: WindowState,

    /// The previous state (before minimize/maximize/fullscreen).
    previous_state: WindowState,

    /// Window flags controlling appearance and behavior.
    flags: WindowFlags,

    /// Window modality.
    modality: WindowModality,

    /// Minimum window size.
    min_size: Size,

    /// Maximum window size (None means no maximum).
    max_size: Option<Size>,

    /// Geometry when in normal state (for restore from maximize/fullscreen).
    normal_geometry: Rect,

    /// Title bar height.
    title_bar_height: f32,

    /// Button size.
    button_size: f32,

    /// Border width for resize handles.
    border_width: f32,

    /// Resize handle size (extends beyond border for easier grabbing).
    resize_handle_size: f32,

    /// Content margins inside the window.
    content_margins: ContentMargins,

    // Visual styling
    /// Title bar background color.
    title_bar_color: Color,
    /// Title bar background when active.
    title_bar_active_color: Color,
    /// Title text color.
    title_text_color: Color,
    /// Background color of content area.
    content_background: Color,
    /// Border color.
    border_color: Color,
    /// Button background color.
    button_color: Color,
    /// Button hover color.
    button_hover_color: Color,
    /// Button pressed color.
    button_pressed_color: Color,
    /// Close button hover color (red).
    close_button_hover_color: Color,

    // Interaction state
    /// Button states.
    minimize_button_state: ButtonState,
    maximize_button_state: ButtonState,
    close_button_state: ButtonState,
    /// Whether dragging the title bar to move.
    dragging: bool,
    /// Whether resizing the window.
    resizing: bool,
    /// Current resize edge.
    resize_edge: ResizeEdge,
    /// Drag/resize start position (in global coordinates).
    drag_start: Point,
    /// Widget geometry at drag/resize start.
    drag_start_geometry: Rect,
    /// Whether the window is currently active.
    active: bool,

    // Signals
    /// Signal emitted when close is requested.
    pub close_requested: Signal<()>,
    /// Signal emitted when the window state changes.
    pub state_changed: Signal<WindowState>,
    /// Signal emitted when the title changes.
    pub title_changed: Signal<String>,
    /// Signal emitted when the flags change.
    pub flags_changed: Signal<WindowFlags>,
    /// Signal emitted when the modality changes.
    pub modality_changed: Signal<WindowModality>,
    /// Signal emitted when the window becomes active.
    pub activated: Signal<()>,
    /// Signal emitted when the window is deactivated.
    pub deactivated: Signal<()>,
}

impl Window {
    /// Create a new window with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::ClickFocus);
        base.set_size_policy(SizePolicyPair::new(SizePolicy::Preferred, SizePolicy::Preferred));

        Self {
            base,
            title: title.into(),
            content_widget: None,
            state: WindowState::Normal,
            previous_state: WindowState::Normal,
            flags: WindowFlags::DEFAULT,
            modality: WindowModality::NonModal,
            min_size: Size::new(150.0, 100.0),
            max_size: None,
            normal_geometry: Rect::new(0.0, 0.0, 640.0, 480.0),
            title_bar_height: 28.0,
            button_size: 20.0,
            border_width: 1.0,
            resize_handle_size: 5.0,
            content_margins: ContentMargins::uniform(0.0),
            title_bar_color: Color::from_rgb8(240, 240, 240),
            title_bar_active_color: Color::from_rgb8(200, 220, 240),
            title_text_color: Color::from_rgb8(40, 40, 40),
            content_background: Color::WHITE,
            border_color: Color::from_rgb8(160, 160, 160),
            button_color: Color::from_rgb8(240, 240, 240),
            button_hover_color: Color::from_rgb8(220, 220, 220),
            button_pressed_color: Color::from_rgb8(200, 200, 200),
            close_button_hover_color: Color::from_rgb8(232, 17, 35),
            minimize_button_state: ButtonState::default(),
            maximize_button_state: ButtonState::default(),
            close_button_state: ButtonState::default(),
            dragging: false,
            resizing: false,
            resize_edge: ResizeEdge::None,
            drag_start: Point::ZERO,
            drag_start_geometry: Rect::ZERO,
            active: false,
            close_requested: Signal::new(),
            state_changed: Signal::new(),
            title_changed: Signal::new(),
            flags_changed: Signal::new(),
            modality_changed: Signal::new(),
            activated: Signal::new(),
            deactivated: Signal::new(),
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
        let new_title = title.into();
        if self.title != new_title {
            self.title = new_title.clone();
            self.base.update();
            self.title_changed.emit(new_title);
        }
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
    pub fn content_widget(&self) -> Option<ObjectId> {
        self.content_widget
    }

    /// Set the content widget.
    pub fn set_content_widget(&mut self, widget_id: ObjectId) {
        self.content_widget = Some(widget_id);
        self.base.update();
    }

    /// Set content widget using builder pattern.
    pub fn with_content_widget(mut self, widget_id: ObjectId) -> Self {
        self.content_widget = Some(widget_id);
        self
    }

    // =========================================================================
    // Window State
    // =========================================================================

    /// Get the current window state.
    pub fn state(&self) -> WindowState {
        self.state
    }

    /// Set the window state.
    pub fn set_state(&mut self, state: WindowState) {
        if self.state != state {
            // Store previous state for restoration
            if self.state == WindowState::Normal {
                self.normal_geometry = self.base.geometry();
            }
            self.previous_state = self.state;
            self.state = state;
            self.state_changed.emit(state);
            self.base.update();
        }
    }

    /// Check if the window is minimized.
    pub fn is_minimized(&self) -> bool {
        self.state.is_minimized()
    }

    /// Check if the window is maximized.
    pub fn is_maximized(&self) -> bool {
        self.state.is_maximized()
    }

    /// Check if the window is fullscreen.
    pub fn is_fullscreen(&self) -> bool {
        self.state.is_fullscreen()
    }

    /// Minimize the window.
    pub fn minimize(&mut self) {
        self.set_state(WindowState::Minimized);
    }

    /// Maximize the window.
    pub fn maximize(&mut self) {
        self.set_state(WindowState::Maximized);
    }

    /// Enter fullscreen mode.
    pub fn show_fullscreen(&mut self) {
        self.set_state(WindowState::Fullscreen);
    }

    /// Restore the window to normal state.
    pub fn show_normal(&mut self) {
        if self.state != WindowState::Normal {
            // Restore normal geometry
            self.base.set_geometry(self.normal_geometry);
        }
        self.set_state(WindowState::Normal);
    }

    /// Toggle between maximized and normal states.
    pub fn toggle_maximize(&mut self) {
        if self.state == WindowState::Maximized {
            self.show_normal();
        } else {
            self.maximize();
        }
    }

    // =========================================================================
    // Window Flags
    // =========================================================================

    /// Get the window flags.
    pub fn flags(&self) -> WindowFlags {
        self.flags
    }

    /// Set the window flags.
    pub fn set_flags(&mut self, flags: WindowFlags) {
        if self.flags != flags {
            self.flags = flags;
            self.flags_changed.emit(flags);
            self.base.update();
        }
    }

    /// Set flags using builder pattern.
    pub fn with_flags(mut self, flags: WindowFlags) -> Self {
        self.flags = flags;
        self
    }

    // =========================================================================
    // Window Modality
    // =========================================================================

    /// Get the window modality.
    pub fn modality(&self) -> WindowModality {
        self.modality
    }

    /// Set the window modality.
    pub fn set_modality(&mut self, modality: WindowModality) {
        if self.modality != modality {
            self.modality = modality;
            self.modality_changed.emit(modality);
        }
    }

    /// Set modality using builder pattern.
    pub fn with_modality(mut self, modality: WindowModality) -> Self {
        self.modality = modality;
        self
    }

    /// Check if the window is modal.
    pub fn is_modal(&self) -> bool {
        self.modality.is_modal()
    }

    // =========================================================================
    // Size Constraints
    // =========================================================================

    /// Get the minimum window size.
    pub fn min_size(&self) -> Size {
        self.min_size
    }

    /// Set the minimum window size.
    pub fn set_min_size(&mut self, size: Size) {
        self.min_size = size;
    }

    /// Set minimum size using builder pattern.
    pub fn with_min_size(mut self, width: f32, height: f32) -> Self {
        self.min_size = Size::new(width, height);
        self
    }

    /// Get the maximum window size.
    pub fn max_size(&self) -> Option<Size> {
        self.max_size
    }

    /// Set the maximum window size.
    pub fn set_max_size(&mut self, size: Option<Size>) {
        self.max_size = size;
    }

    /// Set maximum size using builder pattern.
    pub fn with_max_size(mut self, width: f32, height: f32) -> Self {
        self.max_size = Some(Size::new(width, height));
        self
    }

    // =========================================================================
    // Size and Position (Builder Pattern)
    // =========================================================================

    /// Set the window size using builder pattern.
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.base.set_size(Size::new(width, height));
        self.normal_geometry.size = Size::new(width, height);
        self
    }

    /// Set the window position using builder pattern.
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.base.set_pos(Point::new(x, y));
        self.normal_geometry.origin = Point::new(x, y);
        self
    }

    // =========================================================================
    // Window Operations
    // =========================================================================

    /// Show the window.
    pub fn show(&mut self) {
        self.base.show();
    }

    /// Hide the window.
    pub fn hide(&mut self) {
        self.base.hide();
    }

    /// Close the window.
    ///
    /// This emits `close_requested` and then hides the window.
    pub fn close(&mut self) {
        self.close_requested.emit(());
        self.hide();
    }

    /// Move the window to the specified position.
    pub fn move_to(&mut self, x: f32, y: f32) {
        if self.flags.is_movable() {
            self.base.move_to(x, y);
            if self.state == WindowState::Normal {
                self.normal_geometry.origin = Point::new(x, y);
            }
        }
    }

    /// Resize the window.
    pub fn resize(&mut self, width: f32, height: f32) {
        if self.flags.is_resizable() {
            let clamped_width = width.max(self.min_size.width);
            let clamped_height = height.max(self.min_size.height);

            let (final_width, final_height) = if let Some(max) = self.max_size {
                (clamped_width.min(max.width), clamped_height.min(max.height))
            } else {
                (clamped_width, clamped_height)
            };

            self.base.resize(final_width, final_height);
            if self.state == WindowState::Normal {
                self.normal_geometry.size = Size::new(final_width, final_height);
            }
        }
    }

    // =========================================================================
    // Active State
    // =========================================================================

    /// Check if the window is currently active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Activate the window.
    pub fn activate(&mut self) {
        if !self.active {
            self.active = true;
            self.base.update();
            self.activated.emit(());
        }
    }

    /// Deactivate the window.
    pub fn deactivate(&mut self) {
        if self.active {
            self.active = false;
            self.base.update();
            self.deactivated.emit(());
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
    fn title_bar_rect(&self) -> Option<Rect> {
        if !self.flags.has_title_bar() {
            return None;
        }

        let rect = self.base.rect();
        Some(Rect::new(0.0, 0.0, rect.width(), self.title_bar_height))
    }

    /// Get the content area rectangle.
    pub fn content_rect(&self) -> Rect {
        let rect = self.base.rect();
        let title_bar_height = if self.flags.has_title_bar() {
            self.title_bar_height
        } else {
            0.0
        };
        let border = if self.flags.has_border() {
            self.border_width
        } else {
            0.0
        };

        Rect::new(
            border + self.content_margins.left,
            title_bar_height + self.content_margins.top,
            rect.width() - border * 2.0 - self.content_margins.horizontal(),
            rect.height() - title_bar_height - border - self.content_margins.vertical(),
        )
    }

    /// Get the minimize button rectangle.
    fn minimize_button_rect(&self) -> Option<Rect> {
        if !self.flags.has_minimize_button() || !self.flags.has_title_bar() {
            return None;
        }

        let title_rect = self.title_bar_rect()?;
        let padding = (self.title_bar_height - self.button_size) / 2.0;

        // Count buttons from the right
        let mut offset = padding;
        if self.flags.has_close_button() {
            offset += self.button_size + 2.0;
        }
        if self.flags.has_maximize_button() {
            offset += self.button_size + 2.0;
        }

        Some(Rect::new(
            title_rect.width() - offset - self.button_size,
            padding,
            self.button_size,
            self.button_size,
        ))
    }

    /// Get the maximize button rectangle.
    fn maximize_button_rect(&self) -> Option<Rect> {
        if !self.flags.has_maximize_button() || !self.flags.has_title_bar() {
            return None;
        }

        let title_rect = self.title_bar_rect()?;
        let padding = (self.title_bar_height - self.button_size) / 2.0;

        // Count buttons from the right
        let mut offset = padding;
        if self.flags.has_close_button() {
            offset += self.button_size + 2.0;
        }

        Some(Rect::new(
            title_rect.width() - offset - self.button_size,
            padding,
            self.button_size,
            self.button_size,
        ))
    }

    /// Get the close button rectangle.
    fn close_button_rect(&self) -> Option<Rect> {
        if !self.flags.has_close_button() || !self.flags.has_title_bar() {
            return None;
        }

        let title_rect = self.title_bar_rect()?;
        let padding = (self.title_bar_height - self.button_size) / 2.0;

        Some(Rect::new(
            title_rect.width() - padding - self.button_size,
            padding,
            self.button_size,
            self.button_size,
        ))
    }

    /// Determine which resize edge is at the given position.
    fn hit_test_resize_edge(&self, pos: Point) -> ResizeEdge {
        if !self.flags.is_resizable() {
            return ResizeEdge::None;
        }

        let rect = self.base.rect();
        let handle_size = self.resize_handle_size;

        let at_left = pos.x < handle_size;
        let at_right = pos.x > rect.width() - handle_size;
        let at_top = pos.y < handle_size;
        let at_bottom = pos.y > rect.height() - handle_size;

        match (at_top, at_bottom, at_left, at_right) {
            (true, false, true, false) => ResizeEdge::TopLeft,
            (true, false, false, true) => ResizeEdge::TopRight,
            (false, true, true, false) => ResizeEdge::BottomLeft,
            (false, true, false, true) => ResizeEdge::BottomRight,
            (true, false, false, false) => ResizeEdge::Top,
            (false, true, false, false) => ResizeEdge::Bottom,
            (false, false, true, false) => ResizeEdge::Left,
            (false, false, false, true) => ResizeEdge::Right,
            _ => ResizeEdge::None,
        }
    }

    // =========================================================================
    // Hit Testing
    // =========================================================================

    /// Check which button is at the given position.
    fn hit_test_button(&self, pos: Point) -> Option<TitleBarButton> {
        if let Some(close_rect) = self.close_button_rect() {
            if close_rect.contains(pos) {
                return Some(TitleBarButton::Close);
            }
        }
        if let Some(max_rect) = self.maximize_button_rect() {
            if max_rect.contains(pos) {
                return Some(TitleBarButton::Maximize);
            }
        }
        if let Some(min_rect) = self.minimize_button_rect() {
            if min_rect.contains(pos) {
                return Some(TitleBarButton::Minimize);
            }
        }
        None
    }

    /// Check if the position is in the title bar drag area.
    fn is_in_title_bar_drag_area(&self, pos: Point) -> bool {
        if let Some(title_rect) = self.title_bar_rect() {
            if !title_rect.contains(pos) {
                return false;
            }
            // Not over any button
            self.hit_test_button(pos).is_none()
        } else {
            false
        }
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
                TitleBarButton::Maximize => {
                    self.maximize_button_state.pressed = true;
                    self.base.update();
                    return true;
                }
                TitleBarButton::Minimize => {
                    self.minimize_button_state.pressed = true;
                    self.base.update();
                    return true;
                }
            }
        }

        // Check resize edges
        let resize_edge = self.hit_test_resize_edge(pos);
        if resize_edge.is_some() {
            self.resizing = true;
            self.resize_edge = resize_edge;
            self.drag_start = event.global_pos;
            self.drag_start_geometry = self.base.geometry();
            return true;
        }

        // Check title bar drag
        if self.flags.is_movable() && self.is_in_title_bar_drag_area(pos) {
            self.dragging = true;
            self.drag_start = event.global_pos;
            self.drag_start_geometry = self.base.geometry();
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
            if let Some(rect) = self.close_button_rect() {
                if rect.contains(pos) {
                    self.close_requested.emit(());
                }
            }
            self.base.update();
            return true;
        }

        if self.maximize_button_state.pressed {
            self.maximize_button_state.pressed = false;
            if let Some(rect) = self.maximize_button_rect() {
                if rect.contains(pos) {
                    self.toggle_maximize();
                }
            }
            self.base.update();
            return true;
        }

        if self.minimize_button_state.pressed {
            self.minimize_button_state.pressed = false;
            if let Some(rect) = self.minimize_button_rect() {
                if rect.contains(pos) {
                    self.minimize();
                }
            }
            self.base.update();
            return true;
        }

        // End drag/resize
        if self.dragging {
            self.dragging = false;
            return true;
        }

        if self.resizing {
            self.resizing = false;
            self.resize_edge = ResizeEdge::None;
            return true;
        }

        false
    }

    fn handle_mouse_move(&mut self, event: &MouseMoveEvent) -> bool {
        let pos = event.local_pos;

        // Update button hover states
        let new_close_hover = self.close_button_rect().is_some_and(|r| r.contains(pos));
        let new_max_hover = self.maximize_button_rect().is_some_and(|r| r.contains(pos));
        let new_min_hover = self.minimize_button_rect().is_some_and(|r| r.contains(pos));

        let hover_changed = self.close_button_state.hovered != new_close_hover
            || self.maximize_button_state.hovered != new_max_hover
            || self.minimize_button_state.hovered != new_min_hover;

        self.close_button_state.hovered = new_close_hover;
        self.maximize_button_state.hovered = new_max_hover;
        self.minimize_button_state.hovered = new_min_hover;

        if hover_changed {
            self.base.update();
        }

        // Handle dragging
        if self.dragging {
            let delta = Point::new(
                event.global_pos.x - self.drag_start.x,
                event.global_pos.y - self.drag_start.y,
            );

            let new_pos = Point::new(
                self.drag_start_geometry.origin.x + delta.x,
                self.drag_start_geometry.origin.y + delta.y,
            );
            self.base.set_pos(new_pos);
            if self.state == WindowState::Normal {
                self.normal_geometry.origin = new_pos;
            }
            return true;
        }

        // Handle resizing
        if self.resizing {
            let delta = Point::new(
                event.global_pos.x - self.drag_start.x,
                event.global_pos.y - self.drag_start.y,
            );

            let mut new_x = self.drag_start_geometry.origin.x;
            let mut new_y = self.drag_start_geometry.origin.y;
            let mut new_width = self.drag_start_geometry.size.width;
            let mut new_height = self.drag_start_geometry.size.height;

            match self.resize_edge {
                ResizeEdge::Top => {
                    new_y += delta.y;
                    new_height -= delta.y;
                }
                ResizeEdge::Bottom => {
                    new_height += delta.y;
                }
                ResizeEdge::Left => {
                    new_x += delta.x;
                    new_width -= delta.x;
                }
                ResizeEdge::Right => {
                    new_width += delta.x;
                }
                ResizeEdge::TopLeft => {
                    new_x += delta.x;
                    new_y += delta.y;
                    new_width -= delta.x;
                    new_height -= delta.y;
                }
                ResizeEdge::TopRight => {
                    new_y += delta.y;
                    new_width += delta.x;
                    new_height -= delta.y;
                }
                ResizeEdge::BottomLeft => {
                    new_x += delta.x;
                    new_width -= delta.x;
                    new_height += delta.y;
                }
                ResizeEdge::BottomRight => {
                    new_width += delta.x;
                    new_height += delta.y;
                }
                ResizeEdge::None => {}
            }

            // Apply size constraints
            new_width = new_width.max(self.min_size.width);
            new_height = new_height.max(self.min_size.height);

            if let Some(max) = self.max_size {
                new_width = new_width.min(max.width);
                new_height = new_height.min(max.height);
            }

            self.base.set_geometry(Rect::new(new_x, new_y, new_width, new_height));
            if self.state == WindowState::Normal {
                self.normal_geometry = Rect::new(new_x, new_y, new_width, new_height);
            }

            return true;
        }

        hover_changed
    }

    fn handle_double_click(&mut self, event: &MouseDoubleClickEvent) -> bool {
        if event.button != MouseButton::Left {
            return false;
        }

        // Double-click on title bar toggles maximize
        if self.flags.has_maximize_button() && self.is_in_title_bar_drag_area(event.local_pos) {
            self.toggle_maximize();
            return true;
        }

        false
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Escape to close if modal or has close button
        if event.key == Key::Escape && (self.is_modal() || self.flags.has_close_button()) {
            self.close_requested.emit(());
            return true;
        }

        false
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn paint_border(&self, ctx: &mut PaintContext<'_>) {
        if !self.flags.has_border() {
            return;
        }

        let rect = self.base.rect();
        let border_rect = Rect::new(0.0, 0.0, rect.width(), rect.height());
        let stroke = Stroke::new(self.border_color, self.border_width);
        ctx.renderer().stroke_rect(border_rect, &stroke);
    }

    fn paint_title_bar(&self, ctx: &mut PaintContext<'_>) {
        let Some(title_rect) = self.title_bar_rect() else {
            return;
        };

        // Background
        let bg_color = if self.active {
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
        // Minimize button
        if let Some(rect) = self.minimize_button_rect() {
            let bg = if self.minimize_button_state.pressed {
                self.button_pressed_color
            } else if self.minimize_button_state.hovered {
                self.button_hover_color
            } else {
                self.button_color
            };
            ctx.renderer().fill_rect(rect, bg);

            // Draw minimize icon (horizontal line)
            let icon_margin = 5.0;
            let line_y = rect.origin.y + rect.height() / 2.0 + 3.0;
            let icon_color = Color::from_rgb8(80, 80, 80);
            let stroke = Stroke::new(icon_color, 1.5);
            ctx.renderer().draw_line(
                Point::new(rect.origin.x + icon_margin, line_y),
                Point::new(rect.origin.x + rect.width() - icon_margin, line_y),
                &stroke,
            );
        }

        // Maximize button
        if let Some(rect) = self.maximize_button_rect() {
            let bg = if self.maximize_button_state.pressed {
                self.button_pressed_color
            } else if self.maximize_button_state.hovered {
                self.button_hover_color
            } else {
                self.button_color
            };
            ctx.renderer().fill_rect(rect, bg);

            // Draw maximize/restore icon
            let icon_margin = 4.0;
            let icon_color = Color::from_rgb8(80, 80, 80);
            let stroke = Stroke::new(icon_color, 1.0);

            if self.state == WindowState::Maximized {
                // Draw restore icon (two overlapping rectangles)
                let small_rect = Rect::new(
                    rect.origin.x + icon_margin + 2.0,
                    rect.origin.y + icon_margin,
                    rect.width() - icon_margin * 2.0 - 2.0,
                    rect.height() - icon_margin * 2.0 - 2.0,
                );
                let large_rect = Rect::new(
                    rect.origin.x + icon_margin,
                    rect.origin.y + icon_margin + 2.0,
                    rect.width() - icon_margin * 2.0 - 2.0,
                    rect.height() - icon_margin * 2.0 - 2.0,
                );
                ctx.renderer().stroke_rect(small_rect, &stroke);
                ctx.renderer().fill_rect(large_rect, bg);
                ctx.renderer().stroke_rect(large_rect, &stroke);
            } else {
                // Draw maximize icon (single rectangle)
                let icon_rect = Rect::new(
                    rect.origin.x + icon_margin,
                    rect.origin.y + icon_margin,
                    rect.width() - icon_margin * 2.0,
                    rect.height() - icon_margin * 2.0,
                );
                ctx.renderer().stroke_rect(icon_rect, &stroke);
            }
        }

        // Close button
        if let Some(rect) = self.close_button_rect() {
            let bg = if self.close_button_state.pressed {
                self.button_pressed_color
            } else if self.close_button_state.hovered {
                self.close_button_hover_color
            } else {
                self.button_color
            };
            ctx.renderer().fill_rect(rect, bg);

            // Draw X icon
            let icon_margin = 5.0;
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
        ctx.renderer().fill_rect(content_rect, self.content_background);
    }
}

impl Widget for Window {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        let preferred = Size::new(640.0, 480.0);
        SizeHint::new(preferred).with_minimum(self.min_size)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Don't paint if minimized
        if self.state == WindowState::Minimized {
            return;
        }

        // Paint in order: content background, title bar, border
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
                let changed = self.close_button_state.hovered
                    || self.maximize_button_state.hovered
                    || self.minimize_button_state.hovered;
                self.close_button_state.hovered = false;
                self.maximize_button_state.hovered = false;
                self.minimize_button_state.hovered = false;
                if changed {
                    self.base.update();
                }
                false
            }
            WidgetEvent::FocusIn(_) => {
                self.activate();
                true
            }
            WidgetEvent::FocusOut(_) => {
                self.deactivate();
                true
            }
            _ => false,
        }
    }
}

impl Object for Window {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Default for Window {
    fn default() -> Self {
        Self::new("Window")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_state() {
        assert!(WindowState::Normal.is_normal());
        assert!(WindowState::Minimized.is_minimized());
        assert!(WindowState::Maximized.is_maximized());
        assert!(WindowState::Fullscreen.is_fullscreen());
    }

    #[test]
    fn test_window_flags_default() {
        let flags = WindowFlags::DEFAULT;
        assert!(flags.has_minimize_button());
        assert!(flags.has_maximize_button());
        assert!(flags.has_close_button());
        assert!(flags.is_resizable());
        assert!(flags.is_movable());
        assert!(flags.has_title_bar());
        assert!(flags.has_border());
        assert!(!flags.is_frameless());
    }

    #[test]
    fn test_window_flags_frameless() {
        let flags = WindowFlags::FRAMELESS;
        assert!(flags.is_frameless());
        assert!(!flags.has_title_bar()); // frameless overrides title bar
        assert!(!flags.has_border()); // frameless overrides border
    }

    #[test]
    fn test_window_flags_bitwise() {
        let flags = WindowFlags::CLOSE_BUTTON | WindowFlags::MOVABLE;
        assert!(flags.has_close_button());
        assert!(flags.is_movable());
        assert!(!flags.has_minimize_button());
        assert!(!flags.has_maximize_button());
    }

    #[test]
    fn test_window_modality() {
        assert!(WindowModality::NonModal.is_non_modal());
        assert!(!WindowModality::NonModal.is_modal());

        assert!(WindowModality::WindowModal.is_window_modal());
        assert!(WindowModality::WindowModal.is_modal());

        assert!(WindowModality::ApplicationModal.is_application_modal());
        assert!(WindowModality::ApplicationModal.is_modal());
    }

    #[test]
    fn test_window_flags_dialog() {
        let flags = WindowFlags::DIALOG;
        assert!(flags.has_close_button());
        assert!(flags.is_movable());
        assert!(flags.has_title_bar());
        assert!(flags.has_border());
        assert!(!flags.has_minimize_button());
        assert!(!flags.has_maximize_button());
        assert!(!flags.is_resizable());
    }
}
