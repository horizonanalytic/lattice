# Widgets Guide

This guide covers the widget system in depth.

## Widget Trait

Every widget implements the `Widget` trait:

```rust,ignore
pub trait Widget {
    fn widget_base(&self) -> &WidgetBase;
    fn widget_base_mut(&mut self) -> &mut WidgetBase;

    fn size_hint(&self) -> SizeHint { SizeHint::default() }
    fn paint(&self, ctx: &mut PaintContext<'_>) {}
    fn event(&mut self, event: &mut WidgetEvent) -> bool { false }
}
```

## Creating Custom Widgets

```rust,ignore
struct ProgressBar {
    base: WidgetBase,
    value: f32,      // 0.0 to 1.0
    color: Color,
}

impl ProgressBar {
    pub fn new() -> Self {
        Self {
            base: WidgetBase::new::<Self>(),
            value: 0.0,
            color: Color::from_rgb8(52, 152, 219),
        }
    }

    pub fn set_value(&mut self, value: f32) {
        self.value = value.clamp(0.0, 1.0);
        self.base.request_repaint();
    }
}

impl Widget for ProgressBar {
    fn widget_base(&self) -> &WidgetBase { &self.base }
    fn widget_base_mut(&mut self) -> &mut WidgetBase { &mut self.base }

    fn size_hint(&self) -> SizeHint {
        SizeHint::from_dimensions(200.0, 20.0)
            .with_minimum_dimensions(50.0, 10.0)
    }

    fn paint(&self, ctx: &mut PaintContext<'_>) {
        let rect = ctx.rect();

        // Background
        ctx.renderer().fill_rect(rect, Color::from_rgb8(200, 200, 200));

        // Progress fill
        let fill_width = rect.width() * self.value;
        let fill_rect = Rect::new(rect.x(), rect.y(), fill_width, rect.height());
        ctx.renderer().fill_rect(fill_rect, self.color);
    }
}
```

## Widget Lifecycle

1. `new()` - Create widget with WidgetBase
2. Configure properties and connect signals
3. Add to parent/layout
4. `show()` is called (inherited from parent)
5. `paint()` called when visible
6. `event()` called for input
7. Widget dropped when parent is destroyed

## Size Hints and Policies

Size hints tell the layout system your widget's preferred dimensions:

```rust,ignore
fn size_hint(&self) -> SizeHint {
    SizeHint::new()
        .with_preferred(Size::new(100.0, 30.0))
        .with_minimum(Size::new(50.0, 20.0))
        .with_maximum(Size::new(200.0, 50.0))
}
```

Size policies control how widgets grow/shrink:
- `Fixed` - Uses preferred size exactly
- `Minimum` - Can grow, won't shrink below minimum
- `Maximum` - Can shrink, won't grow above maximum
- `Preferred` - Prefers hint, can grow or shrink
- `Expanding` - Actively wants more space

## Painting

The `paint()` method receives a `PaintContext` with:
- `renderer()` - The 2D renderer
- `rect()` - Widget's bounds in local coordinates
- `style()` - Computed style properties

```rust,ignore
fn paint(&self, ctx: &mut PaintContext<'_>) {
    let renderer = ctx.renderer();
    let rect = ctx.rect();

    // Fill background
    renderer.fill_rect(rect, ctx.style().background_color);

    // Draw border
    if let Some(border) = ctx.style().border {
        renderer.stroke_rect(rect, border.color, border.width);
    }

    // Draw text
    renderer.draw_text(&self.text, rect.center(), ctx.style().color);
}
```

## Event Handling

Handle events by implementing `event()`:

```rust,ignore
fn event(&mut self, event: &mut WidgetEvent) -> bool {
    match event {
        WidgetEvent::MouseEnter => {
            self.hovered = true;
            self.base.request_repaint();
            true
        }
        WidgetEvent::MouseLeave => {
            self.hovered = false;
            self.base.request_repaint();
            true
        }
        WidgetEvent::MousePress(e) if e.button() == MouseButton::Left => {
            self.pressed = true;
            event.accept();
            true
        }
        WidgetEvent::MouseRelease(e) if e.button() == MouseButton::Left => {
            if self.pressed {
                self.pressed = false;
                self.clicked.emit(());
            }
            true
        }
        _ => false,
    }
}
```

## Built-in Widgets

See the [Widget Catalog](../reference/widgets.md) for all available widgets.
