//! Session management services.
//!
//! This module provides cross-platform session management functionality including
//! session event detection, shutdown/logout inhibition, and state persistence.
//!
//! # Session Events
//!
//! ```ignore
//! use horizon_lattice::platform::SessionEventWatcher;
//!
//! let watcher = SessionEventWatcher::new()?;
//!
//! watcher.session_ending().connect(|reason| {
//!     println!("Session ending: {:?}", reason);
//!     // Save state, close connections, etc.
//! });
//!
//! watcher.start()?;
//! ```
//!
//! # Session Inhibition
//!
//! ```ignore
//! use horizon_lattice::platform::SessionInhibitor;
//!
//! // Prevent shutdown while saving important work
//! let _guard = SessionInhibitor::new()
//!     .reason("Saving document...")
//!     .inhibit_shutdown(true)
//!     .start()?;
//!
//! save_important_work();
//! // Guard drops here, shutdown proceeds normally
//! ```
//!
//! # State Persistence
//!
//! ```ignore
//! use horizon_lattice::platform::{StateLocation, ApplicationState};
//!
//! // Get platform-appropriate state directories
//! let location = StateLocation::new("com.example.myapp", "MyApp")?;
//! println!("State dir: {:?}", location.state_dir());
//! println!("Config dir: {:?}", location.config_dir());
//!
//! // Use ApplicationState for simple key-value state
//! let state = ApplicationState::new("com.example.myapp", "MyApp")?;
//! state.save("window_geometry", b"100,100,800,600")?;
//! let geometry = state.load("window_geometry")?;
//! ```
//!
//! # Platform Notes
//!
//! ## Session Events
//! - **Windows**: Uses `WM_QUERYENDSESSION` and `WM_ENDSESSION` messages
//! - **macOS**: Uses `NSWorkspace.willPowerOffNotification` (cannot distinguish logout vs shutdown)
//! - **Linux**: Uses `systemd-logind` D-Bus `PrepareForShutdown` signal
//!
//! ## Session Inhibition
//! - **Windows**: Uses `ShutdownBlockReasonCreate/Destroy` (shows reason in shutdown UI)
//! - **macOS**: Limited - only prevents idle sleep, cannot block user-initiated shutdown
//! - **Linux**: Uses `systemd-logind` D-Bus `Inhibit()` method with delay lock
//!
//! ## State Persistence
//! - **Windows**: Uses `%APPDATA%\<app>` for config, `%LOCALAPPDATA%\<app>` for state
//! - **macOS**: Uses `~/Library/Application Support/<app>`
//! - **Linux**: Uses XDG directories (`~/.config/<app>`, `~/.local/state/<app>`)

use std::fmt;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use horizon_lattice_core::Signal;

// ============================================================================
// Error Types
// ============================================================================

/// Error type for session management operations.
#[derive(Debug)]
pub struct SessionManagementError {
    kind: SessionManagementErrorKind,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum SessionManagementErrorKind {
    /// Failed to watch session events.
    SessionEvents,
    /// Failed to inhibit shutdown/logout.
    Inhibit,
    /// Failed to access state storage.
    StateStorage,
    /// Operation not supported on this platform.
    UnsupportedPlatform,
    /// I/O or system error.
    Io,
}

impl SessionManagementError {
    #[allow(dead_code)]
    fn session_events(message: impl Into<String>) -> Self {
        Self {
            kind: SessionManagementErrorKind::SessionEvents,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    fn inhibit(message: impl Into<String>) -> Self {
        Self {
            kind: SessionManagementErrorKind::Inhibit,
            message: message.into(),
        }
    }

    fn state_storage(message: impl Into<String>) -> Self {
        Self {
            kind: SessionManagementErrorKind::StateStorage,
            message: message.into(),
        }
    }

    fn unsupported_platform(message: impl Into<String>) -> Self {
        Self {
            kind: SessionManagementErrorKind::UnsupportedPlatform,
            message: message.into(),
        }
    }

    fn io(message: impl Into<String>) -> Self {
        Self {
            kind: SessionManagementErrorKind::Io,
            message: message.into(),
        }
    }

    /// Returns true if this error indicates the operation is not supported.
    pub fn is_unsupported_platform(&self) -> bool {
        self.kind == SessionManagementErrorKind::UnsupportedPlatform
    }
}

impl fmt::Display for SessionManagementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            SessionManagementErrorKind::SessionEvents => {
                write!(f, "session events error: {}", self.message)
            }
            SessionManagementErrorKind::Inhibit => {
                write!(f, "inhibit error: {}", self.message)
            }
            SessionManagementErrorKind::StateStorage => {
                write!(f, "state storage error: {}", self.message)
            }
            SessionManagementErrorKind::UnsupportedPlatform => {
                write!(f, "unsupported platform: {}", self.message)
            }
            SessionManagementErrorKind::Io => {
                write!(f, "I/O error: {}", self.message)
            }
        }
    }
}

