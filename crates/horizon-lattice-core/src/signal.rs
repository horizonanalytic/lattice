//! Signal/slot system for Horizon Lattice.
//!
//! This module provides a type-safe, Qt-inspired signal/slot mechanism for
//! inter-object communication. Signals are emitted by objects when their state
//! changes, and connected slots (callbacks) are invoked in response.
//!
//! # Key Types
//!
//! - [`Signal<Args>`] - The main signal type for emitting notifications
//! - [`ConnectionId`] - Unique identifier returned when connecting a slot
//! - [`ConnectionType`] - How a slot should be invoked (Direct, Queued, etc.)
//! - [`ConnectionGuard`] - RAII guard that disconnects when dropped
//!
//! # Connection Types
//!
//! - **Direct**: Slot is called immediately in the emitting thread
//! - **Queued**: Slot execution is deferred to the event loop (cross-thread safe)
//! - **Auto**: Direct if same thread, Queued otherwise (default)
//! - **BlockingQueued**: Like Queued, but blocks until the slot completes
//!
//! # Thread Safety
//!
//! Signals support cross-thread communication through queued connections. When
//! a slot is connected from thread A and the signal is emitted from thread B:
//!
//! - With [`ConnectionType::Auto`] (default), the slot is automatically queued
//!   to execute on thread A's event loop.
//! - With [`ConnectionType::Queued`], the slot is always queued regardless of
//!   which thread emits.
//! - With [`ConnectionType::BlockingQueued`], the emitting thread blocks until
//!   the slot finishes executing on the target thread.
//!
//! # Related Modules
//!
//! - [`crate::Property`] - Reactive properties that typically emit signals on change
//! - [`crate::Application`] - Provides the event loop for queued connections
//! - [`crate::Object`] - Base trait for types that use signals
//!
//! # Example
//!
//! ```
//! use horizon_lattice_core::Signal;
//!
//! // Create a signal that passes a string argument
//! let text_changed = Signal::<String>::new();
//!
//! // Connect a slot (closure)
//! let conn_id = text_changed.connect(|text| {
//!     println!("Text changed to: {}", text);
//! });
//!
//! // Emit the signal
//! text_changed.emit("Hello, World!".to_string());
//!
//! // Disconnect when done
//! text_changed.disconnect(conn_id);
//! ```
//!
//! # Guide
//!
//! For a comprehensive guide on the signal/slot pattern, see the
//! [Signals Guide](https://horizonanalyticstudios.github.io/horizon-lattice/guides/signals.html).

use std::any::Any;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::ThreadId;

use parking_lot::Mutex;
use slotmap::{new_key_type, SlotMap};

use crate::invocation::{completion_pair, invocation_registry, QueuedInvocation};

new_key_type! {
    /// A unique identifier for a signal-slot connection.
    ///
    /// Use this ID to disconnect a specific connection via [`Signal::disconnect`].
    /// The ID remains valid until the connection is explicitly disconnected or
    /// the signal is dropped.
    ///
    /// # Related
    ///
    /// - [`Signal::connect`] - Returns a `ConnectionId`
    /// - [`Signal::disconnect`] - Removes a connection by ID
    /// - [`ConnectionGuard`] - RAII alternative that auto-disconnects
    pub struct ConnectionId;
}

