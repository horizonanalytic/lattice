//! Object model for Horizon Lattice.
//!
//! Provides the base object system with:
//! - Unique object identifiers via arena-based storage
//! - Parent-child ownership relationships with automatic drop cascade
//! - Object naming and lookup
//! - Dynamic property storage
//!
//! This is the Rust equivalent of Qt's QObject system.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;

use parking_lot::{Mutex, RwLock};
use slotmap::{new_key_type, SlotMap};

new_key_type! {
    /// A unique identifier for an object in the registry.
    ///
    /// ObjectIds are stable handles that remain valid even as the object tree changes.
    /// They become invalid when the object is destroyed.
    pub struct ObjectId;
}

/// Errors that can occur during object operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectError {
    /// The object ID is invalid or has been destroyed.
    InvalidObjectId,
    /// Attempted to set an object as its own parent/ancestor.
    CircularParentage,
    /// The property was not found.
    PropertyNotFound,
    /// The property type did not match.
    PropertyTypeMismatch {
        expected: &'static str,
        got: &'static str,
    },
    /// The property is read-only.
    PropertyReadOnly,
    /// The object registry is not initialized.
    RegistryNotInitialized,
}

impl fmt::Display for ObjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidObjectId => write!(f, "Invalid or destroyed object ID"),
            Self::CircularParentage => {
                write!(f, "Cannot set an object as its own parent or ancestor")
            }
            Self::PropertyNotFound => write!(f, "Property not found"),
            Self::PropertyTypeMismatch { expected, got } => {
                write!(f, "Property type mismatch: expected {expected}, got {got}")
            }
            Self::PropertyReadOnly => write!(f, "Property is read-only"),
            Self::RegistryNotInitialized => write!(f, "Object registry not initialized"),
        }
    }
}

impl std::error::Error for ObjectError {}

/// Result type for object operations.
pub type ObjectResult<T> = std::result::Result<T, ObjectError>;

/// Internal data stored in the registry for each object.
struct ObjectData {
    /// Human-readable name for debugging and lookup.
    name: String,
    /// The type ID of the concrete Object implementation.
    type_id: TypeId,
    /// The type name for debugging.
    type_name: &'static str,
    /// Parent object (if any).
    parent: Option<ObjectId>,
    /// Child objects (owned).
    children: Vec<ObjectId>,
    /// Dynamic properties (type-erased).
    dynamic_properties: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl ObjectData {
    fn new(type_id: TypeId, type_name: &'static str) -> Self {
        Self {
            name: String::new(),
            type_id,
            type_name,
            parent: None,
            children: Vec::new(),
            dynamic_properties: HashMap::new(),
        }
    }
}

/// The central registry that manages all objects and their relationships.
///
/// Uses arena-based storage via SlotMap for stable object IDs and efficient
/// parent-child relationship management.
pub struct ObjectRegistry {
    objects: SlotMap<ObjectId, ObjectData>,
}

impl ObjectRegistry {
    /// Create a new empty object registry.
    pub fn new() -> Self {
        Self {
            objects: SlotMap::with_key(),
        }
    }

    /// Register a new object and return its ID.
    pub fn register<T: Object + 'static>(&mut self) -> ObjectId {
        let data = ObjectData::new(TypeId::of::<T>(), std::any::type_name::<T>());
        let id = self.objects.insert(data);
        tracing::trace!(target: "horizon_lattice_core::object", ?id, type_name = std::any::type_name::<T>(), "registered object");
        id
    }

    /// Remove an object and all its children from the registry.
    ///
    /// This implements Qt's cascade delete behavior where destroying a parent
    /// also destroys all children.
    #[tracing::instrument(skip(self), target = "horizon_lattice_core::object", level = "trace")]
    pub fn destroy(&mut self, id: ObjectId) -> ObjectResult<()> {
        // First collect all children to destroy (depth-first).
        let children_to_destroy = self.collect_descendants(id)?;
        tracing::trace!(target: "horizon_lattice_core::object", ?id, descendant_count = children_to_destroy.len(), "destroying object tree");

        // Remove from parent's children list.
        if let Some(data) = self.objects.get(id) {
            if let Some(parent_id) = data.parent {
                if let Some(parent_data) = self.objects.get_mut(parent_id) {
                    parent_data.children.retain(|&child| child != id);
                }
            }
        }

        // Destroy all descendants (children first, then self).
        for child_id in children_to_destroy {
            self.objects.remove(child_id);
        }
        self.objects.remove(id);

        Ok(())
    }

