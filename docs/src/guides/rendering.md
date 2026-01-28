# Rendering and Graphics

This guide covers the rendering primitives and graphics operations in Horizon Lattice.

## Geometry Types

### Points and Sizes

The fundamental types for positioning and dimensions:

```rust
use horizon_lattice::render::{Point, Size};

// Creating points
let origin = Point::ZERO;
let p1 = Point::new(100.0, 50.0);
let p2: Point = (200.0, 100.0).into();

// Point arithmetic
let offset = Point::new(10.0, 5.0);
let moved = Point::new(p1.x + offset.x, p1.y + offset.y);

// Creating sizes
let size = Size::new(800.0, 600.0);
let empty = Size::ZERO;

// Check if empty
assert!(empty.is_empty());
assert!(!size.is_empty());

// From integer dimensions
let from_u32: Size = Size::from((1920u32, 1080u32));
```

### Rectangles

Rectangles define regions for layout and drawing:

```rust
use horizon_lattice::render::{Rect, Point, Size};

// Create from origin and size
let rect = Rect::new(10.0, 20.0, 200.0, 100.0);

// Create from points
let from_points = Rect::from_points(
    Point::new(10.0, 20.0),
    Point::new(210.0, 120.0),
);

// Access properties
assert_eq!(rect.x, 10.0);
assert_eq!(rect.y, 20.0);
assert_eq!(rect.width, 200.0);
assert_eq!(rect.height, 100.0);

// Corner accessors
let top_left = rect.origin();
let bottom_right = rect.bottom_right();
let center = rect.center();

// Point containment
let point = Point::new(50.0, 50.0);
assert!(rect.contains(point));

// Rectangle operations
let other = Rect::new(100.0, 50.0, 200.0, 100.0);
if let Some(intersection) = rect.intersection(&other) {
    println!("Overlapping area: {:?}", intersection);
}

let bounding = rect.union(&other);

// Inflate/deflate (grow or shrink)
let padded = rect.inflate(10.0, 10.0);
let inset = rect.deflate(5.0, 5.0);

// Offset (move)
let moved = rect.offset(50.0, 25.0);
```

### Rounded Rectangles

For drawing rectangles with rounded corners:

```rust
use horizon_lattice::render::{Rect, RoundedRect, CornerRadii};

let rect = Rect::new(0.0, 0.0, 200.0, 100.0);

// Uniform corner radius
let uniform = RoundedRect::new(rect, CornerRadii::uniform(8.0));

// Per-corner radii (top-left, top-right, bottom-right, bottom-left)
let varied = RoundedRect::with_radii(
    rect,
    10.0,  // top-left
    10.0,  // top-right
    0.0,   // bottom-right (square)
    0.0,   // bottom-left (square)
);

// Check if it's actually rounded
assert!(!uniform.is_rect());

// Access the underlying rect
let bounds = uniform.rect();
```

## Colors

### Creating Colors

Multiple ways to create colors:

```rust
use horizon_lattice::render::Color;

// From RGB (0.0-1.0 range)
let red = Color::from_rgb(1.0, 0.0, 0.0);

// From RGBA with alpha
let semi_transparent = Color::from_rgba(1.0, 0.0, 0.0, 0.5);

// From 8-bit RGB values (0-255)
let blue = Color::from_rgb8(0, 0, 255);
let green_alpha = Color::from_rgba8(0, 255, 0, 128);

// From hex string
let purple = Color::from_hex("#8B5CF6").unwrap();
let with_alpha = Color::from_hex("#8B5CF680").unwrap(); // 50% alpha

// From HSV (hue 0-360, saturation/value 0-1)
let orange = Color::from_hsv(30.0, 1.0, 1.0);

// Predefined constants
let white = Color::WHITE;
let black = Color::BLACK;
let transparent = Color::TRANSPARENT;
```

### Color Operations

```rust
use horizon_lattice::render::Color;

let color = Color::from_rgb(0.2, 0.4, 0.8);

// Modify alpha
let faded = color.with_alpha(0.5);

// Interpolate between colors
let start = Color::RED;
let end = Color::BLUE;
let midpoint = start.lerp(&end, 0.5); // Purple-ish

// Convert to different formats
let [r, g, b, a] = color.to_array();
let (r8, g8, b8, a8) = color.to_rgba8();
let hex = color.to_hex(); // "#3366CC"

// Convert to HSV
let (h, s, v) = color.to_hsv();
```

## Paths

Paths define shapes for filling and stroking.

### Building Paths Manually

