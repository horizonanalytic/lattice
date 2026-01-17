//! Desktop notification support for cross-platform system notifications.
//!
//! This module provides a cross-platform API for displaying system notifications
//! with support for titles, bodies, icons, urgency levels, and user interactions.
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::platform::{Notification, Timeout};
//!
//! // Simple notification
//! Notification::new()
//!     .summary("Download Complete")
//!     .body("Your file has been downloaded successfully.")
//!     .show()?;
//!
//! // Notification with timeout
//! Notification::new()
//!     .summary("Reminder")
//!     .body("Meeting in 5 minutes")
//!     .timeout(Timeout::Milliseconds(5000))
//!     .show()?;
//! ```
//!
//! # Platform Notes
//!
//! - **Linux**: Full support including actions, urgency levels, and close notifications
//! - **macOS**: Basic support (summary, body, subtitle). No actions or urgency.
//! - **Windows**: Basic support (summary, body, icon). No actions or urgency.
//!
//! # Feature Flags
//!
//! - `notifications`: Enable notification support (included in default features)
//! - `notification-actions`: Enable action buttons (Linux only)

use std::fmt;
use std::sync::Arc;

use horizon_lattice_core::signal::Signal;

/// Error type for notification operations.
#[derive(Debug)]
pub struct NotificationError {
    message: String,
}

impl NotificationError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "notification error: {}", self.message)
    }
}

impl std::error::Error for NotificationError {}

impl From<notify_rust::error::Error> for NotificationError {
    fn from(err: notify_rust::error::Error) -> Self {
        Self::new(err.to_string())
    }
}

/// Notification urgency level.
///
/// Determines how prominently the notification is displayed.
/// On platforms that don't support urgency, this is ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Urgency {
    /// Low urgency - may be displayed less prominently.
    Low,
    /// Normal urgency - standard display.
    #[default]
    Normal,
    /// Critical urgency - displayed prominently, may require user acknowledgment.
    Critical,
}

#[cfg(target_os = "linux")]
impl From<Urgency> for notify_rust::Urgency {
    fn from(urgency: Urgency) -> Self {
        match urgency {
            Urgency::Low => notify_rust::Urgency::Low,
            Urgency::Normal => notify_rust::Urgency::Normal,
            Urgency::Critical => notify_rust::Urgency::Critical,
        }
    }
}

/// Notification timeout configuration.
///
/// Controls how long the notification is displayed before auto-dismissing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Timeout {
    /// Use the system/notification server default timeout.
    #[default]
    Default,
    /// Never auto-dismiss (persistent notification).
    Never,
    /// Dismiss after the specified number of milliseconds.
    Milliseconds(u32),
}

impl From<Timeout> for notify_rust::Timeout {
    fn from(timeout: Timeout) -> Self {
        match timeout {
            Timeout::Default => notify_rust::Timeout::Default,
            Timeout::Never => notify_rust::Timeout::Never,
            Timeout::Milliseconds(ms) => notify_rust::Timeout::Milliseconds(ms),
        }
    }
}

/// Reason why a notification was closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    /// The notification expired (timed out).
    Expired,
    /// The user dismissed the notification.
    Dismissed,
    /// The notification was closed via API call.
    CloseAction,
    /// The close reason is unknown or platform doesn't provide this info.
    Unknown,
}

/// An action that can be attached to a notification.
///
/// Actions appear as buttons on the notification (Linux only).
#[cfg(feature = "notification-actions")]
#[derive(Debug, Clone)]
pub struct NotificationAction {
    /// Unique identifier for this action.
    pub id: String,
    /// Display label for the action button.
    pub label: String,
}

#[cfg(feature = "notification-actions")]
impl NotificationAction {
    /// Create a new notification action.
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
        }
    }
}

/// Builder for creating and displaying notifications.
///
/// Use [`Notification::new()`] to create a new builder, configure it with
/// the various methods, and call [`show()`](Notification::show) to display it.
#[derive(Debug, Clone)]
pub struct Notification {
    summary: String,
    body: Option<String>,
    #[cfg(target_os = "macos")]
    subtitle: Option<String>,
    icon: Option<String>,
    app_name: Option<String>,
    timeout: Timeout,
    urgency: Urgency,
    #[cfg(feature = "notification-actions")]
    actions: Vec<NotificationAction>,
}

impl Default for Notification {
    fn default() -> Self {
        Self::new()
    }
}

