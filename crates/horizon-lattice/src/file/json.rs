//! JSON parsing, generation, and manipulation.
//!
//! This module provides a convenient API for working with JSON data, including
//! parsing, serialization, and path-based manipulation.
//!
//! # Parsing JSON
//!
//! ```ignore
//! use horizon_lattice::file::json::{parse_json, read_json, read_json_as};
//!
//! // Parse from string
//! let value = parse_json(r#"{"name": "Alice", "age": 30}"#)?;
//!
//! // Read from file
//! let value = read_json("config.json")?;
//!
//! // Read and deserialize to a typed struct
//! #[derive(Deserialize)]
//! struct Config {
//!     name: String,
//!     age: i32,
//! }
//! let config: Config = read_json_as("config.json")?;
//! ```
//!
//! # Generating JSON
//!
//! ```ignore
//! use horizon_lattice::file::json::{write_json, write_json_pretty, to_json_string};
//!
//! // Write to file (compact)
//! write_json("output.json", &data)?;
//!
//! // Write to file (pretty-printed)
//! write_json_pretty("output.json", &data)?;
//!
//! // Convert to string
//! let json_str = to_json_string(&data)?;
//! let pretty_str = to_json_string_pretty(&data)?;
//! ```
//!
//! # Path-Based Access
//!
//! ```ignore
//! use horizon_lattice::file::json::JsonValue;
//!
//! let mut value = parse_json(r#"{"users": [{"name": "Alice"}, {"name": "Bob"}]}"#)?;
//!
//! // Access nested values using dot notation and array indexing
//! let name = value.get("users[0].name")?.as_str();
//! assert_eq!(name, Some("Alice"));
//!
//! // Modify values
//! value.set("users[0].name", "Alicia");
//! value.set("users[2].name", "Charlie"); // Creates intermediate structures
//!
//! // Remove values
//! value.remove("users[1]");
//! ```
//!
//! # Working with JsonValue
//!
//! ```ignore
//! use horizon_lattice::file::json::JsonValue;
//!
//! // Create values
//! let mut obj = JsonValue::object();
//! obj.insert_key("name", "Alice");
//! obj.insert_key("scores", vec![95, 87, 92]);
//!
//! let mut arr = JsonValue::array();
//! arr.push(1);
//! arr.push(2);
//! arr.push(3);
//!
//! // Type checking
//! assert!(obj.is_object());
//! assert!(arr.is_array());
//!
//! // Iterate over objects
//! for (key, value) in obj.entries() {
//!     println!("{}: {}", key, value);
//! }
//!
//! // Serialize
//! let json_str = obj.to_string_pretty();
//! ```

use std::fmt;
use std::ops::{Index, IndexMut};
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::error::{FileError, FileErrorKind, FileResult};
use super::operations::{atomic_write, read_text};

// Re-export serde_json::Map for convenience
pub use serde_json::Map;

/// A JSON value with convenient manipulation methods.
///
/// This is a wrapper around `serde_json::Value` that provides additional
/// convenience methods for path-based access and manipulation.
#[derive(Debug, Clone, PartialEq)]
pub struct JsonValue(serde_json::Value);

impl JsonValue {
    // ========================================================================
    // Construction
    // ========================================================================

    /// Creates a JSON null value.
    pub fn null() -> Self {
        JsonValue(serde_json::Value::Null)
    }

    /// Creates an empty JSON array.
    pub fn array() -> Self {
        JsonValue(serde_json::Value::Array(Vec::new()))
    }

    /// Creates an empty JSON object.
    pub fn object() -> Self {
        JsonValue(serde_json::Value::Object(serde_json::Map::new()))
    }

    /// Creates a JSON boolean value.
    pub fn bool(value: bool) -> Self {
        JsonValue(serde_json::Value::Bool(value))
    }

    /// Creates a JSON number from an integer.
    pub fn int(value: i64) -> Self {
        JsonValue(serde_json::Value::Number(value.into()))
    }

