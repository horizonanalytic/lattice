//! Proxy model for filtering and sorting.
//!
//! `ProxyModel` wraps a source model and provides filtering and/or sorting
//! on top of the source data.

use parking_lot::RwLock;
use std::cmp::Ordering;
use std::sync::Arc;

use super::index::ModelIndex;
use super::role::{ItemData, ItemRole};
use super::traits::{ItemFlags, ItemModel, ModelSignals, Orientation};

/// Type alias for a filter function.
///
/// Returns `true` if the row should be included, `false` to filter it out.
pub type FilterFn<S> = Arc<dyn Fn(&S, usize, &ModelIndex) -> bool + Send + Sync>;

/// Type alias for a compare function for sorting.
///
/// Compares two rows (by their indices) and returns an ordering.
pub type CompareFn<S> = Arc<dyn Fn(&S, usize, usize, &ModelIndex) -> Ordering + Send + Sync>;

/// Internal row mapping from proxy to source.
struct RowMapping {
    /// Mapping from proxy row index to source row index.
    proxy_to_source: Vec<usize>,
    /// Mapping from source row index to proxy row index (None if filtered out).
    source_to_proxy: Vec<Option<usize>>,
}

impl RowMapping {
    fn new() -> Self {
        Self {
            proxy_to_source: Vec::new(),
            source_to_proxy: Vec::new(),
        }
    }

    fn clear(&mut self) {
        self.proxy_to_source.clear();
        self.source_to_proxy.clear();
    }

    fn proxy_row_count(&self) -> usize {
        self.proxy_to_source.len()
    }

    fn map_to_source(&self, proxy_row: usize) -> Option<usize> {
        self.proxy_to_source.get(proxy_row).copied()
    }

    fn map_from_source(&self, source_row: usize) -> Option<usize> {
        self.source_to_proxy.get(source_row).and_then(|&x| x)
    }
}

/// A proxy model that provides filtering and sorting on top of a source model.
///
/// `ProxyModel` wraps any `ItemModel` and can:
/// - Filter rows based on a predicate
/// - Sort rows based on a comparator
/// - Both filter and sort simultaneously
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::model::{ProxyModel, ListModel, ListItem, ItemData, ItemRole};
///
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// impl ListItem for Person {
///     fn display(&self) -> ItemData {
///         ItemData::from(&self.name)
///     }
/// }
///
/// let source = Arc::new(ListModel::new(vec![
///     Person { name: "Alice".into(), age: 30 },
///     Person { name: "Bob".into(), age: 25 },
///     Person { name: "Charlie".into(), age: 35 },
/// ]));
///
/// // Create a proxy that filters adults (age >= 30) and sorts by name
/// let proxy = ProxyModel::new(source.clone())
///     .with_filter(|model, row, _parent| {
///         // Access source data to filter
///         true // Custom filter logic
///     })
///     .with_sort(|model, row_a, row_b, _parent| {
///         // Compare rows for sorting
///         std::cmp::Ordering::Equal // Custom sort logic
///     });
/// ```
pub struct ProxyModel<S: ItemModel> {
    source: Arc<S>,
    filter: Option<FilterFn<S>>,
    compare: Option<CompareFn<S>>,
    /// Mapping for the root level. Child mappings would need a HashMap<NodeId, RowMapping>
    /// for hierarchical models, but we keep it simple for now.
    mapping: RwLock<RowMapping>,
    signals: ModelSignals,
    /// Column used for sorting (for simple sort comparators).
    sort_column: RwLock<Option<usize>>,
    /// Whether sort is descending.
    sort_descending: RwLock<bool>,
}