impl Notification {
    /// Create a new notification builder with default settings.
    pub fn new() -> Self {
        Self {
            summary: String::new(),
            body: None,
            #[cfg(target_os = "macos")]
            subtitle: None,
            icon: None,
            app_name: None,
            timeout: Timeout::Default,
            urgency: Urgency::Normal,
            #[cfg(feature = "notification-actions")]
            actions: Vec::new(),
        }
    }

    /// Set the notification summary (title).
    ///
    /// This is the main heading of the notification and is required.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    /// Set the notification body text.
    ///
    /// This is the detailed message content.
    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Set the notification subtitle (macOS only).
    ///
    /// On other platforms, this is ignored.
    #[cfg(target_os = "macos")]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Set the notification icon.
    ///
    /// This can be:
    /// - An icon name from the current icon theme (e.g., "dialog-information")
    /// - A path to an image file
    ///
    /// On platforms that don't support custom icons, this is ignored.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the application name for the notification.
    ///
    /// If not set, the system may use a default or the process name.
    pub fn app_name(mut self, name: impl Into<String>) -> Self {
        self.app_name = Some(name.into());
        self
    }

    /// Set the notification timeout.
    ///
    /// See [`Timeout`] for available options.
    pub fn timeout(mut self, timeout: Timeout) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the notification urgency level.
    ///
    /// On platforms that don't support urgency (macOS, Windows), this is ignored.
    pub fn urgency(mut self, urgency: Urgency) -> Self {
        self.urgency = urgency;
        self
    }

    /// Add an action button to the notification.
    ///
    /// This is only available on Linux with the `notification-actions` feature.
    /// Actions appear as buttons that the user can click.
    ///
    /// # Arguments
    ///
    /// * `id` - A unique identifier for this action, returned when clicked
    /// * `label` - The display text for the button
    #[cfg(feature = "notification-actions")]
    pub fn action(mut self, id: impl Into<String>, label: impl Into<String>) -> Self {
        self.actions.push(NotificationAction::new(id, label));
        self
    }

    /// Build the internal notify-rust notification.
    fn build_notification(&self) -> Result<notify_rust::Notification, NotificationError> {
        if self.summary.is_empty() {
            return Err(NotificationError::new("notification summary is required"));
        }

        let mut notification = notify_rust::Notification::new();
        notification.summary(&self.summary);

        if let Some(ref body) = self.body {
            notification.body(body);
        }

        #[cfg(target_os = "macos")]
        if let Some(ref subtitle) = self.subtitle {
            notification.subtitle(subtitle);
        }

        if let Some(ref icon) = self.icon {
            notification.icon(icon);
        }

        if let Some(ref app_name) = self.app_name {
            notification.appname(app_name);
        }

        let timeout: notify_rust::Timeout = self.timeout.into();
        notification.timeout(timeout);

        // Urgency is only supported on Linux
        #[cfg(target_os = "linux")]
        {
            notification.urgency(self.urgency.into());
        }

        // Actions are only supported on Linux with the feature enabled
        #[cfg(feature = "notification-actions")]
        {
            for action in &self.actions {
                notification.action(&action.id, &action.label);
            }
        }

        Ok(notification)
    }

    /// Display the notification.
    ///
    /// Returns a [`NotificationHandle`] that can be used to track the notification
    /// and receive events when the user interacts with it.
    ///
    /// # Platform Notes
    ///
    /// - On **Linux** and **macOS**, returns a full handle with interaction support
    /// - On **Windows**, the notification is shown but the handle has limited functionality
    ///
    /// # Errors
    ///
    /// Returns an error if the notification cannot be displayed, which may happen if:
    /// - The notification server is not available
    /// - Required fields (summary) are missing
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub fn show(self) -> Result<NotificationHandle, NotificationError> {
        let notification = self.build_notification()?;
        let handle = notification.show()?;
        Ok(NotificationHandle::new(handle))
    }

    /// Display the notification (Windows version).
    ///
    /// On Windows, notifications don't return a handle for interaction tracking.
    ///
    /// # Errors
    ///
    /// Returns an error if the notification cannot be displayed.
    #[cfg(target_os = "windows")]
    pub fn show(self) -> Result<NotificationHandle, NotificationError> {
        let mut notification = self.build_notification()?;
        notification.show()?;
        Ok(NotificationHandle::new())
    }

