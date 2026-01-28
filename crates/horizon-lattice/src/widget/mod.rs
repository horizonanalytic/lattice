//! Widget system for Horizon Lattice.
//!
//! This module provides the foundational widget architecture including:
//!
//! - [`Widget`] trait: The base trait for all UI elements
//! - [`WidgetBase`]: Common implementation for widget functionality
//! - Size hints and policies for layout negotiation
//! - Widget events for input handling and lifecycle
//!
//! # Overview
//!
//! The widget system follows Qt's design philosophy while being idiomatic Rust.
//! Each widget implements the [`Widget`] trait and typically contains a
//! [`WidgetBase`] that handles common functionality.
//!
//! # Creating a Widget
//!
//! To create a custom widget:
//!
//! 1. Define a struct with a `WidgetBase` field
//! 2. Implement the `Widget` trait
//! 3. Provide `size_hint()` for layout
//! 4. Implement `paint()` for rendering
//!
//! ```ignore
//! use horizon_lattice::widget::*;
//! use horizon_lattice_render::{Color, Renderer};
//!
//! struct MyButton {
//!     base: WidgetBase,
//!     label: String,
//! }
//!
//! impl MyButton {
//!     pub fn new(label: impl Into<String>) -> Self {
//!         let mut widget = Self {
//!             base: WidgetBase::new::<Self>(),
//!             label: label.into(),
//!         };
//!         widget.base.set_focusable(true);
//!         widget
//!     }
//! }
//!
//! impl Widget for MyButton {
//!     fn widget_base(&self) -> &WidgetBase { &self.base }
//!     fn widget_base_mut(&mut self) -> &mut WidgetBase { &mut self.base }
//!
//!     fn size_hint(&self) -> SizeHint {
//!         SizeHint::from_dimensions(80.0, 30.0)
//!             .with_minimum_dimensions(40.0, 24.0)
//!     }
//!
//!     fn paint(&self, ctx: &mut PaintContext<'_>) {
//!         let color = if self.base.is_hovered() {
//!             Color::from_rgb8(70, 130, 180)  // Steel blue
//!         } else {
//!             Color::from_rgb8(65, 105, 225)  // Royal blue
//!         };
//!         ctx.renderer().fill_rect(ctx.rect(), color);
//!     }
//!
//!     fn event(&mut self, event: &mut WidgetEvent) -> bool {
//!         match event {
//!             WidgetEvent::MousePress(_) => {
//!                 println!("Button clicked: {}", self.label);
//!                 event.accept();
//!                 true
//!             }
//!             _ => false,
//!         }
//!     }
//! }
//! ```
//!
//! # Widget Tree
//!
//! Widgets form a tree structure through parent-child relationships.
//! This is managed by the object system from `horizon-lattice-core`.
//!
//! ```ignore
//! use horizon_lattice::widget::widgets::PushButton;
//! use horizon_lattice::widget::Widget;
//! use horizon_lattice_core::Object;
//!
//! let parent = PushButton::new("Parent");
//! let child = PushButton::new("Click me");
//!
//! // Set up parent-child relationship
//! child.widget_base().set_parent(Some(parent.object_id())).unwrap();
//! ```
//!
//! # Coordinate Systems
//!
//! Widgets use multiple coordinate systems:
//!
//! - **Local coordinates**: Origin at widget's top-left corner
//! - **Parent coordinates**: Relative to parent widget's top-left
//! - **Window coordinates**: Relative to window's top-left
//! - **Global coordinates**: Screen coordinates
//!
//! Use the coordinate mapping methods to convert between systems:
//!
//! ```no_run
//! use horizon_lattice::widget::{Widget, PushButton};
//! use horizon_lattice_render::Point;
//!
//! let widget = PushButton::new("Test");
//! let local_point = Point::new(10.0, 20.0);
//! let parent_point = widget.map_to_parent(local_point);
//! ```
//!
//! # Size Policies
//!
//! Size policies control how widgets behave during layout:
//!
//! - [`SizePolicy::Fixed`]: Cannot grow or shrink
//! - [`SizePolicy::Preferred`]: Can grow/shrink but has a preferred size
//! - [`SizePolicy::Expanding`]: Actively wants more space
//!
//! ```no_run
//! use horizon_lattice::widget::{Widget, PushButton, SizePolicy, SizePolicyPair};
//!
//! let mut widget = PushButton::new("Test");
//! widget.set_size_policy(SizePolicyPair::new(
//!     SizePolicy::Expanding,  // horizontal
//!     SizePolicy::Fixed,      // vertical
//! ));
//! ```
//!
//! # Guides
//!
//! For comprehensive guides on the widget system, see:
//! - [Widget Guide](https://horizonanalyticstudios.github.io/horizon-lattice/guides/widgets.html)
//! - [Layout Guide](https://horizonanalyticstudios.github.io/horizon-lattice/guides/layouts.html)
//! - [Styling Guide](https://horizonanalyticstudios.github.io/horizon-lattice/guides/styling.html)

