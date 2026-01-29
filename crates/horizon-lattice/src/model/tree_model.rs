//! Hierarchical tree model implementation.
//!
//! `TreeModel` provides a way to display hierarchical data with parent-child relationships.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::index::ModelIndex;
use super::role::{ItemData, ItemRole};
use super::traits::{ItemFlags, ItemModel, ModelSignals};

/// A node ID for internal tracking.
type NodeId = u64;

/// Counter for generating unique node IDs.
static NODE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_node_id() -> NodeId {
    NODE_ID_COUNTER.fetch_add(1, Ordering::Relaxed)
}

/// Trait for tree node data that provides its own display information.
///
/// Implement this trait for types that should be directly usable as tree nodes.
pub trait TreeNodeData: Send + Sync {
    /// Returns the primary display text for this node.
    fn display(&self) -> ItemData;

    /// Returns the decoration (icon) for this node.
    fn decoration(&self) -> ItemData {
        ItemData::None
    }

    /// Returns the tooltip text for this node.
    fn tooltip(&self) -> ItemData {
        ItemData::None
    }

    /// Returns data for a specific role.
    fn data(&self, _role: ItemRole) -> ItemData {
        ItemData::None
    }

    /// Returns the flags for this node.
    fn flags(&self) -> ItemFlags {
        ItemFlags::new()
    }
}

/// Implement TreeNodeData for String for convenience.
impl TreeNodeData for String {
    fn display(&self) -> ItemData {
        ItemData::from(self.as_str())
    }
}

/// A node in the tree structure.
struct TreeNode<T> {
    id: NodeId,
    data: T,
    children: Vec<NodeId>,
    parent: Option<NodeId>,
}

impl<T> TreeNode<T> {
    fn new(data: T, parent: Option<NodeId>) -> Self {
        Self {
            id: next_node_id(),
            data,
            children: Vec::new(),
            parent,
        }
    }
}

/// Internal storage for tree nodes.
struct TreeStorage<T> {
    nodes: HashMap<NodeId, TreeNode<T>>,
    root_children: Vec<NodeId>,
}

impl<T> TreeStorage<T> {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            root_children: Vec::new(),
        }
    }

    fn get_node(&self, id: NodeId) -> Option<&TreeNode<T>> {
        self.nodes.get(&id)
    }

    fn get_node_mut(&mut self, id: NodeId) -> Option<&mut TreeNode<T>> {
        self.nodes.get_mut(&id)
    }

    fn add_root(&mut self, data: T) -> NodeId {
        let node = TreeNode::new(data, None);
        let id = node.id;
        self.nodes.insert(id, node);
        self.root_children.push(id);
        id
    }

    fn add_child(&mut self, parent_id: NodeId, data: T) -> Option<NodeId> {
        let node = TreeNode::new(data, Some(parent_id));
        let id = node.id;
        self.nodes.insert(id, node);

        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.push(id);
            Some(id)
        } else {
            self.nodes.remove(&id);
            None
        }
    }

    fn remove_node(&mut self, id: NodeId) -> Option<T> {
        // First, remove from parent's children list
        if let Some(node) = self.nodes.get(&id) {
            if let Some(parent_id) = node.parent {
                if let Some(parent) = self.nodes.get_mut(&parent_id) {
                    parent.children.retain(|&child_id| child_id != id);
                }
            } else {
                // It's a root node
                self.root_children.retain(|&child_id| child_id != id);
            }
        }

        // Remove the node and all its descendants
        self.remove_subtree(id)
    }

    fn remove_subtree(&mut self, id: NodeId) -> Option<T> {
        let node = self.nodes.remove(&id)?;

        // Recursively remove children
        for child_id in node.children {
            self.remove_subtree(child_id);
        }

        Some(node.data)
    }

    fn children_of(&self, parent_id: Option<NodeId>) -> &[NodeId] {
        match parent_id {
            None => &self.root_children,
            Some(id) => self
                .nodes
                .get(&id)
                .map(|n| n.children.as_slice())
                .unwrap_or(&[]),
        }
    }

    fn child_count(&self, parent_id: Option<NodeId>) -> usize {
        self.children_of(parent_id).len()
    }

    fn child_at(&self, parent_id: Option<NodeId>, index: usize) -> Option<NodeId> {
        self.children_of(parent_id).get(index).copied()
    }

    fn parent_of(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(&id).and_then(|n| n.parent)
    }

    fn row_of(&self, id: NodeId) -> Option<usize> {
        let parent_id = self.parent_of(id);
        let siblings = self.children_of(parent_id);
        siblings.iter().position(|&child_id| child_id == id)
    }
}

