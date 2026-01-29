//! Prelude module for Horizon Lattice.
//!
//! This module re-exports the most commonly used types for convenient importing:
//!
//! ```ignore
//! use horizon_lattice::prelude::*;
//! ```
//!
//! This provides access to:
//! - Application lifecycle (`Application`)
//! - Signal/slot system (`Signal`, `Property`, `ConnectionType`)
//! - Widget foundation (`Widget`, `WidgetBase`, `PaintContext`)
//! - Common widgets (`PushButton`, `Label`, `LineEdit`, etc.)
//! - Layout system (`HBoxLayout`, `VBoxLayout`, `GridLayout`, etc.)
//! - Geometry types (`Point`, `Size`, `Rect`, `Color`)

// ============================================================================
// Core Application
// ============================================================================

pub use crate::Application;

// ============================================================================
// Signal/Slot and Property System
// ============================================================================

pub use crate::signal::{ConnectionId, ConnectionType, Signal};
pub use crate::property::{Binding, Property, ReadOnlyProperty};

// ============================================================================
// Object System
// ============================================================================

pub use crate::object::{Object, ObjectBase, ObjectId};

// ============================================================================
// Widget Foundation
// ============================================================================

pub use crate::widget::{
    AsWidget, PaintContext, SizeHint, SizePolicy, SizePolicyPair, Widget, WidgetBase,
};

// ============================================================================
// Common Widgets
// ============================================================================

// Buttons (from widget::widgets)
pub use crate::widget::widgets::{
    AbstractButton, ButtonVariant, CheckBox, PushButton, RadioButton, ToolButton,
};

// Text Display (from widget re-export)
pub use crate::widget::Label;

// Text Input (from widget::widgets)
pub use crate::widget::widgets::{LineEdit, PlainTextEdit, TextEdit};

// Numeric Input (from widget::widgets)
pub use crate::widget::widgets::{DoubleSpinBox, Slider, SpinBox};

// Selection (from widget::widgets)
pub use crate::widget::widgets::ComboBox;

// Progress and Status (from widget re-export)
pub use crate::widget::ProgressBar;

// ============================================================================
// Container Widgets
// ============================================================================

pub use crate::widget::widgets::{
    ContainerWidget, Dialog, Frame, GroupBox, MainWindow, Popup, StackedWidget, TabWidget, Window,
};
pub use crate::widget::widgets::{Separator, Spacer};
pub use crate::widget::ScrollArea;

// ============================================================================
// Layout System
// ============================================================================

pub use crate::widget::layout::{
    AnchorLayout, BoxLayout, FlowLayout, FormLayout, GridLayout, HBoxLayout, LayoutKind,
    StackLayout, VBoxLayout,
};
pub use crate::widget::layout::{Alignment, Orientation};
pub use crate::widget::{ContentMargins, Layout, LayoutBase, LayoutItem, SpacerItem};

// ============================================================================
// Geometry and Graphics Types
// ============================================================================

pub use crate::render::{Color, Point, Rect, RoundedRect, Size};

// ============================================================================
// Event Types
// ============================================================================

pub use crate::widget::{Key, KeyboardModifiers, MouseButton, WidgetEvent};

#[cfg(test)]
mod tests {
    #![allow(unused)]
    use super::*;

    /// Verify that all prelude exports are accessible and the types exist.
    /// This test uses type assertions rather than instantiation to avoid
    /// requiring runtime initialization (object registry, event loop, etc.)
    #[test]
    fn test_prelude_types_exist() {
        // Signal/Property types
        let _signal: Signal<i32> = Signal::new();
        let _property: Property<String> = Property::new(String::new());

        // Verify layout types exist (HBoxLayout/VBoxLayout are aliases for BoxLayout)
        let _hbox: HBoxLayout = BoxLayout::horizontal();
        let _vbox: VBoxLayout = BoxLayout::vertical();
        let _grid = GridLayout::new();
        let _form = FormLayout::new();

        // Verify geometry types
        let _point = Point::new(0.0, 0.0);
        let _size = Size::new(100.0, 100.0);
        let _rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        let _color = Color::from_rgb8(255, 255, 255);
    }

    /// Verify widget types are accessible (compile-time check only).
    /// These functions verify the types exist without calling them.
    #[allow(dead_code)]
    fn _widget_types_check() {
        // Widget trait bound check
        fn _takes_widget<W: Widget>(_w: &W) {}

        // Type existence checks (not called, just for compile-time verification)
        fn _window(_title: &str) -> Window {
            Window::new(_title)
        }
        fn _dialog(_title: &str) -> Dialog {
            Dialog::new(_title)
        }
        fn _button(_text: &str) -> PushButton {
            PushButton::new(_text)
        }
        fn _label(_text: &str) -> Label {
            Label::new(_text)
        }
        fn _checkbox(_text: &str) -> CheckBox {
            CheckBox::new(_text)
        }
        fn _line_edit() -> LineEdit {
            LineEdit::new()
        }
    }
}
