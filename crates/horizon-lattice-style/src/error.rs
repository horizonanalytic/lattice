//! Error types for the styling system.

use std::path::PathBuf;

/// Result type alias for style operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the styling system.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// CSS parsing error.
    #[error("CSS parse error at line {line}, column {column}: {message}")]
    Parse {
        /// The error message.
        message: String,
        /// Line number where the error occurred.
        line: u32,
        /// Column number where the error occurred.
        column: u32,
    },

    /// Selector parsing error.
    #[error("Invalid selector '{selector}': {message}")]
    InvalidSelector {
        /// The invalid selector string.
        selector: String,
        /// Description of what's wrong with the selector.
        message: String,
    },

    /// File I/O error.
    #[error("Failed to read stylesheet '{path}': {source}")]
    Io {
        /// Path to the file that failed to read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Hot-reload error.
    #[cfg(feature = "hot-reload")]
    #[error("Hot-reload error: {0}")]
    HotReload(String),

    /// Invalid property value.
    #[error("Invalid value for property '{property}': {message}")]
    InvalidValue {
        /// The property name that has an invalid value.
        property: String,
        /// Description of what's wrong with the value.
        message: String,
    },
}

impl Error {
    /// Create a parse error.
    pub fn parse(message: impl Into<String>, line: u32, column: u32) -> Self {
        Self::Parse {
            message: message.into(),
            line,
            column,
        }
    }

    /// Create a selector error.
    pub fn invalid_selector(selector: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidSelector {
            selector: selector.into(),
            message: message.into(),
        }
    }

    /// Create an I/O error.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Create a value error.
    pub fn invalid_value(property: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidValue {
            property: property.into(),
            message: message.into(),
        }
    }
}