    /// Creates a JSON number from a float.
    ///
    /// Returns `None` if the float is NaN or infinite.
    pub fn float(value: f64) -> Option<Self> {
        serde_json::Number::from_f64(value).map(|n| JsonValue(serde_json::Value::Number(n)))
    }

    /// Creates a JSON string value.
    pub fn string(value: impl Into<String>) -> Self {
        JsonValue(serde_json::Value::String(value.into()))
    }

    /// Creates a JSON value from any serializable type.
    pub fn from_serialize<T: Serialize>(value: &T) -> FileResult<Self> {
        let json = serde_json::to_value(value).map_err(json_error)?;
        Ok(JsonValue(json))
    }

    /// Creates a JSON value from a raw serde_json::Value.
    pub fn from_raw(value: serde_json::Value) -> Self {
        JsonValue(value)
    }

    /// Returns the underlying serde_json::Value.
    pub fn into_raw(self) -> serde_json::Value {
        self.0
    }

    /// Returns a reference to the underlying serde_json::Value.
    pub fn as_raw(&self) -> &serde_json::Value {
        &self.0
    }

    /// Returns a mutable reference to the underlying serde_json::Value.
    pub fn as_raw_mut(&mut self) -> &mut serde_json::Value {
        &mut self.0
    }

    // ========================================================================
    // Type Checking
    // ========================================================================

    /// Returns true if this value is null.
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Returns true if this value is a boolean.
    pub fn is_bool(&self) -> bool {
        self.0.is_boolean()
    }

    /// Returns true if this value is a number.
    pub fn is_number(&self) -> bool {
        self.0.is_number()
    }

    /// Returns true if this value is an integer.
    pub fn is_i64(&self) -> bool {
        self.0.is_i64()
    }

    /// Returns true if this value is an unsigned integer.
    pub fn is_u64(&self) -> bool {
        self.0.is_u64()
    }

    /// Returns true if this value is a float.
    pub fn is_f64(&self) -> bool {
        self.0.is_f64()
    }

    /// Returns true if this value is a string.
    pub fn is_string(&self) -> bool {
        self.0.is_string()
    }

    /// Returns true if this value is an array.
    pub fn is_array(&self) -> bool {
        self.0.is_array()
    }

    /// Returns true if this value is an object.
    pub fn is_object(&self) -> bool {
        self.0.is_object()
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
        self.0.as_i64()
    }

    /// Returns this value as a u64, if it is an unsigned integer.
    pub fn as_u64(&self) -> Option<u64> {
        self.0.as_u64()
    }

    /// Returns this value as an f64.
    ///
    /// If the value is an integer, it will be converted to f64.
    pub fn as_f64(&self) -> Option<f64> {
        self.0.as_f64()
    }

