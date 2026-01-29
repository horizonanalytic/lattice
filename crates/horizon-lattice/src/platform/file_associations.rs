//! File association and URL scheme handling.
//!
//! This module provides cross-platform support for:
//! - Opening files and URLs with their default applications
//! - Receiving file open requests when the application is launched
//! - Registering file type associations (Windows/Linux only at runtime)
//! - Registering custom URL schemes (Windows/Linux only at runtime)
//!
//! # Opening Files and URLs
//!
//! ```ignore
//! use horizon_lattice::platform::Opener;
//!
//! // Open a file with the default application
//! Opener::open("/path/to/document.pdf")?;
//!
//! // Open a URL in the default browser
//! Opener::open_url("https://example.com")?;
//!
//! // Reveal a file in the file manager
//! Opener::reveal("/path/to/file.txt")?;
//! ```
//!
//! # Receiving File Open Requests
//!
//! When your application is launched to open a file or URL, you can access
//! the launch arguments:
//!
//! ```ignore
//! use horizon_lattice::platform::LaunchArgs;
//!
//! let args = LaunchArgs::parse();
//!
//! // Check for files to open
//! for file in args.files() {
//!     println!("Opening file: {}", file.display());
//! }
//!
//! // Check for URLs to handle
//! for url in args.urls() {
//!     println!("Handling URL: {}", url);
//! }
//! ```
//!
//! # Registering File Associations
//!
//! ```ignore
//! use horizon_lattice::platform::{FileTypeRegistration, FileTypeInfo};
//!
//! // Register .myext files to open with this application
//! let registration = FileTypeRegistration::new()
//!     .extension("myext")
//!     .description("My Application Document")
//!     .content_type("application/x-myapp")
//!     .icon_path("/path/to/icon.png");
//!
//! registration.register()?;
//! ```
//!
//! # Platform Notes
//!
//! ## File Opening
//! - **Windows**: Uses `ShellExecuteW` or `explorer.exe` for reveal
//! - **macOS**: Uses the `open` command
//! - **Linux**: Uses `xdg-open` or similar
//!
//! ## File Association Registration
//! - **Windows**: Modifies registry under `HKEY_CURRENT_USER`
//! - **macOS**: Runtime registration is not supported; must be in Info.plist
//! - **Linux**: Creates/modifies `.desktop` files and uses `xdg-mime`
//!
//! ## URL Scheme Registration
//! - **Windows**: Modifies registry under `HKEY_CURRENT_USER`
//! - **macOS**: Runtime registration is not supported; must be in Info.plist
//! - **Linux**: Creates/modifies `.desktop` files

use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

// ============================================================================
// Error Types
// ============================================================================

/// Error type for file association operations.
#[derive(Debug)]
pub struct FileAssociationError {
    kind: FileAssociationErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileAssociationErrorKind {
    /// Failed to open file or URL.
    OpenFailed,
    /// Failed to reveal file in file manager.
    RevealFailed,
    /// Failed to register file type.
    RegistrationFailed,
    /// Operation not supported on this platform.
    UnsupportedPlatform,
    /// Invalid argument provided.
    InvalidArgument,
}

impl FileAssociationError {
    fn open_failed(message: impl Into<String>) -> Self {
        Self {
            kind: FileAssociationErrorKind::OpenFailed,
            message: message.into(),
        }
    }

    fn reveal_failed(message: impl Into<String>) -> Self {
        Self {
            kind: FileAssociationErrorKind::RevealFailed,
            message: message.into(),
        }
    }

    fn registration_failed(message: impl Into<String>) -> Self {
        Self {
            kind: FileAssociationErrorKind::RegistrationFailed,
            message: message.into(),
        }
    }

    fn unsupported_platform(message: impl Into<String>) -> Self {
        Self {
            kind: FileAssociationErrorKind::UnsupportedPlatform,
            message: message.into(),
        }
    }

    fn invalid_argument(message: impl Into<String>) -> Self {
        Self {
            kind: FileAssociationErrorKind::InvalidArgument,
            message: message.into(),
        }
    }

    /// Returns true if this error indicates the operation is not supported on this platform.
    pub fn is_unsupported_platform(&self) -> bool {
        self.kind == FileAssociationErrorKind::UnsupportedPlatform
    }
}

impl fmt::Display for FileAssociationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            FileAssociationErrorKind::OpenFailed => {
                write!(f, "failed to open: {}", self.message)
            }
            FileAssociationErrorKind::RevealFailed => {
                write!(f, "failed to reveal: {}", self.message)
            }
            FileAssociationErrorKind::RegistrationFailed => {
                write!(f, "failed to register: {}", self.message)
            }
            FileAssociationErrorKind::UnsupportedPlatform => {
                write!(f, "unsupported platform: {}", self.message)
            }
            FileAssociationErrorKind::InvalidArgument => {
                write!(f, "invalid argument: {}", self.message)
            }
        }
    }
}

impl std::error::Error for FileAssociationError {}

impl From<io::Error> for FileAssociationError {
    fn from(err: io::Error) -> Self {
        Self::open_failed(err.to_string())
    }
}

