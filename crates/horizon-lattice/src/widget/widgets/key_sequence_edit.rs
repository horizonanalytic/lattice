//! Key sequence edit widget implementation.
//!
//! This module provides [`KeySequenceEdit`], a widget for capturing and editing
//! keyboard shortcuts.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::KeySequenceEdit;
//! use horizon_lattice::widget::KeySequence;
//!
//! // Create a key sequence editor
//! let mut editor = KeySequenceEdit::new();
//!
//! // Optionally set an initial key sequence
//! editor.set_key_sequence(Some(KeySequence::ctrl(Key::S)));
//!
//! // Connect to the changed signal
//! editor.key_sequence_changed.connect(|seq| {
//!     if let Some(seq) = seq {
//!         println!("New shortcut: {}", seq);
//!     } else {
//!         println!("Shortcut cleared");
//!     }
//! });
//! ```

use horizon_lattice_core::{Object, ObjectId, Signal};
use horizon_lattice_render::{
    Color, Font, FontSystem, Point, Rect, Renderer, RoundedRect, Stroke, TextLayout, TextRenderer,
};

use crate::widget::KeySequence;
use crate::widget::{
    FocusOutEvent, FocusPolicy, Key, KeyPressEvent, KeyboardModifiers, MouseButton,
    MousePressEvent, MouseReleaseEvent, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget,
    WidgetBase, WidgetEvent,
};

/// A widget for capturing and editing keyboard shortcuts.
///
/// KeySequenceEdit allows users to define keyboard shortcuts by pressing
/// key combinations. It displays the current shortcut and provides a clear button.
///
/// # Behavior
///
/// When the widget has focus:
/// - Press any key combination (e.g., Ctrl+S) to set the shortcut
/// - Press Escape to cancel editing without changing the value
/// - Press Backspace or Delete to clear the shortcut
/// - Click the clear button (×) to clear the shortcut
///
/// # Signals
///
/// - `key_sequence_changed(Option<KeySequence>)`: Emitted when the shortcut changes
pub struct KeySequenceEdit {
    /// Widget base.
    base: WidgetBase,

    /// Current key sequence (None if cleared).
    key_sequence: Option<KeySequence>,

    /// Whether the widget is currently recording a key sequence.
    recording: bool,

    /// Current modifiers being held during recording.
    current_modifiers: KeyboardModifiers,

    /// Placeholder text shown when no sequence is set.
    placeholder: String,

    /// Hovered state.
    hovered: bool,

    /// Whether the clear button is hovered.
    clear_button_hovered: bool,

    /// Whether the clear button is pressed.
    clear_button_pressed: bool,

    /// Background color.
    background_color: Color,

    /// Border color.
    border_color: Color,

    /// Focus border color.
    focus_border_color: Color,

    /// Recording indicator color.
    recording_color: Color,

    /// Text color.
    text_color: Color,

    /// Placeholder text color.
    placeholder_color: Color,

    /// Clear button color.
    clear_button_color: Color,

    /// Border radius.
    border_radius: f32,

    /// Clear button size.
    clear_button_size: f32,

    /// Signal emitted when the key sequence changes.
    pub key_sequence_changed: Signal<Option<KeySequence>>,
}

