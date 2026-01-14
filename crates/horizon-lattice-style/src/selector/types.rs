//! Selector type definitions.

use std::fmt;

/// A complete CSS selector (e.g., "Button.primary:hover > Label").
///
/// A selector consists of one or more selector parts connected by combinators.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Selector {
    /// Chain of selector parts with their connecting combinators.
    /// The combinator connects to the *next* part (None for the last part).
    pub parts: Vec<SelectorPart>,
    /// Combinators between parts (length = parts.len() - 1).
    pub combinators: Vec<Combinator>,
}

impl Selector {
    /// Create a simple type selector.
    pub fn type_selector(widget_type: impl Into<String>) -> Self {
        Self {
            parts: vec![SelectorPart::type_only(widget_type)],
            combinators: vec![],
        }
    }

    /// Create a universal selector (*).
    pub fn universal() -> Self {
        Self {
            parts: vec![SelectorPart::universal()],
            combinators: vec![],
        }
    }

    /// Create a class selector.
    pub fn class(class_name: impl Into<String>) -> Self {
        Self {
            parts: vec![SelectorPart::class_only(class_name)],
            combinators: vec![],
        }
    }

    /// Create an ID selector.
    pub fn id(id: impl Into<String>) -> Self {
        Self {
            parts: vec![SelectorPart::id_only(id)],
            combinators: vec![],
        }
    }

    /// Add a descendant selector part.
    pub fn descendant(mut self, part: SelectorPart) -> Self {
        if !self.parts.is_empty() {
            self.combinators.push(Combinator::Descendant);
        }
        self.parts.push(part);
        self
    }

    /// Add a child selector part.
    pub fn child(mut self, part: SelectorPart) -> Self {
        if !self.parts.is_empty() {
            self.combinators.push(Combinator::Child);
        }
        self.parts.push(part);
        self
    }

    /// Get the rightmost (subject) selector part.
    pub fn subject(&self) -> Option<&SelectorPart> {
        self.parts.last()
    }
}

impl fmt::Display for Selector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                match &self.combinators[i - 1] {
                    Combinator::Descendant => write!(f, " ")?,
                    Combinator::Child => write!(f, " > ")?,
                    Combinator::AdjacentSibling => write!(f, " + ")?,
                    Combinator::GeneralSibling => write!(f, " ~ ")?,
                }
            }
            write!(f, "{}", part)?;
        }
        Ok(())
    }
}

/// A single selector segment (e.g., "Button.primary:hover").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SelectorPart {
    /// Type selector (widget type name or universal).
    pub type_selector: Option<TypeSelector>,
    /// ID selector (#id).
    pub id: Option<String>,
    /// Class selectors (.class).
    pub classes: Vec<String>,
    /// Pseudo-class selectors (:hover, :pressed, etc.).
    pub pseudo_classes: Vec<PseudoClass>,
}

impl SelectorPart {
    /// Create a new empty selector part.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a type-only selector.
    pub fn type_only(widget_type: impl Into<String>) -> Self {
        Self {
            type_selector: Some(TypeSelector::Type(widget_type.into())),
            ..Default::default()
        }
    }

    /// Create a universal selector part.
    pub fn universal() -> Self {
        Self {
            type_selector: Some(TypeSelector::Universal),
            ..Default::default()
        }
    }

    /// Create a class-only selector.
    pub fn class_only(class_name: impl Into<String>) -> Self {
        Self {
            classes: vec![class_name.into()],
            ..Default::default()
        }
    }

    /// Create an ID-only selector.
    pub fn id_only(id: impl Into<String>) -> Self {
        Self {
            id: Some(id.into()),
            ..Default::default()
        }
    }

    /// Add a type selector.
    pub fn with_type(mut self, widget_type: impl Into<String>) -> Self {
        self.type_selector = Some(TypeSelector::Type(widget_type.into()));
        self
    }

    /// Add an ID selector.
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Add a class selector.
    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.classes.push(class.into());
        self
    }

    /// Add a pseudo-class selector.
    pub fn with_pseudo(mut self, pseudo: PseudoClass) -> Self {
        self.pseudo_classes.push(pseudo);
        self
    }

    /// Check if this is a universal selector with no other constraints.
    pub fn is_universal_only(&self) -> bool {
        matches!(self.type_selector, Some(TypeSelector::Universal))
            && self.id.is_none()
            && self.classes.is_empty()
            && self.pseudo_classes.is_empty()
    }
}

impl fmt::Display for SelectorPart {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.type_selector {
            Some(TypeSelector::Universal) => write!(f, "*")?,
            Some(TypeSelector::Type(t)) => write!(f, "{}", t)?,
            None => {}
        }

        if let Some(id) = &self.id {
            write!(f, "#{}", id)?;
        }

        for class in &self.classes {
            write!(f, ".{}", class)?;
        }

        for pseudo in &self.pseudo_classes {
            write!(f, ":{}", pseudo)?;
        }

        Ok(())
    }
}

/// Type selector - matches widget type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeSelector {
    /// Universal selector (*) - matches any widget.
    Universal,
    /// Named type (e.g., "Button", "Label").
    Type(String),
}

/// Combinator between selector parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Combinator {
    /// Descendant combinator (space): matches any descendant.
    Descendant,
    /// Child combinator (>): matches direct child only.
    Child,
    /// Adjacent sibling (+): matches immediately following sibling.
    AdjacentSibling,
    /// General sibling (~): matches any following sibling.
    GeneralSibling,
}

