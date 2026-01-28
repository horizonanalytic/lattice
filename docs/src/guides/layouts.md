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

Creating a custom layout involves implementing the `Layout` trait and optionally using `LayoutBase` to handle common functionality.

### Architecture Overview

Custom layouts in Horizon Lattice follow this architecture:

1. **Layout trait**: Defines 21 methods for item management, size calculation, geometry, and invalidation
2. **LayoutBase**: A helper struct that provides common functionality (item storage, margins, spacing, caching)
3. **LayoutItem**: Enum wrapping widgets, spacers, or nested layouts
4. **Two-pass algorithm**: Collection (size hints) followed by distribution (positioning)

### Step-by-Step: Creating a Centered Layout

Let's build a `CenteredLayout` that centers a single widget within its bounds.

#### Step 1: Define the Struct

```rust
use horizon_lattice::widget::layout::{Layout, LayoutBase, LayoutItem, ContentMargins};
use horizon_lattice::widget::geometry::{SizeHint, SizePolicyPair};
use horizon_lattice::widget::dispatcher::WidgetAccess;
use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

/// A layout that centers its single child widget.
#[derive(Debug, Clone)]
pub struct CenteredLayout {
    /// Delegate common functionality to LayoutBase.
    base: LayoutBase,
}

impl CenteredLayout {
    /// Create a new centered layout.
    pub fn new() -> Self {
        Self {
            base: LayoutBase::new(),
        }
    }
}

impl Default for CenteredLayout {
    fn default() -> Self {
        Self::new()
    }
}
```

#### Step 2: Implement the Layout Trait

The `Layout` trait requires implementing methods across several categories:

**Item Management** - Delegate to LayoutBase:

```rust
impl Layout for CenteredLayout {
    fn add_item(&mut self, item: LayoutItem) {
        // Only allow one item for centering
        if self.base.is_empty() {
            self.base.add_item(item);
        }
    }

    fn insert_item(&mut self, index: usize, item: LayoutItem) {
        if self.base.is_empty() && index == 0 {
            self.base.insert_item(index, item);
        }
    }

    fn remove_item(&mut self, index: usize) -> Option<LayoutItem> {
        self.base.remove_item(index)
    }

    fn remove_widget(&mut self, widget: ObjectId) -> bool {
        self.base.remove_widget(widget)
    }

    fn item_count(&self) -> usize {
        self.base.item_count()
    }

    fn item_at(&self, index: usize) -> Option<&LayoutItem> {
        self.base.item_at(index)
    }

    fn item_at_mut(&mut self, index: usize) -> Option<&mut LayoutItem> {
        self.base.item_at_mut(index)
    }

    fn clear(&mut self) {
        self.base.clear();
    }
    // ... continued below
}
```

**Size Hints** - Calculate based on the child:

```rust
impl Layout for CenteredLayout {
    // ... item management above

    fn size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        // Return cached hint if available
        if let Some(cached) = self.base.cached_size_hint() {
            return cached;
        }

        // Get child's size hint (if we have a child)
        let child_hint = if let Some(item) = self.base.item_at(0) {
            self.base.get_item_size_hint(storage, item)
        } else {
            SizeHint::default()
        };

        // Add margins
        let margins = self.base.content_margins();
        SizeHint {
            preferred: Size::new(
                child_hint.preferred.width + margins.horizontal(),
                child_hint.preferred.height + margins.vertical(),
            ),
            minimum: child_hint.minimum.map(|s| Size::new(
                s.width + margins.horizontal(),
                s.height + margins.vertical(),
            )),
            maximum: child_hint.maximum.map(|s| Size::new(
                s.width + margins.horizontal(),
                s.height + margins.vertical(),
            )),
        }
    }

    fn minimum_size<S: WidgetAccess>(&self, storage: &S) -> Size {
        self.size_hint(storage).effective_minimum()
    }

    fn size_policy(&self) -> SizePolicyPair {
        SizePolicyPair::default()
    }
    // ... continued below
}
```

**Geometry** - Delegate to LayoutBase:

