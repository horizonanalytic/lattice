//! Object model for Horizon Lattice.
//!
//! Provides the base object system with:
//! - Unique object identifiers via arena-based storage
//! - Parent-child ownership relationships with automatic drop cascade
//! - Object naming and lookup
//! - Dynamic property storage
//!
//! This is the Rust equivalent of Qt's QObject system.
//!
//! # Key Types
//!
//! - [`Object`] - Base trait that all objects implement
//! - [`ObjectBase`] - Helper struct for implementing [`Object`]
//! - [`ObjectId`] - Unique stable identifier for each object
//! - [`ObjectRegistry`] - Central registry managing all objects
//! - [`SharedObjectRegistry`] - Thread-safe wrapper around [`ObjectRegistry`]
//!
//! # Related Modules
//!
//! - [`crate::Signal`] - Objects typically contain signals
//! - [`crate::Property`] - Objects typically contain properties
//! - [`crate::meta::MetaObject`] - Runtime type information for objects
//! - [`crate::Application`] - Initializes the global object registry
//!
//! # Guide
//!
//! For a comprehensive guide on the object system, see the
//! [Architecture Guide](https://horizonanalyticstudios.github.io/horizon-lattice/guides/architecture.html).

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::fmt;

use parking_lot::{Mutex, RwLock};
use slotmap::{new_key_type, SlotMap};

new_key_type! {
    /// A unique identifier for an object in the registry.
    ///
    /// `ObjectId`s are stable handles that remain valid even as the object tree changes.
    /// They become invalid when the object is destroyed.
    ///
    /// # Related Types
    ///
    /// - [`Object`] - Trait that provides [`object_id()`](Object::object_id)
    /// - [`ObjectBase`] - Generates an `ObjectId` on construction
    /// - [`ObjectRegistry`] - Manages the mapping from `ObjectId` to object data
    pub struct ObjectId;
}

impl ObjectId {
    /// Convert the ObjectId to a raw u64 value.
    ///
    /// This is useful for interop with external systems that need a numeric ID.
    /// The raw value can be converted back using [`ObjectId::from_raw`].
    #[inline]
    pub fn as_raw(self) -> u64 {
        use slotmap::Key;
        self.data().as_ffi()
    }

    /// Create an ObjectId from a raw u64 value.
    ///
    /// Returns `Some` if the raw value could be a valid ObjectId.
    /// Note: This does not check if the ObjectId exists in the registry.
    #[inline]
    pub fn from_raw(raw: u64) -> Option<Self> {
        let key_data = slotmap::KeyData::from_ffi(raw);
        Some(Self::from(key_data))
    }
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
        /// The expected type name.
        expected: &'static str,
        /// The actual type name that was provided.
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

/// Widget-specific state stored in the registry for state propagation.
///
/// This is stored separately from the widget instance so that parent state
/// can be queried by ObjectId when computing effective visibility/enabled state.
#[derive(Clone, Copy, Debug, Default)]
pub struct WidgetState {
    /// Whether the widget is visible (its own state, not considering ancestors).
    pub visible: bool,
    /// Whether the widget is enabled (its own state, not considering ancestors).
    pub enabled: bool,
}

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
    /// Widget state for state propagation (None for non-widget objects).
    widget_state: Option<WidgetState>,
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
            widget_state: None,
        }
    }
}

