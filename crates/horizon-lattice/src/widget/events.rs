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
    /// Letter A key.
    A,
    /// Letter B key.
    B,
    /// Letter C key.
    C,
    /// Letter D key.
    D,
    /// Letter E key.
    E,
    /// Letter F key.
    F,
    /// Letter G key.
    G,
    /// Letter H key.
    H,
    /// Letter I key.
    I,
    /// Letter J key.
    J,
    /// Letter K key.
    K,
    /// Letter L key.
    L,
    /// Letter M key.
    M,
    /// Letter N key.
    N,
    /// Letter O key.
    O,
    /// Letter P key.
    P,
    /// Letter Q key.
    Q,
    /// Letter R key.
    R,
    /// Letter S key.
    S,
    /// Letter T key.
    T,
    /// Letter U key.
    U,
    /// Letter V key.
    V,
    /// Letter W key.
    W,
    /// Letter X key.
    X,
    /// Letter Y key.
    Y,
    /// Letter Z key.
    Z,

    /// Digit 0 key (main keyboard).
    Digit0,
    /// Digit 1 key (main keyboard).
    Digit1,
    /// Digit 2 key (main keyboard).
    Digit2,
    /// Digit 3 key (main keyboard).
    Digit3,
    /// Digit 4 key (main keyboard).
    Digit4,
    /// Digit 5 key (main keyboard).
    Digit5,
    /// Digit 6 key (main keyboard).
    Digit6,
    /// Digit 7 key (main keyboard).
    Digit7,
    /// Digit 8 key (main keyboard).
    Digit8,
    /// Digit 9 key (main keyboard).
    Digit9,

    /// Function key F1.
    F1,
    /// Function key F2.
    F2,
    /// Function key F3.
    F3,
    /// Function key F4.
    F4,
    /// Function key F5.
    F5,
    /// Function key F6.
    F6,
    /// Function key F7.
    F7,
    /// Function key F8.
    F8,
    /// Function key F9.
    F9,
    /// Function key F10.
    F10,
    /// Function key F11.
    F11,
    /// Function key F12.
    F12,

    /// Arrow up navigation key.
    ArrowUp,
    /// Arrow down navigation key.
    ArrowDown,
    /// Arrow left navigation key.
    ArrowLeft,
    /// Arrow right navigation key.
    ArrowRight,
    /// Home navigation key.
    Home,
    /// End navigation key.
    End,
    /// Page up navigation key.
    PageUp,
    /// Page down navigation key.
    PageDown,

    /// Backspace editing key.
    Backspace,
    /// Delete editing key.
    Delete,
    /// Insert editing key.
    Insert,
    /// Enter/Return key.
    Enter,
    /// Tab key.
    Tab,

    /// Space bar key.
    Space,

    /// Left Shift modifier key.
    ShiftLeft,
    /// Right Shift modifier key.
    ShiftRight,
    /// Left Control modifier key.
    ControlLeft,
    /// Right Control modifier key.
    ControlRight,
    /// Left Alt/Option modifier key.
    AltLeft,
    /// Right Alt/Option modifier key.
    AltRight,
    /// Left Meta/Command/Windows modifier key.
    MetaLeft,
    /// Right Meta/Command/Windows modifier key.
    MetaRight,

    /// Minus/hyphen key.
    Minus,
    /// Equal/plus key.
    Equal,
    /// Left bracket key.
    BracketLeft,
    /// Right bracket key.
    BracketRight,
    /// Backslash key.
    Backslash,
    /// Semicolon key.
    Semicolon,
    /// Quote/apostrophe key.
    Quote,
    /// Comma key.
    Comma,
    /// Period/dot key.
    Period,
    /// Forward slash key.
    Slash,
    /// Grave accent/backtick key.
    Grave,

    /// Escape key.
    Escape,
    /// Caps Lock toggle key.
    CapsLock,
    /// Num Lock toggle key.
    NumLock,
    /// Scroll Lock toggle key.
    ScrollLock,
    /// Print Screen key.
    PrintScreen,
    /// Pause/Break key.
    Pause,

    /// Numpad 0 key.
    Numpad0,
    /// Numpad 1 key.
    Numpad1,
    /// Numpad 2 key.
    Numpad2,
    /// Numpad 3 key.
    Numpad3,
    /// Numpad 4 key.
    Numpad4,
    /// Numpad 5 key.
    Numpad5,
    /// Numpad 6 key.
    Numpad6,
    /// Numpad 7 key.
    Numpad7,
    /// Numpad 8 key.
    Numpad8,
    /// Numpad 9 key.
    Numpad9,
    /// Numpad add/plus key.
    NumpadAdd,
    /// Numpad subtract/minus key.
    NumpadSubtract,
    /// Numpad multiply key.
    NumpadMultiply,
    /// Numpad divide key.
    NumpadDivide,
    /// Numpad decimal/period key.
    NumpadDecimal,
    /// Numpad enter key.
    NumpadEnter,

    /// Media play/pause key.
    MediaPlayPause,
    /// Media stop key.
    MediaStop,
    /// Media next track key.
    MediaNext,
    /// Media previous track key.
    MediaPrevious,
    /// Volume up key.
    AudioVolumeUp,
    /// Volume down key.
    AudioVolumeDown,
    /// Volume mute toggle key.
    AudioVolumeMute,

    /// Unknown or unmapped key with raw code.
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
    /// The hardware scan code, if available.
    ///
    /// Scan codes represent the physical key position on the keyboard,
    /// independent of the keyboard layout. This is useful for games
    /// and applications that need consistent key positions.
    pub scan_code: Option<u32>,
}