/// Specifies how a connected slot should be invoked when the signal is emitted.
///
/// Use with [`Signal::connect_with_type`] to control invocation behavior.
///
/// # Related
///
/// - [`Signal::connect`] - Uses [`ConnectionType::Auto`] by default
/// - [`Signal::connect_with_type`] - Allows specifying connection type
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConnectionType {
    /// Invoke the slot immediately in the current thread.
    ///
    /// This is the fastest option but requires the slot to be safe to call
    /// from any thread. Use this when you know the signal and slot are
    /// in the same thread.
    Direct,

    /// Queue the slot invocation to the receiver's event loop.
    ///
    /// This is safe for cross-thread communication. The slot will be
    /// invoked when the event loop processes pending events.
    Queued,

    /// Automatically choose Direct or Queued based on thread affinity.
    ///
    /// - Same thread: Direct invocation
    /// - Different thread: Queued invocation
    ///
    /// This is the default and recommended option for most use cases.
    #[default]
    Auto,

    /// Like Queued, but blocks the emitting thread until the slot completes.
    ///
    /// This is useful when you need to ensure a slot has finished before
    /// continuing, typically for synchronization purposes.
    ///
    /// # Warning
    ///
    /// Using `BlockingQueued` when emitting from the same thread that the
    /// slot will execute on will cause a **deadlock**. The emit will block
    /// waiting for the event loop to process the queued invocation, but the
    /// event loop is blocked waiting for emit to return.
    ///
    /// Always use `Auto` or `Direct` if the signal might be emitted from
    /// the target thread.
    BlockingQueued,
}

/// Internal storage for a single connection.
struct Connection<Args> {
    /// The slot function to invoke (Arc-wrapped for safe cross-thread capture).
    slot: Arc<dyn Fn(&Args) + Send + Sync>,
    /// How to invoke this slot.
    connection_type: ConnectionType,
    /// The thread this connection was created on (for Auto/Queued types).
    target_thread: ThreadId,
}

/// A type-safe signal that can have multiple connected slots.
///
/// Signals are the core of the observer pattern in Horizon Lattice. When a
/// signal is emitted, all connected slots are invoked with the provided arguments.
///
/// # Type Parameter
///
/// - `Args`: The argument type passed to connected slots. Use `()` for signals
///   with no arguments, or a tuple like `(String, i32)` for multiple arguments.
///
/// # Thread Safety
///
/// `Signal<Args>` is `Send + Sync` and can be safely shared between threads.
/// The [`ConnectionType`] determines how slots are invoked across thread boundaries.
///
/// # Related Types
///
/// - [`ConnectionId`] - Returned by [`connect`](Self::connect), used to disconnect
/// - [`ConnectionType`] - Controls how slots are invoked
/// - [`ConnectionGuard`] - RAII-style connection that auto-disconnects on drop
/// - [`crate::Property`] - Often paired with signals for change notification
pub struct Signal<Args> {
    /// All active connections.
    connections: Mutex<SlotMap<ConnectionId, Connection<Args>>>,
    /// Whether signal emission is temporarily blocked.
    blocked: AtomicBool,
    /// Counter for queued invocations (used for event identification).
    invocation_counter: AtomicU64,
}