    /// Collect all descendant IDs in depth-first order (children before parents).
    fn collect_descendants(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        let mut result = Vec::new();
        self.collect_descendants_recursive(id, &mut result)?;
        Ok(result)
    }

    fn collect_descendants_recursive(
        &self,
        id: ObjectId,
        result: &mut Vec<ObjectId>,
    ) -> ObjectResult<()> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;
        for &child_id in &data.children {
            self.collect_descendants_recursive(child_id, result)?;
            result.push(child_id);
        }
        Ok(())
    }

    /// Check if an object exists in the registry.
    pub fn contains(&self, id: ObjectId) -> bool {
        self.objects.contains_key(id)
    }

    /// Set the parent of an object.
    ///
    /// This handles removing from the old parent and adding to the new parent.
    /// Passing `None` makes the object a root object.
    pub fn set_parent(&mut self, id: ObjectId, new_parent: Option<ObjectId>) -> ObjectResult<()> {
        // Validate the object exists.
        if !self.objects.contains_key(id) {
            return Err(ObjectError::InvalidObjectId);
        }

        // Validate new parent exists (if specified).
        if let Some(parent_id) = new_parent {
            if !self.objects.contains_key(parent_id) {
                return Err(ObjectError::InvalidObjectId);
            }
            // Check for circular parentage.
            if self.is_ancestor_of(id, parent_id)? {
                return Err(ObjectError::CircularParentage);
            }
        }

        // Remove from old parent.
        let old_parent = self.objects.get(id).and_then(|d| d.parent);
        if let Some(old_parent_id) = old_parent {
            if let Some(parent_data) = self.objects.get_mut(old_parent_id) {
                parent_data.children.retain(|&child| child != id);
            }
        }

        // Update the object's parent reference.
        if let Some(data) = self.objects.get_mut(id) {
            data.parent = new_parent;
        }

        // Add to new parent's children.
        if let Some(parent_id) = new_parent {
            if let Some(parent_data) = self.objects.get_mut(parent_id) {
                parent_data.children.push(id);
            }
        }

        Ok(())
    }

    /// Check if `potential_ancestor` is an ancestor of `id`.
    fn is_ancestor_of(&self, potential_ancestor: ObjectId, id: ObjectId) -> ObjectResult<bool> {
        let mut current = Some(id);
        while let Some(current_id) = current {
            if current_id == potential_ancestor {
                return Ok(true);
            }
            current = self.objects.get(current_id).and_then(|d| d.parent);
        }
        Ok(false)
    }

    /// Get the parent of an object.
    pub fn parent(&self, id: ObjectId) -> ObjectResult<Option<ObjectId>> {
        self.objects
            .get(id)
            .map(|d| d.parent)
            .ok_or(ObjectError::InvalidObjectId)
    }

    /// Get the children of an object.
    pub fn children(&self, id: ObjectId) -> ObjectResult<&[ObjectId]> {
        self.objects
            .get(id)
            .map(|d| d.children.as_slice())
            .ok_or(ObjectError::InvalidObjectId)
    }

    /// Get the object's name.
    pub fn object_name(&self, id: ObjectId) -> ObjectResult<&str> {
        self.objects
            .get(id)
            .map(|d| d.name.as_str())
            .ok_or(ObjectError::InvalidObjectId)
    }

    /// Set the object's name.
    pub fn set_object_name(&mut self, id: ObjectId, name: String) -> ObjectResult<()> {
        self.objects
            .get_mut(id)
            .map(|d| d.name = name)
            .ok_or(ObjectError::InvalidObjectId)
    }

    /// Get the type ID of an object.
    pub fn type_id(&self, id: ObjectId) -> ObjectResult<TypeId> {
        self.objects
            .get(id)
            .map(|d| d.type_id)
            .ok_or(ObjectError::InvalidObjectId)
    }

    /// Get the type name of an object.
    pub fn type_name(&self, id: ObjectId) -> ObjectResult<&'static str> {
        self.objects
            .get(id)
            .map(|d| d.type_name)
            .ok_or(ObjectError::InvalidObjectId)
    }

    /// Find a child by name (direct children only).
    pub fn find_child_by_name(&self, id: ObjectId, name: &str) -> ObjectResult<Option<ObjectId>> {
        let children = self.children(id)?;
        for &child_id in children {
            if let Some(data) = self.objects.get(child_id) {
                if data.name == name {
                    return Ok(Some(child_id));
                }
            }
        }
        Ok(None)
    }

    /// Find a child by name and type (direct children only).
    pub fn find_child<T: 'static>(&self, id: ObjectId, name: &str) -> ObjectResult<Option<ObjectId>> {
        let target_type = TypeId::of::<T>();
        let children = self.children(id)?;
        for &child_id in children {
            if let Some(data) = self.objects.get(child_id) {
                if data.name == name && data.type_id == target_type {
                    return Ok(Some(child_id));
                }
            }
        }
        Ok(None)
    }

    /// Find all children with the given type (direct children only).
    pub fn find_children_by_type<T: 'static>(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        let target_type = TypeId::of::<T>();
        let children = self.children(id)?;
        Ok(children
            .iter()
            .filter(|&&child_id| {
                self.objects
                    .get(child_id)
                    .is_some_and(|d| d.type_id == target_type)
            })
            .copied()
            .collect())
    }

    /// Recursively find all descendants with the given name.
    pub fn find_descendants_by_name(
        &self,
        id: ObjectId,
        name: &str,
    ) -> ObjectResult<Vec<ObjectId>> {
        let mut result = Vec::new();
        self.find_descendants_by_name_recursive(id, name, &mut result)?;
        Ok(result)
    }

    fn find_descendants_by_name_recursive(
        &self,
        id: ObjectId,
        name: &str,
        result: &mut Vec<ObjectId>,
    ) -> ObjectResult<()> {
        let children = self.children(id)?;
        for &child_id in children {
            if let Some(data) = self.objects.get(child_id) {
                if data.name == name {
                    result.push(child_id);
                }
            }
            self.find_descendants_by_name_recursive(child_id, name, result)?;
        }
        Ok(())
    }

    /// Set a dynamic property on an object.
    pub fn set_dynamic_property<T: Any + Send + Sync>(
        &mut self,
        id: ObjectId,
        name: impl Into<String>,
        value: T,
    ) -> ObjectResult<()> {
        let data = self.objects.get_mut(id).ok_or(ObjectError::InvalidObjectId)?;
        data.dynamic_properties.insert(name.into(), Box::new(value));
        Ok(())
    }

    /// Get a dynamic property from an object.
    pub fn dynamic_property<T: Any>(&self, id: ObjectId, name: &str) -> ObjectResult<Option<&T>> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;
        Ok(data
            .dynamic_properties
            .get(name)
            .and_then(|v| v.downcast_ref::<T>()))
    }

    /// Remove a dynamic property from an object.
    pub fn remove_dynamic_property(
        &mut self,
        id: ObjectId,
        name: &str,
    ) -> ObjectResult<Option<Box<dyn Any + Send + Sync>>> {
        let data = self.objects.get_mut(id).ok_or(ObjectError::InvalidObjectId)?;
        Ok(data.dynamic_properties.remove(name))
    }

    /// Get all dynamic property names for an object.
    pub fn dynamic_property_names(&self, id: ObjectId) -> ObjectResult<Vec<&str>> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;
        Ok(data.dynamic_properties.keys().map(|s| s.as_str()).collect())
    }

    /// Get the number of registered objects.
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    /// Iterate over all root objects (objects with no parent).
    pub fn root_objects(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.objects
            .iter()
            .filter(|(_, data)| data.parent.is_none())
            .map(|(id, _)| id)
    }

    /// Debug dump of the object tree.
    pub fn dump_object_tree(&self, id: ObjectId) -> ObjectResult<String> {
        let mut output = String::new();
        self.dump_object_tree_recursive(id, 0, &mut output)?;
        Ok(output)
    }

    fn dump_object_tree_recursive(
        &self,
        id: ObjectId,
        depth: usize,
        output: &mut String,
    ) -> ObjectResult<()> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;
        let indent = "  ".repeat(depth);
        let name_display = if data.name.is_empty() {
            "(unnamed)"
        } else {
            &data.name
        };
        output.push_str(&format!(
            "{}[{:?}] {} ({})\n",
            indent, id, name_display, data.type_name
        ));
        for &child_id in &data.children {
            self.dump_object_tree_recursive(child_id, depth + 1, output)?;
        }
        Ok(())
    }
}

