//! Path manipulation and standard directory utilities.
//!
//! This module provides cross-platform path operations and standard directory
//! discovery. It wraps Rust's standard library path APIs and the `directories`
//! crate for ergonomic, cross-platform path handling.
//!
//! # Path Manipulation
//!
//! ```ignore
//! use horizon_lattice::file::{join_path, parent, file_name, extension, canonicalize};
//!
//! // Join path components
//! let path = join_path("/home/user", "documents/file.txt");
//! assert_eq!(path.to_string_lossy(), "/home/user/documents/file.txt");
//!
//! // Get path components
//! let p = PathBuf::from("/home/user/file.txt");
//! assert_eq!(file_name(&p), Some("file.txt"));
//! assert_eq!(extension(&p), Some("txt"));
//! assert_eq!(parent(&p).unwrap().to_string_lossy(), "/home/user");
//!
//! // Canonicalize (resolve symlinks and make absolute)
//! let canonical = canonicalize("./relative/path")?;
//! ```
//!
//! # Standard Directories
//!
//! ```ignore
//! use horizon_lattice::file::{home_dir, config_dir, data_dir, cache_dir, temp_dir};
//!
//! // Get system standard directories
//! let home = home_dir()?;           // e.g., /home/user
//! let config = config_dir()?;       // e.g., ~/.config
//! let data = data_dir()?;           // e.g., ~/.local/share
//! let cache = cache_dir()?;         // e.g., ~/.cache
//! let temp = temp_dir();            // e.g., /tmp
//! let docs = documents_dir()?;      // e.g., ~/Documents
//! ```
//!
//! # Application Paths
//!
//! ```ignore
//! use horizon_lattice::file::AppPaths;
//!
//! // Create app-specific paths for your application
//! let paths = AppPaths::new("com.example", "MyCompany", "MyApp")?;
//!
//! // Access app-specific directories
//! let config = paths.config();    // e.g., ~/.config/myapp
//! let data = paths.data();        // e.g., ~/.local/share/myapp
//! let cache = paths.cache();      // e.g., ~/.cache/myapp
//! let logs = paths.logs();        // e.g., ~/.local/share/myapp/logs
//! ```

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs, UserDirs};

use super::error::{FileError, FileErrorKind, FileResult};

// ============================================================================
// Path Manipulation
// ============================================================================

/// Joins two path components together.
///
/// This is a convenience wrapper around `Path::join` that accepts any types
/// that can be converted to paths.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::join_path;
///
/// let path = join_path("/home/user", "documents/file.txt");
/// assert_eq!(path.to_string_lossy(), "/home/user/documents/file.txt");
///
/// // Works with PathBuf, String, &str, etc.
/// let base = PathBuf::from("/var/log");
/// let full = join_path(&base, "app.log");
/// ```
pub fn join_path(base: impl AsRef<Path>, component: impl AsRef<Path>) -> PathBuf {
    base.as_ref().join(component)
}

/// Joins multiple path components together.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::join_paths;
///
/// let path = join_paths(&["/home", "user", "documents", "file.txt"]);
/// assert_eq!(path.to_string_lossy(), "/home/user/documents/file.txt");
/// ```
pub fn join_paths<P: AsRef<Path>>(components: &[P]) -> PathBuf {
    let mut result = PathBuf::new();
    for component in components {
        result.push(component);
    }
    result
}

/// Returns the parent directory of a path.
///
/// Returns `None` if the path has no parent (e.g., "/" or "C:\").
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::parent;
/// use std::path::PathBuf;
///
/// assert_eq!(parent("/home/user/file.txt"), Some(PathBuf::from("/home/user")));
/// assert_eq!(parent("/"), None);
/// ```
pub fn parent(path: impl AsRef<Path>) -> Option<PathBuf> {
    path.as_ref().parent().map(|p| p.to_path_buf())
}

