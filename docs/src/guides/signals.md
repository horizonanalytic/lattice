# Signals and Slots Guide

Signals are Horizon Lattice's mechanism for event-driven programming. They provide type-safe, thread-safe communication between objects.

## Basic Usage

Signals emit values that connected slots (callbacks) receive:

```rust
use horizon_lattice_core::Signal;

// Create a signal
let clicked = Signal::<()>::new();

// Connect a slot
let conn_id = clicked.connect(|_| {
    println!("Button clicked!");
});

// Emit the signal
clicked.emit(());

// Disconnect later if needed
clicked.disconnect(conn_id);
```

## Signal Types

### Parameterless Signals

For events that don't carry data:

```rust
use horizon_lattice_core::Signal;

let clicked = Signal::<()>::new();
clicked.connect(|_| println!("Clicked!"));
clicked.emit(());
```

### Signals with Parameters

For events that carry data:

```rust
use horizon_lattice_core::Signal;

// Single parameter
let text_changed = Signal::<String>::new();
text_changed.connect(|new_text| {
    println!("Text is: {}", new_text);
});
text_changed.emit("Hello".to_string());

// Primitive parameter (note the reference pattern)
let value_changed = Signal::<i32>::new();
value_changed.connect(|&value| {
    println!("Value: {}", value);
});
value_changed.emit(42);
```

### Signals with Multiple Parameters

Use tuples for multiple values:

```rust
use horizon_lattice_core::Signal;

let position_changed = Signal::<(f32, f32)>::new();
position_changed.connect(|(x, y)| {
    println!("Position: ({}, {})", x, y);
});
position_changed.emit((100.0, 200.0));
```

## Connection Types

Control how slots are invoked:

```rust
use horizon_lattice_core::{Signal, ConnectionType};

let signal = Signal::<i32>::new();

// Auto (default) - Direct if same thread, Queued if different
signal.connect(|&n| println!("Auto: {}", n));

// Direct - Called immediately, same thread
signal.connect_with_type(|&n| println!("Direct: {}", n), ConnectionType::Direct);

// Queued - Always posted to event loop (cross-thread safe)
signal.connect_with_type(|&n| println!("Queued: {}", n), ConnectionType::Queued);

signal.emit(42);
```

### Connection Type Details

| Type | Behavior | Use Case |
|------|----------|----------|
| `Auto` | Direct if same thread, Queued otherwise | Most situations (default) |
| `Direct` | Immediate, synchronous call | Same-thread, performance critical |
| `Queued` | Posted to event loop | Cross-thread communication |
| `BlockingQueued` | Queued but blocks until complete | Synchronization across threads |

## Creating Custom Signals

Embed signals in your types:

```rust
use horizon_lattice_core::{Signal, Property};

struct Counter {
    value: Property<i32>,
    value_changed: Signal<i32>,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            value: Property::new(0),
            value_changed: Signal::new(),
        }
    }

    pub fn value_changed(&self) -> &Signal<i32> {
        &self.value_changed
    }

    pub fn value(&self) -> i32 {
        self.value.get()
    }

    pub fn set_value(&self, new_value: i32) {
        if self.value.set(new_value) {
            self.value_changed.emit(new_value);
        }
    }

    pub fn increment(&self) {
        self.set_value(self.value() + 1);
    }
}

// Usage
let counter = Counter::new();
counter.value_changed().connect(|&v| println!("Counter: {}", v));
counter.increment();  // Prints: Counter: 1
counter.increment();  // Prints: Counter: 2
```

## Scoped Connections

Automatically disconnect when the guard is dropped (RAII pattern):

```rust
use horizon_lattice_core::Signal;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

let signal = Signal::<i32>::new();
let counter = Arc::new(AtomicI32::new(0));

{
    let counter_clone = counter.clone();
    let _guard = signal.connect_scoped(move |&n| {
        counter_clone.fetch_add(n, Ordering::SeqCst);
    });
    signal.emit(10);  // counter = 10
    // _guard is dropped here
}

signal.emit(20);  // Nothing happens, connection was dropped
assert_eq!(counter.load(Ordering::SeqCst), 10);
```

## Blocking Signal Emission

Temporarily disable a signal:

```rust
use horizon_lattice_core::Signal;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;

let signal = Signal::<i32>::new();
let counter = Arc::new(AtomicI32::new(0));

let counter_clone = counter.clone();
signal.connect(move |&n| {
    counter_clone.fetch_add(n, Ordering::SeqCst);
});

signal.emit(1);  // counter = 1
signal.set_blocked(true);
signal.emit(2);  // Blocked - nothing happens
signal.set_blocked(false);
signal.emit(3);  // counter = 4

assert_eq!(counter.load(Ordering::SeqCst), 4);
```

## Thread Safety

Signals are thread-safe (`Send + Sync`). Cross-thread emissions are automatically handled:

```rust
use horizon_lattice_core::{Signal, ConnectionType};
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};

let signal = Arc::new(Signal::<i32>::new());
let counter = Arc::new(AtomicI32::new(0));

// Connect from main thread
let counter_clone = counter.clone();
signal.connect_with_type(move |&n| {
    counter_clone.fetch_add(n, Ordering::SeqCst);
}, ConnectionType::Direct);

// Emit from worker thread
let signal_clone = signal.clone();
let handle = std::thread::spawn(move || {
    signal_clone.emit(42);
});

handle.join().unwrap();
assert_eq!(counter.load(Ordering::SeqCst), 42);
```

## Best Practices

1. **Keep slots short** - Long operations should spawn background tasks
2. **Avoid blocking** - Never block the main thread in a slot
3. **Use scoped connections** - When the receiver has a shorter lifetime than the signal
4. **Don't recurse** - Emitting the same signal from its handler can cause infinite loops
5. **Use Direct for performance** - When you know both sides are on the same thread
6. **Use Queued for safety** - When crossing thread boundaries or uncertain

## Common Patterns

### One-shot Connection

Connect, emit once, then auto-disconnect:

```rust
use horizon_lattice_core::Signal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

let signal = Signal::<()>::new();
let done = Arc::new(AtomicBool::new(false));

let done_clone = done.clone();
let id = signal.connect(move |_| {
    done_clone.store(true, Ordering::SeqCst);
});

signal.emit(());
signal.disconnect(id);  // Manually disconnect after first use
```

### Forwarding Signals

Chain signals together:

```rust
use horizon_lattice_core::Signal;
use std::sync::Arc;

let source = Arc::new(Signal::<String>::new());
let destination = Arc::new(Signal::<String>::new());

// Forward from source to destination
let dest_clone = destination.clone();
source.connect(move |s| {
    dest_clone.emit(s.clone());
});
```
