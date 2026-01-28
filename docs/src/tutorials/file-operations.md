# Tutorial: File Operations

Learn to work with files, dialogs, and application settings.

## What You'll Learn

- Using native file dialogs
- Reading and writing files
- Managing application settings
- Working with application directories

## Prerequisites

- Completed the [Theming](./theming.md) tutorial
- Understanding of Rust's `std::fs` module

## Step 1: Native File Dialogs

Horizon Lattice provides native file dialogs that integrate with the operating system:

```rust,ignore
use horizon_lattice::widget::widgets::native_dialogs::{
    NativeFileDialogOptions, NativeFileFilter,
    open_file, open_files, save_file, select_directory
};

fn main() {
    // Open a single file
    let options = NativeFileDialogOptions::with_title("Open Document")
        .filter(NativeFileFilter::new("Text Files", &["txt", "md"]))
        .filter(NativeFileFilter::new("All Files", &["*"]));

    if let Some(path) = open_file(options) {
        println!("Selected: {:?}", path);
    }
}
```

### Open Multiple Files

```rust,ignore
let options = NativeFileDialogOptions::with_title("Select Images")
    .filter(NativeFileFilter::new("Images", &["png", "jpg", "jpeg", "gif"]))
    .multiple(true);

if let Some(paths) = open_files(options) {
    for path in paths {
        println!("Selected: {:?}", path);
    }
}
```

### Save File Dialog

```rust,ignore
let options = NativeFileDialogOptions::with_title("Save Document")
    .default_name("untitled.txt")
    .filter(NativeFileFilter::new("Text Files", &["txt"]));

if let Some(path) = save_file(options) {
    println!("Save to: {:?}", path);
}
```

### Select Directory

```rust,ignore
let options = NativeFileDialogOptions::with_title("Choose Folder")
    .directory("/home/user/Documents");

if let Some(path) = select_directory(options) {
    println!("Selected directory: {:?}", path);
}
```

## Step 2: The FileDialog Widget

For more control, use the `FileDialog` widget:

```rust,ignore
use horizon_lattice::widget::widgets::{FileDialog, FileDialogMode, FileFilter};
use std::path::PathBuf;

// Create dialog for opening files
let dialog = FileDialog::for_open()
    .with_title("Open Project")
    .with_directory("/home/user/projects")
    .with_filter(FileFilter::new("Rust Files", &["*.rs", "*.toml"]));

// Connect to selection signal
dialog.file_selected.connect(|path: &PathBuf| {
    println!("Selected: {:?}", path);
});

// Show the dialog
dialog.open();
```

### Static Helper Methods

```rust,ignore
use horizon_lattice::widget::widgets::{FileDialog, FileFilter};

// Quick open file dialog
let filters = vec![
    FileFilter::text_files(),
    FileFilter::all_files(),
];

if let Some(path) = FileDialog::get_open_file_name("Open", "/home", &filters) {
    println!("Opening: {:?}", path);
}

// Quick save file dialog
if let Some(path) = FileDialog::get_save_file_name("Save As", "/home", &filters) {
    println!("Saving to: {:?}", path);
}

// Quick directory selection
if let Some(path) = FileDialog::get_existing_directory("Select Folder", "/home") {
    println!("Directory: {:?}", path);
}
```

## Step 3: Reading and Writing Files

Horizon Lattice provides convenient file operations:

### Quick Operations

```rust,ignore
use horizon_lattice::file::operations::{
    read_text, read_bytes, read_lines,
    write_text, write_bytes, append_text
};

// Read entire file as string
let content = read_text("config.txt")?;

// Read file as bytes
let data = read_bytes("image.png")?;

// Read file line by line
let lines = read_lines("data.csv")?;
for line in lines {
    println!("{}", line);
}

// Write string to file
write_text("output.txt", "Hello, World!")?;

// Write bytes to file
write_bytes("data.bin", &[0x00, 0x01, 0x02])?;

// Append to file
append_text("log.txt", "New log entry\n")?;
```

### File Reader

```rust,ignore
use horizon_lattice::file::File;

let mut file = File::open("document.txt")?;

// Read entire content
let content = file.read_to_string()?;

// Or iterate over lines
let file = File::open("document.txt")?;
for line in file.lines() {
    let line = line?;
    println!("{}", line);
}
```

### File Writer

```rust,ignore
use horizon_lattice::file::FileWriter;

// Create new file (overwrites existing)
let mut writer = FileWriter::create("output.txt")?;
writer.write_str("Line 1\n")?;
writer.write_line("Line 2")?;
writer.flush()?;

// Append to existing file
let mut writer = FileWriter::append("log.txt")?;
writer.write_line("New entry")?;
```

### Atomic Writes (Safe for Config Files)

```rust,ignore
use horizon_lattice::file::operations::atomic_write;
use horizon_lattice::file::AtomicWriter;

// Atomic write ensures file is complete or unchanged
atomic_write("config.json", |writer: &mut AtomicWriter| {
    writer.write_str("{\n")?;
    writer.write_str("  \"version\": 1\n")?;
    writer.write_str("}\n")?;
    Ok(())
})?;
// File is atomically renamed only if write succeeds
```

