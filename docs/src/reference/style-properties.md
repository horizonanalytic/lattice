# Style Properties Reference

All CSS properties supported by Horizon Lattice.

## Box Model

### margin
Outer spacing around the widget.

```css
margin: 10px;           /* All sides */
margin: 10px 20px;      /* Vertical, horizontal */
margin: 10px 20px 15px 25px;  /* Top, right, bottom, left */
```

### padding
Inner spacing within the widget.

```css
padding: 8px;
padding: 8px 16px;
```

### border-width
Border thickness.

```css
border-width: 1px;
```

### border-color
Border color.

```css
border-color: #333;
border-color: rgb(51, 51, 51);
```

### border-style
Border line style.

```css
border-style: solid;
border-style: none;
```

### border-radius
Corner rounding.

```css
border-radius: 4px;
border-radius: 4px 8px;  /* TL/BR, TR/BL */
```

## Colors

### color
Text color.

```css
color: white;
color: #ffffff;
color: rgb(255, 255, 255);
color: rgba(255, 255, 255, 0.8);
```

### background-color
Background fill color.

```css
background-color: #3498db;
background-color: transparent;
```

## Typography

### font-size
Text size.

```css
font-size: 14px;
font-size: 1.2em;
```

### font-weight
Text weight.

```css
font-weight: normal;
font-weight: bold;
font-weight: 500;
```

### font-style
Text style.

```css
font-style: normal;
font-style: italic;
```

### font-family
Font selection.

```css
font-family: "Helvetica Neue", sans-serif;
```

### text-align
Horizontal text alignment.

```css
text-align: left;
text-align: center;
text-align: right;
```

### line-height
Line spacing multiplier.

```css
line-height: 1.5;
```

## Effects

### opacity
Transparency (0.0 to 1.0).

```css
opacity: 0.8;
```

### box-shadow
Drop shadow.

```css
box-shadow: 2px 2px 4px rgba(0, 0, 0, 0.2);
box-shadow: 0 4px 8px #00000033;
```

## Sizing

### width, height
Explicit dimensions.

```css
width: 200px;
height: 100px;
```

### min-width, min-height
Minimum dimensions.

```css
min-width: 50px;
min-height: 24px;
```

### max-width, max-height
Maximum dimensions.

```css
max-width: 400px;
max-height: 300px;
```

## Interaction

### cursor
Mouse cursor style.

```css
cursor: pointer;
cursor: default;
cursor: text;
```

### pointer-events
Enable/disable mouse interaction.

```css
pointer-events: auto;
pointer-events: none;
```

## Special Values

### inherit
Inherit value from parent.

```css
color: inherit;
font-size: inherit;
```

### initial
Reset to default value.

```css
margin: initial;
```

## Flexbox Properties

### display

Set layout mode.

```css
display: flex;
display: block;
display: none;
```

### flex-direction

Direction of flex items.

```css
flex-direction: row;
flex-direction: column;
flex-direction: row-reverse;
flex-direction: column-reverse;
```

### justify-content

Alignment along main axis.

```css
justify-content: flex-start;
justify-content: flex-end;
justify-content: center;
justify-content: space-between;
justify-content: space-around;
```

### align-items

Alignment along cross axis.

```css
align-items: flex-start;
align-items: flex-end;
align-items: center;
align-items: stretch;
```

### flex

Flex grow, shrink, and basis.

```css
flex: 1;
flex: 0 0 auto;
flex: 1 1 200px;
```

### gap

Space between flex/grid items.

```css
gap: 8px;
gap: 8px 16px;  /* row-gap column-gap */
```

## Transitions

### transition

Animate property changes.

```css
transition: background-color 0.2s ease;
transition: all 0.3s ease-in-out;
transition: opacity 0.15s, transform 0.2s;
```

### transition-property

Properties to animate.

```css
transition-property: background-color;
transition-property: all;
transition-property: none;
```

### transition-duration