impl KeyPressEvent {
    /// Create a new key press event.
    pub fn new(
        key: Key,
        modifiers: KeyboardModifiers,
        text: impl Into<String>,
        is_repeat: bool,
    ) -> Self {
        Self {
            base: EventBase::new(),
            key,
            modifiers,
            text: text.into(),
            is_repeat,
            scan_code: None,
        }
    }

    /// Create a new key press event with a scan code.
    pub fn new_with_scan_code(
        key: Key,
        modifiers: KeyboardModifiers,
        text: impl Into<String>,
        is_repeat: bool,
        scan_code: Option<u32>,
    ) -> Self {
        Self {
            base: EventBase::new(),
            key,
            modifiers,
            text: text.into(),
            is_repeat,
            scan_code,
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
    /// The hardware scan code, if available.
    ///
    /// Scan codes represent the physical key position on the keyboard,
    /// independent of the keyboard layout.
    pub scan_code: Option<u32>,
}

impl KeyReleaseEvent {
    /// Create a new key release event.
    pub fn new(key: Key, modifiers: KeyboardModifiers) -> Self {
        Self {
            base: EventBase::new(),
            key,
            modifiers,
            scan_code: None,
        }
    }

    /// Create a new key release event with a scan code.
    pub fn new_with_scan_code(
        key: Key,
        modifiers: KeyboardModifiers,
        scan_code: Option<u32>,
    ) -> Self {
        Self {
            base: EventBase::new(),
            key,
            modifiers,
            scan_code,
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

// =============================================================================
// IME (Input Method Editor) Events
// =============================================================================

/// IME enabled event.
///
/// This event is sent when the Input Method Editor is enabled for the widget.
/// After receiving this event, you can expect `ImePreedit` and `ImeCommit` events.
///
/// # Example
///
/// ```ignore
/// fn event(&mut self, event: &mut WidgetEvent) -> bool {
///     match event {
///         WidgetEvent::ImeEnabled(e) => {
///             // IME is now active - prepare for composition input
///             self.ime_active = true;
///             event.accept();
///             true
///         }
///         _ => false,
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ImeEnabledEvent {
    /// Base event data.
    pub base: EventBase,
}

impl ImeEnabledEvent {
    /// Create a new IME enabled event.
    pub fn new() -> Self {
        Self {
            base: EventBase::new(),
        }
    }
}

impl Default for ImeEnabledEvent {
    fn default() -> Self {
        Self::new()
    }
}

/// IME preedit (composition) event.
///
/// This event is sent when the user is composing text through the IME.
/// The preedit text should be displayed at the cursor position with
/// visual distinction (typically an underline).
///
/// # Preedit Text
///
/// The preedit text is the text currently being composed. It should be
/// displayed differently from committed text (e.g., with an underline)
/// to indicate that it's not yet finalized.
///
/// An empty preedit text indicates that the composition was cancelled
/// or cleared.
///
/// # Cursor Position
///
/// The cursor position is an optional byte-indexed range within the
/// preedit text. If provided, it indicates where the cursor should be
/// displayed within the composition. If `None`, no cursor should be shown.
///
/// # Example
///
/// ```ignore
/// fn event(&mut self, event: &mut WidgetEvent) -> bool {
///     match event {
///         WidgetEvent::ImePreedit(e) => {
///             if e.text.is_empty() {
///                 // Composition cleared
///                 self.preedit_text = None;
///             } else {
///                 // Update displayed composition text
///                 self.preedit_text = Some(e.text.clone());
///                 self.preedit_cursor = e.cursor;
///             }
///             self.request_repaint();
///             event.accept();
///             true
///         }
///         _ => false,
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ImePreeditEvent {
    /// Base event data.
    pub base: EventBase,
    /// The preedit (composition) text.
    ///
    /// An empty string indicates the preedit was cleared.
    pub text: String,
    /// Cursor position within the preedit text as byte indices.
    ///
    /// The tuple represents (start, end) of the cursor/selection.
    /// If `None`, no cursor should be displayed.
    pub cursor: Option<(usize, usize)>,
}

impl ImePreeditEvent {
    /// Create a new preedit event.
    pub fn new(text: impl Into<String>, cursor: Option<(usize, usize)>) -> Self {
        Self {
            base: EventBase::new(),
            text: text.into(),
            cursor,
        }
    }

    /// Create a preedit event indicating the composition was cleared.
    pub fn cleared() -> Self {
        Self::new("", None)
    }

    /// Check if this event indicates the preedit was cleared.
    pub fn is_cleared(&self) -> bool {
        self.text.is_empty()
    }
}

/// IME commit event.
///
/// This event is sent when the user finalizes their composition.
/// The commit text should be inserted into the document at the cursor
/// position, replacing any displayed preedit text.
///
/// Note: An empty `ImePreedit` event is typically sent immediately
/// before this event to clear the composition display.
///
/// # Example
///
/// ```ignore
/// fn event(&mut self, event: &mut WidgetEvent) -> bool {
///     match event {
///         WidgetEvent::ImeCommit(e) => {
///             // Clear any displayed preedit
///             self.preedit_text = None;
///             // Insert the committed text
///             self.insert_text(&e.text);
///             event.accept();
///             true
///         }
///         _ => false,
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ImeCommitEvent {
    /// Base event data.
    pub base: EventBase,
    /// The finalized text to insert.
    pub text: String,
}

impl ImeCommitEvent {
    /// Create a new commit event.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            base: EventBase::new(),
            text: text.into(),
        }
    }
}

/// IME disabled event.
///
/// This event is sent when the Input Method Editor is disabled.
/// After receiving this event, you should clear any displayed preedit
/// text and no longer expect IME events until the next `ImeEnabled`.
///
/// # Example
///
/// ```ignore
/// fn event(&mut self, event: &mut WidgetEvent) -> bool {
///     match event {
///         WidgetEvent::ImeDisabled(e) => {
///             // IME is no longer active
///             self.ime_active = false;
///             self.preedit_text = None;
///             self.request_repaint();
///             event.accept();
///             true
///         }
///         _ => false,
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ImeDisabledEvent {
    /// Base event data.
    pub base: EventBase,
}

impl ImeDisabledEvent {
    /// Create a new IME disabled event.
    pub fn new() -> Self {
        Self {
            base: EventBase::new(),
        }
    }
}

impl Default for ImeDisabledEvent {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Touch Events
// =============================================================================

/// Phase of a touch event.
///
/// Describes the current state of a touch point in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TouchPhase {
    /// A new touch point started (finger touched the screen).
    Started,
    /// An existing touch point moved.
    Moved,
    /// A touch point ended normally (finger lifted).
    Ended,
    /// A touch point was cancelled (e.g., palm rejection, window lost focus).
    Cancelled,
}

/// Force/pressure information for a touch point.
///
/// Different devices provide force information in different formats.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TouchForce {
    /// Calibrated force data (typically from iOS devices with 3D Touch).
    ///
    /// Provides absolute force values with maximum force information.
    Calibrated {
        /// The force of the touch, where `1.0` represents the average pressure.
        force: f64,
        /// The maximum possible force value.
        max_possible_force: f64,
        /// The altitude angle of a stylus in radians, if applicable.
        /// `0` = parallel to surface, `π/2` = perpendicular.
        altitude_angle: Option<f64>,
    },
    /// Normalized force value in the range `0.0` to `1.0`.
    ///
    /// Used when device-specific calibration is not available.
    Normalized(f64),
}

impl TouchForce {
    /// Get the force as a normalized value in the range `0.0` to `1.0`.
    pub fn normalized(&self) -> f64 {
        match self {
            TouchForce::Calibrated {
                force,
                max_possible_force,
                ..
            } => {
                if *max_possible_force > 0.0 {
                    (*force / *max_possible_force).clamp(0.0, 1.0)
                } else {
                    0.0
                }
            }
            TouchForce::Normalized(f) => f.clamp(0.0, 1.0),
        }
    }
}

/// Information about a single touch point.
///
/// Each touch point has a unique ID that persists across events
/// from `Started` through `Moved` to `Ended` or `Cancelled`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchPoint {
    /// Unique identifier for this touch point.
    ///
    /// The ID is unique for the duration of the touch (from `Started` to
    /// `Ended`/`Cancelled`). After a touch ends, the ID may be reused
    /// for new touches.
    pub id: u64,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Position in global screen coordinates.
    pub global_pos: Point,
    /// The current phase of this touch point.
    pub phase: TouchPhase,
    /// Force/pressure information, if available.
    pub force: Option<TouchForce>,
}

impl TouchPoint {
    /// Create a new touch point.
    pub fn new(
        id: u64,
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        phase: TouchPhase,
    ) -> Self {
        Self {
            id,
            local_pos,
            window_pos,
            global_pos,
            phase,
            force: None,
        }
    }