impl Default for ObjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A thread-safe wrapper around `ObjectRegistry`.
pub struct SharedObjectRegistry {
    inner: RwLock<ObjectRegistry>,
}

impl SharedObjectRegistry {
    /// Create a new shared object registry.
    pub fn new() -> Self {
        Self {
            inner: RwLock::new(ObjectRegistry::new()),
        }
    }

    /// Register a new object.
    pub fn register<T: Object + 'static>(&self) -> ObjectId {
        self.inner.write().register::<T>()
    }

    /// Destroy an object and its children.
    pub fn destroy(&self, id: ObjectId) -> ObjectResult<()> {
        self.inner.write().destroy(id)
    }

    /// Check if an object exists.
    pub fn contains(&self, id: ObjectId) -> bool {
        self.inner.read().contains(id)
    }

    /// Set the parent of an object.
    pub fn set_parent(&self, id: ObjectId, parent: Option<ObjectId>) -> ObjectResult<()> {
        self.inner.write().set_parent(id, parent)
    }

    /// Get the parent of an object.
    pub fn parent(&self, id: ObjectId) -> ObjectResult<Option<ObjectId>> {
        self.inner.read().parent(id)
    }

    /// Get the children of an object (returns owned Vec for thread safety).
    pub fn children(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        self.inner.read().children(id).map(|c| c.to_vec())
    }

    /// Get the object's name.
    pub fn object_name(&self, id: ObjectId) -> ObjectResult<String> {
        self.inner.read().object_name(id).map(|s| s.to_string())
    }

    /// Set the object's name.
    pub fn set_object_name(&self, id: ObjectId, name: String) -> ObjectResult<()> {
        self.inner.write().set_object_name(id, name)
    }

    /// Get the type ID of an object.
    pub fn type_id(&self, id: ObjectId) -> ObjectResult<TypeId> {
        self.with_read(|r| r.type_id(id))
    }

    /// Get the type name of an object.
    pub fn type_name(&self, id: ObjectId) -> ObjectResult<&'static str> {
        self.with_read(|r| r.type_name(id))
    }

    /// Find a child by name.
    pub fn find_child_by_name(&self, id: ObjectId, name: &str) -> ObjectResult<Option<ObjectId>> {
        self.inner.read().find_child_by_name(id, name)
    }

    /// Find a child by name and type.
    pub fn find_child<T: 'static>(&self, id: ObjectId, name: &str) -> ObjectResult<Option<ObjectId>> {
        self.inner.read().find_child::<T>(id, name)
    }

    /// Set a dynamic property.
    pub fn set_dynamic_property<T: Any + Send + Sync>(
        &self,
        id: ObjectId,
        name: impl Into<String>,
        value: T,
    ) -> ObjectResult<()> {
        self.inner.write().set_dynamic_property(id, name, value)
    }

    /// Remove a dynamic property.
    pub fn remove_dynamic_property(
        &self,
        id: ObjectId,
        name: &str,
    ) -> ObjectResult<Option<Box<dyn Any + Send + Sync>>> {
        self.inner.write().remove_dynamic_property(id, name)
    }

    /// Get the number of registered objects.
    pub fn object_count(&self) -> usize {
        self.inner.read().object_count()
    }

    /// Get all root objects.
    pub fn root_objects(&self) -> Vec<ObjectId> {
        self.inner.read().root_objects().collect()
    }

    /// Access the registry with a read lock for complex operations.
    pub fn with_read<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&ObjectRegistry) -> R,
    {
        f(&self.inner.read())
    }

    /// Access the registry with a write lock for complex operations.
    pub fn with_write<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut ObjectRegistry) -> R,
    {
        f(&mut self.inner.write())
    }
}

