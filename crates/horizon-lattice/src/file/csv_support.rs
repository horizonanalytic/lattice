//! CSV file parsing, generation, and manipulation.
//!
//! This module provides a convenient API for working with CSV (Comma-Separated Values) files,
//! including parsing, serialization, and record manipulation. CSV format is a common
//! format for tabular data exchange.
//!
//! # Parsing CSV
//!
//! ```ignore
//! use horizon_lattice::file::csv_support::{parse_csv, read_csv, CsvOptions};
//!
//! // Parse from string
//! let table = parse_csv("name,age\nAlice,30\nBob,25")?;
//!
//! // Read from file
//! let table = read_csv("data.csv")?;
//!
//! // Parse with custom options
//! let options = CsvOptions::new().delimiter(b';').no_headers();
//! let table = parse_csv_with_options("Alice;30\nBob;25", &options)?;
//! ```
//!
//! # Deserializing to Typed Records
//!
//! ```ignore
//! use horizon_lattice::file::csv_support::read_csv_as;
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct Person {
//!     name: String,
//!     age: u32,
//! }
//!
//! let people: Vec<Person> = read_csv_as("people.csv")?;
//! ```
//!
//! # Writing CSV
//!
//! ```ignore
//! use horizon_lattice::file::csv_support::{write_csv, CsvTable, CsvOptions};
//!
//! // Write a table to file
//! let mut table = CsvTable::new();
//! table.set_headers(vec!["name", "age"]);
//! table.push_record(vec!["Alice", "30"]);
//! table.push_record(vec!["Bob", "25"]);
//! write_csv("output.csv", &table)?;
//!
//! // Write with custom options
//! let options = CsvOptions::new().delimiter(b'\t');
//! write_csv_with_options("output.tsv", &table, &options)?;
//! ```
//!
//! # Serializing Typed Records
//!
//! ```ignore
//! use horizon_lattice::file::csv_support::write_csv_records;
//! use serde::Serialize;
//!
//! #[derive(Serialize)]
//! struct Person {
//!     name: String,
//!     age: u32,
//! }
//!
//! let people = vec![
//!     Person { name: "Alice".into(), age: 30 },
//!     Person { name: "Bob".into(), age: 25 },
//! ];
//! write_csv_records("people.csv", &people)?;
//! ```
//!
//! # Working with CsvTable
//!
//! ```ignore
//! use horizon_lattice::file::csv_support::CsvTable;
//!
//! let table = parse_csv("name,age\nAlice,30\nBob,25")?;
//!
//! // Access headers
//! let headers = table.headers();  // Some(&["name", "age"])
//!
//! // Access records
//! for record in table.records() {
//!     println!("{}: {}", record.get(0).unwrap(), record.get(1).unwrap());
//! }
//!
//! // Get typed values
//! let age: Option<i32> = table.get_as(0, 1);  // row 0, column 1 as i32
//!
//! // Access by column name (if headers exist)
//! let name = table.get_by_name(0, "name");  // row 0, column "name"
//! ```

use std::fmt;
use std::io::Read;
use std::path::Path;
use std::str::FromStr;

use serde::de::DeserializeOwned;
use serde::Serialize;

use super::error::{FileError, FileErrorKind, FileResult};
use super::operations::{atomic_write, read_bytes};

/// Configuration options for CSV parsing and writing.
#[derive(Debug, Clone)]
pub struct CsvOptions {
    /// Field delimiter (default: comma)
    delimiter: u8,
    /// Whether the first row contains headers (default: true)
    has_headers: bool,
    /// Quote character (default: double quote)
    quote: u8,
    /// Whether to allow flexible record lengths (default: false)
    flexible: bool,
    /// Comment character (records starting with this are ignored)
    comment: Option<u8>,
    /// Whether to trim whitespace from fields (default: false)
    trim: bool,
    /// Whether to double quotes for escaping (default: true for writing)
    double_quote: bool,
}

impl CsvOptions {
    /// Creates default CSV options.
    pub fn new() -> Self {
        Self {
            delimiter: b',',
            has_headers: true,
            quote: b'"',
            flexible: false,
            comment: None,
            trim: false,
            double_quote: true,
        }
    }

    /// Sets the field delimiter.
    ///
    /// Common values:
    /// - `b','` - Comma (default, CSV)
    /// - `b'\t'` - Tab (TSV)
    /// - `b';'` - Semicolon (common in European locales)
    pub fn delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Indicates that the CSV has no header row.
    pub fn no_headers(mut self) -> Self {
        self.has_headers = false;
        self
    }

    /// Sets whether the first row contains headers.
    pub fn has_headers(mut self, has_headers: bool) -> Self {
        self.has_headers = has_headers;
        self
    }

    /// Sets the quote character.
    pub fn quote(mut self, quote: u8) -> Self {
        self.quote = quote;
        self
    }

    /// Allows records with varying numbers of fields.
    pub fn flexible(mut self, flexible: bool) -> Self {
        self.flexible = flexible;
        self
    }

    /// Sets a comment character (lines starting with this are ignored).
    pub fn comment(mut self, comment: u8) -> Self {
        self.comment = Some(comment);
        self
    }

    /// Enables trimming of whitespace from fields.
    pub fn trim(mut self, trim: bool) -> Self {
        self.trim = trim;
        self
    }

    /// Sets whether to use double quotes for escaping.
    pub fn double_quote(mut self, double_quote: bool) -> Self {
        self.double_quote = double_quote;
        self
    }

    /// Creates a reader builder with these options.
    fn reader_builder(&self) -> csv::ReaderBuilder {
        let mut builder = csv::ReaderBuilder::new();
        builder
            .delimiter(self.delimiter)
            .has_headers(self.has_headers)
            .quote(self.quote)
            .flexible(self.flexible)
            .double_quote(self.double_quote);

        if let Some(comment) = self.comment {
            builder.comment(Some(comment));
        }

        if self.trim {
            builder.trim(csv::Trim::All);
        }

        builder
    }

    /// Creates a writer builder with these options.
    fn writer_builder(&self) -> csv::WriterBuilder {
        let mut builder = csv::WriterBuilder::new();
        builder
            .delimiter(self.delimiter)
            .quote(self.quote)
            .double_quote(self.double_quote);
        builder
    }
}

impl Default for CsvOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// A CSV record (row) containing string fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvRecord {
    fields: Vec<String>,
}

