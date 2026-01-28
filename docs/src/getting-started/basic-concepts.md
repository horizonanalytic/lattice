# Basic Concepts

This page covers the fundamental concepts you'll use throughout Horizon Lattice.

## The Widget Tree

Widgets in Horizon Lattice form a tree structure. Every widget (except the root) has a parent, and can have children.

```
Window
└── Container
    ├── Label
    ├── Button
    └── Container
        ├── TextEdit
        └── Button
```

This hierarchy determines:
- **Rendering order**: Parents paint before children
- **Event propagation**: Events bubble up from children to parents
- **Lifetime management**: When a parent is destroyed, its children are too

## Widget Lifecycle

1. **Creation**: `Widget::new()` creates the widget
2. **Configuration**: Set properties, connect signals
3. **Layout**: Widget is added to a layout or parent
4. **Showing**: `show()` makes it visible
5. **Running**: Widget responds to events and repaints
6. **Destruction**: Widget goes out of scope or is explicitly removed

## Signals and Slots

Signals are a type-safe way to connect events to handlers.

### Emitting Signals

Widgets define signals for events they can produce:

```rust,ignore
// Button has a clicked signal
button.clicked().connect(|_| {
    println!("Clicked!");
});
```

### Signal Parameters

Signals can carry data:

```rust,ignore
// TextEdit emits the new text when changed
text_edit.text_changed().connect(|new_text: &String| {
    println!("Text is now: {}", new_text);
});
```

### Connection Types

By default, connections are automatic—direct if on the same thread, queued if cross-thread:

```rust,ignore
// Explicit connection type
button.clicked().connect_with_type(
    |_| { /* handler */ },
    ConnectionType::Queued,
);
```

## Layouts

Layouts automatically position and size child widgets.

### HBoxLayout and VBoxLayout

Arrange widgets in a row or column:

```rust,ignore
let mut hbox = HBoxLayout::new();
hbox.add_widget(button1);
hbox.add_spacing(10);
hbox.add_widget(button2);
hbox.add_stretch(1); // Pushes remaining widgets to the right
hbox.add_widget(button3);
```

### GridLayout

Arrange widgets in a grid:

```rust,ignore
let mut grid = GridLayout::new();
grid.add_widget(widget, row, column);
grid.add_widget_with_span(wide_widget, row, column, row_span, col_span);
```

### Size Policies

Control how widgets grow and shrink:

```rust,ignore
// Fixed size - won't grow or shrink
widget.set_size_policy(SizePolicy::Fixed, SizePolicy::Fixed);

// Expanding - actively wants more space
widget.set_size_policy(SizePolicy::Expanding, SizePolicy::Preferred);
```

## Styling

Widgets can be styled with CSS-like syntax:

```rust,ignore
// Inline style
button.set_style("background-color: #3498db; color: white;");

// From stylesheet
app.set_stylesheet(r#"
    Button {
        background-color: #3498db;
        color: white;
        padding: 8px 16px;
        border-radius: 4px;
    }

    Button:hover {
        background-color: #2980b9;
    }
"#)?;
```

## Coordinate Systems

Widgets use several coordinate systems:

- **Local**: Origin at widget's top-left (0, 0)
- **Parent**: Relative to parent widget
- **Window**: Relative to window's top-left
- **Global**: Screen coordinates

Convert between them:

```rust,ignore
let parent_pos = widget.map_to_parent(local_pos);
let window_pos = widget.map_to_window(local_pos);
let global_pos = widget.map_to_global(local_pos);
```

## Event Handling

Widgets receive events through the `event()` method:

```rust,ignore
impl Widget for MyWidget {
    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                println!("Clicked at {:?}", e.position());
                event.accept();
                true // Event was handled
            }
            WidgetEvent::KeyPress(e) => {
                if e.key() == Key::Enter {
                    self.submit();
                    event.accept();
                    true
                } else {
                    false // Let parent handle it
                }
            }
            _ => false,
        }
    }
}
```

## Next Steps

Now that you understand the basics, explore the detailed guides:

- [Widgets Guide](../guides/widgets.md) - Deep dive into the widget system
- [Layouts Guide](../guides/layouts.md) - Master layout management
- [Signals Guide](../guides/signals.md) - Advanced signal patterns
- [Styling Guide](../guides/styling.md) - CSS-like styling in depth
