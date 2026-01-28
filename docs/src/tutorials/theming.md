# Tutorial: Theming

Learn to style your application with themes and switch between light and dark modes.

## What You'll Learn

- Understanding the theme system
- Applying built-in themes
- Creating custom themes
- Switching themes at runtime
- Detecting and following system dark mode
- Styling individual widgets

## Prerequisites

- Completed the [Custom Widgets](./custom-widget.md) tutorial
- Understanding of the Widget system
- Familiarity with the [Styling Guide](../guides/styling.md)

## The Theme System

Horizon Lattice uses a comprehensive theming system inspired by Material Design:

- **Theme** - Defines colors, typography, and widget defaults
- **ColorPalette** - The color scheme (primary, secondary, background, etc.)
- **StyleEngine** - Resolves and applies styles to widgets
- **ThemeMode** - Light, Dark, or High Contrast

## Step 1: Built-in Themes

Horizon Lattice provides three built-in themes:

```rust,ignore
use horizon_lattice_style::{Theme, ThemeMode};

// Light theme (default)
let light = Theme::light();

// Dark theme
let dark = Theme::dark();

// High contrast theme (accessibility)
let high_contrast = Theme::high_contrast();
```

### Using a Theme with StyleEngine

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{Label, PushButton, Container, Window};
use horizon_lattice::widget::layout::{VBoxLayout, LayoutKind};
use horizon_lattice_style::{StyleEngine, Theme};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    // Create style engine with dark theme
    let style_engine = StyleEngine::dark();
    app.set_style_engine(style_engine);

    let mut window = Window::new("Dark Theme App")
        .with_size(400.0, 300.0);

    let label = Label::new("Welcome to the dark side!");
    let button = PushButton::new("Click me");

    let mut layout = VBoxLayout::new();
    layout.add_widget(label.object_id());
    layout.add_widget(button.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Step 2: Understanding ColorPalette

The `ColorPalette` defines all colors used throughout the theme:

```rust,ignore
use horizon_lattice_style::ColorPalette;
use horizon_lattice::render::Color;

// Get the light palette
let palette = ColorPalette::light();

// Access specific colors
let primary = palette.primary;           // Main brand color
let background = palette.background;     // App background
let text = palette.text_primary;         // Primary text color
let error = palette.error;               // Error/danger color
```

### Color Categories

| Category | Colors | Purpose |
|----------|--------|---------|
| **Primary** | `primary`, `primary_light`, `primary_dark`, `on_primary` | Brand/accent colors |
| **Secondary** | `secondary`, `secondary_light`, `secondary_dark`, `on_secondary` | Complementary accent |
| **Background** | `background`, `surface`, `surface_variant` | Container backgrounds |
| **Text** | `text_primary`, `text_secondary`, `text_disabled` | Text colors |
| **Semantic** | `error`, `warning`, `success`, `info` | Status indicators |
| **Borders** | `border`, `border_light`, `divider` | Lines and separators |

### Light vs Dark Palette Colors

```rust,ignore
// Light palette
let light = ColorPalette::light();
assert_eq!(light.background, Color::from_rgb8(255, 255, 255)); // White
assert_eq!(light.text_primary, Color::from_rgb8(33, 33, 33));  // Near black

// Dark palette
let dark = ColorPalette::dark();
assert_eq!(dark.background, Color::from_rgb8(18, 18, 18));     // Near black
assert_eq!(dark.text_primary, Color::from_rgb8(255, 255, 255)); // White
```

## Step 3: Creating Custom Themes

Create a custom theme with your own color palette:

```rust,ignore
use horizon_lattice_style::{Theme, ThemeMode, ColorPalette};
use horizon_lattice::render::Color;

// Start with a base palette and customize
fn create_brand_theme() -> Theme {
    let mut palette = ColorPalette::light();

    // Set brand colors
    palette.primary = Color::from_hex("#6200EE").unwrap();       // Purple
    palette.primary_light = Color::from_hex("#9D46FF").unwrap();
    palette.primary_dark = Color::from_hex("#3700B3").unwrap();
    palette.on_primary = Color::WHITE;

    palette.secondary = Color::from_hex("#03DAC6").unwrap();     // Teal
    palette.secondary_light = Color::from_hex("#66FFF8").unwrap();
    palette.secondary_dark = Color::from_hex("#00A896").unwrap();
    palette.on_secondary = Color::BLACK;

    // Create theme from custom palette
    Theme::custom(ThemeMode::Light, palette)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let brand_theme = create_brand_theme();
    let style_engine = StyleEngine::new(brand_theme);
    app.set_style_engine(style_engine);

    // ... rest of app setup
    Ok(())
}
```

### Creating a Complete Dark Brand Theme

```rust,ignore
fn create_dark_brand_theme() -> Theme {
    let mut palette = ColorPalette::dark();

    // Adjust primary for dark backgrounds
    palette.primary = Color::from_hex("#BB86FC").unwrap();       // Light purple
    palette.primary_light = Color::from_hex("#E4B8FF").unwrap();
    palette.primary_dark = Color::from_hex("#8858C8").unwrap();
    palette.on_primary = Color::BLACK;

    palette.secondary = Color::from_hex("#03DAC6").unwrap();     // Teal
    palette.on_secondary = Color::BLACK;

    // Adjust backgrounds for OLED-friendly dark
    palette.background = Color::from_rgb8(0, 0, 0);              // Pure black
    palette.surface = Color::from_rgb8(30, 30, 30);
    palette.surface_variant = Color::from_rgb8(45, 45, 45);

    Theme::custom(ThemeMode::Dark, palette)
}
```

## Step 4: Switching Themes at Runtime

Switch between themes dynamically:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    PushButton, Label, Container, Window, ButtonVariant
};
use horizon_lattice::widget::layout::{VBoxLayout, HBoxLayout, LayoutKind};
use horizon_lattice_style::{StyleEngine, Theme};
use std::sync::{Arc, RwLock};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    // Wrap style engine for shared access
    let style_engine = Arc::new(RwLock::new(StyleEngine::light()));
    app.set_shared_style_engine(style_engine.clone());

    let mut window = Window::new("Theme Switcher")
        .with_size(400.0, 300.0);

    let label = Label::new("Current theme: Light");

    // Theme buttons
    let light_btn = PushButton::new("Light Theme");
    let dark_btn = PushButton::new("Dark Theme")
        .with_variant(ButtonVariant::Secondary);
    let contrast_btn = PushButton::new("High Contrast")
        .with_variant(ButtonVariant::Outlined);

    // Light theme button
    let engine = style_engine.clone();
    let label_clone = label.clone();
    light_btn.clicked().connect(move |_| {
        let mut eng = engine.write().unwrap();
        eng.set_theme(Theme::light());
        eng.invalidate_all(); // Refresh all widget styles
        label_clone.set_text("Current theme: Light");
    });

    // Dark theme button
    let engine = style_engine.clone();
    let label_clone = label.clone();
    dark_btn.clicked().connect(move |_| {
        let mut eng = engine.write().unwrap();
        eng.set_theme(Theme::dark());
        eng.invalidate_all();
        label_clone.set_text("Current theme: Dark");
    });

    // High contrast button
    let engine = style_engine.clone();
    let label_clone = label.clone();
    contrast_btn.clicked().connect(move |_| {
        let mut eng = engine.write().unwrap();
        eng.set_theme(Theme::high_contrast());
        eng.invalidate_all();
        label_clone.set_text("Current theme: High Contrast");
    });

    // Button row
    let mut button_row = HBoxLayout::new();
    button_row.set_spacing(8.0);
    button_row.add_widget(light_btn.object_id());
    button_row.add_widget(dark_btn.object_id());
    button_row.add_widget(contrast_btn.object_id());

    let mut button_container = Container::new();
    button_container.set_layout(LayoutKind::from(button_row));

    // Main layout
    let mut layout = VBoxLayout::new();
    layout.set_spacing(20.0);
    layout.add_widget(label.object_id());
    layout.add_widget(button_container.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Step 5: Following System Dark Mode

Automatically follow the system's dark mode setting:

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::platform::{SystemTheme, ColorScheme, ThemeWatcher, ThemeAutoUpdater};
use horizon_lattice_style::{StyleEngine, Theme};
use std::sync::{Arc, RwLock};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    // Detect initial system theme
    let initial_theme = match SystemTheme::color_scheme() {
        ColorScheme::Dark => Theme::dark(),
        _ => Theme::light(),
    };

    let style_engine = Arc::new(RwLock::new(StyleEngine::new(initial_theme)));
    app.set_shared_style_engine(style_engine.clone());

    // Set up automatic theme updates
    let watcher = ThemeWatcher::new()?;
    let auto_updater = ThemeAutoUpdater::new(watcher, style_engine.clone());
    auto_updater.start()?;

    // ... rest of app setup

    app.run()
}
```

### Manual Theme Watching

For more control, handle theme changes manually:

```rust,ignore
use horizon_lattice::platform::{ThemeWatcher, ColorScheme};
use horizon_lattice_style::{StyleEngine, Theme};
use std::sync::{Arc, RwLock};