impl CsvRecord {
    /// Creates a new empty record.
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Creates a record from a vector of strings.
    pub fn from_fields(fields: Vec<String>) -> Self {
        Self { fields }
    }

    /// Creates a record from an iterator of string-like values.
    pub fn from_iter<I, S>(iter: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            fields: iter.into_iter().map(|s| s.into()).collect(),
        }
    }

    /// Returns the number of fields in this record.
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns true if this record has no fields.
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Gets a field by index.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.fields.get(index).map(|s| s.as_str())
    }

    /// Gets a field and parses it as the specified type.
    pub fn get_as<T: FromStr>(&self, index: usize) -> Option<T> {
        self.get(index).and_then(|s| s.parse().ok())
    }

    /// Pushes a field to the record.
    pub fn push(&mut self, field: impl Into<String>) {
        self.fields.push(field.into());
    }

    /// Returns an iterator over the fields.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|s| s.as_str())
    }

    /// Returns the fields as a slice.
    pub fn as_slice(&self) -> &[String] {
        &self.fields
    }

    /// Converts this record to a vector of strings.
    pub fn into_vec(self) -> Vec<String> {
        self.fields
    }
}

impl Default for CsvRecord {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> IntoIterator for &'a CsvRecord {
    type Item = &'a str;
    type IntoIter = std::iter::Map<std::slice::Iter<'a, String>, fn(&'a String) -> &'a str>;

    fn into_iter(self) -> Self::IntoIter {
        self.fields.iter().map(|s| s.as_str())
    }
}

impl fmt::Display for CsvRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let joined = self.fields.join(",");
        write!(f, "{}", joined)
    }
}

/// A CSV table containing optional headers and records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvTable {
    headers: Option<CsvRecord>,
    records: Vec<CsvRecord>,
}

impl CsvTable {
    // ========================================================================
    // Construction
    // ========================================================================

    /// Creates a new empty CSV table.
    pub fn new() -> Self {
        Self {
            headers: None,
            records: Vec::new(),
        }
    }

