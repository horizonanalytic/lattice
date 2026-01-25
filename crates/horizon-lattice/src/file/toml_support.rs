//! TOML parsing, generation, and manipulation.
//!
//! This module provides a convenient API for working with TOML data, including
//! parsing, serialization, and manipulation. TOML is particularly well-suited
//! for configuration files.
//!
//! # Parsing TOML
//!
//! ```ignore
//! use horizon_lattice::file::toml_support::{parse_toml, read_toml, read_toml_as};
//!
//! // Parse from string
//! let value = parse_toml(r#"
//! [database]
//! host = "localhost"
//! port = 5432
//! "#)?;
//!
//! // Read from file
//! let value = read_toml("config.toml")?;
//!
//! // Read and deserialize to a typed struct
//! #[derive(Deserialize)]
//! struct Config {
//!     database: Database,
//! }
//! let config: Config = read_toml_as("config.toml")?;
//! ```
//!
//! # Generating TOML
//!
//! ```ignore
//! use horizon_lattice::file::toml_support::{write_toml, to_toml_string};
//!
//! // Write to file
//! write_toml("output.toml", &data)?;
//!
//! // Convert to string
//! let toml_str = to_toml_string(&data)?;
//! ```
//!
//! # Working with TomlValue
//!
//! ```ignore
//! use horizon_lattice::file::toml_support::TomlValue;
//!
//! let value = parse_toml(r#"
//! [server]
//! host = "localhost"
//! port = 8080
//! "#)?;
//!
//! // Access nested values using dot notation
//! let host = value.get("server.host")?.as_str();
//! let port = value.get("server.port")?.as_i64();
//!
//! // Modify values
//! let mut value = TomlValue::table();
//! value.set("server.host", "127.0.0.1");
//! value.set("server.port", 3000);
//! ```

use std::fmt;
use std::ops::Index;
use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

use super::error::{FileError, FileErrorKind, FileResult};
use super::operations::{atomic_write, read_text};

/// A TOML value with convenient manipulation methods.
///
/// This is a wrapper around `toml::Value` that provides additional
/// convenience methods for path-based access and manipulation.
#[derive(Debug, Clone, PartialEq)]
pub struct TomlValue(toml::Value);

impl TomlValue {
    // ========================================================================
    // Construction
    // ========================================================================

    /// Creates an empty TOML table (object).
    pub fn table() -> Self {
        TomlValue(toml::Value::Table(toml::Table::new()))
    }

    /// Creates an empty TOML array.
    pub fn array() -> Self {
        TomlValue(toml::Value::Array(Vec::new()))
    }

    /// Creates a TOML boolean value.
    pub fn bool(value: bool) -> Self {
        TomlValue(toml::Value::Boolean(value))
    }

    /// Creates a TOML integer value.
    pub fn int(value: i64) -> Self {
        TomlValue(toml::Value::Integer(value))
    }

    /// Creates a TOML float value.
    pub fn float(value: f64) -> Self {
        TomlValue(toml::Value::Float(value))
    }

    /// Creates a TOML string value.
    pub fn string(value: impl Into<String>) -> Self {
        TomlValue(toml::Value::String(value.into()))
    }

    /// Creates a TOML value from any serializable type.
    pub fn from_serialize<T: Serialize>(value: &T) -> FileResult<Self> {
        let toml = toml::Value::try_from(value).map_err(toml_ser_error)?;
        Ok(TomlValue(toml))
    }

    /// Creates a TOML value from a raw toml::Value.
    pub fn from_raw(value: toml::Value) -> Self {
        TomlValue(value)
    }

    /// Returns the underlying toml::Value.
    pub fn into_raw(self) -> toml::Value {
        self.0
    }

    /// Returns a reference to the underlying toml::Value.
    pub fn as_raw(&self) -> &toml::Value {
        &self.0
    }

    /// Returns a mutable reference to the underlying toml::Value.
    pub fn as_raw_mut(&mut self) -> &mut toml::Value {
        &mut self.0
    }

