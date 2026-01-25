//! Embedded resource system for compile-time and runtime resources.
//!
//! This module provides a resource management system that supports:
//! - Compile-time embedded resources via `include_dir!` macro
//! - Runtime filesystem resources
//! - Path-based resource access with prefix routing
//! - Sync, async, and lazy loading patterns
//!
//! # Resource Paths
//!
//! Resources are accessed via paths with optional prefixes:
//! - `:/path/to/resource` - Embedded resource (default prefix)
//! - `prefix:/path` - Custom prefix for registered directories
//! - `/absolute/path` or `relative/path` - Filesystem resource
//!
//! # Embedding Resources
//!
//! Use the `include_dir!` macro to embed a directory at compile time:
//!
//! ```ignore
//! use include_dir::{include_dir, Dir};
//! use horizon_lattice::file::{ResourceManager, EmbeddedDir};
//!
//! // Embed the assets directory at compile time
//! static ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");
//!
//! // Register with the resource manager
//! ResourceManager::global().register_embedded("assets", EmbeddedDir::new(&ASSETS));
//!
//! // Access resources by path
//! if let Some(data) = ResourceManager::global().get("assets:/images/icon.png") {
//!     // data is &'static [u8]
//! }
//!
//! // Or use the default ":" prefix
//! ResourceManager::global().register_embedded("", EmbeddedDir::new(&ASSETS));
//! let icon = ResourceManager::global().get(":/images/icon.png");
//! ```
//!
//! # Async Loading
//!
//! For filesystem resources, async loading is supported:
//!
//! ```ignore
//! use horizon_lattice::file::ResourceManager;
//!
//! // Load resource asynchronously from filesystem
//! let data = ResourceManager::global().load_async("/path/to/file.bin").await?;
//!
//! // Load text resource
//! let text = ResourceManager::global().load_text_async("/path/to/file.txt").await?;
//! ```
//!
//! # Lazy Loading
//!
//! For resources that should be loaded on first access:
//!
//! ```ignore
//! use horizon_lattice::file::LazyResource;
//!
//! // Create a lazy resource
//! let resource = LazyResource::new(":/images/large_icon.png");
//!
//! // Data is loaded on first access
//! if let Some(data) = resource.get() {
//!     // Use the data
//! }
//! ```
//!
//! # Resource Types
//!
//! The resource system provides raw bytes or text. Consuming systems
//! (images, fonts, stylesheets) handle interpretation:
//!
//! ```ignore
//! // Get raw bytes (for images, fonts, binary data)
//! let bytes: Option<&[u8]> = resources.get(":/icon.png");
//!
//! // Get as text (for stylesheets, config files)
//! let text: Option<&str> = resources.get_text(":/styles/main.css");
//!
//! // List resources in a directory
//! for path in resources.list(":/images/") {
//!     println!("Found: {}", path);
//! }
//! ```

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use include_dir::{Dir, DirEntry};
use parking_lot::RwLock;

use super::error::{FileError, FileResult};

/// Global resource manager instance.
static GLOBAL_MANAGER: OnceLock<ResourceManager> = OnceLock::new();

/// A wrapper around an embedded directory from `include_dir!`.
///
/// This provides a uniform interface for accessing embedded resources.
#[derive(Clone, Copy)]
pub struct EmbeddedDir {
    dir: &'static Dir<'static>,
}

impl EmbeddedDir {
    /// Creates a new embedded directory wrapper.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use include_dir::{include_dir, Dir};
    /// use horizon_lattice::file::EmbeddedDir;
    ///
    /// static ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");
    /// let embedded = EmbeddedDir::new(&ASSETS);
    /// ```
    pub const fn new(dir: &'static Dir<'static>) -> Self {
        Self { dir }
    }