/// Type alias for a tree data extractor function.
pub type TreeDataExtractor<T> = Arc<dyn Fn(&T, ItemRole) -> ItemData + Send + Sync>;

/// A hierarchical tree model for displaying parent-child data.
///
/// `TreeModel` stores data in a tree structure where each node can have
/// multiple children. It supports both trait-based and closure-based
/// approaches for data extraction.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::model::{TreeModel, TreeNodeData, ItemData};
///
/// #[derive(Clone)]
/// struct FileNode {
///     name: String,
///     is_directory: bool,
/// }
///
/// impl TreeNodeData for FileNode {
///     fn display(&self) -> ItemData {
///         ItemData::from(&self.name)
///     }
/// }
///
/// let mut model = TreeModel::<FileNode>::new();
///
/// let root = model.add_root(FileNode {
///     name: "Documents".into(),
///     is_directory: true,
/// });
///
/// model.add_child(root, FileNode {
///     name: "file.txt".into(),
///     is_directory: false,
/// });
/// ```
pub struct TreeModel<T> {
    storage: RwLock<TreeStorage<T>>,
    column_count: usize,
    extractor: Option<TreeDataExtractor<T>>,
    signals: ModelSignals,
}

impl<T: TreeNodeData + 'static> TreeModel<T> {
    /// Creates a new empty tree model.
    pub fn new() -> Self {
        Self {
            storage: RwLock::new(TreeStorage::new()),
            column_count: 1,
            extractor: None,
            signals: ModelSignals::new(),
        }
    }
}

impl<T: Send + Sync + 'static> TreeModel<T> {
    /// Creates a new tree model with a custom data extractor.
    pub fn with_extractor<F>(extractor: F) -> Self
    where
        F: Fn(&T, ItemRole) -> ItemData + Send + Sync + 'static,
    {
        Self {
            storage: RwLock::new(TreeStorage::new()),
            column_count: 1,
            extractor: Some(Arc::new(extractor)),
            signals: ModelSignals::new(),
        }
    }

    /// Sets the number of columns.
    pub fn set_column_count(&mut self, count: usize) {
        self.column_count = count;
    }

    /// Returns the number of columns.
    pub fn column_count_value(&self) -> usize {
        self.column_count
    }

    /// Adds a root-level node and returns its ID.
    pub fn add_root(&self, data: T) -> NodeId {
        let id;
        let row;
        {
            let mut storage = self.storage.write();
            row = storage.root_children.len();
            id = storage.add_root(data);
        }
        self.signals
            .rows_inserted
            .emit((ModelIndex::invalid(), row, row));
        id
    }

    /// Adds a child node to the specified parent and returns its ID.
    ///
    /// Returns `None` if the parent doesn't exist.
    pub fn add_child(&self, parent_id: NodeId, data: T) -> Option<NodeId> {
        let id;
        let row;
        let parent_index;
        {
            let mut storage = self.storage.write();

            // Get the parent's row before adding
            row = storage
                .nodes
                .get(&parent_id)
                .map(|n| n.children.len())
                .unwrap_or(0);

            // Create parent index
            parent_index = self.create_index_for_id(&storage, parent_id)?;

            id = storage.add_child(parent_id, data)?;
        }
        self.signals.rows_inserted.emit((parent_index, row, row));
        Some(id)
    }

    /// Removes a node and all its descendants.
    ///
    /// Returns the removed node's data, or `None` if the node doesn't exist.
    pub fn remove(&self, id: NodeId) -> Option<T> {
        let row;
        let parent_index;
        {
            let storage = self.storage.read();
            row = storage.row_of(id)?;
            let parent_id = storage.parent_of(id);
            parent_index = match parent_id {
                Some(pid) => self.create_index_for_id(&storage, pid)?,
                None => ModelIndex::invalid(),
            };
        }

        self.signals
            .rows_about_to_be_removed
            .emit((parent_index.clone(), row, row));
        let result = self.storage.write().remove_node(id);
        self.signals.rows_removed.emit((parent_index, row, row));
        result
    }

    /// Clears all nodes from the tree.
    pub fn clear(&self) {
        self.signals.emit_reset(|| {
            let mut storage = self.storage.write();
            storage.nodes.clear();
            storage.root_children.clear();
        });
    }

    /// Returns the number of root-level nodes.
    pub fn root_count(&self) -> usize {
        self.storage.read().root_children.len()
    }

    /// Returns `true` if the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.storage.read().root_children.is_empty()
    }

    /// Provides read access to a node's data.
    pub fn with_node<F, R>(&self, id: NodeId, f: F) -> Option<R>
    where
        F: FnOnce(&T) -> R,
    {
        let storage = self.storage.read();
        storage.get_node(id).map(|node| f(&node.data))
    }

    /// Provides mutable access to a node's data.
    ///
    /// Emits `data_changed` signal after modification.
    pub fn modify_node<F, R>(&self, id: NodeId, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let result;
        let index;
        {
            let mut storage = self.storage.write();
            let node = storage.get_node_mut(id)?;
            result = f(&mut node.data);
            index = self.create_index_for_id(&storage, id)?;
        }
        self.signals
            .emit_data_changed_single(index, vec![ItemRole::Display]);
        Some(result)
    }

    /// Creates a ModelIndex for a node ID.
    fn create_index_for_id(&self, storage: &TreeStorage<T>, id: NodeId) -> Option<ModelIndex> {
        let row = storage.row_of(id)?;
        let parent_id = storage.parent_of(id);
        let parent_index = match parent_id {
            Some(pid) => self.create_index_for_id(storage, pid)?,
            None => ModelIndex::invalid(),
        };
        Some(ModelIndex::with_internal_id(row, 0, parent_index, id))
    }

    /// Gets the node ID from a ModelIndex.
    fn node_id_from_index(&self, index: &ModelIndex) -> Option<NodeId> {
        if !index.is_valid() {
            return None;
        }
        Some(index.internal_id())
    }

    /// Gets the parent node ID from a ModelIndex.
    fn parent_node_id(&self, parent: &ModelIndex) -> Option<NodeId> {
        if parent.is_valid() {
            Some(parent.internal_id())
        } else {
            None
        }
    }
}

