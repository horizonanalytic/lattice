# Architecture Overview

This guide explains the high-level architecture of Horizon Lattice.

## System Overview

Horizon Lattice is organized into several crates:

```
horizon-lattice          # Main crate (re-exports everything)
├── horizon-lattice-core     # Event loop, signals, properties, objects
├── horizon-lattice-render   # GPU rendering with wgpu
├── horizon-lattice-style    # CSS-like styling system
├── horizon-lattice-macros   # Procedural macros
├── horizon-lattice-multimedia  # Audio/video (optional)
└── horizon-lattice-net      # Networking (optional)
```

## Core Components

### Application and Event Loop

The `Application` singleton manages the main event loop. It:
- Processes platform events (window, input)
- Dispatches signals
- Schedules timers and async tasks
- Coordinates repainting

### Object System

All widgets inherit from `Object`, providing:
- Unique object IDs
- Parent-child relationships
- Dynamic properties
- Thread affinity tracking

### Widget System

The widget system provides:
- Base `Widget` trait with lifecycle methods
- `WidgetBase` for common functionality
- Event dispatch and propagation
- Focus management
- Coordinate mapping

### Rendering

The rendering system uses wgpu for GPU-accelerated 2D graphics:
- Immediate-mode `Renderer` trait
- Damage tracking for efficient updates
- Layer compositing with blend modes
- Text shaping and rendering

### Styling

The style system provides CSS-like styling:
- Selector matching (type, class, id, pseudo-class)
- Property inheritance
- Computed style caching

## Threading Model

Horizon Lattice follows Qt's threading model:

- **Main thread**: All UI operations must happen here
- **Worker threads**: Background computation via `ThreadPool`
- **Signal delivery**: Cross-thread signals are queued to the main thread

## Design Decisions

### Why Not Trait Objects for Widgets?

We use `dyn Widget` trait objects for flexibility, but store widgets in a registry with `Arc<Mutex<dyn Widget>>`. This allows:
- Parent-child relationships via IDs
- Safe cross-thread signal delivery
- Dynamic widget creation

### Why wgpu?

wgpu provides:
- Cross-platform GPU access (Vulkan, Metal, DX12, WebGPU)
- Safe Rust API
- Excellent performance for 2D rendering

### Why Signals Instead of Callbacks?

Signals provide:
- Type-safe connections at compile time
- Automatic cross-thread marshalling
- Multiple connections to a single signal
- Clean disconnection via `ConnectionId`