Animation duration.

```css
transition-duration: 0.2s;
transition-duration: 200ms;
```

### transition-timing-function

Easing function.

```css
transition-timing-function: ease;
transition-timing-function: ease-in;
transition-timing-function: ease-out;
transition-timing-function: ease-in-out;
transition-timing-function: linear;
transition-timing-function: cubic-bezier(0.4, 0, 0.2, 1);
```

## Pseudo-Classes

Horizon Lattice supports these pseudo-classes for state-based styling:

```css
/* Mouse states */
Button:hover { background-color: #eee; }
Button:pressed { background-color: #ddd; }

/* Focus states */
LineEdit:focus { border-color: #3498db; }
LineEdit:focus-visible { outline: 2px solid #3498db; }

/* Enabled/disabled */
Button:disabled { opacity: 0.5; }
Button:enabled { opacity: 1.0; }

/* Checked state (for checkboxes, radio buttons) */
CheckBox:checked { color: #27ae60; }
CheckBox:unchecked { color: #999; }
CheckBox:indeterminate { color: #666; }

/* First/last child */
ListItem:first-child { border-top: none; }
ListItem:last-child { border-bottom: none; }

/* Nth child */
TableRow:nth-child(odd) { background-color: #f9f9f9; }
TableRow:nth-child(even) { background-color: #fff; }
TableRow:nth-child(3n) { font-weight: bold; }
```

## Widget-Specific Properties

### Subcontrol styling

Some widgets have subcontrols that can be styled:

```css
/* Scrollbar */
ScrollBar::handle { background-color: #888; }
ScrollBar::handle:hover { background-color: #555; }
ScrollBar::add-line { background-color: #ddd; }
ScrollBar::sub-line { background-color: #ddd; }

/* Tab */
TabBar::tab { padding: 8px 16px; }
TabBar::tab:selected { background-color: white; }

/* Checkbox indicator */
CheckBox::indicator { width: 16px; height: 16px; }
CheckBox::indicator:checked { background-color: #3498db; }

/* ComboBox dropdown */
ComboBox::drop-down { width: 20px; }
ComboBox::down-arrow { image: url(down-arrow.png); }
```

## Color Functions

### rgb / rgba

```css
color: rgb(255, 128, 0);
color: rgba(255, 128, 0, 0.5);
```

### hsl / hsla

```css
color: hsl(200, 80%, 50%);
color: hsla(200, 80%, 50%, 0.8);
```

### color-mix

Blend two colors.

```css
background-color: color-mix(in srgb, blue 30%, white);
```

## Custom Properties (Variables)

### Definition

```css
:root {
    --primary-color: #3498db;
    --spacing-unit: 8px;
    --border-radius: 4px;
}
```

### Usage

```css
Button {
    background-color: var(--primary-color);
    padding: var(--spacing-unit);
    border-radius: var(--border-radius);
}
```

### Fallback values

```css
color: var(--undefined-color, #333);
```

## Units

| Unit | Description | Example |
|------|-------------|---------|
| `px` | Pixels (device-independent) | `16px` |
| `em` | Relative to parent font size | `1.5em` |
| `rem` | Relative to root font size | `1rem` |
| `%` | Percentage of parent | `50%` |
| `vw` | Viewport width percentage | `100vw` |
| `vh` | Viewport height percentage | `50vh` |

## Shorthand Properties

### margin / padding

```css
/* Single value: all sides */
margin: 10px;

/* Two values: vertical horizontal */
margin: 10px 20px;

/* Three values: top horizontal bottom */
margin: 10px 20px 15px;

/* Four values: top right bottom left */
margin: 10px 20px 15px 25px;
```

### border

```css
/* width style color */
border: 1px solid #333;

/* Individual sides */
border-top: 2px dashed red;
border-left: none;
```

### background

```css
/* color image position/size repeat */
background: #fff url(bg.png) center/cover no-repeat;
```
