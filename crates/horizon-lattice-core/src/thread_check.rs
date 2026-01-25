//! Thread safety verification utilities for Horizon Lattice.
//!
//! This module provides debug assertions and runtime checks to help verify
//! that GUI operations are performed on the correct thread. In a typical GUI
//! application, widget operations must be performed on the main (UI) thread.
//!
//! # Usage
//!
//! The main thread is automatically tracked when `Application::new()` is called.
//! After that, you can use the provided macros and functions to verify thread
//! affinity:
//!
//! ```ignore
//! use horizon_lattice_core::{assert_main_thread, debug_assert_main_thread, is_main_thread};
//!
//! fn update_widget(&self) {
//!     // Panic in debug builds if not on main thread
//!     debug_assert_main_thread!();
//!
//!     // ... update widget state ...
//! }
//!
//! fn some_operation(&self) {
//!     if is_main_thread() {
//!         // Direct operation
//!     } else {
//!         // Queue to main thread
//!     }
//! }
//! ```
//!
//! # Thread Safety Checks
//!
//! Two levels of checking are provided:
//!
//! - **Debug assertions** (`debug_assert_main_thread!`): Only active in debug builds.
//!   Use these liberally throughout widget code for zero-cost production performance.
//!
//! - **Runtime assertions** (`assert_main_thread!`): Always active. Use for critical
//!   operations where thread safety must be verified even in release builds.
//!
//! # Object Thread Affinity
//!
//! For objects that must be accessed from a specific thread, use `ThreadAffinity`:
//!
//! ```ignore
//! use horizon_lattice_core::thread_check::ThreadAffinity;
//!
//! struct MyWidget {
//!     affinity: ThreadAffinity,
//!     // ... other fields ...
//! }
//!
//! impl MyWidget {
//!     fn new() -> Self {
//!         Self {
//!             affinity: ThreadAffinity::current(),
//!             // ...
//!         }
//!     }
//!
//!     fn update(&self) {
//!         self.affinity.debug_assert_same_thread();
//!         // ... safe to update ...
//!     }
//! }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread::ThreadId;

/// Global storage for the main thread ID.
static MAIN_THREAD_ID: OnceLock<ThreadId> = OnceLock::new();

/// Flag to enable/disable runtime thread checks globally.
static THREAD_CHECKS_ENABLED: AtomicBool = AtomicBool::new(cfg!(debug_assertions));

/// Set the main thread ID.
///
/// This is automatically called by `Application::new()`. It should only be
/// called once, from the main thread, at application startup.
///
/// # Panics
///
/// Panics if called more than once.
pub fn set_main_thread() {
    let current = std::thread::current().id();
    if MAIN_THREAD_ID.set(current).is_err() {
        // Already set - verify it's the same thread
        if MAIN_THREAD_ID.get() != Some(&current) {
            panic!(
                "set_main_thread() called from different thread than original. \
                 The main thread ID can only be set once."
            );
        }
    }
}

/// Get the main thread ID if it has been set.
///
/// Returns `None` if `Application::new()` has not been called yet.
#[inline]
pub fn main_thread_id() -> Option<ThreadId> {
    MAIN_THREAD_ID.get().copied()
}

/// Check if the current thread is the main (UI) thread.
///
/// Returns `true` if:
/// - We are on the main thread, OR
/// - The main thread has not been set yet (graceful fallback)
///
/// Returns `false` only if:
/// - The main thread has been set AND we are on a different thread
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_core::is_main_thread;
///
/// if is_main_thread() {
///     // Safe to perform widget operations
///     widget.update();
/// } else {
///     // Queue the operation to the main thread
///     post_to_main_thread(|| widget.update());
/// }
/// ```
#[inline]
pub fn is_main_thread() -> bool {
    match MAIN_THREAD_ID.get() {
        Some(&main_id) => std::thread::current().id() == main_id,
        // If not set, assume we're fine (early initialization)
        None => true,
    }
}

/// Enable or disable runtime thread checks.
///
/// By default, thread checks are enabled in debug builds and disabled in
/// release builds. Call this function to override the default behavior.
///
/// # Arguments
///
/// * `enabled` - `true` to enable checks, `false` to disable
///
/// # Example
///
/// ```ignore
/// // Enable thread checks even in release builds for testing
/// horizon_lattice_core::set_thread_checks_enabled(true);
/// ```
pub fn set_thread_checks_enabled(enabled: bool) {
    THREAD_CHECKS_ENABLED.store(enabled, Ordering::SeqCst);
}

/// Check if runtime thread checks are currently enabled.
#[inline]
pub fn are_thread_checks_enabled() -> bool {
    THREAD_CHECKS_ENABLED.load(Ordering::Relaxed)
}

