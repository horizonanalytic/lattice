# Widget Catalog

A comprehensive reference of all built-in widgets in Horizon Lattice.

## Basic Widgets

### Label

Displays read-only text with optional rich text formatting.

```rust,ignore
use horizon_lattice::widget::widgets::Label;

let label = Label::new("Hello, World!");
label.set_alignment(TextAlign::Center);
label.set_word_wrap(true);
label.set_selectable(true);  // Allow text selection
```

**Key Properties:**
- `text` - The displayed text
- `alignment` - Text alignment (Left, Center, Right)
- `word_wrap` - Enable word wrapping
- `selectable` - Allow text selection
- `elide_mode` - How to elide overflow (None, Left, Middle, Right)

### PushButton

A clickable push button with optional icon.

```rust,ignore
use horizon_lattice::widget::widgets::{PushButton, ButtonVariant};

let button = PushButton::new("Click Me")
    .with_variant(ButtonVariant::Primary)
    .with_icon(Icon::from_name("check"))
    .with_default(true);  // Responds to Enter key

button.clicked().connect(|_| println!("Clicked!"));
```

**Variants:**
- `Primary` - Prominent action button
- `Secondary` - Default button style
- `Outlined` - Border-only button
- `Text` - Text-only button
- `Danger` - Destructive action button

**Signals:**
- `clicked` - Emitted when button is clicked
- `pressed` - Emitted when button is pressed down
- `released` - Emitted when button is released

### CheckBox

A toggleable checkbox with optional label.

```rust,ignore
use horizon_lattice::widget::widgets::CheckBox;

let checkbox = CheckBox::new("Enable feature");
checkbox.set_tristate(true);  // Allow indeterminate state

checkbox.toggled().connect(|&checked| {
    println!("Checked: {}", checked);
});
```

**Properties:**
- `text` - Checkbox label
- `checked` - Current checked state
- `tristate` - Enable three-state mode (checked, unchecked, indeterminate)
- `check_state` - Full state (Unchecked, PartiallyChecked, Checked)

### RadioButton

Mutually exclusive option buttons within a group.

```rust,ignore
use horizon_lattice::widget::widgets::{RadioButton, ButtonGroup};

let mut group = ButtonGroup::new();
let opt_a = RadioButton::new("Option A");
let opt_b = RadioButton::new("Option B");
let opt_c = RadioButton::new("Option C");

group.add_button(opt_a.clone());
group.add_button(opt_b.clone());
group.add_button(opt_c.clone());

group.button_clicked.connect(|id| {
    println!("Selected option: {}", id);
});
```

### ToolButton

Compact button typically used in toolbars.

```rust,ignore
use horizon_lattice::widget::widgets::{ToolButton, ToolButtonStyle};

let tool_btn = ToolButton::new()
    .with_icon(Icon::from_name("bold"))
    .with_text("Bold")
    .with_style(ToolButtonStyle::IconOnly)
    .with_checkable(true);
```

**Styles:**
- `IconOnly` - Show only icon
- `TextOnly` - Show only text
- `TextBesideIcon` - Text next to icon
- `TextUnderIcon` - Text below icon

## Input Widgets

### LineEdit

Single-line text input field.

```rust,ignore
use horizon_lattice::widget::widgets::{LineEdit, EchoMode};

let edit = LineEdit::new();
edit.set_placeholder("Enter your name...");
edit.set_max_length(50);
edit.set_echo_mode(EchoMode::Password);

edit.text_changed.connect(|text| {
    println!("Text: {}", text);
});

edit.return_pressed.connect(|| {
    println!("Enter pressed!");
});
```

**Echo Modes:**
- `Normal` - Display text as entered
- `Password` - Display bullets/asterisks
- `NoEcho` - Display nothing
- `PasswordEchoOnEdit` - Show while typing

### TextEdit

Multi-line text editing widget with rich text support.

```rust,ignore
use horizon_lattice::widget::widgets::TextEdit;

let editor = TextEdit::new();
editor.set_text("Initial content");
editor.set_word_wrap(true);
editor.set_read_only(false);
editor.set_tab_stop_width(4);

// Editing operations
editor.undo();
editor.redo();
editor.cut();
editor.copy();
editor.paste();
editor.select_all();
```

**Signals:**
- `text_changed` - Content changed
- `cursor_position_changed` - Cursor moved
- `selection_changed` - Selection changed

### SpinBox

Numeric input with increment/decrement buttons.