```rust
impl Layout for CenteredLayout {
    // ... size hints above

    fn geometry(&self) -> Rect {
        self.base.geometry()
    }

    fn set_geometry(&mut self, rect: Rect) {
        self.base.set_geometry(rect);
    }

    fn content_margins(&self) -> ContentMargins {
        self.base.content_margins()
    }

    fn set_content_margins(&mut self, margins: ContentMargins) {
        self.base.set_content_margins(margins);
    }

    fn spacing(&self) -> f32 {
        self.base.spacing()
    }

    fn set_spacing(&mut self, spacing: f32) {
        self.base.set_spacing(spacing);
    }
    // ... continued below
}
```

**Layout Calculation** - The core algorithm:

```rust
impl Layout for CenteredLayout {
    // ... geometry above

    fn calculate<S: WidgetAccess>(&mut self, storage: &S, _available: Size) -> Size {
        let content_rect = self.base.content_rect();

        if let Some(item) = self.base.item_at(0) {
            // Get the child's preferred size
            let hint = self.base.get_item_size_hint(storage, item);

            // Constrain to available space
            let child_width = hint.preferred.width.min(content_rect.width());
            let child_height = hint.preferred.height.min(content_rect.height());

            // Calculate centered position
            let x = content_rect.origin.x + (content_rect.width() - child_width) / 2.0;
            let y = content_rect.origin.y + (content_rect.height() - child_height) / 2.0;

            // Store the calculated geometry
            self.base.set_item_geometry(0, Rect::new(x, y, child_width, child_height));
        }

        // Cache size hint for performance
        let hint = self.size_hint(storage);
        self.base.set_cached_size_hint(hint);
        self.base.mark_valid();

        self.base.geometry().size
    }

    fn apply<S: WidgetAccess>(&self, storage: &mut S) {
        // Apply calculated geometry to the widget
        if let Some(item) = self.base.item_at(0) {
            if let Some(geometry) = self.base.item_geometry(0) {
                LayoutBase::apply_item_geometry(storage, item, geometry);
            }
        }
    }
    // ... continued below
}
```

**Invalidation and Ownership** - Delegate to LayoutBase:

```rust
impl Layout for CenteredLayout {
    // ... calculate and apply above

    fn invalidate(&mut self) {
        self.base.invalidate();
    }

    fn needs_recalculation(&self) -> bool {
        self.base.needs_recalculation()
    }

    fn parent_widget(&self) -> Option<ObjectId> {
        self.base.parent_widget()
    }

    fn set_parent_widget(&mut self, parent: Option<ObjectId>) {
        self.base.set_parent_widget(parent);
    }
}
```

### Using LayoutBase Helpers

`LayoutBase` provides several helper methods for implementing layouts:

```rust
use horizon_lattice::widget::layout::LayoutBase;

// In your calculate() implementation:
fn calculate<S: WidgetAccess>(&mut self, storage: &S, available: Size) -> Size {
    // Get the content area (geometry minus margins)
    let content = self.base.content_rect();

    // Iterate over items and check visibility
    for (i, item) in self.base.items().iter().enumerate() {
        // Skip hidden widgets
        if !self.base.is_item_visible(storage, item) {
            continue;
        }

        // Get size hint for this item
        let hint = self.base.get_item_size_hint(storage, item);

        // Get size policy for this item
        let policy = self.base.get_item_size_policy(storage, item);

        // Calculate position and store it
        let rect = Rect::new(/* your calculation */);
        self.base.set_item_geometry(i, rect);
    }

    // Count visible items (for spacing calculations)
    let visible_count = self.base.visible_item_count(storage);

    self.base.mark_valid();
    available
}
```

### Space Distribution

For layouts that distribute space among multiple items, use `LayoutBase::distribute_space`:

```rust
use horizon_lattice::widget::geometry::{SizeHint, SizePolicy};

// Collect item information: (size_hint, policy, stretch_factor)
let items: Vec<(SizeHint, SizePolicy, u8)> = /* gather from items */;

// Calculate totals
let total_hint: f32 = items.iter().map(|(h, _, _)| h.preferred.width).sum();
let total_min: f32 = items.iter().map(|(h, _, _)| h.effective_minimum().width).sum();
let total_max: f32 = items.iter().map(|(h, _, _)| h.effective_maximum().width).sum();

// Distribute available space
let sizes = LayoutBase::distribute_space(
    &items,
    available_width,  // Total available space
    total_hint,       // Sum of preferred sizes
    total_min,        // Sum of minimum sizes
    total_max,        // Sum of maximum sizes
);

// sizes[i] is the width to assign to item i
```

### RTL (Right-to-Left) Support

For horizontal layouts, support RTL text direction:

```rust
fn calculate<S: WidgetAccess>(&mut self, storage: &S, available: Size) -> Size {
    let content = self.base.content_rect();
    let mut x_pos: f32 = 0.0;

    for (i, item) in self.base.items().iter().enumerate() {
        let item_width = /* calculated width */;

        // Mirror x position for RTL layouts
        let x = self.base.mirror_x(x_pos, item_width, content.width());

        let rect = Rect::new(
            content.origin.x + x,
            content.origin.y,
            item_width,
            content.height(),
        );
        self.base.set_item_geometry(i, rect);

        x_pos += item_width + self.base.spacing();
    }

    self.base.mark_valid();
    available
}
```

### Height-for-Width Layouts

Some layouts (like flow layouts) need to adjust height based on available width:

```rust
impl Layout for FlowingLayout {
    fn has_height_for_width(&self) -> bool {
        true
    }

    fn height_for_width<S: WidgetAccess>(&self, storage: &S, width: f32) -> Option<f32> {
        // Calculate how many rows needed at this width
        let mut current_x: f32 = 0.0;
        let mut current_row_height: f32 = 0.0;
        let mut total_height: f32 = 0.0;
        let content_width = width - self.base.content_margins().horizontal();

        for item in self.base.items() {
            let hint = self.base.get_item_size_hint(storage, item);
            let item_width = hint.preferred.width;
            let item_height = hint.preferred.height;

            if current_x + item_width > content_width && current_x > 0.0 {
                // Wrap to next row
                total_height += current_row_height + self.base.spacing();
                current_x = 0.0;
                current_row_height = 0.0;
            }

            current_row_height = current_row_height.max(item_height);
            current_x += item_width + self.base.spacing();
        }

        total_height += current_row_height;
        Some(total_height + self.base.content_margins().vertical())
    }
}
```

### Complete Example: Diagonal Layout

Here's a complete custom layout that arranges items diagonally:

```rust
use horizon_lattice::widget::layout::{Layout, LayoutBase, LayoutItem, ContentMargins};
use horizon_lattice::widget::geometry::{SizeHint, SizePolicyPair};
use horizon_lattice::widget::dispatcher::WidgetAccess;
use horizon_lattice_core::ObjectId;
use horizon_lattice_render::{Rect, Size};

/// Arranges items diagonally from top-left to bottom-right.
#[derive(Debug, Clone)]
pub struct DiagonalLayout {
    base: LayoutBase,
    /// Horizontal offset per item.
    x_offset: f32,
    /// Vertical offset per item.
    y_offset: f32,
}

impl DiagonalLayout {
    pub fn new(x_offset: f32, y_offset: f32) -> Self {
        Self {
            base: LayoutBase::new(),
            x_offset,
            y_offset,
        }
    }
}

impl Layout for DiagonalLayout {
    // Item management - delegate to base
    fn add_item(&mut self, item: LayoutItem) { self.base.add_item(item); }
    fn insert_item(&mut self, index: usize, item: LayoutItem) { self.base.insert_item(index, item); }
    fn remove_item(&mut self, index: usize) -> Option<LayoutItem> { self.base.remove_item(index) }
    fn remove_widget(&mut self, widget: ObjectId) -> bool { self.base.remove_widget(widget) }
    fn item_count(&self) -> usize { self.base.item_count() }
    fn item_at(&self, index: usize) -> Option<&LayoutItem> { self.base.item_at(index) }
    fn item_at_mut(&mut self, index: usize) -> Option<&mut LayoutItem> { self.base.item_at_mut(index) }
    fn clear(&mut self) { self.base.clear(); }

    fn size_hint<S: WidgetAccess>(&self, storage: &S) -> SizeHint {
        let margins = self.base.content_margins();
        let visible_count = self.base.visible_item_count(storage);
        let mut max_width: f32 = 0.0;
        let mut max_height: f32 = 0.0;

        for (i, item) in self.base.items().iter().enumerate() {
            if !self.base.is_item_visible(storage, item) { continue; }
            let hint = self.base.get_item_size_hint(storage, item);
            let x_end = (i as f32) * self.x_offset + hint.preferred.width;
            let y_end = (i as f32) * self.y_offset + hint.preferred.height;
            max_width = max_width.max(x_end);
            max_height = max_height.max(y_end);
        }

        SizeHint::new(Size::new(
            max_width + margins.horizontal(),
            max_height + margins.vertical(),
        ))
    }

    fn minimum_size<S: WidgetAccess>(&self, storage: &S) -> Size {
        self.size_hint(storage).effective_minimum()
    }

    fn size_policy(&self) -> SizePolicyPair { SizePolicyPair::default() }

    // Geometry - delegate to base
    fn geometry(&self) -> Rect { self.base.geometry() }
    fn set_geometry(&mut self, rect: Rect) { self.base.set_geometry(rect); }
    fn content_margins(&self) -> ContentMargins { self.base.content_margins() }
    fn set_content_margins(&mut self, margins: ContentMargins) { self.base.set_content_margins(margins); }
    fn spacing(&self) -> f32 { self.base.spacing() }
    fn set_spacing(&mut self, spacing: f32) { self.base.set_spacing(spacing); }

    fn calculate<S: WidgetAccess>(&mut self, storage: &S, available: Size) -> Size {
        let content = self.base.content_rect();
        let mut visible_index = 0;

        for (i, item) in self.base.items().iter().enumerate() {
            if !self.base.is_item_visible(storage, item) {
                self.base.set_item_geometry(i, Rect::ZERO);
                continue;
            }

            let hint = self.base.get_item_size_hint(storage, item);
            let x = content.origin.x + (visible_index as f32) * self.x_offset;
            let y = content.origin.y + (visible_index as f32) * self.y_offset;

            self.base.set_item_geometry(i, Rect::new(
                x,
                y,
                hint.preferred.width.min(content.width()),
                hint.preferred.height.min(content.height()),
            ));

            visible_index += 1;
        }

        self.base.mark_valid();
        available
    }

    fn apply<S: WidgetAccess>(&self, storage: &mut S) {
        for (i, item) in self.base.items().iter().enumerate() {
            if let Some(geometry) = self.base.item_geometry(i) {
                LayoutBase::apply_item_geometry(storage, item, geometry);
            }
        }
    }

    // Invalidation - delegate to base
    fn invalidate(&mut self) { self.base.invalidate(); }
    fn needs_recalculation(&self) -> bool { self.base.needs_recalculation() }
    fn parent_widget(&self) -> Option<ObjectId> { self.base.parent_widget() }
    fn set_parent_widget(&mut self, parent: Option<ObjectId>) { self.base.set_parent_widget(parent); }
}
```

### Best Practices for Custom Layouts

1. **Always use LayoutBase** - It handles caching, invalidation, and common operations
2. **Mark layout valid after calculation** - Call `self.base.mark_valid()` at the end of `calculate()`
3. **Skip hidden items** - Use `is_item_visible()` to skip hidden widgets
4. **Cache size hints** - Use `set_cached_size_hint()` for performance
5. **Handle empty layouts** - Return early if `item_count() == 0`
6. **Respect size policies** - Use `get_item_size_policy()` to determine if items can grow/shrink
7. **Account for margins** - Use `content_rect()` to get the area inside margins
8. **Test with RTL** - If horizontal, test with RTL text direction

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
