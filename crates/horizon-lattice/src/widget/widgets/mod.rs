//! Standard widgets for Horizon Lattice.
//!
//! This module provides common UI widgets:
//!
//! - [`Label`]: Text display widget
//! - [`PushButton`]: Standard clickable button
//! - [`ToolButton`]: Icon-focused button for toolbars with menu support
//! - [`ButtonVariant`]: Visual variants for buttons (Primary, Secondary, Danger, Flat, Outlined)
//! - [`CheckBox`]: Checkbox for boolean/tri-state selection
//! - [`RadioButton`]: Radio button for exclusive selection
//! - [`RadioGroup`]: Visual container for radio buttons with automatic exclusivity
//! - [`ButtonGroup`]: Non-visual container for exclusive button groups
//! - [`AbstractButton`]: Base for all button widgets
//! - [`LineEdit`]: Single-line text input
//! - [`TextEdit`]: Multi-line text editor with scrolling and word wrap
//! - [`PlainTextEdit`]: Plain text editor optimized for large documents with syntax highlighting
//! - [`SpinBox`]: Integer input with increment/decrement buttons
//! - [`DoubleSpinBox`]: Floating-point input with increment/decrement buttons
//! - [`ProgressBar`]: Progress indicator widget
//! - [`ContainerWidget`]: Generic container widget with layout support
//! - [`Frame`]: Container widget with border decoration
//! - [`GroupBox`]: Titled container with optional checkbox mode
//! - [`Separator`]: Visual dividing line
//! - [`Spacer`]: Invisible widget for layout spacing
//! - [`ScrollBar`]: Standalone scrollbar widget
//! - [`ScrollArea`]: Scrollable container widget
//! - [`Splitter`]: Resizable pane container
//! - [`TabBar`]: Standalone tab bar widget
//! - [`TabWidget`]: Tabbed page container
//! - [`StackedWidget`]: Container showing one child at a time
//! - [`ToolBox`]: Accordion-style container with expandable pages
//! - [`DockWidget`]: Dockable panel widget
//! - [`MainWindow`]: Main application window with dock areas
//! - [`Popup`]: Temporary floating container widget
//! - [`Window`]: Top-level window widget
//! - [`ColorButton`]: Button that displays a color swatch
//! - [`ColorPicker`]: Inline HSV color picker with saturation/value square and hue bar
//! - [`FontComboBox`]: Dropdown for selecting font families with preview
//! - [`KeySequenceEdit`]: Keyboard shortcut capture and editing widget

mod abstract_button;
mod button_group;
mod calendar;
mod checkbox;
mod color_button;
mod color_picker;
mod combo_box;
mod container;
mod date_edit;
mod date_time_edit;
mod dial;
mod dock_widget;
mod double_spin_box;
mod font_combo_box;
mod frame;
mod group_box;
mod label;
mod line_edit;
mod main_window;
mod popup;
mod progress_bar;
mod push_button;
mod radio_button;
mod radio_group;
mod scroll_area;
mod scroll_bar;
mod separator;
mod slider;
mod spacer;
mod spin_box;
mod splitter;
mod stacked_widget;
mod tab_bar;
mod tab_widget;
mod plain_text_edit;
mod text_edit;
mod time_edit;
mod tool_box;
mod tool_button;
mod window;
mod key_sequence_edit;

pub use abstract_button::{AbstractButton, ButtonVariant};
pub use button_group::ButtonGroup;
pub use calendar::CalendarWidget;
pub use checkbox::{CheckBox, CheckState};
pub use color_button::ColorButton;
pub use color_picker::ColorPicker;
pub use combo_box::{
    ComboBox, ComboBoxItem, ComboBoxItemDelegate, ComboBoxModel, DefaultComboBoxDelegate,
    IconListComboModel, StringListComboModel,
};
pub use container::ContainerWidget;
pub use date_edit::{DateEdit, DateFormat};
pub use date_time_edit::DateTimeEdit;
pub use dial::Dial;
pub use dock_widget::{DockArea, DockAreas, DockWidget, DockWidgetFeatures};
pub use frame::{Frame, FrameShadow, FrameShape};
pub use group_box::GroupBox;
pub use label::{ElideMode, Label};
pub use line_edit::{EchoMode, LineEdit};
// Re-export validation types for convenience
pub use super::validator::{
    CustomValidator, DoubleValidator, IntValidator, RegexValidator, ValidationState, Validator,
};
// Re-export completer types for convenience
pub use super::completer::{CaseSensitivity, Completer, CompleterModel, StringListModel};
pub use main_window::MainWindow;
pub use popup::{Popup, PopupFlags, PopupPlacement};
pub use progress_bar::{Orientation, ProgressBar};
pub use push_button::PushButton;
pub use radio_button::RadioButton;
pub use radio_group::RadioGroup;
pub use scroll_area::{ScrollArea, ScrollBarPolicy};
pub use scroll_bar::ScrollBar;
pub use separator::{Separator, SeparatorOrientation};
pub use slider::{Slider, TickPosition};
pub use spacer::Spacer;
pub use splitter::Splitter;
pub use stacked_widget::StackedWidget;
pub use tab_bar::{TabBar, TabPosition};
pub use tab_widget::TabWidget;
pub use plain_text_edit::{HighlightSpan, LineNumberConfig, PlainTextEdit, SyntaxHighlighter};
pub use text_edit::{TextEdit, TextWrapMode};
pub use double_spin_box::DoubleSpinBox;
pub use font_combo_box::{FontComboBox, FontFilter};
pub use spin_box::SpinBox;
pub use time_edit::{TimeEdit, TimeFormat};
pub use tool_box::ToolBox;
pub use tool_button::{ToolButton, ToolButtonPopupMode, ToolButtonStyle};
pub use window::{Window, WindowFlags, WindowModality, WindowState};
pub use key_sequence_edit::KeySequenceEdit;