/// Returns the final component of a path as a string.
///
/// Returns `None` if the path terminates in ".." or is empty, or if the
/// file name is not valid UTF-8.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::file_name;
///
/// assert_eq!(file_name("/home/user/file.txt"), Some("file.txt".to_string()));
/// assert_eq!(file_name("/home/user/"), Some("user".to_string()));
/// assert_eq!(file_name("/"), None);
/// ```
pub fn file_name(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_name()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Returns the final component of a path as an OsString.
///
/// This is useful when the filename may not be valid UTF-8.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::file_name_os;
///
/// let name = file_name_os("/home/user/file.txt");
/// assert_eq!(name.map(|s| s.to_string_lossy().to_string()), Some("file.txt".to_string()));
/// ```
pub fn file_name_os(path: impl AsRef<Path>) -> Option<std::ffi::OsString> {
    path.as_ref().file_name().map(|s| s.to_os_string())
}

/// Returns the file stem (filename without extension).
///
/// Returns `None` if the path has no file name, or if the file stem is not
/// valid UTF-8.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::file_stem;
///
/// assert_eq!(file_stem("/home/user/file.txt"), Some("file".to_string()));
/// assert_eq!(file_stem("/home/user/archive.tar.gz"), Some("archive.tar".to_string()));
/// assert_eq!(file_stem("/home/user/.gitignore"), Some(".gitignore".to_string()));
/// ```
pub fn file_stem(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Returns the file extension.
///
/// Returns `None` if the path has no extension, or if the extension is not
/// valid UTF-8.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::extension;
///
/// assert_eq!(extension("/home/user/file.txt"), Some("txt".to_string()));
/// assert_eq!(extension("/home/user/archive.tar.gz"), Some("gz".to_string()));
/// assert_eq!(extension("/home/user/.gitignore"), None);
/// ```
pub fn extension(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

/// Returns the canonical, absolute form of a path.
///
/// This resolves all symbolic links, removes `.` and `..` components, and
/// returns an absolute path. The path must exist.
///
/// # Errors
///
/// Returns an error if:
/// - The path does not exist
/// - A component of the path is not a directory
/// - The process lacks permissions to access a component
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::canonicalize;
///
/// let canonical = canonicalize("./relative/path")?;
/// assert!(canonical.is_absolute());
/// ```
pub fn canonicalize(path: impl AsRef<Path>) -> FileResult<PathBuf> {
    let path = path.as_ref();
    std::fs::canonicalize(path).map_err(|e| FileError::from_io(e, path))
}

/// Converts a path to an absolute path without resolving symlinks.
///
/// Unlike `canonicalize`, this does not require the path to exist and does
/// not resolve symbolic links. It simply prepends the current working
/// directory if the path is relative.
///
/// # Errors
///
/// Returns an error if the current directory cannot be determined.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::absolute_path;
///
/// let abs = absolute_path("relative/path")?;
/// assert!(abs.is_absolute());
/// ```
pub fn absolute_path(path: impl AsRef<Path>) -> FileResult<PathBuf> {
    let path = path.as_ref();
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let cwd = std::env::current_dir().map_err(|e| FileError::from_io(e, path))?;
        Ok(cwd.join(path))
    }
}

/// Computes a relative path from a base to a target.
///
/// Returns the path to `target` relative to `base`. Both paths should be
/// either both absolute or both relative for consistent results.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::relative_to;
///
/// let rel = relative_to("/home/user/docs", "/home/user/docs/file.txt");
/// assert_eq!(rel.to_string_lossy(), "file.txt");
///
/// let rel = relative_to("/home/user/docs", "/home/user/images/photo.jpg");
/// assert_eq!(rel.to_string_lossy(), "../images/photo.jpg");
/// ```
pub fn relative_to(base: impl AsRef<Path>, target: impl AsRef<Path>) -> PathBuf {
    let base = base.as_ref();
    let target = target.as_ref();

    // Find common ancestor
    let mut base_components = base.components().peekable();
    let mut target_components = target.components().peekable();

    // Skip common prefix
    while let (Some(b), Some(t)) = (base_components.peek(), target_components.peek()) {
        if b == t {
            base_components.next();
            target_components.next();
        } else {
            break;
        }
    }

    // Build relative path
    let mut result = PathBuf::new();

    // Add ".." for each remaining base component
    for _ in base_components {
        result.push("..");
    }

    // Add remaining target components
    for component in target_components {
        result.push(component);
    }

    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

/// Changes the file extension of a path.
///
/// Returns a new path with the given extension. If the path has no extension,
/// one is added. If `extension` is empty, the extension is removed.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::with_extension;
///
/// assert_eq!(with_extension("file.txt", "md").to_string_lossy(), "file.md");
/// assert_eq!(with_extension("file.tar.gz", "xz").to_string_lossy(), "file.tar.xz");
/// assert_eq!(with_extension("file", "txt").to_string_lossy(), "file.txt");
/// assert_eq!(with_extension("file.txt", "").to_string_lossy(), "file");
/// ```
pub fn with_extension(path: impl AsRef<Path>, extension: impl AsRef<OsStr>) -> PathBuf {
    let mut new_path = path.as_ref().to_path_buf();
    new_path.set_extension(extension);
    new_path
}

/// Changes the file name of a path.
///
/// Returns a new path with the given file name.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::with_file_name;
///
/// assert_eq!(
///     with_file_name("/home/user/old.txt", "new.txt").to_string_lossy(),
///     "/home/user/new.txt"
/// );
/// ```
pub fn with_file_name(path: impl AsRef<Path>, file_name: impl AsRef<OsStr>) -> PathBuf {
    let mut new_path = path.as_ref().to_path_buf();
    new_path.set_file_name(file_name);
    new_path
}

/// Returns true if the path is absolute.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::is_absolute;
///
/// assert!(is_absolute("/home/user"));
/// assert!(!is_absolute("relative/path"));
/// ```
pub fn is_absolute(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_absolute()
}

/// Returns true if the path is relative.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::is_relative;
///
/// assert!(is_relative("relative/path"));
/// assert!(!is_relative("/home/user"));
/// ```
pub fn is_relative(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_relative()
}

/// Normalizes a path by removing `.` and `..` components where possible.
///
/// This does not access the filesystem and does not resolve symlinks.
/// It operates purely on the path string.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::normalize_path;
///
/// assert_eq!(normalize_path("./foo/bar/../baz").to_string_lossy(), "foo/baz");
/// assert_eq!(normalize_path("/foo/./bar/./baz").to_string_lossy(), "/foo/bar/baz");
/// ```
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    use std::path::Component;

    let mut result = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            Component::Prefix(p) => result.push(p.as_os_str()),
            Component::RootDir => result.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !result.pop() {
                    result.push("..");
                }
            }
            Component::Normal(c) => result.push(c),
        }
    }
    if result.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        result
    }
}

