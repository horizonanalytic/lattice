//! Core systems for Horizon Lattice.
//!
//! This crate provides the foundational components of the Horizon Lattice GUI framework:
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
mod error;
mod event;
pub mod invocation;
pub mod logging;
pub mod meta;
pub mod object;
pub mod property;
mod scheduler;
pub mod signal;
mod task;
pub mod threadpool;
mod timer;
pub mod worker;

pub use application::{Application, WindowEventHandler};
pub use error::{LatticeError, Result, SchedulerError, SignalError, ThreadPoolError, TimerError};
pub use event::{EventPriority, LatticeEvent};
pub use logging::{ObjectTreeDebug, PerfSpan, TreeFormatOptions, TreeStyle};
pub use meta::{
    init_type_registry, MetaError, MetaObject, MetaProperty, MetaResult, MethodMeta, SignalMeta,
    TypeRegistry,
};
pub use object::{
    global_registry, init_global_registry, object_cast, object_cast_mut, Object, ObjectBase,
    ObjectError, ObjectId, ObjectRegistry, ObjectResult, SharedObjectRegistry, WidgetState,
};
pub use property::{Binding, IntoProperty, Property, PropertyError, PropertyMeta, ReadOnlyProperty};
pub use signal::{ConnectionGuard, ConnectionId, ConnectionType, Signal, SignalEmitter};
pub use scheduler::{ScheduledTaskId, ScheduledTaskKind};
pub use task::TaskId;
pub use timer::TimerId;
pub use worker::{Worker, WorkerBuilder, WorkerConfig};

// Re-export winit types that users may need
pub use winit::event::Modifiers;
pub use winit::event_loop::ActiveEventLoop;
pub use winit::window::{Window, WindowAttributes, WindowId};