    /// Returns this value as a string slice, if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_str()
    }

    /// Returns this value as a vector reference, if it is an array.
    pub fn as_array(&self) -> Option<Vec<&JsonValue>> {
        self.0.as_array().map(|arr| {
            // Safe because JsonValue is repr(transparent) over serde_json::Value
            arr.iter()
                .map(|v| unsafe { &*(v as *const serde_json::Value as *const JsonValue) })
                .collect()
        })
    }

    /// Returns this value as a mutable vector reference, if it is an array.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<serde_json::Value>> {
        self.0.as_array_mut()
    }

    /// Returns this value as an object reference, if it is an object.
    pub fn as_object(&self) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.0.as_object()
    }

    /// Returns this value as a mutable object reference, if it is an object.
    pub fn as_object_mut(&mut self) -> Option<&mut serde_json::Map<String, serde_json::Value>> {
        self.0.as_object_mut()
    }

    // ========================================================================
    // Path-Based Access
    // ========================================================================

    /// Gets a value at the specified path.
    ///
    /// The path uses dot notation for object keys and bracket notation for
    /// array indices. For example: `"users[0].name"` or `"config.window.width"`.
    ///
    /// Returns `None` if the path doesn't exist.
    pub fn get(&self, path: &str) -> Option<&JsonValue> {
        let parts = parse_path(path);
        let mut current = &self.0;

        for part in parts {
            match part {
                PathPart::Key(key) => {
                    current = current.get(key)?;
                }
                PathPart::Index(idx) => {
                    current = current.get(idx)?;
                }
            }
        }

        // Safe because JsonValue is repr(transparent) over serde_json::Value
        Some(unsafe { &*(current as *const serde_json::Value as *const JsonValue) })
    }

    /// Gets a mutable reference to a value at the specified path.
    ///
    /// Returns `None` if the path doesn't exist.
    pub fn get_mut(&mut self, path: &str) -> Option<&mut JsonValue> {
        let parts = parse_path(path);
        let mut current = &mut self.0;

        for part in parts {
            match part {
                PathPart::Key(key) => {
                    current = current.get_mut(key)?;
                }
                PathPart::Index(idx) => {
                    current = current.get_mut(idx)?;
                }
            }
        }

        // Safe because JsonValue is repr(transparent) over serde_json::Value
        Some(unsafe { &mut *(current as *mut serde_json::Value as *mut JsonValue) })
    }

    /// Sets a value at the specified path.
    ///
    /// Intermediate objects and arrays are created as needed. For array indices,
    /// the array will be extended with null values if necessary.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut value = JsonValue::object();
    /// value.set("user.name", "Alice");
    /// value.set("user.scores[0]", 95);
    /// value.set("user.scores[1]", 87);
    /// ```
    pub fn set<V: Into<JsonValue>>(&mut self, path: &str, value: V) {
        let value = value.into();
        let parts = parse_path(path);

        if parts.is_empty() {
            self.0 = value.0;
            return;
        }

        set_nested(&mut self.0, &parts, value.0);
    }

    /// Removes a value at the specified path.
    ///
    /// Returns the removed value, if any.
    pub fn remove(&mut self, path: &str) -> Option<JsonValue> {
        let parts = parse_path(path);
        if parts.is_empty() {
            return None;
        }

        remove_nested(&mut self.0, &parts).map(JsonValue)
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
    pub fn push<V: Into<JsonValue>>(&mut self, value: V) {
        if let Some(arr) = self.0.as_array_mut() {
            arr.push(value.into().0);
        }
    }

    /// Inserts a value at the specified index in this array.
    ///
    /// Does nothing if this is not an array or if the index is out of bounds.
    pub fn insert_at<V: Into<JsonValue>>(&mut self, index: usize, value: V) {
        if let Some(arr) = self.0.as_array_mut()
            && index <= arr.len() {
                arr.insert(index, value.into().0);
            }
    }

    /// Removes and returns the value at the specified index.
    ///
    /// Returns `None` if this is not an array or if the index is out of bounds.
    pub fn remove_at(&mut self, index: usize) -> Option<JsonValue> {
        self.0
            .as_array_mut()
            .filter(|arr| index < arr.len())
            .map(|arr| JsonValue(arr.remove(index)))
    }

    /// Returns the length of this array or object.
    ///
    /// Returns 0 for other value types.
    pub fn len(&self) -> usize {
        match &self.0 {
            serde_json::Value::Array(arr) => arr.len(),
            serde_json::Value::Object(obj) => obj.len(),
            _ => 0,
        }
    }

    /// Returns true if this array or object is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    // ========================================================================
    // Object Operations
    // ========================================================================

    /// Inserts a key-value pair into this object.
    ///
    /// Does nothing if this is not an object.
    pub fn insert_key<V: Into<JsonValue>>(&mut self, key: impl Into<String>, value: V) {
        if let Some(obj) = self.0.as_object_mut() {
            obj.insert(key.into(), value.into().0);
        }
    }

    /// Removes a key from this object.
    ///
    /// Returns the removed value, if any.
    pub fn remove_key(&mut self, key: &str) -> Option<JsonValue> {
        self.0.as_object_mut()?.remove(key).map(JsonValue)
    }

    /// Returns true if this object contains the specified key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.0
            .as_object()
            .map(|obj| obj.contains_key(key))
            .unwrap_or(false)
    }

    /// Returns an iterator over the keys of this object.
    ///
    /// Returns an empty iterator for non-objects.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0
            .as_object()
            .into_iter()
            .flat_map(|obj| obj.keys().map(|s| s.as_str()))
    }

    /// Returns an iterator over the values of this object.
    ///
    /// Returns an empty iterator for non-objects.
    pub fn values(&self) -> impl Iterator<Item = &JsonValue> {
        self.0.as_object().into_iter().flat_map(|obj| {
            obj.values()
                .map(|v| unsafe { &*(v as *const serde_json::Value as *const JsonValue) })
        })
    }

    /// Returns an iterator over the key-value pairs of this object.
    ///
    /// Returns an empty iterator for non-objects.
    pub fn entries(&self) -> impl Iterator<Item = (&str, &JsonValue)> {
        self.0.as_object().into_iter().flat_map(|obj| {
            obj.iter().map(|(k, v)| {
                (k.as_str(), unsafe {
                    &*(v as *const serde_json::Value as *const JsonValue)
                })
            })
        })
    }

    // ========================================================================
    // Iteration for Arrays
    // ========================================================================

    /// Returns an iterator over the elements of this array.
    ///
    /// Returns an empty iterator for non-arrays.
    pub fn iter(&self) -> impl Iterator<Item = &JsonValue> {
        self.0.as_array().into_iter().flat_map(|arr| {
            arr.iter()
                .map(|v| unsafe { &*(v as *const serde_json::Value as *const JsonValue) })
        })
    }

    // ========================================================================
    // Serialization
    // ========================================================================

    /// Converts this value to a compact JSON string.
    pub fn as_json_string(&self) -> String {
        self.0.to_string()
    }

    /// Converts this value to a pretty-printed JSON string.
    pub fn to_string_pretty(&self) -> String {
        serde_json::to_string_pretty(&self.0).unwrap_or_else(|_| self.0.to_string())
    }

    /// Converts this value to a JSON byte vector.
    pub fn to_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.0).unwrap_or_else(|_| self.0.to_string().into_bytes())
    }

    /// Converts this value to a pretty-printed JSON byte vector.
    pub fn to_bytes_pretty(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(&self.0).unwrap_or_else(|_| self.0.to_string().into_bytes())
    }

    /// Deserializes this value to a typed value.
    pub fn deserialize<T: DeserializeOwned>(&self) -> FileResult<T> {
        serde_json::from_value(self.0.clone()).map_err(json_error)
    }

    /// Saves this value to a file (compact).
    pub fn save(&self, path: impl AsRef<Path>) -> FileResult<()> {
        let bytes = serde_json::to_vec(&self.0).map_err(json_error)?;
        atomic_write(&path, |writer| writer.write_all(&bytes))
    }

    /// Saves this value to a file (pretty-printed).
    pub fn save_pretty(&self, path: impl AsRef<Path>) -> FileResult<()> {
        let bytes = serde_json::to_vec_pretty(&self.0).map_err(json_error)?;
        atomic_write(&path, |writer| writer.write_all(&bytes))
    }
}

