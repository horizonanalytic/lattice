//! CSS selector types and matching.

mod types;
mod specificity;
mod matcher;

pub use types::*;
pub use specificity::{Specificity, SpecificityWithOrder};
pub use matcher::{SelectorMatcher, WidgetMatchContext, WidgetState, SiblingInfo};
