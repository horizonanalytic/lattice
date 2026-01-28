# Platform Differences

Behavior differences across Windows, macOS, and Linux.

## Window Management

### Title Bar

| Feature | Windows | macOS | Linux |
|---------|---------|-------|-------|
| Custom title bar | Supported | Limited | Varies by WM |
| Traffic lights position | N/A | Left | N/A |
| Menu in title bar | Supported | System menu bar | Supported |

### Window Decorations

- **Windows**: Standard Win32 decorations
- **macOS**: Native NSWindow decorations
- **Linux**: Depends on window manager (X11/Wayland)

## Styling

### System Colors

Use `SystemTheme::accent_color()` for the platform accent color.

| Platform | Accent Color Source |
|----------|---------------------|
| Windows | WinRT UISettings |
| macOS | NSColor.controlAccentColor |
| Linux | XDG Portal (if available) |

### Dark Mode

Use `SystemTheme::color_scheme()` to detect light/dark mode.

| Platform | Detection Method |
|----------|------------------|
| Windows | AppsUseLightTheme registry |
| macOS | NSApp.effectiveAppearance |
| Linux | XDG Portal color-scheme |

## Text Rendering

### Font Selection

System fonts vary by platform:
- **Windows**: Segoe UI
- **macOS**: SF Pro / San Francisco
- **Linux**: System dependent (often DejaVu)

### Font Rendering

| Feature | Windows | macOS | Linux |
|---------|---------|-------|-------|
| Subpixel AA | ClearType | Native | FreeType |
| Font hinting | Strong | None | Configurable |

## Input

### Keyboard

| Feature | Windows | macOS | Linux |
|---------|---------|-------|-------|
| Command key | Ctrl | Cmd | Ctrl |
| Context menu | Application key | Ctrl+Click | Application key |
| IME | TSF | Input Sources | IBus/Fcitx |

### Touch

| Feature | Windows | macOS | Linux |
|---------|---------|-------|-------|
| Touch events | Native | Native | Via libinput |
| Gestures | WM_GESTURE | NSEvent | Limited |

## File System

### Path Conventions

| Platform | Config Dir | Data Dir |
|----------|------------|----------|
| Windows | `%APPDATA%` | `%LOCALAPPDATA%` |
| macOS | `~/Library/Application Support` | Same |
| Linux | `~/.config` | `~/.local/share` |

Use `platform::directories()` for cross-platform paths.

## Known Limitations

### Linux

- High contrast detection not fully implemented
- Some advanced clipboard formats not supported on Wayland
- Native file dialogs depend on portal availability

### macOS

- Custom title bar colors limited
- Some animations may differ from system style

### Windows

- DPI scaling may require manifest for older apps
- Per-monitor DPI awareness needs explicit opt-in

## Graphics Backend

### Renderer Selection

| Platform | Primary Backend | Fallback |
|----------|-----------------|----------|
| Windows | Direct3D 12 | Vulkan, Direct3D 11 |
| macOS | Metal | - |
| Linux | Vulkan | OpenGL |

### Performance Considerations

```rust,ignore
use horizon_lattice::render::GraphicsConfig;

// Force specific backend
let config = GraphicsConfig::new()
    .with_preferred_backend(Backend::Vulkan);
```

## Clipboard

### Supported Formats

| Format | Windows | macOS | Linux |
|--------|---------|-------|-------|
| Text | Full | Full | Full |
| HTML | Full | Full | Partial |
| Images | Full | Full | X11 only |
| Files | Full | Full | Wayland limited |

### Async Clipboard (Wayland)

On Wayland, clipboard operations may be asynchronous:

```rust,ignore
use horizon_lattice::clipboard::Clipboard;

// Prefer async API on Wayland
Clipboard::get_text_async(|text| {
    if let Some(t) = text {
        println!("Got: {}", t);
    }
});
```

## Drag and Drop

| Feature | Windows | macOS | Linux |
|---------|---------|-------|-------|
| File drops | Full | Full | Full |
| Custom data | Full | Full | Partial |
| Drag images | Full | Full | X11 only |

## Window Behavior

### Fullscreen

```rust,ignore
// Native fullscreen (best integration)
window.set_fullscreen(FullscreenMode::Native);

// Borderless fullscreen (consistent across platforms)
window.set_fullscreen(FullscreenMode::Borderless);
```

| Mode | Windows | macOS | Linux |
|------|---------|-------|-------|
| Native | Win32 | NSWindow | WM dependent |
| Borderless | Consistent | Consistent | Consistent |

### Always on Top

```rust,ignore
window.set_always_on_top(true);
```

Works consistently across all platforms.

### Transparency

```rust,ignore
window.set_transparent(true);
window.set_opacity(0.9);
```

| Feature | Windows | macOS | Linux |
|---------|---------|-------|-------|
| Window opacity | Full | Full | Compositor dependent |
| Transparent regions | Full | Full | Compositor dependent |

## Dialogs

### Native Dialogs

| Dialog | Windows | macOS | Linux |
|--------|---------|-------|-------|
| File Open/Save | IFileDialog | NSOpenPanel | Portal/GTK |
| Color Picker | ChooseColor | NSColorPanel | Portal/GTK |
| Font Picker | ChooseFont | NSFontPanel | Portal/GTK |
| Message Box | MessageBox | NSAlert | Portal/GTK |

### Linux Portal Integration

On Linux, Horizon Lattice uses XDG Desktop Portal when available:

```rust,ignore
use horizon_lattice::platform::linux;

// Check if portals are available
if linux::portals_available() {
    // Native dialogs will use portals
} else {
    // Falls back to GTK dialogs
}
```

## Keyboard Shortcuts

### Modifier Key Mapping

| Action | Windows/Linux | macOS |
|--------|---------------|-------|
| Copy | Ctrl+C | Cmd+C |
| Paste | Ctrl+V | Cmd+V |
| Cut | Ctrl+X | Cmd+X |
| Undo | Ctrl+Z | Cmd+Z |
| Redo | Ctrl+Y | Cmd+Shift+Z |
| Select All | Ctrl+A | Cmd+A |
| Save | Ctrl+S | Cmd+S |
| Find | Ctrl+F | Cmd+F |
| Close Window | Alt+F4 | Cmd+W |
| Quit | Alt+F4 | Cmd+Q |

Horizon Lattice automatically maps shortcuts appropriately per platform.

## Accessibility

### Screen Readers

| Platform | Supported API |
|----------|---------------|
| Windows | UI Automation |
| macOS | NSAccessibility |
| Linux | AT-SPI2 |

### High Contrast

```rust,ignore
use horizon_lattice::platform::SystemTheme;

if SystemTheme::is_high_contrast() {
    // Adjust colors for visibility
}
```

| Platform | Detection |
|----------|-----------|
| Windows | SystemParametersInfo |
| macOS | NSWorkspace |
| Linux | Portal (partial) |

## Locale and Text

### Input Methods

| Platform | IME Framework |
|----------|---------------|
| Windows | TSF (Text Services Framework) |
| macOS | Input Sources |
| Linux | IBus, Fcitx, XIM |

### Right-to-Left Text

Full RTL support on all platforms. Use `TextDirection::Auto` for automatic detection:

```rust,ignore
use horizon_lattice::text::TextDirection;

label.set_text_direction(TextDirection::Auto);
```