    /// Create a new touch point with force information.
    pub fn with_force(
        id: u64,
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        phase: TouchPhase,
        force: TouchForce,
    ) -> Self {
        Self {
            id,
            local_pos,
            window_pos,
            global_pos,
            phase,
            force: Some(force),
        }
    }
}

/// Touch event containing one or more touch points.
///
/// Touch events are generated when the user touches the screen.
/// Multi-touch interactions generate events with multiple touch points.
///
/// # Example
///
/// ```ignore
/// fn event(&mut self, event: &mut WidgetEvent) -> bool {
///     match event {
///         WidgetEvent::Touch(e) => {
///             for point in &e.points {
///                 match point.phase {
///                     TouchPhase::Started => {
///                         println!("Touch {} started at {:?}", point.id, point.local_pos);
///                     }
///                     TouchPhase::Moved => {
///                         println!("Touch {} moved to {:?}", point.id, point.local_pos);
///                     }
///                     TouchPhase::Ended => {
///                         println!("Touch {} ended", point.id);
///                     }
///                     TouchPhase::Cancelled => {
///                         println!("Touch {} cancelled", point.id);
///                     }
///                 }
///             }
///             event.accept();
///             true
///         }
///         _ => false,
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TouchEvent {
    /// Base event data.
    pub base: EventBase,
    /// The touch points involved in this event.
    pub points: Vec<TouchPoint>,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl TouchEvent {
    /// Create a new touch event with a single touch point.
    pub fn new(point: TouchPoint, modifiers: KeyboardModifiers) -> Self {
        Self {
            base: EventBase::new(),
            points: vec![point],
            modifiers,
        }
    }

    /// Create a new touch event with multiple touch points.
    pub fn with_points(points: Vec<TouchPoint>, modifiers: KeyboardModifiers) -> Self {
        Self {
            base: EventBase::new(),
            points,
            modifiers,
        }
    }

    /// Get the primary touch point (first in the list).
    pub fn primary_point(&self) -> Option<&TouchPoint> {
        self.points.first()
    }

    /// Get a touch point by ID.
    pub fn point_by_id(&self, id: u64) -> Option<&TouchPoint> {
        self.points.iter().find(|p| p.id == id)
    }

    /// Get all points in a specific phase.
    pub fn points_in_phase(&self, phase: TouchPhase) -> impl Iterator<Item = &TouchPoint> {
        self.points.iter().filter(move |p| p.phase == phase)
    }

    /// Check if this event contains any points that started.
    pub fn has_started(&self) -> bool {
        self.points.iter().any(|p| p.phase == TouchPhase::Started)
    }

    /// Check if this event contains any points that ended.
    pub fn has_ended(&self) -> bool {
        self.points.iter().any(|p| p.phase == TouchPhase::Ended)
    }

    /// Get the number of touch points.
    pub fn touch_count(&self) -> usize {
        self.points.len()
    }
}

// =============================================================================
// Gesture Events
// =============================================================================

/// Type of gesture being performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GestureType {
    /// Single tap gesture.
    Tap,
    /// Double tap gesture.
    DoubleTap,
    /// Long press (tap and hold) gesture.
    LongPress,
    /// Pinch gesture (two-finger zoom).
    Pinch,
    /// Rotation gesture (two-finger rotate).
    Rotation,
    /// Swipe gesture (quick directional movement).
    Swipe,
    /// Pan gesture (drag/scroll).
    Pan,
}

/// Direction of a swipe gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SwipeDirection {
    /// Swipe from left to right.
    Right,
    /// Swipe from right to left.
    Left,
    /// Swipe from bottom to top.
    Up,
    /// Swipe from top to bottom.
    Down,
}