impl<T: TreeNodeData + 'static> Default for TreeModel<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: TreeNodeData + 'static> ItemModel for TreeModel<T> {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        let storage = self.storage.read();
        let parent_id = self.parent_node_id(parent);
        storage.child_count(parent_id)
    }

    fn column_count(&self, _parent: &ModelIndex) -> usize {
        self.column_count
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        if !index.is_valid() {
            return ItemData::None;
        }

        let storage = self.storage.read();
        let node_id = index.internal_id();

        let node = match storage.get_node(node_id) {
            Some(n) => n,
            None => return ItemData::None,
        };

        // Use extractor if available
        if let Some(ref extractor) = self.extractor {
            return extractor(&node.data, role);
        }

        // Otherwise use TreeNodeData trait
        match role {
            ItemRole::Display => node.data.display(),
            ItemRole::Decoration => node.data.decoration(),
            ItemRole::ToolTip => node.data.tooltip(),
            _ => node.data.data(role),
        }
    }

    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex {
        if column >= self.column_count {
            return ModelIndex::invalid();
        }

        let storage = self.storage.read();
        let parent_id = self.parent_node_id(parent);

        let child_id = match storage.child_at(parent_id, row) {
            Some(id) => id,
            None => return ModelIndex::invalid(),
        };

        ModelIndex::with_internal_id(row, column, parent.clone(), child_id)
    }

    fn parent(&self, index: &ModelIndex) -> ModelIndex {
        if !index.is_valid() {
            return ModelIndex::invalid();
        }

        let storage = self.storage.read();
        let node_id = index.internal_id();

        let parent_id = match storage.parent_of(node_id) {
            Some(id) => id,
            None => return ModelIndex::invalid(),
        };

        // Find the parent's row in its parent's children
        let grandparent_id = storage.parent_of(parent_id);
        let parent_row = match storage.row_of(parent_id) {
            Some(r) => r,
            None => return ModelIndex::invalid(),
        };

        let grandparent_index = match grandparent_id {
            Some(gid) => {
                // Recursively build the index chain using create_index_for_id
                self.create_index_for_id(&storage, gid)
                    .unwrap_or_else(ModelIndex::invalid)
            }
            None => ModelIndex::invalid(),
        };

        ModelIndex::with_internal_id(parent_row, 0, grandparent_index, parent_id)
    }

    fn signals(&self) -> &ModelSignals {
        &self.signals
    }

    fn flags(&self, index: &ModelIndex) -> ItemFlags {
        if !index.is_valid() {
            return ItemFlags::disabled();
        }

        if self.extractor.is_some() {
            return ItemFlags::new();
        }

        let storage = self.storage.read();
        let node_id = index.internal_id();

        storage
            .get_node(node_id)
            .map(|n| n.data.flags())
            .unwrap_or_else(ItemFlags::disabled)
    }

    fn has_children(&self, parent: &ModelIndex) -> bool {
        self.row_count(parent) > 0
    }
}