impl<Args: Clone + Send + 'static> Default for Signal<Args> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Args: Clone + Send + 'static> Signal<Args> {
    /// Create a new signal with no connections.
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(SlotMap::with_key()),
            blocked: AtomicBool::new(false),
            invocation_counter: AtomicU64::new(0),
        }
    }

    /// Connect a slot (closure) to this signal.
    ///
    /// The slot will be invoked with `ConnectionType::Auto`, meaning it will
    /// be called directly if in the same thread, or queued otherwise.
    ///
    /// Returns a `ConnectionId` that can be used to disconnect the slot later.
    ///
    /// # Example
    ///
    /// ```
    /// use horizon_lattice_core::Signal;
    ///
    /// let signal = Signal::<String>::new();
    /// let id = signal.connect(|s| println!("Got: {}", s));
    /// signal.emit("Hello".to_string());
    /// ```
    pub fn connect<F>(&self, slot: F) -> ConnectionId
    where
        F: Fn(&Args) + Send + Sync + 'static,
    {
        self.connect_with_type(slot, ConnectionType::Auto)
    }

    /// Connect a slot with a specific connection type.
    ///
    /// # Example
    ///
    /// ```
    /// use horizon_lattice_core::{Signal, ConnectionType};
    ///
    /// let signal = Signal::<i32>::new();
    ///
    /// // Always invoke directly (fast, but not cross-thread safe)
    /// signal.connect_with_type(|n| println!("{}", n), ConnectionType::Direct);
    ///
    /// // Always queue (safe for cross-thread)
    /// signal.connect_with_type(|n| println!("{}", n), ConnectionType::Queued);
    ///
    /// signal.emit(42);
    /// ```
    pub fn connect_with_type<F>(&self, slot: F, connection_type: ConnectionType) -> ConnectionId
    where
        F: Fn(&Args) + Send + Sync + 'static,
    {
        let connection = Connection {
            slot: Arc::new(slot),
            connection_type,
            target_thread: std::thread::current().id(),
        };
        self.connections.lock().insert(connection)
    }

    /// Disconnect a specific slot by its connection ID.
    ///
    /// Returns `true` if the connection was found and removed, `false` otherwise.
    pub fn disconnect(&self, id: ConnectionId) -> bool {
        self.connections.lock().remove(id).is_some()
    }

    /// Disconnect all slots from this signal.
    pub fn disconnect_all(&self) {
        self.connections.lock().clear();
    }

    /// Get the number of connected slots.
    pub fn connection_count(&self) -> usize {
        self.connections.lock().len()
    }

    /// Block signal emission temporarily.
    ///
    /// While blocked, calls to `emit()` will do nothing. This is useful
    /// during initialization or batch updates to prevent cascading notifications.
    pub fn set_blocked(&self, blocked: bool) {
        self.blocked.store(blocked, Ordering::SeqCst);
    }

    /// Check if signal emission is currently blocked.
    pub fn is_blocked(&self) -> bool {
        self.blocked.load(Ordering::SeqCst)
    }

    /// Emit the signal, invoking all connected slots.
    ///
    /// If the signal is blocked, this does nothing. Otherwise, all connected
    /// slots are invoked according to their connection type:
    ///
    /// - `Direct`: Called immediately in the current thread
    /// - `Auto`: Called directly if same thread, queued otherwise
    /// - `Queued`: Always queued to the target thread's event loop
    /// - `BlockingQueued`: Queued, but blocks until slot completes
    ///
    /// # Arguments
    ///
    /// - `args`: The arguments to pass to each slot. These are cloned for
    ///   each Queued/BlockingQueued connection.
    #[tracing::instrument(skip_all, target = "horizon_lattice_core::signal", level = "trace")]
    pub fn emit(&self, args: Args) {
        if self.is_blocked() {
            tracing::trace!(target: "horizon_lattice_core::signal", "signal blocked, skipping emit");
            return;
        }

        let current_thread = std::thread::current().id();
        let connections = self.connections.lock();
        tracing::trace!(target: "horizon_lattice_core::signal", connection_count = connections.len(), "emitting signal");

        // Collect blocking waiters to wait on after releasing the lock
        let mut blocking_waiters = Vec::new();

        for (_, conn) in connections.iter() {
            match conn.connection_type {
                ConnectionType::Direct => {
                    // Always invoke directly, regardless of thread
                    (conn.slot)(&args);
                }
                ConnectionType::Auto => {
                    if conn.target_thread == current_thread {
                        // Same thread: call directly
                        (conn.slot)(&args);
                    } else {
                        // Different thread: queue for deferred execution
                        self.queue_invocation(conn.slot.clone(), args.clone(), false);
                    }
                }
                ConnectionType::Queued => {
                    // Always queue, even if same thread
                    self.queue_invocation(conn.slot.clone(), args.clone(), false);
                }
                ConnectionType::BlockingQueued => {
                    // Queue and prepare to block
                    if let Some(waiter) =
                        self.queue_invocation_blocking(conn.slot.clone(), args.clone())
                    {
                        blocking_waiters.push(waiter);
                    }
                }
            }
        }

        // Release the lock before waiting on blocking connections
        drop(connections);

        // Wait for all blocking connections to complete
        for waiter in blocking_waiters {
            waiter.wait();
        }
    }

    /// Queue an invocation to the event loop.
    fn queue_invocation(
        &self,
        slot: Arc<dyn Fn(&Args) + Send + Sync>,
        args: Args,
        _blocking: bool,
    ) {
        let _ = self.invocation_counter.fetch_add(1, Ordering::SeqCst);

        // Create the invocation closure that captures the slot and args
        let invocation = QueuedInvocation::new(move || {
            slot(&args);
        });

        // Register and post to event loop
        let invocation_id = invocation_registry().register(invocation);

        // Try to wake up the event loop
        if let Some(app) = crate::Application::try_instance() {
            let _ = app.post_event(crate::event::LatticeEvent::QueuedSignal { invocation_id });
        } else {
            // No event loop available - execute immediately as fallback
            // This can happen during testing or early initialization
            tracing::warn!(
                target: "horizon_lattice_core::signal",
                "No event loop available for queued signal, executing immediately"
            );
            if let Some(inv) = invocation_registry().take(invocation_id) {
                inv.execute();
            }
        }
    }

    /// Queue an invocation with blocking wait.
    fn queue_invocation_blocking(
        &self,
        slot: Arc<dyn Fn(&Args) + Send + Sync>,
        args: Args,
    ) -> Option<crate::invocation::CompletionWaiter> {
        let _ = self.invocation_counter.fetch_add(1, Ordering::SeqCst);

        // Create completion pair for synchronization
        let (handle, waiter) = completion_pair();

        // Create the invocation closure
        let invocation = QueuedInvocation::with_completion(
            move || {
                slot(&args);
            },
            handle,
        );

        // Register and post to event loop
        let invocation_id = invocation_registry().register(invocation);

        if let Some(app) = crate::Application::try_instance() {
            let _ = app.post_event(crate::event::LatticeEvent::QueuedSignal { invocation_id });
            Some(waiter)
        } else {
            // No event loop - execute immediately (no blocking needed)
            tracing::warn!(
                target: "horizon_lattice_core::signal",
                "No event loop available for blocking queued signal, executing immediately"
            );
            if let Some(inv) = invocation_registry().take(invocation_id) {
                inv.execute();
            }
            None
        }
    }

    /// Emit with explicit queuing through the event loop.
    ///
    /// This forces all slots to be invoked asynchronously through the event
    /// loop, regardless of their connection type. Use this when you need to
    /// guarantee deferred execution.
    ///
    /// Unlike `emit()`, which respects each connection's type, this method
    /// queues all invocations. This is useful for:
    /// - Deferring signal handling to avoid re-entrancy issues
    /// - Batching updates by queuing multiple signals
    ///
    /// Returns the number of slots that were queued, or 0 if the signal
    /// is blocked.
    pub fn emit_queued(&self, args: Args) -> usize {
        if self.is_blocked() {
            return 0;
        }

        let connections = self.connections.lock();
        let count = connections.len();

        for (_, conn) in connections.iter() {
            self.queue_invocation(conn.slot.clone(), args.clone(), false);
        }

        count
    }
}