impl Default for SharedObjectRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global object registry (lazy initialized).
static GLOBAL_REGISTRY: Mutex<Option<SharedObjectRegistry>> = Mutex::new(None);

/// Initialize the global object registry.
///
/// This is called automatically by `Application::new()`.
pub fn init_global_registry() {
    let mut guard = GLOBAL_REGISTRY.lock();
    if guard.is_none() {
        *guard = Some(SharedObjectRegistry::new());
    }
}

/// Get a reference to the global object registry.
///
/// Returns an error if the registry hasn't been initialized.
pub fn global_registry() -> ObjectResult<&'static SharedObjectRegistry> {
    // SAFETY: Once initialized, the registry is never moved or deallocated.
    // We use a static Mutex to protect initialization.
    let guard = GLOBAL_REGISTRY.lock();
    if guard.is_some() {
        // Get a static reference by re-acquiring without the lock.
        drop(guard);
        let guard = GLOBAL_REGISTRY.lock();
        // SAFETY: The Option is Some and we never set it back to None.
        Ok(unsafe {
            let ptr = guard.as_ref().unwrap() as *const SharedObjectRegistry;
            &*ptr
        })
    } else {
        Err(ObjectError::RegistryNotInitialized)
    }
}

/// The base trait that all objects must implement.
///
/// This is the Rust equivalent of Qt's QObject. Types implementing this trait
/// can participate in the object tree, have dynamic properties, and will support
/// signals/slots when that system is implemented.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice_core::object::{Object, ObjectId, ObjectBase, global_registry};
///
/// struct MyWidget {
///     base: ObjectBase,
///     // ... widget-specific fields
/// }
///
/// impl Object for MyWidget {
///     fn object_id(&self) -> ObjectId {
///         self.base.id()
///     }
/// }
/// ```
pub trait Object: Any + Send + Sync {
    /// Get this object's unique identifier.
    fn object_id(&self) -> ObjectId;