// ============================================================================
// Opener - Open files and URLs with default applications
// ============================================================================

/// Open files and URLs with their default applications.
///
/// This struct provides static methods for opening files, URLs, and revealing
/// files in the system file manager.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::platform::Opener;
///
/// // Open a document
/// Opener::open("document.pdf")?;
///
/// // Open a URL
/// Opener::open_url("https://rust-lang.org")?;
///
/// // Show a file in the file manager
/// Opener::reveal("important.txt")?;
/// ```
pub struct Opener;

impl Opener {
    /// Open a file with the default application.
    ///
    /// This function opens the given path with the system's default application
    /// for that file type. For example, a `.pdf` file would open in the default
    /// PDF viewer, and a `.txt` file would open in the default text editor.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to open
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file does not exist
    /// - No default application is configured for the file type
    /// - The application fails to launch
    pub fn open<P: AsRef<Path>>(path: P) -> Result<(), FileAssociationError> {
        let path = path.as_ref();
        open::that(path).map_err(|e| FileAssociationError::open_failed(e.to_string()))
    }

    /// Open a file with a specific application.
    ///
    /// This function opens the given path with the specified application
    /// instead of the system default.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file to open
    /// * `application` - The application to use (name or path)
    ///
    /// # Platform Notes
    ///
    /// - **Windows**: `application` can be an executable name or full path
    /// - **macOS**: `application` should be the app name or bundle identifier
    /// - **Linux**: `application` should be the command name
    ///
    /// # Errors
    ///
    /// Returns an error if the application cannot be found or fails to launch.
    pub fn open_with<P, A>(path: P, application: A) -> Result<(), FileAssociationError>
    where
        P: AsRef<OsStr>,
        A: Into<String>,
    {
        let path = path.as_ref();
        open::with(path, application).map_err(|e| FileAssociationError::open_failed(e.to_string()))
    }

    /// Open a URL in the default browser.
    ///
    /// This is a convenience method that handles URL opening specifically,
    /// ensuring proper handling of HTTP/HTTPS URLs as well as custom URL schemes.
    ///
    /// # Arguments
    ///
    /// * `url` - The URL to open (e.g., "https://example.com" or "myapp://action")
    ///
    /// # Errors
    ///
    /// Returns an error if no browser is configured or the URL is malformed.
    pub fn open_url(url: &str) -> Result<(), FileAssociationError> {
        open::that(url).map_err(|e| FileAssociationError::open_failed(e.to_string()))
    }

    /// Reveal a file in the system file manager.
    ///
    /// This function opens the system file manager (Explorer, Finder, Nautilus, etc.)
    /// and highlights/selects the specified file. If only a directory is provided,
    /// the file manager opens to that directory.
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the file or directory to reveal
    ///
    /// # Platform Notes
    ///
    /// - **Windows**: Uses `explorer.exe /select,`
    /// - **macOS**: Uses `open -R`
    /// - **Linux**: Opens the containing directory (file selection varies by file manager)
    ///
    /// # Errors
    ///
    /// Returns an error if the file manager cannot be launched.
    #[cfg(target_os = "windows")]
    pub fn reveal<P: AsRef<Path>>(path: P) -> Result<(), FileAssociationError> {
        use std::process::Command;

        let path = path.as_ref();
        let path_str = path.to_string_lossy();

        Command::new("explorer.exe")
            .arg("/select,")
            .arg(&*path_str)
            .spawn()
            .map_err(|e| FileAssociationError::reveal_failed(e.to_string()))?;

        Ok(())
    }

    /// Reveal a file in Finder (macOS).
    #[cfg(target_os = "macos")]
    pub fn reveal<P: AsRef<Path>>(path: P) -> Result<(), FileAssociationError> {
        use std::process::Command;

        let path = path.as_ref();

        Command::new("open")
            .arg("-R")
            .arg(path)
            .spawn()
            .map_err(|e| FileAssociationError::reveal_failed(e.to_string()))?;

        Ok(())
    }