// ============================================================================
// Standard Directories
// ============================================================================

/// Returns the user's home directory.
///
/// # Platform Behavior
///
/// - **Linux/macOS**: Returns `$HOME` (e.g., `/home/alice`)
/// - **Windows**: Returns `{FOLDERID_Profile}` (e.g., `C:\Users\Alice`)
///
/// # Errors
///
/// Returns an error if the home directory cannot be determined.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::home_dir;
///
/// let home = home_dir()?;
/// println!("Home: {}", home.display());
/// ```
pub fn home_dir() -> FileResult<PathBuf> {
    BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine home directory",
                )),
            )
        })
}

/// Returns the user's configuration directory.
///
/// This is the directory for user-specific configuration files.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_CONFIG_HOME` or `~/.config`
/// - **macOS**: `~/Library/Application Support`
/// - **Windows**: `{FOLDERID_RoamingAppData}` (e.g., `C:\Users\Alice\AppData\Roaming`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::config_dir;
///
/// let config = config_dir()?;
/// println!("Config: {}", config.display());
/// ```
pub fn config_dir() -> FileResult<PathBuf> {
    BaseDirs::new()
        .and_then(|dirs| dirs.config_dir().to_path_buf().into())
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine config directory",
                )),
            )
        })
}

/// Returns the user's data directory.
///
/// This is the directory for user-specific data files.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_DATA_HOME` or `~/.local/share`
/// - **macOS**: `~/Library/Application Support`
/// - **Windows**: `{FOLDERID_RoamingAppData}` (e.g., `C:\Users\Alice\AppData\Roaming`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::data_dir;
///
/// let data = data_dir()?;
/// println!("Data: {}", data.display());
/// ```
pub fn data_dir() -> FileResult<PathBuf> {
    BaseDirs::new()
        .and_then(|dirs| dirs.data_dir().to_path_buf().into())
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine data directory",
                )),
            )
        })
}

/// Returns the user's local data directory.
///
/// This is the directory for user-specific, non-roaming data files.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_DATA_HOME` or `~/.local/share`
/// - **macOS**: `~/Library/Application Support`
/// - **Windows**: `{FOLDERID_LocalAppData}` (e.g., `C:\Users\Alice\AppData\Local`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
pub fn data_local_dir() -> FileResult<PathBuf> {
    BaseDirs::new()
        .and_then(|dirs| dirs.data_local_dir().to_path_buf().into())
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine local data directory",
                )),
            )
        })
}

