# Threading Guide

Horizon Lattice follows a single-threaded UI model with support for background tasks.

## Threading Model

- **Main thread**: All UI operations must happen here
- **Worker threads**: For CPU-intensive or blocking operations
- **Signal marshalling**: Cross-thread signals are automatically queued

## Main Thread Rule

UI widgets are not thread-safe. Always access them from the main thread:

```rust,ignore
// BAD - Don't do this!
std::thread::spawn(|| {
    label.set_text("Updated");  // Undefined behavior!
});

// GOOD - Post to main thread
Application::instance().invoke_later(|| {
    label.set_text("Updated");
});
```

## Background Tasks

Use `ThreadPool` for CPU-intensive work:

```rust,ignore
use horizon_lattice_core::ThreadPool;

let pool = ThreadPool::global();

pool.execute(|| {
    // Heavy computation here
    let result = expensive_calculation();

    // Post result back to UI thread
    Application::instance().invoke_later(move || {
        label.set_text(&format!("Result: {}", result));
    });
});
```

## Async Operations

Use the async runtime for I/O-bound tasks:

```rust,ignore
use horizon_lattice_core::async_runtime;

async_runtime::spawn(async move {
    let data = fetch_from_server().await?;

    // Update UI on main thread
    Application::instance().invoke_later(move || {
        update_ui_with_data(&data);
    });

    Ok(())
});
```

## Worker Objects

For persistent background workers:

```rust,ignore
use horizon_lattice_core::Worker;

let worker = Worker::new("DataProcessor");

// Send work to the worker
worker.post(|| {
    process_data();
});

// Worker has its own signal for completion
worker.finished().connect(|result| {
    // Called on main thread
    handle_result(result);
});
```

## Progress Reporting

Report progress from background tasks:

```rust,ignore
use horizon_lattice_core::Progress;

let progress = Progress::new();

// Connect UI to progress
progress.progress_changed().connect(|&value| {
    progress_bar.set_value(value);
});

progress.status_changed().connect(|status| {
    label.set_text(status);
});

// In background thread
pool.execute(move || {
    for i in 0..100 {
        do_work_step(i);
        progress.set_progress(i as f32 / 100.0);
        progress.set_status(&format!("Processing step {}", i));
    }
    progress.set_completed();
});
```

## Thread Safety Checks

Debug builds include thread safety assertions:

```rust,ignore
// Panics in debug builds if not on main thread
debug_assert_main_thread!();

// Marks a type as main-thread-only
struct MainThreadWidget {
    _marker: PhantomMainThread,
}
```

## Best Practices

1. **Never block the main thread** - Keep UI responsive
2. **Minimize cross-thread communication** - Batch updates
3. **Use signals for thread communication** - They handle marshalling
4. **Prefer async for I/O** - Don't waste threads waiting
5. **Test with thread sanitizers** - Catch races early