fn setup_theme_watcher(
    style_engine: Arc<RwLock<StyleEngine>>,
) -> Result<ThemeWatcher, Box<dyn std::error::Error>> {
    let watcher = ThemeWatcher::new()?;

    // Connect to color scheme changes
    let engine = style_engine.clone();
    watcher.color_scheme_changed().connect(move |&scheme| {
        let mut eng = engine.write().unwrap();
        match scheme {
            ColorScheme::Dark => {
                eng.set_theme(Theme::dark());
                println!("Switched to dark theme");
            }
            ColorScheme::Light | ColorScheme::Unknown => {
                eng.set_theme(Theme::light());
                println!("Switched to light theme");
            }
        }
        eng.invalidate_all();
    });

    // Connect to high contrast changes
    let engine = style_engine.clone();
    watcher.high_contrast_changed().connect(move |&enabled| {
        if enabled {
            let mut eng = engine.write().unwrap();
            eng.set_theme(Theme::high_contrast());
            eng.invalidate_all();
            println!("Switched to high contrast theme");
        }
    });

    watcher.start()?;
    Ok(watcher)
}
```

### Checking System Settings

```rust,ignore
use horizon_lattice::platform::{SystemTheme, ColorScheme};

fn check_system_theme() {
    // Get current color scheme
    let scheme = SystemTheme::color_scheme();
    match scheme {
        ColorScheme::Light => println!("System is in light mode"),
        ColorScheme::Dark => println!("System is in dark mode"),
        ColorScheme::Unknown => println!("Could not detect system theme"),
    }

    // Check high contrast
    if SystemTheme::is_high_contrast() {
        println!("High contrast is enabled");
    }

    // Get system accent color (if available)
    if let Some(accent) = SystemTheme::accent_color() {
        println!("System accent color: {:?}", accent.color);
    }
}
```

## Step 6: Styling Individual Widgets

Apply custom styles to specific widgets:

### Inline Styles

```rust,ignore
use horizon_lattice::widget::widgets::{Label, PushButton};
use horizon_lattice_style::{Style, LengthValue};
use horizon_lattice::render::Color;

