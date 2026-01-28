# Tutorial: Lists and Models

Learn the Model/View architecture for displaying collections of data.

## What You'll Learn

- Understanding the Model/View pattern
- Creating list models
- Displaying data in ListView
- Handling item selection
- Dynamic item operations (add, remove, modify)

## Prerequisites

- Completed the [Forms](./forms.md) tutorial
- Understanding of Rust traits

## The Model/View Architecture

Horizon Lattice separates data (Model) from presentation (View):

- **Model**: Holds the data and emits change signals
- **View**: Displays the data and handles user interaction
- **Selection Model**: Tracks which items are selected

This separation allows:
- Multiple views of the same data
- Efficient updates (only changed items redraw)
- Reusable views with different data sources

## Step 1: Using ListWidget (Simple Approach)

For simple lists, `ListWidget` manages its own data:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{ListWidget, Window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Simple List")
        .with_size(300.0, 400.0);

    // Create list widget with items
    let mut list = ListWidget::new();
    list.add_item("Apple");
    list.add_item("Banana");
    list.add_item("Cherry");
    list.add_item("Date");
    list.add_item("Elderberry");

    // Handle item clicks
    list.item_clicked.connect(|&row| {
        println!("Clicked row: {}", row);
    });

    // Handle selection changes
    list.current_row_changed.connect(|(old, new)| {
        println!("Selection changed from {:?} to {:?}", old, new);
    });

    window.set_content_widget(list.object_id());
    window.show();

    app.run()
}
```

### ListWidget Operations

```rust,ignore
use horizon_lattice::widget::widgets::ListWidget;

let mut list = ListWidget::new();

// Add items
list.add_item("Item 1");
list.add_item("Item 2");

// Insert at specific position
list.insert_item(1, "Inserted Item");

// Remove item
let removed = list.take_item(0);

// Clear all items
list.clear();

// Get current selection
let current_row = list.current_row();

// Set selection programmatically
list.set_current_row(Some(2));

// Get item count
let count = list.count();
```

## Step 2: ListView with ListModel

For more control, use `ListView` with a separate `ListModel`:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{ListView, Window};
use horizon_lattice::model::{ListModel, ListItem, ItemData};

// Define your data item
#[derive(Clone)]
struct Fruit {
    name: String,
    color: String,
}

// Implement ListItem to tell the model how to display it
impl ListItem for Fruit {
    fn display(&self) -> ItemData {
        ItemData::from(&self.name)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Fruit List")
        .with_size(300.0, 400.0);

    // Create model with data
    let model = ListModel::new(vec![
        Fruit { name: "Apple".into(), color: "Red".into() },
        Fruit { name: "Banana".into(), color: "Yellow".into() },
        Fruit { name: "Grape".into(), color: "Purple".into() },
    ]);

    // Create view with model
    let list_view = ListView::new()
        .with_model(model);

    // Handle clicks
    list_view.clicked.connect(|index| {
        println!("Clicked row: {}", index.row());
    });

    // Handle double-clicks
    list_view.double_clicked.connect(|index| {
        println!("Double-clicked row: {}", index.row());
    });

    window.set_content_widget(list_view.object_id());
    window.show();

    app.run()
}
```

## Step 3: Custom Data Display

Use a closure-based extractor for complex display logic:

```rust,ignore
use horizon_lattice::model::{ListModel, ItemData, ItemRole};

#[derive(Clone)]
struct Contact {
    name: String,
    email: String,
    phone: String,
}

// Create model with custom data extraction
let model = ListModel::with_extractor(
    vec![
        Contact {
            name: "Alice".into(),
            email: "alice@example.com".into(),
            phone: "555-1234".into(),
        },
        Contact {
            name: "Bob".into(),
            email: "bob@example.com".into(),
            phone: "555-5678".into(),
        },
    ],
    |contact, role| match role {
        ItemRole::Display => ItemData::from(&contact.name),
        ItemRole::ToolTip => ItemData::from(format!("{}\n{}", contact.email, contact.phone)),
        _ => ItemData::None,
    },
);
```

### Item Roles

Different roles provide different aspects of item data:

| Role | Purpose |
|------|---------|
| `Display` | Main text to show |
| `Decoration` | Icon or image |
| `ToolTip` | Hover tooltip text |
| `Edit` | Value for editing |
| `CheckState` | Checkbox state |
| `BackgroundColor` | Background color |
| `ForegroundColor` | Text color |
| `Font` | Custom font |

## Step 4: Selection Handling

Control how users select items:

```rust,ignore
use horizon_lattice::widget::widgets::ListView;
use horizon_lattice::model::{SelectionMode, SelectionFlags};

let mut list_view = ListView::new()
    .with_model(model)
    .with_selection_mode(SelectionMode::ExtendedSelection);

// Selection modes:
// - NoSelection: Nothing selectable
// - SingleSelection: One item at a time
// - MultiSelection: Ctrl+click for multiple
// - ExtendedSelection: Shift+click for ranges + Ctrl+click

// Get the selection model
let selection = list_view.selection_model();

// Listen for selection changes
selection.selection_changed.connect(|(selected, deselected)| {
    println!("Selection changed!");
    for idx in &selected {
        println!("  Selected: row {}", idx.row());
    }
    for idx in &deselected {
        println!("  Deselected: row {}", idx.row());
    }
});

// Listen for current item changes
selection.current_changed.connect(|(new_index, old_index)| {
    println!("Current changed from {:?} to {:?}",
        old_index.map(|i| i.row()),
        new_index.map(|i| i.row())
    );
});
```

### Programmatic Selection

```rust,ignore
use horizon_lattice::model::{ModelIndex, SelectionFlags};

let selection = list_view.selection_model();

// Select a single item
let index = ModelIndex::new(2, 0);  // Row 2, Column 0
selection.select(index, SelectionFlags::CLEAR_SELECT_CURRENT);

// Select a range
selection.select_range(0, 4, SelectionFlags::CLEAR_AND_SELECT);

// Get selected items
let selected_indices = selection.selected_indices();
let selected_rows = selection.selected_rows();

// Clear selection
selection.clear_selection();

// Check if index is selected
let is_selected = selection.is_selected(index);
```

## Step 5: Dynamic List Operations