    /// Gets a file's contents by path.
    ///
    /// Returns `None` if the file doesn't exist.
    pub fn get_file(&self, path: &str) -> Option<&'static [u8]> {
        self.dir.get_file(path).map(|f| f.contents())
    }

    /// Gets a file's contents as a UTF-8 string.
    ///
    /// Returns `None` if the file doesn't exist or isn't valid UTF-8.
    pub fn get_text(&self, path: &str) -> Option<&'static str> {
        self.dir.get_file(path).and_then(|f| f.contents_utf8())
    }

    /// Checks if a file exists at the given path.
    pub fn contains(&self, path: &str) -> bool {
        self.dir.get_file(path).is_some()
    }

    /// Checks if a directory exists at the given path.
    pub fn contains_dir(&self, path: &str) -> bool {
        self.dir.get_dir(path).is_some()
    }

    /// Lists all file paths in the embedded directory (recursively).
    pub fn list_files(&self) -> Vec<&'static str> {
        let mut paths = Vec::new();
        self.collect_files(self.dir, &mut paths);
        paths
    }

    /// Lists file paths in a subdirectory.
    pub fn list_files_in(&self, subdir: &str) -> Vec<&'static str> {
        let mut paths = Vec::new();
        if let Some(dir) = self.dir.get_dir(subdir) {
            self.collect_files(dir, &mut paths);
        }
        paths
    }

    /// Lists immediate children (files and directories) of a path.
    pub fn list_entries(&self, path: &str) -> Vec<ResourceEntry> {
        let dir = if path.is_empty() {
            Some(self.dir)
        } else {
            self.dir.get_dir(path)
        };

        dir.map(|d| {
            d.entries()
                .iter()
                .map(|e| match e {
                    DirEntry::Dir(d) => ResourceEntry::Directory(d.path().to_string_lossy().into_owned()),
                    DirEntry::File(f) => ResourceEntry::File(f.path().to_string_lossy().into_owned()),
                })
                .collect()
        })
        .unwrap_or_default()
    }

    fn collect_files(&self, dir: &'static Dir<'static>, paths: &mut Vec<&'static str>) {
        for entry in dir.entries() {
            match entry {
                DirEntry::Dir(subdir) => {
                    self.collect_files(subdir, paths);
                }
                DirEntry::File(file) => {
                    if let Some(path) = file.path().to_str() {
                        paths.push(path);
                    }
                }
            }
        }
    }
}

impl std::fmt::Debug for EmbeddedDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EmbeddedDir")
            .field("file_count", &self.list_files().len())
            .finish()
    }
}

/// An entry in a resource directory listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceEntry {
    /// A file entry with its path.
    File(String),
    /// A directory entry with its path.
    Directory(String),
}

impl ResourceEntry {
    /// Returns the path of this entry.
    pub fn path(&self) -> &str {
        match self {
            ResourceEntry::File(p) | ResourceEntry::Directory(p) => p,
        }
    }

    /// Returns true if this is a file entry.
    pub fn is_file(&self) -> bool {
        matches!(self, ResourceEntry::File(_))
    }

    /// Returns true if this is a directory entry.
    pub fn is_dir(&self) -> bool {
        matches!(self, ResourceEntry::Directory(_))
    }
}

/// Parsed resource path with prefix and relative path components.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourcePath<'a> {
    /// The prefix (e.g., "assets" from "assets:/path").
    /// Empty string for default prefix (":").
    pub prefix: Cow<'a, str>,
    /// The path within the resource prefix.
    pub path: Cow<'a, str>,
    /// Whether this is an embedded resource (has :/ syntax).
    pub is_embedded: bool,
}

impl<'a> ResourcePath<'a> {
    /// Parses a resource path string.
    ///
    /// # Examples
    ///
    /// - `":/images/icon.png"` -> prefix="", path="images/icon.png", embedded=true
    /// - `"assets:/fonts/main.ttf"` -> prefix="assets", path="fonts/main.ttf", embedded=true
    /// - `"/absolute/path"` -> prefix="", path="/absolute/path", embedded=false
    /// - `"relative/path"` -> prefix="", path="relative/path", embedded=false
    pub fn parse(input: &'a str) -> Self {
        // Check for embedded resource syntax (prefix:/path or :/path)
        if let Some(colon_pos) = input.find(":/") {
            let prefix = &input[..colon_pos];
            let path = &input[colon_pos + 2..]; // Skip ":/"
            ResourcePath {
                prefix: Cow::Borrowed(prefix),
                path: Cow::Borrowed(path),
                is_embedded: true,
            }
        } else {
            // Filesystem path
            ResourcePath {
                prefix: Cow::Borrowed(""),
                path: Cow::Borrowed(input),
                is_embedded: false,
            }
        }
    }

