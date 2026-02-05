# Your First Application

Let's build a simple counter application to learn the basics of Horizon Lattice.

## Project Setup

Create a new Rust project:

```bash
cargo new counter-app
cd counter-app
```

Add Horizon Lattice to `Cargo.toml`:

```toml
[dependencies]
horizon-lattice = "1.0"
```

## The Counter App

Replace `src/main.rs` with:

```rust,ignore
use horizon_lattice::prelude::*;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

fn main() -> Result<(), horizon_lattice::LatticeError> {
    // Initialize the application
    let app = Application::new()?;

    // Create the main window
    let mut window = Window::new("Counter")
        .with_size(300.0, 150.0);

    // Shared counter state
    let count = Arc::new(AtomicI32::new(0));

    // Create widgets
    let label = Label::new("Count: 0");
    let label_id = label.object_id();

    let increment_btn = PushButton::new("+");
    let decrement_btn = PushButton::new("-");

    // Connect signals
    let count_inc = count.clone();
    increment_btn.clicked().connect(move |_checked| {
        let new_value = count_inc.fetch_add(1, Ordering::SeqCst) + 1;
        println!("Count: {}", new_value);
        // Note: To update the label, you would use the widget dispatcher
        // or a property binding system in a full application
    });

    let count_dec = count.clone();
    decrement_btn.clicked().connect(move |_checked| {
        let new_value = count_dec.fetch_sub(1, Ordering::SeqCst) - 1;
        println!("Count: {}", new_value);
    });

    // Create layout
    let mut layout = BoxLayout::horizontal();
    layout.set_spacing(10.0);
    layout.add_widget(decrement_btn.object_id());
    layout.add_widget(label_id);
    layout.add_widget(increment_btn.object_id());

    // Create container with layout
    let mut container = ContainerWidget::new();
    container.set_layout(LayoutKind::from(layout));

    // Set up window
    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Understanding the Code

### Application Initialization

```rust,ignore
let app = Application::new()?;
```

Every Horizon Lattice application starts with `Application::new()`. This initializes the event loop, graphics context, and platform integration. There can only be one `Application` per process.

### Creating Windows

```rust,ignore
let mut window = Window::new("Counter")
    .with_size(300.0, 150.0);
```

Windows are created with a title and can be configured using the builder pattern. Set properties like size and position before calling `show()`.

### Widgets

```rust,ignore
let label = Label::new("Count: 0");
let increment_btn = PushButton::new("+");
```

Widgets are the building blocks of your UI. Common widgets include:
- `Label` - Display text
- `PushButton` - Clickable button
- `LineEdit` - Single-line text input
- `ContainerWidget` - Group other widgets

### Object IDs

```rust,ignore
let label_id = label.object_id();
```

Each widget has a unique `ObjectId`. This is used when adding widgets to layouts or containers, and for referencing widgets elsewhere in your code.

### Signals and Slots

```rust,ignore
increment_btn.clicked().connect(move |_checked| {
    // Handle click
});
```

Signals are the Qt-inspired way to handle events. When a button is clicked, it emits a `clicked` signal with a boolean indicating if the button is checked (for toggle buttons). You connect a closure (slot) to respond to it.

### Layouts

```rust,ignore
let mut layout = BoxLayout::horizontal();
layout.set_spacing(10.0);
layout.add_widget(decrement_btn.object_id());
layout.add_widget(label.object_id());
layout.add_widget(increment_btn.object_id());
```

Layouts automatically arrange widgets. `BoxLayout::horizontal()` arranges them in a row. Other layouts include:
- `BoxLayout::vertical()` - Vertical arrangement
- `GridLayout` - Grid arrangement
- `FormLayout` - Label/field pairs
- `FlowLayout` - Flowing arrangement that wraps

### Containers

```rust,ignore
let mut container = ContainerWidget::new();
container.set_layout(LayoutKind::from(layout));
```

Containers hold child widgets and can apply layouts to position them. Use `LayoutKind::from()` to convert a specific layout type.

### Running the Event Loop

```rust,ignore
app.run()
```

This starts the event loop, which:
- Processes user input (mouse, keyboard)
- Dispatches signals
- Repaints widgets as needed

The function blocks until all windows are closed.

## Run It

```bash
cargo run
```

You should see a window with - and + buttons around a "Count: 0" label. Clicking the buttons will print the counter value to the console.

## Next Steps

Continue to [Basic Concepts](./basic-concepts.md) to learn more about the widget system, signals, and layouts.