Add, remove, and modify items dynamically:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    ListView, PushButton, LineEdit, Container, Window
};
use horizon_lattice::widget::layout::{VBoxLayout, HBoxLayout, LayoutKind};
use horizon_lattice::model::ListModel;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Dynamic List")
        .with_size(400.0, 500.0);

    // Shared model (wrapped in Arc<Mutex> for thread-safe access)
    let model = Arc::new(Mutex::new(ListModel::new(vec![
        "Item 1".to_string(),
        "Item 2".to_string(),
        "Item 3".to_string(),
    ])));

    // Create view
    let list_view = ListView::new()
        .with_model(model.lock().unwrap().clone());

    // Input field for new items
    let mut input = LineEdit::new();
    input.set_placeholder("Enter new item...");

    // Buttons
    let add_btn = PushButton::new("Add");
    let remove_btn = PushButton::new("Remove Selected");
    let clear_btn = PushButton::new("Clear All");

    // Connect Add button
    let model_clone = model.clone();
    let input_clone = input.clone();
    add_btn.clicked().connect(move |_| {
        let text = input_clone.text();
        if !text.is_empty() {
            model_clone.lock().unwrap().push(text.clone());
            input_clone.set_text("");
        }
    });

    // Connect Remove button
    let model_clone = model.clone();
    let list_clone = list_view.clone();
    remove_btn.clicked().connect(move |_| {
        let selection = list_clone.selection_model();
        let mut rows: Vec<usize> = selection.selected_rows();
        // Remove from highest to lowest to avoid index shifting
        rows.sort_by(|a, b| b.cmp(a));
        for row in rows {
            model_clone.lock().unwrap().remove(row);
        }
    });

    // Connect Clear button
    let model_clone = model.clone();
    clear_btn.clicked().connect(move |_| {
        model_clone.lock().unwrap().clear();
    });

    // Layout
    let mut button_row = HBoxLayout::new();
    button_row.set_spacing(8.0);
    button_row.add_widget(add_btn.object_id());
    button_row.add_widget(remove_btn.object_id());
    button_row.add_widget(clear_btn.object_id());

    let mut button_container = Container::new();
    button_container.set_layout(LayoutKind::from(button_row));

    let mut main_layout = VBoxLayout::new();
    main_layout.set_spacing(10.0);
    main_layout.add_widget(input.object_id());
    main_layout.add_widget(button_container.object_id());
    main_layout.add_widget(list_view.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(main_layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

### Model Operations

```rust,ignore
use horizon_lattice::model::ListModel;

let mut model = ListModel::new(vec!["A", "B", "C"]);

// Add items
model.push("D");                    // Append at end
model.insert(1, "Inserted");        // Insert at position

// Remove items
let item = model.remove(0);         // Remove and return item
model.clear();                      // Remove all

// Replace all items
model.set_items(vec!["X", "Y", "Z"]);

// Modify an item in place
model.modify(0, |item| {
    *item = "Modified".to_string();
});

// Sort items
model.sort_by(|a, b| a.cmp(b));

// Query
let count = model.len();
let is_empty = model.is_empty();
```

## Step 6: View Modes

ListView supports different display modes:

```rust,ignore
use horizon_lattice::widget::widgets::{ListView, ListViewMode};

// List mode (vertical list, one item per row)
let list = ListView::new()
    .with_view_mode(ListViewMode::ListMode)
    .with_model(model.clone());

// Icon mode (grid of items)
let grid = ListView::new()
    .with_view_mode(ListViewMode::IconMode)
    .with_model(model);
```

## Complete Example: Task Manager

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    ListView, PushButton, LineEdit, CheckBox, Label,
    Container, Window, ButtonVariant
};
use horizon_lattice::widget::layout::{
    VBoxLayout, HBoxLayout, ContentMargins, LayoutKind
};
use horizon_lattice::model::{
    ListModel, ListItem, ItemData, ItemRole, SelectionMode
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct Task {
    title: String,
    completed: bool,
}

impl ListItem for Task {
    fn display(&self) -> ItemData {
        let prefix = if self.completed { "[x]" } else { "[ ]" };
        ItemData::from(format!("{} {}", prefix, self.title))
    }

    fn data(&self, role: ItemRole) -> ItemData {
        match role {
            ItemRole::Display => self.display(),
            ItemRole::CheckState => {
                if self.completed {
                    ItemData::from(true)
                } else {
                    ItemData::from(false)
                }
            }
            _ => ItemData::None,
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Task Manager")
        .with_size(400.0, 500.0);

    // Model with initial tasks
    let model = Arc::new(Mutex::new(ListModel::new(vec![
        Task { title: "Buy groceries".into(), completed: false },
        Task { title: "Walk the dog".into(), completed: true },
        Task { title: "Read a book".into(), completed: false },
    ])));

    // Views and controls
    let list_view = ListView::new()
        .with_model(model.lock().unwrap().clone())
        .with_selection_mode(SelectionMode::SingleSelection);

    let mut task_input = LineEdit::new();
    task_input.set_placeholder("New task...");

    let add_btn = PushButton::new("Add Task");
    let toggle_btn = PushButton::new("Toggle Done")
        .with_variant(ButtonVariant::Secondary);
    let delete_btn = PushButton::new("Delete")
        .with_variant(ButtonVariant::Danger);

    let title = Label::new("My Tasks");

    // Add task
    let model_clone = model.clone();
    let input_clone = task_input.clone();
    add_btn.clicked().connect(move |_| {
        let text = input_clone.text();
        if !text.is_empty() {
            model_clone.lock().unwrap().push(Task {
                title: text.clone(),
                completed: false,
            });
            input_clone.set_text("");
        }
    });

    // Toggle completion
    let model_clone = model.clone();
    let list_clone = list_view.clone();
    toggle_btn.clicked().connect(move |_| {
        let selection = list_clone.selection_model();
        if let Some(index) = selection.current_index() {
            let row = index.row() as usize;
            model_clone.lock().unwrap().modify(row, |task| {
                task.completed = !task.completed;
            });
        }
    });

    // Delete task
    let model_clone = model.clone();
    let list_clone = list_view.clone();
    delete_btn.clicked().connect(move |_| {
        let selection = list_clone.selection_model();
        if let Some(index) = selection.current_index() {
            model_clone.lock().unwrap().remove(index.row() as usize);
        }
    });

    // Enter key adds task
    let model_clone = model.clone();
    let input_clone = task_input.clone();
    task_input.return_pressed.connect(move || {
        let text = input_clone.text();
        if !text.is_empty() {
            model_clone.lock().unwrap().push(Task {
                title: text.clone(),
                completed: false,
            });
            input_clone.set_text("");
        }
    });

    // Layout
    let mut input_row = HBoxLayout::new();
    input_row.set_spacing(8.0);
    input_row.add_widget(task_input.object_id());
    input_row.add_widget(add_btn.object_id());

    let mut input_container = Container::new();
    input_container.set_layout(LayoutKind::from(input_row));

    let mut action_row = HBoxLayout::new();
    action_row.set_spacing(8.0);
    action_row.add_stretch(1);
    action_row.add_widget(toggle_btn.object_id());
    action_row.add_widget(delete_btn.object_id());

    let mut action_container = Container::new();
    action_container.set_layout(LayoutKind::from(action_row));

    let mut main_layout = VBoxLayout::new();
    main_layout.set_content_margins(ContentMargins::uniform(16.0));
    main_layout.set_spacing(12.0);
    main_layout.add_widget(title.object_id());
    main_layout.add_widget(input_container.object_id());
    main_layout.add_widget(list_view.object_id());
    main_layout.add_widget(action_container.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(main_layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Best Practices

1. **Use ListWidget for simple cases** - When you don't need complex data binding
2. **Use ListView + ListModel for structured data** - Better separation of concerns
3. **Implement ListItem for custom types** - Clean data display logic
4. **Handle selection appropriately** - Use the right SelectionMode for your use case
5. **Remove from highest to lowest index** - Prevents index shifting issues
6. **Use Arc<Mutex<>> for shared model access** - Thread-safe model updates

## Next Steps

- [Custom Widgets](./custom-widget.md) - Create your own widgets
- [Theming](./theming.md) - Style your lists
- [Architecture Guide](../guides/architecture.md) - Understand the Model/View pattern in depth
