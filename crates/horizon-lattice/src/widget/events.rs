//! Widget-specific event types.
//!
//! This module defines events that are specific to the widget system,
//! including paint events, resize events, mouse events, and keyboard events.

use horizon_lattice_render::{Point, Rect, Size};

/// Keyboard modifiers that may be held during input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyboardModifiers {
    /// The Shift key is held.
    pub shift: bool,
    /// The Control key is held (Cmd on macOS).
    pub control: bool,
    /// The Alt key is held (Option on macOS).
    pub alt: bool,
    /// The Meta/Super key is held (Windows key, Cmd on macOS).
    pub meta: bool,
}

impl KeyboardModifiers {
    /// No modifiers pressed.
    pub const NONE: Self = Self {
        shift: false,
        control: false,
        alt: false,
        meta: false,
    };

    /// Check if any modifier is pressed.
    pub fn any(&self) -> bool {
        self.shift || self.control || self.alt || self.meta
    }

    /// Check if no modifiers are pressed.
    pub fn none(&self) -> bool {
        !self.any()
    }
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum MouseButton {
    /// Primary button (usually left).
    Left = 0,
    /// Secondary button (usually right).
    Right = 1,
    /// Middle button (scroll wheel click).
    Middle = 2,
    /// Additional button 1 (e.g., browser back).
    Button4 = 3,
    /// Additional button 2 (e.g., browser forward).
    Button5 = 4,
}

/// Common data for all widget events.
#[derive(Debug, Clone, Copy)]
pub struct EventBase {
    /// Whether the event has been accepted (handled).
    accepted: bool,
}

impl Default for EventBase {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBase {
    /// Create a new event base.
    pub fn new() -> Self {
        Self { accepted: false }
    }

    /// Check if the event has been accepted.
    pub fn is_accepted(&self) -> bool {
        self.accepted
    }

    /// Accept the event, preventing further propagation.
    pub fn accept(&mut self) {
        self.accepted = true;
    }

    /// Ignore the event, allowing further propagation.
    pub fn ignore(&mut self) {
        self.accepted = false;
    }
}

/// Paint event, sent when a widget needs to be repainted.
#[derive(Debug, Clone)]
pub struct PaintEvent {
    /// Base event data.
    pub base: EventBase,
    /// The region that needs to be repainted (in widget-local coordinates).
    pub rect: Rect,
}

impl PaintEvent {
    /// Create a new paint event for the given region.
    pub fn new(rect: Rect) -> Self {
        Self {
            base: EventBase::new(),
            rect,
        }
    }

