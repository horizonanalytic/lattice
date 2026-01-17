//! Input Method Editor (IME) handling and conversion from platform events.
//!
//! This module provides support for complex text input through Input Method Editors,
//! which are essential for languages that require composition (like Chinese, Japanese,
//! Korean) or for entering accented characters through dead keys.
//!
//! # Overview
//!
//! The IME system works by:
//! 1. Enabling IME when a text input widget gains focus
//! 2. Receiving preedit (composition) events as the user types
//! 3. Displaying the preedit text with visual distinction (e.g., underline)
//! 4. Receiving commit events when the user finalizes their input
//! 5. Disabling IME when the widget loses focus
//!
//! # Usage
//!
//! ```ignore
//! use horizon_lattice::widget::ime::{ImeInputHandler, ImeState};
//!
//! let mut handler = ImeInputHandler::new();
//!
//! // When the widget gains focus and wants IME input:
//! handler.set_enabled(true);
//!
//! // When receiving a winit IME event:
//! if let Some(widget_event) = handler.process_ime_event(&winit_ime_event) {
//!     // Dispatch widget_event to the focused widget
//! }
//! ```
//!
//! # Platform Notes
//!
//! - **macOS**: IME must be enabled to receive dead-key sequences for accented characters
//! - **Windows**: Full IME support for all input methods
//! - **Linux/X11**: Only position is used in cursor area (size is ignored)
//! - **Linux/Wayland**: No explicit preedit clearing before new preedit

use winit::event::Ime;

use super::events::{
    ImeCommitEvent, ImeDisabledEvent, ImeEnabledEvent, ImePreeditEvent, WidgetEvent,
};

/// Purpose hint for the Input Method Editor.
///
/// This provides hints to the IME about what kind of content is being entered,
/// which may affect the behavior of the input method or virtual keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImePurpose {
    /// Normal text input (default).
    #[default]
    Normal,
    /// Password input - may disable IME candidate display.
    Password,
    /// Terminal input - may show additional buttons on Wayland OSK.
    Terminal,
}

impl ImePurpose {
    /// Convert to winit's ImePurpose.
    pub fn to_winit(self) -> winit::window::ImePurpose {
        match self {
            ImePurpose::Normal => winit::window::ImePurpose::Normal,
            ImePurpose::Password => winit::window::ImePurpose::Password,
            ImePurpose::Terminal => winit::window::ImePurpose::Terminal,
        }
    }
}

/// Current state of the IME.
#[derive(Debug, Clone, Default)]
pub struct ImeState {
    /// Whether IME is currently enabled.
    enabled: bool,
    /// The current preedit (composition) text, if any.
    preedit_text: Option<String>,
    /// Cursor position within the preedit text (byte indices).
    preedit_cursor: Option<(usize, usize)>,
}

impl ImeState {
    /// Create a new IME state with IME disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if IME is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the current preedit text, if any.
    pub fn preedit_text(&self) -> Option<&str> {
        self.preedit_text.as_deref()
    }

    /// Get the cursor position within the preedit text.
    pub fn preedit_cursor(&self) -> Option<(usize, usize)> {
        self.preedit_cursor
    }

    /// Check if there is an active composition.
    pub fn has_preedit(&self) -> bool {
        self.preedit_text.is_some()
    }

    /// Clear any active preedit.
    fn clear_preedit(&mut self) {
        self.preedit_text = None;
        self.preedit_cursor = None;
    }
}

/// Handler for IME input that manages composition state.
///
/// This struct provides a stateful interface for converting winit IME
/// events into widget events, tracking composition state across events.
#[derive(Debug, Default)]
pub struct ImeInputHandler {
    /// Current IME state.
    state: ImeState,
}

impl ImeInputHandler {
    /// Create a new IME input handler with IME disabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the current IME state.
    pub fn state(&self) -> &ImeState {
        &self.state
    }

    /// Check if IME is enabled.
    pub fn is_enabled(&self) -> bool {
        self.state.enabled
    }

    /// Get the current preedit text, if any.
    pub fn preedit_text(&self) -> Option<&str> {
        self.state.preedit_text()
    }

    /// Get the cursor position within the preedit text.
    pub fn preedit_cursor(&self) -> Option<(usize, usize)> {
        self.state.preedit_cursor()
    }

    /// Check if there is an active composition.
    pub fn has_preedit(&self) -> bool {
        self.state.has_preedit()
    }

