//! Selector matching algorithm.

use super::{Selector, SelectorPart, TypeSelector, PseudoClass, Combinator};

/// Widget state for selector matching.
#[derive(Debug, Clone, Default)]
pub struct WidgetMatchContext<'a> {
    /// Widget type name (e.g., "Button", "Label").
    pub widget_type: &'a str,
    /// Widget name/ID (for #id selectors).
    pub widget_name: Option<&'a str>,
    /// Widget's CSS classes.
    pub classes: &'a [String],
    /// Widget state flags.
    pub state: WidgetState,
    /// Sibling information for structural pseudo-classes.
    pub sibling_info: Option<SiblingInfo>,
    /// Number of children (for :empty).
    pub child_count: usize,
}

/// Widget interaction state.
#[derive(Debug, Clone, Copy, Default)]
pub struct WidgetState {
    /// Whether the mouse is hovering over the widget.
    pub hovered: bool,
    /// Whether the widget is being pressed/clicked.
    pub pressed: bool,
    /// Whether the widget has keyboard focus.
    pub focused: bool,
    /// Whether the widget is enabled for interaction.
    pub enabled: bool,
    /// Checked state for checkable widgets (None = not checkable).
    pub checked: Option<bool>,
}

/// Sibling position information.
#[derive(Debug, Clone, Copy)]
pub struct SiblingInfo {
    /// Zero-based index among siblings.
    pub index: usize,
    /// Total number of siblings (including self).
    pub count: usize,
}

impl SiblingInfo {
    /// Returns true if this is the first sibling.
    pub fn is_first(&self) -> bool {
        self.index == 0
    }

    /// Returns true if this is the last sibling.
    pub fn is_last(&self) -> bool {
        self.index + 1 == self.count
    }

    /// Returns true if this is the only child.
    pub fn is_only(&self) -> bool {
        self.count == 1
    }
}

/// Selector matching engine.
pub struct SelectorMatcher;

impl SelectorMatcher {
    /// Check if a selector's subject (rightmost part) matches the widget.
    ///
    /// This only checks the final selector part. For full matching with
    /// combinators, use `matches_with_ancestors`.
    pub fn matches_subject(selector: &Selector, context: &WidgetMatchContext<'_>) -> bool {
        if let Some(subject) = selector.subject() {
            Self::part_matches(subject, context)
        } else {
            false
        }
    }

    /// Check if a selector part matches the widget.
    pub fn part_matches(part: &SelectorPart, context: &WidgetMatchContext<'_>) -> bool {
        // Check type selector
        if let Some(type_sel) = &part.type_selector {
            match type_sel {
                TypeSelector::Universal => {} // Always matches
                TypeSelector::Type(name) => {
                    if name != context.widget_type {
                        return false;
                    }
                }
            }
        }

        // Check ID selector
        if let Some(id) = &part.id {
            match context.widget_name {
                Some(name) if name == id => {}
                _ => return false,
            }
        }

        // Check class selectors (all must match)
        for class in &part.classes {
            if !context.classes.iter().any(|c| c == class) {
                return false;
            }
        }

        // Check pseudo-class selectors (all must match)
        for pseudo in &part.pseudo_classes {
            if !Self::pseudo_matches(pseudo, context) {
                return false;
            }
        }

        true
    }

    /// Check if a pseudo-class matches the widget state.
    fn pseudo_matches(pseudo: &PseudoClass, context: &WidgetMatchContext<'_>) -> bool {
        match pseudo {
            PseudoClass::Hover => context.state.hovered,
            PseudoClass::Pressed => context.state.pressed,
            PseudoClass::Focused => context.state.focused,
            PseudoClass::Disabled => !context.state.enabled,
            PseudoClass::Enabled => context.state.enabled,
            PseudoClass::Checked => context.state.checked == Some(true),
            PseudoClass::Unchecked => context.state.checked == Some(false),

            PseudoClass::FirstChild => {
                context.sibling_info.map(|s| s.is_first()).unwrap_or(false)
            }
            PseudoClass::LastChild => {
                context.sibling_info.map(|s| s.is_last()).unwrap_or(false)
            }
            PseudoClass::OnlyChild => {
                context.sibling_info.map(|s| s.is_only()).unwrap_or(false)
            }
            PseudoClass::NthChild(expr) => {
                context.sibling_info.map(|s| expr.matches(s.index)).unwrap_or(false)
            }
            PseudoClass::Empty => context.child_count == 0,

            PseudoClass::Not(inner) => !Self::part_matches(inner, context),
        }
    }
}