    // ========================================================================
    // Type Checking
    // ========================================================================

    /// Returns true if this value is a boolean.
    pub fn is_bool(&self) -> bool {
        self.0.is_bool()
    }

    /// Returns true if this value is an integer.
    pub fn is_integer(&self) -> bool {
        self.0.is_integer()
    }

    /// Returns true if this value is a float.
    pub fn is_float(&self) -> bool {
        self.0.is_float()
    }

    /// Returns true if this value is a string.
    pub fn is_string(&self) -> bool {
        self.0.is_str()
    }

    /// Returns true if this value is an array.
    pub fn is_array(&self) -> bool {
        self.0.is_array()
    }

    /// Returns true if this value is a table (object).
    pub fn is_table(&self) -> bool {
        self.0.is_table()
    }

    /// Returns true if this value is a datetime.
    pub fn is_datetime(&self) -> bool {
        self.0.is_datetime()
    }

    // ========================================================================
    // Value Extraction
    // ========================================================================

    /// Returns this value as a boolean, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        self.0.as_bool()
    }

    /// Returns this value as an i64, if it is an integer.
    pub fn as_i64(&self) -> Option<i64> {
        self.0.as_integer()
    }

    /// Returns this value as an f64, if it is a float.
    pub fn as_f64(&self) -> Option<f64> {
        self.0.as_float()
    }

    /// Returns this value as a string slice, if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_str()
    }

    /// Returns this value as an array reference, if it is an array.
    pub fn as_array(&self) -> Option<Vec<&TomlValue>> {
        self.0.as_array().map(|arr| {
            arr.iter()
                .map(|v| unsafe { &*(v as *const toml::Value as *const TomlValue) })
                .collect()
        })
    }

    /// Returns this value as a mutable array reference, if it is an array.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<toml::Value>> {
        self.0.as_array_mut()
    }

    /// Returns this value as a table reference, if it is a table.
    pub fn as_table(&self) -> Option<&toml::Table> {
        self.0.as_table()
    }

    /// Returns this value as a mutable table reference, if it is a table.
    pub fn as_table_mut(&mut self) -> Option<&mut toml::Table> {
        self.0.as_table_mut()
    }

    // ========================================================================
    // Path-Based Access
    // ========================================================================

    /// Gets a value at the specified path.
    ///
    /// The path uses dot notation for table keys. For example: `"database.host"`
    /// or `"server.port"`.
    ///
    /// Returns `None` if the path doesn't exist.
    pub fn get(&self, path: &str) -> Option<&TomlValue> {
        let parts: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        let mut current = &self.0;

        for part in parts {
            current = current.get(part)?;
        }

        Some(unsafe { &*(current as *const toml::Value as *const TomlValue) })
    }

    /// Gets a mutable reference to a value at the specified path.
    ///
    /// Returns `None` if the path doesn't exist.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut TomlValue> {
        let parts: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        let mut current = &mut self.0;

        for part in parts {
            current = current.get_mut(part)?;
        }

        Some(unsafe { &mut *(current as *mut toml::Value as *mut TomlValue) })
    }

    /// Sets a value at the specified path.
    ///
    /// Intermediate tables are created as needed.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut value = TomlValue::table();
    /// value.set("database.host", "localhost");
    /// value.set("database.port", 5432);
    /// ```
    pub fn set<V: Into<TomlValue>>(&mut self, path: &str, value: V) {
        let value = value.into();
        let parts: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();

        if parts.is_empty() {
            self.0 = value.0;
            return;
        }

        set_nested(&mut self.0, &parts, value.0);
    }

    /// Removes a value at the specified path.
    ///
    /// Returns the removed value, if any.
    pub fn remove(&mut self, path: &str) -> Option<TomlValue> {
        let parts: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
        if parts.is_empty() {
            return None;
        }

        remove_nested(&mut self.0, &parts).map(TomlValue)
    }

    /// Returns true if a value exists at the specified path.
    pub fn contains(&self, path: &str) -> bool {
        self.get(path).is_some()
    }

    // ========================================================================
    // Array Operations
    // ========================================================================

    /// Pushes a value to the end of this array.
    ///
    /// Does nothing if this is not an array.
    pub fn push<V: Into<TomlValue>>(&mut self, value: V) {
        if let Some(arr) = self.0.as_array_mut() {
            arr.push(value.into().0);
        }
    }

    /// Returns the length of this array or table.
    ///
    /// Returns 0 for other value types.
    pub fn len(&self) -> usize {
        match &self.0 {
            toml::Value::Array(arr) => arr.len(),
            toml::Value::Table(table) => table.len(),
            _ => 0,
        }
    }

    /// Returns true if this array or table is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // ========================================================================
    // Table Operations
    // ========================================================================

    /// Inserts a key-value pair into this table.
    ///
    /// Does nothing if this is not a table.
    pub fn insert_key<V: Into<TomlValue>>(&mut self, key: impl Into<String>, value: V) {
        if let Some(table) = self.0.as_table_mut() {
            table.insert(key.into(), value.into().0);
        }
    }

    /// Removes a key from this table.
    ///
    /// Returns the removed value, if any.
    pub fn remove_key(&mut self, key: &str) -> Option<TomlValue> {
        self.0.as_table_mut()?.remove(key).map(TomlValue)
    }

    /// Returns true if this table contains the specified key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.0
            .as_table()
            .map(|table| table.contains_key(key))
            .unwrap_or(false)
    }

    /// Returns an iterator over the keys of this table.
    ///
    /// Returns an empty iterator for non-tables.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0
            .as_table()
            .into_iter()
            .flat_map(|table| table.keys().map(|s| s.as_str()))
    }

    /// Returns an iterator over the values of this table.
    ///
    /// Returns an empty iterator for non-tables.
    pub fn values(&self) -> impl Iterator<Item = &TomlValue> {
        self.0.as_table().into_iter().flat_map(|table| {
            table
                .values()
                .map(|v| unsafe { &*(v as *const toml::Value as *const TomlValue) })
        })
    }

    /// Returns an iterator over the key-value pairs of this table.
    ///
    /// Returns an empty iterator for non-tables.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &TomlValue)> {
        self.0.as_table().into_iter().flat_map(|table| {
            table.iter().map(|(k, v)| {
                (
                    k.as_str(),
                    unsafe { &*(v as *const toml::Value as *const TomlValue) },
                )
            })
        })
    }

    // ========================================================================
    // Iteration for Arrays
    // ========================================================================

    /// Returns an iterator over the elements of this array.
    ///
    /// Returns an empty iterator for non-arrays.
    pub fn iter(&self) -> impl Iterator<Item = &TomlValue> {
        self.0.as_array().into_iter().flat_map(|arr| {
            arr.iter()
                .map(|v| unsafe { &*(v as *const toml::Value as *const TomlValue) })
        })
    }

    // ========================================================================
    // Serialization
    // ========================================================================

    /// Converts this value to a TOML string.
    pub fn to_string(&self) -> FileResult<String> {
        toml::to_string(&self.0).map_err(toml_ser_error)
    }

    /// Converts this value to a pretty-printed TOML string.
    pub fn to_string_pretty(&self) -> FileResult<String> {
        toml::to_string_pretty(&self.0).map_err(toml_ser_error)
    }

    /// Deserializes this value to a typed value.
    pub fn deserialize<T: DeserializeOwned>(&self) -> FileResult<T> {
        self.0.clone().try_into().map_err(toml_de_error)
    }

    /// Saves this value to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> FileResult<()> {
        let content = self.to_string_pretty()?;
        atomic_write(&path, |writer| writer.write_all(content.as_bytes()))
    }
}

