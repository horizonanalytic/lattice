# Threading Guide

Horizon Lattice follows a single-threaded UI model with support for background tasks.

## Threading Model

- **Main thread**: All UI operations must happen here
- **Worker threads**: For CPU-intensive or blocking operations
- **Signal marshalling**: Cross-thread signals are automatically queued

## Main Thread Rule

UI widgets are not thread-safe. Always access them from the main thread:

```rust,no_run
use horizon_lattice::Application;

// BAD - Don't do this!
// std::thread::spawn(|| {
//     label.set_text("Updated");  // Undefined behavior!
// });

// GOOD - Post to main thread
fn update_label_safely(app: &Application) {
    app.post_task(|| {
        // UI operations are safe here - runs on main thread
        println!("This runs on the main thread!");
    });
}
```

## Thread Pool

Use `ThreadPool` for CPU-intensive work:

```rust
use horizon_lattice_core::threadpool::{ThreadPool, ThreadPoolConfig};

// Create a custom thread pool
let pool = ThreadPool::new(ThreadPoolConfig::with_threads(4))
    .expect("Failed to create thread pool");

// Spawn a background task
let handle = pool.spawn(|| {
    // Heavy computation here
    let mut sum = 0u64;
    for i in 0..1_000_000 {
        sum += i;
    }
    sum
});

// Wait for the result
let result = handle.wait();
assert_eq!(result, Some(499999500000));
```

## Thread Pool with UI Callbacks

Spawn tasks that deliver results to the main thread:

```rust,no_run
use horizon_lattice_core::threadpool::ThreadPool;

let pool = ThreadPool::global();

// Spawn a task that delivers its result to the UI thread
pool.spawn_with_callback(
    || {
        // Background work - runs on worker thread
        std::thread::sleep(std::time::Duration::from_millis(100));
        "computed result".to_string()
    },
    |result| {
        // This callback runs on the UI thread
        println!("Got result: {}", result);
    },
);
```

## Cancellable Tasks

Use `CancellationToken` for cooperative task cancellation:

```rust
use horizon_lattice_core::threadpool::{ThreadPool, ThreadPoolConfig, CancellationToken};
use std::time::Duration;

let pool = ThreadPool::new(ThreadPoolConfig::with_threads(2)).unwrap();

let (handle, token) = pool.spawn_cancellable(|cancel_token| {
    for i in 0..100 {
        if cancel_token.is_cancelled() {
            return format!("Cancelled at step {}", i);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    "Completed".to_string()
});

// Cancel after a short delay
std::thread::sleep(Duration::from_millis(50));
token.cancel();

// The task will return early due to cancellation
let result = handle.wait();
assert!(result.is_some());
println!("Task result: {:?}", result);
```

## Worker Objects

For persistent background workers that process tasks sequentially:

```rust
use horizon_lattice_core::worker::Worker;
use std::sync::{Arc, atomic::{AtomicI32, Ordering}};

// Create a worker that produces String results
let worker = Worker::<String>::new();
let counter = Arc::new(AtomicI32::new(0));

// Connect to the result signal
let counter_clone = counter.clone();
worker.on_result().connect(move |result| {
    println!("Worker produced: {}", result);
    counter_clone.fetch_add(1, Ordering::SeqCst);
});

// Send tasks to the worker (processed sequentially)
worker.send(|| "Task 1 complete".to_string());
worker.send(|| "Task 2 complete".to_string());

// Wait for processing
std::thread::sleep(std::time::Duration::from_millis(100));

// Graceful shutdown
worker.stop();
worker.join();

assert!(counter.load(Ordering::SeqCst) >= 1);
```

## Worker with Callbacks

Send tasks with direct callbacks that bypass the signal:

```rust
use horizon_lattice_core::worker::Worker;
use std::sync::{Arc, Mutex};

let worker = Worker::<i32>::new();
let result_holder = Arc::new(Mutex::new(None));

let result_clone = result_holder.clone();
worker.send_with_callback(
    || {
        // Compute something
        42 * 2
    },
    move |result| {
        // Callback receives the result
        *result_clone.lock().unwrap() = Some(result);
    },
);

// Wait for processing
std::thread::sleep(std::time::Duration::from_millis(100));

assert_eq!(*result_holder.lock().unwrap(), Some(84));

worker.stop_and_join();
```

