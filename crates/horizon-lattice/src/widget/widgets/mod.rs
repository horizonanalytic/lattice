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
//! - [`ProgressBar`]: Progress indicator widget
//! - [`ContainerWidget`]: Generic container widget with layout support
//! - [`Frame`]: Container widget with border decoration
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

mod abstract_button;
mod button_group;
mod checkbox;
mod container;
mod dock_widget;
mod frame;
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
mod spacer;
mod splitter;
mod stacked_widget;
mod tab_bar;
mod tab_widget;
mod tool_box;
mod tool_button;
mod window;

pub use abstract_button::{AbstractButton, ButtonVariant};
pub use button_group::ButtonGroup;
pub use checkbox::{CheckBox, CheckState};
pub use container::ContainerWidget;
pub use dock_widget::{DockArea, DockAreas, DockWidget, DockWidgetFeatures};
pub use frame::{Frame, FrameShadow, FrameShape};
pub use label::{ElideMode, Label};
pub use line_edit::{EchoMode, LineEdit};
pub use main_window::MainWindow;
pub use popup::{Popup, PopupFlags, PopupPlacement};
pub use progress_bar::{Orientation, ProgressBar};
pub use push_button::PushButton;
pub use radio_button::RadioButton;
pub use radio_group::RadioGroup;
pub use scroll_area::{ScrollArea, ScrollBarPolicy};
pub use scroll_bar::ScrollBar;
pub use separator::{Separator, SeparatorOrientation};
pub use spacer::Spacer;
pub use splitter::Splitter;
pub use stacked_widget::StackedWidget;
pub use tab_bar::{TabBar, TabPosition};
pub use tab_widget::TabWidget;
pub use tool_box::ToolBox;
pub use tool_button::{ToolButton, ToolButtonPopupMode, ToolButtonStyle};
pub use window::{Window, WindowFlags, WindowModality, WindowState};