/// Trait for providing ancestor context for selector matching.
pub trait AncestorProvider {
    /// Get the parent's match context, if any.
    fn parent_context(&self) -> Option<WidgetMatchContext<'_>>;

    /// Get ancestor contexts from parent to root.
    fn ancestors(&self) -> Vec<WidgetMatchContext<'_>>;

    /// Get the previous sibling's match context, if any.
    fn previous_sibling_context(&self) -> Option<WidgetMatchContext<'_>>;

    /// Get all previous siblings' contexts.
    fn previous_siblings(&self) -> Vec<WidgetMatchContext<'_>>;
}

/// Check if a full selector matches, considering combinators.
///
/// This walks the selector from right to left, checking each part
/// against the widget and its ancestors/siblings based on combinators.
pub fn matches_full<A: AncestorProvider>(
    selector: &Selector,
    context: &WidgetMatchContext<'_>,
    ancestors: &A,
) -> bool {
    if selector.parts.is_empty() {
        return false;
    }

    // Start with the subject (rightmost part)
    if !SelectorMatcher::part_matches(&selector.parts[selector.parts.len() - 1], context) {
        return false;
    }

    // If only one part, we're done
    if selector.parts.len() == 1 {
        return true;
    }

    // Walk backwards through remaining parts
    let ancestor_list = ancestors.ancestors();
    let mut ancestor_idx = 0;

    for i in (0..selector.parts.len() - 1).rev() {
        let part = &selector.parts[i];
        let combinator = &selector.combinators[i];

        match combinator {
            Combinator::Descendant => {
                // Find any matching ancestor
                let mut found = false;
                while ancestor_idx < ancestor_list.len() {
                    if SelectorMatcher::part_matches(part, &ancestor_list[ancestor_idx]) {
                        ancestor_idx += 1;
                        found = true;
                        break;
                    }
                    ancestor_idx += 1;
                }
                if !found {
                    return false;
                }
            }

            Combinator::Child => {
                // Must match immediate parent (next ancestor)
                if ancestor_idx >= ancestor_list.len() {
                    return false;
                }
                if !SelectorMatcher::part_matches(part, &ancestor_list[ancestor_idx]) {
                    return false;
                }
                ancestor_idx += 1;
            }

            Combinator::AdjacentSibling => {
                // Must match immediately preceding sibling
                if let Some(prev) = ancestors.previous_sibling_context() {
                    if !SelectorMatcher::part_matches(part, &prev) {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            Combinator::GeneralSibling => {
                // Must match any preceding sibling
                let siblings = ancestors.previous_siblings();
                let mut found = false;
                for sibling in &siblings {
                    if SelectorMatcher::part_matches(part, sibling) {
                        found = true;
                        break;
                    }
                }
                if !found {
                    return false;
                }
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context<'a>(
        widget_type: &'a str,
        classes: &'a [String],
        state: WidgetState,
    ) -> WidgetMatchContext<'a> {
        WidgetMatchContext {
            widget_type,
            widget_name: None,
            classes,
            state,
            sibling_info: None,
            child_count: 0,
        }
    }

    #[test]
    fn type_selector_matches() {
        let classes = vec![];
        let context = make_context("Button", &classes, WidgetState::default());

        let part = SelectorPart::type_only("Button");
        assert!(SelectorMatcher::part_matches(&part, &context));

        let part = SelectorPart::type_only("Label");
        assert!(!SelectorMatcher::part_matches(&part, &context));

        let part = SelectorPart::universal();
        assert!(SelectorMatcher::part_matches(&part, &context));
    }

    #[test]
    fn class_selector_matches() {
        let classes = vec!["primary".to_string(), "large".to_string()];
        let context = make_context("Button", &classes, WidgetState::default());

        let part = SelectorPart::class_only("primary");
        assert!(SelectorMatcher::part_matches(&part, &context));

        let part = SelectorPart::class_only("secondary");
        assert!(!SelectorMatcher::part_matches(&part, &context));

        // Multiple classes
        let part = SelectorPart::new()
            .with_class("primary")
            .with_class("large");
        assert!(SelectorMatcher::part_matches(&part, &context));

        let part = SelectorPart::new()
            .with_class("primary")
            .with_class("small");
        assert!(!SelectorMatcher::part_matches(&part, &context));
    }

    #[test]
    fn id_selector_matches() {
        let classes = vec![];
        let mut context = make_context("Button", &classes, WidgetState::default());
        context.widget_name = Some("submit");

        let part = SelectorPart::id_only("submit");
        assert!(SelectorMatcher::part_matches(&part, &context));

        let part = SelectorPart::id_only("cancel");
        assert!(!SelectorMatcher::part_matches(&part, &context));
    }

    #[test]
    fn pseudo_class_state_matches() {
        let classes = vec![];
        let mut state = WidgetState::default();
        state.hovered = true;
        state.enabled = true;

        let context = make_context("Button", &classes, state);

        let part = SelectorPart::new().with_pseudo(PseudoClass::Hover);
        assert!(SelectorMatcher::part_matches(&part, &context));

        let part = SelectorPart::new().with_pseudo(PseudoClass::Pressed);
        assert!(!SelectorMatcher::part_matches(&part, &context));

        let part = SelectorPart::new().with_pseudo(PseudoClass::Enabled);
        assert!(SelectorMatcher::part_matches(&part, &context));

        let part = SelectorPart::new().with_pseudo(PseudoClass::Disabled);
        assert!(!SelectorMatcher::part_matches(&part, &context));
    }

    #[test]
    fn structural_pseudo_class_matches() {
        let classes = vec![];
        let state = WidgetState::default();
        let mut context = make_context("Item", &classes, state);
        context.sibling_info = Some(SiblingInfo { index: 0, count: 3 });

        let part = SelectorPart::new().with_pseudo(PseudoClass::FirstChild);
        assert!(SelectorMatcher::part_matches(&part, &context));

        let part = SelectorPart::new().with_pseudo(PseudoClass::LastChild);
        assert!(!SelectorMatcher::part_matches(&part, &context));

        // Test last child
        context.sibling_info = Some(SiblingInfo { index: 2, count: 3 });
        let part = SelectorPart::new().with_pseudo(PseudoClass::LastChild);
        assert!(SelectorMatcher::part_matches(&part, &context));
    }

    #[test]
    fn not_pseudo_class_matches() {
        let classes = vec![];
        let state = WidgetState::default();
        let context = make_context("Button", &classes, state);

        // :not(.primary) should match because widget has no classes
        let part = SelectorPart::new().with_pseudo(PseudoClass::Not(
            Box::new(SelectorPart::class_only("primary"))
        ));
        assert!(SelectorMatcher::part_matches(&part, &context));

        // Add the class
        let classes = vec!["primary".to_string()];
        let context = make_context("Button", &classes, state);
        assert!(!SelectorMatcher::part_matches(&part, &context));
    }

    #[test]
    fn complex_selector_matches() {
        let classes = vec!["primary".to_string()];
        let mut state = WidgetState::default();
        state.hovered = true;
        state.enabled = true;

        let context = make_context("Button", &classes, state);

        // Button.primary:hover:enabled
        let part = SelectorPart::type_only("Button")
            .with_class("primary")
            .with_pseudo(PseudoClass::Hover)
            .with_pseudo(PseudoClass::Enabled);

        assert!(SelectorMatcher::part_matches(&part, &context));
    }
}