## Step 4: Application Settings

The `Settings` API provides a hierarchical key-value store:

```rust,ignore
use horizon_lattice::file::Settings;

// Create settings
let settings = Settings::new();

// Store values (hierarchical keys with . or / separator)
settings.set("app.window.width", 1024);
settings.set("app.window.height", 768);
settings.set("app/theme/name", "dark");
settings.set("app.recent_files", vec!["file1.txt", "file2.txt"]);

// Retrieve values with type safety
let width: i32 = settings.get("app.window.width").unwrap_or(800);
let theme: String = settings.get_or("app.theme.name", "light".to_string());

// Check if key exists
if settings.contains("app.window.width") {
    println!("Width is configured");
}

// List keys in a group
let window_keys = settings.group_keys("app.window");
// Returns: ["width", "height"]
```

### Persisting Settings

```rust,ignore
use horizon_lattice::file::{Settings, SettingsFormat};

// Save to JSON
settings.save_json("config.json")?;

// Save to TOML
settings.save_toml("config.toml")?;

// Save to INI (flat structure)
settings.save_ini("config.ini")?;

// Load from file
let settings = Settings::load_json("config.json")?;
let settings = Settings::load_toml("config.toml")?;
```

### Auto-Save Settings

```rust,ignore
use horizon_lattice::file::{Settings, SettingsFormat};

let settings = Settings::new();

// Enable auto-save (writes on every change)
settings.set_auto_save("config.json", SettingsFormat::Json);

// Changes are automatically persisted
settings.set("app.volume", 75);  // Saved automatically

// Force immediate write
settings.sync()?;

// Disable auto-save
settings.disable_auto_save();
```

### Listening to Changes

```rust,ignore
let settings = Settings::new();

// Connect to change signal
settings.changed().connect(|key: &String| {
    println!("Setting changed: {}", key);
});

settings.set("app.theme", "dark");
// Prints: "Setting changed: app.theme"
```

## Step 5: Application Directories

Get standard directories for your application:

```rust,ignore
use horizon_lattice::file::path::{
    home_dir, config_dir, data_dir, cache_dir,
    documents_dir, downloads_dir, AppPaths
};

// Standard user directories
let home = home_dir()?;           // /home/user
let config = config_dir()?;       // /home/user/.config
let data = data_dir()?;           // /home/user/.local/share
let cache = cache_dir()?;         // /home/user/.cache
let docs = documents_dir()?;      // /home/user/Documents
let downloads = downloads_dir()?; // /home/user/Downloads

// Application-specific directories
let app_paths = AppPaths::new("com", "example", "myapp")?;
let app_config = app_paths.config();     // ~/.config/myapp
let app_data = app_paths.data();         // ~/.local/share/myapp
let app_cache = app_paths.cache();       // ~/.cache/myapp
let app_logs = app_paths.logs();         // ~/.local/share/myapp/logs
```

## Step 6: File Information

Query file metadata:

```rust,ignore
use horizon_lattice::file::{FileInfo, exists, is_file, is_dir, file_size};

// Quick checks
if exists("config.json") {
    println!("Config exists");
}

if is_file("document.txt") {
    let size = file_size("document.txt")?;
    println!("Size: {} bytes", size);
}

if is_dir("projects") {
    println!("Projects directory exists");
}

// Detailed file information
let info = FileInfo::new("document.txt")?;
println!("Size: {} bytes", info.size());
println!("Is readable: {}", info.is_readable());
println!("Is writable: {}", info.is_writable());

if let Some(modified) = info.modified() {
    println!("Modified: {:?}", modified);
}
```