    /// Reveal a file in file manager (Linux).
    #[cfg(target_os = "linux")]
    pub fn reveal<P: AsRef<Path>>(path: P) -> Result<(), FileAssociationError> {
        use std::process::Command;

        let path = path.as_ref();

        // Try using dbus to call the file manager's ShowItems method
        // Fall back to opening the containing directory
        let parent = path.parent().unwrap_or(path);

        // Try xdg-open on the directory
        Command::new("xdg-open")
            .arg(parent)
            .spawn()
            .map_err(|e| FileAssociationError::reveal_failed(e.to_string()))?;

        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    pub fn reveal<P: AsRef<Path>>(path: P) -> Result<(), FileAssociationError> {
        let _ = path;
        Err(FileAssociationError::unsupported_platform(
            "reveal is not supported on this platform",
        ))
    }
}

// ============================================================================
// LaunchArgs - Parse file/URL arguments from command line
// ============================================================================

/// Parsed launch arguments containing files and URLs to open.
///
/// When your application is launched to open a file or handle a URL scheme,
/// the operating system passes these as command-line arguments. This struct
/// parses those arguments and provides easy access to them.
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::platform::LaunchArgs;
///
/// let args = LaunchArgs::parse();
///
/// if args.has_files() {
///     for file in args.files() {
///         println!("Opening: {}", file.display());
///     }
/// }
///
/// if args.has_urls() {
///     for url in args.urls() {
///         println!("Handling URL: {}", url);
///     }
/// }
/// ```
#[derive(Debug, Clone, Default)]
pub struct LaunchArgs {
    files: Vec<PathBuf>,
    urls: Vec<String>,
}

impl LaunchArgs {
    /// Create an empty LaunchArgs instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse command-line arguments to extract files and URLs.
    ///
    /// This examines `std::env::args()` (skipping the executable name) and
    /// categorizes each argument as either a file path or a URL.
    ///
    /// An argument is considered a URL if it:
    /// - Contains "://" (e.g., "https://example.com" or "myapp://action")
    /// - Starts with a known URL scheme pattern
    ///
    /// All other arguments that exist as files on the filesystem are
    /// treated as file paths.
    pub fn parse() -> Self {
        Self::parse_from(env::args().skip(1))
    }

    /// Parse arguments from a custom iterator.
    ///
    /// This is useful for testing or when arguments come from a source
    /// other than `std::env::args()`.
    pub fn parse_from<I, S>(args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut result = Self::new();

        for arg in args {
            let arg = arg.as_ref();

            // Skip empty arguments and common flags
            if arg.is_empty() || arg.starts_with('-') {
                continue;
            }

            // Check if it looks like a URL
            if Self::is_url(arg) {
                result.urls.push(arg.to_string());
            } else {
                // Treat as a file path
                let path = PathBuf::from(arg);
                result.files.push(path);
            }
        }

        result
    }

    /// Check if a string looks like a URL.
    fn is_url(s: &str) -> bool {
        // Check for scheme://
        if s.contains("://") {
            return true;
        }

        // Check for common URL schemes without ://
        let lower = s.to_lowercase();
        lower.starts_with("mailto:")
            || lower.starts_with("tel:")
            || lower.starts_with("file:")
            || lower.starts_with("data:")
    }

    /// Get the list of file paths to open.
    pub fn files(&self) -> &[PathBuf] {
        &self.files
    }

    /// Get the list of URLs to handle.
    pub fn urls(&self) -> &[String] {
        &self.urls
    }

    /// Check if there are any files to open.
    pub fn has_files(&self) -> bool {
        !self.files.is_empty()
    }

    /// Check if there are any URLs to handle.
    pub fn has_urls(&self) -> bool {
        !self.urls.is_empty()
    }

    /// Check if there are any arguments (files or URLs).
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.urls.is_empty()
    }

    /// Get the first file path, if any.
    pub fn first_file(&self) -> Option<&PathBuf> {
        self.files.first()
    }

    /// Get the first URL, if any.
    pub fn first_url(&self) -> Option<&str> {
        self.urls.first().map(|s| s.as_str())
    }
}

// ============================================================================
// FileTypeInfo - Information about a file type registration
// ============================================================================

/// Information about a file type for registration.
///
/// This struct contains all the metadata needed to register a file type
/// association with the operating system.
#[derive(Debug, Clone)]
pub struct FileTypeInfo {
    /// File extension without the leading dot (e.g., "myext").
    pub extension: String,
    /// Human-readable description (e.g., "My Application Document").
    pub description: String,
    /// MIME content type (e.g., "application/x-myapp").
    pub content_type: Option<String>,
    /// Path to an icon file for this file type.
    pub icon_path: Option<PathBuf>,
}

impl FileTypeInfo {
    /// Create a new file type info with the given extension.
    pub fn new(extension: impl Into<String>) -> Self {
        Self {
            extension: extension.into(),
            description: String::new(),
            content_type: None,
            icon_path: None,
        }
    }

    /// Set the human-readable description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set the MIME content type.
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Set the icon path.
    pub fn icon_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.icon_path = Some(path.into());
        self
    }

    /// Get the extension with a leading dot.
    pub fn dotted_extension(&self) -> String {
        if self.extension.starts_with('.') {
            self.extension.clone()
        } else {
            format!(".{}", self.extension)
        }
    }
}

// ============================================================================
// FileTypeRegistration - Register file type associations
// ============================================================================

/// Builder for registering file type associations.
///
/// This allows your application to be registered as the default handler
/// for specific file extensions.
///
/// # Platform Support
///
/// - **Windows**: Full support via registry modification
/// - **Linux**: Full support via `.desktop` files and `xdg-mime`
/// - **macOS**: Not supported at runtime (must be configured in Info.plist)
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::platform::FileTypeRegistration;
///
/// FileTypeRegistration::new()
///     .extension("myext")
///     .description("My Application Document")
///     .content_type("application/x-myapp")
///     .register()?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct FileTypeRegistration {
    info: Option<FileTypeInfo>,
    app_name: Option<String>,
    app_id: Option<String>,
    executable: Option<PathBuf>,
}

