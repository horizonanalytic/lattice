//! Table model implementation for 2D grid data.
//!
//! `TableModel` provides a simple way to display tabular data with rows and columns.
//! It supports header data for both rows and columns.

use parking_lot::RwLock;
use std::sync::Arc;

use super::index::ModelIndex;
use super::role::{ItemData, ItemRole};
use super::traits::{ItemModel, ModelSignals, Orientation};

/// Type alias for a cell data extractor function.
pub type CellExtractor<T> = Arc<dyn Fn(&T, usize, ItemRole) -> ItemData + Send + Sync>;

/// Type alias for a header data function.
pub type HeaderExtractor = Arc<dyn Fn(usize, Orientation, ItemRole) -> ItemData + Send + Sync>;

/// A table model for displaying 2D grid data.
///
/// `TableModel` stores rows of data, where each row can have multiple columns.
/// Column access is done via an extractor function that maps column indices to data.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::model::{TableModel, ItemRole, ItemData, Orientation};
///
/// struct Employee {
///     name: String,
///     department: String,
///     salary: u32,
/// }
///
/// let employees = vec![
///     Employee { name: "Alice".into(), department: "Engineering".into(), salary: 100000 },
///     Employee { name: "Bob".into(), department: "Sales".into(), salary: 80000 },
/// ];
///
/// let model = TableModel::new(
///     employees,
///     3, // column count
///     |employee, column, role| {
///         if role != ItemRole::Display {
///             return ItemData::None;
///         }
///         match column {
///             0 => ItemData::from(&employee.name),
///             1 => ItemData::from(&employee.department),
///             2 => ItemData::from(employee.salary as i64),
///             _ => ItemData::None,
///         }
///     },
/// ).with_headers(|section, orientation, role| {
///     if orientation != Orientation::Horizontal || role != ItemRole::Display {
///         return ItemData::None;
///     }
///     match section {
///         0 => ItemData::from("Name"),
///         1 => ItemData::from("Department"),
///         2 => ItemData::from("Salary"),
///         _ => ItemData::None,
///     }
/// });
/// ```
pub struct TableModel<T> {
    rows: RwLock<Vec<T>>,
    column_count: usize,
    cell_extractor: CellExtractor<T>,
    header_extractor: Option<HeaderExtractor>,
    signals: ModelSignals,
}