// ============================================================================
// Display
// ============================================================================

impl fmt::Display for TomlValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_string() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "{:?}", self.0),
        }
    }
}

// ============================================================================
// Indexing
// ============================================================================

impl Index<&str> for TomlValue {
    type Output = TomlValue;

    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("key not found")
    }
}

impl Index<usize> for TomlValue {
    type Output = TomlValue;

    fn index(&self, index: usize) -> &Self::Output {
        let value = self.0.get(index).expect("index out of bounds");
        unsafe { &*(value as *const toml::Value as *const TomlValue) }
    }
}

// ============================================================================
// From Implementations
// ============================================================================

impl From<bool> for TomlValue {
    fn from(v: bool) -> Self {
        TomlValue::bool(v)
    }
}

impl From<i32> for TomlValue {
    fn from(v: i32) -> Self {
        TomlValue::int(v as i64)
    }
}

impl From<i64> for TomlValue {
    fn from(v: i64) -> Self {
        TomlValue::int(v)
    }
}

impl From<f64> for TomlValue {
    fn from(v: f64) -> Self {
        TomlValue::float(v)
    }
}

impl From<f32> for TomlValue {
    fn from(v: f32) -> Self {
        TomlValue::float(v as f64)
    }
}

impl From<&str> for TomlValue {
    fn from(v: &str) -> Self {
        TomlValue::string(v)
    }
}

