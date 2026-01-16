//! Clipboard access for cross-platform copy/paste operations.
//!
//! This module provides a thin wrapper around the `arboard` crate for clipboard
//! operations. It supports text operations on all major platforms (Windows, macOS, Linux).
//!
//! # Example
//!
//! ```ignore
//! use horizon_lattice::platform::Clipboard;
//!
//! // Copy text to clipboard
//! if let Ok(mut clipboard) = Clipboard::new() {
//!     clipboard.set_text("Hello, world!").ok();
//! }
//!
//! // Paste text from clipboard
//! if let Ok(clipboard) = Clipboard::new() {
//!     if let Ok(text) = clipboard.get_text() {
//!         println!("Clipboard contains: {}", text);
//!     }
//! }
//! ```
//!
//! # Platform Notes
//!
//! - **Windows**: Uses the Win32 clipboard API
//! - **macOS**: Uses NSPasteboard
//! - **Linux**: Uses X11 selections or Wayland data-control protocol

use std::fmt;

/// Error type for clipboard operations.
#[derive(Debug)]
pub struct ClipboardError {
    message: String,
}

impl ClipboardError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "clipboard error: {}", self.message)
    }
}

impl std::error::Error for ClipboardError {}

impl From<arboard::Error> for ClipboardError {
    fn from(err: arboard::Error) -> Self {
        Self::new(err.to_string())
    }
}

/// Cross-platform clipboard access.
///
/// Provides methods for reading and writing text to the system clipboard.
/// The clipboard instance should be created when needed and can be dropped
/// after use.
///
/// # Thread Safety
///
/// While `Clipboard` is `Send`, it's recommended to perform clipboard
/// operations on the main/UI thread for best compatibility across platforms.
pub struct Clipboard {
    inner: arboard::Clipboard,
}

impl Clipboard {
    /// Create a new clipboard instance.
    ///
    /// # Errors
    ///
    /// Returns an error if the clipboard cannot be accessed, which can happen
    /// if the system clipboard is unavailable or locked by another process.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::platform::Clipboard;
    ///
    /// let clipboard = Clipboard::new()?;
    /// ```
    pub fn new() -> Result<Self, ClipboardError> {
        Ok(Self {
            inner: arboard::Clipboard::new()?,
        })
    }

    /// Get the current text content from the clipboard.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The clipboard is empty
    /// - The clipboard contains non-text data
    /// - The clipboard cannot be accessed
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::platform::Clipboard;
    ///
    /// let mut clipboard = Clipboard::new()?;
    /// match clipboard.get_text() {
    ///     Ok(text) => println!("Clipboard: {}", text),
    ///     Err(_) => println!("No text in clipboard"),
    /// }
    /// ```
    pub fn get_text(&mut self) -> Result<String, ClipboardError> {
        self.inner.get_text().map_err(Into::into)
    }

    /// Set the clipboard text content.
    ///
    /// This replaces any existing clipboard content with the provided text.
    ///
    /// # Errors
    ///
    /// Returns an error if the text cannot be written to the clipboard.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::platform::Clipboard;
    ///
    /// let mut clipboard = Clipboard::new()?;
    /// clipboard.set_text("Hello, world!")?;
    /// ```
    pub fn set_text(&mut self, text: impl AsRef<str>) -> Result<(), ClipboardError> {
        self.inner.set_text(text.as_ref()).map_err(Into::into)
    }

    /// Clear the clipboard contents.
    ///
    /// # Errors
    ///
    /// Returns an error if the clipboard cannot be cleared.
    pub fn clear(&mut self) -> Result<(), ClipboardError> {
        self.inner.clear().map_err(Into::into)
    }

    /// Set HTML content on the clipboard with a plain text fallback.
    ///
    /// This places both HTML and plain text versions on the clipboard, allowing
    /// applications that support rich text to paste the formatted version while
    /// others receive the plain text fallback.
    ///
    /// # Errors
    ///
    /// Returns an error if the content cannot be written to the clipboard.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::platform::Clipboard;
    ///
    /// let mut clipboard = Clipboard::new()?;
    /// clipboard.set_html("<b>Hello</b>", "Hello")?;
    /// ```
    pub fn set_html(
        &mut self,
        html: impl AsRef<str>,
        alt_text: impl AsRef<str>,
    ) -> Result<(), ClipboardError> {
        self.inner
            .set_html(html.as_ref(), Some(alt_text.as_ref()))
            .map_err(Into::into)
    }

    /// Get HTML content from the clipboard.
    ///
    /// Returns the HTML content if available. Many applications place HTML
    /// on the clipboard when copying formatted text.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The clipboard doesn't contain HTML
    /// - The clipboard cannot be accessed
    ///
    /// # Example
    ///
    /// ```ignore
    /// use horizon_lattice::platform::Clipboard;
    ///
    /// let mut clipboard = Clipboard::new()?;
    /// match clipboard.get_html() {
    ///     Ok(html) => println!("HTML: {}", html),
    ///     Err(_) => println!("No HTML in clipboard"),
    /// }
    /// ```
    pub fn get_html(&mut self) -> Result<String, ClipboardError> {
        self.inner.get().html().map_err(Into::into)
    }
}

impl fmt::Debug for Clipboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Clipboard").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_creation() {
        // This test may fail in CI environments without a display
        let result = Clipboard::new();
        // Just verify it doesn't panic - actual clipboard access depends on environment
        let _ = result;
    }

    #[test]
    fn test_clipboard_error_display() {
        let error = ClipboardError::new("test error");
        assert_eq!(error.to_string(), "clipboard error: test error");
    }
}
