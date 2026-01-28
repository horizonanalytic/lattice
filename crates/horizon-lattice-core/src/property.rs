//! Property system for Horizon Lattice.
//!
//! This module provides reactive properties with change notification and
//! computed property bindings. Properties are the data backbone of the
//! signal/slot system - when a property changes, it can emit a signal
//! to notify interested parties.
//!
//! # Property Types
//!
//! - **Property<T>**: A reactive property with optional change notification
//! - **Binding<T>**: A computed property that derives its value from others
//! - **PropertyMeta**: Runtime metadata for property introspection
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice_core::property::Property;
//! use horizon_lattice_core::signal::Signal;
//!
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
//!     fn set_value(&self, new_value: i32) {
//!         if self.value.set(new_value) {
//!             self.value_changed.emit(new_value);
//!         }
//!     }
//! }
//! ```

use std::any::TypeId;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::RwLock;

/// A reactive property that tracks changes.
///
/// `Property<T>` wraps a value and provides change detection. When `set()` is
/// called, it compares the new value with the current one and returns whether
/// the value actually changed. This enables efficient change notification.
///
/// # Thread Safety
///
/// `Property<T>` uses interior mutability with `RwLock` and is `Send + Sync`.
///
/// # Example
///
/// ```ignore
/// let prop = Property::new(42);
/// assert_eq!(prop.get(), 42);
///
/// // Setting same value returns false (no change)
/// assert!(!prop.set(42));
///
/// // Setting different value returns true (changed)
/// assert!(prop.set(100));
/// assert_eq!(prop.get(), 100);
/// ```
pub struct Property<T> {
    value: RwLock<T>,
}

impl<T: Clone> Property<T> {
    /// Create a new property with an initial value.
    pub fn new(value: T) -> Self {
        Self {
            value: RwLock::new(value),
        }
    }

    /// Get the current value.
    ///
    /// This clones the value. For large types, consider using `with()` instead.
    pub fn get(&self) -> T {
        self.value.read().clone()
    }

    /// Access the value through a closure without cloning.
    ///
    /// This is more efficient for large types when you don't need ownership.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        f(&self.value.read())
    }

    /// Set the value without change notification.
    ///
    /// This is useful during initialization or batch updates where you
    /// want to defer notifications.
    pub fn set_silent(&self, value: T) {
        *self.value.write() = value;
    }
}

impl<T: Clone + PartialEq> Property<T> {
    /// Set the value, returning `true` if the value changed.
    ///
    /// This compares the new value with the current one using `PartialEq`.
    /// If they are equal, the value is not updated and `false` is returned.
    ///
    /// The caller should emit the associated notification signal when this
    /// returns `true`.
    pub fn set(&self, value: T) -> bool {
        let mut current = self.value.write();
        if *current != value {
            *current = value;
            true
        } else {
            false
        }
    }

    /// Set the value, returning the old value if it changed.
    ///
    /// This is useful when you need to know the previous value for
    /// change notifications.
    pub fn replace(&self, value: T) -> Option<T> {
        let mut current = self.value.write();
        if *current != value {
            let old = std::mem::replace(&mut *current, value);
            Some(old)
        } else {
            None
        }
    }
}

impl<T: Clone> Clone for Property<T> {
    fn clone(&self) -> Self {
        Self::new(self.get())
    }
}

impl<T: Clone + Default> Default for Property<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Clone + fmt::Debug> fmt::Debug for Property<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Property")
            .field("value", &self.get())
            .finish()
    }
}

// Property is Send + Sync when T is Send + Sync
unsafe impl<T: Send> Send for Property<T> {}
unsafe impl<T: Send + Sync> Sync for Property<T> {}

/// A read-only view of a property.
///
/// This provides read access without the ability to modify the underlying value.
/// Useful for exposing properties publicly while keeping the setter private.
pub struct ReadOnlyProperty<'a, T> {
    inner: &'a Property<T>,
}

impl<'a, T: Clone> ReadOnlyProperty<'a, T> {
    /// Create a read-only view of a property.
    pub fn new(property: &'a Property<T>) -> Self {
        Self { inner: property }
    }

    /// Get the current value.
    pub fn get(&self) -> T {
        self.inner.get()
    }

    /// Access the value through a closure.
    pub fn with<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        self.inner.with(f)
    }
}

