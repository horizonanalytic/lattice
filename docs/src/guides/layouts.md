# Layouts Guide

Layouts automatically arrange child widgets within a container.

## Layout Algorithm

Layouts use a two-pass algorithm:

1. **Measure pass**: Query each child's `size_hint()` and size policy
2. **Arrange pass**: Assign positions and sizes to children

## HBoxLayout and VBoxLayout

Arrange widgets horizontally or vertically:

```rust,ignore
let mut hbox = HBoxLayout::new();
hbox.set_spacing(10);  // Space between widgets
hbox.set_margins(EdgeInsets::uniform(8));  // Outer margins

hbox.add_widget(label);
hbox.add_widget(text_edit);
hbox.add_stretch(1);  // Flexible space
hbox.add_widget(button);
```

## GridLayout

Arrange widgets in rows and columns:

```rust,ignore
let mut grid = GridLayout::new();
grid.set_column_stretch(1, 1);  // Column 1 expands

grid.add_widget(Label::new("Name:"), 0, 0);
grid.add_widget(name_edit, 0, 1);

grid.add_widget(Label::new("Email:"), 1, 0);
grid.add_widget(email_edit, 1, 1);

// Span multiple columns
grid.add_widget_with_span(submit_btn, 2, 0, 1, 2);
```

## FormLayout

Convenient layout for label-field pairs:

```rust,ignore
let mut form = FormLayout::new();
form.add_row("Username:", username_edit);
form.add_row("Password:", password_edit);
form.add_row("Remember me:", checkbox);
```

## Nested Layouts

Layouts can be nested for complex UIs:

```rust,ignore
let mut main = VBoxLayout::new();

// Header
let mut header = HBoxLayout::new();
header.add_widget(logo);
header.add_stretch(1);
header.add_widget(menu_bar);
main.add_layout(header);

// Content
main.add_widget(content_area);

// Footer
main.add_widget(status_bar);
```

## Custom Layouts

Implement the `Layout` trait for custom behavior:

```rust,ignore
impl Layout for FlowLayout {
    fn calculate_size(&self, children: &[LayoutItem]) -> Size {
        // Calculate total size needed
    }

    fn arrange(&self, rect: Rect, children: &mut [LayoutItem]) {
        // Position each child within rect
    }
}
```

See the [Layout Reference](../reference/layouts.md) for all layout types.