impl From<String> for TomlValue {
    fn from(v: String) -> Self {
        TomlValue::string(v)
    }
}

impl<T: Into<TomlValue>> From<Vec<T>> for TomlValue {
    fn from(v: Vec<T>) -> Self {
        let arr: Vec<toml::Value> = v.into_iter().map(|x| x.into().0).collect();
        TomlValue(toml::Value::Array(arr))
    }
}

impl From<toml::Value> for TomlValue {
    fn from(v: toml::Value) -> Self {
        TomlValue(v)
    }
}

impl From<TomlValue> for toml::Value {
    fn from(v: TomlValue) -> Self {
        v.0
    }
}

impl Default for TomlValue {
    fn default() -> Self {
        TomlValue::table()
    }
}

// ============================================================================
// Serde Support
// ============================================================================

impl Serialize for TomlValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for TomlValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        toml::Value::deserialize(deserializer).map(TomlValue)
    }
}

// ============================================================================
// Module-Level Functions
// ============================================================================

/// Parses a TOML string into a TomlValue.
///
/// # Example
///
/// ```ignore
/// let value = parse_toml(r#"
/// [server]
/// host = "localhost"
/// port = 8080
/// "#)?;
/// ```
pub fn parse_toml(s: &str) -> FileResult<TomlValue> {
    let value: toml::Value = s.parse().map_err(toml_de_error)?;
    Ok(TomlValue(value))
}

/// Parses a TOML string into a typed value.
///
/// # Example
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct Config {
///     server: Server,
/// }
///
/// let config: Config = parse_toml_as(toml_str)?;
/// ```
pub fn parse_toml_as<T: DeserializeOwned>(s: &str) -> FileResult<T> {
    toml::from_str(s).map_err(toml_de_error)
}

/// Reads and parses a TOML file into a TomlValue.
///
/// # Example
///
/// ```ignore
/// let value = read_toml("config.toml")?;
/// let host = value.get("server.host")?.as_str();
/// ```
pub fn read_toml(path: impl AsRef<Path>) -> FileResult<TomlValue> {
    let content = read_text(&path)?;
    let value: toml::Value = content.parse().map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(path.as_ref().to_path_buf()),
            Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;
    Ok(TomlValue(value))
}

/// Reads and deserializes a TOML file into a typed value.
///
/// # Example
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct Config {
///     database: Database,
/// }
///
/// let config: Config = read_toml_as("config.toml")?;
/// ```
pub fn read_toml_as<T: DeserializeOwned>(path: impl AsRef<Path>) -> FileResult<T> {
    let content = read_text(&path)?;
    toml::from_str(&content).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(path.as_ref().to_path_buf()),
            Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })
}