    /// Process a winit IME event and return the corresponding widget event.
    ///
    /// This method updates the internal state and returns the appropriate
    /// widget event to dispatch to the focused widget.
    ///
    /// # Arguments
    ///
    /// * `ime` - The winit IME event to process
    ///
    /// # Returns
    ///
    /// The widget event to dispatch, or `None` if no event should be sent.
    pub fn process_ime_event(&mut self, ime: &Ime) -> Option<WidgetEvent> {
        match ime {
            Ime::Enabled => {
                self.state.enabled = true;
                Some(WidgetEvent::ImeEnabled(ImeEnabledEvent::new()))
            }
            Ime::Preedit(text, cursor) => {
                if text.is_empty() {
                    // Preedit cleared
                    self.state.clear_preedit();
                    Some(WidgetEvent::ImePreedit(ImePreeditEvent::cleared()))
                } else {
                    // Update preedit
                    self.state.preedit_text = Some(text.clone());
                    self.state.preedit_cursor = *cursor;
                    Some(WidgetEvent::ImePreedit(ImePreeditEvent::new(
                        text.clone(),
                        *cursor,
                    )))
                }
            }
            Ime::Commit(text) => {
                // Clear preedit state on commit
                self.state.clear_preedit();
                Some(WidgetEvent::ImeCommit(ImeCommitEvent::new(text.clone())))
            }
            Ime::Disabled => {
                self.state.enabled = false;
                self.state.clear_preedit();
                Some(WidgetEvent::ImeDisabled(ImeDisabledEvent::new()))
            }
        }
    }

    /// Manually set the IME enabled state.
    ///
    /// This is used when programmatically enabling/disabling IME,
    /// for example when a text widget gains or loses focus.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.state.enabled = enabled;
        if !enabled {
            self.state.clear_preedit();
        }
    }

    /// Clear any active composition.
    ///
    /// This can be used when the widget loses focus or when
    /// the composition should be cancelled.
    pub fn clear_preedit(&mut self) {
        self.state.clear_preedit();
    }

    /// Reset the handler to its initial state.
    pub fn reset(&mut self) {
        self.state = ImeState::default();
    }
}

/// An IME event that can be enabled, preedit, commit, or disabled.
#[derive(Debug, Clone)]
pub enum ImeEvent {
    /// IME was enabled.
    Enabled,
    /// Preedit (composition) text update.
    Preedit {
        /// The preedit text. Empty string means cleared.
        text: String,
        /// Cursor position as byte indices.
        cursor: Option<(usize, usize)>,
    },
    /// Text was committed (finalized).
    Commit(String),
    /// IME was disabled.
    Disabled,
}

impl ImeEvent {
    /// Convert this IME event into a WidgetEvent.
    pub fn into_widget_event(self) -> WidgetEvent {
        match self {
            ImeEvent::Enabled => WidgetEvent::ImeEnabled(ImeEnabledEvent::new()),
            ImeEvent::Preedit { text, cursor } => {
                WidgetEvent::ImePreedit(ImePreeditEvent::new(text, cursor))
            }
            ImeEvent::Commit(text) => WidgetEvent::ImeCommit(ImeCommitEvent::new(text)),
            ImeEvent::Disabled => WidgetEvent::ImeDisabled(ImeDisabledEvent::new()),
        }
    }

