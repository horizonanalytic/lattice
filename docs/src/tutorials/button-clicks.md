# Tutorial: Button Clicks

Learn how to add interactivity with buttons and the signal/slot pattern.

## What You'll Learn

- Creating buttons with PushButton
- Connecting to the `clicked` signal
- Handling events with closures
- Toggle buttons and state management
- Updating UI in response to clicks

## Prerequisites

- Completed the [Hello World](./hello-world.md) tutorial
- Understanding of Rust closures

## Step 1: A Simple Clickable Button

Let's start with a button that prints a message when clicked:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{PushButton, Window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Button Click")
        .with_size(400.0, 300.0);

    // Create a button
    let button = PushButton::new("Click me!");

    // Connect to the clicked signal
    button.clicked().connect(|&checked| {
        println!("Button clicked! Checked: {}", checked);
    });

    window.set_central_widget(button);
    window.show();

    app.run()
}
```

Run this and click the button - you'll see "Button clicked! Checked: false" printed to the console.

## Understanding Signals

Signals are Horizon Lattice's way of communicating events. They're inspired by Qt's signal/slot mechanism but are fully type-safe at compile time.

### The `clicked` Signal

```rust,ignore
// The clicked signal carries a bool indicating checked state
button.clicked().connect(|&checked: &bool| {
    // `checked` is false for normal buttons
    // `checked` is true/false for toggle buttons
});
```

### Available Button Signals

PushButton provides four signals:

```rust,ignore
// Emitted when button is clicked (completed press + release)
button.clicked().connect(|&checked| { /* ... */ });

// Emitted when mouse button is pressed down
button.pressed().connect(|&()| { /* ... */ });

// Emitted when mouse button is released
button.released().connect(|&()| { /* ... */ });

// Emitted when checked state changes (toggle buttons only)
button.toggled().connect(|&checked| { /* ... */ });
```

## Step 2: Toggle Buttons

Toggle buttons maintain a checked/unchecked state:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{PushButton, Window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Toggle Button")
        .with_size(400.0, 300.0);

    // Create a toggle button
    let toggle = PushButton::new("Toggle me")
        .with_checkable(true);

    // React to toggle state changes
    toggle.toggled().connect(|&checked| {
        if checked {
            println!("Toggle is ON");
        } else {
            println!("Toggle is OFF");
        }
    });

    window.set_central_widget(toggle);
    window.show();

    app.run()
}
```

The button visually changes when toggled, and you can query its state with `is_checked()`.

## Step 3: Updating a Label from a Button

To update UI elements from a signal handler, you need to share state. Use `Arc` for thread-safe sharing:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{Label, PushButton, Container, Window};
use horizon_lattice::widget::layout::VBoxLayout;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Counter")
        .with_size(300.0, 200.0);

    // Shared counter state
    let count = Arc::new(AtomicU32::new(0));

    // Create widgets
    let label = Label::new("Count: 0");
    let button = PushButton::new("Increment");

    // Connect button to update label
    let label_clone = label.clone();
    let count_clone = count.clone();
    button.clicked().connect(move |_| {
        let new_count = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
        label_clone.set_text(&format!("Count: {}", new_count));
    });

    // Layout the widgets vertically
    let mut layout = VBoxLayout::new();
    layout.add_widget(label.object_id());
    layout.add_widget(button.object_id());

    let mut container = Container::new();
    container.set_layout(layout);

    window.set_central_widget(container);
    window.show();

    app.run()
}
```

### Key Concepts

1. **Clone before `move`**: Clone `label` and `count` before using in the closure
2. **`move` closure**: Takes ownership of cloned values
3. **Thread-safe state**: Use `AtomicU32` (or `Mutex` for complex state)

## Step 4: Multiple Buttons

Handle multiple buttons with different actions:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{Label, PushButton, Container, Window};
use horizon_lattice::widget::layout::HBoxLayout;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Counter")
        .with_size(300.0, 150.0);

    let count = Arc::new(AtomicI32::new(0));

    let label = Label::new("0");
    let decrement = PushButton::new("-");
    let increment = PushButton::new("+");

    // Decrement button
    let label_clone = label.clone();
    let count_clone = count.clone();
    decrement.clicked().connect(move |_| {
        let new_value = count_clone.fetch_sub(1, Ordering::SeqCst) - 1;
        label_clone.set_text(&new_value.to_string());
    });

    // Increment button
    let label_clone = label.clone();
    let count_clone = count.clone();
    increment.clicked().connect(move |_| {
        let new_value = count_clone.fetch_add(1, Ordering::SeqCst) + 1;
        label_clone.set_text(&new_value.to_string());
    });

    // Horizontal layout: [-] [0] [+]
    let mut layout = HBoxLayout::new();
    layout.add_widget(decrement.object_id());
    layout.add_widget(label.object_id());
    layout.add_widget(increment.object_id());

    let mut container = Container::new();
    container.set_layout(layout);

    window.set_central_widget(container);
    window.show();

    app.run()
}
```