impl<T: Send + Sync + 'static> TableModel<T> {
    /// Creates a new table model.
    ///
    /// # Arguments
    ///
    /// * `rows` - The row data
    /// * `column_count` - Number of columns
    /// * `cell_extractor` - Function to extract cell data: (row_data, column, role) -> data
    pub fn new<F>(rows: Vec<T>, column_count: usize, cell_extractor: F) -> Self
    where
        F: Fn(&T, usize, ItemRole) -> ItemData + Send + Sync + 'static,
    {
        Self {
            rows: RwLock::new(rows),
            column_count,
            cell_extractor: Arc::new(cell_extractor),
            header_extractor: None,
            signals: ModelSignals::new(),
        }
    }

    /// Creates an empty table model with the specified column count.
    pub fn empty(column_count: usize) -> Self
    where
        T: Default,
    {
        Self {
            rows: RwLock::new(Vec::new()),
            column_count,
            cell_extractor: Arc::new(|_, _, _| ItemData::None),
            header_extractor: None,
            signals: ModelSignals::new(),
        }
    }

    /// Adds a header extractor to the model.
    ///
    /// The header extractor is called for header data requests:
    /// (section, orientation, role) -> data
    pub fn with_headers<F>(mut self, header_extractor: F) -> Self
    where
        F: Fn(usize, Orientation, ItemRole) -> ItemData + Send + Sync + 'static,
    {
        self.header_extractor = Some(Arc::new(header_extractor));
        self
    }

    /// Sets the header extractor.
    pub fn set_header_extractor<F>(&mut self, header_extractor: F)
    where
        F: Fn(usize, Orientation, ItemRole) -> ItemData + Send + Sync + 'static,
    {
        self.header_extractor = Some(Arc::new(header_extractor));
    }

    /// Returns the number of rows.
    pub fn row_count_value(&self) -> usize {
        self.rows.read().len()
    }

    /// Returns the number of columns.
    pub fn column_count_value(&self) -> usize {
        self.column_count
    }

    /// Returns `true` if the model is empty.
    pub fn is_empty(&self) -> bool {
        self.rows.read().is_empty()
    }

    /// Appends a row to the end of the table.
    pub fn push_row(&self, row: T) {
        let row_index = self.rows.read().len();
        self.signals
            .emit_rows_inserted(ModelIndex::invalid(), row_index, row_index, || {
                self.rows.write().push(row);
            });
    }

    /// Inserts a row at the specified index.
    pub fn insert_row(&self, index: usize, row: T) {
        self.signals
            .emit_rows_inserted(ModelIndex::invalid(), index, index, || {
                self.rows.write().insert(index, row);
            });
    }

    /// Removes and returns the row at the specified index.
    pub fn remove_row(&self, index: usize) -> T {
        let mut removed = None;
        self.signals
            .emit_rows_removed(ModelIndex::invalid(), index, index, || {
                removed = Some(self.rows.write().remove(index));
            });
        removed.unwrap()
    }

    /// Removes all rows from the table.
    pub fn clear(&self) {
        self.signals.emit_reset(|| {
            self.rows.write().clear();
        });
    }

    /// Replaces all rows in the table.
    pub fn set_rows(&self, rows: Vec<T>) {
        self.signals.emit_reset(|| {
            *self.rows.write() = rows;
        });
    }

    /// Returns a reference to the rows (read-only access).
    pub fn rows(&self) -> impl std::ops::Deref<Target = Vec<T>> + '_ {
        self.rows.read()
    }

    /// Provides mutable access to a row via a closure.
    ///
    /// Emits `data_changed` signal for the entire row after modification.
    pub fn modify_row<F, R>(&self, row_index: usize, f: F) -> Option<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut rows = self.rows.write();
        if row_index >= rows.len() {
            return None;
        }
        let result = f(&mut rows[row_index]);
        drop(rows);

        // Emit data changed for all columns in the row
        let first = ModelIndex::new(row_index, 0, ModelIndex::invalid());
        let last = ModelIndex::new(row_index, self.column_count.saturating_sub(1), ModelIndex::invalid());
        self.signals
            .data_changed
            .emit((first, last, vec![ItemRole::Display]));
        Some(result)
    }

    /// Sorts rows using the provided comparator.
    pub fn sort_by<F>(&self, compare: F)
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        self.signals.emit_layout_changed(|| {
            self.rows.write().sort_by(compare);
        });
    }
}

impl<T: Send + Sync + 'static> ItemModel for TableModel<T> {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        if parent.is_valid() {
            0 // Flat table has no children
        } else {
            self.rows.read().len()
        }
    }

    fn column_count(&self, _parent: &ModelIndex) -> usize {
        self.column_count
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        if !index.is_valid() {
            return ItemData::None;
        }

        let rows = self.rows.read();
        let row_idx = index.row();
        let col_idx = index.column();

        if row_idx >= rows.len() || col_idx >= self.column_count {
            return ItemData::None;
        }

        (self.cell_extractor)(&rows[row_idx], col_idx, role)
    }

    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex {
        if parent.is_valid() {
            return ModelIndex::invalid();
        }

        let rows = self.rows.read();
        if row >= rows.len() || column >= self.column_count {
            return ModelIndex::invalid();
        }

        ModelIndex::new(row, column, ModelIndex::invalid())
    }

    fn parent(&self, _index: &ModelIndex) -> ModelIndex {
        ModelIndex::invalid() // Flat table has no parents
    }

    fn signals(&self) -> &ModelSignals {
        &self.signals
    }

    fn header_data(&self, section: usize, orientation: Orientation, role: ItemRole) -> ItemData {
        if let Some(ref extractor) = self.header_extractor {
            extractor(section, orientation, role)
        } else {
            ItemData::None
        }
    }
}

/// A simple table model that stores data in a 2D vector.
///
/// This is useful for simple tables where you don't need custom row types.
pub struct SimpleTableModel {
    data: RwLock<Vec<Vec<ItemData>>>,
    column_count: usize,
    headers: RwLock<Vec<String>>,
    signals: ModelSignals,
}