    /// Create from a winit IME event.
    pub fn from_winit(ime: &Ime) -> Self {
        match ime {
            Ime::Enabled => ImeEvent::Enabled,
            Ime::Preedit(text, cursor) => ImeEvent::Preedit {
                text: text.clone(),
                cursor: *cursor,
            },
            Ime::Commit(text) => ImeEvent::Commit(text.clone()),
            Ime::Disabled => ImeEvent::Disabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ime_handler_initial_state() {
        let handler = ImeInputHandler::new();
        assert!(!handler.is_enabled());
        assert!(!handler.has_preedit());
        assert!(handler.preedit_text().is_none());
    }

    #[test]
    fn test_ime_enabled_event() {
        let mut handler = ImeInputHandler::new();
        let event = handler.process_ime_event(&Ime::Enabled);

        assert!(handler.is_enabled());
        assert!(matches!(event, Some(WidgetEvent::ImeEnabled(_))));
    }

    #[test]
    fn test_ime_preedit_event() {
        let mut handler = ImeInputHandler::new();
        handler.process_ime_event(&Ime::Enabled);

        let event = handler.process_ime_event(&Ime::Preedit("你好".to_string(), Some((0, 6))));

        assert!(handler.has_preedit());
        assert_eq!(handler.preedit_text(), Some("你好"));
        assert_eq!(handler.preedit_cursor(), Some((0, 6)));
        assert!(matches!(event, Some(WidgetEvent::ImePreedit(_))));
    }

    #[test]
    fn test_ime_preedit_cleared() {
        let mut handler = ImeInputHandler::new();
        handler.process_ime_event(&Ime::Enabled);
        handler.process_ime_event(&Ime::Preedit("你好".to_string(), Some((0, 6))));

        let event = handler.process_ime_event(&Ime::Preedit(String::new(), None));

        assert!(!handler.has_preedit());
        assert!(handler.preedit_text().is_none());
        if let Some(WidgetEvent::ImePreedit(e)) = event {
            assert!(e.is_cleared());
        } else {
            panic!("Expected ImePreedit event");
        }
    }

    #[test]
    fn test_ime_commit_event() {
        let mut handler = ImeInputHandler::new();
        handler.process_ime_event(&Ime::Enabled);
        handler.process_ime_event(&Ime::Preedit("你好".to_string(), Some((0, 6))));

        let event = handler.process_ime_event(&Ime::Commit("你好".to_string()));

        assert!(!handler.has_preedit());
        if let Some(WidgetEvent::ImeCommit(e)) = event {
            assert_eq!(e.text, "你好");
        } else {
            panic!("Expected ImeCommit event");
        }
    }

    #[test]
    fn test_ime_disabled_event() {
        let mut handler = ImeInputHandler::new();
        handler.process_ime_event(&Ime::Enabled);
        handler.process_ime_event(&Ime::Preedit("你好".to_string(), Some((0, 6))));

        let event = handler.process_ime_event(&Ime::Disabled);

        assert!(!handler.is_enabled());
        assert!(!handler.has_preedit());
        assert!(matches!(event, Some(WidgetEvent::ImeDisabled(_))));
    }

    #[test]
    fn test_ime_purpose_default() {
        let purpose = ImePurpose::default();
        assert_eq!(purpose, ImePurpose::Normal);
    }

    #[test]
    fn test_ime_state_new() {
        let state = ImeState::new();
        assert!(!state.is_enabled());
        assert!(!state.has_preedit());
        assert!(state.preedit_text().is_none());
        assert!(state.preedit_cursor().is_none());
    }

    #[test]
    fn test_manual_enable_disable() {
        let mut handler = ImeInputHandler::new();

        handler.set_enabled(true);
        assert!(handler.is_enabled());

        handler.set_enabled(false);
        assert!(!handler.is_enabled());
    }

    #[test]
    fn test_disable_clears_preedit() {
        let mut handler = ImeInputHandler::new();
        handler.process_ime_event(&Ime::Enabled);
        handler.process_ime_event(&Ime::Preedit("test".to_string(), Some((0, 4))));

        handler.set_enabled(false);

        assert!(!handler.has_preedit());
    }

    #[test]
    fn test_reset() {
        let mut handler = ImeInputHandler::new();
        handler.process_ime_event(&Ime::Enabled);
        handler.process_ime_event(&Ime::Preedit("test".to_string(), Some((0, 4))));

        handler.reset();

        assert!(!handler.is_enabled());
        assert!(!handler.has_preedit());
    }

    #[test]
    fn test_ime_event_from_winit() {
        let winit_enabled = Ime::Enabled;
        let event = ImeEvent::from_winit(&winit_enabled);
        assert!(matches!(event, ImeEvent::Enabled));

        let winit_preedit = Ime::Preedit("test".to_string(), Some((0, 4)));
        let event = ImeEvent::from_winit(&winit_preedit);
        if let ImeEvent::Preedit { text, cursor } = event {
            assert_eq!(text, "test");
            assert_eq!(cursor, Some((0, 4)));
        } else {
            panic!("Expected Preedit event");
        }

        let winit_commit = Ime::Commit("final".to_string());
        let event = ImeEvent::from_winit(&winit_commit);
        if let ImeEvent::Commit(text) = event {
            assert_eq!(text, "final");
        } else {
            panic!("Expected Commit event");
        }

        let winit_disabled = Ime::Disabled;
        let event = ImeEvent::from_winit(&winit_disabled);
        assert!(matches!(event, ImeEvent::Disabled));
    }
}
