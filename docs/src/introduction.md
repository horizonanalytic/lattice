# Introduction

Welcome to **Horizon Lattice**, a Rust-native GUI framework inspired by Qt6's comprehensive design philosophy.

## What is Horizon Lattice?

Horizon Lattice is a cross-platform GUI toolkit built from the ground up in Rust. It takes Qt's proven concepts—signals/slots, declarative UI, comprehensive widget set, cross-platform support—and implements them idiomatically using Rust's ownership model and safety guarantees.

## Why Horizon Lattice?

### Pure Rust, No C++ Dependencies

Unlike Qt bindings, Horizon Lattice is written entirely in Rust. This means:
- No external MOC tool required
- Compile-time type checking for signals and slots
- Memory safety guaranteed by the Rust compiler
- Easy integration with the Rust ecosystem

### Qt-Inspired, Rust-Idiomatic

We've adopted Qt's battle-tested design patterns while making them feel natural in Rust:

| Feature | Qt | Horizon Lattice |
|---------|-----|-----------------|
| Code generation | External MOC tool | Rust proc-macros |
| Signal type safety | Runtime | Compile-time |
| Memory management | Manual + parent-child | Rust ownership |
| License | LGPL/Commercial | MIT/Apache 2.0 |

### Modern Graphics

Horizon Lattice uses modern graphics APIs through wgpu:
- Vulkan, Metal, DX12, and WebGPU backends
- GPU-accelerated 2D rendering
- Efficient damage tracking for minimal redraws

## Quick Example

```rust,ignore
use horizon_lattice::prelude::*;

fn main() -> Result<(), horizon_lattice::LatticeError> {
    let app = Application::new()?;

    let mut window = Window::new("Hello, Horizon Lattice!")
        .with_size(400.0, 300.0);

    let button = PushButton::new("Click me!");
    button.clicked().connect(|_checked| {
        println!("Button clicked!");
    });

    window.set_content_widget(button.object_id());
    window.show();

    app.run()
}
```

## Getting Help

- **API Documentation**: [docs.rs/horizon-lattice](https://docs.rs/horizon-lattice)
- **GitHub**: [github.com/horizonanalytic/lattice](https://github.com/horizonanalytic/lattice)
- **Issues**: Report bugs or request features on GitHub

## License

Horizon Lattice is dual-licensed under MIT and Apache 2.0. You may use it under either license.
