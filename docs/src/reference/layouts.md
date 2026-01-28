# Layout Reference

All layout types available in Horizon Lattice.

## HBoxLayout

Arranges widgets horizontally (left to right).

```rust,ignore
let mut layout = HBoxLayout::new();
layout.set_spacing(8);
layout.add_widget(widget1);
layout.add_widget(widget2);
```

## VBoxLayout

Arranges widgets vertically (top to bottom).

```rust,ignore
let mut layout = VBoxLayout::new();
layout.set_spacing(8);
layout.add_widget(widget1);
layout.add_widget(widget2);
```

## GridLayout

Arranges widgets in a grid.

```rust,ignore
let mut layout = GridLayout::new();
layout.add_widget(widget, row, col);
layout.add_widget_with_span(widget, row, col, row_span, col_span);
```

## FormLayout

Two-column layout for forms.

```rust,ignore
let mut layout = FormLayout::new();
layout.add_row("Label:", widget);
```

## StackLayout

Shows one widget at a time.

```rust,ignore
let mut layout = StackLayout::new();
layout.add_widget(page1);
layout.add_widget(page2);
layout.set_current_index(0);
```

---

> **Note**: This reference is under construction. See the [API documentation](https://docs.rs/horizon-lattice) for complete details.