/// The central registry that manages all objects and their relationships.
///
/// Uses arena-based storage via SlotMap for stable object IDs and efficient
/// parent-child relationship management.
///
/// # Related Types
///
/// - [`SharedObjectRegistry`] - Thread-safe wrapper for concurrent access
/// - [`ObjectId`] - Keys into this registry
/// - [`ObjectBase`] - Automatically registers objects here
/// - [`global_registry`] - Access the singleton instance
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

    // =========================================================================
    // Widget State (for state propagation)
    // =========================================================================

    /// Initialize widget state for an object.
    ///
    /// Called when a widget is created to set up initial state in the registry.
    /// This enables state propagation queries via `is_effectively_visible`/`is_effectively_enabled`.
    pub fn init_widget_state(&mut self, id: ObjectId, state: WidgetState) -> ObjectResult<()> {
        let data = self.objects.get_mut(id).ok_or(ObjectError::InvalidObjectId)?;
        data.widget_state = Some(state);
        Ok(())
    }

    /// Get the widget state for an object.
    ///
    /// Returns `None` if the object is not a widget or doesn't have state initialized.
    pub fn widget_state(&self, id: ObjectId) -> ObjectResult<Option<WidgetState>> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;
        Ok(data.widget_state)
    }

    /// Set the visible state for a widget.
    pub fn set_widget_visible(&mut self, id: ObjectId, visible: bool) -> ObjectResult<()> {
        let data = self.objects.get_mut(id).ok_or(ObjectError::InvalidObjectId)?;
        if let Some(ref mut state) = data.widget_state {
            state.visible = visible;
        } else {
            data.widget_state = Some(WidgetState {
                visible,
                enabled: true,
            });
        }
        Ok(())
    }

    /// Set the enabled state for a widget.
    pub fn set_widget_enabled(&mut self, id: ObjectId, enabled: bool) -> ObjectResult<()> {
        let data = self.objects.get_mut(id).ok_or(ObjectError::InvalidObjectId)?;
        if let Some(ref mut state) = data.widget_state {
            state.enabled = enabled;
        } else {
            data.widget_state = Some(WidgetState {
                visible: true,
                enabled,
            });
        }
        Ok(())
    }

    /// Check if a widget is effectively visible (itself and all ancestors are visible).
    ///
    /// Returns `true` if the object is visible and all ancestors are also visible.
    /// Returns `false` if any ancestor is hidden.
    /// Returns `None` if the object doesn't have widget state.
    pub fn is_effectively_visible(&self, id: ObjectId) -> ObjectResult<Option<bool>> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;

        // Check if this is a widget
        let state = match data.widget_state {
            Some(s) => s,
            None => return Ok(None),
        };

        // If self is not visible, not effectively visible
        if !state.visible {
            return Ok(Some(false));
        }

        // Check all ancestors
        let mut current = data.parent;
        while let Some(current_id) = current {
            if let Some(ancestor_data) = self.objects.get(current_id) {
                if let Some(ancestor_state) = ancestor_data.widget_state {
                    if !ancestor_state.visible {
                        return Ok(Some(false));
                    }
                }
                current = ancestor_data.parent;
            } else {
                break;
            }
        }

        Ok(Some(true))
    }

    /// Check if a widget is effectively enabled (itself and all ancestors are enabled).
    ///
    /// Returns `true` if the object is enabled and all ancestors are also enabled.
    /// Returns `false` if any ancestor is disabled.
    /// Returns `None` if the object doesn't have widget state.
    pub fn is_effectively_enabled(&self, id: ObjectId) -> ObjectResult<Option<bool>> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;

        // Check if this is a widget
        let state = match data.widget_state {
            Some(s) => s,
            None => return Ok(None),
        };

        // If self is not enabled, not effectively enabled
        if !state.enabled {
            return Ok(Some(false));
        }

        // Check all ancestors
        let mut current = data.parent;
        while let Some(current_id) = current {
            if let Some(ancestor_data) = self.objects.get(current_id) {
                if let Some(ancestor_state) = ancestor_data.widget_state {
                    if !ancestor_state.enabled {
                        return Ok(Some(false));
                    }
                }
                current = ancestor_data.parent;
            } else {
                break;
            }
        }

        Ok(Some(true))
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

    // =========================================================================
    // Z-Order / Sibling Ordering
    // =========================================================================

    /// Get the index of an object among its siblings.
    ///
    /// Returns `None` if the object has no parent (is a root object).
    /// Index 0 is the back/bottom, higher indices are front/top.
    pub fn sibling_index(&self, id: ObjectId) -> ObjectResult<Option<usize>> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;

        if let Some(parent_id) = data.parent {
            let parent_data = self
                .objects
                .get(parent_id)
                .ok_or(ObjectError::InvalidObjectId)?;
            Ok(parent_data.children.iter().position(|&child| child == id))
        } else {
            Ok(None)
        }
    }

    /// Get the next sibling (higher z-order / closer to front).
    pub fn next_sibling(&self, id: ObjectId) -> ObjectResult<Option<ObjectId>> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;

        if let Some(parent_id) = data.parent {
            let parent_data = self
                .objects
                .get(parent_id)
                .ok_or(ObjectError::InvalidObjectId)?;

            if let Some(pos) = parent_data.children.iter().position(|&child| child == id) {
                if pos + 1 < parent_data.children.len() {
                    return Ok(Some(parent_data.children[pos + 1]));
                }
            }
        }
        Ok(None)
    }

    /// Get the previous sibling (lower z-order / closer to back).
    pub fn previous_sibling(&self, id: ObjectId) -> ObjectResult<Option<ObjectId>> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;

        if let Some(parent_id) = data.parent {
            let parent_data = self
                .objects
                .get(parent_id)
                .ok_or(ObjectError::InvalidObjectId)?;

            if let Some(pos) = parent_data.children.iter().position(|&child| child == id) {
                if pos > 0 {
                    return Ok(Some(parent_data.children[pos - 1]));
                }
            }
        }
        Ok(None)
    }

    /// Raise an object to the front (highest z-order among siblings).
    ///
    /// Moves the object to the end of its parent's children list.
    pub fn raise(&mut self, id: ObjectId) -> ObjectResult<()> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;

        if let Some(parent_id) = data.parent {
            let parent_data = self
                .objects
                .get_mut(parent_id)
                .ok_or(ObjectError::InvalidObjectId)?;

            // Remove from current position and add at end
            parent_data.children.retain(|&child| child != id);
            parent_data.children.push(id);
        }
        Ok(())
    }

    /// Lower an object to the back (lowest z-order among siblings).
    ///
    /// Moves the object to the start of its parent's children list.
    pub fn lower(&mut self, id: ObjectId) -> ObjectResult<()> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;

        if let Some(parent_id) = data.parent {
            let parent_data = self
                .objects
                .get_mut(parent_id)
                .ok_or(ObjectError::InvalidObjectId)?;

            // Remove from current position and insert at beginning
            parent_data.children.retain(|&child| child != id);
            parent_data.children.insert(0, id);
        }
        Ok(())
    }

    /// Stack an object under (behind) a sibling.
    ///
    /// The object will be positioned just before the sibling in the children list.
    /// Returns an error if the sibling is not actually a sibling.
    pub fn stack_under(&mut self, id: ObjectId, sibling: ObjectId) -> ObjectResult<()> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;
        let sibling_data = self
            .objects
            .get(sibling)
            .ok_or(ObjectError::InvalidObjectId)?;

        // Check they share the same parent
        if data.parent != sibling_data.parent || data.parent.is_none() {
            return Err(ObjectError::InvalidObjectId); // Not siblings
        }

        let parent_id = data.parent.unwrap();
        let parent_data = self
            .objects
            .get_mut(parent_id)
            .ok_or(ObjectError::InvalidObjectId)?;

        // Remove both, find sibling position, insert id before sibling
        parent_data.children.retain(|&child| child != id);

        if let Some(sibling_pos) = parent_data.children.iter().position(|&c| c == sibling) {
            parent_data.children.insert(sibling_pos, id);
        }

        Ok(())
    }

    /// Stack an object above (in front of) a sibling.
    ///
    /// The object will be positioned just after the sibling in the children list.
    /// Returns an error if the sibling is not actually a sibling.
    pub fn stack_above(&mut self, id: ObjectId, sibling: ObjectId) -> ObjectResult<()> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;
        let sibling_data = self
            .objects
            .get(sibling)
            .ok_or(ObjectError::InvalidObjectId)?;

        // Check they share the same parent
        if data.parent != sibling_data.parent || data.parent.is_none() {
            return Err(ObjectError::InvalidObjectId); // Not siblings
        }

        let parent_id = data.parent.unwrap();
        let parent_data = self
            .objects
            .get_mut(parent_id)
            .ok_or(ObjectError::InvalidObjectId)?;

        // Remove id, find sibling position, insert id after sibling
        parent_data.children.retain(|&child| child != id);

        if let Some(sibling_pos) = parent_data.children.iter().position(|&c| c == sibling) {
            parent_data.children.insert(sibling_pos + 1, id);
        }

        Ok(())
    }

    /// Get all siblings of an object (excluding the object itself).
    pub fn siblings(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;

        if let Some(parent_id) = data.parent {
            let parent_data = self
                .objects
                .get(parent_id)
                .ok_or(ObjectError::InvalidObjectId)?;

            Ok(parent_data
                .children
                .iter()
                .filter(|&&child| child != id)
                .copied()
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    // =========================================================================
    // Tree Traversal
    // =========================================================================

    /// Get all ancestors of an object from immediate parent to root.
    pub fn ancestors(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        if !self.objects.contains_key(id) {
            return Err(ObjectError::InvalidObjectId);
        }

        let mut result = Vec::new();
        let mut current = self.objects.get(id).and_then(|d| d.parent);

        while let Some(current_id) = current {
            result.push(current_id);
            current = self.objects.get(current_id).and_then(|d| d.parent);
        }

        Ok(result)
    }

    /// Perform a depth-first pre-order traversal starting from an object.
    ///
    /// Visits the node first, then its children recursively.
    /// Returns objects in the order: root, child1, grandchild1, grandchild2, child2, ...
    pub fn depth_first_preorder(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        let mut result = Vec::new();
        self.depth_first_preorder_recursive(id, &mut result)?;
        Ok(result)
    }

    fn depth_first_preorder_recursive(
        &self,
        id: ObjectId,
        result: &mut Vec<ObjectId>,
    ) -> ObjectResult<()> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;
        result.push(id);
        for &child_id in &data.children {
            self.depth_first_preorder_recursive(child_id, result)?;
        }
        Ok(())
    }

    /// Perform a depth-first post-order traversal starting from an object.
    ///
    /// Visits children recursively first, then the node.
    /// Returns objects in the order: grandchild1, grandchild2, child1, child2, root
    pub fn depth_first_postorder(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        let mut result = Vec::new();
        self.depth_first_postorder_recursive(id, &mut result)?;
        Ok(result)
    }

    fn depth_first_postorder_recursive(
        &self,
        id: ObjectId,
        result: &mut Vec<ObjectId>,
    ) -> ObjectResult<()> {
        let data = self.objects.get(id).ok_or(ObjectError::InvalidObjectId)?;
        for &child_id in &data.children {
            self.depth_first_postorder_recursive(child_id, result)?;
        }
        result.push(id);
        Ok(())
    }

    /// Perform a breadth-first (level-order) traversal starting from an object.
    ///
    /// Visits all nodes at depth N before any nodes at depth N+1.
    pub fn breadth_first(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        if !self.objects.contains_key(id) {
            return Err(ObjectError::InvalidObjectId);
        }

        let mut result = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(id);

        while let Some(current_id) = queue.pop_front() {
            result.push(current_id);
            if let Some(data) = self.objects.get(current_id) {
                for &child_id in &data.children {
                    queue.push_back(child_id);
                }
            }
        }

        Ok(result)
    }

    // =========================================================================
    // Debug / Diagnostics
    // =========================================================================

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

/// A thread-safe wrapper around [`ObjectRegistry`].
///
/// Provides concurrent read access with exclusive write access via `RwLock`.
///
/// # Related
///
/// - [`ObjectRegistry`] - The underlying registry
/// - [`global_registry`] - Returns a `SharedObjectRegistry`
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

    // =========================================================================
    // Widget State (for state propagation)
    // =========================================================================

    /// Initialize widget state for an object.
    pub fn init_widget_state(&self, id: ObjectId, state: WidgetState) -> ObjectResult<()> {
        self.inner.write().init_widget_state(id, state)
    }

    /// Get the widget state for an object.
    pub fn widget_state(&self, id: ObjectId) -> ObjectResult<Option<WidgetState>> {
        self.inner.read().widget_state(id)
    }

    /// Set the visible state for a widget.
    pub fn set_widget_visible(&self, id: ObjectId, visible: bool) -> ObjectResult<()> {
        self.inner.write().set_widget_visible(id, visible)
    }

    /// Set the enabled state for a widget.
    pub fn set_widget_enabled(&self, id: ObjectId, enabled: bool) -> ObjectResult<()> {
        self.inner.write().set_widget_enabled(id, enabled)
    }

    /// Check if a widget is effectively visible (itself and all ancestors are visible).
    pub fn is_effectively_visible(&self, id: ObjectId) -> ObjectResult<Option<bool>> {
        self.inner.read().is_effectively_visible(id)
    }

    /// Check if a widget is effectively enabled (itself and all ancestors are enabled).
    pub fn is_effectively_enabled(&self, id: ObjectId) -> ObjectResult<Option<bool>> {
        self.inner.read().is_effectively_enabled(id)
    }

    // =========================================================================
    // Z-Order / Sibling Ordering
    // =========================================================================

    /// Get the index of an object among its siblings.
    pub fn sibling_index(&self, id: ObjectId) -> ObjectResult<Option<usize>> {
        self.inner.read().sibling_index(id)
    }

    /// Get the next sibling (higher z-order).
    pub fn next_sibling(&self, id: ObjectId) -> ObjectResult<Option<ObjectId>> {
        self.inner.read().next_sibling(id)
    }

    /// Get the previous sibling (lower z-order).
    pub fn previous_sibling(&self, id: ObjectId) -> ObjectResult<Option<ObjectId>> {
        self.inner.read().previous_sibling(id)
    }

    /// Raise an object to the front (highest z-order among siblings).
    pub fn raise(&self, id: ObjectId) -> ObjectResult<()> {
        self.inner.write().raise(id)
    }

    /// Lower an object to the back (lowest z-order among siblings).
    pub fn lower(&self, id: ObjectId) -> ObjectResult<()> {
        self.inner.write().lower(id)
    }

    /// Stack an object under (behind) a sibling.
    pub fn stack_under(&self, id: ObjectId, sibling: ObjectId) -> ObjectResult<()> {
        self.inner.write().stack_under(id, sibling)
    }

    /// Stack an object above (in front of) a sibling.
    pub fn stack_above(&self, id: ObjectId, sibling: ObjectId) -> ObjectResult<()> {
        self.inner.write().stack_above(id, sibling)
    }

    /// Get all siblings of an object (excluding the object itself).
    pub fn siblings(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        self.inner.read().siblings(id)
    }

    // =========================================================================
    // Tree Traversal
    // =========================================================================

    /// Get all ancestors of an object from immediate parent to root.
    pub fn ancestors(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        self.inner.read().ancestors(id)
    }

    /// Perform a depth-first pre-order traversal starting from an object.
    pub fn depth_first_preorder(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        self.inner.read().depth_first_preorder(id)
    }

    /// Perform a depth-first post-order traversal starting from an object.
    pub fn depth_first_postorder(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        self.inner.read().depth_first_postorder(id)
    }

    /// Perform a breadth-first (level-order) traversal starting from an object.
    pub fn breadth_first(&self, id: ObjectId) -> ObjectResult<Vec<ObjectId>> {
        self.inner.read().breadth_first(id)
    }

    // =========================================================================
    // Advanced Access
    // =========================================================================

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
/// can participate in the object tree, have dynamic properties, and support
/// signals/slots through the [`Signal`](crate::Signal) system.
///
/// # Related Types
///
/// - [`ObjectBase`] - Helper for implementing this trait
/// - [`ObjectId`] - Returned by [`object_id()`](Self::object_id)
/// - [`crate::meta::MetaObject`] - Runtime type information via [`meta_object()`](Self::meta_object)
/// - [`object_cast`] - Safe downcasting function
///
/// # Example
///
/// ```
/// use horizon_lattice_core::{Object, ObjectId, ObjectBase, init_global_registry};
///
/// // Initialize the registry before creating objects
/// init_global_registry();
///
/// struct MyWidget {
///     base: ObjectBase,
///     title: String,
/// }
///
/// impl MyWidget {
///     fn new(title: &str) -> Self {
///         Self {
///             base: ObjectBase::new::<Self>(),
///             title: title.to_string(),
///         }
///     }
/// }
///
/// impl Object for MyWidget {
///     fn object_id(&self) -> ObjectId {
///         self.base.id()
///     }
/// }
///
/// let widget = MyWidget::new("Hello");
/// assert_eq!(widget.title, "Hello");
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
    /// ```no_run
    /// use horizon_lattice_core::Object;
    ///
    /// fn print_meta(obj: &dyn Object) {
    ///     if let Some(meta) = obj.meta_object() {
    ///         println!("Type: {}", meta.type_name);
    ///         for prop_name in meta.property_names() {
    ///             println!("  Property: {}", prop_name);
    ///         }
    ///     }
    /// }
    /// ```
    fn meta_object(&self) -> Option<&'static crate::meta::MetaObject> {
        None
    }
}