    /// Get the static meta-object for this type.
    ///
    /// Returns `Some(&MetaObject)` if this type has meta-object information
    /// (typically generated by `#[derive(Object)]`), or `None` for types
    /// without full meta-object support.
    ///
    /// The meta-object provides runtime type information including:
    /// - Type name and inheritance chain
    /// - Property metadata with dynamic accessors
    /// - Signal metadata
    /// - Method metadata for dynamic invocation
    ///
    /// # Example
    ///
    /// ```ignore
    /// let obj: &dyn Object = &my_widget;
    /// if let Some(meta) = obj.meta_object() {
    ///     println!("Type: {}", meta.type_name);
    ///     for prop_name in meta.property_names() {
    ///         println!("  Property: {}", prop_name);
    ///     }
    /// }
    /// ```
    fn meta_object(&self) -> Option<&'static crate::meta::MetaObject> {
        None
    }
}

/// Helper for implementing the Object trait.
///
/// Include this as a field in your object types to handle registration
/// and provide the object ID.
///
/// # Example
///
/// ```ignore
/// struct MyWidget {
///     base: ObjectBase,
///     title: String,
/// }
///
/// impl MyWidget {
///     fn new() -> Self {
///         Self {
///             base: ObjectBase::new::<Self>(),
///             title: String::new(),
///         }
///     }
/// }
///
/// impl Object for MyWidget {
///     fn object_id(&self) -> ObjectId {
///         self.base.id()
///     }
/// }
/// ```
pub struct ObjectBase {
    id: ObjectId,
}

