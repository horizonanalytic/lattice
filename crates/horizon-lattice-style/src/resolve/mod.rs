//! Style resolution engine.

mod cascade;
mod inheritance;
mod cache;
mod engine;

pub use engine::{StyleEngine, StyleContext, WidgetStyleState};
pub use cascade::cascade_properties;