/// Helper for implementing the [`Object`] trait.
///
/// Include this as a field in your object types to handle registration
/// and provide the object ID. On construction, it automatically registers
/// the object with the [`global_registry`].
///
/// # Related Types
///
/// - [`Object`] - The trait this helps implement
/// - [`ObjectId`] - Obtained via [`id()`](Self::id)
/// - [`ObjectRegistry`] - Where objects are registered
///
/// # Example
///
/// ```
/// use horizon_lattice_core::{Object, ObjectId, ObjectBase, init_global_registry};
///
/// init_global_registry();
///
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
///
/// let widget = MyWidget::new();
/// widget.base.set_name("my_widget");
/// assert_eq!(widget.base.name(), "my_widget");
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

    // =========================================================================
    // Z-Order / Sibling Ordering
    // =========================================================================

    /// Get this object's index among its siblings.
    ///
    /// Index 0 is the back/bottom, higher indices are front/top.
    /// Returns `None` if the object has no parent.
    pub fn sibling_index(&self) -> Option<usize> {
        global_registry()
            .and_then(|r| r.sibling_index(self.id))
            .ok()
            .flatten()
    }

    /// Get the next sibling (higher z-order / closer to front).
    pub fn next_sibling(&self) -> Option<ObjectId> {
        global_registry()
            .and_then(|r| r.next_sibling(self.id))
            .ok()
            .flatten()
    }

    /// Get the previous sibling (lower z-order / closer to back).
    pub fn previous_sibling(&self) -> Option<ObjectId> {
        global_registry()
            .and_then(|r| r.previous_sibling(self.id))
            .ok()
            .flatten()
    }

    /// Raise this object to the front (highest z-order among siblings).
    pub fn raise(&self) -> ObjectResult<()> {
        global_registry()?.raise(self.id)
    }

    /// Lower this object to the back (lowest z-order among siblings).
    pub fn lower(&self) -> ObjectResult<()> {
        global_registry()?.lower(self.id)
    }

    /// Stack this object under (behind) a sibling.
    pub fn stack_under(&self, sibling: ObjectId) -> ObjectResult<()> {
        global_registry()?.stack_under(self.id, sibling)
    }

    /// Stack this object above (in front of) a sibling.
    pub fn stack_above(&self, sibling: ObjectId) -> ObjectResult<()> {
        global_registry()?.stack_above(self.id, sibling)
    }

    /// Get all siblings (excluding this object).
    pub fn siblings(&self) -> Vec<ObjectId> {
        global_registry()
            .and_then(|r| r.siblings(self.id))
            .unwrap_or_default()
    }

    // =========================================================================
    // Tree Traversal
    // =========================================================================

    /// Get all ancestors from immediate parent to root.
    pub fn ancestors(&self) -> Vec<ObjectId> {
        global_registry()
            .and_then(|r| r.ancestors(self.id))
            .unwrap_or_default()
    }

    /// Get descendants in depth-first pre-order (self, then children recursively).
    pub fn depth_first_preorder(&self) -> Vec<ObjectId> {
        global_registry()
            .and_then(|r| r.depth_first_preorder(self.id))
            .unwrap_or_default()
    }

    /// Get descendants in depth-first post-order (children recursively, then self).
    pub fn depth_first_postorder(&self) -> Vec<ObjectId> {
        global_registry()
            .and_then(|r| r.depth_first_postorder(self.id))
            .unwrap_or_default()
    }

    /// Get descendants in breadth-first (level) order.
    pub fn breadth_first(&self) -> Vec<ObjectId> {
        global_registry()
            .and_then(|r| r.breadth_first(self.id))
            .unwrap_or_default()
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

/// Safe downcast function for [`Object`] trait objects.
///
/// This is the equivalent of Qt's `qobject_cast`. Returns `Some(&T)` if the
/// object is of type `T`, otherwise `None`.
///
/// # Related
///
/// - [`object_cast_mut`] - Mutable version
/// - [`Object`] - The trait being downcast
pub fn object_cast<T: Object + 'static>(obj: &dyn Object) -> Option<&T> {
    (obj as &dyn Any).downcast_ref::<T>()
}

