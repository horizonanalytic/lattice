# Signals and Slots Guide

Signals are Horizon Lattice's mechanism for event-driven programming.

## Basic Usage

```rust,ignore
// Connect to a signal
let conn_id = button.clicked().connect(|_| {
    println!("Button clicked!");
});

// Disconnect later if needed
button.clicked().disconnect(conn_id);
```

## Signal Types

### Parameterless Signals

```rust,ignore
button.clicked().connect(|_| { });
window.closed().connect(|_| { });
```

### Signals with Parameters

```rust,ignore
text_edit.text_changed().connect(|new_text: &String| {
    println!("Text is: {}", new_text);
});

slider.value_changed().connect(|&value: &i32| {
    println!("Value: {}", value);
});
```

## Connection Types

```rust,ignore
// Auto (default) - Direct if same thread, Queued if different
signal.connect(handler);

// Direct - Called immediately, same thread required
signal.connect_with_type(handler, ConnectionType::Direct);

// Queued - Always posted to event loop
signal.connect_with_type(handler, ConnectionType::Queued);

// BlockingQueued - Queued but blocks until complete
signal.connect_with_type(handler, ConnectionType::BlockingQueued);
```

## Creating Custom Signals

```rust,ignore
use horizon_lattice_core::Signal;

struct Counter {
    base: WidgetBase,
    value: i32,
    value_changed: Signal<i32>,
}

impl Counter {
    pub fn value_changed(&self) -> &Signal<i32> {
        &self.value_changed
    }

    pub fn set_value(&mut self, value: i32) {
        if self.value != value {
            self.value = value;
            self.value_changed.emit(value);
        }
    }
}
```

## Scoped Connections

Automatically disconnect when the guard is dropped:

```rust,ignore
{
    let _guard = signal.connect_scoped(|_| { });
    // Signal is connected here
}
// Signal automatically disconnected when _guard dropped
```

## Thread Safety

Signals are thread-safe. Cross-thread emissions are automatically queued:

```rust,ignore
let signal = Arc::new(Signal::<String>::new());

// Connect from main thread
signal.connect(|msg| println!("Received: {}", msg));

// Emit from worker thread
let signal_clone = signal.clone();
std::thread::spawn(move || {
    signal_clone.emit("Hello from thread!".to_string());
});
```

## Best Practices

1. **Keep slots short** - Long operations should spawn tasks
2. **Avoid blocking** - Never block the main thread in a slot
3. **Use scoped connections** - When the receiver has a shorter lifetime
4. **Don't recurse** - Emitting the same signal from its handler can loop