/// State of an ongoing gesture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GestureState {
    /// Gesture recognition has started.
    #[default]
    Started,
    /// Gesture is in progress and being updated.
    Updated,
    /// Gesture has ended successfully.
    Ended,
    /// Gesture was cancelled.
    Cancelled,
}

/// Tap gesture event.
///
/// Sent when a quick tap is detected on a touch surface.
#[derive(Debug, Clone, Copy)]
pub struct TapGestureEvent {
    /// Base event data.
    pub base: EventBase,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Position in global screen coordinates.
    pub global_pos: Point,
    /// Number of taps (1 for single tap, 2 for double tap, etc.).
    pub tap_count: u32,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl TapGestureEvent {
    /// Create a new tap gesture event.
    pub fn new(
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        tap_count: u32,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            global_pos,
            tap_count,
            modifiers,
        }
    }

    /// Check if this is a double tap.
    pub fn is_double_tap(&self) -> bool {
        self.tap_count >= 2
    }
}

/// Long press gesture event.
///
/// Sent when the user touches and holds for a threshold duration.
#[derive(Debug, Clone, Copy)]
pub struct LongPressGestureEvent {
    /// Base event data.
    pub base: EventBase,
    /// Position in widget-local coordinates.
    pub local_pos: Point,
    /// Position in window coordinates.
    pub window_pos: Point,
    /// Position in global screen coordinates.
    pub global_pos: Point,
    /// The state of the gesture.
    pub state: GestureState,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl LongPressGestureEvent {
    /// Create a new long press gesture event.
    pub fn new(
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        state: GestureState,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            global_pos,
            state,
            modifiers,
        }
    }
}

