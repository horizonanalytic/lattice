# Layout Reference

A comprehensive reference of all layout types available in Horizon Lattice.

## Common Concepts

### ContentMargins

All layouts support content margins - the space between the layout edge and its contents.

```rust,ignore
use horizon_lattice::widget::layout::ContentMargins;

// Uniform margins on all sides
let margins = ContentMargins::uniform(16.0);

// Symmetric margins (vertical, horizontal)
let margins = ContentMargins::symmetric(8.0, 16.0);

// Individual margins (left, top, right, bottom)
let margins = ContentMargins::new(10.0, 8.0, 10.0, 12.0);

// Apply to layout
layout.set_content_margins(margins);
```

### LayoutKind

Layouts must be wrapped in `LayoutKind` when setting on a container.

```rust,ignore
use horizon_lattice::widget::layout::{VBoxLayout, LayoutKind};
use horizon_lattice::widget::widgets::Container;

let mut layout = VBoxLayout::new();
// Configure layout...

let mut container = Container::new();
container.set_layout(LayoutKind::from(layout));
```

### Widget IDs

Layouts reference widgets by their `ObjectId`, obtained via `widget.object_id()`.

```rust,ignore
let button = PushButton::new("Click");
layout.add_widget(button.object_id());
```

## Box Layouts

### HBoxLayout

Arranges widgets horizontally from left to right.

```rust,ignore
use horizon_lattice::widget::layout::HBoxLayout;

let mut layout = HBoxLayout::new();
layout.set_spacing(8.0);
layout.set_content_margins(ContentMargins::uniform(10.0));

// Add widgets in order
layout.add_widget(icon.object_id());
layout.add_widget(label.object_id());
layout.add_stretch(1);  // Flexible space
layout.add_widget(button.object_id());
```

**Methods:**
- `set_spacing(f32)` - Space between widgets
- `add_widget(ObjectId)` - Add widget at end
- `add_stretch(i32)` - Add flexible space with stretch factor
- `insert_widget(usize, ObjectId)` - Insert at position
- `insert_stretch(usize, i32)` - Insert stretch at position

### VBoxLayout

Arranges widgets vertically from top to bottom.

```rust,ignore
use horizon_lattice::widget::layout::VBoxLayout;

let mut layout = VBoxLayout::new();
layout.set_spacing(12.0);

layout.add_widget(title.object_id());
layout.add_widget(content.object_id());
layout.add_stretch(1);  // Push buttons to bottom
layout.add_widget(buttons.object_id());
```

### BoxLayout (Generic)

Base box layout with configurable orientation.

```rust,ignore
use horizon_lattice::widget::layout::{BoxLayout, Orientation};

let mut layout = BoxLayout::new(Orientation::Horizontal);
layout.set_spacing(8.0);
```

### Alignment

Control item alignment within box layouts.

```rust,ignore
use horizon_lattice::widget::layout::Alignment;

// Add widget with specific alignment
layout.add_widget_with_alignment(
    widget.object_id(),
    Alignment::Center
);
```

**Alignment Values:**
- `Leading` - Left/Top aligned
- `Center` - Centered
- `Trailing` - Right/Bottom aligned
- `Fill` - Stretch to fill (default)

## GridLayout

Arranges widgets in a two-dimensional grid.

```rust,ignore
use horizon_lattice::widget::layout::GridLayout;

let mut grid = GridLayout::new();
grid.set_horizontal_spacing(8.0);
grid.set_vertical_spacing(8.0);

// Add widgets at specific positions (row, column)
grid.add_widget_at(label1.object_id(), 0, 0);
grid.add_widget_at(input1.object_id(), 0, 1);
grid.add_widget_at(label2.object_id(), 1, 0);
grid.add_widget_at(input2.object_id(), 1, 1);

// Widget spanning multiple cells
grid.add_widget_spanning(
    wide_widget.object_id(),
    2,  // row
    0,  // column
    1,  // row span
    2   // column span
);
```

### Row and Column Configuration

