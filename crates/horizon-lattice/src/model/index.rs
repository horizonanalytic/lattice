//! Model index for addressing items in hierarchical models.
//!
//! The `ModelIndex` type is the fundamental way to reference items within
//! an `ItemModel`. It contains row, column, and parent information to
//! uniquely identify any item in a hierarchical data structure.

use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

/// A global counter for generating unique internal IDs.
static INTERNAL_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Represents a position within an `ItemModel`.
///
/// `ModelIndex` is used by views, delegates, and selection models to locate
/// items within a model. Each index contains:
/// - Row and column within the parent
/// - A reference to the parent index (for hierarchical models)
/// - An internal ID for model-specific identification
///
/// # Index Validity
///
/// Model indices should be used immediately and not stored long-term.
/// After model modifications (insertions, deletions, moves), previously
/// obtained indices may become invalid.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::model::ModelIndex;
///
/// // Get a root-level item
/// let index = model.index(0, 0, ModelIndex::invalid());
///
/// // Get a child item
/// let child = model.index(0, 0, index);
///
/// // Navigate to siblings
/// let sibling = index.sibling(1, 0);
/// ```
#[derive(Clone)]
pub struct ModelIndex {
    /// The row within the parent.
    row: usize,
    /// The column within the parent.
    column: usize,
    /// The parent index. `None` indicates a root-level item.
    parent: Option<Box<ModelIndex>>,
    /// An internal ID that models can use for their own purposes.
    /// This is typically a pointer to internal data or a unique identifier.
    internal_id: u64,
    /// Whether this index is valid.
    valid: bool,
}

impl Default for ModelIndex {
    fn default() -> Self {
        Self::invalid()
    }
}

impl ModelIndex {
    /// Creates an invalid (null) model index.
    ///
    /// An invalid index is used to represent:
    /// - The root of the model (as a parent reference)
    /// - A non-existent or out-of-bounds item
    /// - An uninitialized index
    ///
    /// # Example
    ///
    /// ```ignore
    /// let root = ModelIndex::invalid();
    /// assert!(!root.is_valid());
    /// ```
    #[inline]
    pub const fn invalid() -> Self {
        Self {
            row: 0,
            column: 0,
            parent: None,
            internal_id: 0,
            valid: false,
        }
    }

    /// Creates a new valid model index.
    ///
    /// This is typically called by model implementations via
    /// `create_index` methods rather than directly.
    ///
    /// # Arguments
    ///
    /// * `row` - The row within the parent
    /// * `column` - The column within the parent
    /// * `parent` - The parent index, or `ModelIndex::invalid()` for root items
    #[inline]
    pub fn new(row: usize, column: usize, parent: ModelIndex) -> Self {
        let internal_id = INTERNAL_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self {
            row,
            column,
            parent: if parent.is_valid() {
                Some(Box::new(parent))
            } else {
                None
            },
            internal_id,
            valid: true,
        }
    }

    /// Creates a new valid model index with a custom internal ID.
    ///
    /// Models can use the internal ID to store a pointer or identifier
    /// to their internal data structures for efficient lookups.
    ///
    /// # Arguments
    ///
    /// * `row` - The row within the parent
    /// * `column` - The column within the parent
    /// * `parent` - The parent index
    /// * `internal_id` - Model-specific identifier
    #[inline]
    pub fn with_internal_id(
        row: usize,
        column: usize,
        parent: ModelIndex,
        internal_id: u64,
    ) -> Self {
        Self {
            row,
            column,
            parent: if parent.is_valid() {
                Some(Box::new(parent))
            } else {
                None
            },
            internal_id,
            valid: true,
        }
    }

    /// Returns `true` if this is a valid index.
    ///
    /// Invalid indices are returned when:
    /// - Requesting an out-of-bounds item
    /// - Using `ModelIndex::invalid()`
    /// - Referencing the root (which has no index)
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Returns the row of this index within its parent.
    ///
    /// Returns 0 for invalid indices.
    #[inline]
    pub fn row(&self) -> usize {
        self.row
    }

