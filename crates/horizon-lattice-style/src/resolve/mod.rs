//! Style resolution engine.

mod cache;
mod cascade;
mod engine;
mod inheritance;

pub use cascade::cascade_properties;
pub use engine::{StyleContext, StyleEngine, WidgetStyleState};