```rust,ignore
// Set minimum row/column sizes
grid.set_row_minimum_height(0, 30.0);
grid.set_column_minimum_width(1, 200.0);

// Set stretch factors (relative sizing)
grid.set_row_stretch(0, 1);
grid.set_row_stretch(1, 2);  // Second row gets 2x space
grid.set_column_stretch(0, 0);
grid.set_column_stretch(1, 1);
```

### Cell Alignment

```rust,ignore
use horizon_lattice::widget::layout::CellAlignment;

grid.add_widget_at_aligned(
    widget.object_id(),
    0, 0,
    CellAlignment::new(Alignment::Center, Alignment::Center)
);
```

## FormLayout

Two-column layout optimized for label-field pairs.

```rust,ignore
use horizon_lattice::widget::layout::FormLayout;

let mut form = FormLayout::new();

// Add label-field pairs
form.add_row(Label::new("Name:"), name_input);
form.add_row(Label::new("Email:"), email_input);
form.add_row(Label::new("Password:"), password_input);

// Field spanning full width
form.add_spanning_widget(remember_me_checkbox);

// Just a field (no label)
form.add_row_field_only(submit_button);
```

### Form Policies

```rust,ignore
use horizon_lattice::widget::layout::{RowWrapPolicy, FieldGrowthPolicy};

// Row wrap policy: when to wrap long labels
form.set_row_wrap_policy(RowWrapPolicy::WrapLongRows);

// Field growth: how fields expand
form.set_field_growth_policy(FieldGrowthPolicy::ExpandingFieldsGrow);
```

**RowWrapPolicy:**
- `DontWrapRows` - Never wrap, may clip
- `WrapLongRows` - Wrap labels that don't fit
- `WrapAllRows` - Always put labels above fields

**FieldGrowthPolicy:**
- `FieldsStayAtSizeHint` - Fields stay at preferred size
- `ExpandingFieldsGrow` - Only expanding fields grow
- `AllNonFixedFieldsGrow` - All non-fixed fields grow

### Row Access

```rust,ignore
// Get form row by index
let row = form.row_at(0);
row.set_visible(false);  // Hide entire row

// Remove a row
form.remove_row(1);
```

## StackLayout

Shows one widget at a time (like pages in a wizard).

```rust,ignore
use horizon_lattice::widget::layout::{StackLayout, StackSizeMode};

let mut stack = StackLayout::new();

// Add pages
let page1_id = stack.add_widget(intro_page.object_id());
let page2_id = stack.add_widget(settings_page.object_id());
let page3_id = stack.add_widget(confirm_page.object_id());

// Switch pages
stack.set_current_index(0);
stack.set_current_widget(settings_page.object_id());

// Size mode
stack.set_size_mode(StackSizeMode::CurrentWidgetSize);
```

**StackSizeMode:**
- `StackFitLargestWidget` - Size to fit largest child
- `CurrentWidgetSize` - Size to fit current child only

## FlowLayout

Arranges widgets in rows, wrapping to new lines as needed.

```rust,ignore
use horizon_lattice::widget::layout::FlowLayout;

let mut flow = FlowLayout::new();
flow.set_horizontal_spacing(8.0);
flow.set_vertical_spacing(8.0);

// Add items - they flow and wrap automatically
for tag in tags {
    flow.add_widget(TagWidget::new(tag).object_id());
}
```

**Use Cases:**
- Tag clouds
- Photo galleries
- Toolbar buttons that wrap

## AnchorLayout

Position widgets relative to parent or sibling edges.

```rust,ignore
use horizon_lattice::widget::layout::{AnchorLayout, Anchor, AnchorLine, AnchorTarget};

let mut anchor = AnchorLayout::new();

// Add widgets
anchor.add_widget(sidebar.object_id());
anchor.add_widget(content.object_id());
anchor.add_widget(footer.object_id());

// Anchor sidebar to parent left edge
anchor.add_anchor(Anchor::new(
    AnchorTarget::Widget(sidebar.object_id()),
    AnchorLine::Left,
    AnchorTarget::Parent,
    AnchorLine::Left,
    10.0  // margin
));

// Anchor sidebar to parent top
anchor.add_anchor(Anchor::new(
    AnchorTarget::Widget(sidebar.object_id()),
    AnchorLine::Top,
    AnchorTarget::Parent,
    AnchorLine::Top,
    10.0
));

// Anchor content to sidebar's right edge
anchor.add_anchor(Anchor::new(
    AnchorTarget::Widget(content.object_id()),
    AnchorLine::Left,
    AnchorTarget::Widget(sidebar.object_id()),
    AnchorLine::Right,
    8.0
));
```