impl SimpleTableModel {
    /// Creates a new simple table model with the specified column count.
    pub fn new(column_count: usize) -> Self {
        Self {
            data: RwLock::new(Vec::new()),
            column_count,
            headers: RwLock::new(vec![String::new(); column_count]),
            signals: ModelSignals::new(),
        }
    }

    /// Creates a simple table model from 2D data.
    pub fn from_data(data: Vec<Vec<ItemData>>) -> Self {
        let column_count = data.first().map(|r| r.len()).unwrap_or(0);
        Self {
            data: RwLock::new(data),
            column_count,
            headers: RwLock::new(vec![String::new(); column_count]),
            signals: ModelSignals::new(),
        }
    }

    /// Sets the column headers.
    pub fn set_headers(&self, headers: Vec<String>) {
        *self.headers.write() = headers;
        self.signals
            .header_data_changed
            .emit((Orientation::Horizontal, 0, self.column_count.saturating_sub(1)));
    }

    /// Sets a single header.
    pub fn set_header(&self, column: usize, header: String) {
        let mut headers = self.headers.write();
        if column < headers.len() {
            headers[column] = header;
            drop(headers);
            self.signals
                .header_data_changed
                .emit((Orientation::Horizontal, column, column));
        }
    }

    /// Sets the data at the specified cell.
    pub fn set_cell(&self, row: usize, column: usize, value: ItemData) {
        let mut data = self.data.write();
        if row < data.len() && column < self.column_count {
            if column < data[row].len() {
                data[row][column] = value;
                drop(data);
                let index = ModelIndex::new(row, column, ModelIndex::invalid());
                self.signals
                    .emit_data_changed_single(index, vec![ItemRole::Display]);
            }
        }
    }

    /// Appends a row.
    pub fn append_row(&self, row: Vec<ItemData>) {
        let row_index = self.data.read().len();
        self.signals
            .emit_rows_inserted(ModelIndex::invalid(), row_index, row_index, || {
                self.data.write().push(row);
            });
    }

    /// Removes a row.
    pub fn remove_row(&self, index: usize) {
        self.signals
            .emit_rows_removed(ModelIndex::invalid(), index, index, || {
                self.data.write().remove(index);
            });
    }

    /// Clears all data.
    pub fn clear(&self) {
        self.signals.emit_reset(|| {
            self.data.write().clear();
        });
    }

    /// Returns the number of rows.
    pub fn row_count_value(&self) -> usize {
        self.data.read().len()
    }

    /// Returns the number of columns.
    pub fn column_count_value(&self) -> usize {
        self.column_count
    }
}

impl ItemModel for SimpleTableModel {
    fn row_count(&self, parent: &ModelIndex) -> usize {
        if parent.is_valid() {
            0
        } else {
            self.data.read().len()
        }
    }

    fn column_count(&self, _parent: &ModelIndex) -> usize {
        self.column_count
    }

    fn data(&self, index: &ModelIndex, role: ItemRole) -> ItemData {
        if !index.is_valid() || role != ItemRole::Display {
            return ItemData::None;
        }

        let data = self.data.read();
        let row = index.row();
        let col = index.column();

        if row >= data.len() || col >= self.column_count {
            return ItemData::None;
        }

        if col < data[row].len() {
            data[row][col].clone()
        } else {
            ItemData::None
        }
    }

    fn index(&self, row: usize, column: usize, parent: &ModelIndex) -> ModelIndex {
        if parent.is_valid() {
            return ModelIndex::invalid();
        }

        let data = self.data.read();
        if row >= data.len() || column >= self.column_count {
            return ModelIndex::invalid();
        }

        ModelIndex::new(row, column, ModelIndex::invalid())
    }

    fn parent(&self, _index: &ModelIndex) -> ModelIndex {
        ModelIndex::invalid()
    }

    fn signals(&self) -> &ModelSignals {
        &self.signals
    }