impl std::error::Error for SessionManagementError {}

impl From<io::Error> for SessionManagementError {
    fn from(e: io::Error) -> Self {
        Self::io(e.to_string())
    }
}

// ============================================================================
// Session End Reason
// ============================================================================

/// The reason why a session is ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SessionEndReason {
    /// System is shutting down.
    Shutdown,
    /// System is restarting.
    Restart,
    /// User is logging out.
    Logout,
    /// Session is ending for unknown/unspecified reason.
    /// Used when the platform cannot distinguish between shutdown types.
    #[default]
    Unknown,
}

// ============================================================================
// Session Event Watcher
// ============================================================================

/// Internal state for the session event watcher.
struct SessionEventWatcherInner {
    session_ending: Signal<SessionEndReason>,
    running: AtomicBool,
}

/// Watches for session-related system events.
///
/// This allows applications to be notified when the user is logging out or
/// the system is shutting down, enabling them to save state and clean up.
pub struct SessionEventWatcher {
    inner: Arc<SessionEventWatcherInner>,
}

impl SessionEventWatcher {
    /// Create a new session event watcher.
    pub fn new() -> Result<Self, SessionManagementError> {
        Ok(Self {
            inner: Arc::new(SessionEventWatcherInner {
                session_ending: Signal::new(),
                running: AtomicBool::new(false),
            }),
        })
    }

    /// Signal emitted when the session is about to end.
    ///
    /// Connect to this signal to save state, close connections, or perform
    /// cleanup before the session ends. The signal provides the reason for
    /// the session ending (shutdown, restart, logout, or unknown).
    ///
    /// Note: On macOS, the reason is always `Unknown` as the platform doesn't
    /// distinguish between shutdown and logout.
    pub fn session_ending(&self) -> &Signal<SessionEndReason> {
        &self.inner.session_ending
    }

    /// Start watching for session events.
    ///
    /// This spawns a background thread or registers event handlers to monitor
    /// for session state changes. Events will be delivered to connected signals.
    #[cfg(target_os = "linux")]
    pub fn start(&self) -> Result<(), SessionManagementError> {
        if self.inner.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        let inner = Arc::clone(&self.inner);

        std::thread::spawn(move || {
            let result = pollster::block_on(async { linux_session_event_loop(&inner).await });

            if let Err(e) = result {
                eprintln!("Session event watcher error: {}", e);
            }
        });

        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub fn start(&self) -> Result<(), SessionManagementError> {
        if self.inner.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        let inner = Arc::clone(&self.inner);

        std::thread::spawn(move || {
            if let Err(e) = windows_session_event_loop(&inner) {
                eprintln!("Session event watcher error: {}", e);
            }
        });

        Ok(())
    }

    /// Start listening for session events (macOS - limited support).
    #[cfg(target_os = "macos")]
    pub fn start(&self) -> Result<(), SessionManagementError> {
        // macOS session events require NSWorkspace notification observers which
        // need MainThreadMarker and complex Objective-C runtime integration.
        // The willPowerOffNotification has limited reliability and cannot
        // distinguish between logout and shutdown.
        // State persistence still works normally.
        Err(SessionManagementError::unsupported_platform(
            "session events require complex Objective-C observer setup; \
             use ApplicationState for state persistence instead",
        ))
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    pub fn start(&self) -> Result<(), SessionManagementError> {
        Err(SessionManagementError::unsupported_platform(
            "session events not supported on this platform",
        ))
    }

    /// Stop watching for session events.
    pub fn stop(&self) {
        self.inner.running.store(false, Ordering::SeqCst);
    }

    /// Check if the watcher is currently running.
    pub fn is_running(&self) -> bool {
        self.inner.running.load(Ordering::SeqCst)
    }
}

impl Default for SessionEventWatcher {
    fn default() -> Self {
        Self::new().expect("Failed to create SessionEventWatcher")
    }
}

// ============================================================================
// Session Inhibitor
// ============================================================================

/// Options for what types of session actions to inhibit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SessionInhibitOptions {
    /// Prevent shutdown/power off.
    pub shutdown: bool,
    /// Prevent logout.
    pub logout: bool,
}

/// Builder for creating a session inhibitor.
///
/// Use this to prevent the system from shutting down or logging out while
/// performing critical operations like saving important data.
#[derive(Debug, Clone)]
pub struct SessionInhibitorBuilder {
    reason: String,
    app_name: String,
    options: SessionInhibitOptions,
}

impl Default for SessionInhibitorBuilder {
    fn default() -> Self {
        Self {
            reason: "Application requested".to_string(),
            app_name: "horizon-lattice".to_string(),
            options: SessionInhibitOptions {
                shutdown: true,
                logout: false,
            },
        }
    }
}

impl SessionInhibitorBuilder {
    /// Create a new session inhibitor builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the reason for preventing shutdown/logout (shown in system UI).
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = reason.into();
        self
    }

