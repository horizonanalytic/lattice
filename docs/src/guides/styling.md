# Styling Guide

Horizon Lattice uses a CSS-like styling system for widget appearance.

## Selectors

### Type Selectors

Match widgets by type name:

```rust
use horizon_lattice_style::prelude::*;

// Simple type selector
let button = Selector::type_selector("Button");
assert_eq!(button.to_string(), "Button");

let label = Selector::type_selector("Label");
assert_eq!(label.to_string(), "Label");

// Universal selector (matches any widget)
let any = Selector::universal();
assert_eq!(any.to_string(), "*");
```

### Class Selectors

Match widgets with a specific class:

```rust
use horizon_lattice_style::prelude::*;

// Class selector
let primary = Selector::class("primary");
assert_eq!(primary.to_string(), ".primary");

let danger = Selector::class("danger");
assert_eq!(danger.to_string(), ".danger");

// Combine type and class
let primary_button = Selector::type_selector("Button")
    .descendant(SelectorPart::class_only("primary"));
assert_eq!(primary_button.to_string(), "Button .primary");
```

### ID Selectors

Match a specific widget by ID:

```rust
use horizon_lattice_style::prelude::*;

// ID selector
let submit = Selector::id("submit-button");
assert_eq!(submit.to_string(), "#submit-button");

let header = Selector::id("main-header");
assert_eq!(header.to_string(), "#main-header");
```

### Pseudo-Classes

Match widget states:

```rust
use horizon_lattice_style::prelude::*;

// Create a selector with hover pseudo-class
let hover = Selector::type_selector("Button")
    .descendant(SelectorPart::new().with_pseudo(PseudoClass::Hover));
assert_eq!(hover.to_string(), "Button :hover");

// Button with pressed state
let pressed = SelectorPart::type_only("Button")
    .with_pseudo(PseudoClass::Pressed);
assert_eq!(pressed.to_string(), "Button:pressed");

// Available pseudo-classes
let _ = PseudoClass::Hover;     // Mouse over widget
let _ = PseudoClass::Pressed;   // Mouse button down
let _ = PseudoClass::Focused;   // Has keyboard focus
let _ = PseudoClass::Disabled;  // Widget is disabled
let _ = PseudoClass::Enabled;   // Widget is enabled
let _ = PseudoClass::Checked;   // For checkable widgets
let _ = PseudoClass::Unchecked; // For checkable widgets
let _ = PseudoClass::FirstChild;  // First among siblings
let _ = PseudoClass::LastChild;   // Last among siblings
let _ = PseudoClass::OnlyChild;   // Only child of parent
let _ = PseudoClass::Empty;       // Has no children
```

### Combinators

Combine selectors for hierarchical matching:

```rust
use horizon_lattice_style::prelude::*;

// Descendant combinator (any depth)
let nested = Selector::type_selector("Container")
    .descendant(SelectorPart::type_only("Button"));
assert_eq!(nested.to_string(), "Container Button");

// Child combinator (direct child only)
let child = Selector::type_selector("Form")
    .child(SelectorPart::type_only("Label"));
assert_eq!(child.to_string(), "Form > Label");

// Multiple levels
let deep = Selector::type_selector("Window")
    .descendant(SelectorPart::type_only("Container"))
    .child(SelectorPart::class_only("button-row"))
    .descendant(SelectorPart::type_only("Button"));
assert_eq!(deep.to_string(), "Window Container > .button-row Button");
```

## Specificity

CSS specificity determines which styles take precedence:

```rust
use horizon_lattice_style::prelude::*;

// Specificity is (IDs, Classes+PseudoClasses, Types)

// * -> (0,0,0)
let universal = Selector::universal();
assert_eq!(Specificity::of_selector(&universal), Specificity(0, 0, 0));

// Button -> (0,0,1)
let button = Selector::type_selector("Button");
assert_eq!(Specificity::of_selector(&button), Specificity(0, 0, 1));

// .primary -> (0,1,0)
let class = Selector::class("primary");
assert_eq!(Specificity::of_selector(&class), Specificity(0, 1, 0));

// #submit -> (1,0,0)
let id = Selector::id("submit");
assert_eq!(Specificity::of_selector(&id), Specificity(1, 0, 0));

// Button.primary:hover -> (0,2,1) = 1 type + 1 class + 1 pseudo-class
let complex = Selector {
    parts: vec![
        SelectorPart::type_only("Button")
            .with_class("primary")
            .with_pseudo(PseudoClass::Hover)
    ],
    combinators: vec![],
};
assert_eq!(Specificity::of_selector(&complex), Specificity(0, 2, 1));

// Higher specificity wins
assert!(Specificity(1, 0, 0) > Specificity(0, 99, 99)); // ID beats many classes
assert!(Specificity(0, 1, 0) > Specificity(0, 0, 99));  // Class beats many types
```

## Building Selectors

Use the builder pattern for complex selectors:

```rust
use horizon_lattice_style::prelude::*;

// Build a selector programmatically
let selector = Selector::type_selector("Button")
    .child(SelectorPart::class_only("icon"))
    .descendant(SelectorPart::type_only("Image"));

assert_eq!(selector.to_string(), "Button > .icon Image");

// Get the subject (rightmost part)
let subject = selector.subject().unwrap();
assert!(matches!(subject.type_selector, Some(TypeSelector::Type(ref t)) if t == "Image"));

// Build a complex selector part
let part = SelectorPart::type_only("Button")
    .with_class("primary")
    .with_class("large")
    .with_pseudo(PseudoClass::Hover);
assert_eq!(part.to_string(), "Button.primary.large:hover");
```

## Themes

Create and use themes for consistent styling:

```rust
use horizon_lattice_style::prelude::*;

// Use built-in themes
let light = Theme::light();
let dark = Theme::dark();
let high_contrast = Theme::high_contrast();

// Check theme mode
assert_eq!(light.mode, ThemeMode::Light);
assert_eq!(dark.mode, ThemeMode::Dark);
assert_eq!(high_contrast.mode, ThemeMode::HighContrast);

// Access theme colors
let primary_color = light.primary();
let background = light.background();
let text_color = light.text_color();
```

## Nth-Child Expressions

Use nth-child for pattern-based selection:

```rust
use horizon_lattice_style::prelude::*;

// :nth-child(odd) matches 1st, 3rd, 5th... (2n+1)
let odd = NthExpr::odd();
assert!(odd.matches(0));   // 1st child (0-indexed)
assert!(!odd.matches(1));  // 2nd child
assert!(odd.matches(2));   // 3rd child

// :nth-child(even) matches 2nd, 4th, 6th... (2n)
let even = NthExpr::even();
assert!(!even.matches(0)); // 1st child
assert!(even.matches(1));  // 2nd child
assert!(!even.matches(2)); // 3rd child

// :nth-child(3) matches only the 3rd child
let third = NthExpr::new(0, 3);
assert!(!third.matches(0));
assert!(!third.matches(1));
assert!(third.matches(2));  // 3rd child (0-indexed = 2)

// Custom expression: every 3rd starting from 2nd (3n+2)
let custom = NthExpr::new(3, 2);
println!("Formula: {}", custom); // "3n+2"
```

## CSS Pseudo-Class Parsing

Parse pseudo-classes from CSS strings:

```rust
use horizon_lattice_style::prelude::*;

// Parse standard pseudo-classes
assert_eq!(PseudoClass::from_css("hover"), Some(PseudoClass::Hover));
assert_eq!(PseudoClass::from_css("pressed"), Some(PseudoClass::Pressed));
assert_eq!(PseudoClass::from_css("active"), Some(PseudoClass::Pressed)); // CSS alias
assert_eq!(PseudoClass::from_css("focused"), Some(PseudoClass::Focused));
assert_eq!(PseudoClass::from_css("focus"), Some(PseudoClass::Focused)); // CSS alias
assert_eq!(PseudoClass::from_css("disabled"), Some(PseudoClass::Disabled));
assert_eq!(PseudoClass::from_css("first-child"), Some(PseudoClass::FirstChild));

// Unknown pseudo-class returns None
assert_eq!(PseudoClass::from_css("unknown"), None);
```

## Specificity With Source Order

When specificity is equal, later rules win:

```rust
use horizon_lattice_style::prelude::*;

// Same specificity, different source order
let s1 = Specificity(0, 1, 0).with_order(1);
let s2 = Specificity(0, 1, 0).with_order(2);

// Higher order (later in stylesheet) wins
assert!(s2 > s1);

// But higher specificity always beats lower
let s3 = Specificity(0, 2, 0).with_order(0);
assert!(s3 > s1); // More specific, even though earlier
assert!(s3 > s2);
```

## Theme Modes

Support different visual modes:

```rust
use horizon_lattice_style::prelude::*;

fn select_theme(user_preference: &str) -> Theme {
    match user_preference {
        "dark" => Theme::dark(),
        "high-contrast" => Theme::high_contrast(),
        _ => Theme::light(),
    }
}

// Check and respond to theme mode
let theme = Theme::dark();
match theme.mode {
    ThemeMode::Light => println!("Using light theme"),
    ThemeMode::Dark => println!("Using dark theme"),
    ThemeMode::HighContrast => println!("Using high contrast theme"),
}
```

## Best Practices

1. **Use class selectors** for reusable styles across widget types
2. **Use type selectors** for widget-specific default styles
3. **Use ID selectors sparingly** - they have high specificity and are harder to override
4. **Keep specificity low** - makes styles easier to maintain and override
5. **Use combinators** to scope styles without increasing specificity too much
6. **Leverage themes** for consistent colors and spacing across your application
7. **Use pseudo-classes** for interactive states instead of JavaScript-style state changes

## Supported Properties

### Box Model
- `margin`, `padding` - Edge spacing
- `border-width`, `border-color`, `border-style` - Borders
- `border-radius` - Rounded corners

### Colors
- `color` - Text color
- `background-color` - Background fill

### Typography
- `font-size`, `font-weight`, `font-style`
- `font-family` - Font name or generic
- `text-align` - left, center, right
- `line-height` - Line spacing

### Effects
- `opacity` - 0.0 to 1.0

### Cursor
- `cursor` - pointer, text, etc.

See [Style Properties Reference](../reference/style-properties.md) for the complete list.