    /// Create a paint event for the entire widget area.
    pub fn full(size: Size) -> Self {
        Self::new(Rect::new(0.0, 0.0, size.width, size.height))
    }
}

/// Resize event, sent when a widget's size changes.
#[derive(Debug, Clone, Copy)]
pub struct ResizeEvent {
    /// Base event data.
    pub base: EventBase,
    /// The old size of the widget.
    pub old_size: Size,
    /// The new size of the widget.
    pub new_size: Size,
}

impl ResizeEvent {
    /// Create a new resize event.
    pub fn new(old_size: Size, new_size: Size) -> Self {
        Self {
            base: EventBase::new(),
            old_size,
            new_size,
        }
    }
}

/// Move event, sent when a widget's position changes.
#[derive(Debug, Clone, Copy)]
pub struct MoveEvent {
    /// Base event data.
    pub base: EventBase,
    /// The old position of the widget (relative to parent).
    pub old_pos: Point,
    /// The new position of the widget (relative to parent).
    pub new_pos: Point,
}

impl MoveEvent {
    /// Create a new move event.
    pub fn new(old_pos: Point, new_pos: Point) -> Self {
        Self {
            base: EventBase::new(),
            old_pos,
            new_pos,
        }
    }
}

/// Show event, sent when a widget becomes visible.
#[derive(Debug, Clone, Copy)]
pub struct ShowEvent {
    /// Base event data.
    pub base: EventBase,
}

impl ShowEvent {
    /// Create a new show event.
    pub fn new() -> Self {
        Self {
            base: EventBase::new(),
        }
    }
}

impl Default for ShowEvent {
    fn default() -> Self {
        Self::new()
    }
}

/// Hide event, sent when a widget becomes hidden.
#[derive(Debug, Clone, Copy)]
pub struct HideEvent {
    /// Base event data.
    pub base: EventBase,
}

impl HideEvent {
    /// Create a new hide event.
    pub fn new() -> Self {
        Self {
            base: EventBase::new(),
        }
    }
}

impl Default for HideEvent {
    fn default() -> Self {
        Self::new()
    }
}

/// Mouse press event.
#[derive(Debug, Clone, Copy)]
pub struct MousePressEvent {
    /// Base event data.
    pub base: EventBase,
    /// The button that was pressed.
    pub button: MouseButton,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Position in global screen coordinates.
    pub global_pos: Point,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl MousePressEvent {
    /// Create a new mouse press event.
    pub fn new(
        button: MouseButton,
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            button,
            local_pos,
            window_pos,
            global_pos,
            modifiers,
        }
    }
}

/// Mouse release event.
#[derive(Debug, Clone, Copy)]
pub struct MouseReleaseEvent {
    /// Base event data.
    pub base: EventBase,
    /// The button that was released.
    pub button: MouseButton,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Position in global screen coordinates.
    pub global_pos: Point,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl MouseReleaseEvent {
    /// Create a new mouse release event.
    pub fn new(
        button: MouseButton,
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            button,
            local_pos,
            window_pos,
            global_pos,
            modifiers,
        }
    }
}

/// Mouse move event.
#[derive(Debug, Clone, Copy)]
pub struct MouseMoveEvent {
    /// Base event data.
    pub base: EventBase,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Position in global screen coordinates.
    pub global_pos: Point,
    /// Mouse buttons currently held.
    pub buttons: u8,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl MouseMoveEvent {
    /// Create a new mouse move event.
    pub fn new(
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        buttons: u8,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            global_pos,
            buttons,
            modifiers,
        }
    }

    /// Check if a specific button is pressed.
    pub fn is_button_pressed(&self, button: MouseButton) -> bool {
        (self.buttons & (1 << button as u8)) != 0
    }
}

/// Mouse wheel (scroll) event.
#[derive(Debug, Clone, Copy)]
pub struct WheelEvent {
    /// Base event data.
    pub base: EventBase,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Horizontal scroll delta (positive = right).
    pub delta_x: f32,
    /// Vertical scroll delta (positive = up/away from user).
    pub delta_y: f32,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl WheelEvent {
    /// Create a new wheel event.
    pub fn new(
        local_pos: Point,
        window_pos: Point,
        delta_x: f32,
        delta_y: f32,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            delta_x,
            delta_y,
            modifiers,
        }
    }
}

/// Enter event, sent when the mouse enters the widget area.
#[derive(Debug, Clone, Copy)]
pub struct EnterEvent {
    /// Base event data.
    pub base: EventBase,
    /// The position where the mouse entered.
    pub local_pos: Point,
}

impl EnterEvent {
    /// Create a new enter event.
    pub fn new(local_pos: Point) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
        }
    }
}

/// Leave event, sent when the mouse leaves the widget area.
#[derive(Debug, Clone, Copy)]
pub struct LeaveEvent {
    /// Base event data.
    pub base: EventBase,
}

impl LeaveEvent {
    /// Create a new leave event.
    pub fn new() -> Self {
        Self {
            base: EventBase::new(),
        }
    }
}

impl Default for LeaveEvent {
    fn default() -> Self {
        Self::new()
    }
}

/// Focus in event, sent when the widget gains keyboard focus.
#[derive(Debug, Clone, Copy)]
pub struct FocusInEvent {
    /// Base event data.
    pub base: EventBase,
    /// The reason focus was gained.
    pub reason: FocusReason,
}

impl FocusInEvent {
    /// Create a new focus in event.
    pub fn new(reason: FocusReason) -> Self {
        Self {
            base: EventBase::new(),
            reason,
        }
    }
}

/// Focus out event, sent when the widget loses keyboard focus.
#[derive(Debug, Clone, Copy)]
pub struct FocusOutEvent {
    /// Base event data.
    pub base: EventBase,
    /// The reason focus was lost.
    pub reason: FocusReason,
}