/// A computed property that derives its value from a computation.
///
/// `Binding<T>` caches its computed value and only recalculates when
/// explicitly invalidated. This is useful for derived properties that
/// depend on multiple source values.
///
/// # Example
///
/// ```ignore
/// let first_name = Property::new("John".to_string());
/// let last_name = Property::new("Doe".to_string());
///
/// // Note: In real usage, the closure would capture shared references
/// let full_name = Binding::new(|| {
///     format!("{} {}", first_name.get(), last_name.get())
/// });
///
/// assert_eq!(full_name.get(), "John Doe");
///
/// // After changing a dependency, invalidate to recalculate
/// first_name.set("Jane".to_string());
/// full_name.invalidate();
/// assert_eq!(full_name.get(), "Jane Doe");
/// ```
pub struct Binding<T> {
    /// The computation function.
    compute: Box<dyn Fn() -> T + Send + Sync>,
    /// Cached value.
    cached: RwLock<Option<T>>,
    /// Whether the cache needs refreshing.
    dirty: AtomicBool,
}

impl<T: Clone + Send + Sync + 'static> Binding<T> {
    /// Create a new binding with a computation function.
    ///
    /// The function will be called lazily when `get()` is first called,
    /// and again after each `invalidate()` call.
    pub fn new<F>(compute: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        Self {
            compute: Box::new(compute),
            cached: RwLock::new(None),
            dirty: AtomicBool::new(true),
        }
    }

    /// Get the current value, computing it if necessary.
    ///
    /// If the binding is dirty or has never been computed, the computation
    /// function is called and the result is cached.
    ///
    /// # Panics
    ///
    /// Panics if the internal `RwLock` is poisoned (i.e., a thread panicked
    /// while holding the lock).
    pub fn get(&self) -> T {
        if self.dirty.load(Ordering::Acquire) || self.cached.read().is_none() {
            let value = (self.compute)();
            *self.cached.write() = Some(value.clone());
            self.dirty.store(false, Ordering::Release);
            value
        } else {
            self.cached.read().clone().unwrap()
        }
    }

    /// Mark the binding as dirty, causing recalculation on next `get()`.
    ///
    /// Call this when any dependency of the binding changes.
    pub fn invalidate(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    /// Check if the binding needs recalculation.
    pub fn is_dirty(&self) -> bool {
        self.dirty.load(Ordering::Acquire)
    }

    /// Force immediate recalculation and return the new value.
    pub fn refresh(&self) -> T {
        self.invalidate();
        self.get()
    }
}

impl<T: Clone + fmt::Debug + Send + Sync + 'static> fmt::Debug for Binding<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Binding")
            .field("dirty", &self.is_dirty())
            .field("cached", &*self.cached.read())
            .finish()
    }
}

/// Metadata for a property, used for runtime introspection.
///
/// This is primarily used by the meta-object system and procedural macros
/// to provide information about properties at runtime.
#[derive(Clone)]
pub struct PropertyMeta {
    /// The property name.
    pub name: &'static str,
    /// The type name (for debugging/serialization).
    pub type_name: &'static str,
    /// The TypeId for runtime type checking.
    pub type_id: TypeId,
    /// Whether this property is read-only.
    pub read_only: bool,
    /// The name of the signal emitted when this property changes (if any).
    pub notify_signal: Option<&'static str>,
}

impl PropertyMeta {
    /// Create metadata for a property.
    pub const fn new<T: 'static>(
        name: &'static str,
        type_name: &'static str,
        read_only: bool,
        notify_signal: Option<&'static str>,
    ) -> Self {
        Self {
            name,
            type_name,
            type_id: TypeId::of::<T>(),
            read_only,
            notify_signal,
        }
    }
}

impl fmt::Debug for PropertyMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyMeta")
            .field("name", &self.name)
            .field("type_name", &self.type_name)
            .field("read_only", &self.read_only)
            .field("notify_signal", &self.notify_signal)
            .finish()
    }
}

/// Error types for property operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyError {
    /// The property was not found.
    NotFound {
        /// The name of the property that was not found.
        name: String,
    },
    /// The property type did not match.
    TypeMismatch {
        /// The expected type name.
        expected: &'static str,
        /// The actual type name.
        got: &'static str,
    },
    /// The property is read-only and cannot be modified.
    ReadOnly {
        /// The name of the read-only property.
        name: String,
    },
}

