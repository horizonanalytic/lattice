# Example: Image Viewer

An image viewer demonstrating image loading, zoom/pan, and thumbnail lists.

## Overview

This example builds an image viewer with:
- ImageWidget for displaying images
- Zoom and pan controls
- Thumbnail strip for navigation
- File open dialog for images
- Fit-to-window and actual-size modes

## Key Concepts

- **ImageWidget**: Image display with scaling modes
- **ScrollArea**: Panning larger images
- **ListView**: Thumbnail strip
- **File dialogs**: Opening image files
- **Keyboard shortcuts**: Navigation controls

## Full Source

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Window, Container, ImageWidget, ScrollArea, ListView, Splitter,
    PushButton, Label, Slider, FileDialog, FileFilter,
    ImageScaleMode
};
use horizon_lattice::widget::layout::{VBoxLayout, HBoxLayout, LayoutKind, ContentMargins};
use horizon_lattice::model::ListModel;
use horizon_lattice::widget::{Widget, WidgetEvent, Key};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct ImageEntry {
    path: PathBuf,
    name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new("Image Viewer")
        .with_size(1000.0, 700.0);

    // Current state
    let current_index = Arc::new(Mutex::new(0usize));
    let images: Arc<Mutex<Vec<ImageEntry>>> = Arc::new(Mutex::new(Vec::new()));

    // Main image display
    let image_widget = ImageWidget::new();
    image_widget.set_scale_mode(ImageScaleMode::Fit);

    // Wrap in scroll area for panning
    let mut scroll_area = ScrollArea::new();
    scroll_area.set_widget(image_widget.object_id());

    // Zoom controls
    let zoom_slider = Slider::new();
    zoom_slider.set_range(10, 400);  // 10% to 400%
    zoom_slider.set_value(100);

    let zoom_label = Label::new("100%");

    let fit_btn = PushButton::new("Fit");
    let actual_btn = PushButton::new("1:1");

    // Zoom slider updates scale
    let image = image_widget.clone();
    let label = zoom_label.clone();
    zoom_slider.value_changed.connect(move |&value| {
        let scale = value as f32 / 100.0;
        image.set_scale(scale);
        label.set_text(&format!("{}%", value));
    });

    // Fit button
    let image = image_widget.clone();
    let slider = zoom_slider.clone();
    let label = zoom_label.clone();
    fit_btn.clicked().connect(move |_| {
        image.set_scale_mode(ImageScaleMode::Fit);
        slider.set_value(100);
        label.set_text("Fit");
    });

    // Actual size button
    let image = image_widget.clone();
    let slider = zoom_slider.clone();
    let label = zoom_label.clone();
    actual_btn.clicked().connect(move |_| {
        image.set_scale_mode(ImageScaleMode::None);
        image.set_scale(1.0);
        slider.set_value(100);
        label.set_text("100%");
    });

    // Navigation buttons
    let prev_btn = PushButton::new("Previous");
    let next_btn = PushButton::new("Next");
    let open_btn = PushButton::new("Open...");

    // Thumbnail list
    let thumbnail_model = Arc::new(Mutex::new(ListModel::new(Vec::<String>::new())));
    let thumbnail_list = ListView::new()
        .with_model(thumbnail_model.lock().unwrap().clone());

    // Open button - load images
    let image = image_widget.clone();
    let imgs = images.clone();
    let idx = current_index.clone();
    let thumbs = thumbnail_model.clone();
    open_btn.clicked().connect(move |_| {
        let filters = vec![
            FileFilter::image_files(),
            FileFilter::all_files(),
        ];

        if let Some(paths) = FileDialog::get_open_file_names("Open Images", "", &filters) {
            let entries: Vec<ImageEntry> = paths.into_iter().map(|p| {
                let name = p.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                ImageEntry { path: p, name }
            }).collect();

            if !entries.is_empty() {
                // Update thumbnail list
                let names: Vec<String> = entries.iter().map(|e| e.name.clone()).collect();
                thumbs.lock().unwrap().set_items(names);

                // Load first image
                image.set_source_file(&entries[0].path);

                *imgs.lock().unwrap() = entries;
                *idx.lock().unwrap() = 0;
            }
        }
    });

    // Previous button
    let image = image_widget.clone();
    let imgs = images.clone();
    let idx = current_index.clone();
    prev_btn.clicked().connect(move |_| {
        let entries = imgs.lock().unwrap();
        let mut i = idx.lock().unwrap();
        if !entries.is_empty() && *i > 0 {
            *i -= 1;
            image.set_source_file(&entries[*i].path);
        }
    });

    // Next button
    let image = image_widget.clone();
    let imgs = images.clone();
    let idx = current_index.clone();
    next_btn.clicked().connect(move |_| {
        let entries = imgs.lock().unwrap();
        let mut i = idx.lock().unwrap();
        if !entries.is_empty() && *i < entries.len() - 1 {
            *i += 1;
            image.set_source_file(&entries[*i].path);
        }
    });

    // Thumbnail click
    let image = image_widget.clone();
    let imgs = images.clone();
    let idx = current_index.clone();
    thumbnail_list.clicked.connect(move |index| {
        let entries = imgs.lock().unwrap();
        let row = index.row() as usize;
        if row < entries.len() {
            image.set_source_file(&entries[row].path);
            *idx.lock().unwrap() = row;
        }
    });

    // Toolbar
    let mut toolbar = HBoxLayout::new();
    toolbar.set_spacing(8.0);
    toolbar.add_widget(open_btn.object_id());
    toolbar.add_widget(prev_btn.object_id());
    toolbar.add_widget(next_btn.object_id());
    toolbar.add_stretch(1);
    toolbar.add_widget(fit_btn.object_id());
    toolbar.add_widget(actual_btn.object_id());
    toolbar.add_widget(zoom_slider.object_id());
    toolbar.add_widget(zoom_label.object_id());

    let mut toolbar_container = Container::new();
    toolbar_container.set_layout(LayoutKind::from(toolbar));

    // Main content with splitter
    let mut splitter = Splitter::new();
    splitter.add_widget(scroll_area.object_id());
    splitter.add_widget(thumbnail_list.object_id());
    splitter.set_sizes(&[700, 200]);

    // Main layout
    let mut layout = VBoxLayout::new();
    layout.set_content_margins(ContentMargins::uniform(8.0));
    layout.set_spacing(8.0);
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
| **ImageWidget** | Image display with scaling |
| **ScrollArea** | Pan large images |
| **Slider** | Zoom control |
| **ListView** | Thumbnail strip |
| **Splitter** | Resizable panels |
| **FileDialog** | Multi-file selection |

## Exercises

1. **Add keyboard navigation**: Arrow keys for prev/next
2. **Add mouse wheel zoom**: Zoom centered on cursor
3. **Add drag-to-pan**: Click and drag to pan
4. **Add slideshow mode**: Auto-advance with timer
5. **Add image info**: Show dimensions, file size, date

## Related Examples

- [File Browser](./file-browser.md) - File navigation
- [Text Editor](./text-editor.md) - File operations