    /// Returns the column of this index within its parent.
    ///
    /// Returns 0 for invalid indices.
    #[inline]
    pub fn column(&self) -> usize {
        self.column
    }

    /// Returns the parent index, or an invalid index if this is a root item.
    #[inline]
    pub fn parent(&self) -> ModelIndex {
        match &self.parent {
            Some(parent) => (**parent).clone(),
            None => ModelIndex::invalid(),
        }
    }

    /// Returns `true` if this index has a valid parent.
    ///
    /// Root-level items have no parent.
    #[inline]
    pub fn has_parent(&self) -> bool {
        self.parent.is_some()
    }

    /// Returns the internal ID associated with this index.
    ///
    /// The meaning of this ID is model-specific. It can be used to
    /// store a pointer to internal data or a unique identifier.
    #[inline]
    pub fn internal_id(&self) -> u64 {
        self.internal_id
    }

    /// Creates a sibling index at the given row and column.
    ///
    /// This is equivalent to getting the index at `(row, column)` with
    /// the same parent as this index.
    ///
    /// Returns an invalid index if this index is invalid.
    ///
    /// Note: This creates the index structure but doesn't validate
    /// against a model. Use with model methods for validation.
    #[inline]
    pub fn sibling(&self, row: usize, column: usize) -> ModelIndex {
        if !self.is_valid() {
            return ModelIndex::invalid();
        }
        ModelIndex::new(row, column, self.parent())
    }

    /// Creates a sibling at the same column but different row.
    ///
    /// Convenience method for `sibling(row, self.column())`.
    #[inline]
    pub fn sibling_at_row(&self, row: usize) -> ModelIndex {
        self.sibling(row, self.column)
    }

    /// Creates a sibling at the same row but different column.
    ///
    /// Convenience method for `sibling(self.row(), column)`.
    #[inline]
    pub fn sibling_at_column(&self, column: usize) -> ModelIndex {
        self.sibling(self.row, column)
    }

    /// Returns the depth of this index in the tree hierarchy.
    ///
    /// Root-level items have depth 0. Returns 0 for invalid indices.
    pub fn depth(&self) -> usize {
        if !self.is_valid() {
            return 0;
        }
        let mut depth = 0;
        let mut current = self.parent();
        while current.is_valid() {
            depth += 1;
            current = current.parent();
        }
        depth
    }

    /// Returns the chain of ancestors from this index up to (but not including) the root.
    ///
    /// The first element is the immediate parent, and the last is the
    /// top-level ancestor.
    pub fn ancestors(&self) -> Vec<ModelIndex> {
        let mut ancestors = Vec::new();
        let mut current = self.parent();
        while current.is_valid() {
            ancestors.push(current.clone());
            current = current.parent();
        }
        ancestors
    }

    /// Checks if this index is a descendant of the given ancestor.
    ///
    /// Returns `false` if either index is invalid or if `ancestor` is not
    /// actually an ancestor of this index.
    pub fn is_descendant_of(&self, ancestor: &ModelIndex) -> bool {
        if !self.is_valid() || !ancestor.is_valid() {
            return false;
        }
        let mut current = self.parent();
        while current.is_valid() {
            if current == *ancestor {
                return true;
            }
            current = current.parent();
        }
        false
    }
}

impl std::fmt::Debug for ModelIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_valid() {
            f.debug_struct("ModelIndex")
                .field("row", &self.row)
                .field("column", &self.column)
                .field("depth", &self.depth())
                .field("internal_id", &self.internal_id)
                .finish()
        } else {
            write!(f, "ModelIndex(invalid)")
        }
    }
}

impl PartialEq for ModelIndex {
    fn eq(&self, other: &Self) -> bool {
        // Two invalid indices are equal
        if !self.is_valid() && !other.is_valid() {
            return true;
        }
        // One valid, one invalid are not equal
        if self.is_valid() != other.is_valid() {
            return false;
        }
        // Both valid: compare position and parent
        self.row == other.row
            && self.column == other.column
            && self.parent == other.parent
            && self.internal_id == other.internal_id
    }
}

impl Eq for ModelIndex {}