    /// Converts to an owned version.
    pub fn into_owned(self) -> ResourcePath<'static> {
        ResourcePath {
            prefix: Cow::Owned(self.prefix.into_owned()),
            path: Cow::Owned(self.path.into_owned()),
            is_embedded: self.is_embedded,
        }
    }
}

/// The global resource manager for accessing embedded and filesystem resources.
///
/// Resources are registered with prefixes and accessed via path syntax:
/// - `prefix:/path` for embedded resources
/// - Regular paths for filesystem access
pub struct ResourceManager {
    /// Registered embedded directories by prefix.
    embedded: RwLock<HashMap<String, EmbeddedDir>>,
    /// Registered filesystem root directories by prefix.
    filesystem_roots: RwLock<HashMap<String, PathBuf>>,
}

impl ResourceManager {
    /// Creates a new resource manager.
    pub fn new() -> Self {
        Self {
            embedded: RwLock::new(HashMap::new()),
            filesystem_roots: RwLock::new(HashMap::new()),
        }
    }

    /// Gets the global resource manager instance.
    ///
    /// This instance is shared across the application.
    pub fn global() -> &'static ResourceManager {
        GLOBAL_MANAGER.get_or_init(ResourceManager::new)
    }

    /// Registers an embedded directory with a prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix for accessing these resources. Use empty string
    ///   for the default ":" prefix.
    /// * `dir` - The embedded directory to register.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use include_dir::{include_dir, Dir};
    /// use horizon_lattice::file::{ResourceManager, EmbeddedDir};
    ///
    /// static ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets");
    ///
    /// // Register with custom prefix: "assets:/path"
    /// ResourceManager::global().register_embedded("assets", EmbeddedDir::new(&ASSETS));
    ///
    /// // Register as default: ":/path"
    /// ResourceManager::global().register_embedded("", EmbeddedDir::new(&ASSETS));
    /// ```
    pub fn register_embedded(&self, prefix: &str, dir: EmbeddedDir) {
        self.embedded.write().insert(prefix.to_string(), dir);
    }

    /// Unregisters an embedded directory.
    pub fn unregister_embedded(&self, prefix: &str) -> bool {
        self.embedded.write().remove(prefix).is_some()
    }

    /// Registers a filesystem root directory with a prefix.
    ///
    /// This allows mapping a prefix to a filesystem location for runtime
    /// resource loading.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::file::ResourceManager;
    ///
    /// // Map "data" prefix to a filesystem directory
    /// ResourceManager::global().register_filesystem_root("data", "/app/data");
    ///
    /// // Now "data:/config.json" maps to "/app/data/config.json"
    /// ```
    pub fn register_filesystem_root(&self, prefix: &str, root: impl Into<PathBuf>) {
        self.filesystem_roots.write().insert(prefix.to_string(), root.into());
    }

    /// Unregisters a filesystem root.
    pub fn unregister_filesystem_root(&self, prefix: &str) -> bool {
        self.filesystem_roots.write().remove(prefix).is_some()
    }

    /// Gets an embedded resource as bytes.
    ///
    /// Returns `None` if the resource doesn't exist or isn't embedded.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(data) = resources.get(":/images/icon.png") {
    ///     // data is &'static [u8]
    /// }
    /// ```
    pub fn get(&self, path: &str) -> Option<&'static [u8]> {
        let parsed = ResourcePath::parse(path);
        if !parsed.is_embedded {
            return None;
        }

        let embedded = self.embedded.read();
        embedded.get(parsed.prefix.as_ref())?.get_file(&parsed.path)
    }

    /// Gets an embedded resource as UTF-8 text.
    ///
    /// Returns `None` if the resource doesn't exist, isn't embedded,
    /// or isn't valid UTF-8.
    ///
    /// # Example
    ///
    /// ```ignore
    /// if let Some(css) = resources.get_text(":/styles/main.css") {
    ///     // css is &'static str
    /// }
    /// ```
    pub fn get_text(&self, path: &str) -> Option<&'static str> {
        let parsed = ResourcePath::parse(path);
        if !parsed.is_embedded {
            return None;
        }

        let embedded = self.embedded.read();
        embedded.get(parsed.prefix.as_ref())?.get_text(&parsed.path)
    }

    /// Checks if a resource exists.
    ///
    /// For embedded resources, checks the embedded directory.
    /// For filesystem paths, checks the filesystem.
    pub fn exists(&self, path: &str) -> bool {
        let parsed = ResourcePath::parse(path);
        if parsed.is_embedded {
            let embedded = self.embedded.read();
            embedded
                .get(parsed.prefix.as_ref())
                .is_some_and(|dir| dir.contains(&parsed.path))
        } else {
            Path::new(&*parsed.path).exists()
        }
    }

    /// Lists resources in a directory path.
    ///
    /// For embedded resources, lists entries in the embedded directory.
    /// Returns paths relative to the queried directory.
    ///
    /// # Example
    ///
    /// ```ignore
    /// for entry in resources.list(":/images/") {
    ///     println!("Found: {}", entry.path());
    /// }
    /// ```
    pub fn list(&self, path: &str) -> Vec<ResourceEntry> {
        let parsed = ResourcePath::parse(path);
        if !parsed.is_embedded {
            return Vec::new();
        }

        let embedded = self.embedded.read();
        embedded
            .get(parsed.prefix.as_ref())
            .map(|dir| dir.list_entries(&parsed.path))
            .unwrap_or_default()
    }

    /// Lists all file paths under a prefix.
    ///
    /// Returns all embedded file paths (recursively) for the given prefix.
    pub fn list_all(&self, prefix: &str) -> Vec<&'static str> {
        let embedded = self.embedded.read();
        embedded
            .get(prefix)
            .map(|dir| dir.list_files())
            .unwrap_or_default()
    }

    /// Loads a resource asynchronously from the filesystem.
    ///
    /// This is useful for large resources that shouldn't block.
    /// For embedded resources, use `get()` instead (they're already in memory).
    ///
    /// # Example
    ///
    /// ```ignore
    /// let data = resources.load_async("/path/to/large_file.bin").await?;
    /// ```
    pub async fn load_async(&self, path: &str) -> FileResult<Vec<u8>> {
        let parsed = ResourcePath::parse(path);

        let full_path = if parsed.is_embedded {
            // For embedded paths with filesystem roots, resolve to filesystem
            let roots = self.filesystem_roots.read();
            if let Some(root) = roots.get(parsed.prefix.as_ref()) {
                root.join(&*parsed.path)
            } else {
                return Err(FileError::not_found(path));
            }
        } else {
            PathBuf::from(&*parsed.path)
        };

        tokio::fs::read(&full_path)
            .await
            .map_err(|e| FileError::from_io(e, full_path))
    }

    /// Loads a text resource asynchronously from the filesystem.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let text = resources.load_text_async("/path/to/config.txt").await?;
    /// ```
    pub async fn load_text_async(&self, path: &str) -> FileResult<String> {
        let parsed = ResourcePath::parse(path);

        let full_path = if parsed.is_embedded {
            let roots = self.filesystem_roots.read();
            if let Some(root) = roots.get(parsed.prefix.as_ref()) {
                root.join(&*parsed.path)
            } else {
                return Err(FileError::not_found(path));
            }
        } else {
            PathBuf::from(&*parsed.path)
        };

        tokio::fs::read_to_string(&full_path)
            .await
            .map_err(|e| FileError::from_io(e, full_path))
    }

    /// Loads a resource synchronously from the filesystem.
    ///
    /// For embedded resources, prefer `get()` which returns static data.
    /// This is useful for filesystem resources when async isn't available.
    pub fn load_sync(&self, path: &str) -> FileResult<Vec<u8>> {
        let parsed = ResourcePath::parse(path);

        // First check embedded
        if parsed.is_embedded {
            let embedded = self.embedded.read();
            if let Some(dir) = embedded.get(parsed.prefix.as_ref()) {
                if let Some(data) = dir.get_file(&parsed.path) {
                    return Ok(data.to_vec());
                }
            }
            // Fall through to filesystem roots
            let roots = self.filesystem_roots.read();
            if let Some(root) = roots.get(parsed.prefix.as_ref()) {
                let full_path = root.join(&*parsed.path);
                return std::fs::read(&full_path)
                    .map_err(|e| FileError::from_io(e, full_path));
            }
            return Err(FileError::not_found(path));
        }

        std::fs::read(&*parsed.path)
            .map_err(|e| FileError::from_io(e, &*parsed.path))
    }

    /// Loads a text resource synchronously from the filesystem.
    pub fn load_text_sync(&self, path: &str) -> FileResult<String> {
        let parsed = ResourcePath::parse(path);

        // First check embedded
        if parsed.is_embedded {
            let embedded = self.embedded.read();
            if let Some(dir) = embedded.get(parsed.prefix.as_ref()) {
                if let Some(text) = dir.get_text(&parsed.path) {
                    return Ok(text.to_string());
                }
            }
            // Fall through to filesystem roots
            let roots = self.filesystem_roots.read();
            if let Some(root) = roots.get(parsed.prefix.as_ref()) {
                let full_path = root.join(&*parsed.path);
                return std::fs::read_to_string(&full_path)
                    .map_err(|e| FileError::from_io(e, full_path));
            }
            return Err(FileError::not_found(path));
        }

        std::fs::read_to_string(&*parsed.path)
            .map_err(|e| FileError::from_io(e, &*parsed.path))
    }

    /// Returns the number of registered embedded prefixes.
    pub fn embedded_count(&self) -> usize {
        self.embedded.read().len()
    }

    /// Returns all registered embedded prefixes.
    pub fn embedded_prefixes(&self) -> Vec<String> {
        self.embedded.read().keys().cloned().collect()
    }

    /// Clears all registered resources.
    pub fn clear(&self) {
        self.embedded.write().clear();
        self.filesystem_roots.write().clear();
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ResourceManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResourceManager")
            .field("embedded_prefixes", &self.embedded_prefixes())
            .field("filesystem_roots", &self.filesystem_roots.read().keys().collect::<Vec<_>>())
            .finish()
    }
}