/// Extractor-based tree model for when T doesn't implement TreeNodeData.
pub struct ExtractorTreeModel<T> {
    inner: TreeModel<T>,
}

impl<T: Send + Sync + 'static> ExtractorTreeModel<T> {
    /// Creates a new tree model with a data extractor.
    pub fn new<F>(extractor: F) -> Self
    where
        F: Fn(&T, ItemRole) -> ItemData + Send + Sync + 'static,
    {
        Self {
            inner: TreeModel::with_extractor(extractor),
        }
    }

    /// Adds a root-level node.
    pub fn add_root(&self, data: T) -> NodeId {
        self.inner.add_root(data)
    }

    /// Adds a child node.
    pub fn add_child(&self, parent_id: NodeId, data: T) -> Option<NodeId> {
        self.inner.add_child(parent_id, data)
    }

    /// Removes a node.
    pub fn remove(&self, id: NodeId) -> Option<T> {
        self.inner.remove(id)
    }

    /// Clears all nodes.
    pub fn clear(&self) {
        self.inner.clear()
    }

    /// Returns the inner model.
    pub fn inner(&self) -> &TreeModel<T> {
        &self.inner
    }
}

impl<T: Send + Sync + 'static> ItemModel for ExtractorTreeModel<T> {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        let storage = self.inner.storage.read();
        let parent_id = self.inner.parent_node_id(parent);
        storage.child_count(parent_id)
    }

    fn column_count(&self, _parent: &ModelIndex) -> usize {
        self.inner.column_count
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        if !index.is_valid() {
            return ItemData::None;
        }

        let storage = self.inner.storage.read();
        let node_id = index.internal_id();

        let node = match storage.get_node(node_id) {
            Some(n) => n,
            None => return ItemData::None,
        };

        if let Some(ref extractor) = self.inner.extractor {
            extractor(&node.data, role)
        } else {
            ItemData::None
        }
    }

    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex {
        if column >= self.inner.column_count {
            return ModelIndex::invalid();
        }

        let storage = self.inner.storage.read();
        let parent_id = self.inner.parent_node_id(parent);

        let child_id = match storage.child_at(parent_id, row) {
            Some(id) => id,
            None => return ModelIndex::invalid(),
        };

        ModelIndex::with_internal_id(row, column, parent.clone(), child_id)
    }

    fn parent(&self, index: &ModelIndex) -> ModelIndex {
        if !index.is_valid() {
            return ModelIndex::invalid();
        }

        let storage = self.inner.storage.read();
        let node_id = index.internal_id();

        let parent_id = match storage.parent_of(node_id) {
            Some(id) => id,
            None => return ModelIndex::invalid(),
        };

        let parent_row = match storage.row_of(parent_id) {
            Some(r) => r,
            None => return ModelIndex::invalid(),
        };

        let grandparent_id = storage.parent_of(parent_id);
        let grandparent_index = match grandparent_id {
            Some(gid) => self
                .inner
                .create_index_for_id(&storage, gid)
                .unwrap_or_else(ModelIndex::invalid),
            None => ModelIndex::invalid(),
        };

        ModelIndex::with_internal_id(parent_row, 0, grandparent_index, parent_id)
    }

    fn signals(&self) -> &ModelSignals {
        &self.inner.signals
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct FileNode {
        name: String,
        is_dir: bool,
    }

    impl TreeNodeData for FileNode {
        fn display(&self) -> ItemData {
            ItemData::from(self.name.as_str())
        }

        fn tooltip(&self) -> ItemData {
            if self.is_dir {
                ItemData::from("Directory")
            } else {
                ItemData::from("File")
            }
        }
    }

    #[test]
    fn test_tree_model_basic() {
        let model = TreeModel::<FileNode>::new();

        let _root = model.add_root(FileNode {
            name: "Documents".into(),
            is_dir: true,
        });

        assert_eq!(model.root_count(), 1);
        assert_eq!(model.row_count(&ModelIndex::invalid()), 1);

        let index = model.index(0, 0, &ModelIndex::invalid());
        assert!(index.is_valid());
        assert_eq!(
            model.data(&index, ItemRole::Display).as_string(),
            Some("Documents")
        );
    }

    #[test]
    fn test_tree_model_hierarchy() {
        let model = TreeModel::<FileNode>::new();

        let root = model.add_root(FileNode {
            name: "Root".into(),
            is_dir: true,
        });

        let child1 = model
            .add_child(
                root,
                FileNode {
                    name: "Child1".into(),
                    is_dir: true,
                },
            )
            .unwrap();

        model
            .add_child(
                root,
                FileNode {
                    name: "Child2".into(),
                    is_dir: false,
                },
            )
            .unwrap();

        model
            .add_child(
                child1,
                FileNode {
                    name: "Grandchild".into(),
                    is_dir: false,
                },
            )
            .unwrap();

        // Check root
        assert_eq!(model.row_count(&ModelIndex::invalid()), 1);

        // Get root index
        let root_index = model.index(0, 0, &ModelIndex::invalid());
        assert!(root_index.is_valid());

        // Check root's children
        assert_eq!(model.row_count(&root_index), 2);

        // Get first child
        let child1_index = model.index(0, 0, &root_index);
        assert!(child1_index.is_valid());
        assert_eq!(
            model.data(&child1_index, ItemRole::Display).as_string(),
            Some("Child1")
        );

        // Check first child's children
        assert_eq!(model.row_count(&child1_index), 1);

        // Get grandchild
        let grandchild_index = model.index(0, 0, &child1_index);
        assert!(grandchild_index.is_valid());
        assert_eq!(
            model.data(&grandchild_index, ItemRole::Display).as_string(),
            Some("Grandchild")
        );

        // Check parent relationship
        let parent = model.parent(&grandchild_index);
        assert!(parent.is_valid());
        assert_eq!(
            model.data(&parent, ItemRole::Display).as_string(),
            Some("Child1")
        );
    }

    #[test]
    fn test_tree_model_remove() {
        let model = TreeModel::<FileNode>::new();

        let root = model.add_root(FileNode {
            name: "Root".into(),
            is_dir: true,
        });

        let child = model
            .add_child(
                root,
                FileNode {
                    name: "Child".into(),
                    is_dir: false,
                },
            )
            .unwrap();

        assert_eq!(model.row_count(&ModelIndex::invalid()), 1);

        let root_index = model.index(0, 0, &ModelIndex::invalid());
        assert_eq!(model.row_count(&root_index), 1);

        model.remove(child);
        assert_eq!(model.row_count(&root_index), 0);
    }

    #[test]
    fn test_tree_model_clear() {
        let model = TreeModel::<FileNode>::new();

        model.add_root(FileNode {
            name: "Root1".into(),
            is_dir: true,
        });
        model.add_root(FileNode {
            name: "Root2".into(),
            is_dir: true,
        });

        assert_eq!(model.root_count(), 2);

        model.clear();
        assert!(model.is_empty());
        assert_eq!(model.root_count(), 0);
    }

    #[test]
    fn test_extractor_tree_model() {
        struct Node {
            label: String,
            value: i32,
        }

        let model = ExtractorTreeModel::new(|node: &Node, role| match role {
            ItemRole::Display => ItemData::from(node.label.as_str()),
            ItemRole::ToolTip => ItemData::from(format!("Value: {}", node.value)),
            _ => ItemData::None,
        });

        let root = model.add_root(Node {
            label: "Root".into(),
            value: 100,
        });

        model.add_child(
            root,
            Node {
                label: "Child".into(),
                value: 50,
            },
        );

        assert_eq!(model.row_count(&ModelIndex::invalid()), 1);

        let root_index = model.index(0, 0, &ModelIndex::invalid());
        assert_eq!(
            model.data(&root_index, ItemRole::Display).as_string(),
            Some("Root")
        );
        assert_eq!(
            model.data(&root_index, ItemRole::ToolTip).as_string(),
            Some("Value: 100")
        );
    }
}
