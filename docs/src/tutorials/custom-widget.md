# Tutorial: Custom Widgets

Learn to create your own widgets with custom painting and event handling.

## What You'll Learn

- Implementing the Widget trait
- Creating custom painting with PaintContext
- Handling mouse and keyboard events
- Managing widget state and focus
- Emitting custom signals

## Prerequisites

- Completed the [Lists](./lists.md) tutorial
- Understanding of Rust traits and structs
- Basic familiarity with the Widget system from the [Widget Guide](../guides/widgets.md)

## The Widget Architecture

Custom widgets in Horizon Lattice require implementing two traits:

1. **Object** - Provides unique identification via `object_id()`
2. **Widget** - Provides UI behavior: size hints, painting, event handling

Every widget contains a `WidgetBase` that handles common functionality like geometry, visibility, focus, and state tracking.

## Step 1: A Minimal Custom Widget

Let's create a simple `ColorBox` widget that displays a solid color:

```rust,ignore
use horizon_lattice::widget::{Widget, WidgetBase, SizeHint, PaintContext};
use horizon_lattice::render::Color;
use horizon_lattice_core::{Object, ObjectId};

/// A simple widget that displays a solid color.
pub struct ColorBox {
    base: WidgetBase,
    color: Color,
}

impl ColorBox {
    /// Create a new ColorBox with the specified color.
    pub fn new(color: Color) -> Self {
        Self {
            base: WidgetBase::new::<Self>(),
            color,
        }
    }

    /// Get the current color.
    pub fn color(&self) -> Color {
        self.color
    }

    /// Set the color and trigger a repaint.
    pub fn set_color(&mut self, color: Color) {
        if self.color != color {
            self.color = color;
            self.base.update(); // Schedule repaint
        }
    }
}

// Implement Object trait for identification
impl Object for ColorBox {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

// Implement Widget trait for UI behavior
impl Widget for ColorBox {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        // Preferred 100x100, minimum 20x20
        SizeHint::from_dimensions(100.0, 100.0)
            .with_minimum_dimensions(20.0, 20.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        // Fill the entire widget with our color
        ctx.renderer().fill_rect(ctx.rect(), self.color);
    }
}
```

### Using ColorBox

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::Window;
use horizon_lattice::render::Color;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Color Box")
        .with_size(300.0, 200.0);

    let color_box = ColorBox::new(Color::from_rgb8(65, 105, 225)); // Royal Blue

    window.set_content_widget(color_box.object_id());
    window.show();

    app.run()
}
```

## Step 2: Understanding WidgetBase

`WidgetBase` provides essential functionality that all widgets need:

### Geometry

```rust,ignore
// Get widget bounds
let rect = self.base.geometry();      // Position + size in parent coordinates
let size = self.base.size();
let pos = self.base.pos();

// Set geometry (usually done by layout)
self.base.set_geometry(Rect::new(Point::new(10.0, 10.0), Size::new(100.0, 50.0)));
```

### Visibility

```rust,ignore
// Show/hide
self.base.show();
self.base.hide();
self.base.set_visible(true);

// Check visibility
let visible = self.base.is_visible();
let effective = self.base.is_effectively_visible(); // Considers ancestors
```

### Enabled State

```rust,ignore
// Enable/disable
self.base.enable();
self.base.disable();
self.base.set_enabled(false);

// Check state
let enabled = self.base.is_enabled();
let effective = self.base.is_effectively_enabled(); // Considers ancestors
```

### Repaint Scheduling

```rust,ignore
// Schedule repaint for next frame
self.base.update();

// Schedule partial repaint
self.base.update_rect(Rect::new(Point::new(0.0, 0.0), Size::new(50.0, 50.0)));
```

## Step 3: Custom Painting

The `paint()` method receives a `PaintContext` that provides access to the renderer:

```rust,ignore
fn paint(&self, ctx: &mut PaintContext<'_>) {
    let renderer = ctx.renderer();
    let rect = ctx.rect(); // Widget bounds (0,0 to width,height)

    // Fill background
    renderer.fill_rect(rect, self.background_color);

    // Draw border
    let stroke = Stroke::new(self.border_color, 2.0);
    renderer.stroke_rect(rect, &stroke);

    // Draw text
    renderer.draw_text(
        Point::new(10.0, 10.0),
        &self.text,
        &self.font,
        self.text_color,
    );

    // Draw focus indicator if focused
    if ctx.should_show_focus() {
        ctx.draw_focus_indicator(2.0);
    }
}
```

### Coordinate System

- The renderer is pre-translated so `(0, 0)` is the widget's top-left corner
- Use `ctx.rect()` to get the full widget bounds in local coordinates
- `ctx.width()` and `ctx.height()` provide dimensions directly

### PaintContext Methods

| Method | Purpose |
|--------|---------|
| `renderer()` | Get the GpuRenderer for drawing |
| `rect()` | Widget bounds (local coordinates) |
| `width()` / `height()` | Widget dimensions |
| `size()` | Widget size as Size struct |
| `is_alt_held()` | Check if Alt key is pressed (for mnemonics) |
| `should_show_focus()` | Check if focus indicator should be drawn |
| `draw_focus_indicator(inset)` | Draw standard focus ring |

## Step 4: Event Handling

Override the `event()` method to handle user input:

```rust,ignore
use horizon_lattice::widget::{
    Widget, WidgetBase, SizeHint, PaintContext,
    WidgetEvent, MousePressEvent, MouseReleaseEvent, MouseButton
};