    /// Creates a CSV table with headers.
    pub fn with_headers<I, S>(headers: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            headers: Some(CsvRecord::from_iter(headers)),
            records: Vec::new(),
        }
    }

    // ========================================================================
    // Headers
    // ========================================================================

    /// Returns the headers, if present.
    pub fn headers(&self) -> Option<&CsvRecord> {
        self.headers.as_ref()
    }

    /// Returns the header names as a slice, if present.
    pub fn header_names(&self) -> Option<&[String]> {
        self.headers.as_ref().map(|h| h.as_slice())
    }

    /// Sets the headers for this table.
    pub fn set_headers<I, S>(&mut self, headers: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.headers = Some(CsvRecord::from_iter(headers));
    }

    /// Removes the headers from this table.
    pub fn clear_headers(&mut self) {
        self.headers = None;
    }

    /// Returns true if this table has headers.
    pub fn has_headers(&self) -> bool {
        self.headers.is_some()
    }

    /// Returns the index of a column by name, if headers exist.
    pub fn column_index(&self, name: &str) -> Option<usize> {
        self.headers
            .as_ref()
            .and_then(|h| h.iter().position(|n| n == name))
    }

    // ========================================================================
    // Records
    // ========================================================================

    /// Returns the number of records (excluding headers).
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns true if there are no records.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns a reference to the records.
    pub fn records(&self) -> &[CsvRecord] {
        &self.records
    }

    /// Returns a mutable reference to the records.
    pub fn records_mut(&mut self) -> &mut Vec<CsvRecord> {
        &mut self.records
    }

    /// Gets a record by index.
    pub fn record(&self, index: usize) -> Option<&CsvRecord> {
        self.records.get(index)
    }

    /// Pushes a record to the table.
    pub fn push_record(&mut self, record: impl Into<CsvRecord>) {
        self.records.push(record.into());
    }

    /// Returns an iterator over the records.
    pub fn iter(&self) -> impl Iterator<Item = &CsvRecord> {
        self.records.iter()
    }

    // ========================================================================
    // Cell Access
    // ========================================================================

    /// Gets a cell value by row and column index.
    pub fn get(&self, row: usize, col: usize) -> Option<&str> {
        self.records.get(row).and_then(|r| r.get(col))
    }

    /// Gets a cell value and parses it as the specified type.
    pub fn get_as<T: FromStr>(&self, row: usize, col: usize) -> Option<T> {
        self.get(row, col).and_then(|s| s.parse().ok())
    }

    /// Gets a cell value by row index and column name.
    pub fn get_by_name(&self, row: usize, col_name: &str) -> Option<&str> {
        let col_idx = self.column_index(col_name)?;
        self.get(row, col_idx)
    }

    /// Gets a cell value by column name and parses it as the specified type.
    pub fn get_by_name_as<T: FromStr>(&self, row: usize, col_name: &str) -> Option<T> {
        self.get_by_name(row, col_name)
            .and_then(|s| s.parse().ok())
    }

    // ========================================================================
    // Column Operations
    // ========================================================================

    /// Returns the number of columns (based on headers or first record).
    pub fn column_count(&self) -> usize {
        self.headers
            .as_ref()
            .map(|h| h.len())
            .or_else(|| self.records.first().map(|r| r.len()))
            .unwrap_or(0)
    }

    /// Returns an iterator over all values in a column.
    pub fn column(&self, index: usize) -> impl Iterator<Item = &str> {
        self.records.iter().filter_map(move |r| r.get(index))
    }

    /// Returns an iterator over all values in a column by name.
    pub fn column_by_name<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        let col_idx = self.column_index(name);
        self.records
            .iter()
            .filter_map(move |r| col_idx.and_then(|i| r.get(i)))
    }

    // ========================================================================
    // Serialization
    // ========================================================================

    /// Converts this table to a CSV string.
    pub fn as_csv_string(&self) -> String {
        self.to_string_with_options(&CsvOptions::default())
    }

    /// Converts this table to a CSV string with custom options.
    pub fn to_string_with_options(&self, options: &CsvOptions) -> String {
        let mut writer = options.writer_builder().from_writer(Vec::new());

        if let Some(headers) = &self.headers {
            let _ = writer.write_record(headers.iter());
        }

        for record in &self.records {
            let _ = writer.write_record(record.iter());
        }

        let _ = writer.flush();
        String::from_utf8_lossy(&writer.into_inner().unwrap_or_default()).into_owned()
    }

    /// Saves this table to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> FileResult<()> {
        self.save_with_options(path, &CsvOptions::default())
    }

    /// Saves this table to a file with custom options.
    pub fn save_with_options(&self, path: impl AsRef<Path>, options: &CsvOptions) -> FileResult<()> {
        let content = self.to_string_with_options(options);
        atomic_write(&path, |writer| writer.write_all(content.as_bytes()))
    }
}

