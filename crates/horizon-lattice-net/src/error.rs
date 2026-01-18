//! Error types for the networking module.

use std::fmt;

/// Network-specific errors.
#[derive(Debug)]
pub enum NetworkError {
    /// HTTP request failed.
    Request(reqwest::Error),
    /// Invalid URL provided.
    InvalidUrl(url::ParseError),
    /// Request timed out.
    Timeout,
    /// Connection refused or failed.
    Connection(String),
    /// TLS/SSL error.
    Tls(String),
    /// Invalid header name or value.
    InvalidHeader(String),
    /// JSON serialization/deserialization error.
    Json(serde_json::Error),
    /// I/O error.
    Io(std::io::Error),
    /// Request was cancelled.
    Cancelled,
    /// Invalid response body.
    InvalidBody(String),
    /// HTTP error status (4xx or 5xx).
    HttpStatus {
        /// The HTTP status code.
        status: u16,
        /// Optional error message from the response body.
        message: Option<String>,
    },
    /// Redirect limit exceeded.
    TooManyRedirects,
    /// Proxy configuration error.
    Proxy(String),
    /// Authentication failed.
    Authentication(String),
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(err) => write!(f, "HTTP request error: {err}"),
            Self::InvalidUrl(err) => write!(f, "Invalid URL: {err}"),
            Self::Timeout => write!(f, "Request timed out"),
            Self::Connection(msg) => write!(f, "Connection error: {msg}"),
            Self::Tls(msg) => write!(f, "TLS error: {msg}"),
            Self::InvalidHeader(msg) => write!(f, "Invalid header: {msg}"),
            Self::Json(err) => write!(f, "JSON error: {err}"),
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Cancelled => write!(f, "Request was cancelled"),
            Self::InvalidBody(msg) => write!(f, "Invalid response body: {msg}"),
            Self::HttpStatus { status, message } => {
                if let Some(msg) = message {
                    write!(f, "HTTP {status}: {msg}")
                } else {
                    write!(f, "HTTP {status}")
                }
            }
            Self::TooManyRedirects => write!(f, "Too many redirects"),
            Self::Proxy(msg) => write!(f, "Proxy error: {msg}"),
            Self::Authentication(msg) => write!(f, "Authentication error: {msg}"),
        }
    }
}

impl std::error::Error for NetworkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Request(err) => Some(err),
            Self::InvalidUrl(err) => Some(err),
            Self::Json(err) => Some(err),
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for NetworkError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            Self::Timeout
        } else if err.is_connect() {
            Self::Connection(err.to_string())
        } else if err.is_redirect() {
            Self::TooManyRedirects
        } else {
            Self::Request(err)
        }
    }
}

impl From<url::ParseError> for NetworkError {
    fn from(err: url::ParseError) -> Self {
        Self::InvalidUrl(err)
    }
}

impl From<serde_json::Error> for NetworkError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<std::io::Error> for NetworkError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<http::header::InvalidHeaderName> for NetworkError {
    fn from(err: http::header::InvalidHeaderName) -> Self {
        Self::InvalidHeader(err.to_string())
    }
}

impl From<http::header::InvalidHeaderValue> for NetworkError {
    fn from(err: http::header::InvalidHeaderValue) -> Self {
        Self::InvalidHeader(err.to_string())
    }
}

/// A specialized Result type for network operations.
pub type Result<T> = std::result::Result<T, NetworkError>;
