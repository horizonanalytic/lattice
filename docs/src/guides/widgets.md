# Widgets Guide

This guide covers the widget system in depth.

## Widget Trait

Every widget implements the `Widget` trait:

```rust,no_run
use horizon_lattice::widget::{Widget, WidgetBase, PaintContext};
use horizon_lattice::widget::SizeHint;
use horizon_lattice::widget::events::WidgetEvent;
use horizon_lattice_core::{Object, ObjectId};

pub trait WidgetDefinition {
    fn widget_base(&self) -> &WidgetBase;
    fn widget_base_mut(&mut self) -> &mut WidgetBase;

    fn size_hint(&self) -> SizeHint { SizeHint::default() }
    fn paint(&self, ctx: &mut PaintContext<'_>) {}
    fn event(&mut self, event: &mut WidgetEvent) -> bool { false }
}
```

## Size Hints

Size hints tell layouts what size a widget prefers:

```rust
use horizon_lattice::widget::SizeHint;
use horizon_lattice::render::Size;

// Create a simple size hint with preferred dimensions
let hint = SizeHint::from_dimensions(100.0, 30.0);
assert_eq!(hint.preferred, Size::new(100.0, 30.0));

// Add minimum and maximum constraints
let constrained = SizeHint::from_dimensions(100.0, 30.0)
    .with_minimum_dimensions(50.0, 20.0)
    .with_maximum_dimensions(200.0, 50.0);

assert_eq!(constrained.minimum, Some(Size::new(50.0, 20.0)));
assert_eq!(constrained.maximum, Some(Size::new(200.0, 50.0)));

// Create a fixed size (cannot grow or shrink)
let fixed = SizeHint::fixed(Size::new(100.0, 100.0));
assert!(fixed.is_fixed());
```

## Size Policies

Size policies control how widgets grow and shrink:

```rust
use horizon_lattice::widget::{SizePolicy, SizePolicyPair};

// Fixed - cannot resize
let fixed = SizePolicyPair::fixed();
assert!(!fixed.horizontal.can_grow());
assert!(!fixed.horizontal.can_shrink());

// Preferred - can grow or shrink, prefers hint size
let preferred = SizePolicyPair::preferred();
assert!(preferred.horizontal.can_grow());
assert!(preferred.horizontal.can_shrink());

// Expanding - actively wants more space
let expanding = SizePolicyPair::expanding();
assert!(expanding.horizontal.wants_to_grow());

// Custom policy with stretch factor
let stretched = SizePolicyPair::new(SizePolicy::Expanding, SizePolicy::Fixed)
    .with_horizontal_stretch(2);  // Gets 2x extra space compared to stretch=1

assert_eq!(stretched.horizontal_stretch, 2);
```

## Widget Lifecycle

1. `new()` - Create widget with WidgetBase
2. Configure properties and connect signals
3. Add to parent/layout
4. `show()` is called (inherited from parent)
5. `paint()` called when visible
6. `event()` called for input
7. Widget dropped when parent is destroyed

## Creating Custom Widgets

Here's a conceptual example of creating a custom progress bar widget:

```rust,no_run
use horizon_lattice::widget::{Widget, WidgetBase, PaintContext};
use horizon_lattice::widget::SizeHint;
use horizon_lattice::widget::events::WidgetEvent;
use horizon_lattice::render::{Color, Rect};
use horizon_lattice_core::{Object, ObjectId};

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
        self.base.update(); // Request repaint
    }

    pub fn value(&self) -> f32 {
        self.value
    }
}

impl Object for ProgressBar {
    fn object_id(&self) -> ObjectId {
        self.base.object_id()
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
        let fill_rect = Rect::new(0.0, 0.0, fill_width, rect.height());
        ctx.renderer().fill_rect(fill_rect, self.color);
    }
}
```

## Size Hint Examples

Different widgets have different size hint patterns:

```rust
use horizon_lattice::widget::SizeHint;
use horizon_lattice::render::Size;

// Label - prefers text size, can't shrink below it
fn label_size_hint(text_width: f32, text_height: f32) -> SizeHint {
    SizeHint::from_dimensions(text_width, text_height)
        .with_minimum_dimensions(text_width, text_height)
}

// Button - has padding around content
fn button_size_hint(content_width: f32, content_height: f32) -> SizeHint {
    let padding = 16.0;
    SizeHint::from_dimensions(content_width + padding, content_height + padding)
        .with_minimum_dimensions(60.0, 30.0)
}

// Text input - can expand horizontally
fn text_input_size_hint() -> SizeHint {
    SizeHint::from_dimensions(150.0, 30.0)
        .with_minimum_dimensions(50.0, 30.0)
}
```

## Geometry Methods

Widgets provide methods to query and set their geometry:

```rust
use horizon_lattice::render::{Point, Rect, Size};

// Simulating widget geometry operations
let geometry = Rect::new(10.0, 20.0, 100.0, 50.0);

// Position (relative to parent)
let pos = geometry.origin;
assert_eq!(pos, Point::new(10.0, 20.0));

// Size
let size = geometry.size;
assert_eq!(size, Size::new(100.0, 50.0));

// Local rect (always at origin 0,0)
let local_rect = Rect::new(0.0, 0.0, size.width, size.height);
assert_eq!(local_rect.origin, Point::new(0.0, 0.0));

// Check if a point is inside the local rect
let point = Point::new(50.0, 25.0);
assert!(local_rect.contains(point));

let outside = Point::new(150.0, 25.0);
assert!(!local_rect.contains(outside));
```

## Coordinate Mapping

Map points between widget-local and parent coordinate systems:

```rust
use horizon_lattice::render::Point;

// Widget at position (10, 20)
let widget_pos = Point::new(10.0, 20.0);

// Point in widget-local coordinates
let local_point = Point::new(5.0, 5.0);

// Map to parent coordinates
let parent_point = Point::new(
    local_point.x + widget_pos.x,
    local_point.y + widget_pos.y,
);
assert_eq!(parent_point, Point::new(15.0, 25.0));

// Map from parent back to local
let back_to_local = Point::new(
    parent_point.x - widget_pos.x,
    parent_point.y - widget_pos.y,
);
assert_eq!(back_to_local, local_point);
```

## Visibility and Enabled State

Control widget visibility and interaction:

```rust
// Visibility concepts
let mut visible = true;
let mut enabled = true;

// Hide a widget
visible = false;

// Disable a widget (grayed out, can't interact)
enabled = false;

// Check effective state (considering parent hierarchy)
// If parent is hidden, child is effectively hidden too
fn is_effectively_visible(self_visible: bool, parent_visible: bool) -> bool {
    self_visible && parent_visible
}

assert!(!is_effectively_visible(true, false));  // Parent hidden
assert!(!is_effectively_visible(false, true));  // Self hidden
assert!(is_effectively_visible(true, true));    // Both visible
```

## Focus Policy

Control how widgets receive keyboard focus:

```rust
use horizon_lattice::widget::FocusPolicy;

// NoFocus - widget cannot receive focus (e.g., labels)
let no_focus = FocusPolicy::NoFocus;

// TabFocus - focus via Tab key only (e.g., read-only controls)
let tab_focus = FocusPolicy::TabFocus;

// ClickFocus - focus via mouse click only
let click_focus = FocusPolicy::ClickFocus;

// StrongFocus - focus via both Tab and click (e.g., buttons, text fields)
let strong_focus = FocusPolicy::StrongFocus;
```

## Repaint Requests

Request widget repainting when content changes:

```rust
use horizon_lattice::render::Rect;

// Full repaint - entire widget needs redrawing
fn request_full_repaint(needs_repaint: &mut bool) {
    *needs_repaint = true;
}

// Partial repaint - only a region needs redrawing
fn request_partial_repaint(dirty_region: &mut Option<Rect>, new_dirty: Rect) {
    *dirty_region = Some(match dirty_region {
        Some(existing) => existing.union(&new_dirty),
        None => new_dirty,
    });
}

let mut dirty = None;
request_partial_repaint(&mut dirty, Rect::new(0.0, 0.0, 50.0, 50.0));
request_partial_repaint(&mut dirty, Rect::new(40.0, 40.0, 50.0, 50.0));

// Dirty region is now the union of both rects
let combined = dirty.unwrap();
assert_eq!(combined.origin.x, 0.0);
assert_eq!(combined.origin.y, 0.0);
```

## Signals and Properties

Widgets use signals to notify of changes:

```rust
use horizon_lattice_core::{Signal, Property};

// Create signals for widget state changes
let visible_changed: Signal<bool> = Signal::new();
let geometry_changed: Signal<(f32, f32, f32, f32)> = Signal::new();

// Connect to signals
visible_changed.connect(|&visible| {
    println!("Visibility changed to: {}", visible);
});

// Emit when state changes
visible_changed.emit(false);

// Properties with automatic change notification
let value: Property<f32> = Property::new(0.0);

// Get the current value
assert_eq!(value.get(), 0.0);

// Set returns true if value changed
assert!(value.set(0.5));
assert!(!value.set(0.5)); // Same value, returns false
```

## Built-in Widgets

See the [Widget Catalog](../reference/widgets.md) for all available widgets including:

- **Basic**: Label, PushButton, CheckBox, RadioButton
- **Input**: LineEdit, TextEdit, SpinBox, Slider
- **Containers**: Frame, GroupBox, ScrollArea, TabWidget
- **Display**: ProgressBar, StatusBar
- **Dialogs**: MessageBox, FileDialog, ColorDialog
