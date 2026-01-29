//! Keyboard input handling and conversion from platform events.
//!
//! This module provides conversion functions for translating platform-level
//! keyboard events (from winit) into Horizon Lattice widget events.
//!
//! # Usage
//!
//! The main entry point is [`KeyboardInputHandler`], which manages modifier
//! state and converts raw keyboard events into widget events.
//!
//! ```ignore
//! use horizon_lattice::widget::keyboard::KeyboardInputHandler;
//!
//! let mut handler = KeyboardInputHandler::new();
//!
//! // When receiving a winit keyboard event:
//! if let Some(widget_event) = handler.handle_keyboard_event(&winit_event) {
//!     // Dispatch widget_event to the focused widget
//! }
//! ```

use winit::event::{ElementState, Modifiers};
use winit::keyboard::{Key as WinitKey, KeyCode, NamedKey, PhysicalKey};

use super::events::{Key, KeyPressEvent, KeyReleaseEvent, KeyboardModifiers};

/// A scan code representing the physical key on the keyboard.
///
/// Scan codes are keyboard-layout independent and represent the physical
/// position of the key. This is useful for games and applications that
/// need consistent key positions regardless of keyboard layout.
pub type ScanCode = u32;

/// Converts a winit logical key to a Horizon Lattice Key.
///
/// This handles both named keys (like Enter, Backspace) and character keys.
pub fn from_winit_key(key: &WinitKey) -> Key {
    match key {
        WinitKey::Named(named) => from_winit_named_key(named),
        WinitKey::Character(c) => from_character(c),
        WinitKey::Unidentified(_) => Key::Unknown(0),
        WinitKey::Dead(_) => Key::Unknown(0),
    }
}

/// Converts a winit named key to a Horizon Lattice Key.
fn from_winit_named_key(key: &NamedKey) -> Key {
    match key {
        // Letters (not typically named keys, but handle just in case)
        // Navigation
        NamedKey::ArrowUp => Key::ArrowUp,
        NamedKey::ArrowDown => Key::ArrowDown,
        NamedKey::ArrowLeft => Key::ArrowLeft,
        NamedKey::ArrowRight => Key::ArrowRight,
        NamedKey::Home => Key::Home,
        NamedKey::End => Key::End,
        NamedKey::PageUp => Key::PageUp,
        NamedKey::PageDown => Key::PageDown,

        // Editing
        NamedKey::Backspace => Key::Backspace,
        NamedKey::Delete => Key::Delete,
        NamedKey::Insert => Key::Insert,
        NamedKey::Enter => Key::Enter,
        NamedKey::Tab => Key::Tab,
        NamedKey::Space => Key::Space,
        NamedKey::Escape => Key::Escape,

        // Modifiers
        NamedKey::Shift => Key::ShiftLeft, // Generic shift
        NamedKey::Control => Key::ControlLeft,
        NamedKey::Alt => Key::AltLeft,
        NamedKey::Super => Key::MetaLeft, // Super/Meta/Windows/Command

        // Lock keys
        NamedKey::CapsLock => Key::CapsLock,
        NamedKey::NumLock => Key::NumLock,
        NamedKey::ScrollLock => Key::ScrollLock,

        // Function keys
        NamedKey::F1 => Key::F1,
        NamedKey::F2 => Key::F2,
        NamedKey::F3 => Key::F3,
        NamedKey::F4 => Key::F4,
        NamedKey::F5 => Key::F5,
        NamedKey::F6 => Key::F6,
        NamedKey::F7 => Key::F7,
        NamedKey::F8 => Key::F8,
        NamedKey::F9 => Key::F9,
        NamedKey::F10 => Key::F10,
        NamedKey::F11 => Key::F11,
        NamedKey::F12 => Key::F12,

        // System keys
        NamedKey::PrintScreen => Key::PrintScreen,
        NamedKey::Pause => Key::Pause,

        // Media keys
        NamedKey::MediaPlayPause => Key::MediaPlayPause,
        NamedKey::MediaStop => Key::MediaStop,
        NamedKey::MediaTrackNext => Key::MediaNext,
        NamedKey::MediaTrackPrevious => Key::MediaPrevious,
        NamedKey::AudioVolumeUp => Key::AudioVolumeUp,
        NamedKey::AudioVolumeDown => Key::AudioVolumeDown,
        NamedKey::AudioVolumeMute => Key::AudioVolumeMute,

        // Other named keys map to Unknown
        _ => Key::Unknown(0),
    }
}