// Signal is Send + Sync when Args is Send
unsafe impl<Args: Send> Send for Signal<Args> {}
unsafe impl<Args: Send> Sync for Signal<Args> {}

/// Type-erased signal emitter trait for dynamic signal access.
///
/// This trait allows working with signals without knowing their argument type,
/// which is useful for the meta-object system and dynamic property notifications.
pub trait SignalEmitter: Send + Sync {
    /// Disconnect a connection by ID.
    fn disconnect(&self, id: ConnectionId) -> bool;

    /// Disconnect all connections.
    fn disconnect_all(&self);

    /// Get the number of connections.
    fn connection_count(&self) -> usize;

    /// Check if blocked.
    fn is_blocked(&self) -> bool;

    /// Set blocked state.
    fn set_blocked(&self, blocked: bool);

    /// Get this as Any for downcasting.
    fn as_any(&self) -> &dyn Any;

    /// Get this as mutable Any for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<Args: Clone + Send + 'static> SignalEmitter for Signal<Args> {
    fn disconnect(&self, id: ConnectionId) -> bool {
        Signal::disconnect(self, id)
    }

    fn disconnect_all(&self) {
        Signal::disconnect_all(self);
    }

    fn connection_count(&self) -> usize {
        Signal::connection_count(self)
    }