/// Pinch (zoom) gesture event.
///
/// Sent during a two-finger pinch gesture, typically used for zooming.
#[derive(Debug, Clone, Copy)]
pub struct PinchGestureEvent {
    /// Base event data.
    pub base: EventBase,
    /// Center position in widget-local coordinates.
    pub local_pos: Point,
    /// Center position in window coordinates.
    pub window_pos: Point,
    /// Center position in global screen coordinates.
    pub global_pos: Point,
    /// Scale factor relative to the start of the gesture.
    ///
    /// A value of `1.0` means no change, `2.0` means doubled size,
    /// `0.5` means halved size.
    pub scale: f64,
    /// Change in scale since the last event.
    pub delta: f64,
    /// The state of the gesture.
    pub state: GestureState,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl PinchGestureEvent {
    /// Create a new pinch gesture event.
    pub fn new(
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        scale: f64,
        delta: f64,
        state: GestureState,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            global_pos,
            scale,
            delta,
            state,
            modifiers,
        }
    }
}

/// Rotation gesture event.
///
/// Sent during a two-finger rotation gesture.
#[derive(Debug, Clone, Copy)]
pub struct RotationGestureEvent {
    /// Base event data.
    pub base: EventBase,
    /// Center position in widget-local coordinates.
    pub local_pos: Point,
    /// Center position in window coordinates.
    pub window_pos: Point,
    /// Center position in global screen coordinates.
    pub global_pos: Point,
    /// Total rotation in radians since the gesture started.
    ///
    /// Positive values indicate clockwise rotation.
    pub rotation: f64,
    /// Change in rotation since the last event (in radians).
    pub delta: f64,
    /// The state of the gesture.
    pub state: GestureState,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl RotationGestureEvent {
    /// Create a new rotation gesture event.
    pub fn new(
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        rotation: f64,
        delta: f64,
        state: GestureState,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            global_pos,
            rotation,
            delta,
            state,
            modifiers,
        }
    }