/// Converts a character string to a Horizon Lattice Key.
///
/// This handles single character keys like letters, digits, and punctuation.
fn from_character(c: &str) -> Key {
    // Handle single character keys
    let chars: Vec<char> = c.chars().collect();
    if chars.len() != 1 {
        return Key::Unknown(0);
    }

    match chars[0].to_ascii_lowercase() {
        'a' => Key::A,
        'b' => Key::B,
        'c' => Key::C,
        'd' => Key::D,
        'e' => Key::E,
        'f' => Key::F,
        'g' => Key::G,
        'h' => Key::H,
        'i' => Key::I,
        'j' => Key::J,
        'k' => Key::K,
        'l' => Key::L,
        'm' => Key::M,
        'n' => Key::N,
        'o' => Key::O,
        'p' => Key::P,
        'q' => Key::Q,
        'r' => Key::R,
        's' => Key::S,
        't' => Key::T,
        'u' => Key::U,
        'v' => Key::V,
        'w' => Key::W,
        'x' => Key::X,
        'y' => Key::Y,
        'z' => Key::Z,
        '0' => Key::Digit0,
        '1' => Key::Digit1,
        '2' => Key::Digit2,
        '3' => Key::Digit3,
        '4' => Key::Digit4,
        '5' => Key::Digit5,
        '6' => Key::Digit6,
        '7' => Key::Digit7,
        '8' => Key::Digit8,
        '9' => Key::Digit9,
        '-' => Key::Minus,
        '=' => Key::Equal,
        '[' => Key::BracketLeft,
        ']' => Key::BracketRight,
        '\\' => Key::Backslash,
        ';' => Key::Semicolon,
        '\'' => Key::Quote,
        ',' => Key::Comma,
        '.' => Key::Period,
        '/' => Key::Slash,
        '`' => Key::Grave,
        ' ' => Key::Space,
        _ => Key::Unknown(chars[0] as u16),
    }
}

/// Converts a winit physical key (key code) to a Horizon Lattice Key.
///
/// Physical keys represent the physical position on the keyboard,
/// independent of the keyboard layout.
pub fn from_winit_physical_key(physical: &PhysicalKey) -> Key {
    match physical {
        PhysicalKey::Code(code) => from_winit_key_code(code),
        PhysicalKey::Unidentified(_) => Key::Unknown(0),
    }
}