impl Default for CsvTable {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CsvTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string_with_options(&CsvOptions::default()))
    }
}

// Allow creating CsvRecord from Vec<String>
impl From<Vec<String>> for CsvRecord {
    fn from(fields: Vec<String>) -> Self {
        CsvRecord::from_fields(fields)
    }
}

// Allow creating CsvRecord from Vec<&str>
impl From<Vec<&str>> for CsvRecord {
    fn from(fields: Vec<&str>) -> Self {
        CsvRecord::from_iter(fields)
    }
}

// ============================================================================
// Module-Level Functions - Parsing
// ============================================================================

/// Parses a CSV string into a CsvTable.
///
/// By default, assumes the first row contains headers.
///
/// # Example
///
/// ```ignore
/// let table = parse_csv("name,age\nAlice,30\nBob,25")?;
/// assert_eq!(table.len(), 2);
/// assert_eq!(table.get_by_name(0, "name"), Some("Alice"));
/// ```
pub fn parse_csv(s: &str) -> FileResult<CsvTable> {
    parse_csv_with_options(s, &CsvOptions::default())
}

/// Parses a CSV string with custom options.
///
/// # Example
///
/// ```ignore
/// let options = CsvOptions::new().delimiter(b';').no_headers();
/// let table = parse_csv_with_options("Alice;30\nBob;25", &options)?;
/// ```
pub fn parse_csv_with_options(s: &str, options: &CsvOptions) -> FileResult<CsvTable> {
    let mut reader = options.reader_builder().from_reader(s.as_bytes());
    read_into_table(&mut reader, options.has_headers)
}

/// Reads and parses a CSV file into a CsvTable.
///
/// # Example
///
/// ```ignore
/// let table = read_csv("data.csv")?;
/// for record in table.records() {
///     println!("{:?}", record);
/// }
/// ```
pub fn read_csv(path: impl AsRef<Path>) -> FileResult<CsvTable> {
    read_csv_with_options(path, &CsvOptions::default())
}

/// Reads and parses a CSV file with custom options.
///
/// # Example
///
/// ```ignore
/// let options = CsvOptions::new().delimiter(b'\t');
/// let table = read_csv_with_options("data.tsv", &options)?;
/// ```
pub fn read_csv_with_options(path: impl AsRef<Path>, options: &CsvOptions) -> FileResult<CsvTable> {
    let bytes = read_bytes(&path)?;
    let mut reader = options.reader_builder().from_reader(bytes.as_slice());
    read_into_table(&mut reader, options.has_headers).map_err(|_| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(path.as_ref().to_path_buf()),
            None,
        )
    })
}

/// Reads and deserializes CSV records into a vector of typed structs.
///
/// The CSV must have headers that match the struct field names.
///
/// # Example
///
/// ```ignore
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let people: Vec<Person> = read_csv_as("people.csv")?;
/// ```
pub fn read_csv_as<T>(path: impl AsRef<Path>) -> FileResult<Vec<T>>
where
    T: DeserializeOwned,
{
    read_csv_as_with_options(path, &CsvOptions::default())
}

/// Reads and deserializes CSV records with custom options.
pub fn read_csv_as_with_options<T>(path: impl AsRef<Path>, options: &CsvOptions) -> FileResult<Vec<T>>
where
    T: DeserializeOwned,
{
    let bytes = read_bytes(&path)?;
    let mut reader = options.reader_builder().from_reader(bytes.as_slice());

    let mut records = Vec::new();
    for result in reader.deserialize() {
        let record: T = result.map_err(|e| csv_error(e, Some(path.as_ref())))?;
        records.push(record);
    }

    Ok(records)
}