// ============================================================================
// Display
// ============================================================================

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Indexing
// ============================================================================

impl Index<&str> for JsonValue {
    type Output = JsonValue;

    fn index(&self, key: &str) -> &Self::Output {
        self.get(key).expect("key not found")
    }
}

impl Index<usize> for JsonValue {
    type Output = JsonValue;

    fn index(&self, index: usize) -> &Self::Output {
        let value = self.0.get(index).expect("index out of bounds");
        unsafe { &*(value as *const serde_json::Value as *const JsonValue) }
    }
}

impl IndexMut<&str> for JsonValue {
    fn index_mut(&mut self, key: &str) -> &mut Self::Output {
        self.get_mut(key).expect("key not found")
    }
}

impl IndexMut<usize> for JsonValue {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let value = self.0.get_mut(index).expect("index out of bounds");
        unsafe { &mut *(value as *mut serde_json::Value as *mut JsonValue) }
    }
}

// ============================================================================
// From Implementations
// ============================================================================

impl From<bool> for JsonValue {
    fn from(v: bool) -> Self {
        JsonValue::bool(v)
    }
}

impl From<i32> for JsonValue {
    fn from(v: i32) -> Self {
        JsonValue::int(v as i64)
    }
}

impl From<i64> for JsonValue {
    fn from(v: i64) -> Self {
        JsonValue::int(v)
    }
}