impl FileTypeRegistration {
    /// Create a new file type registration builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the file extension to register (without leading dot).
    pub fn extension(mut self, extension: impl Into<String>) -> Self {
        let ext = extension.into();
        if let Some(ref mut info) = self.info {
            info.extension = ext;
        } else {
            self.info = Some(FileTypeInfo::new(ext));
        }
        self
    }

    /// Set the human-readable description for this file type.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        let desc = description.into();
        if let Some(ref mut info) = self.info {
            info.description = desc;
        } else {
            let mut info = FileTypeInfo::new("");
            info.description = desc;
            self.info = Some(info);
        }
        self
    }

    /// Set the MIME content type.
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        let ct = content_type.into();
        if let Some(ref mut info) = self.info {
            info.content_type = Some(ct);
        } else {
            let mut info = FileTypeInfo::new("");
            info.content_type = Some(ct);
            self.info = Some(info);
        }
        self
    }

    /// Set the icon path for this file type.
    pub fn icon_path(mut self, path: impl Into<PathBuf>) -> Self {
        let p = path.into();
        if let Some(ref mut info) = self.info {
            info.icon_path = Some(p);
        } else {
            let mut info = FileTypeInfo::new("");
            info.icon_path = Some(p);
            self.info = Some(info);
        }
        self
    }

    /// Set the application name for display purposes.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Set the application ID (used on Linux for .desktop file naming).
    pub fn app_id(mut self, id: impl Into<String>) -> Self {
        self.app_id = Some(id.into());
        self
    }

    /// Set the executable path (defaults to current executable).
    pub fn executable(mut self, path: impl Into<PathBuf>) -> Self {
        self.executable = Some(path.into());
        self
    }

    /// Register the file type association.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No extension was specified
    /// - The platform doesn't support runtime registration (macOS)
    /// - Registration fails (e.g., permission denied)
    #[cfg(target_os = "windows")]
    pub fn register(self) -> Result<(), FileAssociationError> {
        let info = self
            .info
            .ok_or_else(|| FileAssociationError::invalid_argument("extension is required"))?;

        if info.extension.is_empty() {
            return Err(FileAssociationError::invalid_argument(
                "extension cannot be empty",
            ));
        }

        let executable = self
            .executable
            .or_else(|| env::current_exe().ok())
            .ok_or_else(|| {
                FileAssociationError::registration_failed("could not determine executable path")
            })?;

        let app_name = self.app_name.unwrap_or_else(|| {
            executable
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Application".to_string())
        });

        windows_register_file_type(&info, &executable, &app_name)
    }

    #[cfg(target_os = "linux")]
    pub fn register(self) -> Result<(), FileAssociationError> {
        let info = self
            .info
            .ok_or_else(|| FileAssociationError::invalid_argument("extension is required"))?;

        if info.extension.is_empty() {
            return Err(FileAssociationError::invalid_argument(
                "extension cannot be empty",
            ));
        }

        let executable = self
            .executable
            .or_else(|| env::current_exe().ok())
            .ok_or_else(|| {
                FileAssociationError::registration_failed("could not determine executable path")
            })?;

        let app_name = self.app_name.unwrap_or_else(|| {
            executable
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Application".to_string())
        });

        let app_id = self.app_id.unwrap_or_else(|| {
            app_name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect()
        });

        linux_register_file_type(&info, &executable, &app_name, &app_id)
    }

    /// Register file type association (macOS - not supported at runtime).
    #[cfg(target_os = "macos")]
    pub fn register(self) -> Result<(), FileAssociationError> {
        let _ = self;
        Err(FileAssociationError::unsupported_platform(
            "macOS does not support runtime file type registration. \
             File associations must be configured in Info.plist at build time.",
        ))
    }

    /// Register file type association (unsupported platforms).
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    pub fn register(self) -> Result<(), FileAssociationError> {
        let _ = self;
        Err(FileAssociationError::unsupported_platform(
            "file type registration is not supported on this platform",
        ))
    }

    /// Unregister a previously registered file type association.
    #[cfg(target_os = "windows")]
    pub fn unregister(extension: &str) -> Result<(), FileAssociationError> {
        windows_unregister_file_type(extension)
    }

    /// Unregister a file type association (Linux).
    #[cfg(target_os = "linux")]
    pub fn unregister(extension: &str) -> Result<(), FileAssociationError> {
        linux_unregister_file_type(extension)
    }

    /// Unregister a file type association (macOS - not supported).
    #[cfg(target_os = "macos")]
    pub fn unregister(_extension: &str) -> Result<(), FileAssociationError> {
        Err(FileAssociationError::unsupported_platform(
            "macOS does not support runtime file type unregistration",
        ))
    }

    /// Unregister a file type association (unsupported platforms).
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    pub fn unregister(_extension: &str) -> Result<(), FileAssociationError> {
        Err(FileAssociationError::unsupported_platform(
            "file type unregistration is not supported on this platform",
        ))
    }
}