    /// Set the application name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = name.into();
        self
    }

    /// Set whether to inhibit shutdown.
    pub fn inhibit_shutdown(mut self, inhibit: bool) -> Self {
        self.options.shutdown = inhibit;
        self
    }

    /// Set whether to inhibit logout.
    pub fn inhibit_logout(mut self, inhibit: bool) -> Self {
        self.options.logout = inhibit;
        self
    }

    /// Start inhibiting session actions. Returns a guard that releases
    /// the inhibition when dropped.
    pub fn start(self) -> Result<SessionInhibitorGuard, SessionManagementError> {
        SessionInhibitorGuard::new(self.reason, self.app_name, self.options)
    }
}

/// A guard that inhibits session actions while it exists.
///
/// When this guard is dropped, normal session behavior is restored.
/// This uses RAII to ensure inhibition is always properly released.
pub struct SessionInhibitorGuard {
    #[cfg(target_os = "windows")]
    hwnd: Option<windows::Win32::Foundation::HWND>,
    #[cfg(target_os = "linux")]
    _inhibit_fd: Option<std::os::fd::OwnedFd>,
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    _marker: std::marker::PhantomData<()>,
}

#[cfg(target_os = "windows")]
impl SessionInhibitorGuard {
    fn new(
        reason: String,
        _app_name: String,
        options: SessionInhibitOptions,
    ) -> Result<Self, SessionManagementError> {
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
        use windows::Win32::System::Shutdown::ShutdownBlockReasonCreate;
        use windows::Win32::UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
            GetDesktopWindow, WINDOW_EX_STYLE, WNDCLASSW, WS_OVERLAPPED,
        };
        use windows::core::PCWSTR;

        if !options.shutdown && !options.logout {
            return Ok(Self { hwnd: None });
        }

        fn to_wide(s: &str) -> Vec<u16> {
            OsStr::new(s)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect()
        }

        // Wrapper for DefWindowProcW with correct calling convention
        unsafe extern "system" fn default_wnd_proc(
            hwnd: HWND,
            msg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
        ) -> LRESULT {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }

        unsafe {
            let class_name = to_wide("HorizonLatticeSessionInhibitor");

            // Create a message-only window for the shutdown block
            let wc = WNDCLASSW {
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(default_wnd_proc),
                lpszClassName: PCWSTR(class_name.as_ptr()),
                ..Default::default()
            };

            windows::Win32::UI::WindowsAndMessaging::RegisterClassW(&wc);

            let desktop_hwnd = GetDesktopWindow();
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                PCWSTR(class_name.as_ptr()),
                PCWSTR::null(),
                WS_OVERLAPPED,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                0,
                0,
                desktop_hwnd,
                None,
                None,
                None,
            )
            .map_err(|e| SessionManagementError::inhibit(e.to_string()))?;

