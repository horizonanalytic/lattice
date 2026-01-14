//! Theme system with built-in themes.

mod palette;
mod variables;
mod builtin;

pub use palette::ColorPalette;
pub use variables::ThemeVariables;
pub use builtin::{Theme, ThemeMode};