    fn is_blocked(&self) -> bool {
        Signal::is_blocked(self)
    }

    fn set_blocked(&self, blocked: bool) {
        Signal::set_blocked(self, blocked);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// A connection guard that automatically disconnects when dropped.
///
/// This is useful for RAII-style connection management, ensuring connections
/// are cleaned up when the receiver goes out of scope. Created via
/// [`Signal::connect_scoped`].
///
/// # Related
///
/// - [`Signal::connect_scoped`] - Creates a `ConnectionGuard`
/// - [`ConnectionId`] - Manual connection management alternative
///
/// # Example
///
/// ```
/// use horizon_lattice_core::Signal;
/// use std::sync::atomic::{AtomicI32, Ordering};
/// use std::sync::Arc;
///
/// let signal = Signal::<i32>::new();
/// let counter = Arc::new(AtomicI32::new(0));
/// {
///     let counter_clone = counter.clone();
///     let _guard = signal.connect_scoped(move |&n| {
///         counter_clone.fetch_add(n, Ordering::SeqCst);
///     });
///     signal.emit(42);  // counter = 42
/// }
/// signal.emit(43);  // Nothing happens - connection was dropped
/// assert_eq!(counter.load(Ordering::SeqCst), 42);
/// ```
pub struct ConnectionGuard<Args: Clone + Send + 'static> {
    signal: *const Signal<Args>,
    id: ConnectionId,
}

impl<Args: Clone + Send + 'static> Signal<Args> {
    /// Connect a slot with automatic disconnection when the guard is dropped.
    ///
    /// # Safety
    ///
    /// The returned guard holds a raw pointer to this signal. The signal must
    /// outlive the guard. Using `Arc<Signal<Args>>` is recommended for shared ownership.
    pub fn connect_scoped<F>(&self, slot: F) -> ConnectionGuard<Args>
    where
        F: Fn(&Args) + Send + Sync + 'static,
    {
        let id = self.connect(slot);
        ConnectionGuard {
            signal: self as *const Signal<Args>,
            id,
        }
    }
}

impl<Args: Clone + Send + 'static> Drop for ConnectionGuard<Args> {
    fn drop(&mut self) {
        // SAFETY: The signal pointer is valid if the guard is used correctly.
        // The caller must ensure the signal outlives the guard.
        unsafe {
            if !self.signal.is_null() {
                let _ = (*self.signal).disconnect(self.id);
            }
        }
    }
}

