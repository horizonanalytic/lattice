# Styling Guide

Horizon Lattice uses a CSS-like styling system for widget appearance.

## Applying Styles

### Inline Styles

```rust,ignore
button.set_style("background-color: #3498db; color: white;");
```

### Stylesheets

Apply styles to the entire application:

```rust,ignore
app.set_stylesheet(r#"
    Button {
        background-color: #3498db;
        color: white;
        padding: 8px 16px;
        border-radius: 4px;
    }
"#)?;
```

## Selectors

### Type Selectors

Match widgets by type name:

```css
Button { }
Label { }
TextEdit { }
```

### Class Selectors

Match widgets with a specific class:

```css
.primary { background-color: #3498db; }
.danger { background-color: #e74c3c; }
```

```rust,ignore
button.add_class("primary");
```

### ID Selectors

Match a specific widget by ID:

```css
#submit-button { font-weight: bold; }
```

```rust,ignore
button.set_object_name("submit-button");
```

### Pseudo-Classes

Match widget states:

```css
Button:hover { background-color: #2980b9; }
Button:pressed { background-color: #1a5276; }
Button:disabled { opacity: 0.5; }
TextEdit:focused { border-color: #3498db; }
```

### Combinators

```css
/* Descendant (any depth) */
Container Button { }

/* Child (direct) */
Container > Button { }

/* Adjacent sibling */
Label + TextEdit { }
```

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

### Effects
- `opacity` - 0.0 to 1.0
- `box-shadow` - Drop shadows

See [Style Properties Reference](../reference/style-properties.md) for the complete list.

## Theming

Create themes by loading different stylesheets:

```rust,ignore
fn apply_theme(app: &Application, theme: Theme) {
    let stylesheet = match theme {
        Theme::Light => include_str!("themes/light.css"),
        Theme::Dark => include_str!("themes/dark.css"),
    };
    app.set_stylesheet(stylesheet).unwrap();
}
```

## Dark Mode

Detect and respond to system dark mode:

```rust,ignore
use horizon_lattice::platform::{SystemTheme, ThemeWatcher};

let scheme = SystemTheme::color_scheme();
apply_theme(&app, if scheme == ColorScheme::Dark {
    Theme::Dark
} else {
    Theme::Light
});

// Watch for changes
let watcher = ThemeWatcher::new()?;
watcher.color_scheme_changed().connect(|scheme| {
    apply_theme(&app, if scheme == ColorScheme::Dark {
        Theme::Dark
    } else {
        Theme::Light
    });
});
```
