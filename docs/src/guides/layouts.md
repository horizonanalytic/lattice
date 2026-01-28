# Layouts Guide

Layouts automatically arrange child widgets within a container.

## Layout Algorithm

Layouts use a two-pass algorithm:

1. **Measure pass**: Query each child's `size_hint()` and size policy
2. **Arrange pass**: Assign positions and sizes to children

## Content Margins

All layouts support content margins - spacing between the layout's content and its edges:

```rust
use horizon_lattice::widget::layout::ContentMargins;

// Create uniform margins (same on all sides)
let uniform = ContentMargins::uniform(10.0);
assert_eq!(uniform.left, 10.0);
assert_eq!(uniform.top, 10.0);
assert_eq!(uniform.right, 10.0);
assert_eq!(uniform.bottom, 10.0);

// Create symmetric margins (horizontal/vertical)
let symmetric = ContentMargins::symmetric(20.0, 10.0);
assert_eq!(symmetric.left, 20.0);
assert_eq!(symmetric.right, 20.0);
assert_eq!(symmetric.top, 10.0);
assert_eq!(symmetric.bottom, 10.0);

// Create custom margins
let custom = ContentMargins::new(5.0, 10.0, 15.0, 20.0);
assert_eq!(custom.horizontal(), 20.0); // left + right
assert_eq!(custom.vertical(), 30.0);   // top + bottom
```

## LayoutKind Enum

The `LayoutKind` enum provides a unified interface for all layout types:

```rust
use horizon_lattice::widget::layout::LayoutKind;

// Create different layout types
let hbox = LayoutKind::horizontal();
let vbox = LayoutKind::vertical();
let grid = LayoutKind::grid();
let form = LayoutKind::form();
let stack = LayoutKind::stack();
let flow = LayoutKind::flow();
let anchor = LayoutKind::anchor();

// All layouts share a common interface
let mut layout = LayoutKind::vertical();
assert_eq!(layout.item_count(), 0);
assert!(layout.is_empty());
```

## BoxLayout (HBox and VBox)

Arrange widgets horizontally or vertically:

```rust
use horizon_lattice::widget::layout::{BoxLayout, ContentMargins, Orientation};

// Create a horizontal layout
let mut hbox = BoxLayout::horizontal();
hbox.set_spacing(10.0);  // Space between widgets
hbox.set_content_margins(ContentMargins::uniform(8.0));  // Outer margins

assert_eq!(hbox.spacing(), 10.0);
assert_eq!(hbox.orientation(), Orientation::Horizontal);

// Create a vertical layout
let mut vbox = BoxLayout::vertical();
vbox.set_spacing(5.0);

assert_eq!(vbox.orientation(), Orientation::Vertical);
```

## Adding Items to Layouts

Layouts can contain widgets, spacers, and nested layouts:

```rust
use horizon_lattice::widget::layout::{LayoutKind, LayoutItem, SpacerItem, SpacerType};
use horizon_lattice::render::Size;

let mut layout = LayoutKind::vertical();

// Add a fixed spacer (takes a specific amount of space)
let fixed_spacer = LayoutItem::Spacer(SpacerItem::fixed(Size::new(0.0, 20.0)));
layout.add_item(fixed_spacer);

// Add an expanding spacer (fills available space)
let expanding_spacer = LayoutItem::Spacer(SpacerItem::new(
    Size::ZERO,
    SpacerType::Expanding,
));
layout.add_item(expanding_spacer);

assert_eq!(layout.item_count(), 2);
```

## GridLayout

Arrange widgets in rows and columns:

```rust
use horizon_lattice::widget::layout::GridLayout;

let mut grid = GridLayout::new();

// Set spacing between cells
grid.set_spacing(10.0);
grid.set_horizontal_spacing(15.0);  // Override horizontal only
grid.set_vertical_spacing(5.0);     // Override vertical only

// Set column stretch factors (column 1 expands more)
grid.set_column_stretch(0, 0);  // Column 0: no stretch
grid.set_column_stretch(1, 1);  // Column 1: stretch factor 1

// Set row stretch
grid.set_row_stretch(0, 0);  // Row 0: no stretch
grid.set_row_stretch(1, 2);  // Row 1: stretch factor 2

// Set minimum column width
grid.set_column_minimum_width(0, 100.0);
```

## FormLayout

Convenient layout for label-field pairs:

```rust
use horizon_lattice::widget::layout::{FormLayout, RowWrapPolicy, FieldGrowthPolicy};

let mut form = FormLayout::new();

// Configure form behavior
form.set_row_wrap_policy(RowWrapPolicy::WrapLongRows);
form.set_field_growth_policy(FieldGrowthPolicy::ExpandingFieldsGrow);

// Set spacing
form.set_horizontal_spacing(10.0);
form.set_vertical_spacing(8.0);

// The form automatically aligns labels and fields
// Labels go in the left column, fields in the right
```

## StackLayout

Stack widgets on top of each other (only one visible at a time):

```rust
use horizon_lattice::widget::layout::{StackLayout, StackSizeMode};

let mut stack = StackLayout::new();

// Configure how the stack calculates its size
stack.set_size_mode(StackSizeMode::CurrentWidgetSize);  // Size based on current widget
// or
stack.set_size_mode(StackSizeMode::MaximumSize);  // Size based on largest widget

// Set the current index (which widget is visible)
stack.set_current_index(0);
assert_eq!(stack.current_index(), 0);
```

## FlowLayout

Arrange widgets in a flowing pattern (like text wrapping):

