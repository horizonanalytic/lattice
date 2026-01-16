//! Generic list model implementation.
//!
//! `ListModel<T>` provides a simple way to display a list of items in views.
//! It supports both trait-based and closure-based approaches for data extraction.

use parking_lot::RwLock;
use std::sync::Arc;

use super::index::ModelIndex;
use super::role::{ItemData, ItemRole};
use super::traits::{ItemFlags, ItemModel, ModelSignals};

/// Trait for items that can provide their own display data.
///
/// Implement this trait for types that should be directly usable in a `ListModel`
/// without requiring external data extractors.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::model::{ListItem, ItemData};
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
/// ```
pub trait ListItem: Send + Sync {
    /// Returns the primary display text for this item.
    fn display(&self) -> ItemData;

    /// Returns the decoration (icon) for this item.
    fn decoration(&self) -> ItemData {
        ItemData::None
    }

    /// Returns the tooltip text for this item.
    fn tooltip(&self) -> ItemData {
        ItemData::None
    }

    /// Returns the edit value for this item.
    fn edit(&self) -> ItemData {
        self.display()
    }

    /// Returns data for a custom role.
    fn data(&self, _role: ItemRole) -> ItemData {
        ItemData::None
    }

    /// Returns the flags for this item.
    fn flags(&self) -> ItemFlags {
        ItemFlags::new()
    }
}

/// Implement ListItem for String for convenience.
impl ListItem for String {
    fn display(&self) -> ItemData {
        ItemData::from(self.as_str())
    }
}

/// Type alias for a data extractor function.
pub type DataExtractor<T> = Arc<dyn Fn(&T, ItemRole) -> ItemData + Send + Sync>;

/// Type alias for a flags extractor function.
pub type FlagsExtractor<T> = Arc<dyn Fn(&T) -> ItemFlags + Send + Sync>;

/// A generic list model for displaying a list of items.
///
/// `ListModel<T>` can be used in two ways:
///
/// 1. **Trait-based**: Items implement `ListItem` and provide their own data.
/// 2. **Closure-based**: A data extractor function is provided at construction.
///
/// # Example (Trait-based)
///
/// ```ignore
/// use horizon_lattice::model::{ListModel, ListItem, ItemData};
///
/// #[derive(Clone)]
/// struct Task {
///     title: String,
///     completed: bool,
/// }
///
/// impl ListItem for Task {
///     fn display(&self) -> ItemData {
///         ItemData::from(&self.title)
///     }
/// }
///
/// let model = ListModel::new(vec![
///     Task { title: "Buy groceries".into(), completed: false },
///     Task { title: "Walk dog".into(), completed: true },
/// ]);
/// ```
///
/// # Example (Closure-based)
///
/// ```ignore
/// use horizon_lattice::model::{ListModel, ItemRole, ItemData};
///
/// struct Person {
///     name: String,
///     email: String,
/// }
///
/// let model = ListModel::with_extractor(
///     vec![
///         Person { name: "Alice".into(), email: "alice@example.com".into() },
///         Person { name: "Bob".into(), email: "bob@example.com".into() },
///     ],
///     |person, role| match role {
///         ItemRole::Display => ItemData::from(&person.name),
///         ItemRole::ToolTip => ItemData::from(&person.email),
///         _ => ItemData::None,
///     },
/// );
/// ```
pub struct ListModel<T> {
    items: RwLock<Vec<T>>,
    extractor: Option<DataExtractor<T>>,
    flags_extractor: Option<FlagsExtractor<T>>,
    signals: ModelSignals,
}