```rust,ignore
use horizon_lattice::widget::widgets::SpinBox;

let spin = SpinBox::new();
spin.set_range(0, 100);
spin.set_value(50);
spin.set_step(5);
spin.set_prefix("$");
spin.set_suffix(" USD");
spin.set_wrapping(true);  // Wrap around at limits

spin.value_changed.connect(|&value| {
    println!("Value: {}", value);
});
```

### DoubleSpinBox

Floating-point numeric input.

```rust,ignore
use horizon_lattice::widget::widgets::DoubleSpinBox;

let spin = DoubleSpinBox::new();
spin.set_range(0.0, 1.0);
spin.set_value(0.5);
spin.set_decimals(2);
spin.set_step(0.1);
```

### Slider

Continuous value selection via draggable handle.

```rust,ignore
use horizon_lattice::widget::widgets::{Slider, Orientation};

let slider = Slider::new();
slider.set_orientation(Orientation::Horizontal);
slider.set_range(0, 100);
slider.set_value(50);
slider.set_tick_position(TickPosition::Below);
slider.set_tick_interval(10);

slider.value_changed.connect(|&value| {
    println!("Slider: {}", value);
});
```

### ComboBox

Dropdown selection widget.

```rust,ignore
use horizon_lattice::widget::widgets::ComboBox;

let combo = ComboBox::new();
combo.add_items(&["Option 1", "Option 2", "Option 3"]);
combo.set_current_index(0);
combo.set_editable(true);  // Allow custom input
combo.set_placeholder("Select...");

combo.current_index_changed.connect(|&index| {
    println!("Selected index: {}", index);
});
```

### DateEdit / TimeEdit / DateTimeEdit

Date and time input widgets.

```rust,ignore
use horizon_lattice::widget::widgets::{DateEdit, TimeEdit, DateTimeEdit};
use horizon_lattice::core::{Date, Time, DateTime};

let date = DateEdit::new();
date.set_date(Date::today());
date.set_display_format("yyyy-MM-dd");
date.set_calendar_popup(true);

let time = TimeEdit::new();
time.set_time(Time::current());
time.set_display_format("HH:mm:ss");

let datetime = DateTimeEdit::new();
datetime.set_datetime(DateTime::now());
```

### ColorButton

Color selection button with color picker dialog.

```rust,ignore
use horizon_lattice::widget::widgets::ColorButton;
use horizon_lattice::render::Color;

let color_btn = ColorButton::new();
color_btn.set_color(Color::from_rgb8(255, 128, 0));
color_btn.set_show_alpha(true);

color_btn.color_changed.connect(|color| {
    println!("Selected: {:?}", color);
});
```

### FontComboBox

Font family selection.

```rust,ignore
use horizon_lattice::widget::widgets::FontComboBox;

let font_combo = FontComboBox::new();
font_combo.set_current_font("Helvetica");

font_combo.font_changed.connect(|family| {
    println!("Font: {}", family);
});
```

## Container Widgets

### Container

Generic container for arranging child widgets with a layout.

```rust,ignore
use horizon_lattice::widget::widgets::Container;
use horizon_lattice::widget::layout::{VBoxLayout, LayoutKind};

let mut container = Container::new();
let mut layout = VBoxLayout::new();
layout.add_widget(widget1.object_id());
layout.add_widget(widget2.object_id());
container.set_layout(LayoutKind::from(layout));
```

### ScrollArea

Scrollable container for content larger than visible area.

```rust,ignore
use horizon_lattice::widget::widgets::{ScrollArea, ScrollBarPolicy};

let mut scroll = ScrollArea::new();
scroll.set_widget(content.object_id());
scroll.set_horizontal_policy(ScrollBarPolicy::AsNeeded);
scroll.set_vertical_policy(ScrollBarPolicy::AlwaysOn);
scroll.set_widget_resizable(true);

// Programmatic scrolling
scroll.scroll_to(0, 500);
scroll.ensure_visible(widget.object_id());
```

### TabWidget

Tabbed container showing one page at a time.

```rust,ignore
use horizon_lattice::widget::widgets::TabWidget;

let mut tabs = TabWidget::new();
tabs.add_tab("General", general_page);
tabs.add_tab("Advanced", advanced_page);
tabs.set_tab_position(TabPosition::Top);
tabs.set_tabs_closable(true);
tabs.set_movable(true);

tabs.current_changed.connect(|&index| {
    println!("Tab switched to: {}", index);
});

tabs.tab_close_requested.connect(|&index| {
    tabs.remove_tab(index);
});
```

### Splitter

Resizable split view between widgets.

