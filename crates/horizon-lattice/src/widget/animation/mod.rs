//! Animation and transition support for Horizon Lattice.
//!
//! This module provides easing functions and transition primitives for smooth
//! animations in the UI.
//!
//! # Easing Functions
//!
//! Easing functions control the rate of change during animations. They take a
//! normalized progress value `t` (0.0 to 1.0) and return a transformed value.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::widget::animation::{Easing, ease};
//!
//! let progress = 0.5;
//! let eased = ease(Easing::EaseInOut, progress);
//! ```

mod easing;
mod transition;

pub use easing::{ease, Easing};
pub use transition::{Transition, TransitionState, TransitionType};
