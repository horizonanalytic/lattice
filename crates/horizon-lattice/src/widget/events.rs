//! Widget-specific event types.
//!
//! This module defines events that are specific to the widget system,
//! including paint events, resize events, mouse events, and keyboard events.
//!
//! # Custom Events
//!
//! The event system supports user-defined custom events through `CustomEvent`.
//! Custom events can carry any type of data and are dispatched through the
//! same event system as built-in events.
//!
//! ```ignore
//! use horizon_lattice::widget::{CustomEvent, WidgetEvent};
//!
//! // Define a custom event payload
//! struct MyCustomData {
//!     message: String,
//!     value: i32,
//! }
//!
//! // Create a custom event
//! let event = CustomEvent::new(MyCustomData {
//!     message: "Hello".into(),
//!     value: 42,
//! });
//!
//! // Later, when handling the event:
//! if let WidgetEvent::Custom(custom) = &event {
//!     if let Some(data) = custom.downcast_ref::<MyCustomData>() {
//!         println!("Got message: {}", data.message);
//!     }
//! }
//! ```

use std::any::{Any, TypeId};

use horizon_lattice_core::TimerId;
use horizon_lattice_render::{Point, Rect, Size};

/// Keyboard modifiers that may be held during input events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
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

    /// Shift modifier only.
    pub const SHIFT: Self = Self {
        shift: true,
        control: false,
        alt: false,
        meta: false,
    };

    /// Control modifier only.
    pub const CTRL: Self = Self {
        shift: false,
        control: true,
        alt: false,
        meta: false,
    };

    /// Alt modifier only.
    pub const ALT: Self = Self {
        shift: false,
        control: false,
        alt: true,
        meta: false,
    };

    /// Meta modifier only.
    pub const META: Self = Self {
        shift: false,
        control: false,
        alt: false,
        meta: true,
    };

    /// Control + Shift modifiers.
    pub const CTRL_SHIFT: Self = Self {
        shift: true,
        control: true,
        alt: false,
        meta: false,
    };

    /// Control + Alt modifiers.
    pub const CTRL_ALT: Self = Self {
        shift: false,
        control: true,
        alt: true,
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

/// Mouse double-click event.
#[derive(Debug, Clone, Copy)]
pub struct MouseDoubleClickEvent {
    /// Base event data.
    pub base: EventBase,
    /// The button that was double-clicked.
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

impl MouseDoubleClickEvent {
    /// Create a new mouse double-click event.
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
    /// Focus changed due to keyboard shortcut/mnemonic (Alt+key).
    Shortcut,
    /// Focus changed programmatically.
    #[default]
    Other,
}

/// Keyboard key codes.
///
/// This enum represents the physical/logical keys on a keyboard.
/// It follows a similar structure to web KeyboardEvent.code values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum Key {
    // Letters
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,

    // Numbers (main keyboard)
    Digit0, Digit1, Digit2, Digit3, Digit4,
    Digit5, Digit6, Digit7, Digit8, Digit9,

    // Function keys
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    // Navigation
    ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    Home, End, PageUp, PageDown,

    // Editing
    Backspace, Delete, Insert,
    Enter, Tab,

    // Whitespace
    Space,

    // Modifiers (also tracked via KeyboardModifiers, but useful as key events)
    ShiftLeft, ShiftRight,
    ControlLeft, ControlRight,
    AltLeft, AltRight,
    MetaLeft, MetaRight,

    // Punctuation and symbols
    Minus, Equal,
    BracketLeft, BracketRight, Backslash,
    Semicolon, Quote,
    Comma, Period, Slash,
    Grave,

    // Control
    Escape,
    CapsLock, NumLock, ScrollLock,
    PrintScreen, Pause,

    // Numpad
    Numpad0, Numpad1, Numpad2, Numpad3, Numpad4,
    Numpad5, Numpad6, Numpad7, Numpad8, Numpad9,
    NumpadAdd, NumpadSubtract, NumpadMultiply, NumpadDivide,
    NumpadDecimal, NumpadEnter,

    // Media keys
    MediaPlayPause, MediaStop, MediaNext, MediaPrevious,
    AudioVolumeUp, AudioVolumeDown, AudioVolumeMute,

    // Unknown/unmapped key
    Unknown(u16),
}

impl Key {
    /// Check if this is a modifier key.
    pub fn is_modifier(&self) -> bool {
        matches!(
            self,
            Key::ShiftLeft
                | Key::ShiftRight
                | Key::ControlLeft
                | Key::ControlRight
                | Key::AltLeft
                | Key::AltRight
                | Key::MetaLeft
                | Key::MetaRight
        )
    }

    /// Check if this is a navigation key.
    pub fn is_navigation(&self) -> bool {
        matches!(
            self,
            Key::ArrowUp
                | Key::ArrowDown
                | Key::ArrowLeft
                | Key::ArrowRight
                | Key::Home
                | Key::End
                | Key::PageUp
                | Key::PageDown
        )
    }

    /// Check if this is a function key.
    pub fn is_function_key(&self) -> bool {
        matches!(
            self,
            Key::F1
                | Key::F2
                | Key::F3
                | Key::F4
                | Key::F5
                | Key::F6
                | Key::F7
                | Key::F8
                | Key::F9
                | Key::F10
                | Key::F11
                | Key::F12
        )
    }

    /// Check if this is a letter key.
    pub fn is_letter(&self) -> bool {
        matches!(
            self,
            Key::A
                | Key::B
                | Key::C
                | Key::D
                | Key::E
                | Key::F
                | Key::G
                | Key::H
                | Key::I
                | Key::J
                | Key::K
                | Key::L
                | Key::M
                | Key::N
                | Key::O
                | Key::P
                | Key::Q
                | Key::R
                | Key::S
                | Key::T
                | Key::U
                | Key::V
                | Key::W
                | Key::X
                | Key::Y
                | Key::Z
        )
    }

    /// Check if this is a digit key (main keyboard, not numpad).
    pub fn is_digit(&self) -> bool {
        matches!(
            self,
            Key::Digit0
                | Key::Digit1
                | Key::Digit2
                | Key::Digit3
                | Key::Digit4
                | Key::Digit5
                | Key::Digit6
                | Key::Digit7
                | Key::Digit8
                | Key::Digit9
        )
    }

    /// Convert this key to a lowercase ASCII character, if applicable.
    ///
    /// Returns `Some(char)` for letter keys (A-Z) and digit keys (0-9),
    /// `None` for other keys. Letters are returned in lowercase.
    pub fn to_ascii_char(&self) -> Option<char> {
        match self {
            Key::A => Some('a'),
            Key::B => Some('b'),
            Key::C => Some('c'),
            Key::D => Some('d'),
            Key::E => Some('e'),
            Key::F => Some('f'),
            Key::G => Some('g'),
            Key::H => Some('h'),
            Key::I => Some('i'),
            Key::J => Some('j'),
            Key::K => Some('k'),
            Key::L => Some('l'),
            Key::M => Some('m'),
            Key::N => Some('n'),
            Key::O => Some('o'),
            Key::P => Some('p'),
            Key::Q => Some('q'),
            Key::R => Some('r'),
            Key::S => Some('s'),
            Key::T => Some('t'),
            Key::U => Some('u'),
            Key::V => Some('v'),
            Key::W => Some('w'),
            Key::X => Some('x'),
            Key::Y => Some('y'),
            Key::Z => Some('z'),
            Key::Digit0 => Some('0'),
            Key::Digit1 => Some('1'),
            Key::Digit2 => Some('2'),
            Key::Digit3 => Some('3'),
            Key::Digit4 => Some('4'),
            Key::Digit5 => Some('5'),
            Key::Digit6 => Some('6'),
            Key::Digit7 => Some('7'),
            Key::Digit8 => Some('8'),
            Key::Digit9 => Some('9'),
            _ => None,
        }
    }
}

/// Key press event, sent when a key is pressed.
#[derive(Debug, Clone)]
pub struct KeyPressEvent {
    /// Base event data.
    pub base: EventBase,
    /// The key that was pressed.
    pub key: Key,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
    /// The text input from this key press (if any).
    ///
    /// For printable keys, this contains the character that would be typed.
    /// For non-printable keys (modifiers, function keys, etc.), this is empty.
    pub text: String,
    /// Whether this is a key repeat event (key held down).
    pub is_repeat: bool,
}

impl KeyPressEvent {
    /// Create a new key press event.
    pub fn new(key: Key, modifiers: KeyboardModifiers, text: impl Into<String>, is_repeat: bool) -> Self {
        Self {
            base: EventBase::new(),
            key,
            modifiers,
            text: text.into(),
            is_repeat,
        }
    }
}

/// Key release event, sent when a key is released.
#[derive(Debug, Clone)]
pub struct KeyReleaseEvent {
    /// Base event data.
    pub base: EventBase,
    /// The key that was released.
    pub key: Key,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl KeyReleaseEvent {
    /// Create a new key release event.
    pub fn new(key: Key, modifiers: KeyboardModifiers) -> Self {
        Self {
            base: EventBase::new(),
            key,
            modifiers,
        }
    }
}

/// A custom event that can carry any user-defined payload.
///
/// Custom events allow applications to define their own event types and
/// dispatch them through the widget event system. The payload is stored
/// as a type-erased `Box<dyn Any>`, allowing any `'static` type to be used.
///
/// # Type Safety
///
/// While the payload is type-erased for storage, you can recover the
/// original type using [`downcast_ref`](Self::downcast_ref) or
/// [`downcast_mut`](Self::downcast_mut).
///
/// # Example
///
/// ```ignore
/// // Define your custom event data
/// struct RefreshRequest {
///     source: String,
///     force: bool,
/// }
///
/// // Create the event
/// let event = CustomEvent::new(RefreshRequest {
///     source: "user_action".into(),
///     force: true,
/// });
///
/// // Check the type and extract data
/// if event.is::<RefreshRequest>() {
///     let data = event.downcast_ref::<RefreshRequest>().unwrap();
///     println!("Refresh from: {}", data.source);
/// }
/// ```
pub struct CustomEvent {
    /// Base event data.
    pub base: EventBase,
    /// The type-erased payload.
    payload: Box<dyn Any + Send + Sync>,
    /// Cached TypeId for efficient type checking.
    type_id: TypeId,
    /// Optional event name for debugging/logging.
    name: Option<String>,
}

impl CustomEvent {
    /// Create a new custom event with the given payload.
    ///
    /// The payload can be any type that implements `Send + Sync + 'static`.
    pub fn new<T: Any + Send + Sync>(payload: T) -> Self {
        Self {
            base: EventBase::new(),
            type_id: TypeId::of::<T>(),
            payload: Box::new(payload),
            name: None,
        }
    }

    /// Create a new custom event with a name for debugging.
    ///
    /// The name is purely for debugging/logging purposes and does not
    /// affect event dispatch or handling.
    pub fn with_name<T: Any + Send + Sync>(payload: T, name: impl Into<String>) -> Self {
        Self {
            base: EventBase::new(),
            type_id: TypeId::of::<T>(),
            payload: Box::new(payload),
            name: Some(name.into()),
        }
    }

    /// Get the event name, if one was provided.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the TypeId of the payload.
    pub fn payload_type_id(&self) -> TypeId {
        self.type_id
    }

    /// Check if the payload is of type `T`.
    pub fn is<T: Any>(&self) -> bool {
        self.type_id == TypeId::of::<T>()
    }

    /// Try to get a reference to the payload as type `T`.
    ///
    /// Returns `Some(&T)` if the payload is of type `T`, otherwise `None`.
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        self.payload.downcast_ref::<T>()
    }

    /// Try to get a mutable reference to the payload as type `T`.
    ///
    /// Returns `Some(&mut T)` if the payload is of type `T`, otherwise `None`.
    pub fn downcast_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.payload.downcast_mut::<T>()
    }

    /// Consume the event and try to extract the payload as type `T`.
    ///
    /// Returns `Ok(T)` if the payload is of type `T`, otherwise returns
    /// `Err(self)` with the original event.
    pub fn downcast<T: Any + Send + Sync>(self) -> Result<T, Self> {
        if self.is::<T>() {
            // Safe because we just checked the type
            let payload = self.payload.downcast::<T>().ok().map(|b| *b);
            match payload {
                Some(value) => Ok(value),
                None => Err(Self {
                    base: self.base,
                    type_id: self.type_id,
                    payload: Box::new(()),
                    name: self.name,
                }),
            }
        } else {
            Err(self)
        }
    }
}