impl Widget for MyWidget {
    // ... other methods ...

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                if e.button == MouseButton::Left {
                    // Handle left click
                    e.base.accept(); // Mark event as handled
                    return true;
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if e.button == MouseButton::Left {
                    // Handle release
                    e.base.accept();
                    return true;
                }
            }
            _ => {}
        }
        false // Event not handled
    }
}
```

### Event Types

| Event | When Triggered |
|-------|----------------|
| `MousePress` | Mouse button pressed |
| `MouseRelease` | Mouse button released |
| `MouseMove` | Mouse moved over widget |
| `MouseDoubleClick` | Double-click detected |
| `Enter` | Mouse enters widget bounds |
| `Leave` | Mouse leaves widget bounds |
| `Wheel` | Scroll wheel moved |
| `KeyPress` | Key pressed while focused |
| `KeyRelease` | Key released while focused |
| `FocusIn` | Widget gained focus |
| `FocusOut` | Widget lost focus |
| `Resize` | Widget size changed |
| `Move` | Widget position changed |

### Mouse Event Data

```rust,ignore
WidgetEvent::MousePress(e) => {
    let button = e.button;        // MouseButton::Left, Right, Middle
    let local = e.local_pos;      // Position in widget coordinates
    let window = e.window_pos;    // Position in window coordinates
    let global = e.global_pos;    // Position in screen coordinates
    let mods = e.modifiers;       // KeyboardModifiers { shift, control, alt, meta }
}
```

### Keyboard Event Data

```rust,ignore
WidgetEvent::KeyPress(e) => {
    let key = e.key;              // Key enum (Key::A, Key::Space, etc.)
    let text = &e.text;           // Character(s) typed (for text input)
    let mods = e.modifiers;       // Modifier keys held
    let repeat = e.repeat;        // Is this an auto-repeat?
}
```

## Step 5: Focus Management

To receive keyboard events, widgets must accept focus:

```rust,ignore
use horizon_lattice::widget::FocusPolicy;

impl MyWidget {
    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        // Accept focus from both Tab and mouse click
        base.set_focus_policy(FocusPolicy::StrongFocus);
        Self { base, /* ... */ }
    }
}
```

### Focus Policies

| Policy | Tab Focus | Click Focus |
|--------|-----------|-------------|
| `NoFocus` | No | No |
| `TabFocus` | Yes | No |
| `ClickFocus` | No | Yes |
| `StrongFocus` | Yes | Yes |

### Focus in Paint

```rust,ignore
fn paint(&self, ctx: &mut PaintContext<'_>) {
    // Paint normal content...

    // Show focus indicator when focused
    if ctx.should_show_focus() && self.base.has_focus() {
        ctx.draw_focus_indicator(2.0); // 2px inset from edge
    }
}
```

## Step 6: Widget State

WidgetBase tracks common state automatically:

```rust,ignore
// In paint or event handlers:
let is_pressed = self.base.is_pressed();   // Mouse button held down
let is_hovered = self.base.is_hovered();   // Mouse over widget
let has_focus = self.base.has_focus();     // Widget has keyboard focus
```

Use this state for visual feedback:

```rust,ignore
fn paint(&self, ctx: &mut PaintContext<'_>) {
    // Choose color based on state
    let bg_color = if self.base.is_pressed() {
        Color::from_rgb8(45, 85, 205)  // Darker when pressed
    } else if self.base.is_hovered() {
        Color::from_rgb8(85, 145, 255)  // Lighter when hovered
    } else {
        Color::from_rgb8(65, 105, 225)  // Normal
    };

    ctx.renderer().fill_rect(ctx.rect(), bg_color);
}
```

## Step 7: Custom Signals

Use signals to notify external code of events:

```rust,ignore
use horizon_lattice_core::Signal;

pub struct ClickCounter {
    base: WidgetBase,
    count: u32,

    // Custom signals
    pub clicked: Signal<()>,
    pub count_changed: Signal<u32>,
}

impl ClickCounter {
    pub fn new() -> Self {
        Self {
            base: WidgetBase::new::<Self>(),
            count: 0,
            clicked: Signal::new(),
            count_changed: Signal::new(),
        }
    }