impl<T: Send + Sync + 'static> ListModel<T> {
    /// Creates an empty list model with a data extractor.
    ///
    /// The extractor function is called to get data for each item and role.
    pub fn with_extractor<F>(items: Vec<T>, extractor: F) -> Self
    where
        F: Fn(&T, ItemRole) -> ItemData + Send + Sync + 'static,
    {
        Self {
            items: RwLock::new(items),
            extractor: Some(Arc::new(extractor)),
            flags_extractor: None,
            signals: ModelSignals::new(),
        }
    }

    /// Creates a list model with both data and flags extractors.
    pub fn with_extractors<D, Fl>(items: Vec<T>, data_extractor: D, flags_extractor: Fl) -> Self
    where
        D: Fn(&T, ItemRole) -> ItemData + Send + Sync + 'static,
        Fl: Fn(&T) -> ItemFlags + Send + Sync + 'static,
    {
        Self {
            items: RwLock::new(items),
            extractor: Some(Arc::new(data_extractor)),
            flags_extractor: Some(Arc::new(flags_extractor)),
            signals: ModelSignals::new(),
        }
    }

    /// Returns the number of items in the model.
    pub fn len(&self) -> usize {
        self.items.read().len()
    }

    /// Returns `true` if the model is empty.
    pub fn is_empty(&self) -> bool {
        self.items.read().is_empty()
    }

    /// Appends an item to the end of the list.
    pub fn push(&self, item: T) {
        let row = self.items.read().len();
        self.signals
            .emit_rows_inserted(ModelIndex::invalid(), row, row, || {
                self.items.write().push(item);
            });
    }

    /// Inserts an item at the specified index.
    ///
    /// # Panics
    ///
    /// Panics if `index > len()`.
    pub fn insert(&self, index: usize, item: T) {
        self.signals
            .emit_rows_inserted(ModelIndex::invalid(), index, index, || {
                self.items.write().insert(index, item);
            });
    }

    /// Removes and returns the item at the specified index.
    ///
    /// # Panics
    ///
    /// Panics if `index >= len()`.
    pub fn remove(&self, index: usize) -> T {
        let mut removed = None;
        self.signals
            .emit_rows_removed(ModelIndex::invalid(), index, index, || {
                removed = Some(self.items.write().remove(index));
            });
        removed.unwrap()
    }

    /// Removes all items from the model.
    pub fn clear(&self) {
        self.signals.emit_reset(|| {
            self.items.write().clear();
        });
    }

    /// Replaces all items in the model.
    pub fn set_items(&self, items: Vec<T>) {
        self.signals.emit_reset(|| {
            *self.items.write() = items;
        });
    }

    /// Returns a reference to the items (read-only access).
    pub fn items(&self) -> impl std::ops::Deref<Target = Vec<T>> + '_ {
        self.items.read()
    }

    /// Provides mutable access to an item via a closure.
    ///
    /// Emits `data_changed` signal after modification.
    pub fn modify<F, R>(&self, index: usize, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut items = self.items.write();
        if index >= items.len() {
            return None;
        }
        let result = f(&mut items[index]);
        drop(items);

        let model_index = ModelIndex::new(index, 0, ModelIndex::invalid());
        self.signals
            .emit_data_changed_single(model_index, vec![ItemRole::Display]);
        Some(result)
    }

    /// Swaps two items in the list.
    pub fn swap(&self, a: usize, b: usize) {
        {
            let mut items = self.items.write();
            if a >= items.len() || b >= items.len() {
                return;
            }
            items.swap(a, b);
        }

        // Emit data changed for both positions
        let index_a = ModelIndex::new(a, 0, ModelIndex::invalid());
        let index_b = ModelIndex::new(b, 0, ModelIndex::invalid());
        self.signals
            .data_changed
            .emit((index_a.clone(), index_a, vec![ItemRole::Display]));
        self.signals
            .data_changed
            .emit((index_b.clone(), index_b, vec![ItemRole::Display]));
    }

    /// Sorts the list using the provided comparator.
    ///
    /// Emits layout change signals.
    pub fn sort_by<F>(&self, compare: F)
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        self.signals.emit_layout_changed(|| {
            self.items.write().sort_by(compare);
        });
    }

    /// Get data for the given index and role using the extractor if present.
    fn get_data_with_extractor(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        if !index.is_valid() {
            return ItemData::None;
        }

        let items = self.items.read();
        let row = index.row();

        if row >= items.len() {
            return ItemData::None;
        }

        if let Some(ref extractor) = self.extractor {
            extractor(&items[row], role)
        } else {
            ItemData::None
        }
    }

    /// Get flags for the given index using the extractor if present.
    fn get_flags_with_extractor(&self, index: &ModelIndex) -> ItemFlags {
        if !index.is_valid() {
            return ItemFlags::disabled();
        }

        let items = self.items.read();
        if index.row() >= items.len() {
            return ItemFlags::disabled();
        }

        if let Some(ref flags_extractor) = self.flags_extractor {
            flags_extractor(&items[index.row()])
        } else {
            ItemFlags::new()
        }
    }
}

