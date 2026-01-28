# Example: Settings Dialog

A settings dialog demonstrating TabWidget, form inputs, and preferences management.

## Overview

This example builds a settings dialog with:
- TabWidget for organized settings categories
- Various input widgets (CheckBox, ComboBox, SpinBox, etc.)
- Apply/OK/Cancel buttons with standard behavior
- Settings persistence with auto-save

## Key Concepts

- **TabWidget**: Organize settings into categories
- **FormLayout**: Label-field arrangement
- **Dialog**: Modal dialog with accept/reject
- **Settings**: Persistent preferences storage
- **Validation**: Input validation before saving

## Full Source

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Dialog, TabWidget, Container, Label, CheckBox, ComboBox, SpinBox,
    LineEdit, PushButton, GroupBox, ColorButton, FontComboBox,
    ButtonVariant
};
use horizon_lattice::widget::layout::{
    VBoxLayout, HBoxLayout, FormLayout, ContentMargins, LayoutKind
};
use horizon_lattice::file::{Settings, SettingsFormat, path::AppPaths};
use horizon_lattice::render::Color;
use std::sync::{Arc, Mutex};

fn create_general_tab(settings: &Settings) -> Container {
    let mut form = FormLayout::new();

    // Language selection
    let language = ComboBox::new();
    language.add_items(&["English", "Spanish", "French", "German", "Japanese"]);
    language.set_current_index(settings.get_or("general.language", 0));
    form.add_row(Label::new("Language:"), language);

    // Startup behavior
    let restore_session = CheckBox::new("Restore previous session on startup");
    restore_session.set_checked(settings.get_or("general.restore_session", true));
    form.add_spanning_widget(restore_session);

    let check_updates = CheckBox::new("Check for updates automatically");
    check_updates.set_checked(settings.get_or("general.check_updates", true));
    form.add_spanning_widget(check_updates);

    // Recent files limit
    let recent_limit = SpinBox::new();
    recent_limit.set_range(0, 50);
    recent_limit.set_value(settings.get_or("general.recent_limit", 10));
    form.add_row(Label::new("Recent files limit:"), recent_limit);

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(form));
    container
}

fn create_appearance_tab(settings: &Settings) -> Container {
    let mut layout = VBoxLayout::new();
    layout.set_spacing(16.0);

    // Theme group
    let mut theme_form = FormLayout::new();

    let theme = ComboBox::new();
    theme.add_items(&["System", "Light", "Dark", "High Contrast"]);
    theme.set_current_index(settings.get_or("appearance.theme", 0));
    theme_form.add_row(Label::new("Theme:"), theme);

    let accent_color = ColorButton::new();
    accent_color.set_color(Color::from_hex(
        &settings.get_or("appearance.accent", "#0078D4".to_string())
    ).unwrap_or(Color::from_rgb8(0, 120, 212)));
    theme_form.add_row(Label::new("Accent color:"), accent_color);

    let mut theme_group = GroupBox::new("Theme");
    theme_group.set_layout(LayoutKind::from(theme_form));
    layout.add_widget(theme_group.object_id());

    // Font group
    let mut font_form = FormLayout::new();

    let font_family = FontComboBox::new();
    // font_family.set_current_font(settings.get_or("appearance.font", "System".to_string()));
    font_form.add_row(Label::new("Font:"), font_family);

    let font_size = SpinBox::new();
    font_size.set_range(8, 72);
    font_size.set_value(settings.get_or("appearance.font_size", 12));
    font_form.add_row(Label::new("Size:"), font_size);

    let mut font_group = GroupBox::new("Font");
    font_group.set_layout(LayoutKind::from(font_form));
    layout.add_widget(font_group.object_id());

    layout.add_stretch(1);

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));
    container
}

fn create_editor_tab(settings: &Settings) -> Container {
    let mut layout = VBoxLayout::new();
    layout.set_spacing(12.0);

    // Editor options
    let word_wrap = CheckBox::new("Enable word wrap");
    word_wrap.set_checked(settings.get_or("editor.word_wrap", true));
    layout.add_widget(word_wrap.object_id());

    let line_numbers = CheckBox::new("Show line numbers");
    line_numbers.set_checked(settings.get_or("editor.line_numbers", true));
    layout.add_widget(line_numbers.object_id());

    let highlight_line = CheckBox::new("Highlight current line");
    highlight_line.set_checked(settings.get_or("editor.highlight_line", true));
    layout.add_widget(highlight_line.object_id());

    let auto_indent = CheckBox::new("Auto-indent");
    auto_indent.set_checked(settings.get_or("editor.auto_indent", true));
    layout.add_widget(auto_indent.object_id());

    // Tab settings
    let mut tab_form = FormLayout::new();

    let tab_size = SpinBox::new();
    tab_size.set_range(1, 8);
    tab_size.set_value(settings.get_or("editor.tab_size", 4));
    tab_form.add_row(Label::new("Tab size:"), tab_size);

    let use_spaces = CheckBox::new("Insert spaces instead of tabs");
    use_spaces.set_checked(settings.get_or("editor.use_spaces", true));
    tab_form.add_spanning_widget(use_spaces);

    let mut tab_group = GroupBox::new("Indentation");
    tab_group.set_layout(LayoutKind::from(tab_form));
    layout.add_widget(tab_group.object_id());

    layout.add_stretch(1);

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));
    container
}

