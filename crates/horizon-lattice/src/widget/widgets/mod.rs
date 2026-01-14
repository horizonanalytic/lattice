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

mod abstract_button;
mod button_group;
mod checkbox;
mod frame;
mod label;
mod line_edit;
mod progress_bar;
mod push_button;
mod radio_button;
mod separator;
mod spacer;

pub use abstract_button::AbstractButton;
pub use button_group::ButtonGroup;
pub use checkbox::{CheckBox, CheckState};
pub use frame::{Frame, FrameShadow, FrameShape};
pub use label::{ElideMode, Label};
pub use line_edit::{EchoMode, LineEdit};
pub use progress_bar::{Orientation, ProgressBar};
pub use push_button::PushButton;
pub use radio_button::RadioButton;
pub use separator::{Separator, SeparatorOrientation};
pub use spacer::Spacer;