impl std::fmt::Debug for CustomEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomEvent")
            .field("base", &self.base)
            .field("type_id", &self.type_id)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// Timer event, sent when a widget-owned timer fires.
///
/// Widgets can start timers using the widget timer API. When a timer fires,
/// the owning widget receives a `TimerEvent` through the normal event dispatch.
///
/// # Example
///
/// ```ignore
/// fn event(&mut self, event: &mut WidgetEvent) -> bool {
///     match event {
///         WidgetEvent::Timer(e) => {
///             if e.id == self.repeat_timer_id {
///                 self.perform_repeat_action();
///                 event.accept();
///                 return true;
///             }
///         }
///         _ => {}
///     }
///     false
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct TimerEvent {
    /// Base event data.
    pub base: EventBase,
    /// The ID of the timer that fired.
    pub id: TimerId,
}

impl TimerEvent {
    /// Create a new timer event.
    pub fn new(id: TimerId) -> Self {
        Self {
            base: EventBase::new(),
            id,
        }
    }
}

/// Context menu request event.
///
/// This event is sent when a context menu is requested for a widget.
/// The request can come from:
/// - Right mouse button click
/// - Keyboard Menu key
/// - Programmatic request
///
/// # Example
///
/// ```ignore
/// fn event(&mut self, event: &mut WidgetEvent) -> bool {
///     match event {
///         WidgetEvent::ContextMenu(e) => {
///             // Show a context menu at the requested position
///             let mut menu = Menu::new();
///             menu.add_action(Arc::new(Action::new("Cut")));
///             menu.add_action(Arc::new(Action::new("Copy")));
///             menu.add_action(Arc::new(Action::new("Paste")));
///             menu.popup_at(e.global_pos.x, e.global_pos.y);
///             event.accept();
///             true
///         }
///         _ => false,
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ContextMenuEvent {
    /// Base event data.
    pub base: EventBase,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Position in global screen coordinates.
    pub global_pos: Point,
    /// The reason the context menu was requested.
    pub reason: ContextMenuReason,
}

