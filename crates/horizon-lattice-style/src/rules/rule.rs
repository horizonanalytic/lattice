//! Single style rule definition.

use crate::selector::{Selector, Specificity};
use crate::style::StyleProperties;

/// A style rule mapping a selector to properties.
///
/// Each rule has:
/// - A selector that determines which widgets it applies to
/// - Properties to apply when the selector matches
/// - Pre-computed specificity for efficient sorting
/// - Source order for tie-breaking
#[derive(Debug, Clone)]
pub struct StyleRule {
    /// The selector for matching widgets.
    pub selector: Selector,
    /// The style properties to apply.
    pub properties: StyleProperties,
    /// Pre-computed specificity.
    pub specificity: Specificity,
    /// Source order (for tie-breaking when specificity is equal).
    pub order: u32,
}

impl StyleRule {
    /// Create a new style rule.
    pub fn new(selector: Selector, properties: StyleProperties, order: u32) -> Self {
        let specificity = Specificity::of_selector(&selector);
        Self {
            selector,
            properties,
            specificity,
            order,
        }
    }

    /// Create a rule with a type selector.
    pub fn for_type(
        widget_type: impl Into<String>,
        properties: StyleProperties,
        order: u32,
    ) -> Self {
        Self::new(Selector::type_selector(widget_type), properties, order)
    }

    /// Create a rule with a class selector.
    pub fn for_class(class: impl Into<String>, properties: StyleProperties, order: u32) -> Self {
        Self::new(Selector::class(class), properties, order)
    }

    /// Create a rule with an ID selector.
    pub fn for_id(id: impl Into<String>, properties: StyleProperties, order: u32) -> Self {
        Self::new(Selector::id(id), properties, order)
    }

    /// Get the specificity with source order for comparison.
    pub fn specificity_with_order(&self) -> crate::selector::SpecificityWithOrder {
        self.specificity.with_order(self.order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::Style;

    use horizon_lattice_render::Color;

    #[test]
    fn rule_creation() {
        let props = Style::new().background_color(Color::RED).build();

        let rule = StyleRule::for_type("Button", props, 0);

        assert_eq!(rule.specificity, Specificity(0, 0, 1));
        assert_eq!(rule.order, 0);
    }

    #[test]
    fn rule_specificity_comparison() {
        let props = StyleProperties::default();

        let type_rule = StyleRule::for_type("Button", props.clone(), 0);
        let class_rule = StyleRule::for_class("primary", props.clone(), 1);
        let id_rule = StyleRule::for_id("submit", props, 2);

        assert!(id_rule.specificity > class_rule.specificity);
        assert!(class_rule.specificity > type_rule.specificity);
    }
}