impl fmt::Display for PropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { name } => write!(f, "Property '{}' not found", name),
            Self::TypeMismatch { expected, got } => {
                write!(f, "Property type mismatch: expected {}, got {}", expected, got)
            }
            Self::ReadOnly { name } => write!(f, "Property '{}' is read-only", name),
        }
    }
}

impl std::error::Error for PropertyError {}

/// A helper trait for creating properties with common patterns.
pub trait IntoProperty<T> {
    /// Convert this value into a Property.
    fn into_property(self) -> Property<T>;
}

impl<T: Clone> IntoProperty<T> for T {
    fn into_property(self) -> Property<T> {
        Property::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_property_basic() {
        let prop = Property::new(42);
        assert_eq!(prop.get(), 42);
    }

    #[test]
    fn test_property_set_detects_change() {
        let prop = Property::new(10);

        // Same value - no change
        assert!(!prop.set(10));
        assert_eq!(prop.get(), 10);

        // Different value - changed
        assert!(prop.set(20));
        assert_eq!(prop.get(), 20);
    }

    #[test]
    fn test_property_set_silent() {
        let prop = Property::new(100);
        prop.set_silent(200);
        assert_eq!(prop.get(), 200);
    }

    #[test]
    fn test_property_replace() {
        let prop = Property::new("hello".to_string());

        // Same value - no change, returns None
        let old = prop.replace("hello".to_string());
        assert!(old.is_none());

        // Different value - returns old value
        let old = prop.replace("world".to_string());
        assert_eq!(old, Some("hello".to_string()));
        assert_eq!(prop.get(), "world");
    }

    #[test]
    fn test_property_with_closure() {
        let prop = Property::new(vec![1, 2, 3]);

        // Use with() to avoid cloning
        let sum: i32 = prop.with(|v| v.iter().sum());
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_property_thread_safe() {
        let prop = Arc::new(Property::new(0));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let prop = prop.clone();
                std::thread::spawn(move || {
                    for i in 0..100 {
                        prop.set_silent(i);
                        let _ = prop.get();
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn test_read_only_property() {
        let prop = Property::new(42);
        let ro = ReadOnlyProperty::new(&prop);

        assert_eq!(ro.get(), 42);

        // Modify through original
        prop.set_silent(100);
        assert_eq!(ro.get(), 100);
    }

    #[test]
    fn test_binding_basic() {
        let counter = Arc::new(Property::new(5));

        let counter_clone = counter.clone();
        let doubled = Binding::new(move || counter_clone.get() * 2);

        assert_eq!(doubled.get(), 10);
        assert!(!doubled.is_dirty());

        // Change source and invalidate
        counter.set_silent(10);
        doubled.invalidate();
        assert!(doubled.is_dirty());

        assert_eq!(doubled.get(), 20);
        assert!(!doubled.is_dirty());
    }

    #[test]
    fn test_binding_lazy_evaluation() {
        use std::sync::atomic::AtomicUsize;

        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = call_count.clone();

        let binding = Binding::new(move || {
            call_count_clone.fetch_add(1, Ordering::SeqCst);
            42
        });

        // Not computed yet
        assert_eq!(call_count.load(Ordering::SeqCst), 0);

        // First access computes
        assert_eq!(binding.get(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Second access uses cache
        assert_eq!(binding.get(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // After invalidate, recomputes
        binding.invalidate();
        assert_eq!(binding.get(), 42);
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_binding_refresh() {
        let value = Arc::new(Property::new(1));
        let value_clone = value.clone();

        let binding = Binding::new(move || value_clone.get() + 10);

        assert_eq!(binding.get(), 11);

        value.set_silent(5);
        assert_eq!(binding.refresh(), 15);
    }

    #[test]
    fn test_property_meta() {
        let meta = PropertyMeta::new::<i32>("count", "i32", false, Some("count_changed"));

        assert_eq!(meta.name, "count");
        assert_eq!(meta.type_name, "i32");
        assert!(!meta.read_only);
        assert_eq!(meta.notify_signal, Some("count_changed"));
    }

    #[test]
    fn test_property_default() {
        let prop: Property<i32> = Property::default();
        assert_eq!(prop.get(), 0);

        let prop: Property<String> = Property::default();
        assert_eq!(prop.get(), "");
    }

    #[test]
    fn test_into_property() {
        let prop = 42.into_property();
        assert_eq!(prop.get(), 42);

        let prop = "hello".to_string().into_property();
        assert_eq!(prop.get(), "hello");
    }
}