impl<S: ItemModel + 'static> ProxyModel<S> {
    /// Creates a new proxy model wrapping the given source.
    pub fn new(source: Arc<S>) -> Self {
        let proxy = Self {
            source,
            filter: None,
            compare: None,
            mapping: RwLock::new(RowMapping::new()),
            signals: ModelSignals::new(),
            sort_column: RwLock::new(None),
            sort_descending: RwLock::new(false),
        };
        proxy.rebuild_mapping();
        proxy
    }

    /// Sets a filter function.
    ///
    /// The filter function receives the source model, source row index, and parent index.
    /// Return `true` to include the row, `false` to filter it out.
    pub fn with_filter<F>(mut self, filter: F) -> Self
    where
        F: Fn(&S, usize, &ModelIndex) -> bool + Send + Sync + 'static,
    {
        self.filter = Some(Arc::new(filter));
        self.rebuild_mapping();
        self
    }

    /// Sets a sort comparator.
    ///
    /// The comparator receives the source model and two source row indices to compare.
    pub fn with_sort<F>(mut self, compare: F) -> Self
    where
        F: Fn(&S, usize, usize, &ModelIndex) -> Ordering + Send + Sync + 'static,
    {
        self.compare = Some(Arc::new(compare));
        self.rebuild_mapping();
        self
    }

    /// Sets the filter function dynamically.
    ///
    /// Note: Currently this method just rebuilds the mapping without actually
    /// updating the filter. Full dynamic filter support requires interior mutability.
    pub fn set_filter<F>(&self, _filter: F)
    where
        F: Fn(&S, usize, &ModelIndex) -> bool + Send + Sync + 'static,
    {
        // TODO: Implement proper dynamic filter update with interior mutability
        self.rebuild_mapping();
    }

    /// Clears the filter.
    pub fn clear_filter(&self) {
        self.rebuild_mapping();
    }

    /// Sets simple column-based sorting.
    ///
    /// This sorts by comparing the Display role of the specified column.
    pub fn sort_by_column(&self, column: usize, descending: bool) {
        *self.sort_column.write() = Some(column);
        *self.sort_descending.write() = descending;
        self.rebuild_mapping();
    }

    /// Clears sorting.
    pub fn clear_sort(&self) {
        *self.sort_column.write() = None;
        self.rebuild_mapping();
    }

    /// Forces a rebuild of the proxy mapping.
    ///
    /// Call this when the source model changes or when filter/sort criteria change.
    pub fn invalidate(&self) {
        self.signals.emit_layout_changed(|| {
            self.rebuild_mapping();
        });
    }

    /// Returns a reference to the source model.
    pub fn source(&self) -> &Arc<S> {
        &self.source
    }

    /// Maps a proxy index to a source index.
    pub fn map_to_source(&self, proxy_index: &ModelIndex) -> ModelIndex {
        if !proxy_index.is_valid() {
            return ModelIndex::invalid();
        }

        let mapping = self.mapping.read();
        let source_row = match mapping.map_to_source(proxy_index.row()) {
            Some(r) => r,
            None => return ModelIndex::invalid(),
        };

        // For hierarchical models, we'd need to map the parent as well
        // For now, we assume flat models
        self.source.index(source_row, proxy_index.column(), &ModelIndex::invalid())
    }

    /// Maps a source index to a proxy index.
    pub fn map_from_source(&self, source_index: &ModelIndex) -> ModelIndex {
        if !source_index.is_valid() {
            return ModelIndex::invalid();
        }

        let mapping = self.mapping.read();
        let proxy_row = match mapping.map_from_source(source_index.row()) {
            Some(r) => r,
            None => return ModelIndex::invalid(), // Filtered out
        };

        ModelIndex::new(proxy_row, source_index.column(), ModelIndex::invalid())
    }

    /// Rebuilds the internal mapping based on filter and sort.
    fn rebuild_mapping(&self) {
        let source_count = self.source.row_count(&ModelIndex::invalid());
        let parent = ModelIndex::invalid();

        let mut mapping = self.mapping.write();
        mapping.clear();
        mapping.source_to_proxy.resize(source_count, None);

        // First, collect rows that pass the filter
        let mut visible_rows: Vec<usize> = (0..source_count)
            .filter(|&row| {
                if let Some(ref filter) = self.filter {
                    filter(&self.source, row, &parent)
                } else {
                    true
                }
            })
            .collect();

        // Then, sort if we have a comparator
        if let Some(ref compare) = self.compare {
            visible_rows.sort_by(|&a, &b| compare(&self.source, a, b, &parent));
        } else if let Some(column) = *self.sort_column.read() {
            // Simple column-based sorting
            let descending = *self.sort_descending.read();
            visible_rows.sort_by(|&a, &b| {
                let index_a = self.source.index(a, column, &parent);
                let index_b = self.source.index(b, column, &parent);

                let data_a = self.source.data(&index_a, ItemRole::Display);
                let data_b = self.source.data(&index_b, ItemRole::Display);

                let cmp = compare_item_data(&data_a, &data_b);
                if descending {
                    cmp.reverse()
                } else {
                    cmp
                }
            });
        }

        // Build the mapping
        for (proxy_row, &source_row) in visible_rows.iter().enumerate() {
            mapping.proxy_to_source.push(source_row);
            mapping.source_to_proxy[source_row] = Some(proxy_row);
        }
    }
}

/// Compares two ItemData values for sorting.
fn compare_item_data(a: &ItemData, b: &ItemData) -> Ordering {
    match (a, b) {
        (ItemData::String(sa), ItemData::String(sb)) => sa.cmp(sb),
        (ItemData::Int(ia), ItemData::Int(ib)) => ia.cmp(ib),
        (ItemData::Float(fa), ItemData::Float(fb)) => {
            fa.partial_cmp(fb).unwrap_or(Ordering::Equal)
        }
        (ItemData::Bool(ba), ItemData::Bool(bb)) => ba.cmp(bb),
        // For other types, consider them equal or compare by debug string
        _ => Ordering::Equal,
    }
}