// SAFETY: ConnectionGuard is Send + Sync because:
// - The raw pointer `signal` is only dereferenced in `drop()`, which is called
//   on the owning thread or when the guard is moved to another thread.
// - Signal<Args> itself is Send + Sync (uses Mutex internally for connections).
// - The ConnectionId is a simple Copy type (slotmap key).
// - The guard's safety contract (documented in `connect_scoped`) requires the
//   Signal to outlive the guard, which the caller must ensure.
unsafe impl<Args: Clone + Send + 'static> Send for ConnectionGuard<Args> {}
unsafe impl<Args: Clone + Send + 'static> Sync for ConnectionGuard<Args> {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_signal_connect_emit() {
        let signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        let received_clone = received.clone();
        signal.connect(move |&value| {
            received_clone.lock().push(value);
        });

        signal.emit(42);
        signal.emit(100);

        let values = received.lock();
        assert_eq!(*values, vec![42, 100]);
    }

    #[test]
    fn test_signal_disconnect() {
        let signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        let received_clone = received.clone();
        let conn_id = signal.connect(move |&value| {
            received_clone.lock().push(value);
        });

        signal.emit(1);
        assert!(signal.disconnect(conn_id));
        signal.emit(2);

        let values = received.lock();
        assert_eq!(*values, vec![1]); // Only received before disconnect
    }

    #[test]
    fn test_signal_blocked() {
        let signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        let received_clone = received.clone();
        signal.connect(move |&value| {
            received_clone.lock().push(value);
        });

        signal.emit(1);
        signal.set_blocked(true);
        signal.emit(2); // Should be ignored
        signal.set_blocked(false);
        signal.emit(3);

        let values = received.lock();
        assert_eq!(*values, vec![1, 3]);
    }

    #[test]
    fn test_multiple_connections() {
        let signal = Signal::<String>::new();
        let count = Arc::new(Mutex::new(0));

        for _ in 0..3 {
            let count_clone = count.clone();
            signal.connect(move |_| {
                *count_clone.lock() += 1;
            });
        }

        assert_eq!(signal.connection_count(), 3);
        signal.emit("test".to_string());
        assert_eq!(*count.lock(), 3);
    }

    #[test]
    fn test_disconnect_all() {
        let signal = Signal::<()>::new();

        for _ in 0..5 {
            signal.connect(|_| {});
        }

        assert_eq!(signal.connection_count(), 5);
        signal.disconnect_all();
        assert_eq!(signal.connection_count(), 0);
    }

    #[test]
    fn test_connection_guard() {
        let signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        {
            let received_clone = received.clone();
            let _guard = signal.connect_scoped(move |&value| {
                received_clone.lock().push(value);
            });
            signal.emit(1);
        } // Guard dropped here, connection should be removed

        signal.emit(2); // Should not be received

        let values = received.lock();
        assert_eq!(*values, vec![1]);
    }

    #[test]
    fn test_signal_with_no_args() {
        let signal = Signal::<()>::new();
        let called = Arc::new(AtomicBool::new(false));

        let called_clone = called.clone();
        signal.connect(move |_| {
            called_clone.store(true, Ordering::SeqCst);
        });

        signal.emit(());
        assert!(called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_signal_with_multiple_args() {
        let signal = Signal::<(String, i32)>::new();
        let received = Arc::new(Mutex::new(None));

        let received_clone = received.clone();
        signal.connect(move |args| {
            *received_clone.lock() = Some(args.clone());
        });

        signal.emit(("hello".to_string(), 42));

        let value = received.lock().clone();
        assert_eq!(value, Some(("hello".to_string(), 42)));
    }

    // -------------------------------------------------------------------------
    // Thread-safety tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_direct_connection_type() {
        // Direct connections should always call immediately in the current thread
        let signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let emitting_thread = Arc::new(Mutex::new(None));

        let received_clone = received.clone();
        let emitting_thread_clone = emitting_thread.clone();
        signal.connect_with_type(
            move |&value| {
                received_clone.lock().push(value);
                *emitting_thread_clone.lock() = Some(std::thread::current().id());
            },
            ConnectionType::Direct,
        );

        signal.emit(42);

        assert_eq!(*received.lock(), vec![42]);
        assert_eq!(
            *emitting_thread.lock(),
            Some(std::thread::current().id())
        );
    }

    #[test]
    fn test_cross_thread_direct_emit() {
        // Even with Direct type, slot should be called from emitting thread
        let signal = Arc::new(Signal::<i32>::new());
        let received = Arc::new(Mutex::new(Vec::new()));
        let slot_thread = Arc::new(Mutex::new(None));

        let received_clone = received.clone();
        let slot_thread_clone = slot_thread.clone();
        signal.connect_with_type(
            move |&value| {
                received_clone.lock().push(value);
                *slot_thread_clone.lock() = Some(std::thread::current().id());
            },
            ConnectionType::Direct,
        );

        // Emit from a different thread
        let signal_clone = signal.clone();
        let handle = std::thread::spawn(move || {
            signal_clone.emit(100);
            std::thread::current().id()
        });

        let emitting_thread_id = handle.join().unwrap();

        assert_eq!(*received.lock(), vec![100]);
        // With Direct, slot runs on the emitting thread
        assert_eq!(*slot_thread.lock(), Some(emitting_thread_id));
    }

    #[test]
    fn test_emit_from_multiple_threads() {
        // Multiple threads can emit to the same signal concurrently
        let signal = Arc::new(Signal::<i32>::new());
        let received = Arc::new(Mutex::new(Vec::new()));

        let received_clone = received.clone();
        signal.connect_with_type(
            move |&value| {
                received_clone.lock().push(value);
            },
            ConnectionType::Direct,
        );

        let mut handles = vec![];
        for i in 0..10 {
            let signal_clone = signal.clone();
            handles.push(std::thread::spawn(move || {
                signal_clone.emit(i);
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let values = received.lock();
        assert_eq!(values.len(), 10);
        // All values should be present (order may vary)
        for i in 0..10 {
            assert!(values.contains(&i), "Missing value {}", i);
        }
    }

    #[test]
    fn test_auto_connection_same_thread() {
        // Auto connection on same thread should be direct
        let signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));
        let slot_thread = Arc::new(Mutex::new(None));

        let received_clone = received.clone();
        let slot_thread_clone = slot_thread.clone();
        signal.connect(move |&value| {
            received_clone.lock().push(value);
            *slot_thread_clone.lock() = Some(std::thread::current().id());
        });

        signal.emit(42);

        assert_eq!(*received.lock(), vec![42]);
        assert_eq!(*slot_thread.lock(), Some(std::thread::current().id()));
    }

    #[test]
    fn test_queued_connection_fallback() {
        // Without event loop, queued connections fall back to immediate execution
        let signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        let received_clone = received.clone();
        signal.connect_with_type(
            move |&value| {
                received_clone.lock().push(value);
            },
            ConnectionType::Queued,
        );

        signal.emit(42);

        // Should be executed immediately as fallback
        assert_eq!(*received.lock(), vec![42]);
    }

    #[test]
    fn test_emit_queued_method() {
        // emit_queued should queue all slots regardless of connection type
        let signal = Signal::<i32>::new();
        let direct_count = Arc::new(Mutex::new(0));
        let auto_count = Arc::new(Mutex::new(0));

        let direct_clone = direct_count.clone();
        signal.connect_with_type(
            move |_| {
                *direct_clone.lock() += 1;
            },
            ConnectionType::Direct,
        );

        let auto_clone = auto_count.clone();
        signal.connect(move |_| {
            *auto_clone.lock() += 1;
        });

        let queued_count = signal.emit_queued(42);
        assert_eq!(queued_count, 2);

        // Without event loop, both should be executed via fallback
        assert_eq!(*direct_count.lock(), 1);
        assert_eq!(*auto_count.lock(), 1);
    }

    #[test]
    fn test_signal_shared_across_threads() {
        // Signal can be safely shared and used from multiple threads
        let signal = Arc::new(Signal::<String>::new());
        let received = Arc::new(Mutex::new(Vec::new()));

        // Connect from main thread
        let received_clone = received.clone();
        signal.connect_with_type(
            move |s| {
                received_clone.lock().push(s.clone());
            },
            ConnectionType::Direct,
        );

        // Emit from multiple threads concurrently
        let mut handles = vec![];
        for i in 0..5 {
            let signal_clone = signal.clone();
            handles.push(std::thread::spawn(move || {
                signal_clone.emit(format!("thread-{}", i));
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let values = received.lock();
        assert_eq!(values.len(), 5);
    }

    #[test]
    fn test_connect_from_different_thread() {
        // Connections can be made from any thread
        let signal = Arc::new(Signal::<i32>::new());
        let received = Arc::new(Mutex::new(Vec::new()));

        let signal_clone = signal.clone();
        let received_clone = received.clone();
        let connect_handle = std::thread::spawn(move || {
            signal_clone.connect_with_type(
                move |&value| {
                    received_clone.lock().push(value);
                },
                ConnectionType::Direct,
            )
        });

        let _conn_id = connect_handle.join().unwrap();

        // Emit from main thread
        signal.emit(42);

        assert_eq!(*received.lock(), vec![42]);
    }

    #[test]
    fn test_disconnect_from_different_thread() {
        // Disconnection can happen from any thread
        let signal = Arc::new(Signal::<i32>::new());
        let received = Arc::new(Mutex::new(Vec::new()));

        let received_clone = received.clone();
        let conn_id = signal.connect_with_type(
            move |&value| {
                received_clone.lock().push(value);
            },
            ConnectionType::Direct,
        );

        signal.emit(1);

        // Disconnect from another thread
        let signal_clone = signal.clone();
        let disconnect_handle = std::thread::spawn(move || {
            signal_clone.disconnect(conn_id)
        });

        let disconnected = disconnect_handle.join().unwrap();
        assert!(disconnected);

        signal.emit(2);

        assert_eq!(*received.lock(), vec![1]); // Only first emit received
    }

    #[test]
    fn test_blocking_queued_fallback() {
        // BlockingQueued without event loop should execute immediately
        let signal = Signal::<i32>::new();
        let received = Arc::new(Mutex::new(Vec::new()));

        let received_clone = received.clone();
        signal.connect_with_type(
            move |&value| {
                received_clone.lock().push(value);
            },
            ConnectionType::BlockingQueued,
        );

        // Should not deadlock since there's no event loop
        signal.emit(42);

        assert_eq!(*received.lock(), vec![42]);
    }

    #[test]
    fn test_mixed_connection_types() {
        // Mix of Direct, Auto, and Queued should all work
        let signal = Signal::<i32>::new();
        let direct_received = Arc::new(Mutex::new(Vec::new()));
        let auto_received = Arc::new(Mutex::new(Vec::new()));
        let queued_received = Arc::new(Mutex::new(Vec::new()));

        let direct_clone = direct_received.clone();
        signal.connect_with_type(
            move |&value| {
                direct_clone.lock().push(("direct", value));
            },
            ConnectionType::Direct,
        );

        let auto_clone = auto_received.clone();
        signal.connect(move |&value| {
            auto_clone.lock().push(("auto", value));
        });

        let queued_clone = queued_received.clone();
        signal.connect_with_type(
            move |&value| {
                queued_clone.lock().push(("queued", value));
            },
            ConnectionType::Queued,
        );

        signal.emit(42);

        assert_eq!(*direct_received.lock(), vec![("direct", 42)]);
        assert_eq!(*auto_received.lock(), vec![("auto", 42)]);
        assert_eq!(*queued_received.lock(), vec![("queued", 42)]);
    }

    #[test]
    fn test_signal_stress() {
        // Stress test: many threads, many emissions
        let signal = Arc::new(Signal::<usize>::new());
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let counter_clone = counter.clone();
        signal.connect_with_type(
            move |_| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            },
            ConnectionType::Direct,
        );

        let num_threads = 10;
        let emissions_per_thread = 100;

        let mut handles = vec![];
        for _ in 0..num_threads {
            let signal_clone = signal.clone();
            handles.push(std::thread::spawn(move || {
                for i in 0..emissions_per_thread {
                    signal_clone.emit(i);
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(
            counter.load(Ordering::SeqCst),
            num_threads * emissions_per_thread
        );
    }
}
