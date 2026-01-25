//! Application settings and preferences.
//!
//! This module provides a type-safe, hierarchical key-value storage system
//! for application configuration. Settings can be persisted to JSON or TOML
//! files and support change notifications via signals.
//!
//! # Path-Based Access
//!
//! Settings are organized hierarchically using path-based keys. Paths can use
//! either "." or "/" as separators:
//!
//! ```ignore
//! use horizon_lattice::file::Settings;
//!
//! let mut settings = Settings::new();
//!
//! // Set values using path-based keys
//! settings.set("app.window.width", 1024);
//! settings.set("app.window.height", 768);
//! settings.set("app/theme/name", "dark");
//!
//! // Get values with type inference
//! let width: i32 = settings.get("app.window.width").unwrap();
//! let theme: String = settings.get("app/theme/name").unwrap();
//! ```
//!
//! # Default Values
//!
//! Use `get_or` to provide default values:
//!
//! ```ignore
//! let width = settings.get_or("app.window.width", 800);
//! let theme = settings.get_or("app.theme.name", "light".to_string());
//! ```
//!
//! # Change Notifications
//!
//! Connect to the `changed` signal to be notified when settings change:
//!
//! ```ignore
//! settings.changed().connect(|key| {
//!     println!("Setting changed: {}", key);
//! });
//! ```
//!
//! # Persistence
//!
//! Settings can be saved to and loaded from files:
//!
//! ```ignore
//! // Save as JSON
//! settings.save_json("config.json")?;
//!
//! // Load from JSON
//! let settings = Settings::load_json("config.json")?;
//!
//! // Save as TOML
//! settings.save_toml("config.toml")?;
//!
//! // Load from TOML
//! let settings = Settings::load_toml("config.toml")?;
//! ```
//!
//! # Auto-Save
//!
//! Enable auto-save to automatically persist changes:
//!
//! ```ignore
//! let mut settings = Settings::new();
//! settings.set_auto_save("config.json", SettingsFormat::Json);
//!
//! // Changes are automatically saved
//! settings.set("app.theme", "dark");
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use horizon_lattice_core::signal::Signal;
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use super::error::{FileError, FileErrorKind, FileResult};
use super::operations::{atomic_write, read_text};

/// A value that can be stored in settings.
///
/// This enum represents all the primitive types that can be stored directly.
/// For complex types, use serde serialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SettingsValue {
    /// A null/empty value.
    Null,
    /// A boolean value.
    Bool(bool),
    /// A 64-bit signed integer.
    Integer(i64),
    /// A 64-bit floating point number.
    Float(f64),
    /// A string value.
    String(String),
    /// An array of values.
    Array(Vec<SettingsValue>),
    /// A nested object/table.
    Object(HashMap<String, SettingsValue>),
}

impl SettingsValue {
    /// Returns true if this value is null.
    pub fn is_null(&self) -> bool {
        matches!(self, SettingsValue::Null)
    }

    /// Returns this value as a boolean, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            SettingsValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns this value as an integer, if it is one.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            SettingsValue::Integer(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns this value as a float, if it is one.
    /// Also converts integers to floats.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            SettingsValue::Float(v) => Some(*v),
            SettingsValue::Integer(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// Returns this value as a string, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            SettingsValue::String(v) => Some(v),
            _ => None,
        }
    }

    /// Returns this value as an array, if it is one.
    pub fn as_array(&self) -> Option<&Vec<SettingsValue>> {
        match self {
            SettingsValue::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Returns this value as an object/table, if it is one.
    pub fn as_object(&self) -> Option<&HashMap<String, SettingsValue>> {
        match self {
            SettingsValue::Object(v) => Some(v),
            _ => None,
        }
    }
}

impl Default for SettingsValue {
    fn default() -> Self {
        SettingsValue::Null
    }
}

impl From<bool> for SettingsValue {
    fn from(v: bool) -> Self {
        SettingsValue::Bool(v)
    }
}

impl From<i32> for SettingsValue {
    fn from(v: i32) -> Self {
        SettingsValue::Integer(v as i64)
    }
}

impl From<i64> for SettingsValue {
    fn from(v: i64) -> Self {
        SettingsValue::Integer(v)
    }
}

impl From<f32> for SettingsValue {
    fn from(v: f32) -> Self {
        SettingsValue::Float(v as f64)
    }
}

impl From<f64> for SettingsValue {
    fn from(v: f64) -> Self {
        SettingsValue::Float(v)
    }
}

impl From<String> for SettingsValue {
    fn from(v: String) -> Self {
        SettingsValue::String(v)
    }
}

impl From<&str> for SettingsValue {
    fn from(v: &str) -> Self {
        SettingsValue::String(v.to_string())
    }
}

impl<T: Into<SettingsValue>> From<Vec<T>> for SettingsValue {
    fn from(v: Vec<T>) -> Self {
        SettingsValue::Array(v.into_iter().map(Into::into).collect())
    }
}

/// The format for settings file persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFormat {
    /// JSON format.
    Json,
    /// TOML format.
    Toml,
}