/// A lazily-loaded resource.
///
/// The resource is loaded on first access and cached for subsequent accesses.
/// This is useful for resources that may not be needed immediately.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::LazyResource;
///
/// let icon = LazyResource::new(":/images/icon.png");
///
/// // Resource is not loaded yet
/// assert!(!icon.is_loaded());
///
/// // First access triggers loading
/// if let Some(data) = icon.get() {
///     // Use the data
/// }
///
/// // Subsequent accesses use cached data
/// assert!(icon.is_loaded());
/// ```
pub struct LazyResource {
    path: String,
    data: OnceLock<Option<&'static [u8]>>,
}

impl LazyResource {
    /// Creates a new lazy resource for the given path.
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            data: OnceLock::new(),
        }
    }

    /// Gets the resource data, loading it if necessary.
    ///
    /// Returns `None` if the resource doesn't exist.
    pub fn get(&self) -> Option<&'static [u8]> {
        *self.data.get_or_init(|| {
            ResourceManager::global().get(&self.path)
        })
    }

    /// Gets the resource as UTF-8 text.
    ///
    /// Returns `None` if the resource doesn't exist or isn't valid UTF-8.
    pub fn get_text(&self) -> Option<&'static str> {
        // For text, we can't cache separately, so just call through
        ResourceManager::global().get_text(&self.path)
    }

    /// Returns the resource path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns whether the resource has been loaded.
    pub fn is_loaded(&self) -> bool {
        self.data.get().is_some()
    }
}