            let reason_wide = to_wide(&reason);
            ShutdownBlockReasonCreate(hwnd, PCWSTR(reason_wide.as_ptr()))
                .map_err(|e| SessionManagementError::inhibit(e.to_string()))?;

            Ok(Self { hwnd: Some(hwnd) })
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for SessionInhibitorGuard {
    fn drop(&mut self) {
        if let Some(hwnd) = self.hwnd.take() {
            unsafe {
                let _ = windows::Win32::System::Shutdown::ShutdownBlockReasonDestroy(hwnd);
                let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(hwnd);
            }
        }
    }
}

#[cfg(target_os = "linux")]
impl SessionInhibitorGuard {
    fn new(
        reason: String,
        app_name: String,
        options: SessionInhibitOptions,
    ) -> Result<Self, SessionManagementError> {
        use std::os::fd::OwnedFd;

        if !options.shutdown && !options.logout {
            return Ok(Self { _inhibit_fd: None });
        }

        let what = {
            let mut parts = Vec::new();
            if options.shutdown {
                parts.push("shutdown");
            }
            // Note: logind doesn't have a separate "logout" inhibitor,
            // but we can use "shutdown" which covers session end
            parts.join(":")
        };

        let fd =
            pollster::block_on(async { linux_take_inhibit_lock(&what, &app_name, &reason).await })?;

        Ok(Self {
            _inhibit_fd: Some(fd),
        })
    }
}

#[cfg(target_os = "macos")]
impl SessionInhibitorGuard {
    fn new(
        _reason: String,
        _app_name: String,
        options: SessionInhibitOptions,
    ) -> Result<Self, SessionManagementError> {
        // macOS cannot block user-initiated shutdown/logout.
        // We can only prevent idle sleep via IOPMAssertion, which is handled
        // by the SleepInhibitor in power_management.rs.
        if options.shutdown || options.logout {
            // Return a no-op guard but don't error - just log a warning
            eprintln!(
                "Warning: macOS cannot block user-initiated shutdown/logout. \
                 Use SleepInhibitor for idle sleep prevention."
            );
        }
        Ok(Self {
            _marker: std::marker::PhantomData,
        })
    }
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
impl SessionInhibitorGuard {
    fn new(
        _reason: String,
        _app_name: String,
        _options: SessionInhibitOptions,
    ) -> Result<Self, SessionManagementError> {
        Err(SessionManagementError::unsupported_platform(
            "session inhibition not supported on this platform",
        ))
    }
}

impl fmt::Debug for SessionInhibitorGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionInhibitorGuard").finish()
    }
}

/// Convenience type alias for the session inhibitor builder.
pub type SessionInhibitor = SessionInhibitorBuilder;

// ============================================================================
// State Location
// ============================================================================

/// Provides platform-appropriate paths for storing application state and configuration.
///
/// This follows platform conventions:
/// - **Windows**: Uses `%APPDATA%` for config, `%LOCALAPPDATA%` for state
/// - **macOS**: Uses `~/Library/Application Support`
/// - **Linux**: Uses XDG directories (`~/.config`, `~/.local/state`, `~/.local/share`)
#[derive(Debug, Clone)]
pub struct StateLocation {
    /// Configuration directory (user settings, preferences).
    config_dir: PathBuf,
    /// State directory (session state, caches that should survive reboots).
    state_dir: PathBuf,
    /// Data directory (user data files, databases).
    data_dir: PathBuf,
    /// Cache directory (temporary files that can be safely deleted).
    cache_dir: PathBuf,
}

impl StateLocation {
    /// Create a new StateLocation for the given application.
    ///
    /// # Arguments
    ///
    /// * `qualifier` - Reverse domain qualifier (e.g., "com.example")
    /// * `organization` - Organization name (e.g., "Example Corp")
    /// * `application` - Application name (e.g., "MyApp")
    ///
    /// On Windows and macOS, the qualifier and organization may be used in
    /// the path. On Linux, typically only the application name is used.
    pub fn new(
        qualifier: &str,
        organization: &str,
        application: &str,
    ) -> Result<Self, SessionManagementError> {
        Self::from_app_name_impl(qualifier, organization, application)
    }