/// Converts a winit key code to a Horizon Lattice Key.
fn from_winit_key_code(code: &KeyCode) -> Key {
    match code {
        // Letters
        KeyCode::KeyA => Key::A,
        KeyCode::KeyB => Key::B,
        KeyCode::KeyC => Key::C,
        KeyCode::KeyD => Key::D,
        KeyCode::KeyE => Key::E,
        KeyCode::KeyF => Key::F,
        KeyCode::KeyG => Key::G,
        KeyCode::KeyH => Key::H,
        KeyCode::KeyI => Key::I,
        KeyCode::KeyJ => Key::J,
        KeyCode::KeyK => Key::K,
        KeyCode::KeyL => Key::L,
        KeyCode::KeyM => Key::M,
        KeyCode::KeyN => Key::N,
        KeyCode::KeyO => Key::O,
        KeyCode::KeyP => Key::P,
        KeyCode::KeyQ => Key::Q,
        KeyCode::KeyR => Key::R,
        KeyCode::KeyS => Key::S,
        KeyCode::KeyT => Key::T,
        KeyCode::KeyU => Key::U,
        KeyCode::KeyV => Key::V,
        KeyCode::KeyW => Key::W,
        KeyCode::KeyX => Key::X,
        KeyCode::KeyY => Key::Y,
        KeyCode::KeyZ => Key::Z,

        // Digits
        KeyCode::Digit0 => Key::Digit0,
        KeyCode::Digit1 => Key::Digit1,
        KeyCode::Digit2 => Key::Digit2,
        KeyCode::Digit3 => Key::Digit3,
        KeyCode::Digit4 => Key::Digit4,
        KeyCode::Digit5 => Key::Digit5,
        KeyCode::Digit6 => Key::Digit6,
        KeyCode::Digit7 => Key::Digit7,
        KeyCode::Digit8 => Key::Digit8,
        KeyCode::Digit9 => Key::Digit9,

        // Function keys
        KeyCode::F1 => Key::F1,
        KeyCode::F2 => Key::F2,
        KeyCode::F3 => Key::F3,
        KeyCode::F4 => Key::F4,
        KeyCode::F5 => Key::F5,
        KeyCode::F6 => Key::F6,
        KeyCode::F7 => Key::F7,
        KeyCode::F8 => Key::F8,
        KeyCode::F9 => Key::F9,
        KeyCode::F10 => Key::F10,
        KeyCode::F11 => Key::F11,
        KeyCode::F12 => Key::F12,

        // Navigation
        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,

        // Editing
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Space => Key::Space,
        KeyCode::Escape => Key::Escape,

        // Modifiers - with left/right distinction
        KeyCode::ShiftLeft => Key::ShiftLeft,
        KeyCode::ShiftRight => Key::ShiftRight,
        KeyCode::ControlLeft => Key::ControlLeft,
        KeyCode::ControlRight => Key::ControlRight,
        KeyCode::AltLeft => Key::AltLeft,
        KeyCode::AltRight => Key::AltRight,
        KeyCode::SuperLeft => Key::MetaLeft,
        KeyCode::SuperRight => Key::MetaRight,

        // Lock keys
        KeyCode::CapsLock => Key::CapsLock,
        KeyCode::NumLock => Key::NumLock,
        KeyCode::ScrollLock => Key::ScrollLock,

        // Punctuation
        KeyCode::Minus => Key::Minus,
        KeyCode::Equal => Key::Equal,
        KeyCode::BracketLeft => Key::BracketLeft,
        KeyCode::BracketRight => Key::BracketRight,
        KeyCode::Backslash => Key::Backslash,
        KeyCode::Semicolon => Key::Semicolon,
        KeyCode::Quote => Key::Quote,
        KeyCode::Comma => Key::Comma,
        KeyCode::Period => Key::Period,
        KeyCode::Slash => Key::Slash,
        KeyCode::Backquote => Key::Grave,

        // System
        KeyCode::PrintScreen => Key::PrintScreen,
        KeyCode::Pause => Key::Pause,

        // Numpad
        KeyCode::Numpad0 => Key::Numpad0,
        KeyCode::Numpad1 => Key::Numpad1,
        KeyCode::Numpad2 => Key::Numpad2,
        KeyCode::Numpad3 => Key::Numpad3,
        KeyCode::Numpad4 => Key::Numpad4,
        KeyCode::Numpad5 => Key::Numpad5,
        KeyCode::Numpad6 => Key::Numpad6,
        KeyCode::Numpad7 => Key::Numpad7,
        KeyCode::Numpad8 => Key::Numpad8,
        KeyCode::Numpad9 => Key::Numpad9,
        KeyCode::NumpadAdd => Key::NumpadAdd,
        KeyCode::NumpadSubtract => Key::NumpadSubtract,
        KeyCode::NumpadMultiply => Key::NumpadMultiply,
        KeyCode::NumpadDivide => Key::NumpadDivide,
        KeyCode::NumpadDecimal => Key::NumpadDecimal,
        KeyCode::NumpadEnter => Key::NumpadEnter,

        // Media keys
        KeyCode::MediaPlayPause => Key::MediaPlayPause,
        KeyCode::MediaStop => Key::MediaStop,
        KeyCode::MediaTrackNext => Key::MediaNext,
        KeyCode::MediaTrackPrevious => Key::MediaPrevious,
        KeyCode::AudioVolumeUp => Key::AudioVolumeUp,
        KeyCode::AudioVolumeDown => Key::AudioVolumeDown,
        KeyCode::AudioVolumeMute => Key::AudioVolumeMute,

        _ => Key::Unknown(0),
    }
}

