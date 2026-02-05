# Horizon Lattice

A Rust-native GUI framework inspired by Qt's comprehensive design philosophy.

[![Crates.io](https://img.shields.io/crates/v/horizon-lattice.svg)](https://crates.io/crates/horizon-lattice)
[![Documentation](https://docs.rs/horizon-lattice/badge.svg)](https://docs.rs/horizon-lattice)
[![License](https://img.shields.io/crates/l/horizon-lattice.svg)](LICENSE-MIT)

## Overview

Horizon Lattice brings Qt's proven concepts—signals/slots, declarative UI, comprehensive widget set, and cross-platform support—to Rust, implemented idiomatically using Rust's ownership model and type system.

### Key Features

- **Type-safe signals and slots** - Compile-time checked connections, no runtime type errors
- **Procedural macro-based meta-object system** - No external code generation tools required
- **Modern graphics backend** - GPU-accelerated rendering via wgpu (Vulkan, Metal, DX12)
- **Comprehensive widget library** - Buttons, text inputs, lists, tables, trees, dialogs, and more
- **CSS-like styling** - Flexible theming with hot-reload support during development
- **Cross-platform** - Linux, Windows, and macOS from a single codebase
- **Pure Rust** - No C++ dependencies, fully memory-safe

## Installation

Add Horizon Lattice to your `Cargo.toml`:

```toml
[dependencies]
horizon-lattice = "1.0"
```

### Optional Features

```toml
[dependencies]
horizon-lattice = { version = "1.0", features = ["networking", "multimedia"] }
```

| Feature | Description | Default |
|---------|-------------|---------|
| `accessibility` | Screen reader and accessibility support | Yes |
| `notifications` | Desktop notifications | Yes |
| `power-management` | Battery and power status | Yes |
| `system-theme` | Dark/light mode detection | Yes |
| `localization` | ICU-based locale formatting | No |
| `networking` | HTTP, WebSocket, TCP/UDP, gRPC | No |
| `multimedia` | Audio playback and sound effects | No |

## Quick Start

```rust
use horizon_lattice::prelude::*;

fn main() {
    let app = Application::new();

    let window = MainWindow::new();
    window.set_title("Hello, Horizon Lattice!");
    window.resize(400, 300);

    let button = PushButton::new("Click me!");
    button.on_clicked(|| {
        println!("Button clicked!");
    });

    let layout = VBoxLayout::new();
    layout.add_widget(&button);
    window.set_layout(&layout);

    window.show();
    app.exec();
}
```

## Documentation

- [API Documentation](https://docs.rs/horizon-lattice)
- [User Guide](https://github.com/horizonanalytic/lattice/tree/main/docs)
- [Examples](https://github.com/horizonanalytic/lattice/tree/main/crates/horizon-lattice/examples)

## Crate Structure

Horizon Lattice is organized as a workspace of specialized crates:

| Crate | Description |
|-------|-------------|
| `horizon-lattice` | Main crate with prelude and re-exports |
| `horizon-lattice-core` | Event loop, object model, signals (no GUI) |
| `horizon-lattice-macros` | Procedural macros (`#[derive(Object)]`, etc.) |
| `horizon-lattice-render` | Graphics abstraction and wgpu backend |
| `horizon-lattice-style` | CSS-like styling and theming |
| `horizon-lattice-net` | Networking (optional) |
| `horizon-lattice-multimedia` | Audio playback (optional) |

## Platform Support

| Platform | Status |
|----------|--------|
| Linux x86_64 | Fully supported |
| Windows x86_64 | Fully supported |
| macOS x86_64/aarch64 | Fully supported |
| Linux aarch64 | Best effort |
| WebAssembly | Planned |

## Minimum Supported Rust Version

Horizon Lattice requires **Rust 1.85.0** or later (Edition 2024).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development workflow and release process.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

Horizon Lattice is developed by [Horizon Analytic Studios, LLC](https://horizonanalytic.com).

This project draws inspiration from Qt's design philosophy while reimagining its concepts for Rust's unique strengths.