/// Reason a context menu was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ContextMenuReason {
    /// Context menu was triggered by mouse (typically right-click).
    #[default]
    Mouse,
    /// Context menu was triggered by keyboard (Menu key).
    Keyboard,
    /// Context menu was triggered programmatically.
    Other,
}

impl ContextMenuEvent {
    /// Create a new context menu event from a mouse position.
    pub fn from_mouse(local_pos: Point, window_pos: Point, global_pos: Point) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            global_pos,
            reason: ContextMenuReason::Mouse,
        }
    }

    /// Create a new context menu event from a keyboard request.
    ///
    /// The position is typically the center of the widget or the current
    /// selection position.
    pub fn from_keyboard(local_pos: Point, window_pos: Point, global_pos: Point) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            global_pos,
            reason: ContextMenuReason::Keyboard,
        }
    }

    /// Create a new context menu event with a custom reason.
    pub fn new(
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        reason: ContextMenuReason,
    ) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            global_pos,
            reason,
        }
    }
}

/// Enumeration of all widget event types.
///
/// This allows passing events through a unified interface while preserving
/// type information for event handlers.
///
/// # Custom Events
///
/// User-defined events can be dispatched using the [`Custom`](Self::Custom) variant.
/// See [`CustomEvent`] for details on creating and handling custom events.
#[derive(Debug)]
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
    /// Mouse double-click event.
    DoubleClick(MouseDoubleClickEvent),
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
    /// Key press event.
    KeyPress(KeyPressEvent),
    /// Key release event.
    KeyRelease(KeyReleaseEvent),
    /// User-defined custom event.
    ///
    /// Custom events can carry any payload and are dispatched through the
    /// event system like built-in events. Use [`CustomEvent::downcast_ref`]
    /// to extract the payload in your event handler.
    Custom(CustomEvent),
    /// Timer event.
    ///
    /// Sent when a widget-owned timer fires. Widgets can start timers
    /// using the widget timer API and receive events when they fire.
    Timer(TimerEvent),
    /// Context menu event.
    ///
    /// Sent when a context menu is requested for the widget.
    /// This occurs on right-click or Menu key press when the widget's
    /// context menu policy allows it.
    ContextMenu(ContextMenuEvent),
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
            Self::DoubleClick(e) => e.base.is_accepted(),
            Self::MouseRelease(e) => e.base.is_accepted(),
            Self::MouseMove(e) => e.base.is_accepted(),
            Self::Wheel(e) => e.base.is_accepted(),
            Self::Enter(e) => e.base.is_accepted(),
            Self::Leave(e) => e.base.is_accepted(),
            Self::FocusIn(e) => e.base.is_accepted(),
            Self::FocusOut(e) => e.base.is_accepted(),
            Self::KeyPress(e) => e.base.is_accepted(),
            Self::KeyRelease(e) => e.base.is_accepted(),
            Self::Custom(e) => e.base.is_accepted(),
            Self::Timer(e) => e.base.is_accepted(),
            Self::ContextMenu(e) => e.base.is_accepted(),
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
            Self::DoubleClick(e) => e.base.accept(),
            Self::MouseRelease(e) => e.base.accept(),
            Self::MouseMove(e) => e.base.accept(),
            Self::Wheel(e) => e.base.accept(),
            Self::Enter(e) => e.base.accept(),
            Self::Leave(e) => e.base.accept(),
            Self::FocusIn(e) => e.base.accept(),
            Self::FocusOut(e) => e.base.accept(),
            Self::KeyPress(e) => e.base.accept(),
            Self::KeyRelease(e) => e.base.accept(),
            Self::Custom(e) => e.base.accept(),
            Self::Timer(e) => e.base.accept(),
            Self::ContextMenu(e) => e.base.accept(),
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
            Self::DoubleClick(e) => e.base.ignore(),
            Self::MouseRelease(e) => e.base.ignore(),
            Self::MouseMove(e) => e.base.ignore(),
            Self::Wheel(e) => e.base.ignore(),
            Self::Enter(e) => e.base.ignore(),
            Self::Leave(e) => e.base.ignore(),
            Self::FocusIn(e) => e.base.ignore(),
            Self::FocusOut(e) => e.base.ignore(),
            Self::KeyPress(e) => e.base.ignore(),
            Self::KeyRelease(e) => e.base.ignore(),
            Self::Custom(e) => e.base.ignore(),
            Self::Timer(e) => e.base.ignore(),
            Self::ContextMenu(e) => e.base.ignore(),
        }
    }

    /// Check if this event should propagate to parent widgets.
    ///
    /// Some events (like paint, resize, show, hide) are specific to a widget
    /// and should not propagate. Input events typically propagate if not accepted.
    pub fn should_propagate(&self) -> bool {
        match self {
            // These events are widget-specific and don't propagate
            Self::Paint(_) | Self::Resize(_) | Self::Move(_) | Self::Show(_) | Self::Hide(_) => {
                false
            }
            // Input events propagate if not accepted
            Self::MousePress(_)
            | Self::DoubleClick(_)
            | Self::MouseRelease(_)
            | Self::MouseMove(_)
            | Self::Wheel(_)
            | Self::KeyPress(_)
            | Self::KeyRelease(_) => !self.is_accepted(),
            // Focus events don't propagate
            Self::FocusIn(_) | Self::FocusOut(_) => false,
            // Enter/Leave are about the specific widget and don't propagate
            Self::Enter(_) | Self::Leave(_) => false,
            // Custom events propagate by default (if not accepted)
            Self::Custom(_) => !self.is_accepted(),
            // Timer events are specific to the widget that owns the timer
            Self::Timer(_) => false,
            // Context menu events propagate if not accepted
            Self::ContextMenu(_) => !self.is_accepted(),
        }
    }

    /// Try to get a reference to the inner [`CustomEvent`] if this is a custom event.
    ///
    /// Returns `Some(&CustomEvent)` if this is a `WidgetEvent::Custom`, otherwise `None`.
    pub fn as_custom(&self) -> Option<&CustomEvent> {
        match self {
            Self::Custom(e) => Some(e),
            _ => None,
        }
    }

    /// Try to get a mutable reference to the inner [`CustomEvent`] if this is a custom event.
    ///
    /// Returns `Some(&mut CustomEvent)` if this is a `WidgetEvent::Custom`, otherwise `None`.
    pub fn as_custom_mut(&mut self) -> Option<&mut CustomEvent> {
        match self {
            Self::Custom(e) => Some(e),
            _ => None,
        }
    }
}