    fn increment(&mut self) {
        self.count += 1;
        self.clicked.emit(());
        self.count_changed.emit(self.count);
        self.base.update(); // Repaint to show new count
    }
}

impl Widget for ClickCounter {
    // ... base methods ...

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MouseRelease(e) => {
                if e.button == MouseButton::Left && self.base.is_pressed() {
                    self.increment();
                    e.base.accept();
                    return true;
                }
            }
            _ => {}
        }
        false
    }
}
```

### Connecting to Signals

```rust,ignore
let counter = ClickCounter::new();

counter.clicked.connect(|_| {
    println!("Counter was clicked!");
});

counter.count_changed.connect(|&count| {
    println!("Count is now: {}", count);
});
```

## Complete Example: Interactive Slider

Here's a complete custom slider widget:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::{
    Widget, WidgetBase, SizeHint, PaintContext, FocusPolicy,
    WidgetEvent, MouseButton
};
use horizon_lattice::widget::widgets::{Window, Label, Container};
use horizon_lattice::widget::layout::{VBoxLayout, LayoutKind};
use horizon_lattice::render::{Color, Point, Rect, Size, Stroke};
use horizon_lattice_core::{Object, ObjectId, Signal};
use std::sync::Arc;

/// A custom horizontal slider widget.
pub struct Slider {
    base: WidgetBase,
    value: f32,          // 0.0 to 1.0
    dragging: bool,
    track_color: Color,
    thumb_color: Color,
    thumb_hover_color: Color,

    /// Emitted when the value changes.
    pub value_changed: Signal<f32>,
}

impl Slider {
    const THUMB_WIDTH: f32 = 16.0;
    const THUMB_HEIGHT: f32 = 24.0;
    const TRACK_HEIGHT: f32 = 4.0;

    pub fn new() -> Self {
        let mut base = WidgetBase::new::<Self>();
        base.set_focus_policy(FocusPolicy::StrongFocus);

        Self {
            base,
            value: 0.0,
            dragging: false,
            track_color: Color::from_rgb8(200, 200, 200),
            thumb_color: Color::from_rgb8(65, 105, 225),
            thumb_hover_color: Color::from_rgb8(85, 125, 245),
            value_changed: Signal::new(),
        }
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn set_value(&mut self, value: f32) {
        let clamped = value.clamp(0.0, 1.0);
        if (self.value - clamped).abs() > f32::EPSILON {
            self.value = clamped;
            self.value_changed.emit(self.value);
            self.base.update();
        }
    }

    fn value_from_x(&self, x: f32) -> f32 {
        let usable_width = self.base.width() - Self::THUMB_WIDTH;
        if usable_width <= 0.0 {
            return 0.0;
        }
        let thumb_center_x = x - Self::THUMB_WIDTH / 2.0;
        (thumb_center_x / usable_width).clamp(0.0, 1.0)
    }

    fn thumb_rect(&self) -> Rect {
        let usable_width = self.base.width() - Self::THUMB_WIDTH;
        let thumb_x = self.value * usable_width;
        let thumb_y = (self.base.height() - Self::THUMB_HEIGHT) / 2.0;
        Rect::new(
            Point::new(thumb_x, thumb_y),
            Size::new(Self::THUMB_WIDTH, Self::THUMB_HEIGHT),
        )
    }
}

impl Object for Slider {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
    }
}

impl Widget for Slider {
    fn widget_base(&self) -> &WidgetBase {
        &self.base
    }

    fn widget_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }

    fn size_hint(&self) -> SizeHint {
        SizeHint::from_dimensions(200.0, 30.0)
            .with_minimum_dimensions(50.0, 24.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let width = ctx.width();
        let height = ctx.height();

        // Draw track
        let track_y = (height - Self::TRACK_HEIGHT) / 2.0;
        let track_rect = Rect::new(
            Point::new(Self::THUMB_WIDTH / 2.0, track_y),
            Size::new(width - Self::THUMB_WIDTH, Self::TRACK_HEIGHT),
        );
        ctx.renderer().fill_rect(track_rect, self.track_color);

        // Draw filled portion of track
        let filled_width = self.value * (width - Self::THUMB_WIDTH);
        let filled_rect = Rect::new(
            Point::new(Self::THUMB_WIDTH / 2.0, track_y),
            Size::new(filled_width, Self::TRACK_HEIGHT),
        );
        ctx.renderer().fill_rect(filled_rect, self.thumb_color);

        // Draw thumb
        let thumb_rect = self.thumb_rect();
        let thumb_color = if self.dragging || self.base.is_hovered() {
            self.thumb_hover_color
        } else {
            self.thumb_color
        };
        ctx.renderer().fill_rounded_rect(thumb_rect, 4.0, thumb_color);

        // Draw focus indicator around thumb
        if ctx.should_show_focus() {
            let focus_rect = thumb_rect.inflate(2.0, 2.0);
            let stroke = Stroke::new(Color::from_rgb8(0, 120, 212), 2.0);
            ctx.renderer().stroke_rounded_rect(focus_rect, 6.0, &stroke);
        }
    }

    fn event(&mut self, event: &mut WidgetEvent) -> bool {
        match event {
            WidgetEvent::MousePress(e) => {
                if e.button == MouseButton::Left {
                    self.dragging = true;
                    self.set_value(self.value_from_x(e.local_pos.x));
                    e.base.accept();
                    return true;
                }
            }
            WidgetEvent::MouseRelease(e) => {
                if e.button == MouseButton::Left && self.dragging {
                    self.dragging = false;
                    self.base.update();
                    e.base.accept();
                    return true;
                }
            }
            WidgetEvent::MouseMove(e) => {
                if self.dragging {
                    self.set_value(self.value_from_x(e.local_pos.x));
                    e.base.accept();
                    return true;
                }
            }
            WidgetEvent::KeyPress(e) => {
                use horizon_lattice::widget::Key;
                match e.key {
                    Key::ArrowLeft => {
                        self.set_value(self.value - 0.05);
                        e.base.accept();
                        return true;
                    }
                    Key::ArrowRight => {
                        self.set_value(self.value + 0.05);
                        e.base.accept();
                        return true;
                    }
                    Key::Home => {
                        self.set_value(0.0);
                        e.base.accept();
                        return true;
                    }
                    Key::End => {
                        self.set_value(1.0);
                        e.base.accept();
                        return true;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        false
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Custom Slider")
        .with_size(400.0, 150.0);

    // Create our custom slider
    let slider = Arc::new(std::sync::Mutex::new(Slider::new()));

    // Create label to show value
    let label = Label::new("Value: 0%");

    // Connect slider to label
    let label_clone = label.clone();
    slider.lock().unwrap().value_changed.connect(move |&value| {
        label_clone.set_text(&format!("Value: {:.0}%", value * 100.0));
    });

    // Layout
    let mut layout = VBoxLayout::new();
    layout.set_spacing(20.0);
    layout.add_widget(slider.lock().unwrap().object_id());
    layout.add_widget(label.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Best Practices

### 1. Always Use WidgetBase

Never create widget state that duplicates what `WidgetBase` already provides:

```rust,ignore
// Bad - duplicating state
struct MyWidget {
    base: WidgetBase,
    visible: bool,  // Already in WidgetBase!
    position: Point, // Already in WidgetBase!
}