/// Auto-save configuration.
#[derive(Debug, Clone)]
struct AutoSaveConfig {
    path: PathBuf,
    format: SettingsFormat,
}

/// A hierarchical key-value settings storage.
///
/// Settings provides a type-safe, path-based interface for storing and
/// retrieving application configuration. It supports persistence to JSON
/// or TOML files, and emits change notifications via signals.
pub struct Settings {
    /// The root data store.
    data: RwLock<HashMap<String, SettingsValue>>,
    /// Signal emitted when a setting changes. The argument is the key path.
    changed: Signal<String>,
    /// Auto-save configuration, if enabled.
    auto_save: RwLock<Option<AutoSaveConfig>>,
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    /// Creates a new empty settings store.
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
            changed: Signal::new(),
            auto_save: RwLock::new(None),
        }
    }

    /// Creates settings from a nested HashMap.
    pub fn from_data(data: HashMap<String, SettingsValue>) -> Self {
        Self {
            data: RwLock::new(data),
            changed: Signal::new(),
            auto_save: RwLock::new(None),
        }
    }

    /// Returns a reference to the changed signal.
    ///
    /// This signal is emitted whenever a setting value is modified.
    /// The argument is the full path of the changed key.
    pub fn changed(&self) -> &Signal<String> {
        &self.changed
    }

    /// Enables auto-save to the specified file.
    ///
    /// When auto-save is enabled, any changes to settings will automatically
    /// be persisted to the file. The save is performed atomically.
    pub fn set_auto_save(&self, path: impl AsRef<Path>, format: SettingsFormat) {
        *self.auto_save.write() = Some(AutoSaveConfig {
            path: path.as_ref().to_path_buf(),
            format,
        });
    }

    /// Disables auto-save.
    pub fn disable_auto_save(&self) {
        *self.auto_save.write() = None;
    }

    /// Returns true if auto-save is enabled.
    pub fn is_auto_save_enabled(&self) -> bool {
        self.auto_save.read().is_some()
    }

    /// Sets a value at the specified path.
    ///
    /// The path can use either "." or "/" as separators. Intermediate
    /// objects are created automatically if they don't exist.
    ///
    /// # Example
    ///
    /// ```ignore
    /// settings.set("app.window.width", 1024);
    /// settings.set("app/theme/name", "dark");
    /// ```
    pub fn set<V: Into<SettingsValue>>(&self, path: &str, value: V) {
        let value = value.into();
        let parts = Self::parse_path(path);

        if parts.is_empty() {
            return;
        }

        {
            let mut data = self.data.write();
            Self::set_nested(&mut data, &parts, value);
        }

        self.changed.emit(path.to_string());
        self.try_auto_save();
    }

    /// Sets a value using serde serialization.
    ///
    /// This allows storing any type that implements `Serialize`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(Serialize)]
    /// struct WindowConfig {
    ///     width: i32,
    ///     height: i32,
    /// }
    ///
    /// settings.set_serialized("app.window", &WindowConfig { width: 1024, height: 768 })?;
    /// ```
    pub fn set_serialized<T: Serialize>(&self, path: &str, value: &T) -> FileResult<()> {
        let json = serde_json::to_value(value).map_err(|e| {
            FileError::new(
                FileErrorKind::InvalidData,
                None,
                Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?;

        let settings_value = Self::json_to_settings_value(json);
        self.set(path, settings_value);
        Ok(())
    }

    /// Gets a value at the specified path.
    ///
    /// Returns `None` if the path doesn't exist or the value cannot be
    /// converted to the requested type.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let width: Option<i32> = settings.get("app.window.width");
    /// let theme: Option<String> = settings.get("app.theme.name");
    /// ```
    pub fn get<T: FromSettingsValue>(&self, path: &str) -> Option<T> {
        let data = self.data.read();
        let parts = Self::parse_path(path);
        let value = Self::get_nested(&data, &parts)?;
        T::from_settings_value(value)
    }

    /// Gets a value at the specified path, or returns the default.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let width = settings.get_or("app.window.width", 800);
    /// let theme = settings.get_or("app.theme.name", "light".to_string());
    /// ```
    pub fn get_or<T: FromSettingsValue>(&self, path: &str, default: T) -> T {
        self.get(path).unwrap_or(default)
    }

    /// Gets a value using serde deserialization.
    ///
    /// This allows retrieving any type that implements `Deserialize`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(Deserialize)]
    /// struct WindowConfig {
    ///     width: i32,
    ///     height: i32,
    /// }
    ///
    /// let config: Option<WindowConfig> = settings.get_deserialized("app.window");
    /// ```
    pub fn get_deserialized<T: DeserializeOwned>(&self, path: &str) -> Option<T> {
        let data = self.data.read();
        let parts = Self::parse_path(path);
        let value = Self::get_nested(&data, &parts)?;
        let json = Self::settings_value_to_json(value.clone());
        serde_json::from_value(json).ok()
    }

    /// Gets the raw `SettingsValue` at the specified path.
    pub fn get_raw(&self, path: &str) -> Option<SettingsValue> {
        let data = self.data.read();
        let parts = Self::parse_path(path);
        Self::get_nested(&data, &parts).cloned()
    }

    /// Returns true if a value exists at the specified path.
    pub fn contains(&self, path: &str) -> bool {
        let data = self.data.read();
        let parts = Self::parse_path(path);
        Self::get_nested(&data, &parts).is_some()
    }

    /// Removes a value at the specified path.
    ///
    /// Returns the removed value, if any.
    pub fn remove(&self, path: &str) -> Option<SettingsValue> {
        let parts = Self::parse_path(path);
        if parts.is_empty() {
            return None;
        }

        let result = {
            let mut data = self.data.write();
            Self::remove_nested(&mut data, &parts)
        };

        if result.is_some() {
            self.changed.emit(path.to_string());
            self.try_auto_save();
        }

        result
    }

    /// Clears all settings.
    pub fn clear(&self) {
        self.data.write().clear();
        self.changed.emit(String::new());
        self.try_auto_save();
    }

    /// Returns all keys at the top level.
    pub fn keys(&self) -> Vec<String> {
        self.data.read().keys().cloned().collect()
    }

    /// Returns all keys under a specific path (group).
    pub fn group_keys(&self, path: &str) -> Vec<String> {
        let data = self.data.read();
        let parts = Self::parse_path(path);

        if let Some(SettingsValue::Object(obj)) = Self::get_nested(&data, &parts) {
            obj.keys().cloned().collect()
        } else {
            Vec::new()
        }
    }

    /// Returns the number of top-level keys.
    pub fn len(&self) -> usize {
        self.data.read().len()
    }

    /// Returns true if there are no settings.
    pub fn is_empty(&self) -> bool {
        self.data.read().is_empty()
    }

    // ========================================================================
    // Persistence
    // ========================================================================

    /// Loads settings from a JSON file.
    pub fn load_json(path: impl AsRef<Path>) -> FileResult<Self> {
        let content = read_text(&path)?;
        let data: HashMap<String, SettingsValue> = serde_json::from_str(&content).map_err(|e| {
            FileError::new(
                FileErrorKind::InvalidData,
                Some(path.as_ref().to_path_buf()),
                Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?;
        Ok(Self::from_data(data))
    }

    /// Loads settings from a TOML file.
    pub fn load_toml(path: impl AsRef<Path>) -> FileResult<Self> {
        let content = read_text(&path)?;
        let toml_value: toml::Value = content.parse().map_err(|e: toml::de::Error| {
            FileError::new(
                FileErrorKind::InvalidData,
                Some(path.as_ref().to_path_buf()),
                Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?;

        let data = Self::toml_to_settings(toml_value);
        match data {
            SettingsValue::Object(map) => Ok(Self::from_data(map)),
            _ => Ok(Self::new()),
        }
    }

    /// Saves settings to a JSON file.
    ///
    /// The file is written atomically using a temporary file and rename.
    pub fn save_json(&self, path: impl AsRef<Path>) -> FileResult<()> {
        let data = self.data.read();
        let json = serde_json::to_string_pretty(&*data).map_err(|e| {
            FileError::new(
                FileErrorKind::InvalidData,
                Some(path.as_ref().to_path_buf()),
                Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?;

        atomic_write(&path, |writer| {
            writer.write_all(json.as_bytes())
        })
    }

    /// Saves settings to a TOML file.
    ///
    /// The file is written atomically using a temporary file and rename.
    pub fn save_toml(&self, path: impl AsRef<Path>) -> FileResult<()> {
        let data = self.data.read();
        let toml_value = Self::settings_to_toml(&SettingsValue::Object(data.clone()));
        let toml_str = toml::to_string_pretty(&toml_value).map_err(|e| {
            FileError::new(
                FileErrorKind::InvalidData,
                Some(path.as_ref().to_path_buf()),
                Some(std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            )
        })?;

        atomic_write(&path, |writer| {
            writer.write_all(toml_str.as_bytes())
        })
    }

    /// Syncs settings to disk if auto-save is enabled.
    ///
    /// This is called automatically after each modification when auto-save
    /// is enabled, but can be called manually if needed.
    pub fn sync(&self) -> FileResult<()> {
        let config = self.auto_save.read().clone();
        if let Some(config) = config {
            match config.format {
                SettingsFormat::Json => self.save_json(&config.path),
                SettingsFormat::Toml => self.save_toml(&config.path),
            }
        } else {
            Ok(())
        }
    }

    // ========================================================================
    // Internal helpers
    // ========================================================================

    /// Parses a path string into components.
    fn parse_path(path: &str) -> Vec<&str> {
        path.split(|c| c == '.' || c == '/')
            .filter(|s| !s.is_empty())
            .collect()
    }

    /// Gets a nested value from the data tree.
    fn get_nested<'a>(
        data: &'a HashMap<String, SettingsValue>,
        parts: &[&str],
    ) -> Option<&'a SettingsValue> {
        if parts.is_empty() {
            return None;
        }

        let first = parts[0];
        let value = data.get(first)?;

        if parts.len() == 1 {
            Some(value)
        } else {
            match value {
                SettingsValue::Object(obj) => Self::get_nested(obj, &parts[1..]),
                _ => None,
            }
        }
    }

    /// Sets a nested value in the data tree, creating intermediate objects.
    fn set_nested(data: &mut HashMap<String, SettingsValue>, parts: &[&str], value: SettingsValue) {
        if parts.is_empty() {
            return;
        }

        let first = parts[0].to_string();

        if parts.len() == 1 {
            data.insert(first, value);
        } else {
            let entry = data
                .entry(first)
                .or_insert_with(|| SettingsValue::Object(HashMap::new()));

            if let SettingsValue::Object(obj) = entry {
                Self::set_nested(obj, &parts[1..], value);
            } else {
                // Replace non-object with object
                let mut new_obj = HashMap::new();
                Self::set_nested(&mut new_obj, &parts[1..], value);
                *entry = SettingsValue::Object(new_obj);
            }
        }
    }

    /// Removes a nested value from the data tree.
    fn remove_nested(
        data: &mut HashMap<String, SettingsValue>,
        parts: &[&str],
    ) -> Option<SettingsValue> {
        if parts.is_empty() {
            return None;
        }

        let first = parts[0];

        if parts.len() == 1 {
            data.remove(first)
        } else {
            let value = data.get_mut(first)?;
            if let SettingsValue::Object(obj) = value {
                Self::remove_nested(obj, &parts[1..])
            } else {
                None
            }
        }
    }

    /// Tries to auto-save if enabled.
    fn try_auto_save(&self) {
        let config = self.auto_save.read().clone();
        if let Some(config) = config {
            let result = match config.format {
                SettingsFormat::Json => self.save_json(&config.path),
                SettingsFormat::Toml => self.save_toml(&config.path),
            };
            if let Err(e) = result {
                tracing::error!("Failed to auto-save settings: {}", e);
            }
        }
    }

    /// Converts a serde_json::Value to SettingsValue.
    fn json_to_settings_value(json: serde_json::Value) -> SettingsValue {
        match json {
            serde_json::Value::Null => SettingsValue::Null,
            serde_json::Value::Bool(b) => SettingsValue::Bool(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    SettingsValue::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    SettingsValue::Float(f)
                } else {
                    SettingsValue::Null
                }
            }
            serde_json::Value::String(s) => SettingsValue::String(s),
            serde_json::Value::Array(arr) => {
                SettingsValue::Array(arr.into_iter().map(Self::json_to_settings_value).collect())
            }
            serde_json::Value::Object(obj) => SettingsValue::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, Self::json_to_settings_value(v)))
                    .collect(),
            ),
        }
    }

    /// Converts a SettingsValue to serde_json::Value.
    fn settings_value_to_json(value: SettingsValue) -> serde_json::Value {
        match value {
            SettingsValue::Null => serde_json::Value::Null,
            SettingsValue::Bool(b) => serde_json::Value::Bool(b),
            SettingsValue::Integer(i) => serde_json::Value::Number(i.into()),
            SettingsValue::Float(f) => {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            SettingsValue::String(s) => serde_json::Value::String(s),
            SettingsValue::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Self::settings_value_to_json).collect())
            }
            SettingsValue::Object(obj) => serde_json::Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, Self::settings_value_to_json(v)))
                    .collect(),
            ),
        }
    }

    /// Converts a toml::Value to SettingsValue.
    fn toml_to_settings(toml: toml::Value) -> SettingsValue {
        match toml {
            toml::Value::String(s) => SettingsValue::String(s),
            toml::Value::Integer(i) => SettingsValue::Integer(i),
            toml::Value::Float(f) => SettingsValue::Float(f),
            toml::Value::Boolean(b) => SettingsValue::Bool(b),
            toml::Value::Datetime(dt) => SettingsValue::String(dt.to_string()),
            toml::Value::Array(arr) => {
                SettingsValue::Array(arr.into_iter().map(Self::toml_to_settings).collect())
            }
            toml::Value::Table(table) => SettingsValue::Object(
                table
                    .into_iter()
                    .map(|(k, v)| (k, Self::toml_to_settings(v)))
                    .collect(),
            ),
        }
    }

    /// Converts a SettingsValue to toml::Value.
    fn settings_to_toml(value: &SettingsValue) -> toml::Value {
        match value {
            SettingsValue::Null => toml::Value::String(String::new()),
            SettingsValue::Bool(b) => toml::Value::Boolean(*b),
            SettingsValue::Integer(i) => toml::Value::Integer(*i),
            SettingsValue::Float(f) => toml::Value::Float(*f),
            SettingsValue::String(s) => toml::Value::String(s.clone()),
            SettingsValue::Array(arr) => {
                toml::Value::Array(arr.iter().map(Self::settings_to_toml).collect())
            }
            SettingsValue::Object(obj) => toml::Value::Table(
                obj.iter()
                    .map(|(k, v)| (k.clone(), Self::settings_to_toml(v)))
                    .collect(),
            ),
        }
    }
}

// Thread-safe: Settings uses RwLock internally
unsafe impl Send for Settings {}
unsafe impl Sync for Settings {}

/// Trait for types that can be extracted from a SettingsValue.
pub trait FromSettingsValue: Sized {
    /// Attempts to convert a SettingsValue to this type.
    fn from_settings_value(value: &SettingsValue) -> Option<Self>;
}

impl FromSettingsValue for bool {
    fn from_settings_value(value: &SettingsValue) -> Option<Self> {
        value.as_bool()
    }
}

impl FromSettingsValue for i32 {
    fn from_settings_value(value: &SettingsValue) -> Option<Self> {
        value.as_integer().map(|v| v as i32)
    }
}

impl FromSettingsValue for i64 {
    fn from_settings_value(value: &SettingsValue) -> Option<Self> {
        value.as_integer()
    }
}

impl FromSettingsValue for f32 {
    fn from_settings_value(value: &SettingsValue) -> Option<Self> {
        value.as_float().map(|v| v as f32)
    }
}

impl FromSettingsValue for f64 {
    fn from_settings_value(value: &SettingsValue) -> Option<Self> {
        value.as_float()
    }
}

impl FromSettingsValue for String {
    fn from_settings_value(value: &SettingsValue) -> Option<Self> {
        value.as_str().map(|s| s.to_string())
    }
}

impl<T: FromSettingsValue> FromSettingsValue for Vec<T> {
    fn from_settings_value(value: &SettingsValue) -> Option<Self> {
        value
            .as_array()
            .and_then(|arr| arr.iter().map(T::from_settings_value).collect())
    }
}

impl FromSettingsValue for SettingsValue {
    fn from_settings_value(value: &SettingsValue) -> Option<Self> {
        Some(value.clone())
    }
}

/// A thread-safe, reference-counted settings wrapper.
///
/// This is useful for sharing settings across multiple parts of an application.
pub type SharedSettings = Arc<Settings>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_basic_get_set() {
        let settings = Settings::new();

        settings.set("name", "test");
        settings.set("count", 42);
        settings.set("ratio", 3.14);
        settings.set("enabled", true);

        assert_eq!(settings.get::<String>("name"), Some("test".to_string()));
        assert_eq!(settings.get::<i32>("count"), Some(42));
        assert_eq!(settings.get::<f64>("ratio"), Some(3.14));
        assert_eq!(settings.get::<bool>("enabled"), Some(true));
    }

    #[test]
    fn test_nested_paths() {
        let settings = Settings::new();

        settings.set("app.window.width", 1024);
        settings.set("app.window.height", 768);
        settings.set("app/theme/name", "dark");

        assert_eq!(settings.get::<i32>("app.window.width"), Some(1024));
        assert_eq!(settings.get::<i32>("app/window/height"), Some(768));
        assert_eq!(settings.get::<String>("app.theme.name"), Some("dark".to_string()));
    }

    #[test]
    fn test_get_or_default() {
        let settings = Settings::new();

        settings.set("existing", 100);

        assert_eq!(settings.get_or("existing", 0), 100);
        assert_eq!(settings.get_or("missing", 42), 42);
    }

    #[test]
    fn test_contains_and_remove() {
        let settings = Settings::new();

        settings.set("key", "value");
        assert!(settings.contains("key"));

        let removed = settings.remove("key");
        assert_eq!(removed, Some(SettingsValue::String("value".to_string())));
        assert!(!settings.contains("key"));
    }

    #[test]
    fn test_arrays() {
        let settings = Settings::new();

        settings.set("numbers", vec![1, 2, 3]);
        settings.set("names", vec!["alice", "bob"]);

        let numbers: Vec<i32> = settings.get("numbers").unwrap();
        assert_eq!(numbers, vec![1, 2, 3]);

        let names: Vec<String> = settings.get("names").unwrap();
        assert_eq!(names, vec!["alice".to_string(), "bob".to_string()]);
    }

    #[test]
    fn test_change_signal() {
        let settings = Settings::new();
        let change_count = Arc::new(AtomicUsize::new(0));
        let count_clone = change_count.clone();

        settings.changed().connect(move |_key| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        settings.set("a", 1);
        settings.set("b", 2);
        settings.set("c", 3);

        assert_eq!(change_count.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn test_group_keys() {
        let settings = Settings::new();

        settings.set("app.a", 1);
        settings.set("app.b", 2);
        settings.set("app.c", 3);
        settings.set("other", 4);

        let mut keys = settings.group_keys("app");
        keys.sort();
        assert_eq!(keys, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_json_roundtrip() {
        let settings = Settings::new();
        settings.set("name", "test");
        settings.set("count", 42);
        settings.set("nested.value", true);

        let temp_path = std::env::temp_dir().join("horizon_settings_test.json");
        settings.save_json(&temp_path).unwrap();

        let loaded = Settings::load_json(&temp_path).unwrap();
        assert_eq!(loaded.get::<String>("name"), Some("test".to_string()));
        assert_eq!(loaded.get::<i32>("count"), Some(42));
        assert_eq!(loaded.get::<bool>("nested.value"), Some(true));

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_toml_roundtrip() {
        let settings = Settings::new();
        settings.set("name", "test");
        settings.set("count", 42);
        settings.set("nested.value", true);

        let temp_path = std::env::temp_dir().join("horizon_settings_test.toml");
        settings.save_toml(&temp_path).unwrap();

        let loaded = Settings::load_toml(&temp_path).unwrap();
        assert_eq!(loaded.get::<String>("name"), Some("test".to_string()));
        assert_eq!(loaded.get::<i32>("count"), Some(42));
        assert_eq!(loaded.get::<bool>("nested.value"), Some(true));

        std::fs::remove_file(&temp_path).ok();
    }

    #[test]
    fn test_serialized_types() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct Config {
            width: i32,
            height: i32,
            title: String,
        }

        let settings = Settings::new();
        let config = Config {
            width: 1024,
            height: 768,
            title: "My App".to_string(),
        };

        settings.set_serialized("window", &config).unwrap();

        let loaded: Config = settings.get_deserialized("window").unwrap();
        assert_eq!(loaded, config);
    }

    #[test]
    fn test_clear() {
        let settings = Settings::new();
        settings.set("a", 1);
        settings.set("b", 2);

        assert_eq!(settings.len(), 2);

        settings.clear();
        assert!(settings.is_empty());
    }
}