```rust
use horizon_lattice::render::{Path, Point};

let mut path = Path::new();

// Move to starting point
path.move_to(Point::new(0.0, 0.0));

// Draw lines
path.line_to(Point::new(100.0, 0.0));
path.line_to(Point::new(100.0, 100.0));
path.line_to(Point::new(0.0, 100.0));

// Close the path (connects back to start)
path.close();

// Get bounding box
let bounds = path.bounds();
```

### Path Factory Methods

Convenient methods for common shapes:

```rust
use horizon_lattice::render::{Path, Rect, Point};

// Rectangle
let rect_path = Path::rect(Rect::new(0.0, 0.0, 100.0, 50.0));

// Rounded rectangle
let rounded = Path::rounded_rect(
    Rect::new(0.0, 0.0, 100.0, 50.0),
    8.0, // corner radius
);

// Circle (center point and radius)
let circle = Path::circle(Point::new(50.0, 50.0), 25.0);

// Ellipse
let ellipse = Path::ellipse(
    Point::new(50.0, 50.0), // center
    40.0,                    // x radius
    25.0,                    // y radius
);

// Line segment
let line = Path::line(
    Point::new(0.0, 0.0),
    Point::new(100.0, 100.0),
);

// Polygon from points
let triangle = Path::polygon(&[
    Point::new(50.0, 0.0),
    Point::new(100.0, 100.0),
    Point::new(0.0, 100.0),
]);

// Star shape
let star = Path::star(
    Point::new(50.0, 50.0), // center
    5,                       // points
    40.0,                    // outer radius
    20.0,                    // inner radius
);
```

### Bezier Curves

```rust
use horizon_lattice::render::{Path, Point};

let mut path = Path::new();
path.move_to(Point::new(0.0, 100.0));

// Quadratic bezier (one control point)
path.quad_to(
    Point::new(50.0, 0.0),   // control point
    Point::new(100.0, 100.0), // end point
);

// Cubic bezier (two control points)
path.move_to(Point::new(0.0, 50.0));
path.cubic_to(
    Point::new(25.0, 0.0),   // control point 1
    Point::new(75.0, 100.0), // control point 2
    Point::new(100.0, 50.0), // end point
);
```

## Transforms

### 2D Transforms

Transform matrices for rotating, scaling, and translating:

```rust
use horizon_lattice::render::{Transform2D, Point};

// Identity (no transformation)
let identity = Transform2D::identity();

// Translation
let translate = Transform2D::translation(100.0, 50.0);

// Scaling
let scale = Transform2D::scale(2.0, 2.0); // 2x size

// Rotation (in radians)
use std::f32::consts::PI;
let rotate = Transform2D::rotation(PI / 4.0); // 45 degrees

// Composing transforms (order matters!)
// This scales first, then rotates, then translates
let combined = Transform2D::identity()
    .then_scale(2.0, 2.0)
    .then_rotate(PI / 4.0)
    .then_translate(100.0, 50.0);

// Transform a point
let point = Point::new(10.0, 20.0);
let transformed = combined.transform_point(point);

// Inverse transform
if let Some(inverse) = combined.inverse() {
    let back = inverse.transform_point(transformed);
    // back ≈ point
}

// Rotation around a specific point
let pivot = Point::new(50.0, 50.0);
let rotate_around = Transform2D::identity()
    .then_translate(-pivot.x, -pivot.y)
    .then_rotate(PI / 2.0)
    .then_translate(pivot.x, pivot.y);
```

### Transform Stack

For hierarchical transforms (like nested widgets):

```rust
use horizon_lattice::render::{TransformStack, Point};

let mut stack = TransformStack::new();

// Save current state
stack.save();

// Apply transforms
stack.translate(100.0, 50.0);
stack.scale(2.0, 2.0);

// Transform points
let local = Point::new(10.0, 10.0);
let world = stack.transform_point(local);

// Restore previous state
stack.restore();

// Point transforms back to original coordinate space
let restored = stack.transform_point(local);
assert_eq!(restored, local);
```

## Painting

### Solid Colors and Gradients