/// Returns the user's cache directory.
///
/// This is the directory for user-specific, non-essential cached data.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_CACHE_HOME` or `~/.cache`
/// - **macOS**: `~/Library/Caches`
/// - **Windows**: `{FOLDERID_LocalAppData}` (e.g., `C:\Users\Alice\AppData\Local`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::cache_dir;
///
/// let cache = cache_dir()?;
/// println!("Cache: {}", cache.display());
/// ```
pub fn cache_dir() -> FileResult<PathBuf> {
    BaseDirs::new()
        .and_then(|dirs| dirs.cache_dir().to_path_buf().into())
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine cache directory",
                )),
            )
        })
}

/// Returns the system's temporary directory.
///
/// This is the directory for temporary files. Files in this directory may be
/// deleted by the system at any time.
///
/// # Platform Behavior
///
/// - **Linux**: `$TMPDIR`, `$TMP`, `$TEMP`, or `/tmp`
/// - **macOS**: `$TMPDIR` or `/tmp`
/// - **Windows**: `GetTempPath()` result
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::temp_dir;
///
/// let temp = temp_dir();
/// println!("Temp: {}", temp.display());
/// ```
pub fn temp_dir() -> PathBuf {
    std::env::temp_dir()
}

/// Returns the user's documents directory.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_DOCUMENTS_DIR` or `~/Documents`
/// - **macOS**: `~/Documents`
/// - **Windows**: `{FOLDERID_Documents}` (e.g., `C:\Users\Alice\Documents`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::documents_dir;
///
/// let docs = documents_dir()?;
/// println!("Documents: {}", docs.display());
/// ```
pub fn documents_dir() -> FileResult<PathBuf> {
    UserDirs::new()
        .and_then(|dirs| dirs.document_dir().map(|p| p.to_path_buf()))
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine documents directory",
                )),
            )
        })
}

/// Returns the user's desktop directory.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_DESKTOP_DIR` or `~/Desktop`
/// - **macOS**: `~/Desktop`
/// - **Windows**: `{FOLDERID_Desktop}` (e.g., `C:\Users\Alice\Desktop`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
pub fn desktop_dir() -> FileResult<PathBuf> {
    UserDirs::new()
        .and_then(|dirs| dirs.desktop_dir().map(|p| p.to_path_buf()))
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine desktop directory",
                )),
            )
        })
}

/// Returns the user's downloads directory.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_DOWNLOAD_DIR` or `~/Downloads`
/// - **macOS**: `~/Downloads`
/// - **Windows**: `{FOLDERID_Downloads}` (e.g., `C:\Users\Alice\Downloads`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
pub fn downloads_dir() -> FileResult<PathBuf> {
    UserDirs::new()
        .and_then(|dirs| dirs.download_dir().map(|p| p.to_path_buf()))
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine downloads directory",
                )),
            )
        })
}

/// Returns the user's pictures directory.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_PICTURES_DIR` or `~/Pictures`
/// - **macOS**: `~/Pictures`
/// - **Windows**: `{FOLDERID_Pictures}` (e.g., `C:\Users\Alice\Pictures`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
pub fn pictures_dir() -> FileResult<PathBuf> {
    UserDirs::new()
        .and_then(|dirs| dirs.picture_dir().map(|p| p.to_path_buf()))
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine pictures directory",
                )),
            )
        })
}

/// Returns the user's music directory.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_MUSIC_DIR` or `~/Music`
/// - **macOS**: `~/Music`
/// - **Windows**: `{FOLDERID_Music}` (e.g., `C:\Users\Alice\Music`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
pub fn music_dir() -> FileResult<PathBuf> {
    UserDirs::new()
        .and_then(|dirs| dirs.audio_dir().map(|p| p.to_path_buf()))
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine music directory",
                )),
            )
        })
}

/// Returns the user's videos directory.
///
/// # Platform Behavior
///
/// - **Linux**: `$XDG_VIDEOS_DIR` or `~/Videos`
/// - **macOS**: `~/Movies`
/// - **Windows**: `{FOLDERID_Videos}` (e.g., `C:\Users\Alice\Videos`)
///
/// # Errors
///
/// Returns an error if the directory cannot be determined.
pub fn videos_dir() -> FileResult<PathBuf> {
    UserDirs::new()
        .and_then(|dirs| dirs.video_dir().map(|p| p.to_path_buf()))
        .ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine videos directory",
                )),
            )
        })
}

