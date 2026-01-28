# Example: Text Editor

A functional text editor demonstrating file operations, menus, and text editing.

## Overview

This example builds a text editor with:
- Multi-line text editing with TextEdit widget
- File menu with New, Open, Save, Save As
- Edit menu with Undo, Redo, Cut, Copy, Paste
- Status bar showing cursor position
- Dirty file tracking with save prompts

## Key Concepts

- **MainWindow**: Application window with menu bar and status bar
- **MenuBar and Menu**: Standard application menus with keyboard shortcuts
- **TextEdit**: Multi-line text editing widget
- **File dialogs**: Open and save file dialogs
- **Action**: Reusable menu/toolbar commands

## Full Source

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    MainWindow, TextEdit, StatusBar, Menu, MenuBar, Action,
    FileDialog, FileFilter, MessageBox
};
use horizon_lattice::file::operations::{read_text, atomic_write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

struct EditorState {
    current_file: Option<PathBuf>,
    is_modified: bool,
}

impl EditorState {
    fn new() -> Self {
        Self { current_file: None, is_modified: false }
    }

    fn window_title(&self) -> String {
        let name = self.current_file
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("Untitled");
        if self.is_modified {
            format!("*{} - Text Editor", name)
        } else {
            format!("{} - Text Editor", name)
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;
    let state = Arc::new(Mutex::new(EditorState::new()));

    let mut window = MainWindow::new("Text Editor")
        .with_size(800.0, 600.0);

    // Text editor widget
    let text_edit = TextEdit::new();

    // Status bar
    let status_bar = StatusBar::new();
    status_bar.show_message("Ready");

    // Track modifications
    let state_clone = state.clone();
    let window_clone = window.clone();
    text_edit.text_changed.connect(move || {
        let mut s = state_clone.lock().unwrap();
        s.is_modified = true;
        window_clone.set_title(&s.window_title());
    });

    // === File Menu ===
    let mut file_menu = Menu::new("File");

    let new_action = Action::new("New").with_shortcut("Ctrl+N");
    let open_action = Action::new("Open...").with_shortcut("Ctrl+O");
    let save_action = Action::new("Save").with_shortcut("Ctrl+S");
    let save_as_action = Action::new("Save As...");
    let quit_action = Action::new("Quit").with_shortcut("Ctrl+Q");

    // New file handler
    let editor = text_edit.clone();
    let state_clone = state.clone();
    let window_clone = window.clone();
    new_action.triggered.connect(move || {
        editor.set_text("");
        let mut s = state_clone.lock().unwrap();
        s.current_file = None;
        s.is_modified = false;
        window_clone.set_title(&s.window_title());
    });

    // Open file handler
    let editor = text_edit.clone();
    let state_clone = state.clone();
    let window_clone = window.clone();
    open_action.triggered.connect(move || {
        let filters = vec![FileFilter::text_files(), FileFilter::all_files()];
        if let Some(path) = FileDialog::get_open_file_name("Open", "", &filters) {
            if let Ok(content) = read_text(&path) {
                editor.set_text(&content);
                let mut s = state_clone.lock().unwrap();
                s.current_file = Some(path);
                s.is_modified = false;
                window_clone.set_title(&s.window_title());
            }
        }
    });

    // Save file handler
    let editor = text_edit.clone();
    let state_clone = state.clone();
    let window_clone = window.clone();
    save_action.triggered.connect(move || {
        let s = state_clone.lock().unwrap();
        if let Some(ref path) = s.current_file {
            let content = editor.text();
            drop(s);
            if atomic_write(path, |w| w.write_str(&content)).is_ok() {
                let mut s = state_clone.lock().unwrap();
                s.is_modified = false;
                window_clone.set_title(&s.window_title());
            }
        }
    });

    // Save As handler
    let editor = text_edit.clone();
    let state_clone = state.clone();
    let window_clone = window.clone();
    save_as_action.triggered.connect(move || {
        let filters = vec![FileFilter::text_files(), FileFilter::all_files()];
        if let Some(path) = FileDialog::get_save_file_name("Save As", "", &filters) {
            let content = editor.text();
            if atomic_write(&path, |w| w.write_str(&content)).is_ok() {
                let mut s = state_clone.lock().unwrap();
                s.current_file = Some(path);
                s.is_modified = false;
                window_clone.set_title(&s.window_title());
            }
        }
    });

    // Quit handler
    let app_clone = app.clone();
    quit_action.triggered.connect(move || {
        app_clone.quit();
    });

    file_menu.add_action(new_action);
    file_menu.add_action(open_action);
    file_menu.add_separator();
    file_menu.add_action(save_action);
    file_menu.add_action(save_as_action);
    file_menu.add_separator();
    file_menu.add_action(quit_action);

    // === Edit Menu ===
    let mut edit_menu = Menu::new("Edit");

    let undo_action = Action::new("Undo").with_shortcut("Ctrl+Z");
    let redo_action = Action::new("Redo").with_shortcut("Ctrl+Y");
    let cut_action = Action::new("Cut").with_shortcut("Ctrl+X");
    let copy_action = Action::new("Copy").with_shortcut("Ctrl+C");
    let paste_action = Action::new("Paste").with_shortcut("Ctrl+V");
    let select_all_action = Action::new("Select All").with_shortcut("Ctrl+A");

    let editor = text_edit.clone();
    undo_action.triggered.connect(move || editor.undo());

    let editor = text_edit.clone();
    redo_action.triggered.connect(move || editor.redo());

    let editor = text_edit.clone();
    cut_action.triggered.connect(move || editor.cut());

    let editor = text_edit.clone();
    copy_action.triggered.connect(move || editor.copy());

    let editor = text_edit.clone();
    paste_action.triggered.connect(move || editor.paste());

    let editor = text_edit.clone();
    select_all_action.triggered.connect(move || editor.select_all());

    edit_menu.add_action(undo_action);
    edit_menu.add_action(redo_action);
    edit_menu.add_separator();
    edit_menu.add_action(cut_action);
    edit_menu.add_action(copy_action);
    edit_menu.add_action(paste_action);
    edit_menu.add_separator();
    edit_menu.add_action(select_all_action);

    // === View Menu ===
    let mut view_menu = Menu::new("View");
    let word_wrap_action = Action::new("Word Wrap").with_checkable(true);
    word_wrap_action.set_checked(true);

    let editor = text_edit.clone();
    word_wrap_action.toggled.connect(move |&checked| {
        editor.set_word_wrap(checked);
    });

    view_menu.add_action(word_wrap_action);

    // Build menu bar
    let mut menu_bar = MenuBar::new();
    menu_bar.add_menu(file_menu);
    menu_bar.add_menu(edit_menu);
    menu_bar.add_menu(view_menu);

    // Assemble window
    window.set_menu_bar(menu_bar);
    window.set_central_widget(text_edit.object_id());
    window.set_status_bar(status_bar);
    window.show();

    app.run()
}
```

## Features Demonstrated

| Feature | Description |
|---------|-------------|
| **MainWindow** | Window with menu bar, central widget, status bar |
| **MenuBar/Menu** | Hierarchical menu structure with separators |
| **Action** | Commands with keyboard shortcuts |
| **TextEdit** | Multi-line text editing with undo/redo |
| **FileDialog** | Native file dialogs |
| **State Tracking** | Modified flag and dynamic window title |

## Exercises

1. **Add Find/Replace**: Implement search with Ctrl+F
2. **Add recent files**: Show recently opened files in menu
3. **Add line numbers**: Display line numbers in margin
4. **Add syntax highlighting**: Use PlainTextEdit with highlighter
5. **Add multiple tabs**: Support multiple documents

## Related Examples

- [File Browser](./file-browser.md) - File navigation
- [Settings Dialog](./settings-dialog.md) - Preferences
