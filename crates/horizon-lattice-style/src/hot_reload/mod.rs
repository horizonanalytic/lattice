//! Hot-reload support for stylesheets.
//!
//! This module is only available with the `hot-reload` feature.

mod watcher;

pub use watcher::{StylesheetWatcher, StylesheetChangeEvent, ChangeKind};
