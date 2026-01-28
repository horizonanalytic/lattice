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

---

> **Note**: This reference is under construction. See the [API documentation](https://docs.rs/horizon-lattice) for complete details.
