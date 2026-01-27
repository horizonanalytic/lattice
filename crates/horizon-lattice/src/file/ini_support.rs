//! INI file parsing, generation, and manipulation.
//!
//! This module provides a convenient API for working with INI configuration files,
//! including parsing, serialization, and manipulation. INI format is a simple
//! configuration file format commonly used for application settings.
//!
//! # Parsing INI
//!
//! ```ignore
//! use horizon_lattice::file::ini_support::{parse_ini, read_ini};
//!
//! // Parse from string
//! let value = parse_ini(r#"
//! [database]
//! host = localhost
//! port = 5432
//! "#)?;
//!
//! // Read from file
//! let value = read_ini("config.ini")?;
//! ```
//!
//! # Generating INI
//!
//! ```ignore
//! use horizon_lattice::file::ini_support::{write_ini, to_ini_string};
//!
//! // Write to file
//! write_ini("output.ini", &value)?;
//!
//! // Convert to string
//! let ini_str = to_ini_string(&value)?;
//! ```
//!
//! # Working with IniValue
//!
//! ```ignore
//! use horizon_lattice::file::ini_support::IniValue;
//!
//! let value = parse_ini(r#"
//! [server]
//! host = localhost
//! port = 8080
//! "#)?;
//!
//! // Access values using section.key notation
//! let host = value.get("server.host")?.as_str();
//! let port = value.get("server.port")?.as_i64();
//!
//! // Modify values
//! let mut value = IniValue::new();
//! value.set("server.host", "127.0.0.1");
//! value.set("server.port", "3000");
//! ```

use std::collections::HashMap;
use std::fmt;
use std::path::Path;

use ini::Ini;

use super::error::{FileError, FileErrorKind, FileResult};
use super::operations::{atomic_write, read_text};

/// An INI configuration value with convenient manipulation methods.
///
/// INI files organize values into sections. This wrapper provides
/// path-based access using "section.key" notation.
#[derive(Debug, Clone, PartialEq)]
pub struct IniValue {
    sections: HashMap<String, HashMap<String, String>>,
    /// Values in the global (unnamed) section
    global: HashMap<String, String>,
}

impl IniValue {
    // ========================================================================
    // Construction
    // ========================================================================

    /// Creates an empty INI value.
    pub fn new() -> Self {
        Self {
            sections: HashMap::new(),
            global: HashMap::new(),
        }
    }

    /// Creates an INI value from a parsed Ini.
    fn from_ini(ini: &Ini) -> Self {
        let mut sections = HashMap::new();
        let mut global = HashMap::new();

        for (section, props) in ini.iter() {
            let mut section_map = HashMap::new();
            for (key, value) in props.iter() {
                section_map.insert(key.to_string(), value.to_string());
            }

            match section {
                Some(name) => {
                    sections.insert(name.to_string(), section_map);
                }
                None => {
                    global = section_map;
                }
            }
        }

        Self { sections, global }
    }

    /// Converts this value to an Ini structure.
    fn to_ini(&self) -> Ini {
        let mut ini = Ini::new();

        // Add global properties
        for (key, value) in &self.global {
            ini.with_section(None::<String>).set(key, value);
        }

        // Add section properties
        for (section, props) in &self.sections {
            for (key, value) in props {
                ini.with_section(Some(section.clone())).set(key, value);
            }
        }

        ini
    }

    // ========================================================================
    // Type Checking
    // ========================================================================