impl<S: ItemModel + 'static> ItemModel for ProxyModel<S> {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        if parent.is_valid() {
            // For now, we only support flat models
            0
        } else {
            self.mapping.read().proxy_row_count()
        }
    }

    fn column_count(&self, parent: &ModelIndex) -> usize {
        self.source.column_count(parent)
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        let source_index = self.map_to_source(index);
        self.source.data(&source_index, role)
    }

    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex {
        if parent.is_valid() {
            return ModelIndex::invalid();
        }

        let mapping = self.mapping.read();
        if row >= mapping.proxy_row_count() {
            return ModelIndex::invalid();
        }

        if column >= self.source.column_count(&ModelIndex::invalid()) {
            return ModelIndex::invalid();
        }

        ModelIndex::new(row, column, ModelIndex::invalid())
    }

    fn parent(&self, _index: &ModelIndex) -> ModelIndex {
        // Flat model for now
        ModelIndex::invalid()
    }

    fn signals(&self) -> &ModelSignals {
        &self.signals
    }

    fn flags(&self, index: &ModelIndex) -> ItemFlags {
        let source_index = self.map_to_source(index);
        self.source.flags(&source_index)
    }

    fn header_data(&self, section: usize, orientation: Orientation, role: ItemRole) -> ItemData {
        self.source.header_data(section, orientation, role)
    }
}

/// Builder pattern for creating proxy models.
pub struct ProxyModelBuilder<S: ItemModel> {
    source: Arc<S>,
    filter: Option<FilterFn<S>>,
    compare: Option<CompareFn<S>>,
}