    /// Get the rotation in degrees.
    pub fn rotation_degrees(&self) -> f64 {
        self.rotation.to_degrees()
    }

    /// Get the delta in degrees.
    pub fn delta_degrees(&self) -> f64 {
        self.delta.to_degrees()
    }
}

/// Swipe gesture event.
///
/// Sent when a quick swipe motion is detected.
#[derive(Debug, Clone, Copy)]
pub struct SwipeGestureEvent {
    /// Base event data.
    pub base: EventBase,
    /// Starting position in widget-local coordinates.
    pub start_local_pos: Point,
    /// Ending position in widget-local coordinates.
    pub end_local_pos: Point,
    /// Starting position in window coordinates.
    pub start_window_pos: Point,
    /// Ending position in window coordinates.
    pub end_window_pos: Point,
    /// The direction of the swipe.
    pub direction: SwipeDirection,
    /// The velocity of the swipe in pixels per second.
    pub velocity: f32,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl SwipeGestureEvent {
    /// Create a new swipe gesture event.
    pub fn new(
        start_local_pos: Point,
        end_local_pos: Point,
        start_window_pos: Point,
        end_window_pos: Point,
        direction: SwipeDirection,
        velocity: f32,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            start_local_pos,
            end_local_pos,
            start_window_pos,
            end_window_pos,
            direction,
            velocity,
            modifiers,
        }
    }