impl Hash for ModelIndex {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.valid.hash(state);
        if self.valid {
            self.row.hash(state);
            self.column.hash(state);
            self.internal_id.hash(state);
            // Parent is implicitly encoded in internal_id for uniqueness
        }
    }
}

impl PartialOrd for ModelIndex {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ModelIndex {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Invalid indices sort before valid ones
        match (self.is_valid(), other.is_valid()) {
            (false, false) => std::cmp::Ordering::Equal,
            (false, true) => std::cmp::Ordering::Less,
            (true, false) => std::cmp::Ordering::Greater,
            (true, true) => {
                // Compare by depth first (shallower items first)
                let depth_cmp = self.depth().cmp(&other.depth());
                if depth_cmp != std::cmp::Ordering::Equal {
                    return depth_cmp;
                }
                // Then by row, then by column
                match self.row.cmp(&other.row) {
                    std::cmp::Ordering::Equal => self.column.cmp(&other.column),
                    other => other,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_index() {
        let index = ModelIndex::invalid();
        assert!(!index.is_valid());
        assert_eq!(index.row(), 0);
        assert_eq!(index.column(), 0);
        assert!(!index.has_parent());
    }

    #[test]
    fn test_valid_index() {
        let parent = ModelIndex::invalid();
        let index = ModelIndex::new(5, 3, parent);
        assert!(index.is_valid());
        assert_eq!(index.row(), 5);
        assert_eq!(index.column(), 3);
        assert!(!index.has_parent());
    }

    #[test]
    fn test_hierarchical_index() {
        let root = ModelIndex::invalid();
        let parent = ModelIndex::new(0, 0, root);
        let child = ModelIndex::new(2, 1, parent.clone());

        assert!(child.is_valid());
        assert!(child.has_parent());
        assert_eq!(child.parent().row(), 0);
        assert_eq!(child.parent().column(), 0);
        assert_eq!(child.depth(), 1);
    }

    #[test]
    fn test_sibling() {
        let root = ModelIndex::invalid();
        let index = ModelIndex::new(1, 0, root);
        let sibling = index.sibling(2, 0);

        assert!(sibling.is_valid());
        assert_eq!(sibling.row(), 2);
        assert_eq!(sibling.column(), 0);
    }

    #[test]
    fn test_equality() {
        // Two invalid indices are equal
        assert_eq!(ModelIndex::invalid(), ModelIndex::invalid());

        // Indices with same position and parent structure
        let root = ModelIndex::invalid();
        let idx1 = ModelIndex::with_internal_id(1, 0, root.clone(), 100);
        let idx2 = ModelIndex::with_internal_id(1, 0, root, 100);
        assert_eq!(idx1, idx2);
    }

    #[test]
    fn test_ancestors() {
        let root = ModelIndex::invalid();
        let level1 = ModelIndex::new(0, 0, root);
        let level2 = ModelIndex::new(1, 0, level1.clone());
        let level3 = ModelIndex::new(2, 0, level2.clone());

        let ancestors = level3.ancestors();
        assert_eq!(ancestors.len(), 2);
        assert_eq!(ancestors[0], level2);
        assert_eq!(ancestors[1], level1);
    }

    #[test]
    fn test_is_descendant_of() {
        let root = ModelIndex::invalid();
        let level1 = ModelIndex::new(0, 0, root);
        let level2 = ModelIndex::new(1, 0, level1.clone());
        let level3 = ModelIndex::new(2, 0, level2.clone());

        assert!(level3.is_descendant_of(&level2));
        assert!(level3.is_descendant_of(&level1));
        assert!(!level1.is_descendant_of(&level3));
        assert!(!level1.is_descendant_of(&level2));
    }

    #[test]
    fn test_ordering() {
        let root = ModelIndex::invalid();
        let idx1 = ModelIndex::new(0, 0, root.clone());
        let idx2 = ModelIndex::new(1, 0, root.clone());
        let idx3 = ModelIndex::new(0, 1, root);

        assert!(idx1 < idx2); // Row 0 < Row 1
        assert!(idx1 < idx3); // Same row, Column 0 < Column 1
        assert!(ModelIndex::invalid() < idx1); // Invalid < Valid
    }
}