impl From<u32> for JsonValue {
    fn from(v: u32) -> Self {
        JsonValue::int(v as i64)
    }
}

impl From<u64> for JsonValue {
    fn from(v: u64) -> Self {
        JsonValue(serde_json::Value::Number(v.into()))
    }
}

impl From<f64> for JsonValue {
    fn from(v: f64) -> Self {
        JsonValue::float(v).unwrap_or_else(JsonValue::null)
    }
}

impl From<f32> for JsonValue {
    fn from(v: f32) -> Self {
        JsonValue::float(v as f64).unwrap_or_else(JsonValue::null)
    }
}

impl From<&str> for JsonValue {
    fn from(v: &str) -> Self {
        JsonValue::string(v)
    }
}

impl From<String> for JsonValue {
    fn from(v: String) -> Self {
        JsonValue::string(v)
    }
}

impl<T: Into<JsonValue>> From<Vec<T>> for JsonValue {
    fn from(v: Vec<T>) -> Self {
        let arr: Vec<serde_json::Value> = v.into_iter().map(|x| x.into().0).collect();
        JsonValue(serde_json::Value::Array(arr))
    }
}

impl From<serde_json::Value> for JsonValue {
    fn from(v: serde_json::Value) -> Self {
        JsonValue(v)
    }
}

impl From<JsonValue> for serde_json::Value {
    fn from(v: JsonValue) -> Self {
        v.0
    }
}

impl Default for JsonValue {
    fn default() -> Self {
        JsonValue::null()
    }
}

// ============================================================================
// Serde Support
// ============================================================================

impl Serialize for JsonValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for JsonValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde_json::Value::deserialize(deserializer).map(JsonValue)
    }
}

// ============================================================================
// Module-Level Functions
// ============================================================================

/// Parses a JSON string into a JsonValue.
///
/// # Example
///
/// ```ignore
/// let value = parse_json(r#"{"name": "Alice", "age": 30}"#)?;
/// ```
pub fn parse_json(s: &str) -> FileResult<JsonValue> {
    let value: serde_json::Value = serde_json::from_str(s).map_err(json_error)?;
    Ok(JsonValue(value))
}

/// Parses a JSON string into a typed value.
///
/// # Example
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct Config { name: String, age: i32 }
///
/// let config: Config = parse_json_as(r#"{"name": "Alice", "age": 30}"#)?;
/// ```
pub fn parse_json_as<T: DeserializeOwned>(s: &str) -> FileResult<T> {
    serde_json::from_str(s).map_err(json_error)
}

/// Reads and parses a JSON file into a JsonValue.
///
/// # Example
///
/// ```ignore
/// let value = read_json("config.json")?;
/// let name = value.get("name")?.as_str();
/// ```
pub fn read_json(path: impl AsRef<Path>) -> FileResult<JsonValue> {
    let content = read_text(&path)?;
    let value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(path.as_ref().to_path_buf()),
            Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;
    Ok(JsonValue(value))
}

/// Reads and deserializes a JSON file into a typed value.
///
/// # Example
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct Config { name: String, debug: bool }
///
/// let config: Config = read_json_as("config.json")?;
/// ```
pub fn read_json_as<T: DeserializeOwned>(path: impl AsRef<Path>) -> FileResult<T> {
    let content = read_text(&path)?;
    serde_json::from_str(&content).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(path.as_ref().to_path_buf()),
            Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })
}