    /// Get the distance of the swipe.
    pub fn distance(&self) -> f32 {
        let dx = self.end_local_pos.x - self.start_local_pos.x;
        let dy = self.end_local_pos.y - self.start_local_pos.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Pan (drag/scroll) gesture event.
///
/// Sent during a drag or scroll gesture.
#[derive(Debug, Clone, Copy)]
pub struct PanGestureEvent {
    /// Base event data.
    pub base: EventBase,
    /// Current position in widget-local coordinates.
    pub local_pos: Point,
    /// Current position in window coordinates.
    pub window_pos: Point,
    /// Current position in global screen coordinates.
    pub global_pos: Point,
    /// Total translation since the gesture started.
    pub translation: Point,
    /// Translation since the last event.
    pub delta: Point,
    /// Current velocity in pixels per second.
    pub velocity: Point,
    /// The state of the gesture.
    pub state: GestureState,
    /// Keyboard modifiers held during the event.
    pub modifiers: KeyboardModifiers,
}

impl PanGestureEvent {
    /// Create a new pan gesture event.
    pub fn new(
        local_pos: Point,
        window_pos: Point,
        global_pos: Point,
        translation: Point,
        delta: Point,
        velocity: Point,
        state: GestureState,
        modifiers: KeyboardModifiers,
    ) -> Self {
        Self {
            base: EventBase::new(),
            local_pos,
            window_pos,
            global_pos,
            translation,
            delta,
            velocity,
            state,
            modifiers,
        }
    }
}

// =============================================================================
// Context Menu Event
// =============================================================================

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

/// Close event, sent when a window is about to close.
///
/// Unlike most events which are not accepted by default, a CloseEvent is
/// **accepted by default**. This means the close will proceed unless a handler
/// calls `ignore()` to prevent it.
///
/// # Example
///
/// ```ignore
/// fn handle_close(event: &mut CloseEvent) {
///     if has_unsaved_changes() {
///         // Prevent the close
///         event.ignore();
///     }
///     // Otherwise, close proceeds automatically (accepted by default)
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct CloseEvent {
    /// Whether the close is accepted (will proceed).
    ///
    /// Unlike EventBase, this defaults to `true` - close proceeds unless ignored.
    accepted: bool,
}

impl CloseEvent {
    /// Create a new close event.
    ///
    /// The event is accepted by default (close will proceed).
    pub fn new() -> Self {
        Self { accepted: true }
    }

    /// Check if the close has been accepted.
    ///
    /// Returns `true` if the close will proceed, `false` if it was prevented.
    pub fn is_accepted(&self) -> bool {
        self.accepted
    }

    /// Accept the close event, allowing the window to close.
    ///
    /// This is the default state, so calling this is only necessary if
    /// the event was previously ignored.
    pub fn accept(&mut self) {
        self.accepted = true;
    }

    /// Ignore the close event, preventing the window from closing.
    ///
    /// Call this in your close handler to prevent the close operation.
    pub fn ignore(&mut self) {
        self.accepted = false;
    }
}

impl Default for CloseEvent {
    fn default() -> Self {
        Self::new()
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
    /// IME enabled event.
    ///
    /// Sent when the Input Method Editor is enabled for this widget.
    ImeEnabled(ImeEnabledEvent),
    /// IME preedit (composition) event.
    ///
    /// Sent when the user is composing text through the IME.
    /// The preedit text should be displayed at the cursor position.
    ImePreedit(ImePreeditEvent),
    /// IME commit event.
    ///
    /// Sent when the user finalizes their IME composition.
    /// The text should be inserted at the cursor position.
    ImeCommit(ImeCommitEvent),
    /// IME disabled event.
    ///
    /// Sent when the Input Method Editor is disabled.
    ImeDisabled(ImeDisabledEvent),
    /// Touch event.
    ///
    /// Sent when touch input is detected on a touch-enabled device.
    Touch(TouchEvent),
    /// Tap gesture event.
    ///
    /// Sent when a tap gesture is recognized.
    TapGesture(TapGestureEvent),
    /// Long press gesture event.
    ///
    /// Sent when a long press gesture is recognized.
    LongPressGesture(LongPressGestureEvent),
    /// Pinch gesture event.
    ///
    /// Sent when a pinch (zoom) gesture is recognized.
    PinchGesture(PinchGestureEvent),
    /// Rotation gesture event.
    ///
    /// Sent when a rotation gesture is recognized.
    RotationGesture(RotationGestureEvent),
    /// Swipe gesture event.
    ///
    /// Sent when a swipe gesture is recognized.
    SwipeGesture(SwipeGestureEvent),
    /// Pan gesture event.
    ///
    /// Sent when a pan (drag) gesture is recognized.
    PanGesture(PanGestureEvent),
    /// Drag enter event.
    ///
    /// Sent when a drag operation enters a widget's bounds.
    DragEnter(super::drag_drop::DragEnterEvent),
    /// Drag move event.
    ///
    /// Sent when a drag operation moves within a widget's bounds.
    DragMove(super::drag_drop::DragMoveEvent),
    /// Drag leave event.
    ///
    /// Sent when a drag operation leaves a widget's bounds.
    DragLeave(super::drag_drop::DragLeaveEvent),
    /// Drop event.
    ///
    /// Sent when data is dropped on a widget.
    Drop(super::drag_drop::DropEvent),
    /// Close event.
    ///
    /// Sent when a window is about to close. Unlike other events,
    /// CloseEvent is accepted by default - the close proceeds unless
    /// a handler calls `ignore()` to prevent it.
    Close(CloseEvent),
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
            Self::ImeEnabled(e) => e.base.is_accepted(),
            Self::ImePreedit(e) => e.base.is_accepted(),
            Self::ImeCommit(e) => e.base.is_accepted(),
            Self::ImeDisabled(e) => e.base.is_accepted(),
            Self::Touch(e) => e.base.is_accepted(),
            Self::TapGesture(e) => e.base.is_accepted(),
            Self::LongPressGesture(e) => e.base.is_accepted(),
            Self::PinchGesture(e) => e.base.is_accepted(),
            Self::RotationGesture(e) => e.base.is_accepted(),
            Self::SwipeGesture(e) => e.base.is_accepted(),
            Self::PanGesture(e) => e.base.is_accepted(),
            Self::DragEnter(e) => e.base.is_accepted(),
            Self::DragMove(e) => e.base.is_accepted(),
            Self::DragLeave(e) => e.base.is_accepted(),
            Self::Drop(e) => e.base.is_accepted(),
            Self::Close(e) => e.is_accepted(),
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
            Self::ImeEnabled(e) => e.base.accept(),
            Self::ImePreedit(e) => e.base.accept(),
            Self::ImeCommit(e) => e.base.accept(),
            Self::ImeDisabled(e) => e.base.accept(),
            Self::Touch(e) => e.base.accept(),
            Self::TapGesture(e) => e.base.accept(),
            Self::LongPressGesture(e) => e.base.accept(),
            Self::PinchGesture(e) => e.base.accept(),
            Self::RotationGesture(e) => e.base.accept(),
            Self::SwipeGesture(e) => e.base.accept(),
            Self::PanGesture(e) => e.base.accept(),
            Self::DragEnter(e) => e.base.accept(),
            Self::DragMove(e) => e.base.accept(),
            Self::DragLeave(e) => e.base.accept(),
            Self::Drop(e) => e.base.accept(),
            Self::Close(e) => e.accept(),
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
            Self::ImeEnabled(e) => e.base.ignore(),
            Self::ImePreedit(e) => e.base.ignore(),
            Self::ImeCommit(e) => e.base.ignore(),
            Self::ImeDisabled(e) => e.base.ignore(),
            Self::Touch(e) => e.base.ignore(),
            Self::TapGesture(e) => e.base.ignore(),
            Self::LongPressGesture(e) => e.base.ignore(),
            Self::PinchGesture(e) => e.base.ignore(),
            Self::RotationGesture(e) => e.base.ignore(),
            Self::SwipeGesture(e) => e.base.ignore(),
            Self::PanGesture(e) => e.base.ignore(),
            Self::DragEnter(e) => e.base.ignore(),
            Self::DragMove(e) => e.base.ignore(),
            Self::DragLeave(e) => e.base.ignore(),
            Self::Drop(e) => e.base.ignore(),
            Self::Close(e) => e.ignore(),
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
            // IME events are specific to the focused widget and don't propagate
            Self::ImeEnabled(_)
            | Self::ImePreedit(_)
            | Self::ImeCommit(_)
            | Self::ImeDisabled(_) => false,
            // Touch events propagate if not accepted
            Self::Touch(_) => !self.is_accepted(),
            // Gesture events propagate if not accepted
            Self::TapGesture(_)
            | Self::LongPressGesture(_)
            | Self::PinchGesture(_)
            | Self::RotationGesture(_)
            | Self::SwipeGesture(_)
            | Self::PanGesture(_) => !self.is_accepted(),
            // Drag/drop events don't propagate - they are targeted at specific widgets
            // based on hit testing during the drag operation
            Self::DragEnter(_) | Self::DragMove(_) | Self::DragLeave(_) | Self::Drop(_) => false,
            // Close events are window-specific and don't propagate
            Self::Close(_) => false,
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