impl KeySequenceEdit {
    /// Create a new key sequence edit widget.
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);
        base.set_size_policy(SizePolicyPair::new(
            SizePolicy::Preferred,
            SizePolicy::Fixed,
        ));

        Self {
            base,
            key_sequence: None,
            recording: false,
            current_modifiers: KeyboardModifiers::NONE,
            placeholder: "Press keys...".to_string(),
            hovered: false,
            clear_button_hovered: false,
            clear_button_pressed: false,
            background_color: Color::WHITE,
            border_color: Color::from_rgb8(180, 180, 180),
            focus_border_color: Color::from_rgb8(50, 130, 200),
            recording_color: Color::from_rgba8(50, 130, 200, 30),
            text_color: Color::from_rgb8(30, 30, 30),
            placeholder_color: Color::from_rgb8(150, 150, 150),
            clear_button_color: Color::from_rgb8(120, 120, 120),
            border_radius: 4.0,
            clear_button_size: 16.0,
            key_sequence_changed: Signal::new(),
        }
    }

    // =========================================================================
    // Key Sequence
    // =========================================================================

    /// Get the current key sequence.
    pub fn key_sequence(&self) -> Option<&KeySequence> {
        self.key_sequence.as_ref()
    }

    /// Set the key sequence.
    pub fn set_key_sequence(&mut self, sequence: Option<KeySequence>) {
        if self.key_sequence != sequence {
            self.key_sequence = sequence.clone();
            self.key_sequence_changed.emit(sequence);
            self.base.update();
        }
    }

    /// Set the key sequence using builder pattern.
    pub fn with_key_sequence(mut self, sequence: KeySequence) -> Self {
        self.key_sequence = Some(sequence);
        self
    }

    /// Clear the key sequence.
    pub fn clear(&mut self) {
        self.set_key_sequence(None);
    }

    // =========================================================================
    // Placeholder
    // =========================================================================

    /// Get the placeholder text.
    pub fn placeholder(&self) -> &str {
        &self.placeholder
    }

    /// Set the placeholder text.
    pub fn set_placeholder(&mut self, text: impl Into<String>) {
        self.placeholder = text.into();
        self.base.update();
    }

    /// Set the placeholder text using builder pattern.
    pub fn with_placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    // =========================================================================
    // Recording State
    // =========================================================================

    /// Check if the widget is currently recording.
    pub fn is_recording(&self) -> bool {
        self.recording
    }

    /// Start recording mode.
    fn start_recording(&mut self) {
        if !self.recording {
            self.recording = true;
            self.current_modifiers = KeyboardModifiers::NONE;
            self.base.update();
        }
    }

    /// Stop recording mode.
    fn stop_recording(&mut self) {
        if self.recording {
            self.recording = false;
            self.current_modifiers = KeyboardModifiers::NONE;
            self.base.update();
        }
    }

    // =========================================================================
    // Geometry Helpers
    // =========================================================================

    /// Get the clear button rectangle.
    fn clear_button_rect(&self) -> Option<Rect> {
        self.key_sequence.as_ref()?;

        let rect = self.base.rect();
        let padding = 4.0;
        let size = self.clear_button_size;
        let x = rect.right() - size - padding;
        let y = rect.center().y - size / 2.0;

        Some(Rect::new(x, y, size, size))
    }

    /// Check if a point is inside the clear button.
    fn is_over_clear_button(&self, pos: Point) -> bool {
        if let Some(button_rect) = self.clear_button_rect() {
            button_rect.contains(pos)
        } else {
            false
        }
    }

    // =========================================================================
    // Event Handling
    // =========================================================================

    fn handle_mouse_press(&mut self, event: &MousePressEvent) -> bool {
        if event.button == MouseButton::Left {
            if self.is_over_clear_button(event.local_pos) {
                self.clear_button_pressed = true;
                self.base.update();
                return true;
            }
            // Click on widget starts recording
            self.start_recording();
            return true;
        }
        false
    }

    fn handle_mouse_release(&mut self, event: &MouseReleaseEvent) -> bool {
        if event.button == MouseButton::Left && self.clear_button_pressed {
            self.clear_button_pressed = false;
            if self.is_over_clear_button(event.local_pos) {
                self.clear();
            }
            self.base.update();
            return true;
        }
        false
    }

    fn handle_mouse_move(&mut self, pos: Point) {
        let over_clear = self.is_over_clear_button(pos);
        if self.clear_button_hovered != over_clear {
            self.clear_button_hovered = over_clear;
            self.base.update();
        }
    }

    fn handle_key_press(&mut self, event: &KeyPressEvent) -> bool {
        // Update current modifiers
        self.current_modifiers = event.modifiers;

        // Handle special keys
        match event.key {
            Key::Escape => {
                // Cancel recording without changing value
                self.stop_recording();
                return true;
            }
            Key::Backspace | Key::Delete => {
                // Clear the sequence
                self.clear();
                self.stop_recording();
                return true;
            }
            _ => {}
        }

        // Ignore modifier-only key presses - wait for a real key
        if event.key.is_modifier() {
            self.base.update();
            return true;
        }

        // Capture the key sequence
        let sequence = KeySequence::new(event.key, event.modifiers);
        self.set_key_sequence(Some(sequence));
        self.stop_recording();
        true
    }

    fn handle_focus_out(&mut self, _event: &FocusOutEvent) {
        self.stop_recording();
    }

    // =========================================================================
    // Painting
    // =========================================================================

    fn get_display_text(&self) -> String {
        if self.recording {
            // Show current modifiers being held
            if self.current_modifiers.any() {
                let mut parts = Vec::new();
                if self.current_modifiers.control {
                    parts.push("Ctrl");
                }
                if self.current_modifiers.alt {
                    parts.push("Alt");
                }
                if self.current_modifiers.shift {
                    parts.push("Shift");
                }
                if self.current_modifiers.meta {
                    parts.push("Meta");
                }
                parts.push("...");
                return parts.join("+");
            }
            return self.placeholder.clone();
        }

        if let Some(seq) = &self.key_sequence {
            seq.to_string()
        } else {
            self.placeholder.clone()
        }
    }

    fn paint_clear_button(&self, ctx: &mut PaintContext<'_>) {
        if let Some(button_rect) = self.clear_button_rect() {
            let color = if self.clear_button_pressed {
                Color::from_rgb8(80, 80, 80)
            } else if self.clear_button_hovered {
                Color::from_rgb8(100, 100, 100)
            } else {
                self.clear_button_color
            };

            // Draw × symbol
            let padding = 4.0;
            let x1 = button_rect.left() + padding;
            let y1 = button_rect.top() + padding;
            let x2 = button_rect.right() - padding;
            let y2 = button_rect.bottom() - padding;

            let stroke = Stroke::new(color, 2.0);
            ctx.renderer()
                .draw_line(Point::new(x1, y1), Point::new(x2, y2), &stroke);
            ctx.renderer()
                .draw_line(Point::new(x2, y1), Point::new(x1, y2), &stroke);
        }
    }
}