// Style a label
let mut label = Label::new("Styled Label");
label.set_style(
    Style::new()
        .color(Color::from_hex("#6200EE").unwrap())
        .font_size(LengthValue::Px(24.0))
        .font_weight(horizon_lattice_style::FontWeight::Bold)
        .build()
);

// Style a button
let mut button = PushButton::new("Custom Button");
button.set_style(
    Style::new()
        .background_color(Color::from_hex("#03DAC6").unwrap())
        .color(Color::BLACK)
        .padding_all(LengthValue::Px(16.0))
        .border_radius_all(8.0)
        .build()
);
```

### Widget Classes

Use CSS-like classes for reusable styles:

```rust,ignore
use horizon_lattice::widget::widgets::{Label, PushButton, Container};
use horizon_lattice_style::{StyleSheet, StylePriority, Selector, Style};
use horizon_lattice::render::Color;

// Create a stylesheet with class rules
let mut stylesheet = StyleSheet::application();

// Add a "highlight" class
stylesheet.add_rule(
    Selector::class("highlight"),
    Style::new()
        .background_color(Color::from_rgba(255, 235, 59, 0.3)) // Yellow tint
        .border_width_all(LengthValue::Px(2.0))
        .border_color(Color::from_rgb8(255, 235, 59))
        .border_radius_all(4.0)
        .build()
);

// Add a "large-text" class
stylesheet.add_rule(
    Selector::class("large-text"),
    Style::new()
        .font_size(LengthValue::Px(20.0))
        .line_height(1.6)
        .build()
);

// Register stylesheet with engine
style_engine.add_stylesheet(stylesheet);

// Apply classes to widgets
let mut label = Label::new("Highlighted text");
label.add_class("highlight");
label.add_class("large-text");
```

### State-based Styling

Style widgets differently based on state:

```rust,ignore
use horizon_lattice_style::{Selector, SelectorState};

// Style for hovered buttons
stylesheet.add_rule(
    Selector::widget("Button").with_state(SelectorState::Hovered),
    Style::new()
        .background_color(Color::from_hex("#7C4DFF").unwrap())
        .build()
);