## Complete Example: Note Taking App

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Window, Container, TextEdit, PushButton, Label,
    FileDialog, FileFilter, ButtonVariant
};
use horizon_lattice::widget::layout::{VBoxLayout, HBoxLayout, ContentMargins, LayoutKind};
use horizon_lattice::file::{Settings, operations::{read_text, atomic_write}, path::AppPaths};
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    // Setup app directories and settings
    let app_paths = AppPaths::new("com", "example", "notes")?;
    let settings_path = app_paths.config().join("settings.json");

    let settings = if settings_path.exists() {
        Settings::load_json(&settings_path)?
    } else {
        Settings::new()
    };
    settings.set_auto_save(&settings_path, horizon_lattice::file::SettingsFormat::Json);

    // Track current file
    let current_file: Arc<Mutex<Option<PathBuf>>> = Arc::new(Mutex::new(None));

    // Window setup
    let mut window = Window::new("Notes")
        .with_size(
            settings.get_or("window.width", 600),
            settings.get_or("window.height", 400),
        );

    // Title showing current file
    let title_label = Label::new("Untitled");

    // Text editor
    let text_edit = TextEdit::new();
    text_edit.set_placeholder("Start typing...");

    // Buttons
    let new_btn = PushButton::new("New");
    let open_btn = PushButton::new("Open");
    let save_btn = PushButton::new("Save")
        .with_variant(ButtonVariant::Primary);
    let save_as_btn = PushButton::new("Save As");

    // New button - clear editor
    let editor = text_edit.clone();
    let title = title_label.clone();
    let file = current_file.clone();
    new_btn.clicked().connect(move |_| {
        editor.set_text("");
        title.set_text("Untitled");
        *file.lock().unwrap() = None;
    });

    // Open button
    let editor = text_edit.clone();
    let title = title_label.clone();
    let file = current_file.clone();
    open_btn.clicked().connect(move |_| {
        let filters = vec![
            FileFilter::text_files(),
            FileFilter::all_files(),
        ];

        if let Some(path) = FileDialog::get_open_file_name("Open Note", "", &filters) {
            match read_text(&path) {
                Ok(content) => {
                    editor.set_text(&content);
                    title.set_text(path.file_name().unwrap().to_str().unwrap());
                    *file.lock().unwrap() = Some(path);
                }
                Err(e) => {
                    eprintln!("Failed to open file: {}", e);
                }
            }
        }
    });

    // Save button
    let editor = text_edit.clone();
    let file = current_file.clone();
    save_btn.clicked().connect(move |_| {
        let file_lock = file.lock().unwrap();
        if let Some(ref path) = *file_lock {
            let content = editor.text();
            if let Err(e) = atomic_write(path, |w| {
                w.write_str(&content)
            }) {
                eprintln!("Failed to save: {}", e);
            }
        } else {
            // No file set, trigger Save As
            drop(file_lock);
            // Would trigger save_as here
        }
    });

    // Save As button
    let editor = text_edit.clone();
    let title = title_label.clone();
    let file = current_file.clone();
    save_as_btn.clicked().connect(move |_| {
        let filters = vec![
            FileFilter::text_files(),
            FileFilter::all_files(),
        ];

        if let Some(path) = FileDialog::get_save_file_name("Save Note", "", &filters) {
            let content = editor.text();
            match atomic_write(&path, |w| w.write_str(&content)) {
                Ok(()) => {
                    title.set_text(path.file_name().unwrap().to_str().unwrap());
                    *file.lock().unwrap() = Some(path);
                }
                Err(e) => {
                    eprintln!("Failed to save: {}", e);
                }
            }
        }
    });

    // Button row
    let mut button_row = HBoxLayout::new();
    button_row.set_spacing(8.0);
    button_row.add_widget(new_btn.object_id());
    button_row.add_widget(open_btn.object_id());
    button_row.add_widget(save_btn.object_id());
    button_row.add_widget(save_as_btn.object_id());
    button_row.add_stretch(1);

    let mut button_container = Container::new();
    button_container.set_layout(LayoutKind::from(button_row));

    // Main layout
    let mut layout = VBoxLayout::new();
    layout.set_content_margins(ContentMargins::uniform(12.0));
    layout.set_spacing(8.0);
    layout.add_widget(title_label.object_id());
    layout.add_widget(button_container.object_id());
    layout.add_widget(text_edit.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Error Handling

File operations use `FileResult<T>` which is `Result<T, FileError>`:

```rust,ignore
use horizon_lattice::file::{FileResult, FileError, operations::read_text};

fn load_config() -> FileResult<String> {
    read_text("config.json")
}

fn main() {
    match load_config() {
        Ok(content) => println!("Loaded: {}", content),
        Err(e) if e.is_not_found() => {
            println!("Config not found, using defaults");
        }
        Err(e) if e.is_permission_denied() => {
            eprintln!("Permission denied: {}", e);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}
```

## Best Practices

### 1. Use Atomic Writes for Config Files

```rust,ignore
// Bad - can leave corrupted file on crash
write_text("config.json", &content)?;

// Good - atomic operation
atomic_write("config.json", |w| w.write_str(&content))?;
```

### 2. Use AppPaths for Application Files

```rust,ignore
// Bad - hardcoded paths
let config_path = "/home/user/.myapp/config.json";

// Good - platform-appropriate paths
let app = AppPaths::new("com", "company", "myapp")?;
let config_path = app.config().join("config.json");
```

### 3. Validate File Paths Before Use

```rust,ignore
use horizon_lattice::file::{exists, is_file, is_readable};

if exists(&path) && is_file(&path) && is_readable(&path) {
    let content = read_text(&path)?;
}
```

### 4. Save Window State in Settings

```rust,ignore
// On window resize
settings.set("window.width", window.width());
settings.set("window.height", window.height());

// On startup
let width = settings.get_or("window.width", 800);
let height = settings.get_or("window.height", 600);
```

## Next Steps

- [Examples: Text Editor](../examples/text-editor.md) - Full-featured editor example
- [Examples: File Browser](../examples/file-browser.md) - Directory navigation example
- [Architecture Guide](../guides/architecture.md) - Understanding the file system integration
