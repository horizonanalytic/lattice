# Widget Catalog

A reference of all built-in widgets in Horizon Lattice.

## Basic Widgets

### Label
Displays read-only text.

```rust,ignore
let label = Label::new("Hello, World!");
label.set_alignment(TextAlign::Center);
```

### Button
A clickable push button.

```rust,ignore
let button = Button::new("Click Me");
button.clicked().connect(|_| println!("Clicked!"));
```

### CheckBox
A toggleable checkbox with optional label.

```rust,ignore
let checkbox = CheckBox::new("Enable feature");
checkbox.toggled().connect(|&checked| {
    println!("Checked: {}", checked);
});
```

### RadioButton
Mutually exclusive option buttons.

```rust,ignore
let mut group = ButtonGroup::new();
group.add_button(RadioButton::new("Option A"));
group.add_button(RadioButton::new("Option B"));
```

## Input Widgets

### TextEdit
Single or multi-line text input.

### SpinBox
Numeric input with increment/decrement.

### Slider
Continuous value selection.

### ComboBox
Dropdown selection.

## Container Widgets

### Container
Generic container for child widgets.

### ScrollArea
Scrollable content area.

### TabWidget
Tabbed container.

### Splitter
Resizable split view.

## Display Widgets

### ProgressBar
Progress indication.

### ImageView
Image display with scaling.

### TreeView
Hierarchical data display.

### ListView
List data display.

### TableView
Tabular data display.

---

> **Note**: This catalog is under construction. See the [API documentation](https://docs.rs/horizon-lattice) for complete details.