#[cfg(feature = "accessibility")]
pub mod accessibility;
pub mod animation;
mod base;
pub mod completer;
pub mod cursor;
mod dispatcher;
pub mod drag_drop;
mod events;
pub mod file_drop;
mod focus;
pub mod gesture;
mod geometry;
pub mod ime;
pub mod input_context;
pub mod input_mask;
pub mod keyboard;
pub mod layout;
mod modal;
pub mod mouse;
mod painting;
mod shortcut;
pub mod touch;
mod traits;
pub mod validator;
pub mod widget_timer;
pub mod widgets;

#[cfg(test)]
mod tests;

pub use base::{ContextMenuPolicy, FocusPolicy, WidgetBase};
pub use cursor::{CursorManager, CursorShape};
pub use dispatcher::{DispatchResult, EventDispatcher, WidgetAccess};
pub use drag_drop::{
    DragData, DragDropManager, DragEnterEvent, DragLeaveEvent, DragMoveEvent, DragState,
    DropAction, DropEvent,
};
pub use file_drop::FileDropHandler;
pub use focus::FocusManager;
pub use modal::ModalManager;
pub use events::{
    CloseEvent, ContextMenuEvent, ContextMenuReason, CustomEvent, EnterEvent, EventBase,
    FocusInEvent, FocusOutEvent, FocusReason, GestureState, GestureType, HideEvent, ImeCommitEvent,
    ImeDisabledEvent, ImeEnabledEvent, ImePreeditEvent, Key, KeyPressEvent, KeyReleaseEvent,
    KeyboardModifiers, LeaveEvent, LongPressGestureEvent, MouseButton, MouseDoubleClickEvent,
    MouseMoveEvent, MousePressEvent, MouseReleaseEvent, MoveEvent, PaintEvent, PanGestureEvent,
    PinchGestureEvent, ResizeEvent, RotationGestureEvent, ShowEvent, SwipeDirection,
    SwipeGestureEvent, TapGestureEvent, TimerEvent, TouchEvent, TouchForce, TouchPhase,
    TouchPoint, WheelEvent, WidgetEvent,
};
pub use geometry::{SizeHint, SizePolicy, SizePolicyPair};
pub use layout::{ContentMargins, Layout, LayoutBase, LayoutInvalidator, LayoutItem, SpacerItem};
pub use painting::{FrameRenderer, FrameStats, RepaintManager};
pub use shortcut::{
    mnemonic_to_key, parse_mnemonic, KeyCombination, KeySequence, KeySequenceParseError,
    MnemonicText, SequenceMatch, Shortcut, ShortcutManager, ShortcutResult, StandardKey,
    DEFAULT_CHORD_TIMEOUT_MS, MAX_KEY_SEQUENCE_LENGTH,
};
pub use traits::{AsWidget, PaintContext, Widget};

#[cfg(feature = "accessibility")]
pub use accessibility::{Accessible, AccessibilityManager, AccessibleRole};

// Re-export widgets for convenience
pub use widgets::{
    AbstractButton, CheckBox, CheckState, ElideMode, Label, Orientation, ProgressBar, PushButton,
    ScrollArea, ScrollBar, ScrollBarPolicy,
};