    /// Create a new StateLocation using just the application name.
    ///
    /// This is a convenience method that uses empty qualifier and organization.
    pub fn from_app_name(application: &str) -> Result<Self, SessionManagementError> {
        Self::new("", "", application)
    }

    #[cfg(target_os = "windows")]
    fn from_app_name_impl(
        _qualifier: &str,
        organization: &str,
        application: &str,
    ) -> Result<Self, SessionManagementError> {
        // Use std::env for Windows paths
        let app_folder = if organization.is_empty() {
            application.to_string()
        } else {
            format!("{}\\{}", organization, application)
        };

        let appdata = std::env::var("APPDATA").map_err(|_| {
            SessionManagementError::state_storage("APPDATA environment variable not set")
        })?;
        let local_appdata = std::env::var("LOCALAPPDATA").map_err(|_| {
            SessionManagementError::state_storage("LOCALAPPDATA environment variable not set")
        })?;

        let config_dir = PathBuf::from(&appdata).join(&app_folder);
        let data_dir = PathBuf::from(&appdata).join(&app_folder).join("Data");
        let state_dir = PathBuf::from(&local_appdata)
            .join(&app_folder)
            .join("State");
        let cache_dir = PathBuf::from(&local_appdata)
            .join(&app_folder)
            .join("Cache");

        Ok(Self {
            config_dir,
            state_dir,
            data_dir,
            cache_dir,
        })
    }

    #[cfg(target_os = "macos")]
    fn from_app_name_impl(
        _qualifier: &str,
        _organization: &str,
        application: &str,
    ) -> Result<Self, SessionManagementError> {
        let home = std::env::var("HOME").map_err(|_| {
            SessionManagementError::state_storage("HOME environment variable not set")
        })?;
        let home = PathBuf::from(home);

        let app_support = home.join("Library/Application Support").join(application);
        let cache_dir = home.join("Library/Caches").join(application);

        Ok(Self {
            config_dir: app_support.clone(),
            state_dir: app_support.clone(),
            data_dir: app_support,
            cache_dir,
        })
    }

    #[cfg(target_os = "linux")]
    fn from_app_name_impl(
        _qualifier: &str,
        _organization: &str,
        application: &str,
    ) -> Result<Self, SessionManagementError> {
        let home = std::env::var("HOME").map_err(|_| {
            SessionManagementError::state_storage("HOME environment variable not set")
        })?;
        let home = PathBuf::from(home);

        // Use XDG directories
        let config_home = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".config"));

        let data_home = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".local/share"));

        let state_home = std::env::var("XDG_STATE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".local/state"));

        let cache_home = std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".cache"));

        Ok(Self {
            config_dir: config_home.join(application),
            state_dir: state_home.join(application),
            data_dir: data_home.join(application),
            cache_dir: cache_home.join(application),
        })
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    fn from_app_name_impl(
        _qualifier: &str,
        _organization: &str,
        application: &str,
    ) -> Result<Self, SessionManagementError> {
        // Fallback: use current directory
        let base = std::env::current_dir()
            .map_err(|e| SessionManagementError::state_storage(e.to_string()))?
            .join(format!(".{}", application));

        Ok(Self {
            config_dir: base.join("config"),
            state_dir: base.join("state"),
            data_dir: base.join("data"),
            cache_dir: base.join("cache"),
        })
    }

    /// Get the configuration directory.
    ///
    /// Use this for user settings and preferences that should be backed up.
    pub fn config_dir(&self) -> &PathBuf {
        &self.config_dir
    }

    /// Get the state directory.
    ///
    /// Use this for session state, window positions, and other runtime state
    /// that should survive reboots but doesn't need to be backed up.
    pub fn state_dir(&self) -> &PathBuf {
        &self.state_dir
    }

    /// Get the data directory.
    ///
    /// Use this for user data files, databases, and other important content.
    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Get the cache directory.
    ///
    /// Use this for temporary files that can be safely deleted.
    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    /// Ensure all directories exist, creating them if necessary.
    pub fn ensure_dirs_exist(&self) -> Result<(), SessionManagementError> {
        std::fs::create_dir_all(&self.config_dir)?;
        std::fs::create_dir_all(&self.state_dir)?;
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(&self.cache_dir)?;
        Ok(())
    }
}