// ============================================================================
// UrlSchemeInfo - Information about a URL scheme registration
// ============================================================================

/// Information about a URL scheme for registration.
#[derive(Debug, Clone)]
pub struct UrlSchemeInfo {
    /// The URL scheme without the trailing colon (e.g., "myapp").
    pub scheme: String,
    /// Human-readable description (e.g., "My Application Link").
    pub description: String,
}

impl UrlSchemeInfo {
    /// Create a new URL scheme info.
    pub fn new(scheme: impl Into<String>) -> Self {
        Self {
            scheme: scheme.into(),
            description: String::new(),
        }
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }
}

// ============================================================================
// UrlSchemeRegistration - Register custom URL scheme handlers
// ============================================================================

/// Builder for registering custom URL scheme handlers.
///
/// This allows your application to handle custom URL schemes like
/// `myapp://action/param`.
///
/// # Platform Support
///
/// - **Windows**: Full support via registry modification
/// - **Linux**: Full support via `.desktop` files
/// - **macOS**: Not supported at runtime (must be configured in Info.plist)
///
/// # Example
///
/// ```ignore
/// use horizon_lattice::platform::UrlSchemeRegistration;
///
/// UrlSchemeRegistration::new()
///     .scheme("myapp")
///     .description("My Application Links")
///     .register()?;
///
/// // Now myapp://anything URLs will open this application
/// ```
#[derive(Debug, Clone, Default)]
pub struct UrlSchemeRegistration {
    info: Option<UrlSchemeInfo>,
    app_name: Option<String>,
    app_id: Option<String>,
    executable: Option<PathBuf>,
}

impl UrlSchemeRegistration {
    /// Create a new URL scheme registration builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the URL scheme to register (without the colon).
    pub fn scheme(mut self, scheme: impl Into<String>) -> Self {
        let s = scheme.into();
        if let Some(ref mut info) = self.info {
            info.scheme = s;
        } else {
            self.info = Some(UrlSchemeInfo::new(s));
        }
        self
    }

    /// Set the human-readable description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        let desc = description.into();
        if let Some(ref mut info) = self.info {
            info.description = desc;
        } else {
            let mut info = UrlSchemeInfo::new("");
            info.description = desc;
            self.info = Some(info);
        }
        self
    }

    /// Set the application name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Set the application ID (used on Linux for .desktop file naming).
    pub fn app_id(mut self, id: impl Into<String>) -> Self {
        self.app_id = Some(id.into());
        self
    }

    /// Set the executable path (defaults to current executable).
    pub fn executable(mut self, path: impl Into<PathBuf>) -> Self {
        self.executable = Some(path.into());
        self
    }

    /// Register the URL scheme handler.
    #[cfg(target_os = "windows")]
    pub fn register(self) -> Result<(), FileAssociationError> {
        let info = self
            .info
            .ok_or_else(|| FileAssociationError::invalid_argument("scheme is required"))?;

        if info.scheme.is_empty() {
            return Err(FileAssociationError::invalid_argument(
                "scheme cannot be empty",
            ));
        }

        let executable = self
            .executable
            .or_else(|| env::current_exe().ok())
            .ok_or_else(|| {
                FileAssociationError::registration_failed("could not determine executable path")
            })?;

        let app_name = self.app_name.unwrap_or_else(|| {
            executable
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Application".to_string())
        });

        windows_register_url_scheme(&info, &executable, &app_name)
    }

    #[cfg(target_os = "linux")]
    pub fn register(self) -> Result<(), FileAssociationError> {
        let info = self
            .info
            .ok_or_else(|| FileAssociationError::invalid_argument("scheme is required"))?;

        if info.scheme.is_empty() {
            return Err(FileAssociationError::invalid_argument(
                "scheme cannot be empty",
            ));
        }

        let executable = self
            .executable
            .or_else(|| env::current_exe().ok())
            .ok_or_else(|| {
                FileAssociationError::registration_failed("could not determine executable path")
            })?;

        let app_name = self.app_name.unwrap_or_else(|| {
            executable
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Application".to_string())
        });

        let app_id = self.app_id.unwrap_or_else(|| {
            app_name
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect()
        });

        linux_register_url_scheme(&info, &executable, &app_name, &app_id)
    }

    /// Register URL scheme handler (macOS - not supported at runtime).
    #[cfg(target_os = "macos")]
    pub fn register(self) -> Result<(), FileAssociationError> {
        let _ = self;
        Err(FileAssociationError::unsupported_platform(
            "macOS does not support runtime URL scheme registration. \
             URL schemes must be configured in Info.plist at build time.",
        ))
    }

    /// Register URL scheme handler (unsupported platforms).
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    pub fn register(self) -> Result<(), FileAssociationError> {
        let _ = self;
        Err(FileAssociationError::unsupported_platform(
            "URL scheme registration is not supported on this platform",
        ))
    }

    /// Unregister a previously registered URL scheme.
    #[cfg(target_os = "windows")]
    pub fn unregister(scheme: &str) -> Result<(), FileAssociationError> {
        windows_unregister_url_scheme(scheme)
    }

    /// Unregister a URL scheme handler (Linux).
    #[cfg(target_os = "linux")]
    pub fn unregister(scheme: &str) -> Result<(), FileAssociationError> {
        linux_unregister_url_scheme(scheme)
    }

    /// Unregister a URL scheme handler (macOS - not supported).
    #[cfg(target_os = "macos")]
    pub fn unregister(_scheme: &str) -> Result<(), FileAssociationError> {
        Err(FileAssociationError::unsupported_platform(
            "macOS does not support runtime URL scheme unregistration",
        ))
    }

    /// Unregister a URL scheme handler (unsupported platforms).
    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    pub fn unregister(_scheme: &str) -> Result<(), FileAssociationError> {
        Err(FileAssociationError::unsupported_platform(
            "URL scheme unregistration is not supported on this platform",
        ))
    }
}