## Progress Reporting

Report progress from background tasks:

```rust
use horizon_lattice_core::progress::ProgressReporter;
use std::sync::{Arc, Mutex};

let reporter = ProgressReporter::new();
let progress_values = Arc::new(Mutex::new(Vec::new()));

// Connect to progress updates
let values_clone = progress_values.clone();
reporter.on_progress_changed().connect(move |&progress| {
    values_clone.lock().unwrap().push(progress);
});

// Simulate progress updates
reporter.set_progress(0.25);
reporter.set_progress(0.50);
reporter.set_progress(0.75);
reporter.set_progress(1.0);

// Verify progress was tracked
let values = progress_values.lock().unwrap();
assert!(values.len() >= 4);
assert!((reporter.progress() - 1.0).abs() < f32::EPSILON);
```

## Progress with Status Messages

Combine progress values with status messages:

```rust
use horizon_lattice_core::progress::ProgressReporter;

let reporter = ProgressReporter::new();

// Connect to combined updates
reporter.on_updated().connect(|update| {
    if let Some(ref msg) = update.message {
        println!("Progress: {:.0}% - {}", update.progress * 100.0, msg);
    }
});

// Update both progress and message atomically
reporter.update(0.25, "Loading resources...");
reporter.update(0.50, "Processing data...");
reporter.update(0.75, "Generating output...");
reporter.update(1.0, "Complete!");

assert_eq!(reporter.message(), Some("Complete!".to_string()));
```

## Aggregate Progress

For multi-step operations, combine weighted sub-tasks:

```rust
use horizon_lattice_core::progress::AggregateProgress;

let mut aggregate = AggregateProgress::new();

// Add weighted sub-tasks (weight determines contribution to total)
let download = aggregate.add_task("download", 3.0);  // 75% of total weight
let process = aggregate.add_task("process", 1.0);    // 25% of total weight

// Initial state
assert_eq!(aggregate.progress(), 0.0);

// Complete download only (75% of total due to weight)
download.set_progress(1.0);
assert!((aggregate.progress() - 0.75).abs() < 0.01);

// Complete processing (now at 100%)
process.set_progress(1.0);
assert!((aggregate.progress() - 1.0).abs() < 0.01);
```

## Tasks with Progress Reporting

Combine thread pool tasks with progress reporting:

```rust
use horizon_lattice_core::threadpool::{ThreadPool, ThreadPoolConfig};
use std::time::Duration;

let pool = ThreadPool::new(ThreadPoolConfig::with_threads(2)).unwrap();

let (handle, token, reporter) = pool.spawn_with_progress(|cancel, progress| {
    for i in 0..=10 {
        if cancel.is_cancelled() {
            return "Cancelled".to_string();
        }
        progress.update(i as f32 / 10.0, format!("Step {} of 10", i));
        std::thread::sleep(Duration::from_millis(5));
    }
    "Complete".to_string()
});

// Connect to progress updates
reporter.on_progress_changed().connect(|&p| {
    println!("Progress: {:.0}%", p * 100.0);
});

// Wait for completion
let result = handle.wait();
assert_eq!(result, Some("Complete".to_string()));
assert!((reporter.progress() - 1.0).abs() < f32::EPSILON);
```

## Thread Safety Checks

The framework includes thread affinity checking:

```rust
use horizon_lattice_core::thread_check::{is_main_thread, main_thread_id};

// Check if we're on the main thread
if is_main_thread() {
    println!("Running on main thread");
} else {
    println!("Running on a background thread");
}

// Get the main thread ID (set when Application is created)
if let Some(id) = main_thread_id() {
    println!("Main thread ID: {:?}", id);
}
```

## Best Practices

1. **Never block the main thread** - Keep UI responsive
2. **Minimize cross-thread communication** - Batch updates when possible
3. **Use signals for thread communication** - They handle marshalling automatically
4. **Prefer async for I/O** - Don't waste threads waiting on network/disk
5. **Check cancellation tokens** - Enable graceful shutdown of long-running tasks
6. **Use progress reporters** - Keep users informed about long operations