/// Safe mutable downcast function for [`Object`] trait objects.
///
/// # Related
///
/// - [`object_cast`] - Immutable version
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

    // =========================================================================
    // Z-Order Tests
    // =========================================================================

    #[test]
    fn test_sibling_index() {
        setup();
        let parent = TestObject::new(1);
        let child1 = ChildObject::new("first");
        let child2 = ChildObject::new("second");
        let child3 = ChildObject::new("third");

        child1.base.set_parent(Some(parent.object_id())).unwrap();
        child2.base.set_parent(Some(parent.object_id())).unwrap();
        child3.base.set_parent(Some(parent.object_id())).unwrap();

        assert_eq!(child1.base.sibling_index(), Some(0));
        assert_eq!(child2.base.sibling_index(), Some(1));
        assert_eq!(child3.base.sibling_index(), Some(2));

        // Root object has no sibling index
        assert_eq!(parent.base.sibling_index(), None);
    }

    #[test]
    fn test_next_previous_sibling() {
        setup();
        let parent = TestObject::new(1);
        let child1 = ChildObject::new("first");
        let child2 = ChildObject::new("second");
        let child3 = ChildObject::new("third");

        child1.base.set_parent(Some(parent.object_id())).unwrap();
        child2.base.set_parent(Some(parent.object_id())).unwrap();
        child3.base.set_parent(Some(parent.object_id())).unwrap();

        // First child has no previous, has next
        assert_eq!(child1.base.previous_sibling(), None);
        assert_eq!(child1.base.next_sibling(), Some(child2.object_id()));

        // Middle child has both
        assert_eq!(child2.base.previous_sibling(), Some(child1.object_id()));
        assert_eq!(child2.base.next_sibling(), Some(child3.object_id()));

        // Last child has previous, no next
        assert_eq!(child3.base.previous_sibling(), Some(child2.object_id()));
        assert_eq!(child3.base.next_sibling(), None);
    }

    #[test]
    fn test_raise_lower() {
        setup();
        let parent = TestObject::new(1);
        let child1 = ChildObject::new("first");
        let child2 = ChildObject::new("second");
        let child3 = ChildObject::new("third");

        child1.base.set_parent(Some(parent.object_id())).unwrap();
        child2.base.set_parent(Some(parent.object_id())).unwrap();
        child3.base.set_parent(Some(parent.object_id())).unwrap();

        // Initial order: [child1, child2, child3]
        assert_eq!(child1.base.sibling_index(), Some(0));

        // Raise child1 to front
        child1.base.raise().unwrap();
        // New order: [child2, child3, child1]
        assert_eq!(child1.base.sibling_index(), Some(2));
        assert_eq!(child2.base.sibling_index(), Some(0));

        // Lower child1 to back
        child1.base.lower().unwrap();
        // New order: [child1, child2, child3]
        assert_eq!(child1.base.sibling_index(), Some(0));
        assert_eq!(child3.base.sibling_index(), Some(2));
    }

    #[test]
    fn test_stack_under_above() {
        setup();
        let parent = TestObject::new(1);
        let child1 = ChildObject::new("first");
        let child2 = ChildObject::new("second");
        let child3 = ChildObject::new("third");

        child1.base.set_parent(Some(parent.object_id())).unwrap();
        child2.base.set_parent(Some(parent.object_id())).unwrap();
        child3.base.set_parent(Some(parent.object_id())).unwrap();

        // Initial order: [child1, child2, child3]

        // Stack child3 under child2
        child3.base.stack_under(child2.object_id()).unwrap();
        // New order: [child1, child3, child2]
        assert_eq!(child1.base.sibling_index(), Some(0));
        assert_eq!(child3.base.sibling_index(), Some(1));
        assert_eq!(child2.base.sibling_index(), Some(2));

        // Stack child1 above child2
        child1.base.stack_above(child2.object_id()).unwrap();
        // New order: [child3, child2, child1]
        assert_eq!(child3.base.sibling_index(), Some(0));
        assert_eq!(child2.base.sibling_index(), Some(1));
        assert_eq!(child1.base.sibling_index(), Some(2));
    }

    #[test]
    fn test_siblings() {
        setup();
        let parent = TestObject::new(1);
        let child1 = ChildObject::new("first");
        let child2 = ChildObject::new("second");
        let child3 = ChildObject::new("third");

        child1.base.set_parent(Some(parent.object_id())).unwrap();
        child2.base.set_parent(Some(parent.object_id())).unwrap();
        child3.base.set_parent(Some(parent.object_id())).unwrap();

        let siblings = child2.base.siblings();
        assert_eq!(siblings.len(), 2);
        assert!(siblings.contains(&child1.object_id()));
        assert!(siblings.contains(&child3.object_id()));
        assert!(!siblings.contains(&child2.object_id()));
    }

    // =========================================================================
    // Tree Traversal Tests
    // =========================================================================

    #[test]
    fn test_ancestors() {
        setup();
        let root = TestObject::new(1);
        let parent = ChildObject::new("parent");
        let child = ChildObject::new("child");

        parent.base.set_parent(Some(root.object_id())).unwrap();
        child.base.set_parent(Some(parent.object_id())).unwrap();

        let ancestors = child.base.ancestors();
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0], parent.object_id()); // Immediate parent first
        assert_eq!(ancestors[1], root.object_id()); // Then root
    }

    #[test]
    fn test_depth_first_preorder() {
        setup();
        let registry = global_registry().unwrap();

        // Build tree:
        //       root
        //      /    \
        //   child1  child2
        //     |
        //  grandchild
        let root_id = registry.register::<TestObject>();
        let child1_id = registry.register::<ChildObject>();
        let child2_id = registry.register::<ChildObject>();
        let grandchild_id = registry.register::<ChildObject>();

        registry.set_parent(child1_id, Some(root_id)).unwrap();
        registry.set_parent(child2_id, Some(root_id)).unwrap();
        registry.set_parent(grandchild_id, Some(child1_id)).unwrap();

        let preorder = registry.depth_first_preorder(root_id).unwrap();
        // Expected: root, child1, grandchild, child2
        assert_eq!(preorder.len(), 4);
        assert_eq!(preorder[0], root_id);
        assert_eq!(preorder[1], child1_id);
        assert_eq!(preorder[2], grandchild_id);
        assert_eq!(preorder[3], child2_id);
    }

    #[test]
    fn test_depth_first_postorder() {
        setup();
        let registry = global_registry().unwrap();

        // Same tree as preorder test
        let root_id = registry.register::<TestObject>();
        let child1_id = registry.register::<ChildObject>();
        let child2_id = registry.register::<ChildObject>();
        let grandchild_id = registry.register::<ChildObject>();

        registry.set_parent(child1_id, Some(root_id)).unwrap();
        registry.set_parent(child2_id, Some(root_id)).unwrap();
        registry.set_parent(grandchild_id, Some(child1_id)).unwrap();

        let postorder = registry.depth_first_postorder(root_id).unwrap();
        // Expected: grandchild, child1, child2, root
        assert_eq!(postorder.len(), 4);
        assert_eq!(postorder[0], grandchild_id);
        assert_eq!(postorder[1], child1_id);
        assert_eq!(postorder[2], child2_id);
        assert_eq!(postorder[3], root_id);
    }

    #[test]
    fn test_breadth_first() {
        setup();
        let registry = global_registry().unwrap();

        // Same tree as other tests
        let root_id = registry.register::<TestObject>();
        let child1_id = registry.register::<ChildObject>();
        let child2_id = registry.register::<ChildObject>();
        let grandchild_id = registry.register::<ChildObject>();

        registry.set_parent(child1_id, Some(root_id)).unwrap();
        registry.set_parent(child2_id, Some(root_id)).unwrap();
        registry.set_parent(grandchild_id, Some(child1_id)).unwrap();

        let bfs = registry.breadth_first(root_id).unwrap();
        // Expected: root, child1, child2, grandchild (level by level)
        assert_eq!(bfs.len(), 4);
        assert_eq!(bfs[0], root_id);
        assert_eq!(bfs[1], child1_id);
        assert_eq!(bfs[2], child2_id);
        assert_eq!(bfs[3], grandchild_id);
    }
}