    /// Returns true if this INI value has no sections or global properties.
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty() && self.global.is_empty()
    }

    /// Returns the number of sections (excluding global).
    pub fn section_count(&self) -> usize {
        self.sections.len()
    }

    /// Returns true if a section exists.
    pub fn has_section(&self, section: &str) -> bool {
        self.sections.contains_key(section)
    }

    // ========================================================================
    // Value Extraction
    // ========================================================================

    /// Gets a raw string value at the specified path.
    ///
    /// The path uses "section.key" notation. For global (sectionless) keys,
    /// use just the key name.
    ///
    /// Returns `None` if the path doesn't exist.
    pub fn get_str(&self, path: &str) -> Option<&str> {
        let (section, key) = parse_path(path);

        match section {
            Some(section_name) => self
                .sections
                .get(section_name)
                .and_then(|s| s.get(key))
                .map(|s| s.as_str()),
            None => self.global.get(key).map(|s| s.as_str()),
        }
    }

    /// Gets a value and tries to parse it as the specified type.
    pub fn get<T: std::str::FromStr>(&self, path: &str) -> Option<T> {
        self.get_str(path).and_then(|s| s.parse().ok())
    }

    /// Gets a value at the specified path, returning it as a string reference.
    ///
    /// This is an alias for `get_str` for API consistency with JSON/TOML modules.
    pub fn as_str(&self, path: &str) -> Option<&str> {
        self.get_str(path)
    }

    /// Gets a value as an i64.
    pub fn as_i64(&self, path: &str) -> Option<i64> {
        self.get(path)
    }

    /// Gets a value as an f64.
    pub fn as_f64(&self, path: &str) -> Option<f64> {
        self.get(path)
    }

    /// Gets a value as a bool.
    ///
    /// Recognizes: true/false, yes/no, 1/0, on/off (case-insensitive).
    pub fn as_bool(&self, path: &str) -> Option<bool> {
        self.get_str(path).and_then(|s| {
            match s.to_lowercase().as_str() {
                "true" | "yes" | "1" | "on" => Some(true),
                "false" | "no" | "0" | "off" => Some(false),
                _ => None,
            }
        })
    }

    // ========================================================================
    // Path-Based Access (JsonValue/TomlValue compatible)
    // ========================================================================

    /// Gets an IniValueRef at the specified path for API compatibility.
    ///
    /// Returns `None` if the path doesn't exist.
    pub fn get_ref(&self, path: &str) -> Option<IniValueRef<'_>> {
        self.get_str(path).map(IniValueRef)
    }

    // ========================================================================
    // Modification
    // ========================================================================

    /// Sets a value at the specified path.
    ///
    /// The path uses "section.key" notation. For global (sectionless) keys,
    /// use just the key name.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let mut value = IniValue::new();
    /// value.set("server.host", "127.0.0.1");
    /// value.set("server.port", "3000");
    /// value.set("debug", "true");  // Global key
    /// ```
    pub fn set(&mut self, path: &str, value: impl Into<String>) {
        let value = value.into();
        let (section, key) = parse_path(path);

        match section {
            Some(section_name) => {
                self.sections
                    .entry(section_name.to_string())
                    .or_default()
                    .insert(key.to_string(), value);
            }
            None => {
                self.global.insert(key.to_string(), value);
            }
        }
    }

    /// Removes a value at the specified path.
    ///
    /// Returns the removed value, if any.
    pub fn remove(&mut self, path: &str) -> Option<String> {
        let (section, key) = parse_path(path);

        match section {
            Some(section_name) => self
                .sections
                .get_mut(section_name)
                .and_then(|s| s.remove(key)),
            None => self.global.remove(key),
        }
    }

    /// Removes an entire section.
    ///
    /// Returns the removed section, if any.
    pub fn remove_section(&mut self, section: &str) -> Option<HashMap<String, String>> {
        self.sections.remove(section)
    }

    /// Returns true if a value exists at the specified path.
    pub fn contains(&self, path: &str) -> bool {
        self.get_str(path).is_some()
    }

    // ========================================================================
    // Section Operations
    // ========================================================================

    /// Returns an iterator over section names.
    pub fn sections(&self) -> impl Iterator<Item = &str> {
        self.sections.keys().map(|s| s.as_str())
    }

    /// Returns an iterator over keys in a section.
    ///
    /// Returns an empty iterator if the section doesn't exist.
    pub fn section_keys(&self, section: &str) -> impl Iterator<Item = &str> {
        self.sections
            .get(section)
            .into_iter()
            .flat_map(|s| s.keys().map(|k| k.as_str()))
    }

    /// Returns an iterator over key-value pairs in a section.
    ///
    /// Returns an empty iterator if the section doesn't exist.
    pub fn section_entries(&self, section: &str) -> impl Iterator<Item = (&str, &str)> {
        self.sections
            .get(section)
            .into_iter()
            .flat_map(|s| s.iter().map(|(k, v)| (k.as_str(), v.as_str())))
    }

    /// Returns an iterator over global (sectionless) keys.
    pub fn global_keys(&self) -> impl Iterator<Item = &str> {
        self.global.keys().map(|s| s.as_str())
    }

    /// Returns an iterator over global (sectionless) key-value pairs.
    pub fn global_entries(&self) -> impl Iterator<Item = (&str, &str)> {
        self.global.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    // ========================================================================
    // Serialization
    // ========================================================================

    /// Converts this value to an INI string.
    pub fn as_ini_string(&self) -> String {
        let ini = self.to_ini();
        let mut output = Vec::new();
        ini.write_to(&mut output).unwrap_or_default();
        String::from_utf8_lossy(&output).into_owned()
    }

    /// Saves this value to a file.
    pub fn save(&self, path: impl AsRef<Path>) -> FileResult<()> {
        let content = self.as_ini_string();
        atomic_write(&path, |writer| writer.write_all(content.as_bytes()))
    }
}