// ============================================================================
// Application Paths
// ============================================================================

/// Application-specific paths for configuration, data, cache, and logs.
///
/// This struct provides convenient access to standard application directories
/// following platform conventions. The directories are determined based on
/// the application's qualifier, organization, and name.
///
/// # Platform Behavior
///
/// ## Linux
/// - Config: `~/.config/<app_name>/`
/// - Data: `~/.local/share/<app_name>/`
/// - Cache: `~/.cache/<app_name>/`
///
/// ## macOS
/// - Config: `~/Library/Application Support/<org>.<app>/`
/// - Data: `~/Library/Application Support/<org>.<app>/`
/// - Cache: `~/Library/Caches/<org>.<app>/`
///
/// ## Windows
/// - Config: `C:\Users\<user>\AppData\Roaming\<org>\<app>\config\`
/// - Data: `C:\Users\<user>\AppData\Roaming\<org>\<app>\data\`
/// - Cache: `C:\Users\<user>\AppData\Local\<org>\<app>\cache\`
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::AppPaths;
///
/// // Create paths for "My App" by "My Company"
/// let paths = AppPaths::new("com.example", "MyCompany", "MyApp")?;
///
/// // Access directories (does not create them)
/// let config = paths.config();
/// let data = paths.data();
/// let cache = paths.cache();
/// let logs = paths.logs();
///
/// // Ensure directories exist
/// paths.ensure_all()?;
///
/// // Or create specific ones
/// paths.ensure_config()?;
/// ```
#[derive(Debug, Clone)]
pub struct AppPaths {
    config: PathBuf,
    data: PathBuf,
    data_local: PathBuf,
    cache: PathBuf,
    logs: PathBuf,
    preferences: PathBuf,
}

impl AppPaths {
    /// Creates a new `AppPaths` for the given application.
    ///
    /// # Arguments
    ///
    /// * `qualifier` - A reverse-domain identifier (e.g., "com.example")
    /// * `organization` - The organization name (e.g., "MyCompany")
    /// * `application` - The application name (e.g., "MyApp")
    ///
    /// # Errors
    ///
    /// Returns an error if the application directories cannot be determined.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use horizon_lattice::file::AppPaths;
    ///
    /// let paths = AppPaths::new("com.example", "MyCompany", "MyApp")?;
    /// ```
    pub fn new(qualifier: &str, organization: &str, application: &str) -> FileResult<Self> {
        let dirs = ProjectDirs::from(qualifier, organization, application).ok_or_else(|| {
            FileError::new(
                FileErrorKind::NotFound,
                None,
                Some(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "could not determine application directories",
                )),
            )
        })?;

        let data = dirs.data_dir().to_path_buf();
        let logs = data.join("logs");
        let preferences = dirs.preference_dir().to_path_buf();