// Style for pressed buttons
stylesheet.add_rule(
    Selector::widget("Button").with_state(SelectorState::Pressed),
    Style::new()
        .background_color(Color::from_hex("#5E35B1").unwrap())
        .build()
);

// Style for focused inputs
stylesheet.add_rule(
    Selector::widget("LineEdit").with_state(SelectorState::Focused),
    Style::new()
        .border_color(Color::from_hex("#6200EE").unwrap())
        .border_width_all(LengthValue::Px(2.0))
        .build()
);

// Style for disabled widgets
stylesheet.add_rule(
    Selector::any().with_state(SelectorState::Disabled),
    Style::new()
        .opacity(0.5)
        .build()
);
```

## Complete Example: Theme-Aware App

```rust,ignore
use horizon_lattice::Application;
use horizon_lattice::widget::widgets::{
    Label, PushButton, CheckBox, LineEdit, Container, Window, ButtonVariant
};
use horizon_lattice::widget::layout::{VBoxLayout, HBoxLayout, ContentMargins, LayoutKind};
use horizon_lattice::platform::{SystemTheme, ColorScheme, ThemeWatcher};
use horizon_lattice_style::{StyleEngine, Theme, ColorPalette, ThemeMode, Style, LengthValue};
use horizon_lattice::render::Color;
use std::sync::{Arc, RwLock};

fn create_custom_light_theme() -> Theme {
    let mut palette = ColorPalette::light();
    palette.primary = Color::from_hex("#1976D2").unwrap();     // Blue
    palette.secondary = Color::from_hex("#FF5722").unwrap();   // Orange
    Theme::custom(ThemeMode::Light, palette)
}

fn create_custom_dark_theme() -> Theme {
    let mut palette = ColorPalette::dark();
    palette.primary = Color::from_hex("#90CAF9").unwrap();     // Light blue
    palette.secondary = Color::from_hex("#FFAB91").unwrap();   // Light orange
    palette.background = Color::from_rgb8(18, 18, 18);
    Theme::custom(ThemeMode::Dark, palette)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    // Initialize with system theme preference
    let initial_theme = match SystemTheme::color_scheme() {
        ColorScheme::Dark => create_custom_dark_theme(),
        _ => create_custom_light_theme(),
    };

    let style_engine = Arc::new(RwLock::new(StyleEngine::new(initial_theme)));
    app.set_shared_style_engine(style_engine.clone());

    let mut window = Window::new("Theme-Aware App")
        .with_size(500.0, 400.0);

    // Title
    let mut title = Label::new("Settings");
    title.set_style(
        Style::new()
            .font_size(LengthValue::Px(24.0))
            .font_weight(horizon_lattice_style::FontWeight::Bold)
            .build()
    );

    // Theme selection
    let theme_label = Label::new("Theme:");

    let follow_system = CheckBox::new("Follow system theme");
    follow_system.set_checked(true);

    let light_btn = PushButton::new("Light");
    let dark_btn = PushButton::new("Dark")
        .with_variant(ButtonVariant::Secondary);

    // Name input
    let name_label = Label::new("Display name:");
    let name_input = LineEdit::new();
    name_input.set_placeholder("Enter your name...");

    // Save button
    let save_btn = PushButton::new("Save Settings")
        .with_default(true);

    // Track if following system
    let following_system = Arc::new(std::sync::atomic::AtomicBool::new(true));

    // Follow system checkbox
    let following = following_system.clone();
    let engine = style_engine.clone();
    follow_system.toggled().connect(move |&checked| {
        following.store(checked, std::sync::atomic::Ordering::SeqCst);
        if checked {
            // Switch to current system theme
            let mut eng = engine.write().unwrap();
            match SystemTheme::color_scheme() {
                ColorScheme::Dark => eng.set_theme(create_custom_dark_theme()),
                _ => eng.set_theme(create_custom_light_theme()),
            }
            eng.invalidate_all();
        }
    });

    // Light button
    let following = following_system.clone();
    let engine = style_engine.clone();
    let checkbox = follow_system.clone();
    light_btn.clicked().connect(move |_| {
        checkbox.set_checked(false);
        following.store(false, std::sync::atomic::Ordering::SeqCst);
        let mut eng = engine.write().unwrap();
        eng.set_theme(create_custom_light_theme());
        eng.invalidate_all();
    });

    // Dark button
    let following = following_system.clone();
    let engine = style_engine.clone();
    let checkbox = follow_system.clone();
    dark_btn.clicked().connect(move |_| {
        checkbox.set_checked(false);
        following.store(false, std::sync::atomic::Ordering::SeqCst);
        let mut eng = engine.write().unwrap();
        eng.set_theme(create_custom_dark_theme());
        eng.invalidate_all();
    });

    // Set up system theme watcher
    let watcher = ThemeWatcher::new()?;
    let engine = style_engine.clone();
    let following = following_system.clone();
    watcher.color_scheme_changed().connect(move |&scheme| {
        if following.load(std::sync::atomic::Ordering::SeqCst) {
            let mut eng = engine.write().unwrap();
            match scheme {
                ColorScheme::Dark => eng.set_theme(create_custom_dark_theme()),
                _ => eng.set_theme(create_custom_light_theme()),
            }
            eng.invalidate_all();
        }
    });
    watcher.start()?;

    // Save button action
    let input = name_input.clone();
    save_btn.clicked().connect(move |_| {
        let name = input.text();
        println!("Saved settings for: {}", name);
    });

    // Theme buttons row
    let mut theme_buttons = HBoxLayout::new();
    theme_buttons.set_spacing(8.0);
    theme_buttons.add_widget(light_btn.object_id());
    theme_buttons.add_widget(dark_btn.object_id());

    let mut theme_btn_container = Container::new();
    theme_btn_container.set_layout(LayoutKind::from(theme_buttons));

    // Main layout
    let mut layout = VBoxLayout::new();
    layout.set_content_margins(ContentMargins::uniform(24.0));
    layout.set_spacing(16.0);
    layout.add_widget(title.object_id());
    layout.add_widget(theme_label.object_id());
    layout.add_widget(follow_system.object_id());
    layout.add_widget(theme_btn_container.object_id());
    layout.add_stretch(1);
    layout.add_widget(name_label.object_id());
    layout.add_widget(name_input.object_id());
    layout.add_stretch(1);
    layout.add_widget(save_btn.object_id());

    let mut container = Container::new();
    container.set_layout(LayoutKind::from(layout));

    window.set_content_widget(container.object_id());
    window.show();

    app.run()
}
```

## Theme Variables

Themes expose CSS-like variables for consistent styling:

```rust,ignore
use horizon_lattice_style::ThemeVariables;

