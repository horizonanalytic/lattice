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
horizon-lattice = "0.1"
```

## The Counter App

Replace `src/main.rs` with:

```rust,ignore
use horizon_lattice::prelude::*;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the application
    let app = Application::new()?;

    // Create the main window
    let mut window = Window::new();
    window.set_title("Counter");
    window.set_size(300, 150);

    // Shared counter state
    let count = Arc::new(AtomicI32::new(0));

    // Create widgets
    let label = Label::new("Count: 0");
    let increment_btn = Button::new("+");
    let decrement_btn = Button::new("-");

    // Connect signals
    let label_clone = label.clone();
    let count_clone = count.clone();
    increment_btn.clicked().connect(move |_| {
        let new_value = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
        label_clone.set_text(&format!("Count: {}", new_value));
    });

    let label_clone = label.clone();
    let count_clone = count.clone();
    decrement_btn.clicked().connect(move |_| {
        let new_value = count_clone.fetch_sub(1, Ordering::SeqCst) - 1;
        label_clone.set_text(&format!("Count: {}", new_value));
    });

    // Layout
    let mut layout = HBoxLayout::new();
    layout.add_widget(decrement_btn);
    layout.add_widget(label);
    layout.add_widget(increment_btn);

    let mut container = Container::new();
    container.set_layout(layout);

    window.set_central_widget(container);
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
let mut window = Window::new();
window.set_title("Counter");
window.set_size(300, 150);
```

Windows are top-level containers for your UI. Set properties like title, size, and position before calling `show()`.

### Widgets

```rust,ignore
let label = Label::new("Count: 0");
let increment_btn = Button::new("+");
```

Widgets are the building blocks of your UI. Common widgets include:
- `Label` - Display text
- `Button` - Clickable button
- `TextEdit` - Text input
- `Container` - Group other widgets

### Signals and Slots

```rust,ignore
increment_btn.clicked().connect(move |_| {
    // Handle click
});
```

Signals are the Qt-inspired way to handle events. When a button is clicked, it emits a `clicked` signal. You connect a closure (slot) to respond to it.

### Layouts

```rust,ignore
let mut layout = HBoxLayout::new();
layout.add_widget(decrement_btn);
layout.add_widget(label);
layout.add_widget(increment_btn);
```

Layouts automatically arrange widgets. `HBoxLayout` arranges them horizontally. Other layouts include:
- `VBoxLayout` - Vertical arrangement
- `GridLayout` - Grid arrangement
- `FormLayout` - Label/field pairs

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

You should see a window with - and + buttons around a "Count: 0" label. Clicking the buttons updates the counter.

## Next Steps

Continue to [Basic Concepts](./basic-concepts.md) to learn more about the widget system, signals, and layouts.