impl FocusOutEvent {
    /// Create a new focus out event.
    pub fn new(reason: FocusReason) -> Self {
        Self {
            base: EventBase::new(),
            reason,
        }
    }
}

/// Reason for focus change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusReason {
    /// Focus changed due to mouse click.
    Mouse,
    /// Focus changed due to Tab key.
    Tab,
    /// Focus changed due to Shift+Tab (backtab).
    Backtab,
    /// Focus changed programmatically.
    #[default]
    Other,
}

/// Enumeration of all widget event types.
///
/// This allows passing events through a unified interface while preserving
/// type information for event handlers.
#[derive(Debug, Clone)]
pub enum WidgetEvent {
    /// Paint event.
    Paint(PaintEvent),
    /// Resize event.
    Resize(ResizeEvent),
    /// Move event.
    Move(MoveEvent),
    /// Show event.
    Show(ShowEvent),
    /// Hide event.
    Hide(HideEvent),
    /// Mouse press event.
    MousePress(MousePressEvent),
    /// Mouse release event.
    MouseRelease(MouseReleaseEvent),
    /// Mouse move event.
    MouseMove(MouseMoveEvent),
    /// Mouse wheel event.
    Wheel(WheelEvent),
    /// Mouse enter event.
    Enter(EnterEvent),
    /// Mouse leave event.
    Leave(LeaveEvent),
    /// Focus in event.
    FocusIn(FocusInEvent),
    /// Focus out event.
    FocusOut(FocusOutEvent),
}

impl WidgetEvent {
    /// Check if the event has been accepted.
    pub fn is_accepted(&self) -> bool {
        match self {
            Self::Paint(e) => e.base.is_accepted(),
            Self::Resize(e) => e.base.is_accepted(),
            Self::Move(e) => e.base.is_accepted(),
            Self::Show(e) => e.base.is_accepted(),
            Self::Hide(e) => e.base.is_accepted(),
            Self::MousePress(e) => e.base.is_accepted(),
            Self::MouseRelease(e) => e.base.is_accepted(),
            Self::MouseMove(e) => e.base.is_accepted(),
            Self::Wheel(e) => e.base.is_accepted(),
            Self::Enter(e) => e.base.is_accepted(),
            Self::Leave(e) => e.base.is_accepted(),
            Self::FocusIn(e) => e.base.is_accepted(),
            Self::FocusOut(e) => e.base.is_accepted(),
        }
    }

    /// Accept the event.
    pub fn accept(&mut self) {
        match self {
            Self::Paint(e) => e.base.accept(),
            Self::Resize(e) => e.base.accept(),
            Self::Move(e) => e.base.accept(),
            Self::Show(e) => e.base.accept(),
            Self::Hide(e) => e.base.accept(),
            Self::MousePress(e) => e.base.accept(),
            Self::MouseRelease(e) => e.base.accept(),
            Self::MouseMove(e) => e.base.accept(),
            Self::Wheel(e) => e.base.accept(),
            Self::Enter(e) => e.base.accept(),
            Self::Leave(e) => e.base.accept(),
            Self::FocusIn(e) => e.base.accept(),
            Self::FocusOut(e) => e.base.accept(),
        }
    }

    /// Ignore the event.
    pub fn ignore(&mut self) {
        match self {
            Self::Paint(e) => e.base.ignore(),
            Self::Resize(e) => e.base.ignore(),
            Self::Move(e) => e.base.ignore(),
            Self::Show(e) => e.base.ignore(),
            Self::Hide(e) => e.base.ignore(),
            Self::MousePress(e) => e.base.ignore(),
            Self::MouseRelease(e) => e.base.ignore(),
            Self::MouseMove(e) => e.base.ignore(),
            Self::Wheel(e) => e.base.ignore(),
            Self::Enter(e) => e.base.ignore(),
            Self::Leave(e) => e.base.ignore(),
            Self::FocusIn(e) => e.base.ignore(),
            Self::FocusOut(e) => e.base.ignore(),
        }
    }
}