    /// Display the notification (fallback for other platforms).
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    pub fn show(self) -> Result<NotificationHandle, NotificationError> {
        let mut notification = self.build_notification()?;
        notification.show()?;
        Ok(NotificationHandle::new())
    }
}

/// A handle to a displayed notification.
///
/// This handle allows you to:
/// - Close the notification programmatically (Linux/macOS only)
/// - Wait for and handle user interactions (Linux only)
/// - Connect to signals for notification events
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub struct NotificationHandle {
    #[allow(dead_code)] // Used on Linux, stored on macOS to keep notification alive
    inner: notify_rust::NotificationHandle,
    /// Signal emitted when the notification is clicked.
    clicked: Arc<Signal<()>>,
    /// Signal emitted when an action button is clicked (Linux only).
    #[cfg(feature = "notification-actions")]
    action_invoked: Arc<Signal<String>>,
    /// Signal emitted when the notification is closed.
    closed: Arc<Signal<CloseReason>>,
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl NotificationHandle {
    fn new(handle: notify_rust::NotificationHandle) -> Self {
        Self {
            inner: handle,
            clicked: Arc::new(Signal::new()),
            #[cfg(feature = "notification-actions")]
            action_invoked: Arc::new(Signal::new()),
            closed: Arc::new(Signal::new()),
        }
    }

    /// Get the signal that is emitted when the notification is clicked.
    ///
    /// Note: This only works on Linux. On other platforms, the signal is never emitted.
    pub fn clicked(&self) -> &Signal<()> {
        &self.clicked
    }

    /// Get the signal that is emitted when an action button is clicked.
    ///
    /// The signal carries the action ID that was clicked.
    ///
    /// Note: This only works on Linux with the `notification-actions` feature.
    #[cfg(feature = "notification-actions")]
    pub fn action_invoked(&self) -> &Signal<String> {
        &self.action_invoked
    }

    /// Get the signal that is emitted when the notification is closed.
    ///
    /// The signal carries the reason for closing.
    ///
    /// Note: This only works on Linux. On other platforms, the signal is never emitted.
    pub fn closed(&self) -> &Signal<CloseReason> {
        &self.closed
    }

    /// Close the notification programmatically.
    ///
    /// On platforms that support it, this removes the notification from display.
    #[cfg(target_os = "linux")]
    pub fn close(self) {
        self.inner.close();
    }

    /// Wait for user interaction and emit appropriate signals.
    ///
    /// This is a blocking call that waits for the user to interact with the
    /// notification (click, action, or close). The appropriate signal is emitted
    /// when an interaction occurs.
    ///
    /// This is only fully supported on Linux. On macOS, this returns immediately.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::platform::Notification;
    ///
    /// let handle = Notification::new()
    ///     .summary("Click me!")
    ///     .show()?;
    ///
    /// handle.clicked().connect(|_| {
    ///     println!("Notification was clicked!");
    /// });
    ///
    /// // This blocks until the user interacts
    /// handle.wait_for_action();
    /// ```
    #[cfg(target_os = "linux")]
    pub fn wait_for_action(self) {
        let clicked = self.clicked.clone();
        #[cfg(feature = "notification-actions")]
        let action_invoked = self.action_invoked.clone();
        let closed = self.closed.clone();

        self.inner.wait_for_action(|action| match action {
            "__closed" => {
                closed.emit(CloseReason::Dismissed);
            }
            "default" => {
                clicked.emit(());
            }
            action_id => {
                #[cfg(feature = "notification-actions")]
                {
                    action_invoked.emit(action_id.to_string());
                }
                #[cfg(not(feature = "notification-actions"))]
                {
                    let _ = action_id;
                }
            }
        });
    }

    /// Get the notification ID.
    ///
    /// This can be used to reference the notification for updates.
    #[cfg(target_os = "linux")]
    pub fn id(&self) -> u32 {
        self.inner.id()
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
impl fmt::Debug for NotificationHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotificationHandle").finish_non_exhaustive()
    }
}

/// A handle to a displayed notification (Windows/fallback version).
///
/// On Windows and unsupported platforms, the notification handle has limited
/// functionality since the underlying notification system doesn't provide
/// interaction tracking.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub struct NotificationHandle {
    /// Signal emitted when the notification is clicked (never emitted on this platform).
    clicked: Arc<Signal<()>>,
    /// Signal emitted when an action button is clicked (never emitted on this platform).
    #[cfg(feature = "notification-actions")]
    action_invoked: Arc<Signal<String>>,
    /// Signal emitted when the notification is closed (never emitted on this platform).
    closed: Arc<Signal<CloseReason>>,
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
impl NotificationHandle {
    fn new() -> Self {
        Self {
            clicked: Arc::new(Signal::new()),
            #[cfg(feature = "notification-actions")]
            action_invoked: Arc::new(Signal::new()),
            closed: Arc::new(Signal::new()),
        }
    }

    /// Get the signal that is emitted when the notification is clicked.
    ///
    /// Note: On Windows, this signal is never emitted.
    pub fn clicked(&self) -> &Signal<()> {
        &self.clicked
    }

    /// Get the signal that is emitted when an action button is clicked.
    ///
    /// Note: On Windows, this signal is never emitted.
    #[cfg(feature = "notification-actions")]
    pub fn action_invoked(&self) -> &Signal<String> {
        &self.action_invoked
    }

    /// Get the signal that is emitted when the notification is closed.
    ///
    /// Note: On Windows, this signal is never emitted.
    pub fn closed(&self) -> &Signal<CloseReason> {
        &self.closed
    }

    /// No-op on Windows - notifications cannot be closed programmatically.
    pub fn close(self) {
        // No-op on Windows
    }

    /// No-op on Windows - returns immediately.
    pub fn wait_for_action(self) {
        // No-op on Windows
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
impl fmt::Debug for NotificationHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NotificationHandle").finish_non_exhaustive()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_builder_default() {
        let notification = Notification::new();
        assert!(notification.summary.is_empty());
        assert!(notification.body.is_none());
        assert_eq!(notification.timeout, Timeout::Default);
        assert_eq!(notification.urgency, Urgency::Normal);
    }

    #[test]
    fn test_notification_builder_chain() {
        let notification = Notification::new()
            .summary("Test Summary")
            .body("Test Body")
            .icon("dialog-information")
            .app_name("Test App")
            .timeout(Timeout::Milliseconds(5000))
            .urgency(Urgency::Critical);

        assert_eq!(notification.summary, "Test Summary");
        assert_eq!(notification.body, Some("Test Body".to_string()));
        assert_eq!(notification.icon, Some("dialog-information".to_string()));
        assert_eq!(notification.app_name, Some("Test App".to_string()));
        assert_eq!(notification.timeout, Timeout::Milliseconds(5000));
        assert_eq!(notification.urgency, Urgency::Critical);
    }

    #[test]
    fn test_notification_error() {
        let error = NotificationError::new("test error");
        assert_eq!(error.to_string(), "notification error: test error");
    }

    #[test]
    fn test_urgency_values() {
        assert_eq!(Urgency::default(), Urgency::Normal);
        assert_ne!(Urgency::Low, Urgency::Critical);
    }

    #[test]
    fn test_timeout_values() {
        assert_eq!(Timeout::default(), Timeout::Default);
        assert_ne!(Timeout::Never, Timeout::Milliseconds(1000));
    }

    #[test]
    fn test_close_reason_values() {
        let reasons = [
            CloseReason::Expired,
            CloseReason::Dismissed,
            CloseReason::CloseAction,
            CloseReason::Unknown,
        ];
        // All reasons should be distinct
        for (i, r1) in reasons.iter().enumerate() {
            for (j, r2) in reasons.iter().enumerate() {
                if i != j {
                    assert_ne!(r1, r2);
                }
            }
        }
    }

    #[cfg(feature = "notification-actions")]
    #[test]
    fn test_notification_with_actions() {
        let notification = Notification::new()
            .summary("Test")
            .action("reply", "Reply")
            .action("dismiss", "Dismiss");

        assert_eq!(notification.actions.len(), 2);
        assert_eq!(notification.actions[0].id, "reply");
        assert_eq!(notification.actions[0].label, "Reply");
    }

    #[test]
    fn test_empty_summary_error() {
        let notification = Notification::new();
        let result = notification.build_notification();
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("summary is required"));
        }
    }

    #[test]
    fn test_timeout_conversion() {
        let timeout: notify_rust::Timeout = Timeout::Default.into();
        assert!(matches!(timeout, notify_rust::Timeout::Default));

        let timeout: notify_rust::Timeout = Timeout::Never.into();
        assert!(matches!(timeout, notify_rust::Timeout::Never));

        let timeout: notify_rust::Timeout = Timeout::Milliseconds(5000).into();
        assert!(matches!(timeout, notify_rust::Timeout::Milliseconds(5000)));
    }
}