/// Parses and deserializes CSV from a string into a vector of typed structs.
///
/// # Example
///
/// ```ignore
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let csv = "name,age\nAlice,30\nBob,25";
/// let people: Vec<Person> = parse_csv_as(csv)?;
/// ```
pub fn parse_csv_as<T>(s: &str) -> FileResult<Vec<T>>
where
    T: DeserializeOwned,
{
    parse_csv_as_with_options(s, &CsvOptions::default())
}

/// Parses and deserializes CSV from a string with custom options.
pub fn parse_csv_as_with_options<T>(s: &str, options: &CsvOptions) -> FileResult<Vec<T>>
where
    T: DeserializeOwned,
{
    let mut reader = options.reader_builder().from_reader(s.as_bytes());

    let mut records = Vec::new();
    for result in reader.deserialize() {
        let record: T = result.map_err(|e| csv_error(e, None))?;
        records.push(record);
    }

    Ok(records)
}

// ============================================================================
// Module-Level Functions - Writing
// ============================================================================

/// Writes a CsvTable to a file.
///
/// The file is written atomically using a temporary file and rename.
///
/// # Example
///
/// ```ignore
/// let mut table = CsvTable::with_headers(vec!["name", "age"]);
/// table.push_record(vec!["Alice", "30"]);
/// write_csv("output.csv", &table)?;
/// ```
pub fn write_csv(path: impl AsRef<Path>, table: &CsvTable) -> FileResult<()> {
    table.save(&path)
}

/// Writes a CsvTable to a file with custom options.
///
/// # Example
///
/// ```ignore
/// let options = CsvOptions::new().delimiter(b'\t');
/// write_csv_with_options("output.tsv", &table, &options)?;
/// ```
pub fn write_csv_with_options(
    path: impl AsRef<Path>,
    table: &CsvTable,
    options: &CsvOptions,
) -> FileResult<()> {
    table.save_with_options(&path, options)
}

/// Serializes typed records to a CSV file.
///
/// # Example
///
/// ```ignore
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Person {
///     name: String,
///     age: u32,
/// }
///
/// let people = vec![
///     Person { name: "Alice".into(), age: 30 },
///     Person { name: "Bob".into(), age: 25 },
/// ];
/// write_csv_records("people.csv", &people)?;
/// ```
pub fn write_csv_records<T>(path: impl AsRef<Path>, records: &[T]) -> FileResult<()>
where
    T: Serialize,
{
    write_csv_records_with_options(path, records, &CsvOptions::default())
}

/// Serializes typed records to a CSV file with custom options.
pub fn write_csv_records_with_options<T>(
    path: impl AsRef<Path>,
    records: &[T],
    options: &CsvOptions,
) -> FileResult<()>
where
    T: Serialize,
{
    let content = records_to_csv_string(records, options)?;
    atomic_write(&path, |writer| writer.write_all(content.as_bytes()))
}

/// Converts typed records to a CSV string.
///
/// # Example
///
/// ```ignore
/// let csv_str = to_csv_string(&records)?;
/// ```
pub fn to_csv_string<T>(records: &[T]) -> FileResult<String>
where
    T: Serialize,
{
    records_to_csv_string(records, &CsvOptions::default())
}

/// Converts typed records to a CSV string with custom options.
pub fn to_csv_string_with_options<T>(records: &[T], options: &CsvOptions) -> FileResult<String>
where
    T: Serialize,
{
    records_to_csv_string(records, options)
}

/// Converts a CsvTable to a CSV string.
pub fn table_to_csv_string(table: &CsvTable) -> String {
    table.to_string()
}

/// Converts a CsvTable to a CSV string with custom options.
pub fn table_to_csv_string_with_options(table: &CsvTable, options: &CsvOptions) -> String {
    table.to_string_with_options(options)
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Reads CSV data from a reader into a CsvTable.
fn read_into_table<R: Read>(reader: &mut csv::Reader<R>, has_headers: bool) -> FileResult<CsvTable> {
    let headers = if has_headers {
        let header_record = reader.headers().map_err(|e| csv_error(e, None))?;
        Some(CsvRecord::from_iter(header_record.iter().map(|s| s.to_string())))
    } else {
        None
    };

    let mut records = Vec::new();
    for result in reader.records() {
        let record = result.map_err(|e| csv_error(e, None))?;
        records.push(CsvRecord::from_iter(record.iter().map(|s| s.to_string())));
    }

    Ok(CsvTable { headers, records })
}

/// Converts typed records to a CSV string.
fn records_to_csv_string<T>(records: &[T], options: &CsvOptions) -> FileResult<String>
where
    T: Serialize,
{
    let mut writer = options.writer_builder().from_writer(Vec::new());

    for record in records {
        writer.serialize(record).map_err(|e| csv_error(e, None))?;
    }

    writer.flush().map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            None,
            Some(e),
        )
    })?;

    let bytes = writer.into_inner().map_err(|e| {
        FileError::new(
            FileErrorKind::Other,
            None,
            Some(std::io::Error::new(std::io::ErrorKind::Other, e)),
        )
    })?;

    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Converts a csv error to a FileError.