// ============================================================================
// Application State
// ============================================================================

/// Simple key-value state storage for application session state.
///
/// This provides a convenient API for saving and loading application state
/// using the platform-appropriate state directory.
pub struct ApplicationState {
    location: StateLocation,
}

impl ApplicationState {
    /// Create a new ApplicationState for the given application.
    ///
    /// This will use the platform-appropriate state directory and ensure
    /// the directory exists.
    pub fn new(
        qualifier: &str,
        organization: &str,
        application: &str,
    ) -> Result<Self, SessionManagementError> {
        let location = StateLocation::new(qualifier, organization, application)?;
        location.ensure_dirs_exist()?;
        Ok(Self { location })
    }

    /// Create a new ApplicationState using just the application name.
    pub fn from_app_name(application: &str) -> Result<Self, SessionManagementError> {
        Self::new("", "", application)
    }

    /// Get the underlying StateLocation.
    pub fn location(&self) -> &StateLocation {
        &self.location
    }

    /// Save state data under the given key.
    ///
    /// The data is saved as a file in the state directory. The key should be
    /// a valid filename (alphanumeric, hyphens, underscores).
    pub fn save(&self, key: &str, data: &[u8]) -> Result<(), SessionManagementError> {
        Self::validate_key(key)?;
        let path = self.location.state_dir.join(format!("{}.state", key));
        std::fs::write(&path, data)?;
        Ok(())
    }

    /// Load state data for the given key.
    ///
    /// Returns `None` if no state exists for this key.
    pub fn load(&self, key: &str) -> Result<Option<Vec<u8>>, SessionManagementError> {
        Self::validate_key(key)?;
        let path = self.location.state_dir.join(format!("{}.state", key));
        match std::fs::read(&path) {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(SessionManagementError::from(e)),
        }
    }

    /// Remove state data for the given key.
    ///
    /// Returns `true` if state existed and was removed, `false` if no state existed.
    pub fn remove(&self, key: &str) -> Result<bool, SessionManagementError> {
        Self::validate_key(key)?;
        let path = self.location.state_dir.join(format!("{}.state", key));
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(SessionManagementError::from(e)),
        }
    }

    /// List all saved state keys.
    pub fn list_keys(&self) -> Result<Vec<String>, SessionManagementError> {
        let mut keys = Vec::new();
        let entries = match std::fs::read_dir(&self.location.state_dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(keys),
            Err(e) => return Err(SessionManagementError::from(e)),
        };

        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str()
                && let Some(key) = name.strip_suffix(".state")
            {
                keys.push(key.to_string());
            }
        }

        Ok(keys)
    }

    /// Clear all saved state.
    pub fn clear(&self) -> Result<(), SessionManagementError> {
        for key in self.list_keys()? {
            self.remove(&key)?;
        }
        Ok(())
    }

    fn validate_key(key: &str) -> Result<(), SessionManagementError> {
        if key.is_empty() {
            return Err(SessionManagementError::state_storage("key cannot be empty"));
        }
        if key.len() > 64 {
            return Err(SessionManagementError::state_storage(
                "key cannot be longer than 64 characters",
            ));
        }
        if !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(SessionManagementError::state_storage(
                "key can only contain alphanumeric characters, hyphens, and underscores",
            ));
        }
        Ok(())
    }
}

impl fmt::Debug for ApplicationState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApplicationState")
            .field("state_dir", &self.location.state_dir)
            .finish()
    }
}

// ============================================================================
// Platform-specific implementations - Linux
// ============================================================================