```rust,ignore
use horizon_lattice::widget::widgets::Splitter;

let mut splitter = Splitter::new();
splitter.add_widget(left_panel.object_id());
splitter.add_widget(right_panel.object_id());
splitter.set_sizes(&[200, 400]);
splitter.set_collapsible(0, true);  // First panel can collapse

splitter.splitter_moved.connect(|&(pos, index)| {
    println!("Splitter {} moved to {}", index, pos);
});
```

### GroupBox

Titled container with optional checkbox.

```rust,ignore
use horizon_lattice::widget::widgets::GroupBox;

let mut group = GroupBox::new("Options");
group.set_checkable(true);
group.set_checked(true);
group.set_layout(LayoutKind::from(layout));

group.toggled.connect(|&enabled| {
    println!("Group enabled: {}", enabled);
});
```

### StackWidget

Shows one widget at a time, like a deck of cards.

```rust,ignore
use horizon_lattice::widget::widgets::StackWidget;

let mut stack = StackWidget::new();
stack.add_widget(page1.object_id());
stack.add_widget(page2.object_id());
stack.add_widget(page3.object_id());
stack.set_current_index(0);

// Switch pages
stack.set_current_widget(page2.object_id());
```

## Display Widgets

### ProgressBar

Progress indication for long operations.

```rust,ignore
use horizon_lattice::widget::widgets::ProgressBar;

let progress = ProgressBar::new();
progress.set_range(0, 100);
progress.set_value(50);
progress.set_text_visible(true);
progress.set_format("%v / %m (%p%)");

// Indeterminate mode
progress.set_range(0, 0);
```

### ImageWidget

Image display with scaling modes.

```rust,ignore
use horizon_lattice::widget::widgets::{ImageWidget, ImageScaleMode};

let image = ImageWidget::new();
image.set_source_file("photo.jpg");
image.set_scale_mode(ImageScaleMode::Fit);
image.set_alignment(Alignment::Center);

// Or from data
image.set_source_data(&image_bytes, "png");
```

**Scale Modes:**
- `None` - Display at actual size
- `Fit` - Scale to fit, preserve aspect ratio
- `Fill` - Scale to fill, may crop
- `Stretch` - Stretch to fill, ignore aspect ratio
- `Tile` - Tile the image

### ListView

List data display with item selection.

```rust,ignore
use horizon_lattice::widget::widgets::ListView;
use horizon_lattice::model::{ListModel, SelectionMode};

let model = ListModel::new(vec!["Item 1", "Item 2", "Item 3"]);
let list = ListView::new().with_model(model);
list.set_selection_mode(SelectionMode::Extended);

list.clicked.connect(|index| {
    println!("Clicked row: {}", index.row());
});

list.double_clicked.connect(|index| {
    println!("Double-clicked: {}", index.row());
});
```

### TreeView

Hierarchical data display with expandable nodes.

```rust,ignore
use horizon_lattice::widget::widgets::TreeView;
use horizon_lattice::model::TreeModel;

let model = TreeModel::new();
let root = model.add_root("Root");
model.add_child(root, "Child 1");
model.add_child(root, "Child 2");

let tree = TreeView::new().with_model(model);
tree.set_root_decorated(true);
tree.set_items_expandable(true);

tree.expanded.connect(|index| {
    println!("Expanded: {:?}", index);
});
```

### TableView

Tabular data display with rows and columns.

```rust,ignore
use horizon_lattice::widget::widgets::TableView;
use horizon_lattice::model::TableModel;

let model = TableModel::new(vec![
    vec!["A1", "B1", "C1"],
    vec!["A2", "B2", "C2"],
]);
model.set_headers(&["Column A", "Column B", "Column C"]);

let table = TableView::new().with_model(model);
table.set_column_width(0, 100);
table.set_row_height(30);
table.set_grid_visible(true);
table.set_alternating_row_colors(true);
```

## Window Widgets

### Window

Basic application window.

```rust,ignore
use horizon_lattice::widget::widgets::Window;

let mut window = Window::new("My App")
    .with_size(800.0, 600.0)
    .with_position(100.0, 100.0);

window.set_content_widget(content.object_id());
window.show();
```

### MainWindow

Application window with menu bar, toolbars, and status bar.

```rust,ignore
use horizon_lattice::widget::widgets::{MainWindow, MenuBar, ToolBar, StatusBar};

let mut main = MainWindow::new("My App")
    .with_size(1024.0, 768.0);

main.set_menu_bar(menu_bar);
main.add_tool_bar(tool_bar);
main.set_central_widget(content.object_id());
main.set_status_bar(status_bar);
main.show();
```

