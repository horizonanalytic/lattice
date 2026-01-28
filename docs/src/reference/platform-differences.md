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

---

> **Note**: This reference is under construction.
