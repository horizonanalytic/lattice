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

use std::collections::HashMap;
use std::ops::{BitAnd, BitOr, BitOrAssign};

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{Color, Point, Rect, Renderer, Size, Stroke};

use crate::widget::layout::ContentMargins;
use crate::widget::{
    CloseEvent, FocusManager, FocusPolicy, FocusReason, Key, KeyPressEvent, KeyReleaseEvent,
    MouseButton, MouseDoubleClickEvent, MouseMoveEvent, MousePressEvent, MouseReleaseEvent,
    PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetAccess, WidgetBase,
    WidgetEvent,
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

    /// Window has a transparent background.
    ///
    /// When set, the window background will be transparent, allowing
    /// content behind the window to show through. This is useful for
    /// custom-shaped windows or windows with rounded corners.
    ///
    /// Note: Transparency support varies by platform and compositor.
    pub const TRANSPARENT: WindowFlags = WindowFlags(1 << 9);

    /// Window stays below other windows (desktop widget style).
    pub const STAYS_ON_BOTTOM: WindowFlags = WindowFlags(1 << 10);

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

    /// Check if the window has a transparent background.
    pub fn is_transparent(&self) -> bool {
        self.has(Self::TRANSPARENT)
    }

    /// Check if the window stays on bottom.
    pub fn stays_on_bottom(&self) -> bool {
        self.has(Self::STAYS_ON_BOTTOM)
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

    /// Aspect ratio constraint (width / height).
    ///
    /// When set, the window will maintain this aspect ratio during resize operations.
    aspect_ratio: Option<f32>,

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

    // Mnemonic state
    /// Whether the Alt key is currently held (for mnemonic underline display).
    alt_held: bool,
    /// Current mnemonic cycling state: maps mnemonic character to index in matches.
    mnemonic_cycle_state: HashMap<char, usize>,
    /// The last mnemonic key pressed (for cycle state management).
    last_mnemonic_key: Option<char>,

    // Default button state
    /// The ObjectId of the default button in this window.
    ///
    /// The default button is activated when Enter is pressed at the window level
    /// and no focused widget handles the Enter key.
    default_button: Option<ObjectId>,

    // Shortcut state
    /// Registry of keyboard shortcuts mapped to button ObjectIds.
    ///
    /// Multiple buttons can be registered for the same shortcut; they are
    /// cycled through on repeated presses.
    shortcut_registry: HashMap<crate::widget::KeySequence, Vec<ObjectId>>,
    /// Cycle state for shortcuts: maps KeySequence to current index.
    shortcut_cycle_state: HashMap<crate::widget::KeySequence, usize>,
    /// Last shortcut pressed (for cycle management).
    last_shortcut: Option<crate::widget::KeySequence>,

    // Focus management
    /// The focus manager for this window's widget tree.
    ///
    /// Handles keyboard focus tracking and Tab/Shift+Tab navigation
    /// for all widgets contained within this window.
    focus_manager: FocusManager,

    // Signals
    /// Signal emitted when the window is about to close.
    ///
    /// This signal is emitted after the close has been accepted (either no
    /// close handler was set, or the handler did not call `ignore()`).
    /// Connect to this signal to perform cleanup or save state before
    /// the window is hidden.
    ///
    /// To prevent closing, use [`set_close_handler()`](Self::set_close_handler) instead.
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
    /// Signal emitted when an Alt+key mnemonic combination is pressed.
    ///
    /// The parameter is the lowercase mnemonic key character. Connect a handler
    /// to this signal to perform mnemonic dispatch (finding matching labels
    /// and transferring focus to their buddy widgets).
    pub mnemonic_key_pressed: Signal<char>,

    /// Signal emitted when Enter is pressed and should activate the default button.
    ///
    /// This signal is emitted when:
    /// 1. The Enter key is pressed at the window level
    /// 2. A default button is set for this window
    /// 3. No focused widget consumed the Enter key event
    ///
    /// The parameter is the ObjectId of the default button that should be activated.
    /// Connect a handler to this signal to call `click()` on the default button.
    ///
    /// # Example
    ///
    /// ```ignore
    /// window.default_button_activated.connect(|button_id| {
    ///     if let Some(button) = storage.get_widget_mut(button_id) {
    ///         if let Some(push_button) = button.downcast_mut::<PushButton>() {
    ///             push_button.click();
    ///         }
    ///     }
    /// });
    /// ```
    pub default_button_activated: Signal<ObjectId>,

    /// Signal emitted when a keyboard shortcut should activate a button.
    ///
    /// This signal is emitted when:
    /// 1. A key combination matching a registered shortcut is pressed
    /// 2. No focused widget consumed the key event
    ///
    /// The parameter is the ObjectId of the button that should be activated.
    /// Connect a handler to this signal to call `click()` on the button.
    pub shortcut_activated: Signal<ObjectId>,

    /// Signal emitted when the focused widget changes.
    ///
    /// The parameter is `Some(ObjectId)` when a widget gains focus, or
    /// `None` when all widgets lose focus.
    pub focus_changed: Signal<Option<ObjectId>>,

    /// Signal emitted when Tab key is pressed for focus navigation.
    ///
    /// Connect a handler to this signal to call `window.focus_next(storage)`
    /// to move focus to the next focusable widget.
    ///
    /// # Example
    ///
    /// ```ignore
    /// window.tab_pressed.connect(|| {
    ///     window.focus_next(&mut storage);
    /// });
    /// ```
    pub tab_pressed: Signal<()>,

    /// Signal emitted when Shift+Tab is pressed for reverse focus navigation.
    ///
    /// Connect a handler to this signal to call `window.focus_previous(storage)`
    /// to move focus to the previous focusable widget.
    pub backtab_pressed: Signal<()>,

    // Close event handling
    /// Optional handler for close events.
    ///
    /// This handler is called before the window closes. The handler receives
    /// a mutable reference to a [`CloseEvent`] and can call `ignore()` on it
    /// to prevent the window from closing.
    ///
    /// Unlike signals which are fire-and-forget, this callback allows the
    /// handler to veto the close operation.
    close_handler: Option<Box<dyn FnMut(&mut CloseEvent) + Send + Sync>>,
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
            aspect_ratio: None,
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
            alt_held: false,
            mnemonic_cycle_state: HashMap::new(),
            last_mnemonic_key: None,
            default_button: None,
            shortcut_registry: HashMap::new(),
            shortcut_cycle_state: HashMap::new(),
            last_shortcut: None,
            focus_manager: FocusManager::new(),
            close_requested: Signal::new(),
            state_changed: Signal::new(),
            title_changed: Signal::new(),
            flags_changed: Signal::new(),
            modality_changed: Signal::new(),
            activated: Signal::new(),
            deactivated: Signal::new(),
            mnemonic_key_pressed: Signal::new(),
            default_button_activated: Signal::new(),
            shortcut_activated: Signal::new(),
            focus_changed: Signal::new(),
            tab_pressed: Signal::new(),
            backtab_pressed: Signal::new(),
            close_handler: None,
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

    /// Get the aspect ratio constraint.
    ///
    /// Returns the aspect ratio (width / height) if set, or `None` if the window
    /// can be resized to any aspect ratio.
    pub fn aspect_ratio(&self) -> Option<f32> {
        self.aspect_ratio
    }

    /// Set the aspect ratio constraint (width / height).
    ///
    /// When set, the window will maintain this aspect ratio during resize operations.
    /// Pass `None` to remove the constraint.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Set 16:9 aspect ratio
    /// window.set_aspect_ratio(Some(16.0 / 9.0));
    ///
    /// // Remove aspect ratio constraint
    /// window.set_aspect_ratio(None);
    /// ```
    pub fn set_aspect_ratio(&mut self, ratio: Option<f32>) {
        self.aspect_ratio = ratio;
    }

    /// Set aspect ratio using builder pattern.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let window = Window::new("Video Player")
    ///     .with_aspect_ratio(16.0 / 9.0)
    ///     .with_size(1280.0, 720.0);
    /// ```
    pub fn with_aspect_ratio(mut self, ratio: f32) -> Self {
        self.aspect_ratio = Some(ratio);
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
    /// This creates a [`CloseEvent`], calls the close handler (if set), and
    /// proceeds to close the window unless the handler called `ignore()`.
    ///
    /// Returns `true` if the window was closed, `false` if the close was vetoed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// window.set_close_handler(|event| {
    ///     if has_unsaved_changes() {
    ///         event.ignore(); // Prevent close
    ///     }
    /// });
    ///
    /// if window.close() {
    ///     println!("Window closed");
    /// } else {
    ///     println!("Close was prevented");
    /// }
    /// ```
    pub fn close(&mut self) -> bool {
        let mut event = CloseEvent::new();

        // Call the close handler if set
        if let Some(ref mut handler) = self.close_handler {
            handler(&mut event);
        }

        // Check if close was vetoed
        if event.is_accepted() {
            self.close_requested.emit(());
            self.hide();
            true
        } else {
            false
        }
    }

    /// Force close the window, bypassing any close handler.
    ///
    /// This method closes the window unconditionally, ignoring any
    /// close handler that might have been set. Use this when you need
    /// to ensure the window closes regardless of user preferences.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Application is shutting down, force close all windows
    /// window.force_close();
    /// ```
    pub fn force_close(&mut self) {
        self.close_requested.emit(());
        self.hide();
    }

    /// Set the close handler.
    ///
    /// The handler is called before the window closes. It receives a mutable
    /// reference to a [`CloseEvent`] and can call `ignore()` on it to prevent
    /// the window from closing.
    ///
    /// # Example
    ///
    /// ```ignore
    /// window.set_close_handler(|event| {
    ///     if has_unsaved_changes() {
    ///         // Show "Save changes?" dialog
    ///         if !user_confirmed_discard() {
    ///             event.ignore();
    ///         }
    ///     }
    /// });
    /// ```
    pub fn set_close_handler<F>(&mut self, handler: F)
    where
        F: FnMut(&mut CloseEvent) + Send + Sync + 'static,
    {
        self.close_handler = Some(Box::new(handler));
    }

    /// Clear the close handler.
    ///
    /// After calling this, `close()` will always succeed.
    pub fn clear_close_handler(&mut self) {
        self.close_handler = None;
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
    // Default Button
    // =========================================================================

    /// Get the default button's ObjectId, if one is set.
    ///
    /// The default button is activated when Enter is pressed in the window
    /// and no focused widget handles the Enter key.
    pub fn default_button(&self) -> Option<ObjectId> {
        self.default_button
    }

    /// Set the default button for this window.
    ///
    /// Pass `Some(id)` to set a button as default, or `None` to clear the default.
    /// When Enter is pressed in the window and the default button is set,
    /// the `default_button_activated` signal is emitted with the button's ObjectId.
    ///
    /// Only one button per window should be set as default. Setting a new default
    /// does not automatically unset `is_default` on the previous button - that
    /// is the responsibility of the application code.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Set up the default button
    /// let ok_button = PushButton::new("OK").with_default(true);
    /// let ok_id = ok_button.object_id();
    /// // ... add ok_button to widget storage ...
    ///
    /// window.set_default_button(Some(ok_id));
    ///
    /// // Connect the activation signal
    /// window.default_button_activated.connect(move |button_id| {
    ///     // Activate the button (application handles this)
    /// });
    /// ```
    pub fn set_default_button(&mut self, button_id: Option<ObjectId>) {
        self.default_button = button_id;
    }

    /// Set default button using builder pattern.
    pub fn with_default_button(mut self, button_id: ObjectId) -> Self {
        self.default_button = Some(button_id);
        self
    }

    // =========================================================================
    // Mnemonic State
    // =========================================================================

    /// Check if the Alt key is currently held.
    ///
    /// This is used to determine whether mnemonic underlines should be displayed.
    pub fn is_alt_held(&self) -> bool {
        self.alt_held
    }

    /// Clear the mnemonic cycle state.
    ///
    /// Called when Alt is released or focus changes.
    fn reset_mnemonic_cycle(&mut self) {
        self.mnemonic_cycle_state.clear();
        self.last_mnemonic_key = None;
    }

    /// Get the current cycle index for a mnemonic key and advance it.
    ///
    /// Returns the index to use for this mnemonic activation.
    /// If this is a different key than the last one pressed, resets to 0.
    fn advance_mnemonic_cycle(&mut self, key: char, num_matches: usize) -> usize {
        if num_matches == 0 {
            return 0;
        }

        let index = if self.last_mnemonic_key == Some(key) {
            // Same key pressed again - cycle to next
            let current = self.mnemonic_cycle_state.get(&key).copied().unwrap_or(0);
            (current + 1) % num_matches
        } else {
            // New mnemonic key - start at first match
            self.reset_mnemonic_cycle();
            0
        };

        self.mnemonic_cycle_state.insert(key, index);
        self.last_mnemonic_key = Some(key);
        index
    }

    // =========================================================================
    // Shortcut Registry
    // =========================================================================

    /// Register a keyboard shortcut for a button.
    ///
    /// Multiple buttons can be registered for the same shortcut; pressing the
    /// shortcut repeatedly will cycle through them.
    ///
    /// # Arguments
    ///
    /// * `shortcut` - The key sequence that triggers the button
    /// * `button_id` - The ObjectId of the button to activate
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::widget::KeySequence;
    ///
    /// // Register Ctrl+S to trigger the save button
    /// window.register_shortcut(KeySequence::ctrl(Key::S), save_button_id);
    /// ```
    pub fn register_shortcut(
        &mut self,
        shortcut: crate::widget::KeySequence,
        button_id: ObjectId,
    ) {
        self.shortcut_registry
            .entry(shortcut)
            .or_default()
            .push(button_id);
    }

    /// Unregister a keyboard shortcut for a button.
    ///
    /// Removes the button from the list of targets for this shortcut.
    /// If the button is not registered for this shortcut, this is a no-op.
    pub fn unregister_shortcut(
        &mut self,
        shortcut: &crate::widget::KeySequence,
        button_id: ObjectId,
    ) {
        if let Some(buttons) = self.shortcut_registry.get_mut(shortcut) {
            buttons.retain(|&id| id != button_id);
            if buttons.is_empty() {
                self.shortcut_registry.remove(shortcut);
            }
        }
    }

    /// Unregister all shortcuts for a button.
    ///
    /// Call this when removing a button from the window.
    pub fn unregister_all_shortcuts_for(&mut self, button_id: ObjectId) {
        self.shortcut_registry.retain(|_, buttons| {
            buttons.retain(|&id| id != button_id);
            !buttons.is_empty()
        });
    }

    /// Clear the shortcut cycle state.
    ///
    /// Called when a different key is pressed or focus changes.
    fn reset_shortcut_cycle(&mut self) {
        self.shortcut_cycle_state.clear();
        self.last_shortcut = None;
    }

    /// Get the button to activate for a shortcut and advance the cycle.
    ///
    /// Returns `Some(ObjectId)` if there's a button registered for this shortcut,
    /// `None` otherwise.
    fn get_shortcut_target(
        &mut self,
        shortcut: &crate::widget::KeySequence,
    ) -> Option<ObjectId> {
        // Get button count first to avoid borrow issues
        let num_buttons = self
            .shortcut_registry
            .get(shortcut)
            .map(|b| b.len())
            .unwrap_or(0);

        if num_buttons == 0 {
            return None;
        }

        let index = if self.last_shortcut.as_ref() == Some(shortcut) {
            // Same shortcut pressed again - cycle to next
            let current = self.shortcut_cycle_state.get(shortcut).copied().unwrap_or(0);
            (current + 1) % num_buttons
        } else {
            // Different shortcut - start at first match
            self.reset_shortcut_cycle();
            0
        };

        self.shortcut_cycle_state.insert(shortcut.clone(), index);
        self.last_shortcut = Some(shortcut.clone());

        // Get the button at the computed index
        self.shortcut_registry
            .get(shortcut)
            .and_then(|buttons| buttons.get(index).copied())
    }

    /// Try to dispatch a key event to a registered shortcut.
    ///
    /// Returns `true` if a shortcut was matched and the `shortcut_activated`
    /// signal was emitted.
    pub fn try_dispatch_shortcut(&mut self, event: &KeyPressEvent) -> bool {
        // Don't dispatch on key repeat
        if event.is_repeat {
            return false;
        }

        // Build the key sequence from the event
        let shortcut = crate::widget::KeySequence::new(event.key, event.modifiers);

        // Try to find a matching button
        if let Some(button_id) = self.get_shortcut_target(&shortcut) {
            self.shortcut_activated.emit(button_id);
            return true;
        }

        false
    }

    // =========================================================================
    // Focus Management
    // =========================================================================

    /// Get an immutable reference to the window's focus manager.
    ///
    /// The focus manager tracks which widget has keyboard focus and handles
    /// Tab/Shift+Tab navigation.
    pub fn focus_manager(&self) -> &FocusManager {
        &self.focus_manager
    }

    /// Get a mutable reference to the window's focus manager.
    pub fn focus_manager_mut(&mut self) -> &mut FocusManager {
        &mut self.focus_manager
    }

    /// Get the currently focused widget in this window.
    pub fn focused_widget(&self) -> Option<ObjectId> {
        self.focus_manager.focused_widget()
    }

    /// Set focus to a specific widget.
    ///
    /// This is a convenience method that delegates to the focus manager.
    /// The widget must have an appropriate focus policy to receive focus.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`
    /// * `widget_id` - The widget to focus
    /// * `reason` - The reason for the focus change
    ///
    /// # Returns
    ///
    /// `true` if focus was successfully set, `false` otherwise.
    pub fn set_focus<S: WidgetAccess>(
        &mut self,
        storage: &mut S,
        widget_id: ObjectId,
        reason: FocusReason,
    ) -> bool {
        let old_focused = self.focus_manager.focused_widget();
        let result = self.focus_manager.set_focus(storage, widget_id, reason);
        let new_focused = self.focus_manager.focused_widget();

        // Emit focus_changed signal if focus actually changed
        if result && old_focused != new_focused {
            self.focus_changed.emit(new_focused);
        }

        result
    }

    /// Clear focus from all widgets in this window.
    ///
    /// After calling this, no widget will have focus.
    pub fn clear_focus<S: WidgetAccess>(&mut self, storage: &mut S, reason: FocusReason) {
        let old_focused = self.focus_manager.focused_widget();
        self.focus_manager.clear_focus(storage, reason);

        if old_focused.is_some() {
            self.focus_changed.emit(None);
        }
    }

    /// Move focus to the next focusable widget (Tab navigation).
    ///
    /// Tab order is determined by depth-first traversal of the widget tree.
    /// Only widgets with `TabFocus` or `StrongFocus` policy participate.
    ///
    /// # Returns
    ///
    /// `true` if focus was moved, `false` if no focusable widget was found.
    pub fn focus_next<S: WidgetAccess>(&mut self, storage: &mut S) -> bool {
        let Some(root_id) = self.content_widget else {
            return false;
        };

        let old_focused = self.focus_manager.focused_widget();
        let result = self.focus_manager.focus_next(storage, root_id);
        let new_focused = self.focus_manager.focused_widget();

        if result && old_focused != new_focused {
            self.focus_changed.emit(new_focused);
        }

        result
    }

    /// Move focus to the previous focusable widget (Shift+Tab navigation).
    ///
    /// # Returns
    ///
    /// `true` if focus was moved, `false` if no focusable widget was found.
    pub fn focus_previous<S: WidgetAccess>(&mut self, storage: &mut S) -> bool {
        let Some(root_id) = self.content_widget else {
            return false;
        };

        let old_focused = self.focus_manager.focused_widget();
        let result = self.focus_manager.focus_previous(storage, root_id);
        let new_focused = self.focus_manager.focused_widget();

        if result && old_focused != new_focused {
            self.focus_changed.emit(new_focused);
        }

        result
    }

    /// Handle click-to-focus for a mouse event.
    ///
    /// This method should be called when a mouse press event occurs to
    /// potentially transfer focus to the clicked widget. It performs hit
    /// testing to find the widget under the cursor and sets focus if the
    /// widget accepts click focus.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`
    /// * `window_pos` - The position of the click in window coordinates
    ///
    /// # Returns
    ///
    /// `true` if focus was transferred to a new widget, `false` otherwise.
    pub fn handle_click_focus<S: WidgetAccess>(
        &mut self,
        storage: &mut S,
        window_pos: Point,
    ) -> bool {
        use crate::widget::EventDispatcher;

        let Some(root_id) = self.content_widget else {
            return false;
        };

        // Hit test to find the widget under the cursor
        let Some(target_id) = EventDispatcher::hit_test(storage, root_id, window_pos) else {
            return false;
        };

        // Check if the target widget accepts click focus
        let accepts_click = {
            let Some(widget) = storage.get_widget(target_id) else {
                return false;
            };
            widget.widget_base().accepts_click_focus()
        };

        if accepts_click {
            self.set_focus(storage, target_id, FocusReason::Mouse)
        } else {
            false
        }
    }

    /// Dispatch a mnemonic key press to matching widgets.
    ///
    /// This method searches the window's widget tree for widgets with mnemonics
    /// matching the given key, activates the appropriate one (with cycling support
    /// for multiple matches), and returns the buddy widget ID for focus transfer.
    ///
    /// # Arguments
    ///
    /// * `storage` - Widget storage implementing `WidgetAccess`.
    /// * `mnemonic_key` - The lowercase mnemonic key character.
    ///
    /// # Returns
    ///
    /// The ObjectId of the buddy widget to receive focus, or `None` if no
    /// matching mnemonic was found or the widget has no buddy.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // In response to Window::mnemonic_key_pressed signal:
    /// window.mnemonic_key_pressed.connect(|key| {
    ///     if let Some(buddy_id) = window.dispatch_mnemonic(&mut storage, key) {
    ///         focus_manager.set_focus(&mut storage, buddy_id, FocusReason::Shortcut);
    ///     }
    /// });
    /// ```
    pub fn dispatch_mnemonic<S: WidgetAccess>(
        &mut self,
        storage: &S,
        mnemonic_key: char,
    ) -> Option<ObjectId> {
        // Find all matching widgets
        let matching_widgets = self.find_mnemonic_widgets(storage, mnemonic_key);

        if matching_widgets.is_empty() {
            return None;
        }

        // Get the cycle index for this key
        let index = self.advance_mnemonic_cycle(mnemonic_key, matching_widgets.len());

        // Get the widget at this index
        let widget_id = matching_widgets[index];

        // Activate the mnemonic
        if let Some(widget) = storage.get_widget(widget_id) {
            widget.activate_mnemonic()
        } else {
            None
        }
    }

    /// Find all widgets in the window's content tree that match the given mnemonic key.
    ///
    /// Returns a list of ObjectIds for widgets with matching mnemonics, in
    /// depth-first traversal order. Only visible and enabled widgets are included.
    fn find_mnemonic_widgets<S: WidgetAccess>(
        &self,
        storage: &S,
        mnemonic_key: char,
    ) -> Vec<ObjectId> {
        let mut matches = Vec::new();

        // Start from the content widget
        if let Some(root_id) = self.content_widget {
            self.collect_mnemonic_widgets_recursive(storage, root_id, mnemonic_key, &mut matches);
        }

        matches
    }

    /// Recursively collect widgets with matching mnemonics.
    fn collect_mnemonic_widgets_recursive<S: WidgetAccess>(
        &self,
        storage: &S,
        widget_id: ObjectId,
        mnemonic_key: char,
        matches: &mut Vec<ObjectId>,
    ) {
        let Some(widget) = storage.get_widget(widget_id) else {
            return;
        };

        // Only check visible and enabled widgets
        if widget.is_visible() && widget.is_enabled() {
            // Check if this widget has a matching mnemonic
            if widget.matches_mnemonic_key(mnemonic_key) {
                matches.push(widget_id);
            }
        }

        // Recurse to children
        for child_id in storage.get_children(widget_id) {
            self.collect_mnemonic_widgets_recursive(storage, child_id, mnemonic_key, matches);
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
                    self.close();
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

            // Apply aspect ratio constraint if set
            if let Some(ratio) = self.aspect_ratio {
                // Determine which dimension is "primary" based on the resize edge
                // Primary dimension drives the constraint; the other adjusts to match
                let (primary_width, adjust_for_edge) = match self.resize_edge {
                    // Horizontal edges: width is primary
                    ResizeEdge::Left | ResizeEdge::Right => (true, false),
                    // Vertical edges: height is primary
                    ResizeEdge::Top | ResizeEdge::Bottom => (false, false),
                    // Corners: use width as primary (more intuitive for most users)
                    ResizeEdge::TopLeft | ResizeEdge::TopRight |
                    ResizeEdge::BottomLeft | ResizeEdge::BottomRight => (true, true),
                    ResizeEdge::None => (true, false),
                };

                if primary_width {
                    // Adjust height to match width
                    let target_height = new_width / ratio;
                    // Clamp to min/max
                    let target_height = target_height.max(self.min_size.height);
                    let target_height = if let Some(max) = self.max_size {
                        target_height.min(max.height)
                    } else {
                        target_height
                    };

                    // If height changed significantly, we may need to adjust width too
                    // to maintain the ratio with the clamped height
                    if (target_height - new_width / ratio).abs() > 0.5 {
                        new_width = target_height * ratio;
                        // Re-clamp width
                        new_width = new_width.max(self.min_size.width);
                        if let Some(max) = self.max_size {
                            new_width = new_width.min(max.width);
                        }
                    }

                    // Adjust position for edges that move origin
                    if adjust_for_edge {
                        let height_delta = target_height - new_height;
                        match self.resize_edge {
                            ResizeEdge::TopLeft | ResizeEdge::TopRight => {
                                new_y -= height_delta;
                            }
                            _ => {}
                        }
                    }
                    new_height = target_height;
                } else {
                    // Adjust width to match height
                    let target_width = new_height * ratio;
                    // Clamp to min/max
                    let target_width = target_width.max(self.min_size.width);
                    let target_width = if let Some(max) = self.max_size {
                        target_width.min(max.width)
                    } else {
                        target_width
                    };

                    // If width changed significantly, adjust height too
                    if (target_width - new_height * ratio).abs() > 0.5 {
                        new_height = target_width / ratio;
                        // Re-clamp height
                        new_height = new_height.max(self.min_size.height);
                        if let Some(max) = self.max_size {
                            new_height = new_height.min(max.height);
                        }
                    }

                    // Adjust position for edges that move origin
                    let width_delta = target_width - new_width;
                    match self.resize_edge {
                        ResizeEdge::Top | ResizeEdge::Bottom => {
                            // Center the width change
                            new_x -= width_delta / 2.0;
                        }
                        _ => {}
                    }
                    new_width = target_width;
                }
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
            self.close();
            return true;
        }

        // Try to dispatch registered keyboard shortcuts first
        // (This handles Ctrl+S, etc. before other handlers can consume them)
        if self.try_dispatch_shortcut(event) {
            return true;
        }

        // Handle Tab and Shift+Tab for focus navigation
        if event.key == Key::Tab && !event.is_repeat {
            if event.modifiers.shift {
                // Shift+Tab: move focus backwards
                self.backtab_pressed.emit(());
            } else {
                // Tab: move focus forwards
                self.tab_pressed.emit(());
            }
            return true;
        }

        // Handle Enter key for default button activation
        // This handles Enter at the window level when no focused widget consumed it
        if event.key == Key::Enter && !event.is_repeat {
            if let Some(button_id) = self.default_button {
                self.default_button_activated.emit(button_id);
                return true;
            }
        }

        // Handle Alt key press - show mnemonic underlines
        if matches!(event.key, Key::AltLeft | Key::AltRight) {
            if !self.alt_held {
                self.alt_held = true;
                // Trigger repaint of window to show mnemonic underlines
                self.base.update();
            }
            return false; // Don't consume the Alt key event
        }

        // Handle Alt+key mnemonic activation
        if event.modifiers.alt {
            if let Some(key_char) = event.key.to_ascii_char() {
                // Emit signal for mnemonic dispatch
                self.mnemonic_key_pressed.emit(key_char);
                return true; // Consume the Alt+key event
            }
        }

        false
    }

    fn handle_key_release(&mut self, event: &KeyReleaseEvent) -> bool {
        // Handle Alt key release - hide mnemonic underlines
        if matches!(event.key, Key::AltLeft | Key::AltRight) {
            // Only hide if no Alt keys remain pressed
            // (Check modifiers to see if Alt is still held via the other Alt key)
            if !event.modifiers.alt {
                if self.alt_held {
                    self.alt_held = false;
                    self.reset_mnemonic_cycle();
                    // Trigger repaint of window to hide mnemonic underlines
                    self.base.update();
                }
            }
            return false; // Don't consume the Alt key event
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
            WidgetEvent::KeyRelease(e) => self.handle_key_release(e),
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

    // =========================================================================
    // Default Button Tests
    // =========================================================================

    #[test]
    fn test_default_button_none_by_default() {
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let window = Window::new("Test Window");
        assert!(window.default_button().is_none());
    }

    #[test]
    fn test_set_default_button() {
        use crate::widget::widgets::PushButton;
        use horizon_lattice_core::{init_global_registry, Object};
        init_global_registry();

        let mut window = Window::new("Test Window");
        let button = PushButton::new("OK");
        let button_id = button.object_id();

        window.set_default_button(Some(button_id));
        assert_eq!(window.default_button(), Some(button_id));

        window.set_default_button(None);
        assert!(window.default_button().is_none());
    }

    #[test]
    fn test_default_button_builder() {
        use crate::widget::widgets::PushButton;
        use horizon_lattice_core::{init_global_registry, Object};
        init_global_registry();

        let button = PushButton::new("OK");
        let button_id = button.object_id();
        let window = Window::new("Test Window").with_default_button(button_id);
        assert_eq!(window.default_button(), Some(button_id));
    }

    // =========================================================================
    // Focus Management Tests
    // =========================================================================

    #[test]
    fn test_window_has_focus_manager() {
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let window = Window::new("Test Window");
        // Focus manager should exist and have no focused widget initially
        assert!(window.focus_manager().focused_widget().is_none());
        assert!(window.focused_widget().is_none());
    }

    #[test]
    fn test_window_focus_manager_accessor() {
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let mut window = Window::new("Test Window");

        // Can access focus manager mutably
        let fm = window.focus_manager_mut();
        assert!(fm.focused_widget().is_none());
    }

    #[test]
    fn test_window_focus_signals_exist() {
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let window = Window::new("Test Window");

        // Verify focus-related signals exist (they do if this compiles)
        let _ = &window.focus_changed;
        let _ = &window.tab_pressed;
        let _ = &window.backtab_pressed;
    }

    // =========================================================================
    // Close Event Tests
    // =========================================================================

    #[test]
    fn test_close_event_accepted_by_default() {
        let event = CloseEvent::new();
        assert!(event.is_accepted());
    }

    #[test]
    fn test_close_event_can_be_ignored() {
        let mut event = CloseEvent::new();
        assert!(event.is_accepted());

        event.ignore();
        assert!(!event.is_accepted());

        event.accept();
        assert!(event.is_accepted());
    }

    #[test]
    fn test_window_close_succeeds_without_handler() {
        use crate::widget::Widget;
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let mut window = Window::new("Test Window");
        window.show();
        assert!(window.widget_base().is_visible());

        let closed = window.close();
        assert!(closed);
        assert!(!window.widget_base().is_visible());
    }

    #[test]
    fn test_window_close_can_be_vetoed() {
        use crate::widget::Widget;
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let mut window = Window::new("Test Window");
        window.show();

        // Set a handler that vetoes the close
        window.set_close_handler(|event| {
            event.ignore();
        });

        let closed = window.close();
        assert!(!closed);
        assert!(window.widget_base().is_visible()); // Window should still be visible
    }

    #[test]
    fn test_window_force_close_bypasses_handler() {
        use crate::widget::Widget;
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let mut window = Window::new("Test Window");
        window.show();

        // Set a handler that vetoes the close
        window.set_close_handler(|event| {
            event.ignore();
        });

        // force_close should bypass the handler
        window.force_close();
        assert!(!window.widget_base().is_visible());
    }

    #[test]
    fn test_window_clear_close_handler() {
        use crate::widget::Widget;
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let mut window = Window::new("Test Window");
        window.show();

        // Set a handler that vetoes the close
        window.set_close_handler(|event| {
            event.ignore();
        });

        // Verify it vetoes
        assert!(!window.close());
        window.show(); // Re-show for next test

        // Clear the handler
        window.clear_close_handler();

        // Now close should succeed
        assert!(window.close());
        assert!(!window.widget_base().is_visible());
    }

    #[test]
    fn test_close_requested_signal_emitted_on_accepted_close() {
        use horizon_lattice_core::init_global_registry;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        init_global_registry();

        let mut window = Window::new("Test Window");
        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_received_clone = signal_received.clone();

        window.close_requested.connect(move |()| {
            signal_received_clone.store(true, Ordering::SeqCst);
        });

        window.close();
        assert!(signal_received.load(Ordering::SeqCst));
    }

    #[test]
    fn test_close_requested_signal_not_emitted_on_vetoed_close() {
        use horizon_lattice_core::init_global_registry;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        init_global_registry();

        let mut window = Window::new("Test Window");
        let signal_received = Arc::new(AtomicBool::new(false));
        let signal_received_clone = signal_received.clone();

        window.close_requested.connect(move |()| {
            signal_received_clone.store(true, Ordering::SeqCst);
        });

        window.set_close_handler(|event| {
            event.ignore();
        });

        window.close();
        assert!(!signal_received.load(Ordering::SeqCst));
    }

    // =========================================================================
    // Aspect Ratio Tests
    // =========================================================================

    #[test]
    fn test_window_aspect_ratio_default() {
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let window = Window::new("Test Window");
        assert!(window.aspect_ratio().is_none());
    }

    #[test]
    fn test_window_aspect_ratio_setter() {
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        let mut window = Window::new("Test Window");

        // Set 16:9 aspect ratio
        window.set_aspect_ratio(Some(16.0 / 9.0));
        let ratio = window.aspect_ratio().unwrap();
        assert!((ratio - 1.777).abs() < 0.01);

        // Clear aspect ratio
        window.set_aspect_ratio(None);
        assert!(window.aspect_ratio().is_none());
    }

    #[test]
    fn test_window_aspect_ratio_builder() {
        use horizon_lattice_core::init_global_registry;
        init_global_registry();

        // Build with 4:3 aspect ratio
        let window = Window::new("Test")
            .with_aspect_ratio(4.0 / 3.0);
        let ratio = window.aspect_ratio().unwrap();
        assert!((ratio - 1.333).abs() < 0.01);

        // Build with square aspect ratio
        let window = Window::new("Square")
            .with_aspect_ratio(1.0);
        assert_eq!(window.aspect_ratio(), Some(1.0));
    }
}