impl Default for IniValue {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Display
// ============================================================================

impl fmt::Display for IniValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_ini_string())
    }
}

// ============================================================================
// IniValueRef - Reference type for API compatibility
// ============================================================================

/// A reference to a string value in an INI file.
///
/// This provides API compatibility with JsonValue/TomlValue patterns.
#[derive(Debug, Clone, Copy)]
pub struct IniValueRef<'a>(&'a str);

impl<'a> IniValueRef<'a> {
    /// Returns the value as a string slice.
    pub fn as_str(&self) -> Option<&'a str> {
        Some(self.0)
    }

    /// Returns the value parsed as an i64.
    pub fn as_i64(&self) -> Option<i64> {
        self.0.parse().ok()
    }

    /// Returns the value parsed as an f64.
    pub fn as_f64(&self) -> Option<f64> {
        self.0.parse().ok()
    }

    /// Returns the value parsed as a bool.
    pub fn as_bool(&self) -> Option<bool> {
        match self.0.to_lowercase().as_str() {
            "true" | "yes" | "1" | "on" => Some(true),
            "false" | "no" | "0" | "off" => Some(false),
            _ => None,
        }
    }
}

impl<'a> fmt::Display for IniValueRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================================
// Module-Level Functions
// ============================================================================

/// Parses an INI string into an IniValue.
///
/// # Example
///
/// ```ignore
/// let value = parse_ini(r#"
/// [server]
/// host = localhost
/// port = 8080
/// "#)?;
/// ```
pub fn parse_ini(s: &str) -> FileResult<IniValue> {
    let ini = Ini::load_from_str(s).map_err(ini_error)?;
    Ok(IniValue::from_ini(&ini))
}

/// Reads and parses an INI file into an IniValue.
///
/// # Example
///
/// ```ignore
/// let value = read_ini("config.ini")?;
/// let host = value.get_str("server.host");
/// ```
pub fn read_ini(path: impl AsRef<Path>) -> FileResult<IniValue> {
    let content = read_text(&path)?;
    let ini = Ini::load_from_str(&content).map_err(|e| {
        FileError::new(
            FileErrorKind::InvalidData,
            Some(path.as_ref().to_path_buf()),
            Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
        )
    })?;
    Ok(IniValue::from_ini(&ini))
}

/// Writes an IniValue to a file.
///
/// The file is written atomically using a temporary file and rename.
///
/// # Example
///
/// ```ignore
/// write_ini("config.ini", &value)?;
/// ```
pub fn write_ini(path: impl AsRef<Path>, value: &IniValue) -> FileResult<()> {
    value.save(&path)
}

/// Converts an IniValue to an INI string.
///
/// # Example
///
/// ```ignore
/// let ini = to_ini_string(&value);
/// ```
pub fn to_ini_string(value: &IniValue) -> String {
    value.to_string()
}

// ============================================================================
// Internal Helpers
// ============================================================================

/// Parses a path string into (section, key) components.
///
/// Supports "section.key" notation. If no dot is present, the entire
/// path is treated as a global key.
fn parse_path(path: &str) -> (Option<&str>, &str) {
    if let Some(dot_idx) = path.find('.') {
        let section = &path[..dot_idx];
        let key = &path[dot_idx + 1..];
        (Some(section), key)
    } else {
        (None, path)
    }
}