/// Writes a value to a JSON file (compact format).
///
/// The file is written atomically using a temporary file and rename.
///
/// # Example
///
/// ```ignore
/// write_json("config.json", &my_config)?;
/// ```
pub fn write_json<T: Serialize>(path: impl AsRef<Path>, value: &T) -> FileResult<()> {
    let bytes = serde_json::to_vec(value).map_err(json_error)?;
    atomic_write(&path, |writer| writer.write_all(&bytes))
}

/// Writes a value to a JSON file (pretty-printed format).
///
/// The file is written atomically using a temporary file and rename.
///
/// # Example
///
/// ```ignore
/// write_json_pretty("config.json", &my_config)?;
/// ```
pub fn write_json_pretty<T: Serialize>(path: impl AsRef<Path>, value: &T) -> FileResult<()> {
    let bytes = serde_json::to_vec_pretty(value).map_err(json_error)?;
    atomic_write(&path, |writer| writer.write_all(&bytes))
}

/// Converts a value to a compact JSON string.
///
/// # Example
///
/// ```ignore
/// let json = to_json_string(&my_data)?;
/// ```
pub fn to_json_string<T: Serialize>(value: &T) -> FileResult<String> {
    serde_json::to_string(value).map_err(json_error)
}

/// Converts a value to a pretty-printed JSON string.
///
/// # Example
///
/// ```ignore
/// let json = to_json_string_pretty(&my_data)?;
/// ```
pub fn to_json_string_pretty<T: Serialize>(value: &T) -> FileResult<String> {
    serde_json::to_string_pretty(value).map_err(json_error)
}

/// Converts a value to a JSON byte vector (compact format).
pub fn to_json_bytes<T: Serialize>(value: &T) -> FileResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(json_error)
}

/// Converts a value to a JSON byte vector (pretty-printed format).
pub fn to_json_bytes_pretty<T: Serialize>(value: &T) -> FileResult<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(json_error)
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// A path component for JSON navigation.
#[derive(Debug, Clone)]
enum PathPart<'a> {
    Key(&'a str),
    Index(usize),
}

/// Parses a path string into components.
///
/// Supports dot notation for keys and bracket notation for indices.
/// Examples: "foo.bar", "users[0].name", "data[0][1].value"
fn parse_path(path: &str) -> Vec<PathPart<'_>> {
    if path.is_empty() {
        return Vec::new();
    }

    let mut parts = Vec::new();
    let mut current_key_start = 0;
    let mut chars = path.char_indices().peekable();

    while let Some((i, c)) = chars.next() {
        match c {
            '.' => {
                // End current key
                if i > current_key_start {
                    parts.push(PathPart::Key(&path[current_key_start..i]));
                }
                current_key_start = i + 1;
            }
            '[' => {
                // End current key if any
                if i > current_key_start {
                    parts.push(PathPart::Key(&path[current_key_start..i]));
                }

                // Parse index
                let idx_start = i + 1;
                let mut idx_end = idx_start;

                while let Some(&(j, ch)) = chars.peek() {
                    if ch == ']' {
                        idx_end = j;
                        chars.next(); // consume ']'
                        break;
                    }
                    chars.next();
                }

                if let Ok(idx) = path[idx_start..idx_end].parse::<usize>() {
                    parts.push(PathPart::Index(idx));
                }

                // Skip any '.' after ']'
                if let Some(&(_, '.')) = chars.peek() {
                    chars.next();
                }
                current_key_start = chars.peek().map(|(i, _)| *i).unwrap_or(path.len());
            }
            _ => {}
        }
    }

    // Add final key if any
    if current_key_start < path.len() {
        parts.push(PathPart::Key(&path[current_key_start..]));
    }

    parts
}