/// Writes a value to a TOML file.
///
/// The file is written atomically using a temporary file and rename.
///
/// # Example
///
/// ```ignore
/// write_toml("config.toml", &my_config)?;
/// ```
pub fn write_toml<T: Serialize>(path: impl AsRef<Path>, value: &T) -> FileResult<()> {
    let content = toml::to_string_pretty(value).map_err(toml_ser_error)?;
    atomic_write(&path, |writer| writer.write_all(content.as_bytes()))
}

/// Converts a value to a TOML string.
///
/// # Example
///
/// ```ignore
/// let toml = to_toml_string(&my_data)?;
/// ```
pub fn to_toml_string<T: Serialize>(value: &T) -> FileResult<String> {
    toml::to_string(value).map_err(toml_ser_error)
}

/// Converts a value to a pretty-printed TOML string.
///
/// # Example
///
/// ```ignore
/// let toml = to_toml_string_pretty(&my_data)?;
/// ```
pub fn to_toml_string_pretty<T: Serialize>(value: &T) -> FileResult<String> {
    toml::to_string_pretty(value).map_err(toml_ser_error)
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Sets a nested value, creating intermediate tables as needed.
fn set_nested(current: &mut toml::Value, parts: &[&str], value: toml::Value) {
    if parts.is_empty() {
        *current = value;
        return;
    }

    let key = parts[0];
    let rest = &parts[1..];

    // Ensure current is a table
    if !current.is_table() {
        *current = toml::Value::Table(toml::Table::new());
    }

    let table = current.as_table_mut().unwrap();

    if rest.is_empty() {
        table.insert(key.to_string(), value);
    } else {
        let entry = table
            .entry(key.to_string())
            .or_insert_with(|| toml::Value::Table(toml::Table::new()));
        set_nested(entry, rest, value);
    }
}

/// Removes a nested value.
fn remove_nested(current: &mut toml::Value, parts: &[&str]) -> Option<toml::Value> {
    if parts.is_empty() {
        return None;
    }

    let key = parts[0];
    let rest = &parts[1..];

    let table = current.as_table_mut()?;

    if rest.is_empty() {
        table.remove(key)
    } else {
        let value = table.get_mut(key)?;
        remove_nested(value, rest)
    }
}

/// Converts a toml serialization error to a FileError.
fn toml_ser_error(e: toml::ser::Error) -> FileError {
    FileError::new(
        FileErrorKind::InvalidData,
        None,
        Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    )
}

/// Converts a toml deserialization error to a FileError.
fn toml_de_error(e: toml::de::Error) -> FileError {
    FileError::new(
        FileErrorKind::InvalidData,
        None,
        Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_toml() {
        let value = parse_toml(
            r#"
            [server]
            host = "localhost"
            port = 8080
            "#,
        )
        .unwrap();

        assert!(value.is_table());
        assert_eq!(
            value.get("server.host").unwrap().as_str(),
            Some("localhost")
        );
        assert_eq!(value.get("server.port").unwrap().as_i64(), Some(8080));
    }

    #[test]
    fn test_toml_value_construction() {
        assert!(TomlValue::bool(true).is_bool());
        assert!(TomlValue::int(42).is_integer());
        assert!(TomlValue::float(3.14).is_float());
        assert!(TomlValue::string("hello").is_string());
        assert!(TomlValue::array().is_array());
        assert!(TomlValue::table().is_table());
    }

    #[test]
    fn test_path_based_access() {
        let value = parse_toml(
            r#"
            [database]
            host = "localhost"
            port = 5432

            [database.pool]
            max_connections = 10
            "#,
        )
        .unwrap();

        assert_eq!(
            value.get("database.host").unwrap().as_str(),
            Some("localhost")
        );
        assert_eq!(value.get("database.port").unwrap().as_i64(), Some(5432));
        assert_eq!(
            value.get("database.pool.max_connections").unwrap().as_i64(),
            Some(10)
        );
    }

    #[test]
    fn test_path_based_modification() {
        let mut value = TomlValue::table();

        value.set("server.host", "127.0.0.1");
        value.set("server.port", 3000);
        value.set("database.name", "mydb");

        assert_eq!(
            value.get("server.host").unwrap().as_str(),
            Some("127.0.0.1")
        );
        assert_eq!(value.get("server.port").unwrap().as_i64(), Some(3000));
        assert_eq!(value.get("database.name").unwrap().as_str(), Some("mydb"));
    }

    #[test]
    fn test_remove() {
        let mut value = parse_toml(
            r#"
            [server]
            host = "localhost"
            port = 8080
            "#,
        )
        .unwrap();

        let removed = value.remove("server.port");
        assert_eq!(removed.unwrap().as_i64(), Some(8080));
        assert!(value.get("server.port").is_none());
        assert!(value.get("server.host").is_some());
    }

    #[test]
    fn test_array_operations() {
        let mut arr = TomlValue::array();

        arr.push(1);
        arr.push(2);
        arr.push(3);

        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_i64(), Some(1));
        assert_eq!(arr[1].as_i64(), Some(2));
        assert_eq!(arr[2].as_i64(), Some(3));
    }

    #[test]
    fn test_table_operations() {
        let mut table = TomlValue::table();

        table.insert_key("name", "Alice");
        table.insert_key("age", 30);

        assert_eq!(table.len(), 2);
        assert!(table.contains_key("name"));
        assert!(!table.contains_key("email"));

        let keys: Vec<_> = table.keys().collect();
        assert!(keys.contains(&"name"));
        assert!(keys.contains(&"age"));

        let removed = table.remove_key("age");
        assert_eq!(removed.unwrap().as_i64(), Some(30));
        assert_eq!(table.len(), 1);
    }

    #[test]
    fn test_iteration() {
        let value = parse_toml(
            r#"
            values = [1, 2, 3]
            "#,
        )
        .unwrap();

        let arr = value.get("values").unwrap();
        let values: Vec<i64> = arr.iter().filter_map(|v| v.as_i64()).collect();
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn test_serialization() {
        let mut table = TomlValue::table();
        table.insert_key("name", "Test");
        table.insert_key("count", 42);

        let toml_str = table.to_string_pretty().unwrap();
        assert!(toml_str.contains("name"));
        assert!(toml_str.contains("Test"));
        assert!(toml_str.contains("count"));
        assert!(toml_str.contains("42"));
    }

    #[test]
    fn test_from_serialize() {
        use serde::Serialize;

        #[derive(Serialize)]
        struct Config {
            name: String,
            port: i32,
        }

        let config = Config {
            name: "myapp".to_string(),
            port: 8080,
        };

        let value = TomlValue::from_serialize(&config).unwrap();
        assert_eq!(value.get("name").unwrap().as_str(), Some("myapp"));
        assert_eq!(value.get("port").unwrap().as_i64(), Some(8080));
    }

    #[test]
    fn test_deserialize() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct Server {
            host: String,
            port: i32,
        }

        let value = parse_toml(
            r#"
            host = "localhost"
            port = 8080
            "#,
        )
        .unwrap();

        let server: Server = value.deserialize().unwrap();
        assert_eq!(
            server,
            Server {
                host: "localhost".to_string(),
                port: 8080
            }
        );
    }

    #[test]
    fn test_file_roundtrip() {
        let mut value = TomlValue::table();
        value.set("app.name", "Test");
        value.set("app.version", "1.0.0");
        value.set("server.port", 3000);

        let path = std::env::temp_dir().join("horizon_toml_test.toml");

        value.save(&path).unwrap();

        let loaded = read_toml(&path).unwrap();
        assert_eq!(loaded.get("app.name").unwrap().as_str(), Some("Test"));
        assert_eq!(loaded.get("app.version").unwrap().as_str(), Some("1.0.0"));
        assert_eq!(loaded.get("server.port").unwrap().as_i64(), Some(3000));

        std::fs::remove_file(&path).ok();
    }
}