    fn header_data(&self, section: usize, orientation: Orientation, role: ItemRole) -> ItemData {
        if orientation != Orientation::Horizontal || role != ItemRole::Display {
            return ItemData::None;
        }

        let headers = self.headers.read();
        if section < headers.len() {
            ItemData::from(headers[section].as_str())
        } else {
            ItemData::None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRow {
        name: String,
        value: i32,
        active: bool,
    }

    #[test]
    fn test_table_model_basic() {
        let model = TableModel::new(
            vec![
                TestRow {
                    name: "First".into(),
                    value: 100,
                    active: true,
                },
                TestRow {
                    name: "Second".into(),
                    value: 200,
                    active: false,
                },
            ],
            3,
            |row, col, role| {
                if role != ItemRole::Display {
                    return ItemData::None;
                }
                match col {
                    0 => ItemData::from(row.name.as_str()),
                    1 => ItemData::from(row.value as i64),
                    2 => ItemData::from(row.active),
                    _ => ItemData::None,
                }
            },
        );

        assert_eq!(model.row_count(&ModelIndex::invalid()), 2);
        assert_eq!(model.column_count(&ModelIndex::invalid()), 3);

        let index = model.index(0, 0, &ModelIndex::invalid());
        assert!(index.is_valid());
        assert_eq!(model.data(&index, ItemRole::Display).as_string(), Some("First"));

        let index = model.index(1, 1, &ModelIndex::invalid());
        assert_eq!(model.data(&index, ItemRole::Display).as_int(), Some(200));

        let index = model.index(0, 2, &ModelIndex::invalid());
        assert_eq!(model.data(&index, ItemRole::Display).as_bool(), Some(true));
    }

    #[test]
    fn test_table_model_with_headers() {
        let model = TableModel::new(
            vec![TestRow {
                name: "Test".into(),
                value: 42,
                active: true,
            }],
            3,
            |row, col, role| {
                if role != ItemRole::Display {
                    return ItemData::None;
                }
                match col {
                    0 => ItemData::from(row.name.as_str()),
                    1 => ItemData::from(row.value as i64),
                    2 => ItemData::from(row.active),
                    _ => ItemData::None,
                }
            },
        )
        .with_headers(|section, orientation, role| {
            if orientation != Orientation::Horizontal || role != ItemRole::Display {
                return ItemData::None;
            }
            match section {
                0 => ItemData::from("Name"),
                1 => ItemData::from("Value"),
                2 => ItemData::from("Active"),
                _ => ItemData::None,
            }
        });

        assert_eq!(
            model.header_data(0, Orientation::Horizontal, ItemRole::Display).as_string(),
            Some("Name")
        );
        assert_eq!(
            model.header_data(1, Orientation::Horizontal, ItemRole::Display).as_string(),
            Some("Value")
        );
    }

    #[test]
    fn test_simple_table_model() {
        let model = SimpleTableModel::new(3);
        model.set_headers(vec!["A".into(), "B".into(), "C".into()]);

        model.append_row(vec![
            ItemData::from("a1"),
            ItemData::from("b1"),
            ItemData::from("c1"),
        ]);
        model.append_row(vec![
            ItemData::from("a2"),
            ItemData::from("b2"),
            ItemData::from("c2"),
        ]);

        assert_eq!(model.row_count(&ModelIndex::invalid()), 2);
        assert_eq!(model.column_count(&ModelIndex::invalid()), 3);

        let index = model.index(0, 1, &ModelIndex::invalid());
        assert_eq!(model.data(&index, ItemRole::Display).as_string(), Some("b1"));

        assert_eq!(
            model.header_data(0, Orientation::Horizontal, ItemRole::Display).as_string(),
            Some("A")
        );
    }

    #[test]
    fn test_table_push_and_remove() {
        let model = TableModel::new(
            vec![TestRow {
                name: "First".into(),
                value: 1,
                active: true,
            }],
            3,
            |row, col, role| {
                if role != ItemRole::Display {
                    return ItemData::None;
                }
                match col {
                    0 => ItemData::from(row.name.as_str()),
                    _ => ItemData::None,
                }
            },
        );

        assert_eq!(model.row_count_value(), 1);

        model.push_row(TestRow {
            name: "Second".into(),
            value: 2,
            active: false,
        });
        assert_eq!(model.row_count_value(), 2);

        model.remove_row(0);
        assert_eq!(model.row_count_value(), 1);

        let index = model.index(0, 0, &ModelIndex::invalid());
        assert_eq!(model.data(&index, ItemRole::Display).as_string(), Some("Second"));
    }
}