impl std::fmt::Debug for LazyResource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyResource")
            .field("path", &self.path)
            .field("is_loaded", &self.is_loaded())
            .finish()
    }
}

/// A typed lazy resource with custom deserialization.
///
/// This allows loading resources as specific types (e.g., JSON config).
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::file::TypedLazyResource;
/// use serde::Deserialize;
///
/// #[derive(Deserialize)]
/// struct Config {
///     name: String,
///     debug: bool,
/// }
///
/// let config: TypedLazyResource<Config> = TypedLazyResource::json(":/config.json");
///
/// if let Some(cfg) = config.get() {
///     println!("App name: {}", cfg.name);
/// }
/// ```
pub struct TypedLazyResource<T> {
    path: String,
    data: OnceLock<Option<T>>,
    loader: fn(&str) -> Option<T>,
}

impl<T> TypedLazyResource<T> {
    /// Creates a lazy resource with a custom loader function.
    pub fn with_loader(path: impl Into<String>, loader: fn(&str) -> Option<T>) -> Self {
        Self {
            path: path.into(),
            data: OnceLock::new(),
            loader,
        }
    }

    /// Gets the loaded value, loading if necessary.
    pub fn get(&self) -> Option<&T> {
        self.data.get_or_init(|| (self.loader)(&self.path)).as_ref()
    }