impl Default for KeySequenceEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl Object for KeySequenceEdit {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for KeySequenceEdit {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Width should accommodate typical shortcuts like "Ctrl+Shift+F12"
        // plus clear button
        SizeHint::from_dimensions(150.0, 28.0).with_minimum_dimensions(100.0, 24.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = self.base.rect();
        let has_focus = self.base.has_focus();

        // Draw background
        let bg_color = if self.recording {
            self.recording_color
        } else {
            self.background_color
        };
        let rounded = RoundedRect::new(rect, self.border_radius);
        ctx.renderer().fill_rounded_rect(rounded, bg_color);

        // Draw border
        let border_color = if has_focus {
            self.focus_border_color
        } else if self.hovered {
            Color::from_rgb8(140, 140, 140)
        } else {
            self.border_color
        };
        let stroke = Stroke::new(border_color, if has_focus { 2.0 } else { 1.0 });
        ctx.renderer().stroke_rounded_rect(rounded, &stroke);

        // Calculate text area (leave room for clear button)
        let text_padding = 8.0;
        let clear_button_space = if self.key_sequence.is_some() {
            self.clear_button_size + 8.0
        } else {
            0.0
        };
        let text_rect = Rect::new(
            rect.left() + text_padding,
            rect.top(),
            rect.width() - text_padding * 2.0 - clear_button_space,
            rect.height(),
        );

        // Draw text
        let display_text = self.get_display_text();
        let text_color = if self.key_sequence.is_none() && !self.recording {
            self.placeholder_color
        } else {
            self.text_color
        };

        // Render text using the text layout system
        let mut font_system = FontSystem::new();
        let font = Font::default().with_size(14.0);
        let layout = TextLayout::new(&mut font_system, &display_text, &font);

        // Center text vertically
        let text_y = text_rect.top() + (text_rect.height() - layout.height()) / 2.0;
        let text_pos = Point::new(text_rect.left(), text_y);

        if let Ok(mut text_renderer) = TextRenderer::new() {
            let _ = text_renderer.prepare_layout(&mut font_system, &layout, text_pos, text_color);
        }

        // Draw clear button if sequence is set
        self.paint_clear_button(ctx);
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                if self.handle_mouse_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if self.handle_mouse_release(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::MouseMove(e) => {
                self.handle_mouse_move(e.local_pos);
            }
            WidgetEvent::KeyPress(e) => {
                if self.handle_key_press(e) {
                    event.accept();
                    return true;
                }
            }
            WidgetEvent::FocusIn(_) => {
                self.start_recording();
                self.base.update();
            }
            WidgetEvent::FocusOut(e) => {
                self.handle_focus_out(e);
            }
            WidgetEvent::Enter(_) => {
                self.hovered = true;
                self.base.update();
            }
            WidgetEvent::Leave(_) => {
                self.hovered = false;
                self.clear_button_hovered = false;
                self.clear_button_pressed = false;
                self.base.update();
            }
            _ => {}
        }
        false
    }
}

// Thread safety assertion
static_assertions::assert_impl_all!(KeySequenceEdit: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_lattice_core::init_global_registry;

    #[test]
    fn test_new_has_no_sequence() {
        init_global_registry();
        let edit = KeySequenceEdit::new();
        assert!(edit.key_sequence().is_none());
        assert!(!edit.is_recording());
    }

    #[test]
    fn test_set_key_sequence() {
        init_global_registry();
        let mut edit = KeySequenceEdit::new();
        let seq = KeySequence::ctrl(Key::S);
        edit.set_key_sequence(Some(seq.clone()));
        assert_eq!(edit.key_sequence(), Some(&seq));
    }

    #[test]
    fn test_clear() {
        init_global_registry();
        let mut edit = KeySequenceEdit::new();
        edit.set_key_sequence(Some(KeySequence::ctrl(Key::S)));
        edit.clear();
        assert!(edit.key_sequence().is_none());
    }

    #[test]
    fn test_with_key_sequence_builder() {
        init_global_registry();
        let seq = KeySequence::alt(Key::F4);
        let edit = KeySequenceEdit::new().with_key_sequence(seq.clone());
        assert_eq!(edit.key_sequence(), Some(&seq));
    }

    #[test]
    fn test_placeholder() {
        init_global_registry();
        let mut edit = KeySequenceEdit::new();
        assert_eq!(edit.placeholder(), "Press keys...");

        edit.set_placeholder("Enter shortcut");
        assert_eq!(edit.placeholder(), "Enter shortcut");
    }

    #[test]
    fn test_display_text_with_sequence() {
        init_global_registry();
        let mut edit = KeySequenceEdit::new();
        edit.set_key_sequence(Some(KeySequence::ctrl_shift(Key::N)));
        assert_eq!(edit.get_display_text(), "Ctrl+Shift+N");
    }

    #[test]
    fn test_display_text_placeholder() {
        init_global_registry();
        let edit = KeySequenceEdit::new();
        assert_eq!(edit.get_display_text(), "Press keys...");
    }
}