        Ok(Self {
            config: dirs.config_dir().to_path_buf(),
            data,
            data_local: dirs.data_local_dir().to_path_buf(),
            cache: dirs.cache_dir().to_path_buf(),
            logs,
            preferences,
        })
    }

    /// Creates `AppPaths` using a simple application name.
    ///
    /// This is a convenience method that uses reasonable defaults for
    /// qualifier and organization.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use horizon_lattice::file::AppPaths;
    ///
    /// let paths = AppPaths::from_name("myapp")?;
    /// ```
    pub fn from_name(application: &str) -> FileResult<Self> {
        Self::new("", "", application)
    }

    /// Returns the application's configuration directory.
    ///
    /// This directory is intended for configuration files that the user
    /// may want to edit or back up.
    pub fn config(&self) -> &Path {
        &self.config
    }

    /// Returns the application's data directory.
    ///
    /// This directory is intended for application data that should persist
    /// across sessions and may be synced across devices.
    pub fn data(&self) -> &Path {
        &self.data
    }

    /// Returns the application's local data directory.
    ///
    /// This directory is intended for application data that should persist
    /// but is specific to this machine (not synced).
    pub fn data_local(&self) -> &Path {
        &self.data_local
    }

    /// Returns the application's cache directory.
    ///
    /// This directory is intended for cached data that can be regenerated
    /// if deleted. The system may clear this directory automatically.
    pub fn cache(&self) -> &Path {
        &self.cache
    }

    /// Returns the application's log directory.
    ///
    /// This is a subdirectory of the data directory intended for log files.
    pub fn logs(&self) -> &Path {
        &self.logs
    }

    /// Returns the application's preferences directory.
    ///
    /// On most platforms, this is the same as the config directory.
    /// On macOS, this points to `~/Library/Preferences`.
    pub fn preferences(&self) -> &Path {
        &self.preferences
    }

    /// Creates the configuration directory if it doesn't exist.
    pub fn ensure_config(&self) -> FileResult<()> {
        std::fs::create_dir_all(&self.config).map_err(|e| FileError::from_io(e, &self.config))
    }

    /// Creates the data directory if it doesn't exist.
    pub fn ensure_data(&self) -> FileResult<()> {
        std::fs::create_dir_all(&self.data).map_err(|e| FileError::from_io(e, &self.data))
    }

    /// Creates the local data directory if it doesn't exist.
    pub fn ensure_data_local(&self) -> FileResult<()> {
        std::fs::create_dir_all(&self.data_local)
            .map_err(|e| FileError::from_io(e, &self.data_local))
    }

    /// Creates the cache directory if it doesn't exist.
    pub fn ensure_cache(&self) -> FileResult<()> {
        std::fs::create_dir_all(&self.cache).map_err(|e| FileError::from_io(e, &self.cache))
    }

    /// Creates the logs directory if it doesn't exist.
    pub fn ensure_logs(&self) -> FileResult<()> {
        std::fs::create_dir_all(&self.logs).map_err(|e| FileError::from_io(e, &self.logs))
    }

    /// Creates all application directories if they don't exist.
    pub fn ensure_all(&self) -> FileResult<()> {
        self.ensure_config()?;
        self.ensure_data()?;
        self.ensure_data_local()?;
        self.ensure_cache()?;
        self.ensure_logs()?;
        Ok(())
    }

    /// Returns a path within the config directory.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let config_file = paths.config_path("settings.toml");
    /// ```
    pub fn config_path(&self, name: impl AsRef<Path>) -> PathBuf {
        self.config.join(name)
    }

    /// Returns a path within the data directory.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let db_file = paths.data_path("database.db");
    /// ```
    pub fn data_path(&self, name: impl AsRef<Path>) -> PathBuf {
        self.data.join(name)
    }

    /// Returns a path within the cache directory.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let cache_file = paths.cache_path("thumbnails/image.png");
    /// ```
    pub fn cache_path(&self, name: impl AsRef<Path>) -> PathBuf {
        self.cache.join(name)
    }

    /// Returns a path within the logs directory.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let log_file = paths.log_path("app.log");
    /// ```
    pub fn log_path(&self, name: impl AsRef<Path>) -> PathBuf {
        self.logs.join(name)
    }
}

// ============================================================================
// Path Builder
// ============================================================================

/// A builder for constructing paths fluently.
///
/// # Examples
///
/// ```ignore
/// use horizon_lattice::file::PathBuilder;
///
/// let path = PathBuilder::new("/home/user")
///     .push("documents")
///     .push("project")
///     .with_file("report.txt")
///     .build();
///
/// assert_eq!(path.to_string_lossy(), "/home/user/documents/project/report.txt");
/// ```
#[derive(Debug, Clone)]
pub struct PathBuilder {
    path: PathBuf,
}

impl PathBuilder {
    /// Creates a new `PathBuilder` starting from the given base path.
    pub fn new(base: impl AsRef<Path>) -> Self {
        Self {
            path: base.as_ref().to_path_buf(),
        }
    }

    /// Creates a new `PathBuilder` starting from the current directory.
    pub fn current_dir() -> FileResult<Self> {
        let cwd = std::env::current_dir()
            .map_err(|e| FileError::new(FileErrorKind::Other, None, Some(e)))?;
        Ok(Self::new(cwd))
    }

    /// Creates a new `PathBuilder` starting from the home directory.
    pub fn home() -> FileResult<Self> {
        Ok(Self::new(home_dir()?))
    }

    /// Appends a path component.
    pub fn push(mut self, component: impl AsRef<Path>) -> Self {
        self.path.push(component);
        self
    }

    /// Sets the file name (last component).
    pub fn with_file(mut self, name: impl AsRef<OsStr>) -> Self {
        self.path.set_file_name(name);
        self
    }

    /// Sets the file extension.
    pub fn with_extension(mut self, ext: impl AsRef<OsStr>) -> Self {
        self.path.set_extension(ext);
        self
    }

