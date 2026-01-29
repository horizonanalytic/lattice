//! Queued invocation registry for cross-thread signal delivery.
//!
//! This module provides a global registry for storing deferred slot invocations
//! that need to be executed on a different thread (typically the main/UI thread).
//!
//! # How It Works
//!
//! 1. When a signal is emitted with a `Queued` or `Auto` connection type and the
//!    slot is on a different thread, the slot invocation is wrapped in a closure
//!    and registered here.
//!
//! 2. A `QueuedSignal` event is posted to the event loop with the invocation ID.
//!
//! 3. When the event loop processes the event, it retrieves and executes the
//!    invocation from this registry.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use parking_lot::{Condvar, Mutex};

/// Global invocation counter for unique IDs.
static NEXT_INVOCATION_ID: AtomicU64 = AtomicU64::new(1);

/// Global invocation registry.
static INVOCATION_REGISTRY: OnceLock<InvocationRegistry> = OnceLock::new();

/// A type-erased queued invocation that can be executed later.
///
/// This wraps a closure that captures the slot and its arguments,
/// allowing deferred execution on the target thread.
pub struct QueuedInvocation {
    /// The actual invocation closure.
    invoke: Box<dyn FnOnce() + Send>,
    /// Optional completion notifier for blocking connections.
    completion: Option<CompletionHandle>,
}

impl QueuedInvocation {
    /// Create a new queued invocation.
    pub fn new<F>(invoke: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            invoke: Box::new(invoke),
            completion: None,
        }
    }

    /// Create a new queued invocation with a completion handle for blocking.
    pub fn with_completion<F>(invoke: F, completion: CompletionHandle) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Self {
            invoke: Box::new(invoke),
            completion: Some(completion),
        }
    }

    /// Execute the invocation.
    pub fn execute(self) {
        (self.invoke)();
        // Signal completion if this was a blocking invocation.
        if let Some(completion) = self.completion {
            completion.signal_done();
        }
    }
}

/// A handle for signaling completion of a blocking invocation.
///
/// The sender side signals when the slot has finished executing,
/// allowing the emitting thread to unblock.
pub struct CompletionHandle {
    inner: std::sync::Arc<CompletionState>,
}

impl CompletionHandle {
    /// Signal that the invocation is complete.
    fn signal_done(self) {
        let mut done = self.inner.done.lock();
        *done = true;
        self.inner.condvar.notify_all();
    }
}

/// A waiter for blocking on invocation completion.
pub struct CompletionWaiter {
    inner: std::sync::Arc<CompletionState>,
}

impl CompletionWaiter {
    /// Wait for the invocation to complete.
    ///
    /// This blocks the current thread until the slot finishes executing.
    ///
    /// # Warning
    ///
    /// Calling this from the main/UI thread when the slot is supposed to run
    /// on the main thread will cause a deadlock. Use with caution.
    pub fn wait(self) {
        let mut done = self.inner.done.lock();
        while !*done {
            self.inner.condvar.wait(&mut done);
        }
    }

    /// Wait for the invocation to complete with a timeout.
    ///
    /// Returns `true` if the invocation completed, `false` if the timeout elapsed.
    pub fn wait_timeout(self, timeout: std::time::Duration) -> bool {
        let mut done = self.inner.done.lock();
        if *done {
            return true;
        }
        let result = self.inner.condvar.wait_for(&mut done, timeout);
        *done || !result.timed_out()
    }
}

struct CompletionState {
    done: Mutex<bool>,
    condvar: Condvar,
}

/// Create a completion handle/waiter pair for blocking invocations.
pub fn completion_pair() -> (CompletionHandle, CompletionWaiter) {
    let state = std::sync::Arc::new(CompletionState {
        done: Mutex::new(false),
        condvar: Condvar::new(),
    });

    (
        CompletionHandle {
            inner: state.clone(),
        },
        CompletionWaiter { inner: state },
    )
}

/// Registry for queued signal invocations.
///
/// This is a global registry that stores pending slot invocations waiting
/// to be executed on their target thread.
pub struct InvocationRegistry {
    invocations: Mutex<HashMap<u64, QueuedInvocation>>,
}

impl InvocationRegistry {
    /// Create a new registry.
    fn new() -> Self {
        Self {
            invocations: Mutex::new(HashMap::new()),
        }
    }

    /// Register an invocation and return its unique ID.
    pub fn register(&self, invocation: QueuedInvocation) -> u64 {
        let id = NEXT_INVOCATION_ID.fetch_add(1, Ordering::SeqCst);
        self.invocations.lock().insert(id, invocation);
        id
    }

    /// Take an invocation by ID, removing it from the registry.
    ///
    /// Returns `None` if no invocation with that ID exists.
    pub fn take(&self, id: u64) -> Option<QueuedInvocation> {
        self.invocations.lock().remove(&id)
    }

    /// Get the number of pending invocations.
    pub fn pending_count(&self) -> usize {
        self.invocations.lock().len()
    }

    /// Clear all pending invocations.
    ///
    /// This is primarily for testing or cleanup purposes.
    pub fn clear(&self) {
        self.invocations.lock().clear();
    }
}

/// Get the global invocation registry.
pub fn invocation_registry() -> &'static InvocationRegistry {
    INVOCATION_REGISTRY.get_or_init(InvocationRegistry::new)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;

    #[test]
    fn test_register_and_execute() {
        let registry = InvocationRegistry::new();
        let executed = Arc::new(AtomicBool::new(false));

        let executed_clone = executed.clone();
        let invocation = QueuedInvocation::new(move || {
            executed_clone.store(true, Ordering::SeqCst);
        });

        let id = registry.register(invocation);
        assert_eq!(registry.pending_count(), 1);

        let invocation = registry.take(id).expect("invocation should exist");
        assert_eq!(registry.pending_count(), 0);

        invocation.execute();
        assert!(executed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_take_nonexistent() {
        let registry = InvocationRegistry::new();
        assert!(registry.take(999999).is_none());
    }

    #[test]
    fn test_completion_pair() {
        let (handle, waiter) = completion_pair();

        let thread = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            handle.signal_done();
        });

        waiter.wait();
        thread.join().unwrap();
    }

    #[test]
    fn test_completion_with_invocation() {
        let registry = InvocationRegistry::new();
        let executed = Arc::new(AtomicBool::new(false));

        let (handle, waiter) = completion_pair();

        let executed_clone = executed.clone();
        let invocation = QueuedInvocation::with_completion(
            move || {
                executed_clone.store(true, Ordering::SeqCst);
            },
            handle,
        );

        let id = registry.register(invocation);

        // Simulate event loop processing in another thread
        let thread = std::thread::spawn(move || {
            let inv = registry.take(id).unwrap();
            inv.execute();
        });

        // Wait for completion
        waiter.wait();
        thread.join().unwrap();

        assert!(executed.load(Ordering::SeqCst));
    }

    #[test]
    fn test_completion_timeout() {
        let (_handle, waiter) = completion_pair();

        // Should timeout since we never signal
        let completed = waiter.wait_timeout(std::time::Duration::from_millis(10));
        assert!(!completed);
    }
}
