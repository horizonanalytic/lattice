//! CSS specificity calculation.

use super::{Selector, SelectorPart, TypeSelector, PseudoClass};

/// CSS specificity as (a, b, c) tuple.
///
/// - a: ID selectors
/// - b: Class selectors, attributes, pseudo-classes
/// - c: Type selectors, pseudo-elements
///
/// Compared lexicographically: (1,0,0) > (0,99,99)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Specificity {
    /// Zero specificity (universal selector).
    pub const ZERO: Self = Self(0, 0, 0);

    /// Inline style specificity (always wins over selectors).
    pub const INLINE: Self = Self(u32::MAX, u32::MAX, u32::MAX);

    /// Calculate specificity of a selector.
    pub fn of_selector(selector: &Selector) -> Self {
        let mut a = 0u32;
        let mut b = 0u32;
        let mut c = 0u32;

        for part in &selector.parts {
            Self::add_part(part, &mut a, &mut b, &mut c);
        }

        Self(a, b, c)
    }

    /// Calculate specificity of a selector part.
    pub fn of_part(part: &SelectorPart) -> Self {
        let mut a = 0u32;
        let mut b = 0u32;
        let mut c = 0u32;
        Self::add_part(part, &mut a, &mut b, &mut c);
        Self(a, b, c)
    }

    fn add_part(part: &SelectorPart, a: &mut u32, b: &mut u32, c: &mut u32) {
        // ID selector
        if part.id.is_some() {
            *a += 1;
        }

        // Class selectors
        *b += part.classes.len() as u32;

        // Pseudo-classes (except :not)
        for pseudo in &part.pseudo_classes {
            match pseudo {
                PseudoClass::Not(inner) => {
                    // :not() specificity is that of its argument
                    Self::add_part(inner, a, b, c);
                }
                _ => {
                    *b += 1;
                }
            }
        }

        // Type selector
        if let Some(type_sel) = &part.type_selector {
            match type_sel {
                TypeSelector::Universal => {} // * has no specificity
                TypeSelector::Type(_) => *c += 1,
            }
        }
    }

    /// Get the ID selector count.
    pub fn ids(&self) -> u32 {
        self.0
    }

    /// Get the class/pseudo-class count.
    pub fn classes(&self) -> u32 {
        self.1
    }

    /// Get the type selector count.
    pub fn types(&self) -> u32 {
        self.2
    }

    /// Combine with source order for complete ordering.
    pub fn with_order(self, order: u32) -> SpecificityWithOrder {
        SpecificityWithOrder {
            specificity: self,
            order,
        }
    }
}

impl std::fmt::Display for Specificity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({},{},{})", self.0, self.1, self.2)
    }
}

/// Specificity combined with source order for tie-breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpecificityWithOrder {
    /// The CSS specificity value.
    pub specificity: Specificity,
    /// Source order for tie-breaking (higher = later in stylesheet).
    pub order: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selector::{SelectorPart, PseudoClass};

    #[test]
    fn specificity_calculation() {
        // * -> (0,0,0)
        let sel = Selector::universal();
        assert_eq!(Specificity::of_selector(&sel), Specificity(0, 0, 0));

        // Button -> (0,0,1)
        let sel = Selector::type_selector("Button");
        assert_eq!(Specificity::of_selector(&sel), Specificity(0, 0, 1));

        // .primary -> (0,1,0)
        let sel = Selector::class("primary");
        assert_eq!(Specificity::of_selector(&sel), Specificity(0, 1, 0));

        // #submit -> (1,0,0)
        let sel = Selector::id("submit");
        assert_eq!(Specificity::of_selector(&sel), Specificity(1, 0, 0));

        // Button.primary:hover -> (0,2,1)
        let sel = Selector {
            parts: vec![
                SelectorPart::type_only("Button")
                    .with_class("primary")
                    .with_pseudo(PseudoClass::Hover)
            ],
            combinators: vec![],
        };
        assert_eq!(Specificity::of_selector(&sel), Specificity(0, 2, 1));

        // #submit.primary:hover -> (1,2,0)
        let sel = Selector {
            parts: vec![
                SelectorPart::id_only("submit")
                    .with_class("primary")
                    .with_pseudo(PseudoClass::Hover)
            ],
            combinators: vec![],
        };
        assert_eq!(Specificity::of_selector(&sel), Specificity(1, 2, 0));
    }

    #[test]
    fn specificity_comparison() {
        // ID > class > type
        assert!(Specificity(1, 0, 0) > Specificity(0, 99, 99));
        assert!(Specificity(0, 1, 0) > Specificity(0, 0, 99));
        assert!(Specificity(0, 0, 1) > Specificity(0, 0, 0));

        // Same level, higher count wins
        assert!(Specificity(0, 2, 0) > Specificity(0, 1, 0));
    }

    #[test]
    fn specificity_with_order() {
        let s1 = Specificity(0, 1, 0).with_order(1);
        let s2 = Specificity(0, 1, 0).with_order(2);
        let s3 = Specificity(0, 2, 0).with_order(0);

        // Higher specificity wins regardless of order
        assert!(s3 > s1);
        assert!(s3 > s2);

        // Same specificity, higher order wins
        assert!(s2 > s1);
    }

    #[test]
    fn not_pseudo_class_specificity() {
        // :not(.primary) has specificity of .primary = (0,1,0)
        let sel = Selector {
            parts: vec![
                SelectorPart::new().with_pseudo(PseudoClass::Not(
                    Box::new(SelectorPart::class_only("primary"))
                ))
            ],
            combinators: vec![],
        };
        assert_eq!(Specificity::of_selector(&sel), Specificity(0, 1, 0));
    }
}
