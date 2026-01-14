//! Standard widgets for Horizon Lattice.
//!
//! This module provides common UI widgets:
//!
//! - [`Label`]: Text display widget
//! - [`PushButton`]: Standard clickable button
//! - [`CheckBox`]: Checkbox for boolean/tri-state selection
//! - [`RadioButton`]: Radio button for exclusive selection
//! - [`ButtonGroup`]: Non-visual container for exclusive button groups
//! - [`AbstractButton`]: Base for all button widgets
//! - [`LineEdit`]: Single-line text input
//! - [`ProgressBar`]: Progress indicator widget
//! - [`Frame`]: Container widget with border decoration
//! - [`Separator`]: Visual dividing line
//! - [`Spacer`]: Invisible widget for layout spacing
//! - [`ScrollBar`]: Standalone scrollbar widget
//! - [`ScrollArea`]: Scrollable container widget
//! - [`TabBar`]: Standalone tab bar widget
//! - [`TabWidget`]: Tabbed page container
//! - [`StackedWidget`]: Container showing one child at a time

mod abstract_button;
mod button_group;
mod checkbox;
mod frame;
mod label;
mod line_edit;
mod progress_bar;
mod push_button;
mod radio_button;
mod scroll_area;
mod scroll_bar;
mod separator;
mod spacer;
mod stacked_widget;
mod tab_bar;
mod tab_widget;

pub use abstract_button::AbstractButton;
pub use button_group::ButtonGroup;
pub use checkbox::{CheckBox, CheckState};
pub use frame::{Frame, FrameShadow, FrameShape};
pub use label::{ElideMode, Label};
pub use line_edit::{EchoMode, LineEdit};
pub use progress_bar::{Orientation, ProgressBar};
pub use push_button::PushButton;
pub use radio_button::RadioButton;
pub use scroll_area::{ScrollArea, ScrollBarPolicy};
pub use scroll_bar::ScrollBar;
pub use separator::{Separator, SeparatorOrientation};
pub use spacer::Spacer;
pub use stacked_widget::StackedWidget;
pub use tab_bar::{TabBar, TabPosition};
pub use tab_widget::TabWidget;