#[cfg(target_os = "linux")]
async fn linux_session_event_loop(
    inner: &SessionEventWatcherInner,
) -> Result<(), SessionManagementError> {
    use zbus::Connection;
    use zbus_systemd::login1::ManagerProxy;

    let connection = Connection::system()
        .await
        .map_err(|e| SessionManagementError::session_events(e.to_string()))?;

    let manager = ManagerProxy::new(&connection)
        .await
        .map_err(|e| SessionManagementError::session_events(e.to_string()))?;

    // Subscribe to PrepareForShutdown signal
    let mut stream = manager
        .receive_prepare_for_shutdown()
        .await
        .map_err(|e| SessionManagementError::session_events(e.to_string()))?;

    use futures_util::StreamExt;

    while inner.running.load(Ordering::SeqCst) {
        match futures_util::future::poll_fn(|cx| {
            use std::task::Poll;
            match std::pin::Pin::new(&mut stream).poll_next(cx) {
                Poll::Ready(item) => Poll::Ready(Some(item)),
                Poll::Pending => {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        })
        .await
        {
            Some(Some(sig)) => {
                if let Ok(args) = sig.args() {
                    if args.start {
                        // Session is ending - we don't know if it's shutdown or reboot
                        inner.session_ending.emit(SessionEndReason::Shutdown);
                    }
                }
            }
            Some(None) | None => break,
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
async fn linux_take_inhibit_lock(
    what: &str,
    who: &str,
    why: &str,
) -> Result<std::os::fd::OwnedFd, SessionManagementError> {
    use std::os::fd::OwnedFd;
    use zbus::Connection;
    use zbus_systemd::login1::ManagerProxy;

    let connection = Connection::system()
        .await
        .map_err(|e| SessionManagementError::inhibit(e.to_string()))?;

    let manager = ManagerProxy::new(&connection)
        .await
        .map_err(|e| SessionManagementError::inhibit(e.to_string()))?;

    // Use "delay" mode to give the app time to save, not "block" which would
    // prevent shutdown entirely
    let fd: OwnedFd = manager
        .inhibit(what, who, why, "delay")
        .await
        .map_err(|e| SessionManagementError::inhibit(e.to_string()))?;

    Ok(fd)
}

// ============================================================================
// Platform-specific implementations - Windows
// ============================================================================

#[cfg(target_os = "windows")]
fn windows_session_event_loop(
    inner: &SessionEventWatcherInner,
) -> Result<(), SessionManagementError> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::{
        CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow,
        DispatchMessageW, GetMessageW, MSG, PM_NOREMOVE, PeekMessageW, RegisterClassW,
        TranslateMessage, WNDCLASSW, WS_OVERLAPPEDWINDOW,
    };
    use windows::core::PCWSTR;

    // Session-related window messages
    const WM_QUERYENDSESSION: u32 = 0x0011;
    const WM_ENDSESSION: u32 = 0x0016;
    const ENDSESSION_LOGOFF: usize = 0x80000000;

    fn to_wide(s: &str) -> Vec<u16> {
        OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    thread_local! {
        static INNER_PTR: std::cell::Cell<*const SessionEventWatcherInner> =
            const { std::cell::Cell::new(std::ptr::null()) };
    }

    INNER_PTR.with(|cell| {
        cell.set(inner as *const _);
    });

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_QUERYENDSESSION => {
                // Return TRUE to allow shutdown, the app should save state
                // when it receives WM_ENDSESSION
                LRESULT(1)
            }
            WM_ENDSESSION => {
                if wparam.0 != 0 {
                    // Session is actually ending
                    INNER_PTR.with(|cell| {
                        let ptr = cell.get();
                        if !ptr.is_null() {
                            let inner = unsafe { &*ptr };
                            let reason = if lparam.0 as usize & ENDSESSION_LOGOFF != 0 {
                                SessionEndReason::Logout
                            } else {
                                SessionEndReason::Shutdown
                            };
                            inner.session_ending.emit(reason);
                        }
                    });
                }
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
        }
    }

    unsafe {
        let class_name = to_wide("HorizonLatticeSessionWatcher");
        let hinstance = GetModuleHandleW(None)
            .map_err(|e| SessionManagementError::session_events(e.to_string()))?;

        let wc = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            ..Default::default()
        };

        RegisterClassW(&wc);

        let hwnd = CreateWindowExW(
            Default::default(),
            PCWSTR(class_name.as_ptr()),
            PCWSTR::null(),
            WS_OVERLAPPEDWINDOW,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            None,
            None,
            HINSTANCE(hinstance.0),
            None,
        )
        .map_err(|e| SessionManagementError::session_events(e.to_string()))?;

        let mut msg = MSG::default();
        while inner.running.load(Ordering::SeqCst) {
            if PeekMessageW(&mut msg, hwnd, 0, 0, PM_NOREMOVE).as_bool() {
                if !GetMessageW(&mut msg, hwnd, 0, 0).as_bool() {
                    break;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        let _ = DestroyWindow(hwnd);
    }

    INNER_PTR.with(|cell| {
        cell.set(std::ptr::null());
    });

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = SessionManagementError::session_events("test error");
        assert!(error.to_string().contains("session events"));
        assert!(error.to_string().contains("test error"));

        let error = SessionManagementError::unsupported_platform("test");
        assert!(error.is_unsupported_platform());
    }

    #[test]
    fn test_session_end_reason_default() {
        assert_eq!(SessionEndReason::default(), SessionEndReason::Unknown);
    }

    #[test]
    fn test_session_event_watcher_new() {
        let watcher = SessionEventWatcher::new();
        assert!(watcher.is_ok());
        let watcher = watcher.unwrap();
        assert!(!watcher.is_running());
    }

    #[test]
    fn test_session_inhibitor_builder() {
        let builder = SessionInhibitor::new()
            .reason("Saving document")
            .app_name("TestApp")
            .inhibit_shutdown(true)
            .inhibit_logout(true);

        assert_eq!(builder.reason, "Saving document");
        assert_eq!(builder.app_name, "TestApp");
        assert!(builder.options.shutdown);
        assert!(builder.options.logout);
    }

    #[test]
    fn test_session_inhibit_options_default() {
        let options = SessionInhibitOptions::default();
        assert!(!options.shutdown);
        assert!(!options.logout);
    }

    #[test]
    fn test_state_location_creation() {
        let location = StateLocation::new("com.example", "Example", "TestApp");
        assert!(location.is_ok());
        let location = location.unwrap();

        // Just verify paths are non-empty
        assert!(!location.config_dir().as_os_str().is_empty());
        assert!(!location.state_dir().as_os_str().is_empty());
        assert!(!location.data_dir().as_os_str().is_empty());
        assert!(!location.cache_dir().as_os_str().is_empty());
    }

    #[test]
    fn test_state_location_from_app_name() {
        let location = StateLocation::from_app_name("TestApp");
        assert!(location.is_ok());
    }

    #[test]
    fn test_application_state_key_validation() {
        // Valid keys
        assert!(ApplicationState::validate_key("valid-key").is_ok());
        assert!(ApplicationState::validate_key("valid_key").is_ok());
        assert!(ApplicationState::validate_key("ValidKey123").is_ok());

        // Invalid keys
        assert!(ApplicationState::validate_key("").is_err());
        assert!(ApplicationState::validate_key("invalid/key").is_err());
        assert!(ApplicationState::validate_key("invalid.key").is_err());
        assert!(ApplicationState::validate_key("invalid key").is_err());

        // Key too long
        let long_key = "a".repeat(65);
        assert!(ApplicationState::validate_key(&long_key).is_err());
    }

    #[test]
    fn test_application_state_save_load() {
        use std::env;

        // Use a temp directory for testing
        let temp_dir = env::temp_dir().join("horizon-lattice-test-state");
        let _ = std::fs::remove_dir_all(&temp_dir);

        // Create state with temp directory
        let location = StateLocation {
            config_dir: temp_dir.join("config"),
            state_dir: temp_dir.join("state"),
            data_dir: temp_dir.join("data"),
            cache_dir: temp_dir.join("cache"),
        };
        location.ensure_dirs_exist().unwrap();

        let state = ApplicationState { location };

        // Save and load
        state.save("test-key", b"test data").unwrap();
        let loaded = state.load("test-key").unwrap();
        assert_eq!(loaded, Some(b"test data".to_vec()));

        // Load non-existent key
        let missing = state.load("missing-key").unwrap();
        assert_eq!(missing, None);

        // List keys
        let keys = state.list_keys().unwrap();
        assert!(keys.contains(&"test-key".to_string()));

        // Remove key
        assert!(state.remove("test-key").unwrap());
        assert!(!state.remove("test-key").unwrap());

        // Clean up
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_session_end_reason_variants() {
        let _shutdown = SessionEndReason::Shutdown;
        let _restart = SessionEndReason::Restart;
        let _logout = SessionEndReason::Logout;
        let _unknown = SessionEndReason::Unknown;
    }
}
