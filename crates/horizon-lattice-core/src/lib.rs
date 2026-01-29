//! Core systems for Horizon Lattice.
//!
//! This crate provides the foundational components of the Horizon Lattice GUI framework:

#![warn(missing_docs)]
// Allow complex types in the meta-object system - these are intentional for type-safe signal/slot
#![allow(clippy::type_complexity)]
//!
//! - **Event Loop**: The main application event loop built on winit
//! - **Application**: Global application state and lifecycle management
//! - **Object Model**: Parent-child ownership, naming, dynamic properties
//! - **Signal/Slot System**: Type-safe inter-object communication
//! - **Property System**: Reactive properties with change notification
//! - **Timers**: One-shot and repeating timer system
//! - **Task Queue**: Deferred/idle task processing
//! - **Scheduler**: Background work scheduling with one-shot and periodic tasks
//!
//! # Signal/Slot Example
//!
//! ```
//! use horizon_lattice_core::{Signal, Property};
//!
//! // Create a signal that notifies when a value changes
//! let value_changed = Signal::<i32>::new();
//!
//! // Connect a slot to handle the signal
//! let conn_id = value_changed.connect(|value| {
//!     println!("Value changed to: {}", value);
//! });
//!
//! // Emit the signal
//! value_changed.emit(42);
//!
//! // Disconnect when done
//! value_changed.disconnect(conn_id);
//! ```
//!
//! # Property Example
//!
//! ```
//! use horizon_lattice_core::{Property, Signal};
//!
//! // A reactive counter with change notification
//! struct Counter {
//!     value: Property<i32>,
//!     value_changed: Signal<i32>,
//! }
//!
//! impl Counter {
//!     fn new() -> Self {
//!         Self {
//!             value: Property::new(0),
//!             value_changed: Signal::new(),
//!         }
//!     }
//!
//!     fn increment(&self) {
//!         let new_value = self.value.get() + 1;
//!         if self.value.set(new_value) {
//!             self.value_changed.emit(new_value);
//!         }
//!     }
//! }
//! ```
//!
//! # Event Loop Example
//!
//! ```no_run
//! use horizon_lattice_core::{Application, LatticeEvent};
//! use std::time::Duration;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let app = Application::new()?;
//!
//!     // Set up an event handler
//!     app.set_event_handler(|event| {
//!         match event {
//!             LatticeEvent::Timer { id } => {
//!                 println!("Timer {:?} fired!", id);
//!             }
//!             _ => {}
//!         }
//!     });
//!
//!     // Start a repeating timer
//!     let _timer_id = app.start_repeating_timer(Duration::from_secs(1));
//!
//!     // Post a deferred task
//!     app.post_task(|| {
//!         println!("Idle task executed!");
//!     });
//!
//!     // Run the event loop (blocks until quit)
//!     Ok(app.run()?)
//! }
//! ```

mod application;
#[cfg(feature = "tokio")]
pub mod async_runtime;
mod error;
mod event;
pub mod invocation;
pub mod logging;
pub mod meta;
pub mod object;
pub mod progress;
pub mod property;
mod scheduler;
pub mod signal;
mod task;
pub mod thread_check;
pub mod threadpool;
mod timer;
pub mod worker;

pub use application::{Application, WindowEventHandler};
pub use error::{
    LatticeError, Result, SchedulerError, SignalError, ThreadError, ThreadPoolError, TimerError,
};
pub use event::{EventPriority, LatticeEvent};
pub use logging::{ObjectTreeDebug, PerfSpan, TreeFormatOptions, TreeStyle};
pub use meta::{
    MetaError, MetaObject, MetaProperty, MetaResult, MethodMeta, SignalMeta, TypeRegistry,
    init_type_registry,
};
pub use object::{
    Object, ObjectBase, ObjectError, ObjectId, ObjectRegistry, ObjectResult, SharedObjectRegistry,
    WidgetState, global_registry, init_global_registry, object_cast, object_cast_mut,
};
pub use progress::{AggregateProgress, ProgressReporter, ProgressUpdate};
pub use property::{
    Binding, IntoProperty, Property, PropertyError, PropertyMeta, ReadOnlyProperty,
};
pub use scheduler::{ScheduledTaskId, ScheduledTaskKind};
pub use signal::{ConnectionGuard, ConnectionId, ConnectionType, Signal, SignalEmitter};
pub use task::TaskId;
pub use thread_check::{
    ThreadAffinity, are_thread_checks_enabled, is_main_thread, main_thread_id,
    set_thread_checks_enabled,
};
pub use timer::TimerId;
pub use worker::{Worker, WorkerBuilder, WorkerConfig};

// Re-export winit types that users may need
pub use winit::event::Modifiers;
pub use winit::event_loop::ActiveEventLoop;
pub use winit::window::{Window, WindowAttributes, WindowId};