### Dialog

Modal dialog window.

```rust,ignore
use horizon_lattice::widget::widgets::{Dialog, DialogButtonBox, StandardButton};

let mut dialog = Dialog::new("Confirm")
    .with_size(400.0, 200.0);

let buttons = DialogButtonBox::new()
    .with_standard_buttons(StandardButton::Ok | StandardButton::Cancel);

dialog.set_content_widget(content.object_id());
dialog.set_button_box(buttons);

// Show modally and get result
match dialog.exec() {
    DialogResult::Accepted => println!("OK clicked"),
    DialogResult::Rejected => println!("Cancelled"),
}

// Or show non-modally
dialog.open();
dialog.accepted.connect(|| { /* ... */ });
dialog.rejected.connect(|| { /* ... */ });
```

### MessageBox

Standard message dialogs.

```rust,ignore
use horizon_lattice::widget::widgets::{MessageBox, MessageIcon, StandardButton};

// Information message
MessageBox::information("Info", "Operation completed successfully.");

// Question with buttons
let result = MessageBox::question(
    "Confirm",
    "Are you sure you want to delete this file?",
    StandardButton::Yes | StandardButton::No
);

// Warning
MessageBox::warning("Warning", "This action cannot be undone.");

// Error
MessageBox::critical("Error", "Failed to save file.");
```

### FileDialog

Native file open/save dialogs.

```rust,ignore
use horizon_lattice::widget::widgets::{FileDialog, FileFilter};

// Open single file
let filters = vec![
    FileFilter::new("Images", &["png", "jpg", "gif"]),
    FileFilter::all_files(),
];

if let Some(path) = FileDialog::get_open_file_name("Open Image", "", &filters) {
    println!("Selected: {:?}", path);
}

// Open multiple files
if let Some(paths) = FileDialog::get_open_file_names("Open Files", "", &filters) {
    for path in paths {
        println!("Selected: {:?}", path);
    }
}

// Save file
if let Some(path) = FileDialog::get_save_file_name("Save As", "", &filters) {
    println!("Save to: {:?}", path);
}

// Select directory
if let Some(dir) = FileDialog::get_existing_directory("Choose Directory", "") {
    println!("Directory: {:?}", dir);
}
```

## Menu Widgets

### MenuBar

Application menu bar.

```rust,ignore
use horizon_lattice::widget::widgets::{MenuBar, Menu};

let mut menu_bar = MenuBar::new();
menu_bar.add_menu(file_menu);
menu_bar.add_menu(edit_menu);
menu_bar.add_menu(help_menu);
```

### Menu

Dropdown menu with actions and submenus.

```rust,ignore
use horizon_lattice::widget::widgets::{Menu, Action};

let mut file_menu = Menu::new("File");
file_menu.add_action(Action::new("New").with_shortcut("Ctrl+N"));
file_menu.add_action(Action::new("Open...").with_shortcut("Ctrl+O"));
file_menu.add_separator();
file_menu.add_menu(recent_files_menu);  // Submenu
file_menu.add_separator();
file_menu.add_action(Action::new("Quit").with_shortcut("Ctrl+Q"));
```

### Action

Reusable command with text, icon, and shortcut.

```rust,ignore
use horizon_lattice::widget::widgets::Action;

let save_action = Action::new("Save")
    .with_shortcut("Ctrl+S")
    .with_icon(Icon::from_name("save"))
    .with_enabled(true);

save_action.triggered.connect(|| {
    println!("Save triggered!");
});

// Checkable action
let word_wrap = Action::new("Word Wrap")
    .with_checkable(true)
    .with_checked(true);

word_wrap.toggled.connect(|&checked| {
    editor.set_word_wrap(checked);
});
```

## Toolbar Widgets

### ToolBar

Application toolbar.

```rust,ignore
use horizon_lattice::widget::widgets::ToolBar;

let mut toolbar = ToolBar::new("Main");
toolbar.add_action(new_action);
toolbar.add_action(open_action);
toolbar.add_separator();
toolbar.add_widget(search_box.object_id());
toolbar.set_movable(true);
toolbar.set_floatable(true);
```

## Status Bar

### StatusBar

Window status bar.

```rust,ignore
use horizon_lattice::widget::widgets::StatusBar;

let status = StatusBar::new();
status.show_message("Ready");
status.show_message_for("Saved!", Duration::from_secs(3));

// Permanent widgets
status.add_permanent_widget(progress.object_id());
status.add_permanent_widget(position_label.object_id());
```
