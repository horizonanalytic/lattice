//! CSS selector types and matching.

mod matcher;
mod specificity;
mod types;

pub use matcher::{SelectorMatcher, SiblingInfo, WidgetMatchContext, WidgetState};
pub use specificity::{Specificity, SpecificityWithOrder};
pub use types::*;