impl ObjectBase {
    /// Create a new ObjectBase, registering the object in the global registry.
    ///
    /// The registry must be initialized first (via `Application::new()` or `init_global_registry()`).
    ///
    /// # Panics
    ///
    /// Panics if the global registry is not initialized.
    pub fn new<T: Object + 'static>() -> Self {
        let registry = global_registry().expect("Object registry not initialized");
        let id = registry.register::<T>();
        Self { id }
    }

    /// Get the object's ID.
    pub fn id(&self) -> ObjectId {
        self.id
    }

    /// Get the object's name from the registry.
    pub fn name(&self) -> String {
        global_registry()
            .and_then(|r| r.object_name(self.id))
            .unwrap_or_default()
    }

    /// Set the object's name in the registry.
    pub fn set_name(&self, name: impl Into<String>) {
        if let Ok(registry) = global_registry() {
            let _ = registry.set_object_name(self.id, name.into());
        }
    }

    /// Get the parent object ID.
    pub fn parent(&self) -> Option<ObjectId> {
        global_registry()
            .and_then(|r| r.parent(self.id))
            .ok()
            .flatten()
    }

    /// Set the parent object.
    pub fn set_parent(&self, parent: Option<ObjectId>) -> ObjectResult<()> {
        global_registry()?.set_parent(self.id, parent)
    }

    /// Get child object IDs.
    pub fn children(&self) -> Vec<ObjectId> {
        global_registry()
            .and_then(|r| r.children(self.id))
            .unwrap_or_default()
    }

    /// Find a child by name.
    pub fn find_child_by_name(&self, name: &str) -> Option<ObjectId> {
        global_registry()
            .and_then(|r| r.find_child_by_name(self.id, name))
            .ok()
            .flatten()
    }

    /// Set a dynamic property.
    pub fn set_property<T: Any + Send + Sync>(&self, name: &str, value: T) -> ObjectResult<()> {
        global_registry()?.set_dynamic_property(self.id, name, value)
    }
}

impl Drop for ObjectBase {
    fn drop(&mut self) {
        // Automatically unregister from the global registry when dropped.
        if let Ok(registry) = global_registry() {
            let _ = registry.destroy(self.id);
        }
    }
}

/// Safe downcast function for Object trait objects.
///
/// This is the equivalent of Qt's `qobject_cast`.
pub fn object_cast<T: Object + 'static>(obj: &dyn Object) -> Option<&T> {
    (obj as &dyn Any).downcast_ref::<T>()
}