**AnchorLine Values:**
- `Left`, `Right`, `Top`, `Bottom` - Edges
- `HorizontalCenter`, `VerticalCenter` - Centers

### Fill Anchors

```rust,ignore
// Make widget fill parent horizontally
anchor.fill_horizontal(widget.object_id(), 10.0);

// Make widget fill parent vertically
anchor.fill_vertical(widget.object_id(), 10.0);

// Make widget fill parent completely
anchor.fill(widget.object_id(), ContentMargins::uniform(10.0));
```

## Layout Items

### SpacerItem

Flexible or fixed space in layouts.

```rust,ignore
use horizon_lattice::widget::layout::{LayoutItem, SpacerItem, SpacerType};

// Fixed size spacer
let spacer = SpacerItem::fixed(20.0, 0.0);

// Expanding spacer (flexible)
let spacer = SpacerItem::expanding(SpacerType::Horizontal);

// Add to layout
layout.add_item(LayoutItem::Spacer(spacer));
```

**SpacerType:**
- `Horizontal` - Expands horizontally
- `Vertical` - Expands vertically
- `Both` - Expands in both directions

## Nested Layouts

Layouts can be nested for complex arrangements.

```rust,ignore
use horizon_lattice::widget::layout::{VBoxLayout, HBoxLayout, LayoutKind};

// Create button row
let mut buttons = HBoxLayout::new();
buttons.set_spacing(8.0);
buttons.add_stretch(1);
buttons.add_widget(cancel_btn.object_id());
buttons.add_widget(ok_btn.object_id());

let mut buttons_container = Container::new();
buttons_container.set_layout(LayoutKind::from(buttons));

// Main layout with nested button row
let mut main = VBoxLayout::new();
main.add_widget(content.object_id());
main.add_widget(buttons_container.object_id());
```

## Layout Invalidation

Force layouts to recalculate.

```rust,ignore
use horizon_lattice::widget::layout::{LayoutInvalidator, InvalidationScope};

// Invalidate specific widget's layout
LayoutInvalidator::invalidate(widget.object_id());

// Invalidate with specific scope
LayoutInvalidator::invalidate_with_scope(
    widget.object_id(),
    InvalidationScope::SizeHint
);
```

**InvalidationScope:**
- `Geometry` - Recalculate positions and sizes
- `SizeHint` - Recalculate size hints
- `All` - Full recalculation

## Default Values

```rust,ignore
use horizon_lattice::widget::layout::{DEFAULT_SPACING, DEFAULT_MARGINS};

// DEFAULT_SPACING = 6.0
// DEFAULT_MARGINS = ContentMargins::uniform(9.0)
```

## Layout Trait

For custom layouts, implement the `Layout` trait.

```rust,ignore
use horizon_lattice::widget::layout::{Layout, ContentMargins};
use horizon_lattice::widget::SizeHint;
use horizon_lattice::render::{Rect, Size};

pub struct MyCustomLayout {
    geometry: Rect,
    margins: ContentMargins,
    // ...
}

impl Layout for MyCustomLayout {
    fn size_hint(&self) -> SizeHint {
        // Calculate preferred size
        SizeHint::preferred(Size::new(200.0, 100.0))
    }

    fn set_geometry(&mut self, rect: Rect) {
        self.geometry = rect;
        // Position child widgets
    }

    fn geometry(&self) -> Rect {
        self.geometry
    }

    fn content_margins(&self) -> ContentMargins {
        self.margins
    }

    fn set_content_margins(&mut self, margins: ContentMargins) {
        self.margins = margins;
    }
}
```