```rust
use horizon_lattice::render::{Paint, Color, GradientStop, Point};

// Solid color fill
let solid = Paint::solid(Color::from_rgb(0.2, 0.4, 0.8));

// Linear gradient
let linear = Paint::linear_gradient(
    Point::new(0.0, 0.0),   // start point
    Point::new(100.0, 0.0), // end point
    vec![
        GradientStop::new(0.0, Color::RED),
        GradientStop::new(0.5, Color::WHITE),
        GradientStop::new(1.0, Color::BLUE),
    ],
);

// Radial gradient
let radial = Paint::radial_gradient(
    Point::new(50.0, 50.0), // center
    50.0,                    // radius
    vec![
        GradientStop::new(0.0, Color::WHITE),
        GradientStop::new(1.0, Color::from_rgba(0.0, 0.0, 0.0, 0.0)),
    ],
);

// Radial gradient with offset focus
let spotlight = Paint::radial_gradient_with_focus(
    Point::new(50.0, 50.0), // center
    50.0,                    // radius
    Point::new(30.0, 30.0), // focus point (off-center)
    vec![
        GradientStop::new(0.0, Color::WHITE),
        GradientStop::new(1.0, Color::BLACK),
    ],
);
```

### Strokes

Configure how paths are outlined:

```rust
use horizon_lattice::render::{Stroke, Color, LineCap, LineJoin, DashPattern};

// Basic stroke
let basic = Stroke::new(Color::BLACK, 2.0);

// With line cap style
let rounded_caps = Stroke::new(Color::BLACK, 10.0)
    .with_cap(LineCap::Round);

// Line cap options:
// - LineCap::Butt   - flat, ends at exact endpoint
// - LineCap::Round  - semicircle extending past endpoint
// - LineCap::Square - square extending past endpoint

// With line join style
let rounded_corners = Stroke::new(Color::BLACK, 4.0)
    .with_join(LineJoin::Round);

// Line join options:
// - LineJoin::Miter - sharp corners (default)
// - LineJoin::Round - rounded corners
// - LineJoin::Bevel - flat corners

// Miter limit (prevents very sharp corners from extending too far)
let limited = Stroke::new(Color::BLACK, 4.0)
    .with_join(LineJoin::Miter)
    .with_miter_limit(2.0);

// Dashed lines
let dashed = Stroke::new(Color::BLACK, 2.0)
    .with_dash(DashPattern::simple(5.0, 5.0));

// Complex dash pattern: long, gap, short, gap
let complex_dash = Stroke::new(Color::BLACK, 2.0)
    .with_dash(DashPattern::new(vec![10.0, 3.0, 3.0, 3.0], 0.0));

// Animated dash (offset shifts the pattern)
let animated = Stroke::new(Color::BLACK, 2.0)
    .with_dash(DashPattern::new(vec![5.0, 5.0], 2.5));
```

## Blend Modes

Control how colors combine when drawing overlapping content:

```rust
use horizon_lattice::render::BlendMode;

// Standard alpha blending (default)
let normal = BlendMode::Normal;

// Darkening modes
let multiply = BlendMode::Multiply; // Darken by multiplying
let darken = BlendMode::Darken;     // Take minimum

// Lightening modes
let screen = BlendMode::Screen;   // Lighten (opposite of multiply)
let lighten = BlendMode::Lighten; // Take maximum
let add = BlendMode::Add;         // Additive (glow effects)

// Porter-Duff compositing
let source = BlendMode::Source;           // Replace destination
let dest_out = BlendMode::DestinationOut; // Cut out shape
let xor = BlendMode::Xor;                 // Either but not both
```

## Fill Rules

Determine what's "inside" a path with overlapping regions:

```rust
use horizon_lattice::render::FillRule;

// NonZero (default) - considers winding direction
// A point is inside if the winding number is non-zero
let non_zero = FillRule::NonZero;

// EvenOdd - creates checkerboard pattern for overlaps
// A point is inside if it crosses an odd number of edges
let even_odd = FillRule::EvenOdd;
```

The difference matters for paths with overlapping regions:
- **NonZero**: Inner shapes are filled if they wind the same direction as outer
- **EvenOdd**: Overlapping regions alternate between filled and unfilled

## Images

### Loading and Using Images

```rust,no_run
use horizon_lattice::render::{ImageLoader, ImageScaleMode};

// Create an image loader
let loader = ImageLoader::new();

// Load an image (async in real usage)
// let image = loader.load("path/to/image.png").await?;

// Scale modes for drawing
let mode = ImageScaleMode::Fit;        // Fit within bounds, preserve aspect
let mode = ImageScaleMode::Fill;       // Fill bounds, may crop
let mode = ImageScaleMode::Stretch;    // Stretch to fill, ignores aspect
let mode = ImageScaleMode::Tile;       // Repeat to fill
let mode = ImageScaleMode::Center;     // Center at original size
let mode = ImageScaleMode::None;       // Draw at original size from top-left
```