/// Pseudo-class selectors for widget state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PseudoClass {
    /// :hover - mouse is over widget.
    Hover,
    /// :pressed - mouse button is down on widget.
    Pressed,
    /// :focused - widget has keyboard focus.
    Focused,
    /// :disabled - widget is disabled.
    Disabled,
    /// :enabled - widget is enabled (default).
    Enabled,
    /// :checked - for checkable widgets.
    Checked,
    /// :unchecked - for checkable widgets.
    Unchecked,
    /// :first-child - first among siblings.
    FirstChild,
    /// :last-child - last among siblings.
    LastChild,
    /// :nth-child(n) - nth among siblings.
    NthChild(NthExpr),
    /// :only-child - only child of parent.
    OnlyChild,
    /// :empty - has no children.
    Empty,
    /// :not(selector) - negation.
    Not(Box<SelectorPart>),
}

impl fmt::Display for PseudoClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PseudoClass::Hover => write!(f, "hover"),
            PseudoClass::Pressed => write!(f, "pressed"),
            PseudoClass::Focused => write!(f, "focused"),
            PseudoClass::Disabled => write!(f, "disabled"),
            PseudoClass::Enabled => write!(f, "enabled"),
            PseudoClass::Checked => write!(f, "checked"),
            PseudoClass::Unchecked => write!(f, "unchecked"),
            PseudoClass::FirstChild => write!(f, "first-child"),
            PseudoClass::LastChild => write!(f, "last-child"),
            PseudoClass::NthChild(expr) => write!(f, "nth-child({})", expr),
            PseudoClass::OnlyChild => write!(f, "only-child"),
            PseudoClass::Empty => write!(f, "empty"),
            PseudoClass::Not(inner) => write!(f, "not({})", inner),
        }
    }
}

impl PseudoClass {
    /// Parse a pseudo-class from CSS string.
    pub fn from_css(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "hover" => Some(Self::Hover),
            "pressed" | "active" => Some(Self::Pressed),
            "focused" | "focus" => Some(Self::Focused),
            "disabled" => Some(Self::Disabled),
            "enabled" => Some(Self::Enabled),
            "checked" => Some(Self::Checked),
            "unchecked" => Some(Self::Unchecked),
            "first-child" => Some(Self::FirstChild),
            "last-child" => Some(Self::LastChild),
            "only-child" => Some(Self::OnlyChild),
            "empty" => Some(Self::Empty),
            _ => None,
        }
    }
}

/// Expression for :nth-child (An+B).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NthExpr {
    /// Coefficient (A in An+B).
    pub a: i32,
    /// Offset (B in An+B).
    pub b: i32,
}

impl NthExpr {
    /// Create a new nth expression.
    pub fn new(a: i32, b: i32) -> Self {
        Self { a, b }
    }

    /// Check if a 0-indexed position matches this expression.
    pub fn matches(&self, index: usize) -> bool {
        let n = index as i32 + 1; // Convert to 1-indexed
        if self.a == 0 {
            n == self.b
        } else {
            let diff = n - self.b;
            if self.a > 0 {
                diff >= 0 && diff % self.a == 0
            } else {
                diff <= 0 && diff % self.a == 0
            }
        }
    }

    /// :nth-child(odd) = 2n+1.
    pub fn odd() -> Self {
        Self { a: 2, b: 1 }
    }

    /// :nth-child(even) = 2n.
    pub fn even() -> Self {
        Self { a: 2, b: 0 }
    }

    /// :nth-child(n) - matches all.
    pub fn all() -> Self {
        Self { a: 1, b: 0 }
    }
}

impl fmt::Display for NthExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.a, self.b) {
            (2, 1) => write!(f, "odd"),
            (2, 0) => write!(f, "even"),
            (0, b) => write!(f, "{}", b),
            (1, 0) => write!(f, "n"),
            (a, 0) => write!(f, "{}n", a),
            (1, b) if b > 0 => write!(f, "n+{}", b),
            (1, b) => write!(f, "n{}", b),
            (a, b) if b > 0 => write!(f, "{}n+{}", a, b),
            (a, b) => write!(f, "{}n{}", a, b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_display() {
        let sel = Selector::type_selector("Button")
            .descendant(SelectorPart::class_only("primary").with_pseudo(PseudoClass::Hover));
        assert_eq!(sel.to_string(), "Button .primary:hover");

        let sel = Selector::type_selector("Container")
            .child(SelectorPart::type_only("Label"));
        assert_eq!(sel.to_string(), "Container > Label");
    }

    #[test]
    fn selector_part_display() {
        let part = SelectorPart::type_only("Button")
            .with_class("primary")
            .with_class("large")
            .with_pseudo(PseudoClass::Hover);
        assert_eq!(part.to_string(), "Button.primary.large:hover");
    }

    #[test]
    fn nth_expr_matches() {
        // :nth-child(3)
        let expr = NthExpr::new(0, 3);
        assert!(!expr.matches(0)); // 1st child
        assert!(!expr.matches(1)); // 2nd child
        assert!(expr.matches(2));  // 3rd child
        assert!(!expr.matches(3)); // 4th child

        // :nth-child(odd) = 2n+1
        let expr = NthExpr::odd();
        assert!(expr.matches(0));  // 1st child
        assert!(!expr.matches(1)); // 2nd child
        assert!(expr.matches(2));  // 3rd child
        assert!(!expr.matches(3)); // 4th child

        // :nth-child(even) = 2n
        let expr = NthExpr::even();
        assert!(!expr.matches(0)); // 1st child
        assert!(expr.matches(1));  // 2nd child
        assert!(!expr.matches(2)); // 3rd child
        assert!(expr.matches(3));  // 4th child
    }
}