/// Converts an ini parse error to a FileError.
fn ini_error(e: ini::ParseError) -> FileError {
    FileError::new(
        FileErrorKind::InvalidData,
        None,
        Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ini() {
        let value = parse_ini(
            r#"
            [server]
            host = localhost
            port = 8080
            "#,
        )
        .unwrap();

        assert!(value.has_section("server"));
        assert_eq!(value.get_str("server.host"), Some("localhost"));
        assert_eq!(value.as_i64("server.port"), Some(8080));
    }

    #[test]
    fn test_ini_value_construction() {
        let mut value = IniValue::new();
        assert!(value.is_empty());

        value.set("server.host", "localhost");
        value.set("server.port", "8080");
        value.set("debug", "true");

        assert!(!value.is_empty());
        assert_eq!(value.section_count(), 1);
        assert!(value.has_section("server"));
    }

    #[test]
    fn test_global_values() {
        let value = parse_ini(
            r#"
            debug = true
            log_level = info

            [server]
            host = localhost
            "#,
        )
        .unwrap();

        assert_eq!(value.get_str("debug"), Some("true"));
        assert_eq!(value.get_str("log_level"), Some("info"));
        assert_eq!(value.get_str("server.host"), Some("localhost"));
    }

    #[test]
    fn test_path_based_access() {
        let value = parse_ini(
            r#"
            [database]
            host = localhost
            port = 5432

            [pool]
            max_connections = 10
            "#,
        )
        .unwrap();

        assert_eq!(value.get_str("database.host"), Some("localhost"));
        assert_eq!(value.as_i64("database.port"), Some(5432));
        assert_eq!(value.as_i64("pool.max_connections"), Some(10));
    }

    #[test]
    fn test_path_based_modification() {
        let mut value = IniValue::new();

        value.set("server.host", "127.0.0.1");
        value.set("server.port", "3000");
        value.set("database.name", "mydb");

        assert_eq!(value.get_str("server.host"), Some("127.0.0.1"));
        assert_eq!(value.as_i64("server.port"), Some(3000));
        assert_eq!(value.get_str("database.name"), Some("mydb"));
    }

    #[test]
    fn test_remove() {
        let mut value = parse_ini(
            r#"
            [server]
            host = localhost
            port = 8080
            "#,
        )
        .unwrap();

        let removed = value.remove("server.port");
        assert_eq!(removed, Some("8080".to_string()));
        assert!(value.get_str("server.port").is_none());
        assert!(value.get_str("server.host").is_some());
    }

    #[test]
    fn test_remove_section() {
        let mut value = parse_ini(
            r#"
            [server]
            host = localhost

            [database]
            name = mydb
            "#,
        )
        .unwrap();

        let removed = value.remove_section("server");
        assert!(removed.is_some());
        assert!(!value.has_section("server"));
        assert!(value.has_section("database"));
    }

    #[test]
    fn test_bool_parsing() {
        let value = parse_ini(
            r#"
            [flags]
            a = true
            b = false
            c = yes
            d = no
            e = 1
            f = 0
            g = on
            h = off
            "#,
        )
        .unwrap();

        assert_eq!(value.as_bool("flags.a"), Some(true));
        assert_eq!(value.as_bool("flags.b"), Some(false));
        assert_eq!(value.as_bool("flags.c"), Some(true));
        assert_eq!(value.as_bool("flags.d"), Some(false));
        assert_eq!(value.as_bool("flags.e"), Some(true));
        assert_eq!(value.as_bool("flags.f"), Some(false));
        assert_eq!(value.as_bool("flags.g"), Some(true));
        assert_eq!(value.as_bool("flags.h"), Some(false));
    }

    #[test]
    fn test_section_operations() {
        let value = parse_ini(
            r#"
            [server]
            host = localhost
            port = 8080

            [database]
            name = mydb
            "#,
        )
        .unwrap();

        let sections: Vec<_> = value.sections().collect();
        assert!(sections.contains(&"server"));
        assert!(sections.contains(&"database"));

        let keys: Vec<_> = value.section_keys("server").collect();
        assert!(keys.contains(&"host"));
        assert!(keys.contains(&"port"));
    }

    #[test]
    fn test_serialization() {
        let mut value = IniValue::new();
        value.set("server.host", "localhost");
        value.set("server.port", "8080");

        let ini_str = value.to_string();
        assert!(ini_str.contains("[server]"));
        assert!(ini_str.contains("host"));
        assert!(ini_str.contains("localhost"));
    }

    #[test]
    fn test_file_roundtrip() {
        let mut value = IniValue::new();
        value.set("app.name", "Test");
        value.set("app.version", "1.0.0");
        value.set("server.port", "3000");

        let path = std::env::temp_dir().join("horizon_ini_test.ini");

        value.save(&path).unwrap();

        let loaded = read_ini(&path).unwrap();
        assert_eq!(loaded.get_str("app.name"), Some("Test"));
        assert_eq!(loaded.get_str("app.version"), Some("1.0.0"));
        assert_eq!(loaded.as_i64("server.port"), Some(3000));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn test_ini_value_ref() {
        let value = parse_ini(
            r#"
            [test]
            str = hello
            num = 42
            float = 3.14
            bool = true
            "#,
        )
        .unwrap();

        let str_ref = value.get_ref("test.str").unwrap();
        assert_eq!(str_ref.as_str(), Some("hello"));

        let num_ref = value.get_ref("test.num").unwrap();
        assert_eq!(num_ref.as_i64(), Some(42));

        let float_ref = value.get_ref("test.float").unwrap();
        assert!((float_ref.as_f64().unwrap() - 3.14).abs() < 0.001);

        let bool_ref = value.get_ref("test.bool").unwrap();
        assert_eq!(bool_ref.as_bool(), Some(true));
    }

    #[test]
    fn test_parse_path() {
        assert_eq!(parse_path("section.key"), (Some("section"), "key"));
        assert_eq!(parse_path("key"), (None, "key"));
        assert_eq!(
            parse_path("deep.nested.key"),
            (Some("deep"), "nested.key")
        );
    }
}