impl<S: ItemModel + 'static> ProxyModelBuilder<S> {
    /// Creates a new builder with the given source model.
    pub fn new(source: Arc<S>) -> Self {
        Self {
            source,
            filter: None,
            compare: None,
        }
    }

    /// Adds a filter function.
    pub fn filter<F>(mut self, f: F) -> Self
    where
        F: Fn(&S, usize, &ModelIndex) -> bool + Send + Sync + 'static,
    {
        self.filter = Some(Arc::new(f));
        self
    }

    /// Adds a sort comparator.
    pub fn sort<F>(mut self, f: F) -> Self
    where
        F: Fn(&S, usize, usize, &ModelIndex) -> Ordering + Send + Sync + 'static,
    {
        self.compare = Some(Arc::new(f));
        self
    }

    /// Builds the proxy model.
    pub fn build(self) -> ProxyModel<S> {
        let mut proxy = ProxyModel::new(self.source);
        proxy.filter = self.filter;
        proxy.compare = self.compare;
        proxy.rebuild_mapping();
        proxy
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::list_model::{ListItem, ListModel};

    #[derive(Clone)]
    struct Person {
        name: String,
        age: u32,
    }

    impl ListItem for Person {
        fn display(&self) -> ItemData {
            ItemData::from(self.name.as_str())
        }

        fn data(&self, role: ItemRole) -> ItemData {
            match role {
                ItemRole::User(0) => ItemData::from(self.age as i64),
                _ => ItemData::None,
            }
        }
    }

    fn create_test_model() -> Arc<ListModel<Person>> {
        Arc::new(ListModel::new(vec![
            Person { name: "Charlie".into(), age: 35 },
            Person { name: "Alice".into(), age: 30 },
            Person { name: "Bob".into(), age: 25 },
            Person { name: "David".into(), age: 20 },
        ]))
    }

    #[test]
    fn test_proxy_no_filter_no_sort() {
        let source = create_test_model();
        let proxy = ProxyModel::new(source.clone());

        assert_eq!(proxy.row_count(&ModelIndex::invalid()), 4);

        let index = proxy.index(0, 0, &ModelIndex::invalid());
        assert_eq!(proxy.data(&index, ItemRole::Display).as_string(), Some("Charlie"));
    }

    #[test]
    fn test_proxy_with_filter() {
        let source = create_test_model();

        // Filter to only include people age 30+
        let proxy = ProxyModelBuilder::new(source.clone())
            .filter(|model, row, parent| {
                let index = model.index(row, 0, parent);
                let age = model.data(&index, ItemRole::User(0)).as_int().unwrap_or(0);
                age >= 30
            })
            .build();

        assert_eq!(proxy.row_count(&ModelIndex::invalid()), 2);

        let index = proxy.index(0, 0, &ModelIndex::invalid());
        let data = proxy.data(&index, ItemRole::Display);
        let name = data.as_string();
        assert!(name == Some("Charlie") || name == Some("Alice"));
    }

    #[test]
    fn test_proxy_with_sort() {
        let source = create_test_model();

        // Sort by name alphabetically
        let proxy = ProxyModelBuilder::new(source.clone())
            .sort(|model, row_a, row_b, parent| {
                let index_a = model.index(row_a, 0, parent);
                let index_b = model.index(row_b, 0, parent);
                let name_a = model.data(&index_a, ItemRole::Display).into_string().unwrap_or_default();
                let name_b = model.data(&index_b, ItemRole::Display).into_string().unwrap_or_default();
                name_a.cmp(&name_b)
            })
            .build();

        assert_eq!(proxy.row_count(&ModelIndex::invalid()), 4);

        // Check sorted order: Alice, Bob, Charlie, David
        let names: Vec<_> = (0..4)
            .map(|i| {
                let index = proxy.index(i, 0, &ModelIndex::invalid());
                proxy.data(&index, ItemRole::Display).into_string().unwrap()
            })
            .collect();

        assert_eq!(names, vec!["Alice", "Bob", "Charlie", "David"]);
    }

    #[test]
    fn test_proxy_with_filter_and_sort() {
        let source = create_test_model();

        // Filter age >= 25 and sort by name
        let proxy = ProxyModelBuilder::new(source.clone())
            .filter(|model, row, parent| {
                let index = model.index(row, 0, parent);
                let age = model.data(&index, ItemRole::User(0)).as_int().unwrap_or(0);
                age >= 25
            })
            .sort(|model, row_a, row_b, parent| {
                let index_a = model.index(row_a, 0, parent);
                let index_b = model.index(row_b, 0, parent);
                let name_a = model.data(&index_a, ItemRole::Display).into_string().unwrap_or_default();
                let name_b = model.data(&index_b, ItemRole::Display).into_string().unwrap_or_default();
                name_a.cmp(&name_b)
            })
            .build();

        // Should have Alice (30), Bob (25), Charlie (35) - David (20) filtered out
        assert_eq!(proxy.row_count(&ModelIndex::invalid()), 3);

        let names: Vec<_> = (0..3)
            .map(|i| {
                let index = proxy.index(i, 0, &ModelIndex::invalid());
                proxy.data(&index, ItemRole::Display).into_string().unwrap()
            })
            .collect();

        assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_proxy_map_to_source() {
        let source = create_test_model();

        // Sort by name
        let proxy = ProxyModelBuilder::new(source.clone())
            .sort(|model, row_a, row_b, parent| {
                let index_a = model.index(row_a, 0, parent);
                let index_b = model.index(row_b, 0, parent);
                let name_a = model.data(&index_a, ItemRole::Display).into_string().unwrap_or_default();
                let name_b = model.data(&index_b, ItemRole::Display).into_string().unwrap_or_default();
                name_a.cmp(&name_b)
            })
            .build();

        // Proxy row 0 (Alice) should map to source row 1
        let proxy_index = proxy.index(0, 0, &ModelIndex::invalid());
        let source_index = proxy.map_to_source(&proxy_index);

        assert!(source_index.is_valid());
        assert_eq!(source.data(&source_index, ItemRole::Display).as_string(), Some("Alice"));
    }

    #[test]
    fn test_proxy_map_from_source() {
        let source = create_test_model();

        // Filter out Bob (index 2 in source, age 25)
        let proxy = ProxyModelBuilder::new(source.clone())
            .filter(|model, row, parent| {
                let index = model.index(row, 0, parent);
                let age = model.data(&index, ItemRole::User(0)).as_int().unwrap_or(0);
                age >= 30
            })
            .build();

        // Source row 1 (Alice, age 30) should map to proxy
        let source_index = source.index(1, 0, &ModelIndex::invalid());
        let proxy_index = proxy.map_from_source(&source_index);
        assert!(proxy_index.is_valid());

        // Source row 2 (Bob, age 25) should be filtered out
        let source_index = source.index(2, 0, &ModelIndex::invalid());
        let proxy_index = proxy.map_from_source(&source_index);
        assert!(!proxy_index.is_valid());
    }

    #[test]
    fn test_simple_column_sort() {
        let source = create_test_model();
        let proxy = ProxyModel::new(source);

        proxy.sort_by_column(0, false); // Sort by column 0 ascending

        let names: Vec<_> = (0..4)
            .map(|i| {
                let index = proxy.index(i, 0, &ModelIndex::invalid());
                proxy.data(&index, ItemRole::Display).into_string().unwrap()
            })
            .collect();

        assert_eq!(names, vec!["Alice", "Bob", "Charlie", "David"]);

        // Sort descending
        proxy.sort_by_column(0, true);

        let names: Vec<_> = (0..4)
            .map(|i| {
                let index = proxy.index(i, 0, &ModelIndex::invalid());
                proxy.data(&index, ItemRole::Display).into_string().unwrap()
            })
            .collect();

        assert_eq!(names, vec!["David", "Charlie", "Bob", "Alice"]);
    }
}