/// Sets a nested value, creating intermediate structures as needed.
fn set_nested(current: &mut serde_json::Value, parts: &[PathPart<'_>], value: serde_json::Value) {
    if parts.is_empty() {
        *current = value;
        return;
    }

    let part = &parts[0];
    let rest = &parts[1..];

    match part {
        PathPart::Key(key) => {
            // Ensure current is an object
            if !current.is_object() {
                *current = serde_json::Value::Object(serde_json::Map::new());
            }

            let obj = current.as_object_mut().unwrap();

            if rest.is_empty() {
                obj.insert((*key).to_string(), value);
            } else {
                let entry = obj
                    .entry((*key).to_string())
                    .or_insert_with(|| serde_json::Value::Null);
                set_nested(entry, rest, value);
            }
        }
        PathPart::Index(idx) => {
            // Ensure current is an array
            if !current.is_array() {
                *current = serde_json::Value::Array(Vec::new());
            }

            let arr = current.as_array_mut().unwrap();

            // Extend array if needed
            while arr.len() <= *idx {
                arr.push(serde_json::Value::Null);
            }

            if rest.is_empty() {
                arr[*idx] = value;
            } else {
                set_nested(&mut arr[*idx], rest, value);
            }
        }
    }
}

/// Removes a nested value.
fn remove_nested(
    current: &mut serde_json::Value,
    parts: &[PathPart<'_>],
) -> Option<serde_json::Value> {
    if parts.is_empty() {
        return None;
    }

    let part = &parts[0];
    let rest = &parts[1..];

    match part {
        PathPart::Key(key) => {
            let obj = current.as_object_mut()?;

            if rest.is_empty() {
                obj.remove(*key)
            } else {
                let value = obj.get_mut(*key)?;
                remove_nested(value, rest)
            }
        }
        PathPart::Index(idx) => {
            let arr = current.as_array_mut()?;

            if *idx >= arr.len() {
                return None;
            }

            if rest.is_empty() {
                Some(arr.remove(*idx))
            } else {
                remove_nested(&mut arr[*idx], rest)
            }
        }
    }
}

/// Converts a serde_json error to a FileError.
fn json_error(e: serde_json::Error) -> FileError {
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
    fn test_parse_json() {
        let value = parse_json(r#"{"name": "Alice", "age": 30}"#).unwrap();
        assert!(value.is_object());
        assert_eq!(value.get("name").unwrap().as_str(), Some("Alice"));
        assert_eq!(value.get("age").unwrap().as_i64(), Some(30));
    }

    #[test]
    fn test_json_value_construction() {
        assert!(JsonValue::null().is_null());
        assert!(JsonValue::bool(true).is_bool());
        assert!(JsonValue::int(42).is_number());
        assert!(JsonValue::string("hello").is_string());
        assert!(JsonValue::array().is_array());
        assert!(JsonValue::object().is_object());
    }

    #[test]
    fn test_path_based_access() {
        let value = parse_json(
            r#"{
            "users": [
                {"name": "Alice", "scores": [95, 87]},
                {"name": "Bob", "scores": [82, 91]}
            ],
            "config": {
                "theme": "dark"
            }
        }"#,
        )
        .unwrap();

        assert_eq!(value.get("users[0].name").unwrap().as_str(), Some("Alice"));
        assert_eq!(value.get("users[1].name").unwrap().as_str(), Some("Bob"));
        assert_eq!(value.get("users[0].scores[0]").unwrap().as_i64(), Some(95));
        assert_eq!(value.get("config.theme").unwrap().as_str(), Some("dark"));
    }

    #[test]
    fn test_path_based_modification() {
        let mut value = JsonValue::object();

        value.set("user.name", "Alice");
        value.set("user.scores[0]", 95);
        value.set("user.scores[1]", 87);
        value.set("config.theme", "dark");

        assert_eq!(value.get("user.name").unwrap().as_str(), Some("Alice"));
        assert_eq!(value.get("user.scores[0]").unwrap().as_i64(), Some(95));
        assert_eq!(value.get("user.scores[1]").unwrap().as_i64(), Some(87));
        assert_eq!(value.get("config.theme").unwrap().as_str(), Some("dark"));
    }

    #[test]
    fn test_remove() {
        let mut value =
            parse_json(r#"{"users": [{"name": "Alice"}, {"name": "Bob"}], "count": 2}"#).unwrap();

        let removed = value.remove("count");
        assert_eq!(removed.unwrap().as_i64(), Some(2));
        assert!(value.get("count").is_none());

        let removed = value.remove("users[0]");
        assert_eq!(
            removed.unwrap().get("name").unwrap().as_str(),
            Some("Alice")
        );
        assert_eq!(value.get("users[0].name").unwrap().as_str(), Some("Bob"));
    }

    #[test]
    fn test_array_operations() {
        let mut arr = JsonValue::array();

        arr.push(1);
        arr.push(2);
        arr.push(3);

        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_i64(), Some(1));
        assert_eq!(arr[1].as_i64(), Some(2));
        assert_eq!(arr[2].as_i64(), Some(3));

        arr.insert_at(1, 10);
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[1].as_i64(), Some(10));
        assert_eq!(arr[2].as_i64(), Some(2));

        let removed = arr.remove_at(1);
        assert_eq!(removed.unwrap().as_i64(), Some(10));
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn test_object_operations() {
        let mut obj = JsonValue::object();

        obj.insert_key("name", "Alice");
        obj.insert_key("age", 30);

        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("name"));
        assert!(!obj.contains_key("email"));

        let keys: Vec<_> = obj.keys().collect();
        assert!(keys.contains(&"name"));
        assert!(keys.contains(&"age"));

        let removed = obj.remove_key("age");
        assert_eq!(removed.unwrap().as_i64(), Some(30));
        assert_eq!(obj.len(), 1);
    }

    #[test]
    fn test_iteration() {
        let arr = parse_json("[1, 2, 3]").unwrap();
        let values: Vec<i64> = arr.iter().filter_map(|v| v.as_i64()).collect();
        assert_eq!(values, vec![1, 2, 3]);

        let obj = parse_json(r#"{"a": 1, "b": 2}"#).unwrap();
        let entries: Vec<_> = obj.entries().collect();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_serialization() {
        let mut obj = JsonValue::object();
        obj.insert_key("name", "Alice");
        obj.insert_key("age", 30);

        let compact = obj.to_string();
        assert!(compact.contains("name"));
        assert!(compact.contains("Alice"));

        let pretty = obj.to_string_pretty();
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn test_from_serialize() {
        use serde::Serialize;

        #[derive(Serialize)]
        struct User {
            name: String,
            age: i32,
        }

        let user = User {
            name: "Alice".to_string(),
            age: 30,
        };

        let value = JsonValue::from_serialize(&user).unwrap();
        assert_eq!(value.get("name").unwrap().as_str(), Some("Alice"));
        assert_eq!(value.get("age").unwrap().as_i64(), Some(30));
    }

    #[test]
    fn test_deserialize() {
        use serde::Deserialize;

        #[derive(Deserialize, Debug, PartialEq)]
        struct User {
            name: String,
            age: i32,
        }

        let value = parse_json(r#"{"name": "Alice", "age": 30}"#).unwrap();
        let user: User = value.deserialize().unwrap();

        assert_eq!(
            user,
            User {
                name: "Alice".to_string(),
                age: 30
            }
        );
    }

    #[test]
    fn test_file_roundtrip() {
        let mut value = JsonValue::object();
        value.insert_key("name", "Test");
        value.insert_key("count", 42);
        value.set("nested.value", true);

        let path = std::env::temp_dir().join("horizon_json_test.json");

        value.save_pretty(&path).unwrap();

        let loaded = read_json(&path).unwrap();
        assert_eq!(loaded.get("name").unwrap().as_str(), Some("Test"));
        assert_eq!(loaded.get("count").unwrap().as_i64(), Some(42));
        assert_eq!(loaded.get("nested.value").unwrap().as_bool(), Some(true));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_parse_path() {
        let parts = parse_path("users[0].name");
        assert_eq!(parts.len(), 3);

        let parts = parse_path("config.window.width");
        assert_eq!(parts.len(), 3);

        let parts = parse_path("data[0][1][2]");
        assert_eq!(parts.len(), 4); // "data" + 3 indices

        let parts = parse_path("");
        assert_eq!(parts.len(), 0);
    }
}