// Variables automatically created from palette
// --primary-color, --primary-light, --primary-dark
// --secondary-color, --secondary-light, --secondary-dark
// --background, --surface, --surface-variant
// --text-primary, --text-secondary, --text-disabled
// --error, --warning, --success, --info
// --border, --border-light, --divider

// Spacing variables
// --spacing-xs (4px), --spacing-sm (8px), --spacing-md (16px)
// --spacing-lg (24px), --spacing-xl (32px)

// Border radius variables
// --radius-sm, --radius-md, --radius-lg, --radius-full

// Font size variables
// --font-size-xs through --font-size-2xl
```

## Best Practices

### 1. Always Support Both Light and Dark

Design your custom themes in pairs:

```rust,ignore
fn get_theme(dark: bool) -> Theme {
    if dark {
        create_custom_dark_theme()
    } else {
        create_custom_light_theme()
    }
}
```

### 2. Use Semantic Colors

Use palette colors by meaning, not by value:

```rust,ignore
// Good - uses semantic meaning
let bg = palette.surface;
let text = palette.text_primary;
let accent = palette.primary;

// Bad - hardcoded colors that won't adapt
let bg = Color::WHITE;
let text = Color::BLACK;
```

### 3. Invalidate After Theme Changes

Always call `invalidate_all()` after changing themes:

```rust,ignore
engine.set_theme(new_theme);
engine.invalidate_all(); // Don't forget this!
```

### 4. Respect System Preferences

Default to following system theme, with manual override option:

```rust,ignore
// Good UX: follow system by default
let initial = match SystemTheme::color_scheme() {
    ColorScheme::Dark => Theme::dark(),
    _ => Theme::light(),
};

// Let users override manually if they want
```

### 5. Test High Contrast Mode

Always test your app with high contrast theme for accessibility:

```rust,ignore
// Ensure text is readable in high contrast
let hc_theme = Theme::high_contrast();
// Minimum 4.5:1 contrast ratio for text
```

## Next Steps

- [File Operations](./file-operations.md) - Save and load theme preferences
- [Styling Guide](../guides/styling.md) - Deep dive into the style system
- [Architecture Guide](../guides/architecture.md) - How theming integrates with the framework