    /// Removes the last component from the path.
    pub fn pop(mut self) -> Self {
        self.path.pop();
        self
    }

    /// Returns the constructed path.
    pub fn build(self) -> PathBuf {
        self.path
    }

    /// Returns the constructed path, canonicalized.
    pub fn build_canonical(self) -> FileResult<PathBuf> {
        canonicalize(self.path)
    }

    /// Returns the constructed path as an absolute path.
    pub fn build_absolute(self) -> FileResult<PathBuf> {
        absolute_path(self.path)
    }
}

impl From<PathBuilder> for PathBuf {
    fn from(builder: PathBuilder) -> Self {
        builder.build()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Path manipulation tests

    #[test]
    fn test_join_path() {
        let path = join_path("/home/user", "documents/file.txt");
        assert_eq!(path, PathBuf::from("/home/user/documents/file.txt"));
    }

    #[test]
    fn test_join_paths() {
        let path = join_paths(&["/home", "user", "documents"]);
        assert_eq!(path, PathBuf::from("/home/user/documents"));
    }

    #[test]
    fn test_parent() {
        let path = PathBuf::from("/home/user/file.txt");
        assert_eq!(parent(&path), Some(PathBuf::from("/home/user")));

        // Root "/" has no parent
        let root = PathBuf::from("/");
        assert_eq!(parent(&root), None);
    }

    #[test]
    fn test_file_name() {
        assert_eq!(
            file_name("/home/user/file.txt"),
            Some("file.txt".to_string())
        );
        assert_eq!(file_name("/home/user/"), Some("user".to_string()));
    }

    #[test]
    fn test_file_stem() {
        assert_eq!(file_stem("/home/user/file.txt"), Some("file".to_string()));
        assert_eq!(
            file_stem("/home/user/archive.tar.gz"),
            Some("archive.tar".to_string())
        );
        assert_eq!(
            file_stem("/home/user/.gitignore"),
            Some(".gitignore".to_string())
        );
    }

    #[test]
    fn test_extension() {
        assert_eq!(extension("/home/user/file.txt"), Some("txt".to_string()));
        assert_eq!(
            extension("/home/user/archive.tar.gz"),
            Some("gz".to_string())
        );
        assert_eq!(extension("/home/user/.gitignore"), None);
    }

    #[test]
    fn test_with_extension() {
        assert_eq!(with_extension("file.txt", "md"), PathBuf::from("file.md"));
        assert_eq!(with_extension("file", "txt"), PathBuf::from("file.txt"));
        assert_eq!(with_extension("file.txt", ""), PathBuf::from("file"));
    }

    #[test]
    fn test_with_file_name() {
        assert_eq!(
            with_file_name("/home/user/old.txt", "new.txt"),
            PathBuf::from("/home/user/new.txt")
        );
    }

    #[test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "Unix-style paths behave differently on Windows"
    )]
    fn test_is_absolute_relative() {
        assert!(is_absolute("/home/user"));
        assert!(!is_absolute("relative/path"));
        assert!(is_relative("relative/path"));
        assert!(!is_relative("/home/user"));
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("./foo/bar/../baz"), PathBuf::from("foo/baz"));
        assert_eq!(
            normalize_path("/foo/./bar/./baz"),
            PathBuf::from("/foo/bar/baz")
        );
        assert_eq!(normalize_path(".."), PathBuf::from(".."));
        assert_eq!(normalize_path("."), PathBuf::from("."));
    }

    #[test]
    fn test_relative_to() {
        let rel = relative_to("/home/user/docs", "/home/user/docs/file.txt");
        assert_eq!(rel, PathBuf::from("file.txt"));

        let rel = relative_to("/home/user/docs", "/home/user/images/photo.jpg");
        assert_eq!(rel, PathBuf::from("../images/photo.jpg"));

        let rel = relative_to("/home/user", "/home/user");
        assert_eq!(rel, PathBuf::from("."));
    }

    #[test]
    fn test_canonicalize_nonexistent() {
        let result = canonicalize("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), FileErrorKind::NotFound);
    }

    #[test]
    #[cfg_attr(
        target_os = "windows",
        ignore = "Unix-style paths behave differently on Windows"
    )]
    fn test_absolute_path() {
        // Absolute path stays absolute
        let abs = absolute_path("/home/user").unwrap();
        assert_eq!(abs, PathBuf::from("/home/user"));

        // Relative path becomes absolute
        let abs = absolute_path("relative").unwrap();
        assert!(abs.is_absolute());
    }

    // Standard directories tests

    #[test]
    fn test_home_dir() {
        let home = home_dir();
        assert!(home.is_ok());
        let home = home.unwrap();
        assert!(home.is_absolute());
    }

    #[test]
    fn test_config_dir() {
        let config = config_dir();
        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.is_absolute());
    }

    #[test]
    fn test_data_dir() {
        let data = data_dir();
        assert!(data.is_ok());
        let data = data.unwrap();
        assert!(data.is_absolute());
    }

    #[test]
    fn test_cache_dir() {
        let cache = cache_dir();
        assert!(cache.is_ok());
        let cache = cache.unwrap();
        assert!(cache.is_absolute());
    }

    #[test]
    fn test_temp_dir() {
        let temp = temp_dir();
        assert!(temp.is_absolute());
    }

    #[test]
    fn test_documents_dir() {
        // This may fail on some CI systems without a proper desktop environment
        let docs = documents_dir();
        if docs.is_ok() {
            assert!(docs.unwrap().is_absolute());
        }
    }

    // AppPaths tests

    #[test]
    fn test_app_paths_new() {
        let paths = AppPaths::new("com.example", "TestOrg", "TestApp");
        assert!(paths.is_ok());
        let paths = paths.unwrap();

        assert!(paths.config().is_absolute());
        assert!(paths.data().is_absolute());
        assert!(paths.cache().is_absolute());
        assert!(paths.logs().is_absolute());
    }

    #[test]
    fn test_app_paths_from_name() {
        let paths = AppPaths::from_name("testapp");
        assert!(paths.is_ok());
    }

    #[test]
    fn test_app_paths_subpaths() {
        let paths = AppPaths::new("com.example", "TestOrg", "TestApp").unwrap();

        let config_file = paths.config_path("settings.toml");
        assert!(config_file.ends_with("settings.toml"));

        let data_file = paths.data_path("database.db");
        assert!(data_file.ends_with("database.db"));

        let cache_file = paths.cache_path("temp.dat");
        assert!(cache_file.ends_with("temp.dat"));

        let log_file = paths.log_path("app.log");
        assert!(log_file.ends_with("app.log"));
    }

    #[test]
    fn test_app_paths_ensure() {
        let paths = AppPaths::new("com.test", "HorizonTest", "PathTest").unwrap();

        // Clean up if exists from previous run
        let _ = std::fs::remove_dir_all(paths.config());
        let _ = std::fs::remove_dir_all(paths.data());
        let _ = std::fs::remove_dir_all(paths.cache());

        // Ensure directories
        assert!(paths.ensure_all().is_ok());

        // Verify they exist
        assert!(paths.config().exists());
        assert!(paths.data().exists());
        assert!(paths.cache().exists());
        assert!(paths.logs().exists());

        // Clean up
        let _ = std::fs::remove_dir_all(paths.config());
        let _ = std::fs::remove_dir_all(paths.data());
        let _ = std::fs::remove_dir_all(paths.cache());
    }

    // PathBuilder tests

    #[test]
    fn test_path_builder() {
        // Use push for directories and files
        let path = PathBuilder::new("/home/user")
            .push("documents")
            .push("project")
            .push("report.txt")
            .build();

        assert_eq!(
            path,
            PathBuf::from("/home/user/documents/project/report.txt")
        );

        // with_file replaces the last component
        let path = PathBuilder::new("/home/user/documents/old.txt")
            .with_file("new.txt")
            .build();
        assert_eq!(path, PathBuf::from("/home/user/documents/new.txt"));
    }

    #[test]
    fn test_path_builder_with_extension() {
        let path = PathBuilder::new("/home/user")
            .push("file")
            .with_extension("txt")
            .build();

        assert_eq!(path, PathBuf::from("/home/user/file.txt"));
    }

    #[test]
    fn test_path_builder_pop() {
        let path = PathBuilder::new("/home/user/docs")
            .pop()
            .push("images")
            .build();

        assert_eq!(path, PathBuf::from("/home/user/images"));
    }

    #[test]
    fn test_path_builder_from_home() {
        let builder = PathBuilder::home();
        assert!(builder.is_ok());
        let path = builder.unwrap().push("test").build();
        assert!(path.is_absolute());
    }
}