impl<T: ListItem + 'static> ListModel<T> {
    /// Creates a new list model with items that implement `ListItem`.
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items: RwLock::new(items),
            extractor: None,
            flags_extractor: None,
            signals: ModelSignals::new(),
        }
    }

    /// Creates an empty list model.
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }
}

impl<T: ListItem + 'static> ItemModel for ListModel<T> {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        if parent.is_valid() {
            0 // Flat list has no children
        } else {
            self.items.read().len()
        }
    }

    fn column_count(&self, _parent: &ModelIndex) -> usize {
        1
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        if !index.is_valid() {
            return ItemData::None;
        }

        let items = self.items.read();
        let row = index.row();

        if row >= items.len() {
            return ItemData::None;
        }

        let item = &items[row];

        // Use extractor if available, otherwise use ListItem trait
        if let Some(ref extractor) = self.extractor {
            return extractor(item, role);
        }

        match role {
            ItemRole::Display => item.display(),
            ItemRole::Decoration => item.decoration(),
            ItemRole::ToolTip => item.tooltip(),
            ItemRole::Edit => item.edit(),
            _ => item.data(role),
        }
    }

    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex {
        if parent.is_valid() || column > 0 {
            return ModelIndex::invalid();
        }

        if row >= self.items.read().len() {
            return ModelIndex::invalid();
        }

        ModelIndex::new(row, column, ModelIndex::invalid())
    }

    fn parent(&self, _index: &ModelIndex) -> ModelIndex {
        ModelIndex::invalid() // Flat list has no parents
    }

    fn signals(&self) -> &ModelSignals {
        &self.signals
    }

    fn flags(&self, index: &ModelIndex) -> ItemFlags {
        if !index.is_valid() {
            return ItemFlags::disabled();
        }

        let items = self.items.read();
        if index.row() >= items.len() {
            return ItemFlags::disabled();
        }

        if let Some(ref flags_extractor) = self.flags_extractor {
            return flags_extractor(&items[index.row()]);
        }

        items[index.row()].flags()
    }
}

/// A list model that uses closures for data extraction.
///
/// This is a separate type for when items don't implement `ListItem`.
/// Use `ListModel::with_extractor` for the primary interface.
pub struct ExtractorListModel<T> {
    inner: ListModel<T>,
}

impl<T: Send + Sync + 'static> ExtractorListModel<T> {
    /// Creates a new extractor-based list model.
    pub fn new<F>(items: Vec<T>, extractor: F) -> Self
    where
        F: Fn(&T, ItemRole) -> ItemData + Send + Sync + 'static,
    {
        Self {
            inner: ListModel::with_extractor(items, extractor),
        }
    }

    /// Returns a reference to the inner ListModel.
    pub fn inner(&self) -> &ListModel<T> {
        &self.inner
    }
}

impl<T: Send + Sync + 'static> ItemModel for ExtractorListModel<T> {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        if parent.is_valid() {
            0
        } else {
            self.inner.items.read().len()
        }
    }

    fn column_count(&self, _parent: &ModelIndex) -> usize {
        1
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        self.inner.get_data_with_extractor(index, role)
    }

    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex {
        if parent.is_valid() || column > 0 {
            return ModelIndex::invalid();
        }

        if row >= self.inner.items.read().len() {
            return ModelIndex::invalid();
        }

        ModelIndex::new(row, column, ModelIndex::invalid())
    }

    fn parent(&self, _index: &ModelIndex) -> ModelIndex {
        ModelIndex::invalid()
    }

    fn signals(&self) -> &ModelSignals {
        &self.inner.signals
    }

    fn flags(&self, index: &ModelIndex) -> ItemFlags {
        self.inner.get_flags_with_extractor(index)
    }
}