    /// Returns the resource path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns whether the resource has been loaded.
    pub fn is_loaded(&self) -> bool {
        self.data.get().is_some()
    }
}

impl<T: for<'de> serde::Deserialize<'de>> TypedLazyResource<T> {
    /// Creates a lazy resource that loads JSON data.
    pub fn json(path: impl Into<String>) -> Self {
        Self::with_loader(path, |p| {
            let text = ResourceManager::global().get_text(p)?;
            serde_json::from_str(text).ok()
        })
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for TypedLazyResource<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypedLazyResource")
            .field("path", &self.path)
            .field("is_loaded", &self.is_loaded())
            .finish()
    }
}

// Re-export include_dir types for user convenience
pub use include_dir::Dir as IncludeDir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_path_parsing() {
        // Default prefix
        let p = ResourcePath::parse(":/images/icon.png");
        assert_eq!(p.prefix.as_ref(), "");
        assert_eq!(p.path.as_ref(), "images/icon.png");
        assert!(p.is_embedded);

        // Custom prefix
        let p = ResourcePath::parse("assets:/fonts/main.ttf");
        assert_eq!(p.prefix.as_ref(), "assets");
        assert_eq!(p.path.as_ref(), "fonts/main.ttf");
        assert!(p.is_embedded);

        // Absolute filesystem path
        let p = ResourcePath::parse("/absolute/path/to/file");
        assert_eq!(p.prefix.as_ref(), "");
        assert_eq!(p.path.as_ref(), "/absolute/path/to/file");
        assert!(!p.is_embedded);

        // Relative filesystem path
        let p = ResourcePath::parse("relative/path.txt");
        assert_eq!(p.prefix.as_ref(), "");
        assert_eq!(p.path.as_ref(), "relative/path.txt");
        assert!(!p.is_embedded);
    }

    #[test]
    fn test_resource_manager_basic() {
        let manager = ResourceManager::new();

        // No resources registered
        assert!(manager.get(":/nonexistent.txt").is_none());
        assert!(!manager.exists(":/nonexistent.txt"));
        assert_eq!(manager.embedded_count(), 0);
    }

    #[test]
    fn test_resource_entry() {
        let file = ResourceEntry::File("test.txt".to_string());
        assert!(file.is_file());
        assert!(!file.is_dir());
        assert_eq!(file.path(), "test.txt");

        let dir = ResourceEntry::Directory("subdir".to_string());
        assert!(!dir.is_file());
        assert!(dir.is_dir());
        assert_eq!(dir.path(), "subdir");
    }

    #[test]
    fn test_lazy_resource_not_loaded() {
        let lazy = LazyResource::new(":/nonexistent.txt");
        assert!(!lazy.is_loaded());
        assert_eq!(lazy.path(), ":/nonexistent.txt");
    }

    #[test]
    fn test_filesystem_exists() {
        let manager = ResourceManager::new();

        // Current directory should exist
        assert!(manager.exists("."));

        // Non-existent path
        assert!(!manager.exists("/definitely/not/a/real/path/12345"));
    }
}