/// Safe mutable downcast function for Object trait objects.
pub fn object_cast_mut<T: Object + 'static>(obj: &mut dyn Object) -> Option<&mut T> {
    (obj as &mut dyn Any).downcast_mut::<T>()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test object type
    struct TestObject {
        base: ObjectBase,
        value: i32,
    }

    impl TestObject {
        fn new(value: i32) -> Self {
            Self {
                base: ObjectBase::new::<Self>(),
                value,
            }
        }
    }

    impl Object for TestObject {
        fn object_id(&self) -> ObjectId {
            self.base.id()
        }
    }

    // Another test object type
    struct ChildObject {
        base: ObjectBase,
    }

    impl ChildObject {
        fn new(name: &str) -> Self {
            let obj = Self {
                base: ObjectBase::new::<Self>(),
            };
            obj.base.set_name(name);
            obj
        }
    }

    impl Object for ChildObject {
        fn object_id(&self) -> ObjectId {
            self.base.id()
        }
    }

    fn setup() {
        init_global_registry();
    }

    #[test]
    fn test_object_creation() {
        setup();
        let obj = TestObject::new(42);
        assert!(global_registry().unwrap().contains(obj.object_id()));
    }

    #[test]
    fn test_object_name() {
        setup();
        let obj = TestObject::new(1);
        obj.base.set_name("test_object");
        assert_eq!(obj.base.name(), "test_object");
    }

    #[test]
    fn test_parent_child() {
        setup();
        let parent = TestObject::new(1);
        let child = ChildObject::new("child1");

        child.base.set_parent(Some(parent.object_id())).unwrap();

        assert_eq!(child.base.parent(), Some(parent.object_id()));
        assert!(parent.base.children().contains(&child.object_id()));
    }

    #[test]
    fn test_find_child_by_name() {
        setup();
        let parent = TestObject::new(1);
        let child1 = ChildObject::new("alpha");
        let child2 = ChildObject::new("beta");

        child1.base.set_parent(Some(parent.object_id())).unwrap();
        child2.base.set_parent(Some(parent.object_id())).unwrap();

        let found = parent.base.find_child_by_name("beta");
        assert_eq!(found, Some(child2.object_id()));
    }

    #[test]
    fn test_dynamic_properties() {
        setup();
        let obj = TestObject::new(1);
        obj.base.set_property("counter", 100i32).unwrap();

        let registry = global_registry().unwrap();
        let value = registry
            .with_read(|r| r.dynamic_property::<i32>(obj.object_id(), "counter").ok().flatten().copied());
        assert_eq!(value, Some(100));
    }

    #[test]
    fn test_cascade_destroy() {
        setup();
        let registry = global_registry().unwrap();

        // Create objects directly in registry to test cascade delete without ObjectBase Drop
        let parent_id = registry.register::<TestObject>();
        let child1_id = registry.register::<ChildObject>();
        let child2_id = registry.register::<ChildObject>();
        let grandchild_id = registry.register::<ChildObject>();

        // Set up parent-child relationships
        registry.set_parent(child1_id, Some(parent_id)).unwrap();
        registry.set_parent(child2_id, Some(parent_id)).unwrap();
        registry.set_parent(grandchild_id, Some(child1_id)).unwrap();

        // Verify all exist
        assert!(registry.contains(parent_id));
        assert!(registry.contains(child1_id));
        assert!(registry.contains(child2_id));
        assert!(registry.contains(grandchild_id));

        // Destroy parent - should cascade to all descendants
        registry.destroy(parent_id).unwrap();

        // All should be gone (cascade delete)
        assert!(!registry.contains(parent_id));
        assert!(!registry.contains(child1_id));
        assert!(!registry.contains(child2_id));
        assert!(!registry.contains(grandchild_id));
    }

    #[test]
    fn test_circular_parentage_rejected() {
        setup();
        let obj1 = TestObject::new(1);
        let obj2 = TestObject::new(2);

        obj2.base.set_parent(Some(obj1.object_id())).unwrap();

        // Trying to set obj1's parent to obj2 should fail (circular)
        let result = obj1.base.set_parent(Some(obj2.object_id()));
        assert!(matches!(result, Err(ObjectError::CircularParentage)));
    }

    #[test]
    fn test_reparenting() {
        setup();
        let parent1 = TestObject::new(1);
        let parent2 = TestObject::new(2);
        let child = ChildObject::new("mobile");

        child.base.set_parent(Some(parent1.object_id())).unwrap();
        assert!(parent1.base.children().contains(&child.object_id()));

        // Reparent to parent2
        child.base.set_parent(Some(parent2.object_id())).unwrap();

        assert!(!parent1.base.children().contains(&child.object_id()));
        assert!(parent2.base.children().contains(&child.object_id()));
        assert_eq!(child.base.parent(), Some(parent2.object_id()));
    }

    #[test]
    fn test_object_cast() {
        setup();
        let obj = TestObject::new(42);
        let obj_ref: &dyn Object = &obj;

        let casted = object_cast::<TestObject>(obj_ref);
        assert!(casted.is_some());
        assert_eq!(casted.unwrap().value, 42);

        // Wrong type cast returns None
        let wrong_cast = object_cast::<ChildObject>(obj_ref);
        assert!(wrong_cast.is_none());
    }
}