fn csv_error(e: csv::Error, path: Option<&Path>) -> FileError {
    FileError::new(
        FileErrorKind::InvalidData,
        path.map(|p| p.to_path_buf()),
        Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_csv_with_headers() {
        let table = parse_csv("name,age\nAlice,30\nBob,25").unwrap();

        assert!(table.has_headers());
        assert_eq!(table.len(), 2);
        assert_eq!(table.header_names(), Some(&["name".to_string(), "age".to_string()][..]));
        assert_eq!(table.get_by_name(0, "name"), Some("Alice"));
        assert_eq!(table.get_by_name_as::<i32>(0, "age"), Some(30));
        assert_eq!(table.get_by_name(1, "name"), Some("Bob"));
    }

    #[test]
    fn test_parse_csv_without_headers() {
        let options = CsvOptions::new().no_headers();
        let table = parse_csv_with_options("Alice,30\nBob,25", &options).unwrap();

        assert!(!table.has_headers());
        assert_eq!(table.len(), 2);
        assert_eq!(table.get(0, 0), Some("Alice"));
        assert_eq!(table.get_as::<i32>(0, 1), Some(30));
    }

    #[test]
    fn test_csv_table_construction() {
        let mut table = CsvTable::with_headers(vec!["name", "age"]);
        table.push_record(vec!["Alice", "30"]);
        table.push_record(vec!["Bob", "25"]);

        assert!(table.has_headers());
        assert_eq!(table.len(), 2);
        assert_eq!(table.column_count(), 2);
        assert_eq!(table.column_index("name"), Some(0));
        assert_eq!(table.column_index("age"), Some(1));
    }

    #[test]
    fn test_csv_record() {
        let record = CsvRecord::from_iter(vec!["Alice", "30", "true"]);

        assert_eq!(record.len(), 3);
        assert_eq!(record.get(0), Some("Alice"));
        assert_eq!(record.get_as::<i32>(1), Some(30));
        assert_eq!(record.get_as::<bool>(2), Some(true));
        assert_eq!(record.get(5), None);
    }

    #[test]
    fn test_custom_delimiter() {
        let options = CsvOptions::new().delimiter(b';');
        let table = parse_csv_with_options("name;age\nAlice;30", &options).unwrap();

        assert_eq!(table.get_by_name(0, "name"), Some("Alice"));
        assert_eq!(table.get_by_name_as::<i32>(0, "age"), Some(30));
    }

    #[test]
    fn test_tab_delimiter() {
        let options = CsvOptions::new().delimiter(b'\t');
        let table = parse_csv_with_options("name\tage\nAlice\t30", &options).unwrap();

        assert_eq!(table.get_by_name(0, "name"), Some("Alice"));
    }

    #[test]
    fn test_flexible_records() {
        let options = CsvOptions::new().flexible(true).no_headers();
        let table = parse_csv_with_options("a,b,c\nd,e", &options).unwrap();

        assert_eq!(table.record(0).unwrap().len(), 3);
        assert_eq!(table.record(1).unwrap().len(), 2);
    }

    #[test]
    fn test_column_iteration() {
        let table = parse_csv("name,age\nAlice,30\nBob,25\nCharlie,35").unwrap();

        let names: Vec<_> = table.column(0).collect();
        assert_eq!(names, vec!["Alice", "Bob", "Charlie"]);

        let names_by_name: Vec<_> = table.column_by_name("name").collect();
        assert_eq!(names_by_name, vec!["Alice", "Bob", "Charlie"]);
    }

    #[test]
    fn test_csv_serialization() {
        let mut table = CsvTable::with_headers(vec!["name", "age"]);
        table.push_record(vec!["Alice", "30"]);
        table.push_record(vec!["Bob", "25"]);

        let csv_str = table.to_string();
        assert!(csv_str.contains("name,age"));
        assert!(csv_str.contains("Alice,30"));
        assert!(csv_str.contains("Bob,25"));
    }

    #[test]
    fn test_csv_roundtrip() {
        let mut original = CsvTable::with_headers(vec!["id", "name", "score"]);
        original.push_record(vec!["1", "Alice", "95.5"]);
        original.push_record(vec!["2", "Bob", "87.3"]);

        let csv_str = original.to_string();
        let parsed = parse_csv(&csv_str).unwrap();

        assert_eq!(original.len(), parsed.len());
        assert_eq!(original.header_names(), parsed.header_names());
        assert_eq!(original.get(0, 1), parsed.get(0, 1));
    }

    #[test]
    fn test_serde_deserialization() {
        use serde::Deserialize;

        #[derive(Debug, Deserialize, PartialEq)]
        struct Person {
            name: String,
            age: u32,
        }

        let csv = "name,age\nAlice,30\nBob,25";
        let people: Vec<Person> = parse_csv_as(csv).unwrap();

        assert_eq!(people.len(), 2);
        assert_eq!(
            people[0],
            Person {
                name: "Alice".to_string(),
                age: 30
            }
        );
        assert_eq!(
            people[1],
            Person {
                name: "Bob".to_string(),
                age: 25
            }
        );
    }

    #[test]
    fn test_serde_serialization() {
        use serde::Serialize;

        #[derive(Serialize)]
        struct Person {
            name: String,
            age: u32,
        }

        let people = vec![
            Person {
                name: "Alice".to_string(),
                age: 30,
            },
            Person {
                name: "Bob".to_string(),
                age: 25,
            },
        ];

        let csv_str = to_csv_string(&people).unwrap();
        assert!(csv_str.contains("name,age"));
        assert!(csv_str.contains("Alice,30"));
        assert!(csv_str.contains("Bob,25"));
    }

    #[test]
    fn test_file_roundtrip() {
        let mut table = CsvTable::with_headers(vec!["name", "score"]);
        table.push_record(vec!["Test", "100"]);

        let path = std::env::temp_dir().join("horizon_csv_test.csv");

        table.save(&path).unwrap();

        let loaded = read_csv(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.get_by_name(0, "name"), Some("Test"));
        assert_eq!(loaded.get_by_name_as::<i32>(0, "score"), Some(100));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_quoted_fields() {
        let table = parse_csv(r#"name,description
"Alice","Has a comma, in description"
"Bob","Normal description""#).unwrap();

        assert_eq!(table.get_by_name(0, "description"), Some("Has a comma, in description"));
        assert_eq!(table.get_by_name(1, "description"), Some("Normal description"));
    }

    #[test]
    fn test_empty_csv() {
        let table = parse_csv("name,age").unwrap();
        assert!(table.has_headers());
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_csv_options_builder() {
        let options = CsvOptions::new()
            .delimiter(b';')
            .no_headers()
            .quote(b'\'')
            .flexible(true)
            .comment(b'#')
            .trim(true)
            .double_quote(false);

        assert_eq!(options.delimiter, b';');
        assert!(!options.has_headers);
        assert_eq!(options.quote, b'\'');
        assert!(options.flexible);
        assert_eq!(options.comment, Some(b'#'));
        assert!(options.trim);
        assert!(!options.double_quote);
    }

    #[test]
    fn test_csv_with_whitespace_trimming() {
        let options = CsvOptions::new().trim(true);
        let table = parse_csv_with_options("name , age \n Alice , 30 ", &options).unwrap();

        assert_eq!(table.get_by_name(0, "name"), Some("Alice"));
        assert_eq!(table.get_by_name_as::<i32>(0, "age"), Some(30));
    }
}