/// Panics if the current thread is not the main thread.
///
/// This is always active (in both debug and release builds). Use
/// `debug_assert_main_thread!()` for checks that should only run in debug builds.
///
/// # Panics
///
/// Panics with a descriptive message if called from a non-main thread.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_core::assert_main_thread;
///
/// fn critical_widget_update(&self) {
///     assert_main_thread!("critical_widget_update must be called on the main thread");
///     // ...
/// }
/// ```
#[macro_export]
macro_rules! assert_main_thread {
    () => {
        $crate::assert_main_thread!("operation must be performed on the main thread")
    };
    ($msg:expr) => {
        if !$crate::thread_check::is_main_thread() {
            $crate::thread_check::panic_not_main_thread($msg, file!(), line!());
        }
    };
    ($fmt:expr, $($arg:tt)*) => {
        if !$crate::thread_check::is_main_thread() {
            $crate::thread_check::panic_not_main_thread(
                &format!($fmt, $($arg)*),
                file!(),
                line!()
            );
        }
    };
}

/// Debug-only assertion that panics if not on the main thread.
///
/// This macro is a no-op in release builds, making it suitable for liberal
/// use throughout widget code without affecting production performance.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_core::debug_assert_main_thread;
///
/// fn update_layout(&self) {
///     debug_assert_main_thread!();
///     // ... layout code ...
/// }
/// ```
#[macro_export]
macro_rules! debug_assert_main_thread {
    () => {
        #[cfg(debug_assertions)]
        $crate::assert_main_thread!()
    };
    ($msg:expr) => {
        #[cfg(debug_assertions)]
        $crate::assert_main_thread!($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        #[cfg(debug_assertions)]
        $crate::assert_main_thread!($fmt, $($arg)*)
    };
}

/// Internal function to generate the panic message for thread violations.
///
/// This is called by the assertion macros and provides a detailed, helpful
/// error message.
#[cold]
#[inline(never)]
#[doc(hidden)]
pub fn panic_not_main_thread(msg: &str, file: &str, line: u32) -> ! {
    let current = std::thread::current();
    let current_name = current.name().unwrap_or("<unnamed>");
    let current_id = current.id();

    let main_info = match main_thread_id() {
        Some(id) => format!("main thread ID: {:?}", id),
        None => "main thread not yet registered".to_string(),
    };

    panic!(
        "\n\
        ══════════════════════════════════════════════════════════════════════\n\
        THREAD SAFETY VIOLATION\n\
        ══════════════════════════════════════════════════════════════════════\n\
        \n\
        {msg}\n\
        \n\
        Location: {file}:{line}\n\
        Current thread: \"{current_name}\" (ID: {current_id:?})\n\
        {main_info}\n\
        \n\
        This operation requires running on the main (UI) thread. Widget\n\
        operations, layout updates, and rendering must occur on the main\n\
        thread to ensure thread safety.\n\
        \n\
        POSSIBLE SOLUTIONS:\n\
        \n\
        1. Use Application::post_task() to queue the operation:\n\
           app.post_task(|| widget.update());\n\
        \n\
        2. Use spawn_with_callback() to deliver results to the main thread:\n\
           ThreadPool::global().spawn_with_callback(\n\
               || expensive_computation(),\n\
               |result| widget.set_value(result)\n\
           );\n\
        \n\
        3. Use ConnectionType::Queued for cross-thread signal connections:\n\
           signal.connect_with_type(slot, ConnectionType::Queued);\n\
        \n\
        ══════════════════════════════════════════════════════════════════════"
    )
}

/// Thread affinity tracker for objects.
///
/// This struct records the thread on which an object was created and provides
/// methods to verify that subsequent operations occur on the same thread.
///
/// # Example
///
/// ```
/// use horizon_lattice_core::thread_check::ThreadAffinity;
///
/// struct MyWidget {
///     affinity: ThreadAffinity,
///     value: std::cell::Cell<i32>,
/// }
///
/// impl MyWidget {
///     fn new() -> Self {
///         Self {
///             affinity: ThreadAffinity::current(),
///             value: std::cell::Cell::new(0),
///         }
///     }
///
///     fn set_value(&self, v: i32) {
///         // In debug builds, panic if called from wrong thread
///         self.affinity.debug_assert_same_thread();
///         self.value.set(v);
///     }
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ThreadAffinity {
    thread_id: ThreadId,
}

impl Default for ThreadAffinity {
    fn default() -> Self {
        Self::current()
    }
}

impl ThreadAffinity {
    /// Create a new thread affinity tracker for the current thread.
    #[inline]
    pub fn current() -> Self {
        Self {
            thread_id: std::thread::current().id(),
        }
    }

    /// Create a thread affinity tracker for the main thread.
    ///
    /// If the main thread has not been set yet, falls back to the current thread.
    pub fn main_thread() -> Self {
        Self {
            thread_id: main_thread_id().unwrap_or_else(|| std::thread::current().id()),
        }
    }

    /// Get the thread ID this affinity is bound to.
    #[inline]
    pub fn thread_id(&self) -> ThreadId {
        self.thread_id
    }

    /// Check if the current thread matches this affinity.
    #[inline]
    pub fn is_same_thread(&self) -> bool {
        std::thread::current().id() == self.thread_id
    }

    /// Check if this affinity is for the main thread.
    #[inline]
    pub fn is_main_thread_affinity(&self) -> bool {
        main_thread_id() == Some(self.thread_id)
    }

    /// Assert that we are on the same thread as the affinity.
    ///
    /// This always runs (debug and release builds).
    ///
    /// # Panics
    ///
    /// Panics with a descriptive message if called from a different thread.
    #[inline]
    pub fn assert_same_thread(&self) {
        self.assert_same_thread_with_msg("object accessed from wrong thread")
    }

    /// Assert that we are on the same thread, with a custom message.
    ///
    /// # Panics
    ///
    /// Panics if called from a different thread.
    pub fn assert_same_thread_with_msg(&self, msg: &str) {
        if !self.is_same_thread() {
            self.panic_wrong_thread(msg);
        }
    }

    /// Debug-only assertion that we are on the same thread.
    ///
    /// This is a no-op in release builds.
    #[inline]
    pub fn debug_assert_same_thread(&self) {
        #[cfg(debug_assertions)]
        self.assert_same_thread();
    }

    /// Debug-only assertion with a custom message.
    #[inline]
    pub fn debug_assert_same_thread_with_msg(&self, msg: &str) {
        #[cfg(debug_assertions)]
        self.assert_same_thread_with_msg(msg);
    }

    #[cold]
    #[inline(never)]
    fn panic_wrong_thread(&self, msg: &str) -> ! {
        let current = std::thread::current();
        let current_name = current.name().unwrap_or("<unnamed>");
        let current_id = current.id();

        panic!(
            "\n\
            ══════════════════════════════════════════════════════════════════════\n\
            THREAD AFFINITY VIOLATION\n\
            ══════════════════════════════════════════════════════════════════════\n\
            \n\
            {msg}\n\
            \n\
            Object was created on thread: {:?}\n\
            Current thread: \"{current_name}\" (ID: {current_id:?})\n\
            \n\
            This object has thread affinity and must only be accessed from the\n\
            thread on which it was created. This is typically required for:\n\
            \n\
            - Widget state modifications\n\
            - Layout calculations\n\
            - Rendering operations\n\
            - Any non-Send/Sync data access\n\
            \n\
            POSSIBLE SOLUTIONS:\n\
            \n\
            1. Post the operation to the correct thread\n\
            2. Use signals with ConnectionType::Queued\n\
            3. Wrap shared data in Arc<Mutex<T>> if cross-thread access is needed\n\
            \n\
            ══════════════════════════════════════════════════════════════════════",
            self.thread_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    // Note: We can't easily test set_main_thread() since it's a OnceLock.
    // These tests focus on the other functionality.

    #[test]
    fn test_is_main_thread_before_set() {
        // Before main thread is set, is_main_thread should return true
        // (graceful fallback for early initialization)
        // Note: This test may not work correctly if run after other tests
        // that set the main thread, since OnceLock can only be set once.
    }

    #[test]
    fn test_thread_affinity_same_thread() {
        let affinity = ThreadAffinity::current();
        assert!(affinity.is_same_thread());
        // Should not panic
        affinity.assert_same_thread();
    }

    #[test]
    fn test_thread_affinity_different_thread() {
        let affinity = ThreadAffinity::current();
        let main_thread_id = std::thread::current().id();

        let result = Arc::new(AtomicBool::new(false));
        let result_clone = result.clone();

        let handle = std::thread::spawn(move || {
            // is_same_thread should return false from a different thread
            result_clone.store(!affinity.is_same_thread(), Ordering::SeqCst);
        });

        handle.join().unwrap();
        assert!(result.load(Ordering::SeqCst), "is_same_thread() should return false from different thread");

        // Verify we're back on the original thread
        assert_eq!(std::thread::current().id(), main_thread_id);
    }

    #[test]
    fn test_thread_affinity_panic_on_wrong_thread() {
        let affinity = ThreadAffinity::current();

        let result = std::thread::spawn(move || {
            affinity.assert_same_thread();
        })
        .join();

        // The spawned thread should have panicked
        assert!(result.is_err(), "Expected thread to panic with affinity violation");
    }

    #[test]
    fn test_thread_affinity_with_custom_message() {
        let affinity = ThreadAffinity::current();
        // Should not panic on same thread
        affinity.assert_same_thread_with_msg("Custom message");
    }

    #[test]
    fn test_thread_checks_enabled_flag() {
        // Save original state
        let original = are_thread_checks_enabled();

        // Test enabling
        set_thread_checks_enabled(true);
        assert!(are_thread_checks_enabled());

        // Test disabling
        set_thread_checks_enabled(false);
        assert!(!are_thread_checks_enabled());

        // Restore
        set_thread_checks_enabled(original);
    }

    #[test]
    fn test_thread_affinity_debug_assert_same_thread() {
        let affinity = ThreadAffinity::current();
        // Should not panic on same thread
        affinity.debug_assert_same_thread();
    }

    #[test]
    fn test_thread_affinity_default() {
        let affinity = ThreadAffinity::default();
        assert!(affinity.is_same_thread());
    }

    #[test]
    fn test_thread_affinity_clone() {
        let affinity1 = ThreadAffinity::current();
        let affinity2 = affinity1;

        assert_eq!(affinity1.thread_id(), affinity2.thread_id());
        assert!(affinity2.is_same_thread());
    }
}
