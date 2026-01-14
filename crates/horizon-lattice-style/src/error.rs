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
        message: String,
        line: u32,
        column: u32,
    },

    /// Selector parsing error.
    #[error("Invalid selector '{selector}': {message}")]
    InvalidSelector { selector: String, message: String },

    /// File I/O error.
    #[error("Failed to read stylesheet '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Hot-reload error.
    #[cfg(feature = "hot-reload")]
    #[error("Hot-reload error: {0}")]
    HotReload(String),

    /// Invalid property value.
    #[error("Invalid value for property '{property}': {message}")]
    InvalidValue { property: String, message: String },
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