impl<T: Send + Sync + 'static> std::ops::Deref for ExtractorListModel<T> {
    type Target = ListModel<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use parking_lot::Mutex;
    use std::sync::Arc;

    #[derive(Clone)]
    struct TestItem {
        name: String,
        value: i32,
    }

    impl ListItem for TestItem {
        fn display(&self) -> ItemData {
            ItemData::from(self.name.as_str())
        }

        fn tooltip(&self) -> ItemData {
            ItemData::from(format!("Value: {}", self.value))
        }
    }

    #[test]
    fn test_trait_based_model() {
        let model = ListModel::new(vec![
            TestItem {
                name: "First".into(),
                value: 1,
            },
            TestItem {
                name: "Second".into(),
                value: 2,
            },
        ]);

        assert_eq!(model.len(), 2);
        assert_eq!(model.row_count(&ModelIndex::invalid()), 2);
        assert_eq!(model.column_count(&ModelIndex::invalid()), 1);

        let index = model.index(0, 0, &ModelIndex::invalid());
        assert!(index.is_valid());

        let display = model.data(&index, ItemRole::Display);
        assert_eq!(display.as_string(), Some("First"));

        let tooltip = model.data(&index, ItemRole::ToolTip);
        assert_eq!(tooltip.as_string(), Some("Value: 1"));
    }

    #[test]
    fn test_closure_based_model() {
        struct Person {
            name: String,
            age: u32,
        }

        let model = ExtractorListModel::new(
            vec![
                Person {
                    name: "Alice".into(),
                    age: 30,
                },
                Person {
                    name: "Bob".into(),
                    age: 25,
                },
            ],
            |person, role| match role {
                ItemRole::Display => ItemData::from(person.name.as_str()),
                ItemRole::ToolTip => ItemData::from(format!("Age: {}", person.age)),
                _ => ItemData::None,
            },
        );

        assert_eq!(model.len(), 2);

        let index = model.index(1, 0, &ModelIndex::invalid());
        let display = model.data(&index, ItemRole::Display);
        assert_eq!(display.as_string(), Some("Bob"));
    }

    #[test]
    fn test_push_and_signals() {
        let model = ListModel::<TestItem>::empty();
        let inserted = Arc::new(Mutex::new(Vec::new()));

        let recv = inserted.clone();
        model
            .signals()
            .rows_inserted
            .connect(move |(_, first, last)| {
                recv.lock().push((*first, *last));
            });

        model.push(TestItem {
            name: "New".into(),
            value: 42,
        });

        assert_eq!(model.len(), 1);
        let events = inserted.lock();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], (0, 0));
    }

    #[test]
    fn test_remove_and_signals() {
        let model = ListModel::new(vec![
            TestItem {
                name: "A".into(),
                value: 1,
            },
            TestItem {
                name: "B".into(),
                value: 2,
            },
            TestItem {
                name: "C".into(),
                value: 3,
            },
        ]);

        let removed = Arc::new(Mutex::new(Vec::new()));

        let recv = removed.clone();
        model
            .signals()
            .rows_removed
            .connect(move |(_, first, last)| {
                recv.lock().push((*first, *last));
            });

        let item = model.remove(1);
        assert_eq!(item.name, "B");
        assert_eq!(model.len(), 2);

        let events = removed.lock();
        assert_eq!(events[0], (1, 1));
    }

    #[test]
    fn test_sort() {
        let model = ListModel::new(vec![
            TestItem {
                name: "C".into(),
                value: 3,
            },
            TestItem {
                name: "A".into(),
                value: 1,
            },
            TestItem {
                name: "B".into(),
                value: 2,
            },
        ]);

        let layout_changed = Arc::new(Mutex::new(false));

        let recv = layout_changed.clone();
        model
            .signals()
            .layout_changed
            .connect(move |_| *recv.lock() = true);

        model.sort_by(|a, b| a.name.cmp(&b.name));

        assert!(*layout_changed.lock());

        let index = model.index(0, 0, &ModelIndex::invalid());
        let display = model.data(&index, ItemRole::Display);
        assert_eq!(display.as_string(), Some("A"));
    }

    #[test]
    fn test_string_list() {
        let model = ListModel::new(vec![
            "Apple".to_string(),
            "Banana".to_string(),
            "Cherry".to_string(),
        ]);

        assert_eq!(model.len(), 3);

        let index = model.index(1, 0, &ModelIndex::invalid());
        let display = model.data(&index, ItemRole::Display);
        assert_eq!(display.as_string(), Some("Banana"));
    }

    #[test]
    fn test_modify() {
        let model = ListModel::new(vec![TestItem {
            name: "Original".into(),
            value: 1,
        }]);

        let data_changed = Arc::new(Mutex::new(false));
        let recv = data_changed.clone();
        model.signals().data_changed.connect(move |_| {
            *recv.lock() = true;
        });

        model.modify(0, |item| {
            item.name = "Modified".into();
        });

        assert!(*data_changed.lock());

        let index = model.index(0, 0, &ModelIndex::invalid());
        let display = model.data(&index, ItemRole::Display);
        assert_eq!(display.as_string(), Some("Modified"));
    }
}