### Nine-Patch Images

For scalable UI elements like buttons and panels:

```rust,no_run
use horizon_lattice::render::{NinePatch, Rect};

// Nine-patch divides an image into 9 regions:
// - 4 corners (don't scale)
// - 4 edges (scale in one direction)
// - 1 center (scales in both directions)

// Create with uniform borders
// let nine_patch = NinePatch::uniform(image, 10.0);

// Create with different border sizes
// let nine_patch = NinePatch::new(
//     image,
//     10.0,  // left border
//     10.0,  // right border
//     8.0,   // top border
//     12.0,  // bottom border
// );

// Get minimum size (sum of borders)
// let min_size = nine_patch.min_size();

// Calculate patch regions for rendering
// let dest = Rect::new(0.0, 0.0, 200.0, 60.0);
// let patches = nine_patch.calculate_patches(dest);
```

## Box Shadows

For drop shadows and glow effects:

```rust
use horizon_lattice::render::{BoxShadow, Color, Rect};

// Basic drop shadow
let shadow = BoxShadow {
    offset_x: 2.0,
    offset_y: 4.0,
    blur_radius: 8.0,
    spread_radius: 0.0,
    color: Color::from_rgba(0.0, 0.0, 0.0, 0.3),
    inset: false,
};

// Glow effect (no offset, larger blur)
let glow = BoxShadow {
    offset_x: 0.0,
    offset_y: 0.0,
    blur_radius: 20.0,
    spread_radius: 5.0,
    color: Color::from_rgba(0.3, 0.5, 1.0, 0.6),
    inset: false,
};

// Inset shadow (inner shadow)
let inset = BoxShadow {
    offset_x: 0.0,
    offset_y: 2.0,
    blur_radius: 4.0,
    spread_radius: 0.0,
    color: Color::from_rgba(0.0, 0.0, 0.0, 0.2),
    inset: true,
};

// Calculate the bounding rect needed to render the shadow
let widget_rect = Rect::new(10.0, 10.0, 100.0, 50.0);
let shadow_bounds = shadow.bounds(widget_rect);
```

## Text Rendering

### Font Configuration

```rust
use horizon_lattice::render::text::{Font, FontBuilder, FontWeight, FontStyle};

// Simple font
let font = Font::new("Helvetica", 14.0);

// Using the builder for more options
let custom = FontBuilder::new()
    .family("Inter")
    .fallback("Helvetica")
    .fallback("Arial")
    .fallback("sans-serif")
    .size(16.0)
    .weight(FontWeight::MEDIUM)
    .style(FontStyle::Normal)
    .letter_spacing(0.5)
    .build();

// Enable OpenType features
let with_features = FontBuilder::new()
    .family("Fira Code")
    .size(14.0)
    .feature("liga", 1) // Enable ligatures
    .feature("calt", 1) // Enable contextual alternates
    .build();
```

### Text Layout

```rust
use horizon_lattice::render::text::{
    TextLayoutOptions, HorizontalAlign, VerticalAlign, WrapMode
};

// Basic layout options
let options = TextLayoutOptions::default()
    .with_max_width(Some(300.0))
    .with_wrap_mode(WrapMode::Word);

// Alignment options
let centered = TextLayoutOptions::default()
    .with_horizontal_align(HorizontalAlign::Center)
    .with_vertical_align(VerticalAlign::Middle);

// Wrap modes:
// - WrapMode::None      - No wrapping, single line
// - WrapMode::Char      - Wrap at any character
// - WrapMode::Word      - Wrap at word boundaries
// - WrapMode::WordChar  - Try word, fall back to char

// Line spacing
let spaced = TextLayoutOptions::default()
    .with_line_height(1.5); // 150% line height
```

### Rich Text

```rust
use horizon_lattice::render::text::{TextSpan, TextDecoration};
use horizon_lattice::render::Color;

// Create styled text spans
let spans = vec![
    TextSpan::new("Hello ")
        .with_size(16.0)
        .with_color(Color::BLACK),
    TextSpan::new("World")
        .with_size(16.0)
        .with_color(Color::BLUE)
        .with_weight(700)
        .with_decoration(TextDecoration::Underline),
    TextSpan::new("!")
        .with_size(20.0)
        .with_color(Color::RED),
];
```

## Next Steps

- See the [Widget Guide](./widgets.md) for how rendering integrates with widgets
- See the [Styling Guide](./styling.md) for CSS-like styling of widgets
- Check the [API Documentation](https://docs.rs/horizon-lattice) for complete details