## Step 5: Button Variants

PushButton supports different visual styles:

```rust,ignore
use horizon_lattice::widget::widgets::{PushButton, ButtonVariant};

// Primary (default) - filled with primary color
let primary = PushButton::new("Primary");

// Secondary - outlined with primary color
let secondary = PushButton::new("Secondary")
    .with_variant(ButtonVariant::Secondary);

// Danger - filled with error/red color
let danger = PushButton::new("Delete")
    .with_variant(ButtonVariant::Danger);

// Flat - text only, no background
let flat = PushButton::new("Cancel")
    .with_variant(ButtonVariant::Flat);

// Outlined - outlined with neutral border
let outlined = PushButton::new("Options")
    .with_variant(ButtonVariant::Outlined);
```

## Step 6: Default Button

Mark a button as the "default" to activate it with Enter key:

```rust,ignore
use horizon_lattice::widget::widgets::PushButton;

let ok_button = PushButton::new("OK")
    .with_default(true);

let cancel_button = PushButton::new("Cancel");
```

The default button:
- Has enhanced visual styling (prominent border)
- Activates when Enter is pressed anywhere in the window

## Complete Example: Interactive Counter

Here's a polished counter application:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Label, PushButton, Container, Window, ButtonVariant
};
use horizon_lattice::widget::layout::{HBoxLayout, VBoxLayout, ContentMargins};
use horizon_lattice::render::{HorizontalAlign, VerticalAlign};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Counter App")
        .with_size(300.0, 200.0);

    // Shared state
    let count = Arc::new(AtomicI32::new(0));

    // Title label
    let title = Label::new("Interactive Counter")
        .with_horizontal_align(HorizontalAlign::Center);

    // Count display
    let display = Label::new("0")
        .with_horizontal_align(HorizontalAlign::Center)
        .with_vertical_align(VerticalAlign::Center);

    // Buttons
    let decrement = PushButton::new("-5")
        .with_variant(ButtonVariant::Secondary);
    let increment = PushButton::new("+5");
    let reset = PushButton::new("Reset")
        .with_variant(ButtonVariant::Danger);

    // Connect decrement
    let display_clone = display.clone();
    let count_clone = count.clone();
    decrement.clicked().connect(move |_| {
        let new_value = count_clone.fetch_sub(5, Ordering::SeqCst) - 5;
        display_clone.set_text(&new_value.to_string());
    });

    // Connect increment
    let display_clone = display.clone();
    let count_clone = count.clone();
    increment.clicked().connect(move |_| {
        let new_value = count_clone.fetch_add(5, Ordering::SeqCst) + 5;
        display_clone.set_text(&new_value.to_string());
    });

    // Connect reset
    let display_clone = display.clone();
    let count_clone = count.clone();
    reset.clicked().connect(move |_| {
        count_clone.store(0, Ordering::SeqCst);
        display_clone.set_text("0");
    });

    // Button row layout
    let mut button_row = HBoxLayout::new();
    button_row.set_spacing(10.0);
    button_row.add_widget(decrement.object_id());
    button_row.add_widget(increment.object_id());

    let mut button_container = Container::new();
    button_container.set_layout(button_row);

    // Main vertical layout
    let mut main_layout = VBoxLayout::new();
    main_layout.set_spacing(15.0);
    main_layout.set_content_margins(ContentMargins::uniform(20.0));
    main_layout.add_widget(title.object_id());
    main_layout.add_widget(display.object_id());
    main_layout.add_widget(button_container.object_id());
    main_layout.add_widget(reset.object_id());

    let mut container = Container::new();
    container.set_layout(main_layout);

    window.set_central_widget(container);
    window.show();

    app.run()
}
```

## Signal Connection Types

For advanced use cases, you can specify connection types:

```rust,ignore
use horizon_lattice::ConnectionType;

// Direct: immediate execution (default, same thread only)
button.clicked().connect_with_type(ConnectionType::Direct, |_| {
    // Runs immediately when signal emits
});

// Queued: deferred to event loop (thread-safe)
button.clicked().connect_with_type(ConnectionType::Queued, |_| {
    // Runs on the main thread via event loop
});

// Auto: automatically chooses based on context (recommended)
button.clicked().connect_with_type(ConnectionType::Auto, |_| {
    // Direct if same thread, Queued if cross-thread
});
```

## Disconnecting Signals

Save the connection ID to disconnect later:

```rust,ignore
// Connect and save the ID
let connection_id = button.clicked().connect(|_| {
    println!("Connected!");
});

// Later, disconnect
button.clicked().disconnect(connection_id);
```

## Next Steps

- [Forms and Validation](./forms.md) - Build input forms with multiple widgets
- [Signals Guide](../guides/signals.md) - Deep dive into the signal system
- [Layouts Guide](../guides/layouts.md) - Learn about layout management