fn create_advanced_tab(settings: &Settings) -> Container {
    let mut layout = VBoxLayout::new();
    layout.set_spacing(12.0);

    // Performance
    let mut perf_form = FormLayout::new();

    let max_recent = SpinBox::new();
    max_recent.set_range(100, 10000);
    max_recent.set_value(settings.get_or("advanced.max_undo", 1000));
    perf_form.add_row(Label::new("Max undo history:"), max_recent);

    let auto_save = CheckBox::new("Auto-save files");
    auto_save.set_checked(settings.get_or("advanced.auto_save", false));
    perf_form.add_spanning_widget(auto_save);

    let auto_save_interval = SpinBox::new();
    auto_save_interval.set_range(1, 60);
    auto_save_interval.set_value(settings.get_or("advanced.auto_save_interval", 5));
    auto_save_interval.set_suffix(" min");
    perf_form.add_row(Label::new("Auto-save interval:"), auto_save_interval);

    let mut perf_group = GroupBox::new("Performance");
    perf_group.set_layout(LayoutKind::from(perf_form));
    layout.add_widget(perf_group.object_id());

    // Reset button
    let reset_btn = PushButton::new("Reset to Defaults")
        .with_variant(ButtonVariant::Danger);
    layout.add_widget(reset_btn.object_id());

    layout.add_stretch(1);

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));
    container
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    // Load settings
    let app_paths = AppPaths::new("com", "example", "settings-demo")?;
    let settings_path = app_paths.config().join("settings.json");

    let settings = if settings_path.exists() {
        Settings::load_json(&settings_path).unwrap_or_else(|_| Settings::new())
    } else {
        Settings::new()
    };

    let settings = Arc::new(settings);

    // Create dialog
    let mut dialog = Dialog::new("Settings")
        .with_size(500.0, 450.0);

    // Tab widget
    let mut tabs = TabWidget::new();

    tabs.add_tab("General", create_general_tab(&settings));
    tabs.add_tab("Appearance", create_appearance_tab(&settings));
    tabs.add_tab("Editor", create_editor_tab(&settings));
    tabs.add_tab("Advanced", create_advanced_tab(&settings));

    // Button row
    let ok_btn = PushButton::new("OK")
        .with_variant(ButtonVariant::Primary)
        .with_default(true);
    let cancel_btn = PushButton::new("Cancel");
    let apply_btn = PushButton::new("Apply");

    // OK button - save and close
    let dlg = dialog.clone();
    let s = settings.clone();
    let path = settings_path.clone();
    ok_btn.clicked().connect(move |_| {
        // Would collect values from all widgets and save
        let _ = s.save_json(&path);
        dlg.accept();
    });

    // Cancel button - close without saving
    let dlg = dialog.clone();
    cancel_btn.clicked().connect(move |_| {
        dlg.reject();
    });

    // Apply button - save without closing
    let s = settings.clone();
    let path = settings_path.clone();
    apply_btn.clicked().connect(move |_| {
        // Would collect values from all widgets and save
        let _ = s.save_json(&path);
    });

    let mut button_row = HBoxLayout::new();
    button_row.set_spacing(8.0);
    button_row.add_stretch(1);
    button_row.add_widget(ok_btn.object_id());
    button_row.add_widget(cancel_btn.object_id());
    button_row.add_widget(apply_btn.object_id());

    let mut button_container = Container::new();
    button_container.set_layout(LayoutKind::from(button_row));

    // Main layout
    let mut layout = VBoxLayout::new();
    layout.set_content_margins(ContentMargins::uniform(16.0));
    layout.set_spacing(16.0);
    layout.add_widget(tabs.object_id());
    layout.add_widget(button_container.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    dialog.set_content_widget(container.object_id());
    dialog.open();

    app.run()
}
```

## Features Demonstrated

| Feature | Description |
|---------|-------------|
| **TabWidget** | Organize settings into tabs |
| **FormLayout** | Label-field arrangements |
| **GroupBox** | Titled setting groups |
| **Dialog** | Modal dialog with OK/Cancel |
| **Various inputs** | CheckBox, ComboBox, SpinBox, etc. |
| **Settings** | Persistent preferences |

## Exercises

1. **Add validation**: Validate settings before saving
2. **Add import/export**: Import/export settings to file
3. **Add search**: Search for settings by name
4. **Add keyboard shortcuts tab**: Configure shortcuts
5. **Add preview**: Live preview of appearance changes

## Related Examples

- [Text Editor](./text-editor.md) - Using settings
- [Theming Tutorial](../tutorials/theming.md) - Theme settings