// Good - use WidgetBase
struct MyWidget {
    base: WidgetBase,
    custom_state: String, // Only add unique state
}
```

### 2. Call update() When State Changes

Always schedule a repaint when visual state changes:

```rust,ignore
pub fn set_color(&mut self, color: Color) {
    if self.color != color {
        self.color = color;
        self.base.update(); // Don't forget this!
    }
}
```

### 3. Accept Events You Handle

Mark events as accepted to prevent propagation:

```rust,ignore
fn event(&mut self, event: &mut WidgetEvent) -> bool {
    match event {
        WidgetEvent::MousePress(e) => {
            e.base.accept(); // Important!
            return true;
        }
        _ => {}
    }
    false
}
```

### 4. Set Appropriate Focus Policy

Choose the right policy for your widget type:

- **NoFocus**: Decorative widgets (labels, separators)
- **ClickFocus**: Mouse-primary widgets (list items)
- **TabFocus**: Keyboard-primary widgets (rare)
- **StrongFocus**: Interactive widgets (buttons, sliders, inputs)

### 5. Provide Meaningful Size Hints

Help layouts by providing accurate size information:

```rust,ignore
fn size_hint(&self) -> SizeHint {
    SizeHint::from_dimensions(200.0, 40.0)  // Preferred
        .with_minimum_dimensions(100.0, 30.0)  // Minimum usable
        .with_maximum_dimensions(500.0, 40.0)  // Maximum reasonable
}
```

### 6. Handle Both Mouse and Keyboard

Make widgets accessible by supporting keyboard navigation:

```rust,ignore
fn event(&mut self, event: &mut WidgetEvent) -> bool {
    match event {
        // Mouse activation
        WidgetEvent::MouseRelease(e) if e.button == MouseButton::Left => {
            self.activate();
            e.base.accept();
            true
        }
        // Keyboard activation (Space or Enter)
        WidgetEvent::KeyPress(e) if e.key == Key::Space || e.key == Key::Enter => {
            self.activate();
            e.base.accept();
            true
        }
        _ => false,
    }
}
```

## Next Steps

- [Theming](./theming.md) - Style your custom widgets consistently
- [Widget Guide](../guides/widgets.md) - Deep dive into the widget system
- [Signals Guide](../guides/signals.md) - Advanced signal/slot patterns