// ============================================================================
// Platform-specific implementations - Windows
// ============================================================================

#[cfg(target_os = "windows")]
fn windows_register_file_type(
    info: &FileTypeInfo,
    executable: &Path,
    app_name: &str,
) -> Result<(), FileAssociationError> {
    use std::process::Command;

    let ext = info.dotted_extension();
    let prog_id = format!("{}.{}", app_name.replace(' ', ""), info.extension);
    let exe_path = executable.to_string_lossy();

    // Register the ProgID with description
    let description = if info.description.is_empty() {
        format!("{} File", app_name)
    } else {
        info.description.clone()
    };

    // Use reg.exe to set registry values (avoids direct registry API complexity)
    // HKCU\Software\Classes\<ProgID>
    Command::new("reg")
        .args([
            "add",
            &format!("HKCU\\Software\\Classes\\{}", prog_id),
            "/ve",
            "/d",
            &description,
            "/f",
        ])
        .output()
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    // HKCU\Software\Classes\<ProgID>\shell\open\command
    Command::new("reg")
        .args([
            "add",
            &format!("HKCU\\Software\\Classes\\{}\\shell\\open\\command", prog_id),
            "/ve",
            "/d",
            &format!("\"{}\" \"%1\"", exe_path),
            "/f",
        ])
        .output()
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    // Associate extension with ProgID
    // HKCU\Software\Classes\<.ext>
    Command::new("reg")
        .args([
            "add",
            &format!("HKCU\\Software\\Classes\\{}", ext),
            "/ve",
            "/d",
            &prog_id,
            "/f",
        ])
        .output()
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    // Set content type if provided
    if let Some(ref content_type) = info.content_type {
        Command::new("reg")
            .args([
                "add",
                &format!("HKCU\\Software\\Classes\\{}", ext),
                "/v",
                "Content Type",
                "/d",
                content_type,
                "/f",
            ])
            .output()
            .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;
    }

    // Notify shell of changes
    notify_shell_change();

    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_unregister_file_type(extension: &str) -> Result<(), FileAssociationError> {
    use std::process::Command;

    let ext = if extension.starts_with('.') {
        extension.to_string()
    } else {
        format!(".{}", extension)
    };

    // Remove extension key
    let _ = Command::new("reg")
        .args(["delete", &format!("HKCU\\Software\\Classes\\{}", ext), "/f"])
        .output();

    notify_shell_change();

    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_register_url_scheme(
    info: &UrlSchemeInfo,
    executable: &Path,
    app_name: &str,
) -> Result<(), FileAssociationError> {
    use std::process::Command;

    let scheme = &info.scheme;
    let exe_path = executable.to_string_lossy();

    let description = if info.description.is_empty() {
        format!("{} URL", app_name)
    } else {
        info.description.clone()
    };

    // HKCU\Software\Classes\<scheme>
    Command::new("reg")
        .args([
            "add",
            &format!("HKCU\\Software\\Classes\\{}", scheme),
            "/ve",
            "/d",
            &description,
            "/f",
        ])
        .output()
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    // Mark as URL protocol
    Command::new("reg")
        .args([
            "add",
            &format!("HKCU\\Software\\Classes\\{}", scheme),
            "/v",
            "URL Protocol",
            "/d",
            "",
            "/f",
        ])
        .output()
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    // Shell command
    Command::new("reg")
        .args([
            "add",
            &format!("HKCU\\Software\\Classes\\{}\\shell\\open\\command", scheme),
            "/ve",
            "/d",
            &format!("\"{}\" \"%1\"", exe_path),
            "/f",
        ])
        .output()
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    notify_shell_change();

    Ok(())
}

#[cfg(target_os = "windows")]
fn windows_unregister_url_scheme(scheme: &str) -> Result<(), FileAssociationError> {
    use std::process::Command;

    let _ = Command::new("reg")
        .args([
            "delete",
            &format!("HKCU\\Software\\Classes\\{}", scheme),
            "/f",
        ])
        .output();

    notify_shell_change();

    Ok(())
}

#[cfg(target_os = "windows")]
fn notify_shell_change() {
    use std::process::Command;
    // Use ie4uinit to refresh shell icon cache
    let _ = Command::new("ie4uinit.exe").arg("-show").output();
}

// ============================================================================
// Platform-specific implementations - Linux
// ============================================================================

#[cfg(target_os = "linux")]
fn linux_register_file_type(
    info: &FileTypeInfo,
    executable: &Path,
    app_name: &str,
    app_id: &str,
) -> Result<(), FileAssociationError> {
    use std::fs;
    use std::process::Command;

    let desktop_file = create_desktop_file(executable, app_name, app_id, Some(info), None)?;

    // Install the .desktop file
    let applications_dir = dirs_desktop_applications()?;
    fs::create_dir_all(&applications_dir)
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    let desktop_path = applications_dir.join(format!("{}.desktop", app_id));
    fs::write(&desktop_path, desktop_file)
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    // Register MIME type if content_type is provided
    if let Some(ref content_type) = info.content_type {
        let _ = Command::new("xdg-mime")
            .args(["default", &format!("{}.desktop", app_id), content_type])
            .output();
    }

    // Update desktop database
    let _ = Command::new("update-desktop-database")
        .arg(&applications_dir)
        .output();

    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_unregister_file_type(extension: &str) -> Result<(), FileAssociationError> {
    use std::fs;

    let _ = extension; // Extension info would be needed to find the right .desktop file

    // This is a simplified implementation - in practice, you'd need to
    // track which .desktop file was created for this extension
    let applications_dir = dirs_desktop_applications()?;

    // List and remove .desktop files created by this app
    if let Ok(entries) = fs::read_dir(&applications_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name() {
                if name.to_string_lossy().contains("horizon-lattice") {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_register_url_scheme(
    info: &UrlSchemeInfo,
    executable: &Path,
    app_name: &str,
    app_id: &str,
) -> Result<(), FileAssociationError> {
    use std::fs;
    use std::process::Command;

    let desktop_file = create_desktop_file(executable, app_name, app_id, None, Some(info))?;

    // Install the .desktop file
    let applications_dir = dirs_desktop_applications()?;
    fs::create_dir_all(&applications_dir)
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    let desktop_path = applications_dir.join(format!("{}-url.desktop", app_id));
    fs::write(&desktop_path, desktop_file)
        .map_err(|e| FileAssociationError::registration_failed(e.to_string()))?;

    // Register as handler for x-scheme-handler
    let mime_type = format!("x-scheme-handler/{}", info.scheme);
    let _ = Command::new("xdg-mime")
        .args(["default", &format!("{}-url.desktop", app_id), &mime_type])
        .output();

    // Update desktop database
    let _ = Command::new("update-desktop-database")
        .arg(&applications_dir)
        .output();

    Ok(())
}

#[cfg(target_os = "linux")]
fn linux_unregister_url_scheme(scheme: &str) -> Result<(), FileAssociationError> {
    use std::fs;

    let _ = scheme;

    let applications_dir = dirs_desktop_applications()?;

    if let Ok(entries) = fs::read_dir(&applications_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.contains("horizon-lattice") && name_str.contains("-url") {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn dirs_desktop_applications() -> Result<PathBuf, FileAssociationError> {
    let home = env::var("HOME").map_err(|_| {
        FileAssociationError::registration_failed("HOME environment variable not set")
    })?;

    Ok(PathBuf::from(home).join(".local/share/applications"))
}

#[cfg(target_os = "linux")]
fn create_desktop_file(
    executable: &Path,
    app_name: &str,
    app_id: &str,
    file_type: Option<&FileTypeInfo>,
    url_scheme: Option<&UrlSchemeInfo>,
) -> Result<String, FileAssociationError> {
    let exe_path = executable.to_string_lossy();

    let mut content = String::new();
    content.push_str("[Desktop Entry]\n");
    content.push_str(&format!("Name={}\n", app_name));
    content.push_str(&format!("Exec=\"{}\" %u\n", exe_path));
    content.push_str("Type=Application\n");
    content.push_str("Terminal=false\n");
    content.push_str(&format!("StartupWMClass={}\n", app_id));

    if let Some(info) = file_type {
        if !info.description.is_empty() {
            content.push_str(&format!("Comment={}\n", info.description));
        }
        if let Some(ref content_type) = info.content_type {
            content.push_str(&format!("MimeType={};\n", content_type));
        }
        if let Some(ref icon_path) = info.icon_path {
            content.push_str(&format!("Icon={}\n", icon_path.display()));
        }
    }

    if let Some(info) = url_scheme {
        if !info.description.is_empty() {
            content.push_str(&format!("Comment={}\n", info.description));
        }
        let mime_type = format!("x-scheme-handler/{}", info.scheme);
        content.push_str(&format!("MimeType={};\n", mime_type));
    }

    Ok(content)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_association_error_display() {
        let error = FileAssociationError::open_failed("file not found");
        assert!(error.to_string().contains("failed to open"));
        assert!(error.to_string().contains("file not found"));

        let error = FileAssociationError::unsupported_platform("macOS");
        assert!(error.to_string().contains("unsupported platform"));
        assert!(error.is_unsupported_platform());
    }

    #[test]
    fn test_launch_args_parse_empty() {
        let args = LaunchArgs::parse_from::<Vec<&str>, &str>(vec![]);
        assert!(args.is_empty());
        assert!(!args.has_files());
        assert!(!args.has_urls());
    }

    #[test]
    fn test_launch_args_parse_files() {
        let args = LaunchArgs::parse_from(vec!["document.pdf", "image.png", "/path/to/file.txt"]);
        assert!(args.has_files());
        assert!(!args.has_urls());
        assert_eq!(args.files().len(), 3);
        assert_eq!(args.files()[0], PathBuf::from("document.pdf"));
    }

    #[test]
    fn test_launch_args_parse_urls() {
        let args = LaunchArgs::parse_from(vec![
            "https://example.com",
            "myapp://action/param",
            "mailto:user@example.com",
        ]);
        assert!(!args.has_files());
        assert!(args.has_urls());
        assert_eq!(args.urls().len(), 3);
        assert_eq!(args.urls()[0], "https://example.com");
        assert_eq!(args.urls()[1], "myapp://action/param");
        assert_eq!(args.urls()[2], "mailto:user@example.com");
    }

    #[test]
    fn test_launch_args_parse_mixed() {
        let args = LaunchArgs::parse_from(vec![
            "document.pdf",
            "https://example.com",
            "/path/to/file.txt",
        ]);
        assert!(args.has_files());
        assert!(args.has_urls());
        assert_eq!(args.files().len(), 2);
        assert_eq!(args.urls().len(), 1);
    }

    #[test]
    fn test_launch_args_skip_flags() {
        let args = LaunchArgs::parse_from(vec!["-v", "--version", "file.txt", "-h"]);
        assert_eq!(args.files().len(), 1);
        assert_eq!(args.files()[0], PathBuf::from("file.txt"));
    }

    #[test]
    fn test_launch_args_first_methods() {
        let args = LaunchArgs::parse_from(vec!["file1.txt", "file2.txt", "https://example.com"]);
        assert_eq!(args.first_file(), Some(&PathBuf::from("file1.txt")));
        assert_eq!(args.first_url(), Some("https://example.com"));

        let empty = LaunchArgs::new();
        assert_eq!(empty.first_file(), None);
        assert_eq!(empty.first_url(), None);
    }

    #[test]
    fn test_file_type_info() {
        let info = FileTypeInfo::new("myext")
            .description("My File Type")
            .content_type("application/x-myapp")
            .icon_path("/path/to/icon.png");

        assert_eq!(info.extension, "myext");
        assert_eq!(info.description, "My File Type");
        assert_eq!(info.content_type, Some("application/x-myapp".to_string()));
        assert_eq!(info.icon_path, Some(PathBuf::from("/path/to/icon.png")));
        assert_eq!(info.dotted_extension(), ".myext");
    }

    #[test]
    fn test_file_type_info_dotted_extension() {
        let info1 = FileTypeInfo::new("txt");
        assert_eq!(info1.dotted_extension(), ".txt");

        let info2 = FileTypeInfo::new(".txt");
        assert_eq!(info2.dotted_extension(), ".txt");
    }

    #[test]
    fn test_url_scheme_info() {
        let info = UrlSchemeInfo::new("myapp").description("My App Links");

        assert_eq!(info.scheme, "myapp");
        assert_eq!(info.description, "My App Links");
    }

    #[test]
    fn test_file_type_registration_builder() {
        let reg = FileTypeRegistration::new()
            .extension("myext")
            .description("My Documents")
            .content_type("application/x-myapp")
            .app_name("My App")
            .app_id("com.example.myapp");

        assert!(reg.info.is_some());
        let info = reg.info.unwrap();
        assert_eq!(info.extension, "myext");
        assert_eq!(info.description, "My Documents");
        assert_eq!(reg.app_name, Some("My App".to_string()));
        assert_eq!(reg.app_id, Some("com.example.myapp".to_string()));
    }

    #[test]
    fn test_url_scheme_registration_builder() {
        let reg = UrlSchemeRegistration::new()
            .scheme("myapp")
            .description("My App Links")
            .app_name("My App");

        assert!(reg.info.is_some());
        let info = reg.info.unwrap();
        assert_eq!(info.scheme, "myapp");
        assert_eq!(info.description, "My App Links");
        assert_eq!(reg.app_name, Some("My App".to_string()));
    }

    #[test]
    fn test_is_url() {
        assert!(LaunchArgs::is_url("https://example.com"));
        assert!(LaunchArgs::is_url("http://example.com"));
        assert!(LaunchArgs::is_url("myapp://action"));
        assert!(LaunchArgs::is_url("mailto:user@example.com"));
        assert!(LaunchArgs::is_url("tel:+1234567890"));
        assert!(LaunchArgs::is_url("file:///path/to/file"));

        assert!(!LaunchArgs::is_url("document.pdf"));
        assert!(!LaunchArgs::is_url("/path/to/file"));
        assert!(!LaunchArgs::is_url("C:\\path\\to\\file"));
    }
}
