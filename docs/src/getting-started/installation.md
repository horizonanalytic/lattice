# Installation

This guide covers how to add Horizon Lattice to your Rust project.

## Requirements

- **Rust**: 1.75 or later (for async features)
- **Platform**: Windows 10+, macOS 11+, or Linux with X11/Wayland

### Platform-Specific Dependencies

#### Linux

On Linux, you'll need development headers for graphics and windowing:

```bash
# Ubuntu/Debian
sudo apt install libxkbcommon-dev libwayland-dev

# Fedora
sudo dnf install libxkbcommon-devel wayland-devel

# Arch
sudo pacman -S libxkbcommon wayland
```

#### macOS

No additional dependencies required. Xcode Command Line Tools are recommended:

```bash
xcode-select --install
```

#### Windows

No additional dependencies required. Visual Studio Build Tools are recommended.

## Adding to Your Project

Add Horizon Lattice to your `Cargo.toml`:

```toml
[dependencies]
horizon-lattice = "0.1"
```

### Optional Features

Horizon Lattice provides several optional features:

```toml
[dependencies]
horizon-lattice = { version = "0.1", features = ["multimedia", "network"] }
```

| Feature | Description |
|---------|-------------|
| `multimedia` | Audio/video playback support |
| `network` | HTTP client, WebSocket, TCP/UDP |
| `accessibility` | Screen reader support |

## Verifying Installation

Create a simple test application:

```rust,ignore
// src/main.rs
use horizon_lattice::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = Application::new()?;

    let mut window = Window::new();
    window.set_title("Installation Test");
    window.set_size(300, 200);
    window.show();

    app.run()
}
```

Run it:

```bash
cargo run
```

If a window appears, you're ready to go!

## Troubleshooting

### "Failed to create graphics context"

This usually means the GPU drivers don't support the required graphics API. Try:
- Updating your GPU drivers
- On Linux, ensure Vulkan is installed: `sudo apt install mesa-vulkan-drivers`

### Build errors on Linux

Ensure you have all development headers installed (see Platform-Specific Dependencies above).

## Next Steps

Continue to [Your First Application](./first-app.md) to build something more interesting.
