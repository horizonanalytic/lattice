# Tutorial: Hello World

Build your first Horizon Lattice application.

## What You'll Learn

- Creating an Application instance
- Showing a Window
- Adding a Label widget
- Understanding the basic structure

## Prerequisites

- Rust installed (1.75+)
- A new Cargo project

## Project Setup

Create a new Rust project:

```bash
cargo new hello-lattice
cd hello-lattice
```

Add Horizon Lattice to `Cargo.toml`:

```toml
[dependencies]
horizon-lattice = "0.1"
```

## Step 1: The Minimal Application

Every Horizon Lattice application starts with creating an `Application`. This initializes the event loop, graphics context, and platform integration.

Replace `src/main.rs` with:

```rust,ignore
use horizon_lattice::Application;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the application (must be first)
    let app = Application::new()?;

    // Run the event loop (blocks until quit)
    Ok(app.run()?)
}
```

This compiles and runs, but does nothing visible because there's no window.

## Step 2: Create a Window

Windows are top-level containers for your UI. Import the Window widget and create one:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::Window;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    // Create a window
    let mut window = Window::new("Hello, World!")
        .with_size(400.0, 300.0);

    // Show the window
    window.show();

    app.run()
}
```

Now when you run the application, you'll see an empty window titled "Hello, World!" that's 400x300 pixels.

### Window Properties

Windows support many properties:

```rust,ignore
let mut window = Window::new("My App")
    .with_size(800.0, 600.0)         // Width x Height
    .with_position(100.0, 100.0)     // X, Y position
    .with_minimum_size(320.0, 240.0) // Minimum allowed size
    .with_flags(WindowFlags::DEFAULT);
```

## Step 3: Add a Label

Labels display text. Let's add one to our window:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{Label, Window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    // Create a window
    let mut window = Window::new("Hello, World!")
        .with_size(400.0, 300.0);

    // Create a label
    let label = Label::new("Hello, World!");

    // Set the label as the window's content widget
    window.set_content_widget(label.object_id());
    window.show();

    app.run()
}
```

Run this and you'll see "Hello, World!" displayed in the window.

## Step 4: Style the Label

Labels support various styling options:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{Label, Window};
use horizon_lattice::render::{Color, HorizontalAlign, VerticalAlign};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Hello, World!")
        .with_size(400.0, 300.0);

    // Create a styled label
    let label = Label::new("Hello, World!")
        .with_horizontal_align(HorizontalAlign::Center)
        .with_vertical_align(VerticalAlign::Center)
        .with_text_color(Color::from_rgb8(50, 100, 200));

    window.set_content_widget(label.object_id());
    window.show();

    app.run()
}
```

Now the text is centered and colored blue.

### Label Options

Labels support many display options:

```rust,ignore
// Word wrapping for long text
let wrapped = Label::new("This is a very long text that will wrap to multiple lines")
    .with_word_wrap(true);

// Text elision (truncation with "...")
use horizon_lattice::widget::widgets::ElideMode;

let elided = Label::new("very_long_filename_that_doesnt_fit.txt")
    .with_elide_mode(ElideMode::Right);  // Shows "very_long_filen..."

// Rich text with HTML
let rich = Label::from_html("Hello <b>bold</b> and <i>italic</i>!");
```

## Complete Example

Here's the complete Hello World application:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{Label, Window};
use horizon_lattice::render::{Color, HorizontalAlign, VerticalAlign};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the application
    let app = Application::new()?;

    // Create the main window
    let mut window = Window::new("Hello, World!")
        .with_size(400.0, 300.0);

    // Create a centered, styled label
    let label = Label::new("Hello, Horizon Lattice!")
        .with_horizontal_align(HorizontalAlign::Center)
        .with_vertical_align(VerticalAlign::Center)
        .with_text_color(Color::from_rgb8(50, 100, 200));

    // Set up the window
    window.set_content_widget(label.object_id());
    window.show();

    // Run until window is closed
    app.run()
}
```

## Understanding the Code

### Application Singleton

```rust,ignore
let app = Application::new()?;
```

The `Application` is a singleton - only one can exist per process. It:
- Initializes the graphics system (wgpu)
- Sets up the event loop (winit)
- Registers the main thread for thread-safety checks
- Creates the global object registry

### Window Lifecycle

```rust,ignore
let mut window = Window::new("Title")
    .with_size(400.0, 300.0);
window.show();
```

Windows are created hidden by default. Call `show()` to make them visible. The builder pattern (`with_*` methods) allows fluent configuration.

### Content Widget

```rust,ignore
window.set_content_widget(label.object_id());
```

Each window has a content widget that fills its content area. You pass the widget's `ObjectId` (obtained via `object_id()`). For more complex UIs, you'll set a Container with a layout as the content widget.

### Event Loop

```rust,ignore
app.run()
```

This starts the event loop, which:
- Processes user input (mouse, keyboard)
- Dispatches signals
- Redraws widgets as needed
- Handles window management

The function blocks until all windows are closed (or `app.quit()` is called).

## Run It

```bash
cargo run
```

You should see a window with centered blue text saying "Hello, Horizon Lattice!".

## Next Steps

- [Button Clicks](./button-clicks.md) - Add interactivity with buttons and signals
- [Forms and Validation](./forms.md) - Build input forms with layouts
- [Basic Concepts](../getting-started/basic-concepts.md) - Learn about the widget system in depth