```rust
use horizon_lattice::widget::layout::FlowLayout;

let mut flow = FlowLayout::new();

// Set spacing between items
flow.set_spacing(10.0);
flow.set_horizontal_spacing(15.0);  // Between items in a row
flow.set_vertical_spacing(8.0);     // Between rows
```

## AnchorLayout

Position widgets relative to each other or the container:

```rust
use horizon_lattice::widget::layout::{AnchorLayout, Anchor, AnchorLine, AnchorTarget};
use horizon_lattice_core::ObjectId;

let mut anchor = AnchorLayout::new();

// Anchors connect widget edges to targets
// For example: widget's left edge to parent's left edge plus margin
let left_anchor = Anchor {
    line: AnchorLine::Left,
    target: AnchorTarget::Parent,
    target_line: AnchorLine::Left,
    margin: 10.0,
};

// Center horizontally in parent
let center_anchor = Anchor {
    line: AnchorLine::HorizontalCenter,
    target: AnchorTarget::Parent,
    target_line: AnchorLine::HorizontalCenter,
    margin: 0.0,
};
```

## Nested Layouts

Layouts can be nested for complex UIs:

```rust
use horizon_lattice::widget::layout::{LayoutKind, BoxLayout, ContentMargins};

// Main vertical layout
let mut main = LayoutKind::vertical();

// Header as a horizontal layout
let mut header = BoxLayout::horizontal();
header.set_spacing(10.0);
header.set_content_margins(ContentMargins::uniform(5.0));

// Convert to LayoutKind for nesting
let header_kind = LayoutKind::from(header);

// In a real app, you would add the header layout as an item
// to the main layout
```

## Layout Invalidation

Layouts track when they need recalculation:

```rust
use horizon_lattice::widget::layout::LayoutKind;

let mut layout = LayoutKind::vertical();

// Check if layout needs recalculation
if layout.needs_recalculation() {
    println!("Layout needs to be recalculated");
}

// Invalidate to force recalculation
layout.invalidate();
assert!(layout.needs_recalculation());
```

## Layout Geometry

Set and query layout geometry:

```rust
use horizon_lattice::widget::layout::LayoutKind;
use horizon_lattice::render::Rect;

let mut layout = LayoutKind::vertical();

// Set the layout's bounding rectangle
let rect = Rect::new(10.0, 20.0, 300.0, 400.0);
layout.set_geometry(rect);

// Query the geometry
let geom = layout.geometry();
assert_eq!(geom.origin.x, 10.0);
assert_eq!(geom.origin.y, 20.0);
assert_eq!(geom.size.width, 300.0);
assert_eq!(geom.size.height, 400.0);
```

## Custom Layout Implementation

The `Layout` trait can be implemented for custom behavior:

```rust
use horizon_lattice::widget::layout::{Layout, ContentMargins};
use horizon_lattice::widget::SizeHint;
use horizon_lattice::render::{Rect, Size};

// Conceptual example of a custom layout
struct CenteredLayout {
    geometry: Rect,
    margins: ContentMargins,
    spacing: f32,
}

impl CenteredLayout {
    fn new() -> Self {
        Self {
            geometry: Rect::ZERO,
            margins: ContentMargins::uniform(0.0),
            spacing: 6.0,
        }
    }

    fn set_spacing(&mut self, spacing: f32) {
        self.spacing = spacing;
    }

    fn set_content_margins(&mut self, margins: ContentMargins) {
        self.margins = margins;
    }

    // Calculate where to position a child to center it
    fn center_rect(&self, child_size: Size) -> Rect {
        let available_width = self.geometry.width() - self.margins.horizontal();
        let available_height = self.geometry.height() - self.margins.vertical();

        let x = self.margins.left + (available_width - child_size.width) / 2.0;
        let y = self.margins.top + (available_height - child_size.height) / 2.0;

        Rect::new(x.max(self.margins.left), y.max(self.margins.top),
                  child_size.width.min(available_width),
                  child_size.height.min(available_height))
    }
}

// Example usage
let mut layout = CenteredLayout::new();
layout.set_content_margins(ContentMargins::uniform(10.0));

// Simulate setting geometry and centering a 100x50 widget
layout.geometry = Rect::new(0.0, 0.0, 400.0, 300.0);
let centered = layout.center_rect(Size::new(100.0, 50.0));

// The widget should be centered
assert!((centered.origin.x - 160.0).abs() < 0.01);  // (400-20-100)/2 + 10
assert!((centered.origin.y - 135.0).abs() < 0.01); // (300-20-50)/2 + 10
```

## Default Layout Constants

The layout system provides sensible defaults:

```rust
use horizon_lattice::widget::layout::{DEFAULT_SPACING, DEFAULT_MARGINS};

// Default spacing between items
assert_eq!(DEFAULT_SPACING, 6.0);

// Default content margins
assert_eq!(DEFAULT_MARGINS.left, 9.0);
assert_eq!(DEFAULT_MARGINS.top, 9.0);
assert_eq!(DEFAULT_MARGINS.right, 9.0);
assert_eq!(DEFAULT_MARGINS.bottom, 9.0);
```

## Best Practices

1. **Use appropriate layouts** - VBox/HBox for linear arrangements, Grid for tables, Form for input forms
2. **Set size policies** - Help layouts make better decisions about space distribution
3. **Use stretch factors** - Control how extra space is distributed between widgets
4. **Nest layouts** - Combine simple layouts for complex UIs rather than using one complex layout
5. **Set minimum sizes** - Prevent layouts from shrinking widgets too small
6. **Use spacers** - Add flexible space to push widgets apart or fill gaps

See the [Layout Reference](../reference/layouts.md) for all layout types.
