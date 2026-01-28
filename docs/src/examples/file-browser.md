# Example: File Browser

A file browser demonstrating TreeView, ListView, and file operations.

## Overview

This example builds a file browser with:
- TreeView for directory hierarchy
- ListView for file listing
- Splitter for resizable panes
- Toolbar with navigation buttons
- Address bar for direct path entry

## Key Concepts

- **TreeView**: Hierarchical directory display
- **ListView**: File listing with icons
- **Splitter**: Resizable split view
- **Models**: Custom data models for files
- **File operations**: Reading directory contents

## Full Source

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Window, Container, TreeView, ListView, Splitter, ToolBar, LineEdit,
    PushButton, Label, Action
};
use horizon_lattice::widget::layout::{VBoxLayout, HBoxLayout, LayoutKind};
use horizon_lattice::model::{TreeModel, ListModel, ModelIndex};
use horizon_lattice::file::{FileInfo, path::{home_dir, parent}};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::fs;

#[derive(Clone)]
struct FileEntry {
    name: String,
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

impl FileEntry {
    fn from_path(path: PathBuf) -> Option<Self> {
        let info = FileInfo::new(&path).ok()?;
        Some(Self {
            name: path.file_name()?.to_str()?.to_string(),
            path,
            is_dir: info.is_dir(),
            size: info.size(),
        })
    }

    fn size_string(&self) -> String {
        if self.is_dir {
            String::new()
        } else if self.size < 1024 {
            format!("{} B", self.size)
        } else if self.size < 1024 * 1024 {
            format!("{:.1} KB", self.size as f64 / 1024.0)
        } else {
            format!("{:.1} MB", self.size as f64 / (1024.0 * 1024.0))
        }
    }
}

fn list_directory(path: &PathBuf) -> Vec<FileEntry> {
    let mut entries = Vec::new();
    if let Ok(read_dir) = fs::read_dir(path) {
        for entry in read_dir.filter_map(|e| e.ok()) {
            if let Some(file_entry) = FileEntry::from_path(entry.path()) {
                entries.push(file_entry);
            }
        }
    }
    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
    entries
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("File Browser")
        .with_size(900.0, 600.0);

    // Current path state
    let current_path = Arc::new(Mutex::new(home_dir()?));

    // Address bar
    let address_bar = LineEdit::new();
    address_bar.set_text(current_path.lock().unwrap().to_str().unwrap());

    // Navigation buttons
    let back_btn = PushButton::new("Back");
    let up_btn = PushButton::new("Up");
    let home_btn = PushButton::new("Home");
    let refresh_btn = PushButton::new("Refresh");

    // Directory tree (left pane)
    let tree_view = TreeView::new();

    // File list (right pane)
    let list_model = Arc::new(Mutex::new(ListModel::new(Vec::<FileEntry>::new())));
    let list_view = ListView::new()
        .with_model(list_model.lock().unwrap().clone());

    // Function to update file list
    let update_list = {
        let model = list_model.clone();
        let address = address_bar.clone();
        move |path: &PathBuf| {
            let entries = list_directory(path);
            model.lock().unwrap().set_items(entries);
            address.set_text(path.to_str().unwrap_or(""));
        }
    };

    // Initial load
    update_list(&current_path.lock().unwrap().clone());

    // Back button (history would be implemented)
    back_btn.clicked().connect(|_| {
        // Would implement navigation history
    });

    // Up button
    let path = current_path.clone();
    let update = update_list.clone();
    up_btn.clicked().connect(move |_| {
        let mut p = path.lock().unwrap();
        if let Some(parent_path) = parent(&*p) {
            *p = parent_path.clone();
            update(&parent_path);
        }
    });

    // Home button
    let path = current_path.clone();
    let update = update_list.clone();
    home_btn.clicked().connect(move |_| {
        if let Ok(home) = home_dir() {
            *path.lock().unwrap() = home.clone();
            update(&home);
        }
    });

    // Refresh button
    let path = current_path.clone();
    let update = update_list.clone();
    refresh_btn.clicked().connect(move |_| {
        let p = path.lock().unwrap().clone();
        update(&p);
    });

    // Address bar enter key
    let path = current_path.clone();
    let update = update_list.clone();
    address_bar.return_pressed.connect(move || {
        let text = address_bar.text();
        let new_path = PathBuf::from(&text);
        if new_path.is_dir() {
            *path.lock().unwrap() = new_path.clone();
            update(&new_path);
        }
    });

    // Double-click on list item
    let path = current_path.clone();
    let model = list_model.clone();
    let update = update_list.clone();
    list_view.double_clicked.connect(move |index: &ModelIndex| {
        let m = model.lock().unwrap();
        if let Some(entry) = m.get(index.row() as usize) {
            if entry.is_dir {
                let new_path = entry.path.clone();
                drop(m);
                *path.lock().unwrap() = new_path.clone();
                update(&new_path);
            }
        }
    });

    // Toolbar layout
    let mut toolbar = HBoxLayout::new();
    toolbar.set_spacing(4.0);
    toolbar.add_widget(back_btn.object_id());
    toolbar.add_widget(up_btn.object_id());
    toolbar.add_widget(home_btn.object_id());
    toolbar.add_widget(refresh_btn.object_id());
    toolbar.add_widget(address_bar.object_id());

    let mut toolbar_container = Container::new();
    toolbar_container.set_layout(LayoutKind::from(toolbar));

    // Splitter with tree and list
    let mut splitter = Splitter::new();
    splitter.add_widget(tree_view.object_id());
    splitter.add_widget(list_view.object_id());
    splitter.set_sizes(&[200, 600]);

    // Main layout
    let mut layout = VBoxLayout::new();
    layout.add_widget(toolbar_container.object_id());
    layout.add_widget(splitter.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Features Demonstrated

| Feature | Description |
|---------|-------------|
| **TreeView** | Hierarchical directory tree |
| **ListView** | File list with model |
| **Splitter** | Resizable split panes |
| **ListModel** | Dynamic file list model |
| **File operations** | Reading directories |
| **Navigation** | Up, Home, address bar |

## Exercises

1. **Add file icons**: Show different icons for file types
2. **Add context menu**: Right-click options (Open, Delete, Rename)
3. **Add file details**: Show columns for size, date, type
4. **Add search**: Filter files by name
5. **Add bookmarks**: Quick access sidebar

## Related Examples

- [Text Editor](./text-editor.md) - File opening
- [Image Viewer](./image-viewer.md) - Image browsing