/// Converts winit modifiers to Horizon Lattice KeyboardModifiers.
pub fn from_winit_modifiers(modifiers: &Modifiers) -> KeyboardModifiers {
    let state = modifiers.state();
    KeyboardModifiers {
        shift: state.shift_key(),
        control: state.control_key(),
        alt: state.alt_key(),
        meta: state.super_key(),
    }
}

/// Extracts a scan code from a winit physical key.
///
/// Returns `None` if the physical key doesn't have a scan code.
pub fn scan_code_from_physical_key(physical: &PhysicalKey) -> Option<ScanCode> {
    match physical {
        PhysicalKey::Code(code) => Some(key_code_to_scan_code(code)),
        PhysicalKey::Unidentified(native) => {
            // Try to extract native scan code
            #[cfg(target_os = "windows")]
            {
                use winit::keyboard::NativeKeyCode;
                match native {
                    NativeKeyCode::Windows(scancode) => Some(*scancode as ScanCode),
                    _ => None,
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = native;
                None
            }
        }
    }
}

/// Converts a winit KeyCode to an approximate scan code.
///
/// This provides a platform-independent scan code based on the key code.
/// For actual hardware scan codes, use [`scan_code_from_physical_key`].
fn key_code_to_scan_code(code: &KeyCode) -> ScanCode {
    // Use discriminant as a unique identifier for each key
    // This is not the actual hardware scan code, but a unique identifier
    match code {
        KeyCode::Escape => 0x01,
        KeyCode::Digit1 => 0x02,
        KeyCode::Digit2 => 0x03,
        KeyCode::Digit3 => 0x04,
        KeyCode::Digit4 => 0x05,
        KeyCode::Digit5 => 0x06,
        KeyCode::Digit6 => 0x07,
        KeyCode::Digit7 => 0x08,
        KeyCode::Digit8 => 0x09,
        KeyCode::Digit9 => 0x0A,
        KeyCode::Digit0 => 0x0B,
        KeyCode::Minus => 0x0C,
        KeyCode::Equal => 0x0D,
        KeyCode::Backspace => 0x0E,
        KeyCode::Tab => 0x0F,
        KeyCode::KeyQ => 0x10,
        KeyCode::KeyW => 0x11,
        KeyCode::KeyE => 0x12,
        KeyCode::KeyR => 0x13,
        KeyCode::KeyT => 0x14,
        KeyCode::KeyY => 0x15,
        KeyCode::KeyU => 0x16,
        KeyCode::KeyI => 0x17,
        KeyCode::KeyO => 0x18,
        KeyCode::KeyP => 0x19,
        KeyCode::BracketLeft => 0x1A,
        KeyCode::BracketRight => 0x1B,
        KeyCode::Enter => 0x1C,
        KeyCode::ControlLeft => 0x1D,
        KeyCode::KeyA => 0x1E,
        KeyCode::KeyS => 0x1F,
        KeyCode::KeyD => 0x20,
        KeyCode::KeyF => 0x21,
        KeyCode::KeyG => 0x22,
        KeyCode::KeyH => 0x23,
        KeyCode::KeyJ => 0x24,
        KeyCode::KeyK => 0x25,
        KeyCode::KeyL => 0x26,
        KeyCode::Semicolon => 0x27,
        KeyCode::Quote => 0x28,
        KeyCode::Backquote => 0x29,
        KeyCode::ShiftLeft => 0x2A,
        KeyCode::Backslash => 0x2B,
        KeyCode::KeyZ => 0x2C,
        KeyCode::KeyX => 0x2D,
        KeyCode::KeyC => 0x2E,
        KeyCode::KeyV => 0x2F,
        KeyCode::KeyB => 0x30,
        KeyCode::KeyN => 0x31,
        KeyCode::KeyM => 0x32,
        KeyCode::Comma => 0x33,
        KeyCode::Period => 0x34,
        KeyCode::Slash => 0x35,
        KeyCode::ShiftRight => 0x36,
        KeyCode::NumpadMultiply => 0x37,
        KeyCode::AltLeft => 0x38,
        KeyCode::Space => 0x39,
        KeyCode::CapsLock => 0x3A,
        KeyCode::F1 => 0x3B,
        KeyCode::F2 => 0x3C,
        KeyCode::F3 => 0x3D,
        KeyCode::F4 => 0x3E,
        KeyCode::F5 => 0x3F,
        KeyCode::F6 => 0x40,
        KeyCode::F7 => 0x41,
        KeyCode::F8 => 0x42,
        KeyCode::F9 => 0x43,
        KeyCode::F10 => 0x44,
        KeyCode::NumLock => 0x45,
        KeyCode::ScrollLock => 0x46,
        KeyCode::Numpad7 => 0x47,
        KeyCode::Numpad8 => 0x48,
        KeyCode::Numpad9 => 0x49,
        KeyCode::NumpadSubtract => 0x4A,
        KeyCode::Numpad4 => 0x4B,
        KeyCode::Numpad5 => 0x4C,
        KeyCode::Numpad6 => 0x4D,
        KeyCode::NumpadAdd => 0x4E,
        KeyCode::Numpad1 => 0x4F,
        KeyCode::Numpad2 => 0x50,
        KeyCode::Numpad3 => 0x51,
        KeyCode::Numpad0 => 0x52,
        KeyCode::NumpadDecimal => 0x53,
        KeyCode::F11 => 0x57,
        KeyCode::F12 => 0x58,
        KeyCode::NumpadEnter => 0x11C,
        KeyCode::ControlRight => 0x11D,
        KeyCode::NumpadDivide => 0x135,
        KeyCode::PrintScreen => 0x137,
        KeyCode::AltRight => 0x138,
        KeyCode::Home => 0x147,
        KeyCode::ArrowUp => 0x148,
        KeyCode::PageUp => 0x149,
        KeyCode::ArrowLeft => 0x14B,
        KeyCode::ArrowRight => 0x14D,
        KeyCode::End => 0x14F,
        KeyCode::ArrowDown => 0x150,
        KeyCode::PageDown => 0x151,
        KeyCode::Insert => 0x152,
        KeyCode::Delete => 0x153,
        KeyCode::SuperLeft => 0x15B,
        KeyCode::SuperRight => 0x15C,
        KeyCode::Pause => 0x21D,
        _ => 0,
    }
}

/// Handler for keyboard input that maintains modifier state.
///
/// This struct provides a stateful interface for converting winit keyboard
/// events into widget events, tracking modifier key state across events.
#[derive(Debug, Default)]
pub struct KeyboardInputHandler {
    /// Current modifier key state.
    modifiers: KeyboardModifiers,
}

impl KeyboardInputHandler {
    /// Creates a new keyboard input handler with no modifiers pressed.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the current modifier key state.
    pub fn modifiers(&self) -> KeyboardModifiers {
        self.modifiers
    }

    /// Updates the modifier state from a winit Modifiers event.
    pub fn update_modifiers(&mut self, modifiers: &Modifiers) {
        self.modifiers = from_winit_modifiers(modifiers);
    }

    /// Creates a KeyPressEvent from winit keyboard event data.
    ///
    /// # Arguments
    ///
    /// * `logical_key` - The logical key that was pressed
    /// * `physical_key` - The physical key location
    /// * `text` - The text generated by this key press (if any)
    /// * `is_repeat` - Whether this is an auto-repeat event
    pub fn create_key_press_event(
        &self,
        logical_key: &WinitKey,
        physical_key: &PhysicalKey,
        text: Option<&str>,
        is_repeat: bool,
    ) -> KeyPressEvent {
        // Prefer physical key for consistent key identification
        let key = from_winit_physical_key(physical_key);
        let key = if matches!(key, Key::Unknown(_)) {
            // Fall back to logical key
            from_winit_key(logical_key)
        } else {
            key
        };

        let scan_code = scan_code_from_physical_key(physical_key);

        KeyPressEvent::new_with_scan_code(
            key,
            self.modifiers,
            text.unwrap_or(""),
            is_repeat,
            scan_code,
        )
    }

    /// Creates a KeyReleaseEvent from winit keyboard event data.
    ///
    /// # Arguments
    ///
    /// * `logical_key` - The logical key that was released
    /// * `physical_key` - The physical key location
    pub fn create_key_release_event(
        &self,
        logical_key: &WinitKey,
        physical_key: &PhysicalKey,
    ) -> KeyReleaseEvent {
        // Prefer physical key for consistent key identification
        let key = from_winit_physical_key(physical_key);
        let key = if matches!(key, Key::Unknown(_)) {
            // Fall back to logical key
            from_winit_key(logical_key)
        } else {
            key
        };

        let scan_code = scan_code_from_physical_key(physical_key);

        KeyReleaseEvent::new_with_scan_code(key, self.modifiers, scan_code)
    }

    /// Processes a winit keyboard event and returns the appropriate widget event.
    ///
    /// # Arguments
    ///
    /// * `logical_key` - The logical key from the event
    /// * `physical_key` - The physical key from the event
    /// * `state` - Whether the key was pressed or released
    /// * `text` - The text generated (for press events)
    /// * `is_repeat` - Whether this is an auto-repeat
    ///
    /// # Returns
    ///
    /// Either a `KeyPressEvent` or `KeyReleaseEvent` wrapped in `KeyboardEvent`.
    pub fn process_keyboard_event(
        &self,
        logical_key: &WinitKey,
        physical_key: &PhysicalKey,
        state: ElementState,
        text: Option<&str>,
        is_repeat: bool,
    ) -> KeyboardEvent {
        match state {
            ElementState::Pressed => KeyboardEvent::Press(self.create_key_press_event(
                logical_key,
                physical_key,
                text,
                is_repeat,
            )),
            ElementState::Released => {
                KeyboardEvent::Release(self.create_key_release_event(logical_key, physical_key))
            }
        }
    }
}

/// A keyboard event that can be either a press or release.
#[derive(Debug, Clone)]
pub enum KeyboardEvent {
    /// A key was pressed.
    Press(KeyPressEvent),
    /// A key was released.
    Release(KeyReleaseEvent),
}

impl KeyboardEvent {
    /// Converts this keyboard event into a WidgetEvent.
    pub fn into_widget_event(self) -> super::events::WidgetEvent {
        match self {
            KeyboardEvent::Press(e) => super::events::WidgetEvent::KeyPress(e),
            KeyboardEvent::Release(e) => super::events::WidgetEvent::KeyRelease(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_conversion() {
        assert_eq!(from_character("a"), Key::A);
        assert_eq!(from_character("A"), Key::A); // Case insensitive
        assert_eq!(from_character("z"), Key::Z);
        assert_eq!(from_character("0"), Key::Digit0);
        assert_eq!(from_character("9"), Key::Digit9);
        assert_eq!(from_character(" "), Key::Space);
    }

    #[test]
    fn test_multi_char_returns_unknown() {
        assert!(matches!(from_character("ab"), Key::Unknown(_)));
        assert!(matches!(from_character(""), Key::Unknown(_)));
    }

    #[test]
    fn test_key_code_conversion() {
        assert_eq!(from_winit_key_code(&KeyCode::KeyA), Key::A);
        assert_eq!(from_winit_key_code(&KeyCode::Enter), Key::Enter);
        assert_eq!(from_winit_key_code(&KeyCode::F1), Key::F1);
        assert_eq!(from_winit_key_code(&KeyCode::ShiftLeft), Key::ShiftLeft);
        assert_eq!(from_winit_key_code(&KeyCode::ShiftRight), Key::ShiftRight);
    }

    #[test]
    fn test_named_key_conversion() {
        assert_eq!(from_winit_named_key(&NamedKey::Enter), Key::Enter);
        assert_eq!(from_winit_named_key(&NamedKey::Backspace), Key::Backspace);
        assert_eq!(from_winit_named_key(&NamedKey::Tab), Key::Tab);
        assert_eq!(from_winit_named_key(&NamedKey::Escape), Key::Escape);
    }

    #[test]
    fn test_scan_code_mapping() {
        // Test some well-known scan codes
        assert_eq!(key_code_to_scan_code(&KeyCode::Escape), 0x01);
        assert_eq!(key_code_to_scan_code(&KeyCode::Enter), 0x1C);
        assert_eq!(key_code_to_scan_code(&KeyCode::Space), 0x39);
        assert_eq!(key_code_to_scan_code(&KeyCode::KeyA), 0x1E);
    }

    #[test]
    fn test_keyboard_input_handler() {
        let handler = KeyboardInputHandler::new();
        assert_eq!(handler.modifiers(), KeyboardModifiers::NONE);
    }
}
